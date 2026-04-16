use serde::{Deserialize, Serialize};

/// A ProSet card is a value 1-63 (6-bit number).
/// Each bit represents a colored dot.
pub type Card = u8;

/// Check if a selection of cards forms a valid set (XOR = 0).
pub fn is_valid_set(cards: &[Card]) -> bool {
    if cards.len() < 2 {
        return false;
    }
    cards.iter().fold(0u8, |acc, &c| acc ^ c) == 0
}

/// Check if any valid set exists among the given cards.
pub fn exists_set(cards: &[Card], mode: GameMode) -> bool {
    let nonzero: Vec<Card> = cards.iter().copied().filter(|&c| c > 0).collect();
    match mode {
        GameMode::TwelveCard => exists_set_3(&nonzero),
        GameMode::SevenCard => exists_set_any(&nonzero),
    }
}

/// Optimized check for 3-card sets (12-card mode).
fn exists_set_3(cards: &[Card]) -> bool {
    let mut pairs = [false; 64];
    for (i, &v) in cards.iter().enumerate() {
        if pairs[v as usize] {
            return true;
        }
        for &u in &cards[..i] {
            pairs[(u ^ v) as usize] = true;
        }
    }
    false
}

/// Check for any-size sets (7-card mode).
fn exists_set_any(cards: &[Card]) -> bool {
    let n = cards.len();
    for mask in 3u32..(1u32 << n) {
        if mask.count_ones() < 2 {
            continue;
        }
        let xor: u8 = (0..n)
            .filter(|&i| mask & (1 << i) != 0)
            .fold(0u8, |acc, i| acc ^ cards[i]);
        if xor == 0 {
            return true;
        }
    }
    false
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GameMode {
    TwelveCard,
    SevenCard,
}

impl GameMode {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "proset12" => Some(GameMode::TwelveCard),
            "proset7" => Some(GameMode::SevenCard),
            _ => None,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            GameMode::TwelveCard => "proset12",
            GameMode::SevenCard => "proset7",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            GameMode::TwelveCard => "ProSet 12-Card",
            GameMode::SevenCard => "ProSet 7-Card",
        }
    }
}
