use rand::Rng;
use socket_lobby_common::game::{Game, GameEffect, GameRegistry, PlayerId};
use socket_lobby_common::protocol::ServerMsg;
use std::collections::HashMap;
use tokio::sync::mpsc;

pub struct Lobby {
    registry: GameRegistry,
    pub rooms: HashMap<String, Room>,
    next_player_id: usize,
}

pub struct Room {
    pub game_name: String,
    pub password: Option<String>,
    pub players: Vec<PlayerHandle>,
    pub host: PlayerId,
    pub game: Option<Box<dyn Game>>,
    pub state: RoomState,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RoomState {
    Waiting,
    Playing,
}

pub struct PlayerHandle {
    pub id: usize,
    pub name: String,
    pub tx: mpsc::UnboundedSender<ServerMsg>,
}

impl Lobby {
    pub fn new(registry: GameRegistry) -> Self {
        Self {
            registry,
            rooms: HashMap::new(),
            next_player_id: 0,
        }
    }

    pub fn next_id(&mut self) -> usize {
        let id = self.next_player_id;
        self.next_player_id += 1;
        id
    }

    pub fn create_room(
        &mut self,
        game_name: &str,
        password: Option<String>,
        host_id: usize,
        host_name: String,
        host_tx: mpsc::UnboundedSender<ServerMsg>,
    ) -> Result<String, String> {
        if self.registry.create(game_name).is_none() {
            return Err(format!("unknown game: {game_name}"));
        }

        let code = self.generate_room_code();
        let room = Room {
            game_name: game_name.to_string(),
            password,
            players: vec![PlayerHandle {
                id: host_id,
                name: host_name,
                tx: host_tx,
            }],
            host: host_id,
            game: None,
            state: RoomState::Waiting,
        };
        self.rooms.insert(code.clone(), room);
        Ok(code)
    }

    /// Returns (existing_player_names, game_name, host_name) on success.
    pub fn join_room(
        &mut self,
        room_code: &str,
        password: Option<&str>,
        player_id: usize,
        player_name: String,
        player_tx: mpsc::UnboundedSender<ServerMsg>,
    ) -> Result<(Vec<String>, String, String), String> {
        let room = self
            .rooms
            .get_mut(room_code)
            .ok_or_else(|| format!("room {room_code} not found"))?;

        if let Some(ref room_pw) = room.password {
            match password {
                Some(pw) if pw == room_pw => {}
                _ => return Err("incorrect password".to_string()),
            }
        }

        if room.state != RoomState::Waiting {
            return Err("game already in progress".to_string());
        }

        let join_msg = ServerMsg::PlayerJoined {
            name: player_name.clone(),
        };
        for p in &room.players {
            let _ = p.tx.send(join_msg.clone());
        }

        let player_names: Vec<String> = room.players.iter().map(|p| p.name.clone()).collect();
        let game_name = room.game_name.clone();
        let host_name = room
            .players
            .iter()
            .find(|p| p.id == room.host)
            .map(|p| p.name.clone())
            .unwrap_or_default();

        room.players.push(PlayerHandle {
            id: player_id,
            name: player_name,
            tx: player_tx,
        });

        Ok((player_names, game_name, host_name))
    }

    pub fn leave_room(&mut self, room_code: &str, player_id: usize) {
        let should_remove = if let Some(room) = self.rooms.get_mut(room_code) {
            let name = room
                .players
                .iter()
                .find(|p| p.id == player_id)
                .map(|p| p.name.clone());

            room.players.retain(|p| p.id != player_id);

            if let Some(name) = name {
                if room.state == RoomState::Playing {
                    if let Some(game) = &mut room.game {
                        let effects = game.player_disconnected(&name);
                        apply_effects(room, effects);
                    }
                }

                let leave_msg = ServerMsg::PlayerLeft { name };
                for p in &room.players {
                    let _ = p.tx.send(leave_msg.clone());
                }

                // If the host left, pass host to the earliest-joined
                // remaining player (index 0, since players are appended
                // in join order).
                if room.host == player_id {
                    if let Some(new_host) = room.players.first() {
                        room.host = new_host.id;
                        let host_msg = ServerMsg::HostChanged {
                            name: new_host.name.clone(),
                        };
                        for p in &room.players {
                            let _ = p.tx.send(host_msg.clone());
                        }
                    }
                }
            }

            room.players.is_empty()
        } else {
            false
        };

        if should_remove {
            self.rooms.remove(room_code);
        }
    }

    pub fn start_game(&mut self, room_code: &str, player_id: usize) -> Result<(), String> {
        let room = self
            .rooms
            .get_mut(room_code)
            .ok_or("room not found")?;

        if room.host != player_id {
            return Err("only the host can start the game".to_string());
        }
        if room.state != RoomState::Waiting {
            return Err("game already started".to_string());
        }

        let game_name = room.game_name.clone();
        let mut game = self
            .registry
            .create(&game_name)
            .ok_or("failed to create game")?;

        let (min, max) = game.player_range();
        let count = room.players.len();
        if count < min || count > max {
            return Err(format!("need {min}-{max} players, have {count}"));
        }

        let names: Vec<String> = room.players.iter().map(|p| p.name.clone()).collect();
        game.set_player_names(names);
        let effects = game.start(count);
        room.game = Some(game);
        room.state = RoomState::Playing;

        let started_msg = ServerMsg::GameStarted {
            game: game_name,
        };
        for p in &room.players {
            let _ = p.tx.send(started_msg.clone());
        }

        apply_effects(room, effects);
        Ok(())
    }

    pub fn game_action(
        &mut self,
        room_code: &str,
        player_id: usize,
        action: serde_json::Value,
    ) -> Result<(), String> {
        let room = self
            .rooms
            .get_mut(room_code)
            .ok_or("room not found")?;

        if room.state != RoomState::Playing {
            return Err("no game in progress".to_string());
        }

        let player_idx = room
            .players
            .iter()
            .position(|p| p.id == player_id)
            .ok_or("player not in room")?;

        let effects = if let Some(game) = &mut room.game {
            game.handle_action(player_idx, action)
        } else {
            return Err("no game instance".to_string());
        };

        apply_effects(room, effects);
        Ok(())
    }

    fn generate_room_code(&self) -> String {
        let mut rng = rand::thread_rng();
        loop {
            let code: String = (0..4)
                .map(|_| rng.gen_range(b'A'..=b'Z') as char)
                .collect();
            if !self.rooms.contains_key(&code) {
                return code;
            }
        }
    }
}

fn apply_effects(room: &mut Room, effects: Vec<GameEffect>) {
    for effect in effects {
        match effect {
            GameEffect::Broadcast(msg) => {
                for p in &room.players {
                    let _ = p.tx.send(msg.clone());
                }
            }
            GameEffect::SendTo(player_idx, msg) => {
                if let Some(p) = room.players.get(player_idx) {
                    let _ = p.tx.send(msg);
                }
            }
            GameEffect::GameOver { winner } => {
                let winner_name = room
                    .players
                    .get(winner)
                    .map(|p| p.name.clone())
                    .unwrap_or_else(|| "unknown".to_string());
                let msg = ServerMsg::GameOver {
                    winner: winner_name,
                };
                for p in &room.players {
                    let _ = p.tx.send(msg.clone());
                }
                // Return to the lobby so players can join between games
                // and the host can start a new round.
                room.state = RoomState::Waiting;
                room.game = None;
            }
        }
    }
}
