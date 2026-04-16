use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::{ActiveGame, App, Screen};

pub fn render(f: &mut Frame, app: &App) {
    match app.screen {
        Screen::NameEntry => render_name_entry(f, app),
        Screen::MainMenu => render_main_menu(f, app),
        Screen::GameSelect => render_game_select(f, app),
        Screen::CreatePassword => render_create_password(f, app),
        Screen::JoinCode => render_join_code(f, app),
        Screen::JoinPassword { .. } => render_join_password(f, app),
        Screen::Connecting => render_connecting(f),
        Screen::RoomLobby => render_room_lobby(f, app),
        Screen::InGame => render_in_game(f, app),
    }
}

fn render_name_entry(f: &mut Frame, app: &App) {
    let area = centered_rect(40, 7, f.area());
    let block = Block::default()
        .title(" bore-lobby ")
        .borders(Borders::ALL);
    let text = vec![
        Line::from(""),
        Line::from("  Enter your name:"),
        Line::from(format!("  > {}_", app.input_buffer)),
        Line::from(""),
        Line::from(Span::styled(
            "  [Enter] confirm  [Esc] quit",
            Style::default().fg(Color::DarkGray),
        )),
    ];
    f.render_widget(Paragraph::new(text).block(block), area);
}

fn render_main_menu(f: &mut Frame, app: &App) {
    let area = centered_rect(40, 9, f.area());
    let block = Block::default()
        .title(format!(" {} ", app.player_name))
        .borders(Borders::ALL);
    let mut text = vec![
        Line::from(""),
        Line::from("  [C] Create room"),
        Line::from("  [J] Join room"),
        Line::from(""),
        Line::from(Span::styled(
            "  [Esc] quit",
            Style::default().fg(Color::DarkGray),
        )),
    ];
    if let Some(ref msg) = app.status_message {
        text.push(Line::from(Span::styled(
            format!("  {msg}"),
            Style::default().fg(Color::Red),
        )));
    }
    f.render_widget(Paragraph::new(text).block(block), area);
}

fn render_game_select(f: &mut Frame, app: &App) {
    let height = (app.available_games.len() + 4) as u16;
    let area = centered_rect(40, height, f.area());
    let block = Block::default()
        .title(" Select Game ")
        .borders(Borders::ALL);

    let mut text = vec![Line::from("")];
    for (i, (_id, display_name)) in app.available_games.iter().enumerate() {
        text.push(Line::from(format!("  [{}] {display_name}", i + 1)));
    }
    text.push(Line::from(""));
    text.push(Line::from(Span::styled(
        "  [Esc] back",
        Style::default().fg(Color::DarkGray),
    )));
    f.render_widget(Paragraph::new(text).block(block), area);
}

fn render_create_password(f: &mut Frame, app: &App) {
    let area = centered_rect(44, 8, f.area());
    let block = Block::default()
        .title(" Create Room ")
        .borders(Borders::ALL);
    let masked: String = "*".repeat(app.input_buffer.len());
    let text = vec![
        Line::from(""),
        Line::from("  Set password (or leave blank):"),
        Line::from(format!("  > {masked}_")),
        Line::from(""),
        Line::from(Span::styled(
            "  [Enter] create  [Esc] back",
            Style::default().fg(Color::DarkGray),
        )),
    ];
    f.render_widget(Paragraph::new(text).block(block), area);
}

fn render_join_code(f: &mut Frame, app: &App) {
    let area = centered_rect(44, 8, f.area());
    let block = Block::default()
        .title(" Join Room ")
        .borders(Borders::ALL);
    let mut text = vec![
        Line::from(""),
        Line::from("  Enter room code:"),
        Line::from(format!("  > {}_", app.input_buffer)),
        Line::from(""),
        Line::from(Span::styled(
            "  [Enter] next  [Esc] back",
            Style::default().fg(Color::DarkGray),
        )),
    ];
    if let Some(ref msg) = app.status_message {
        text.push(Line::from(Span::styled(
            format!("  {msg}"),
            Style::default().fg(Color::Red),
        )));
    }
    f.render_widget(Paragraph::new(text).block(block), area);
}

fn render_join_password(f: &mut Frame, app: &App) {
    let code = match &app.screen {
        Screen::JoinPassword { room_code } => room_code.as_str(),
        _ => "????",
    };
    let area = centered_rect(44, 8, f.area());
    let block = Block::default()
        .title(format!(" Join {code} "))
        .borders(Borders::ALL);
    let masked: String = "*".repeat(app.input_buffer.len());
    let text = vec![
        Line::from(""),
        Line::from("  Password (or leave blank):"),
        Line::from(format!("  > {masked}_")),
        Line::from(""),
        Line::from(Span::styled(
            "  [Enter] join  [Esc] back",
            Style::default().fg(Color::DarkGray),
        )),
    ];
    f.render_widget(Paragraph::new(text).block(block), area);
}

fn render_connecting(f: &mut Frame) {
    let area = centered_rect(30, 5, f.area());
    let block = Block::default().borders(Borders::ALL);
    let text = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Joining room...",
            Style::default().fg(Color::Yellow),
        )),
    ];
    f.render_widget(Paragraph::new(text).block(block), area);
}

fn render_room_lobby(f: &mut Frame, app: &App) {
    let area = centered_rect(50, 14, f.area());
    let code = app.room_code.as_deref().unwrap_or("????");
    let block = Block::default()
        .title(format!(" Room: {code} - {} ", app.game_name))
        .borders(Borders::ALL);

    let mut lines = vec![Line::from("")];
    lines.push(Line::from(Span::styled(
        "  Players:",
        Style::default().add_modifier(Modifier::BOLD),
    )));
    for name in &app.players {
        let marker = if name == &app.host_name { "★" } else { "·" };
        lines.push(Line::from(format!("    {marker} {name}")));
    }
    lines.push(Line::from(""));

    if app.is_host {
        lines.push(Line::from("  [Enter] Start game"));
    } else {
        lines.push(Line::from(Span::styled(
            "  Waiting for host to start...",
            Style::default().fg(Color::Yellow),
        )));
    }
    lines.push(Line::from(Span::styled(
        "  [Esc] leave room",
        Style::default().fg(Color::DarkGray),
    )));

    if let Some(ref msg) = app.status_message {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("  {msg}"),
            Style::default().fg(Color::Red),
        )));
    }

    f.render_widget(Paragraph::new(lines).block(block), area);
}

fn render_in_game(f: &mut Frame, app: &App) {
    // Top-level: main content area + controls bar at the bottom
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(10),   // main content (game + events side by side)
            Constraint::Length(3), // controls
        ])
        .split(f.area());

    // Main content: game on the left, events on the right
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(42),      // game area (4 cards × 10 wide + 2 border)
            Constraint::Length(36),    // event log
        ])
        .split(rows[0]);

    // Delegate game rendering to the active game plugin
    match &app.active_game {
        Some(ActiveGame::ProSet(game)) => {
            bore_lobby_proset::ui::render(
                f,
                game,
                cols[0],
                app.room_code.as_deref().unwrap_or("????"),
                &app.player_name,
            );
        }
        None => {
            let p = Paragraph::new("No game loaded")
                .block(Block::default().borders(Borders::ALL));
            f.render_widget(p, cols[0]);
        }
    }

    // Event log on the right
    let inner_width = cols[1].width.saturating_sub(2) as usize; // inside borders
    let inner_height = cols[1].height.saturating_sub(2) as usize;

    // Build lines from events, accounting for wrap to estimate visible count
    let mut wrapped_lines = 0usize;
    let mut visible_start = app.event_log.len();
    for e in app.event_log.iter().rev() {
        let text = format!(" >> {e}");
        let lines_needed = if inner_width > 0 {
            (text.len() + inner_width - 1) / inner_width
        } else {
            1
        };
        if wrapped_lines + lines_needed > inner_height {
            break;
        }
        wrapped_lines += lines_needed;
        visible_start = visible_start.saturating_sub(1);
    }

    let log_lines: Vec<Line> = app.event_log[visible_start..]
        .iter()
        .map(|e| Line::from(format!(" >> {e}")))
        .collect();
    let log = Paragraph::new(log_lines)
        .wrap(Wrap { trim: false })
        .block(
            Block::default()
                .title(" Events ")
                .borders(Borders::ALL),
        );
    f.render_widget(log, cols[1]);

    // Controls (generic) — changes based on game-over state
    let game_over = app.winner.is_some();
    let controls = if game_over {
        Paragraph::new(Line::from(vec![
            Span::styled("[Esc]", Style::default().fg(Color::DarkGray)),
            Span::raw(" Back to lobby"),
        ]))
    } else {
        Paragraph::new(Line::from(vec![
            Span::styled("[Esc]", Style::default().fg(Color::DarkGray)),
            Span::raw(" Leave"),
        ]))
    }
    .block(Block::default().borders(Borders::ALL));
    f.render_widget(controls, rows[1]);

    // Status message overlay
    if let Some(ref msg) = app.status_message {
        let popup = centered_rect(40, 3, f.area());
        let p = Paragraph::new(Line::from(Span::styled(
            msg.clone(),
            Style::default().fg(Color::Red),
        )))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
        f.render_widget(p, popup);
    }

    // Game over overlay — positioned in the game field below the header
    if let Some(ref winner) = app.winner {
        let time_text = if let Some(ms) = app.final_elapsed_ms {
            let total_secs = ms / 1000;
            let h = total_secs / 3600;
            let m = (total_secs % 3600) / 60;
            let s = total_secs % 60;
            if h > 0 {
                Some(format!("  Time: {h:02}:{m:02}:{s:02}"))
            } else {
                Some(format!("  Time: {m:02}:{s:02}"))
            }
        } else {
            None
        };

        let height = 3 + if time_text.is_some() { 1 } else { 0 };
        // Place below the header (3 rows) within the game column
        let overlay_area = Rect::new(
            cols[0].x,
            cols[0].y + 3,
            cols[0].width,
            height,
        );

        let mut lines = vec![
            Line::from(Span::styled(
                format!("  {winner} wins!"),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )),
        ];
        if let Some(ref t) = time_text {
            lines.push(Line::from(Span::styled(
                t.clone(),
                Style::default().fg(Color::DarkGray),
            )));
        }

        let block = Block::default()
            .title(" Game Over ")
            .borders(Borders::ALL);
        f.render_widget(Paragraph::new(lines).block(block), overlay_area);
    }
}


fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length((area.height.saturating_sub(height)) / 2),
            Constraint::Length(height),
            Constraint::Min(0),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length((area.width.saturating_sub(width)) / 2),
            Constraint::Length(width),
            Constraint::Min(0),
        ])
        .split(vertical[1])[1]
}
