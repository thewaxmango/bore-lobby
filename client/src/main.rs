mod app;
mod net;
mod ui;

use app::App;
use crossterm::event::{Event, EventStream};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use futures_lite::StreamExt;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use bore_lobby_common::protocol::ClientMsg;
use std::io::stdout;
use std::time::Duration;
use net::Connection;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:9900".to_string());

    let mut conn = Connection::connect(&addr).await?;

    // Setup terminal
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    let mut event_stream = EventStream::new();
    let mut ping_interval = tokio::time::interval(Duration::from_secs(30));
    ping_interval.tick().await; // consume the immediate first tick

    let result: Result<(), Box<dyn std::error::Error>> = async {
        loop {
            terminal.draw(|f| ui::render(f, &app))?;

            tokio::select! {
                event = event_stream.next() => {
                    if let Some(Ok(Event::Key(key))) = event {
                        app.handle_key(key);
                    }
                }
                msg = conn.rx.recv() => {
                    match msg {
                        Some(msg) => app.handle_server_msg(msg),
                        None => {
                            app.status_message = Some("Disconnected from server".to_string());
                            break;
                        }
                    }
                }
                _ = ping_interval.tick() => {
                    app.outgoing.push(ClientMsg::Ping);
                }
            }

            // Send any outgoing messages
            for msg in app.outgoing.drain(..) {
                if conn.send(msg).await.is_err() {
                    app.status_message = Some("Disconnected from server".to_string());
                    break;
                }
            }

            if app.should_quit {
                break;
            }
        }
        Ok(())
    }.await;

    // Clean up connection and terminal
    conn.shutdown().await;
    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;

    result
}
