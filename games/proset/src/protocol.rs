use serde::{Deserialize, Serialize};

use crate::card::{Card, GameMode};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ProSetAction {
    SelectCards { indices: Vec<usize> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ProSetEvent {
    SetFound {
        player: String,
        indices: Vec<usize>,
        score: usize,
    },
    InvalidSet {
        player: String,
    },
    PlayerDisconnected {
        player: String,
    },
    CardsUpdated,
    GameFinished {
        elapsed_ms: u64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreEntry {
    pub name: String,
    pub score: usize,
    pub connected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProSetState {
    pub cards: Vec<Card>,
    pub deck_remaining: usize,
    pub scores: Vec<ScoreEntry>,
    pub in_game: bool,
    pub mode: GameMode,
    /// Server-side elapsed time in milliseconds since game start.
    pub elapsed_ms: u64,
}
