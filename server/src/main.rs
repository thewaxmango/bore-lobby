mod connection;
mod lobby;

use bore_cli::client::Client as BoreClient;
use lobby::Lobby;
use bore_lobby_common::game::GameRegistry;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::signal;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() {
    let addr = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "0.0.0.0:9900".to_string());
    let listener = TcpListener::bind(&addr).await.expect("failed to bind");
    let local_port = listener.local_addr().expect("local_addr").port();
    println!("bore-lobby server listening on {addr}");

    // Open a bore tunnel so remote players can connect without port
    // forwarding, then keep it alive in a supervisor task that
    // reconnects with exponential backoff if the tunnel drops.
    let initial = BoreClient::new("localhost", local_port, "bore.pub", 0, None).await;
    match &initial {
        Ok(c) => println!("public address: bore.pub:{}", c.remote_port()),
        Err(e) => eprintln!("failed to open bore tunnel: {e} (will keep retrying)"),
    }
    tokio::spawn(maintain_tunnel(local_port, initial.ok()));

    println!("Press Ctrl+C to stop");

    // Register available games
    let mut registry = GameRegistry::new();
    bore_lobby_proset::register_games(&mut registry);

    let lobby = Arc::new(Mutex::new(Lobby::new(registry)));

    loop {
        tokio::select! {
            result = listener.accept() => {
                let (stream, _) = result.expect("failed to accept");
                let lobby = Arc::clone(&lobby);
                tokio::spawn(connection::handle_connection(stream, lobby));
            }
            _ = signal::ctrl_c() => {
                println!("\nShutting down...");
                break;
            }
        }
    }
}

/// Keeps a bore.pub tunnel alive forever. If the tunnel drops (or the
/// initial connect failed), reconnect with exponential backoff. The
/// remote port assigned by bore.pub can change on reconnect, so we log
/// the new address whenever it does.
async fn maintain_tunnel(local_port: u16, initial: Option<BoreClient>) {
    let mut last_port: Option<u16> = initial.as_ref().map(|c| c.remote_port());
    let mut next_client = initial;

    loop {
        let client = match next_client.take() {
            Some(c) => c,
            None => connect_with_backoff(local_port, &mut last_port).await,
        };

        match client.listen().await {
            Ok(()) => eprintln!("bore tunnel closed; reconnecting"),
            Err(e) => eprintln!("bore tunnel error: {e}; reconnecting"),
        }
    }
}

async fn connect_with_backoff(local_port: u16, last_port: &mut Option<u16>) -> BoreClient {
    const MAX_BACKOFF: Duration = Duration::from_secs(60);
    let mut backoff = Duration::from_secs(1);
    loop {
        match BoreClient::new("localhost", local_port, "bore.pub", 0, None).await {
            Ok(c) => {
                let port = c.remote_port();
                if Some(port) != *last_port {
                    println!("public address: bore.pub:{port}");
                    *last_port = Some(port);
                } else {
                    println!("bore tunnel reconnected on bore.pub:{port}");
                }
                return c;
            }
            Err(e) => {
                eprintln!("bore reconnect failed: {e}; retrying in {:?}", backoff);
                tokio::time::sleep(backoff).await;
                backoff = (backoff * 2).min(MAX_BACKOFF);
            }
        }
    }
}
