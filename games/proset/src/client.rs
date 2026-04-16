use crossterm::event::{KeyCode, KeyEvent};
use std::time::Instant;

use crate::card::{Card, GameMode};
use crate::protocol::{ProSetAction, ProSetEvent, ProSetState, ScoreEntry};

const KEYS_ROW0: [char; 4] = ['q', 'w', 'e', 'r'];
const KEYS_ROW1: [char; 4] = ['a', 's', 'd', 'f'];
const KEYS_ROW2: [char; 4] = ['z', 'x', 'c', 'v'];

pub struct ProSetClient {
    pub mode: GameMode,
    pub cards: Vec<Card>,
    pub deck_remaining: usize,
    pub scores: Vec<ScoreEntry>,
    pub in_game: bool,
    pub selected: Vec<usize>,
    /// Server-side elapsed time (synced on each state update).
    pub server_elapsed_ms: u64,
    /// Local time when we last synced the server elapsed time.
    local_sync_time: Option<Instant>,
    /// Final elapsed time (set when game finishes).
    pub final_elapsed_ms: Option<u64>,
}

impl ProSetClient {
    pub fn new(mode: GameMode) -> Self {
        Self {
            mode,
            cards: Vec::new(),
            deck_remaining: 0,
            scores: Vec::new(),
            in_game: false,
            selected: Vec::new(),
            server_elapsed_ms: 0,
            local_sync_time: None,
            final_elapsed_ms: None,
        }
    }

    /// Get the current elapsed time in milliseconds, interpolated locally.
    pub fn elapsed_ms(&self) -> u64 {
        if let Some(final_ms) = self.final_elapsed_ms {
            return final_ms;
        }
        match self.local_sync_time {
            Some(t) => self.server_elapsed_ms + t.elapsed().as_millis() as u64,
            None => self.server_elapsed_ms,
        }
    }

    /// Format elapsed time as MM:SS or HH:MM:SS.
    pub fn elapsed_display(&self) -> String {
        let ms = self.elapsed_ms();
        let total_secs = ms / 1000;
        let h = total_secs / 3600;
        let m = (total_secs % 3600) / 60;
        let s = total_secs % 60;
        if h > 0 {
            format!("{h:02}:{m:02}:{s:02}")
        } else {
            format!("{m:02}:{s:02}")
        }
    }

    /// Apply a GameState update from the server.
    pub fn apply_state(&mut self, state: serde_json::Value) {
        if let Ok(s) = serde_json::from_value::<ProSetState>(state) {
            // Drop any selection whose card changed — e.g. an opponent
            // claimed a set that included one of our selected cards, so
            // that slot now holds a different card from the deck.
            self.selected.retain(|&i| {
                s.cards.get(i).copied() == self.cards.get(i).copied()
            });

            self.cards = s.cards;
            self.deck_remaining = s.deck_remaining;
            self.scores = s.scores;
            self.in_game = s.in_game;
            self.mode = s.mode;
            self.server_elapsed_ms = s.elapsed_ms;
            self.local_sync_time = if s.in_game { Some(Instant::now()) } else { None };
        }
    }

    /// Apply a GameEvent and return a human-readable log string.
    pub fn apply_event(&mut self, event: serde_json::Value) -> Option<String> {
        let e: ProSetEvent = serde_json::from_value(event).ok()?;
        let msg = match e {
            ProSetEvent::SetFound {
                player, score, ..
            } => {
                format!("{player} found a set! (score: {score})")
            }
            ProSetEvent::InvalidSet { player } => {
                format!("{player} submitted an invalid set")
            }
            ProSetEvent::PlayerDisconnected { player } => {
                format!("{player} disconnected")
            }
            ProSetEvent::CardsUpdated => "Cards updated".to_string(),
            ProSetEvent::GameFinished { elapsed_ms } => {
                self.final_elapsed_ms = Some(elapsed_ms);
                let secs = elapsed_ms / 1000;
                let mins = secs / 60;
                let secs = secs % 60;
                format!("Game finished in {mins}:{secs:02}")
            }
        };
        Some(msg)
    }

    /// Handle a key event. Returns an optional action to send to the server.
    pub fn handle_key(&mut self, key: KeyEvent) -> Option<serde_json::Value> {
        let c = match key.code {
            KeyCode::Char(c) => c.to_ascii_lowercase(),
            _ => return None,
        };

        let pos = if let Some(col) = KEYS_ROW0.iter().position(|&k| k == c) {
            Some((col, 0))
        } else if let Some(col) = KEYS_ROW1.iter().position(|&k| k == c) {
            Some((col, 1))
        } else if let Some(col) = KEYS_ROW2.iter().position(|&k| k == c) {
            Some((col, 2))
        } else {
            None
        };

        let (col, row) = pos?;

        let idx = match self.mode {
            GameMode::TwelveCard => {
                let i = col * 3 + row;
                if i < self.cards.len() { Some(i) } else { None }
            }
            GameMode::SevenCard => {
                let positions = [(0, 0), (1, 0), (2, 0), (3, 0), (0, 1), (1, 1), (2, 1)];
                positions
                    .iter()
                    .position(|&p| p == (col, row))
                    .filter(|&i| i < self.cards.len())
            }
        }?;

        // Toggle selection
        if let Some(pos) = self.selected.iter().position(|&i| i == idx) {
            self.selected.remove(pos);
        } else {
            self.selected.push(idx);
        }

        // Auto-submit logic
        let should_submit = match self.mode {
            GameMode::TwelveCard => self.selected.len() == 3,
            GameMode::SevenCard => {
                if self.selected.len() >= 2 {
                    let xor: u8 = self
                        .selected
                        .iter()
                        .map(|&i| self.cards[i])
                        .fold(0u8, |acc, c| acc ^ c);
                    xor == 0
                } else {
                    false
                }
            }
        };

        if should_submit {
            let indices = self.selected.clone();
            self.selected.clear();
            Some(serde_json::to_value(ProSetAction::SelectCards { indices }).unwrap())
        } else {
            None
        }
    }

    /// Get the key label for a card at the given index.
    pub fn key_for_index(&self, idx: usize) -> String {
        match self.mode {
            GameMode::TwelveCard => {
                let col = idx / 3;
                let row = idx % 3;
                let keys = [KEYS_ROW0, KEYS_ROW1, KEYS_ROW2];
                if row < 3 && col < 4 {
                    keys[row][col].to_string()
                } else {
                    String::new()
                }
            }
            GameMode::SevenCard => {
                let positions = [(0, 0), (1, 0), (2, 0), (3, 0), (0, 1), (1, 1), (2, 1)];
                if idx < positions.len() {
                    let (col, row) = positions[idx];
                    let keys = [KEYS_ROW0, KEYS_ROW1, KEYS_ROW2];
                    keys[row][col].to_string()
                } else {
                    String::new()
                }
            }
        }
    }
}
