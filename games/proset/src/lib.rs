pub mod card;
pub mod client;
pub mod engine;
pub mod protocol;
pub mod ui;

use bore_lobby_common::game::GameRegistry;

/// Register all ProSet game variants with the server-side registry.
pub fn register_games(registry: &mut GameRegistry) {
    registry.register(
        "proset12",
        "ProSet 12-Card",
        Box::new(|| Box::new(engine::ProSetGame::new(card::GameMode::TwelveCard))),
    );
    registry.register(
        "proset7",
        "ProSet 7-Card",
        Box::new(|| Box::new(engine::ProSetGame::new(card::GameMode::SevenCard))),
    );
}

/// Create a client-side game instance for the given game ID.
pub fn create_client_game(game_id: &str) -> Option<client::ProSetClient> {
    let mode = card::GameMode::from_str(game_id)?;
    Some(client::ProSetClient::new(mode))
}

/// List available game modes with (id, display_name).
pub fn available_games() -> Vec<(&'static str, &'static str)> {
    vec![
        ("proset12", "ProSet 12-Card"),
        ("proset7", "ProSet 7-Card"),
    ]
}
