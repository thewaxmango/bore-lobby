use crate::protocol::ServerMsg;
use std::collections::HashMap;

pub type PlayerId = usize;

pub enum GameEffect {
    Broadcast(ServerMsg),
    SendTo(PlayerId, ServerMsg),
    GameOver { winner: PlayerId },
}

pub trait Game: Send {
    fn name(&self) -> &str;
    fn player_range(&self) -> (usize, usize);
    fn set_player_names(&mut self, names: Vec<String>);
    fn start(&mut self, player_count: usize) -> Vec<GameEffect>;
    fn handle_action(&mut self, player: PlayerId, action: serde_json::Value) -> Vec<GameEffect>;
    fn player_disconnected(&mut self, name: &str) -> Vec<GameEffect>;
}

pub type GameFactory = Box<dyn Fn() -> Box<dyn Game> + Send + Sync>;

pub struct GameRegistry {
    games: HashMap<String, (GameFactory, GameInfo)>,
}

#[derive(Debug, Clone)]
pub struct GameInfo {
    pub id: String,
    pub display_name: String,
}

impl GameRegistry {
    pub fn new() -> Self {
        Self {
            games: HashMap::new(),
        }
    }

    pub fn register(&mut self, id: &str, display_name: &str, factory: GameFactory) {
        self.games.insert(
            id.to_string(),
            (
                factory,
                GameInfo {
                    id: id.to_string(),
                    display_name: display_name.to_string(),
                },
            ),
        );
    }

    pub fn create(&self, id: &str) -> Option<Box<dyn Game>> {
        self.games.get(id).map(|(factory, _)| factory())
    }

    pub fn list_games(&self) -> Vec<GameInfo> {
        self.games.values().map(|(_, info)| info.clone()).collect()
    }
}
