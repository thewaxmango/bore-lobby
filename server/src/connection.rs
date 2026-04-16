use socket_lobby_common::protocol::{read_msg, write_msg, ClientMsg, ServerMsg};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::sync::{mpsc, Mutex};
use tokio::time::timeout;

use crate::lobby::Lobby;

const IDLE_TIMEOUT: Duration = Duration::from_secs(60);

pub async fn handle_connection(stream: TcpStream, lobby: Arc<Mutex<Lobby>>) {
    let addr = stream
        .peer_addr()
        .map(|a| a.to_string())
        .unwrap_or_else(|_| "unknown".into());
    println!("[+] {addr} connected");

    let (mut reader, mut writer) = stream.into_split();
    let (tx, mut rx) = mpsc::unbounded_channel::<ServerMsg>();

    // Writer task: forward messages from channel to TCP
    let write_handle = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if write_msg(&mut writer, &msg).await.is_err() {
                break;
            }
        }
    });

    // Assign player ID
    let player_id = {
        let mut lobby = lobby.lock().await;
        lobby.next_id()
    };

    let _ = tx.send(ServerMsg::Welcome { player_id });

    let mut player_name = format!("Player {player_id}");
    let mut current_room: Option<String> = None;

    // Reader loop: TCP -> process
    loop {
        let msg: ClientMsg = match timeout(IDLE_TIMEOUT, read_msg(&mut reader)).await {
            Ok(Ok(m)) => m,
            Ok(Err(_)) => break, // connection error
            Err(_) => {
                println!("[!] {addr} timed out (idle {IDLE_TIMEOUT:?})");
                let _ = tx.send(ServerMsg::Error {
                    message: "idle timeout".to_string(),
                });
                break;
            }
        };

        match msg {
            ClientMsg::SetName { name } => {
                player_name = name;
            }

            ClientMsg::CreateRoom { game, password } => {
                let mut lobby = lobby.lock().await;
                match lobby.create_room(&game, password, player_id, player_name.clone(), tx.clone()) {
                    Ok(code) => {
                        current_room = Some(code.clone());
                        let _ = tx.send(ServerMsg::RoomCreated { room_code: code });
                    }
                    Err(e) => {
                        let _ = tx.send(ServerMsg::Error { message: e });
                    }
                }
            }

            ClientMsg::JoinRoom { room_code, password } => {
                let code = room_code.to_uppercase();
                let mut lobby = lobby.lock().await;
                match lobby.join_room(&code, password.as_deref(), player_id, player_name.clone(), tx.clone()) {
                    Ok((existing_players, game_name, host_name)) => {
                        current_room = Some(code.clone());
                        let mut players = existing_players;
                        players.push(player_name.clone());
                        let _ = tx.send(ServerMsg::RoomJoined {
                            room_code: code,
                            players,
                            game: game_name,
                            host: host_name,
                        });
                    }
                    Err(e) => {
                        let _ = tx.send(ServerMsg::Error { message: e });
                    }
                }
            }

            ClientMsg::StartGame => {
                if let Some(ref code) = current_room {
                    let mut lobby = lobby.lock().await;
                    // Set player names in the game before starting
                    if let Err(e) = lobby.start_game(code, player_id) {
                        let _ = tx.send(ServerMsg::Error { message: e });
                    }
                } else {
                    let _ = tx.send(ServerMsg::Error {
                        message: "not in a room".to_string(),
                    });
                }
            }

            ClientMsg::GameAction { action } => {
                if let Some(ref code) = current_room {
                    let mut lobby = lobby.lock().await;
                    if let Err(e) = lobby.game_action(code, player_id, action) {
                        let _ = tx.send(ServerMsg::Error { message: e });
                    }
                } else {
                    let _ = tx.send(ServerMsg::Error {
                        message: "not in a room".to_string(),
                    });
                }
            }

            ClientMsg::LeaveRoom => {
                if let Some(code) = current_room.take() {
                    let mut lobby = lobby.lock().await;
                    lobby.leave_room(&code, player_id);
                }
            }

            ClientMsg::Ping => {
                // Resets the idle timeout — nothing else to do
            }
        }
    }

    // Cleanup on disconnect
    println!("[-] {addr} disconnected");
    if let Some(code) = current_room {
        let mut lobby = lobby.lock().await;
        lobby.leave_room(&code, player_id);
    }
    write_handle.abort();
}
