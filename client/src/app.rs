use crossterm::event::{KeyCode, KeyEvent};
use socket_lobby_common::protocol::{ClientMsg, ServerMsg};
use socket_lobby_proset::client::ProSetClient;

#[derive(Debug, Clone, PartialEq)]
pub enum Screen {
    NameEntry,
    MainMenu,
    GameSelect,
    CreatePassword,
    JoinCode,
    JoinPassword { room_code: String },
    Connecting,
    RoomLobby,
    InGame,
}

/// Wraps game-specific client state. Add variants here for new games.
pub enum ActiveGame {
    ProSet(ProSetClient),
}

impl ActiveGame {
    pub fn apply_state(&mut self, state: serde_json::Value) {
        match self {
            ActiveGame::ProSet(g) => g.apply_state(state),
        }
    }

    pub fn apply_event(&mut self, event: serde_json::Value) -> Option<String> {
        match self {
            ActiveGame::ProSet(g) => g.apply_event(event),
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Option<serde_json::Value> {
        match self {
            ActiveGame::ProSet(g) => g.handle_key(key),
        }
    }
}

pub struct App {
    pub screen: Screen,
    pub player_name: String,
    pub room_code: Option<String>,
    pub game_name: String,
    pub players: Vec<String>,
    pub input_buffer: String,
    pub status_message: Option<String>,
    pub should_quit: bool,
    pub is_host: bool,
    pub host_name: String,

    // Active game plugin
    pub active_game: Option<ActiveGame>,
    pub event_log: Vec<String>,
    pub winner: Option<String>,
    pub final_elapsed_ms: Option<u64>,

    // Available games for the menu
    pub available_games: Vec<(&'static str, &'static str)>,

    // Messages to send
    pub outgoing: Vec<ClientMsg>,
}

impl App {
    pub fn new() -> Self {
        // Collect games from all registered game crates
        let mut available_games = Vec::new();
        available_games.extend(socket_lobby_proset::available_games());

        Self {
            screen: Screen::NameEntry,
            player_name: String::new(),
            room_code: None,
            game_name: String::new(),
            players: Vec::new(),
            input_buffer: String::new(),
            status_message: None,
            should_quit: false,
            is_host: false,
            host_name: String::new(),
            active_game: None,
            event_log: Vec::new(),
            winner: None,
            final_elapsed_ms: None,
            available_games,
            outgoing: Vec::new(),
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        if key.code == KeyCode::Esc {
            match self.screen {
                Screen::NameEntry => self.should_quit = true,
                Screen::MainMenu => self.should_quit = true,
                Screen::GameSelect
                | Screen::CreatePassword
                | Screen::JoinCode
                | Screen::JoinPassword { .. }
                | Screen::Connecting => {
                    self.input_buffer.clear();
                    self.status_message = None;
                    self.screen = Screen::MainMenu;
                }
                Screen::RoomLobby => {
                    self.outgoing.push(ClientMsg::LeaveRoom);
                    self.screen = Screen::MainMenu;
                    self.room_code = None;
                    self.players.clear();
                    self.is_host = false;
                    self.host_name.clear();
                }
                Screen::InGame => {
                    if self.winner.is_some() {
                        // Game is over — return to lobby
                        self.active_game = None;
                        self.event_log.clear();
                        self.winner = None;
                        self.final_elapsed_ms = None;
                        self.screen = Screen::RoomLobby;
                    } else {
                        // Game still in progress — leave room
                        self.outgoing.push(ClientMsg::LeaveRoom);
                        self.reset_game_state();
                        self.screen = Screen::MainMenu;
                    }
                }
            }
            return;
        }

        match self.screen {
            Screen::NameEntry => self.handle_name_entry(key),
            Screen::MainMenu => self.handle_main_menu(key),
            Screen::GameSelect => self.handle_game_select(key),
            Screen::CreatePassword => self.handle_create_password(key),
            Screen::JoinCode => self.handle_join_code(key),
            Screen::JoinPassword { .. } => self.handle_join_password(key),
            Screen::Connecting => {}
            Screen::RoomLobby => self.handle_room_lobby(key),
            Screen::InGame => self.handle_in_game(key),
        }
    }

    fn reset_game_state(&mut self) {
        self.room_code = None;
        self.active_game = None;
        self.event_log.clear();
        self.winner = None;
        self.final_elapsed_ms = None;
        self.host_name.clear();
        self.is_host = false;
    }

    fn handle_name_entry(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Enter => {
                if !self.input_buffer.is_empty() {
                    self.player_name = self.input_buffer.clone();
                    self.input_buffer.clear();
                    self.outgoing.push(ClientMsg::SetName {
                        name: self.player_name.clone(),
                    });
                    self.screen = Screen::MainMenu;
                }
            }
            KeyCode::Char(c) => {
                if self.input_buffer.len() < 20 {
                    self.input_buffer.push(c);
                }
            }
            KeyCode::Backspace => {
                self.input_buffer.pop();
            }
            _ => {}
        }
    }

    fn handle_main_menu(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('c') | KeyCode::Char('C') => {
                self.screen = Screen::GameSelect;
                self.is_host = true;
            }
            KeyCode::Char('j') | KeyCode::Char('J') => {
                self.screen = Screen::JoinCode;
                self.input_buffer.clear();
            }
            _ => {}
        }
    }

    fn handle_game_select(&mut self, key: KeyEvent) {
        // Number keys select from available games list
        if let KeyCode::Char(c) = key.code {
            if let Some(digit) = c.to_digit(10) {
                let idx = digit as usize;
                if idx >= 1 && idx <= self.available_games.len() {
                    self.game_name = self.available_games[idx - 1].0.to_string();
                    self.screen = Screen::CreatePassword;
                    self.input_buffer.clear();
                }
            }
        }
    }

    fn handle_create_password(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Enter => {
                let password = if self.input_buffer.is_empty() {
                    None
                } else {
                    Some(self.input_buffer.clone())
                };
                self.input_buffer.clear();
                self.outgoing.push(ClientMsg::CreateRoom {
                    game: self.game_name.clone(),
                    password,
                });
            }
            KeyCode::Char(c) => {
                if self.input_buffer.len() < 32 {
                    self.input_buffer.push(c);
                }
            }
            KeyCode::Backspace => {
                self.input_buffer.pop();
            }
            _ => {}
        }
    }

    fn handle_join_code(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Enter => {
                if !self.input_buffer.is_empty() {
                    let code = self.input_buffer.clone().to_uppercase();
                    self.input_buffer.clear();
                    self.screen = Screen::JoinPassword { room_code: code };
                }
            }
            KeyCode::Char(c) => {
                if self.input_buffer.len() < 4 {
                    self.input_buffer.push(c.to_ascii_uppercase());
                }
            }
            KeyCode::Backspace => {
                self.input_buffer.pop();
            }
            _ => {}
        }
    }

    fn handle_join_password(&mut self, key: KeyEvent) {
        let room_code = match &self.screen {
            Screen::JoinPassword { room_code } => room_code.clone(),
            _ => return,
        };
        match key.code {
            KeyCode::Enter => {
                let password = if self.input_buffer.is_empty() {
                    None
                } else {
                    Some(self.input_buffer.clone())
                };
                self.input_buffer.clear();
                self.outgoing.push(ClientMsg::JoinRoom {
                    room_code,
                    password,
                });
                self.screen = Screen::Connecting;
            }
            KeyCode::Char(c) => {
                if self.input_buffer.len() < 32 {
                    self.input_buffer.push(c);
                }
            }
            KeyCode::Backspace => {
                self.input_buffer.pop();
            }
            _ => {}
        }
    }

    fn handle_room_lobby(&mut self, key: KeyEvent) {
        if key.code == KeyCode::Enter && self.is_host {
            self.outgoing.push(ClientMsg::StartGame);
        }
    }

    fn handle_in_game(&mut self, key: KeyEvent) {
        if self.winner.is_some() {
            return;
        }
        if let Some(game) = &mut self.active_game {
            if let Some(action) = game.handle_key(key) {
                self.outgoing.push(ClientMsg::GameAction { action });
            }
        }
    }

    /// Try to create an ActiveGame for the given game ID.
    fn create_client_game(game_id: &str) -> Option<ActiveGame> {
        // Try each registered game crate
        if let Some(client) = socket_lobby_proset::create_client_game(game_id) {
            return Some(ActiveGame::ProSet(client));
        }
        None
    }

    pub fn handle_server_msg(&mut self, msg: ServerMsg) {
        self.status_message = None;

        match msg {
            ServerMsg::Welcome { .. } => {}

            ServerMsg::RoomCreated { room_code } => {
                self.room_code = Some(room_code);
                self.players = vec![self.player_name.clone()];
                self.host_name = self.player_name.clone();
                self.is_host = true;
                self.screen = Screen::RoomLobby;
            }

            ServerMsg::RoomJoined {
                room_code,
                players,
                game,
                host,
            } => {
                self.room_code = Some(room_code);
                self.players = players;
                self.game_name = game;
                self.is_host = self.player_name == host;
                self.host_name = host;
                self.screen = Screen::RoomLobby;
            }

            ServerMsg::PlayerJoined { name } => {
                self.players.push(name);
            }

            ServerMsg::PlayerLeft { name } => {
                self.players.retain(|n| n != &name);
            }

            ServerMsg::HostChanged { name } => {
                self.is_host = self.player_name == name;
                self.host_name = name;
            }

            ServerMsg::GameStarted { game } => {
                self.game_name = game.clone();
                self.active_game = Self::create_client_game(&game);
                self.screen = Screen::InGame;
                self.event_log.clear();
                self.winner = None;
                self.final_elapsed_ms = None;
            }

            ServerMsg::GameState { state } => {
                if let Some(game) = &mut self.active_game {
                    game.apply_state(state);
                }
            }

            ServerMsg::GameEvent { event } => {
                if let Some(game) = &mut self.active_game {
                    if let Some(msg) = game.apply_event(event) {
                        self.event_log.push(msg);
                        if self.event_log.len() > 20 {
                            self.event_log.remove(0);
                        }
                    }
                }
            }

            ServerMsg::GameOver { winner } => {
                self.winner = Some(winner);
                self.final_elapsed_ms = match &self.active_game {
                    Some(ActiveGame::ProSet(g)) => g.final_elapsed_ms,
                    None => None,
                };
            }

            ServerMsg::Error { message } => {
                self.status_message = Some(format!("Error: {message}"));
                if self.screen == Screen::Connecting {
                    self.screen = Screen::JoinCode;
                }
            }
        }
    }
}
