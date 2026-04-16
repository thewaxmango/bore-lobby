use rand::seq::SliceRandom;
use socket_lobby_common::game::{Game, GameEffect, PlayerId};
use socket_lobby_common::protocol::ServerMsg;
use std::time::Instant;

use crate::card::{exists_set, is_valid_set, Card, GameMode};
use crate::protocol::{ProSetAction, ProSetEvent, ProSetState, ScoreEntry};

pub struct ProSetGame {
    mode: GameMode,
    cards: Vec<Card>,
    deck: Vec<Card>,
    scores: Vec<usize>,
    player_names: Vec<String>,
    connected: Vec<bool>,
    player_count: usize,
    in_game: bool,
    start_time: Option<Instant>,
}

impl ProSetGame {
    pub fn new(mode: GameMode) -> Self {
        Self {
            mode,
            cards: Vec::new(),
            deck: Vec::new(),
            scores: Vec::new(),
            player_names: Vec::new(),
            connected: Vec::new(),
            player_count: 0,
            in_game: false,
            start_time: None,
        }
    }

    fn fill_cards(&mut self) -> bool {
        match self.mode {
            GameMode::TwelveCard => self.fill_cards_12(),
            GameMode::SevenCard => self.fill_cards_7(),
        }
    }

    fn fill_cards_12(&mut self) -> bool {
        if self.deck.len() >= 3
            && (self.cards.len() <= 12 || !exists_set(&self.cards, self.mode))
        {
            for card in &mut self.cards {
                if *card == 0 {
                    if let Some(new_card) = self.deck.pop() {
                        *card = new_card;
                    }
                }
            }
        } else {
            self.cards.retain(|&c| c > 0);
        }

        while self.deck.len() >= 3
            && (!exists_set(&self.cards, self.mode) || self.cards.len() < 12)
        {
            for _ in 0..3 {
                if let Some(card) = self.deck.pop() {
                    self.cards.push(card);
                }
            }
        }

        exists_set(&self.cards, self.mode)
    }

    fn fill_cards_7(&mut self) -> bool {
        for card in &mut self.cards {
            if self.deck.is_empty() {
                break;
            }
            if *card == 0 {
                if let Some(new_card) = self.deck.pop() {
                    *card = new_card;
                }
            }
        }
        self.cards.retain(|&c| c > 0);

        exists_set(&self.cards, self.mode)
    }

    fn build_state(&self) -> ServerMsg {
        let elapsed_ms = self
            .start_time
            .map(|t| t.elapsed().as_millis() as u64)
            .unwrap_or(0);
        ServerMsg::GameState {
            state: serde_json::to_value(ProSetState {
                cards: self.cards.clone(),
                deck_remaining: self.deck.len(),
                scores: self
                    .player_names
                    .iter()
                    .zip(self.scores.iter())
                    .enumerate()
                    .map(|(i, (n, &s))| ScoreEntry {
                        name: n.clone(),
                        score: s,
                        connected: self.connected.get(i).copied().unwrap_or(true),
                    })
                    .collect(),
                in_game: self.in_game,
                mode: self.mode,
                elapsed_ms,
            })
            .unwrap(),
        }
    }

    fn broadcast_state(&self) -> Vec<GameEffect> {
        vec![GameEffect::Broadcast(self.build_state())]
    }

    fn find_winner(&self) -> PlayerId {
        self.scores
            .iter()
            .enumerate()
            .max_by_key(|(_, &s)| s)
            .map(|(i, _)| i)
            .unwrap_or(0)
    }
}

impl Game for ProSetGame {
    fn name(&self) -> &str {
        self.mode.display_name()
    }

    fn player_range(&self) -> (usize, usize) {
        (1, 10)
    }

    fn set_player_names(&mut self, names: Vec<String>) {
        self.player_names = names;
    }

    fn start(&mut self, player_count: usize) -> Vec<GameEffect> {
        self.player_count = player_count;
        if self.player_names.len() != player_count {
            self.player_names = (0..player_count)
                .map(|i| format!("Player {}", i + 1))
                .collect();
        }
        self.scores = vec![0; player_count];
        self.connected = vec![true; player_count];

        let mut deck: Vec<Card> = (1..=63).collect();
        let mut rng = rand::thread_rng();
        deck.shuffle(&mut rng);
        self.deck = deck;

        match self.mode {
            GameMode::TwelveCard => {
                self.cards = Vec::new();
            }
            GameMode::SevenCard => {
                self.cards = vec![0; 7];
            }
        }

        self.fill_cards();
        self.in_game = true;
        self.start_time = Some(Instant::now());

        self.broadcast_state()
    }

    fn handle_action(&mut self, player: PlayerId, action: serde_json::Value) -> Vec<GameEffect> {
        let action: ProSetAction = match serde_json::from_value(action) {
            Ok(a) => a,
            Err(_) => {
                return vec![GameEffect::SendTo(
                    player,
                    ServerMsg::Error {
                        message: "invalid action".to_string(),
                    },
                )];
            }
        };

        if !self.in_game {
            return vec![GameEffect::SendTo(
                player,
                ServerMsg::Error {
                    message: "game not in progress".to_string(),
                },
            )];
        }

        match action {
            ProSetAction::SelectCards { indices } => self.handle_select(player, indices),
        }
    }

    fn player_disconnected(&mut self, name: &str) -> Vec<GameEffect> {
        let mut effects = Vec::new();
        if let Some(idx) = self.player_names.iter().position(|n| n == name) {
            if let Some(slot) = self.connected.get_mut(idx) {
                if !*slot {
                    // Already marked — nothing new to announce.
                    return self.broadcast_state();
                }
                *slot = false;
            }
            effects.push(GameEffect::Broadcast(ServerMsg::GameEvent {
                event: serde_json::to_value(ProSetEvent::PlayerDisconnected {
                    player: name.to_string(),
                })
                .unwrap(),
            }));
        }
        effects.extend(self.broadcast_state());
        effects
    }
}

impl ProSetGame {
    fn handle_select(&mut self, player: PlayerId, indices: Vec<usize>) -> Vec<GameEffect> {
        let mut effects = Vec::new();

        if indices.is_empty()
            || indices.len() != indices.iter().collect::<std::collections::HashSet<_>>().len()
            || indices.iter().any(|&i| i >= self.cards.len())
        {
            return vec![GameEffect::SendTo(
                player,
                ServerMsg::Error {
                    message: "invalid selection".to_string(),
                },
            )];
        }

        match self.mode {
            GameMode::TwelveCard => {
                if indices.len() != 3 {
                    return vec![GameEffect::SendTo(
                        player,
                        ServerMsg::Error {
                            message: "must select exactly 3 cards".to_string(),
                        },
                    )];
                }
            }
            GameMode::SevenCard => {
                if indices.len() < 2 || indices.len() > 7 {
                    return vec![GameEffect::SendTo(
                        player,
                        ServerMsg::Error {
                            message: "must select 2-7 cards".to_string(),
                        },
                    )];
                }
            }
        }

        let selected_cards: Vec<Card> = indices.iter().map(|&i| self.cards[i]).collect();
        if !is_valid_set(&selected_cards) {
            effects.push(GameEffect::Broadcast(ServerMsg::GameEvent {
                event: serde_json::to_value(ProSetEvent::InvalidSet {
                    player: self.player_names[player].clone(),
                })
                .unwrap(),
            }));
            return effects;
        }

        self.scores[player] += 1;

        effects.push(GameEffect::Broadcast(ServerMsg::GameEvent {
            event: serde_json::to_value(ProSetEvent::SetFound {
                player: self.player_names[player].clone(),
                indices: indices.clone(),
                score: self.scores[player],
            })
            .unwrap(),
        }));

        for &i in &indices {
            self.cards[i] = 0;
        }

        if !self.fill_cards() {
            let elapsed = self
                .start_time
                .map(|t| t.elapsed().as_millis() as u64)
                .unwrap_or(0);

            self.in_game = false;

            effects.push(GameEffect::Broadcast(ServerMsg::GameEvent {
                event: serde_json::to_value(ProSetEvent::GameFinished { elapsed_ms: elapsed })
                    .unwrap(),
            }));

            let winner = self.find_winner();
            effects.push(GameEffect::GameOver { winner });
        }

        effects.extend(self.broadcast_state());
        effects
    }
}
