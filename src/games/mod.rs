pub mod macros;
pub mod pong;
pub mod rand_num;

use std::pin::Pin;
use anyhow::Result;
use iroh::endpoint::{SendStream, RecvStream, Connection};
use ratatui::DefaultTerminal;
use crate::register_games;

/// Metadata about a game
#[derive(Clone, Debug)]
pub struct GameInfo {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub author: &'static str,
}

/// Game initializer function - creates and runs the game
pub type GameInitializer = fn(SendStream, RecvStream, Connection, bool, DefaultTerminal) 
    -> Pin<Box<dyn std::future::Future<Output = Result<()>> + Send>>;

/// Registry entry containing metadata and initializer
pub struct GameRegistry {
    pub info: GameInfo,
    pub initializer: GameInitializer,
}

// Register all games here - developers only need to add a new entry
register_games! {
    pong => {
        types: (PongGame, PongAction, PongState),
        id: "pong",
        name: "Pong",
        description: "Classic Pong game - competitive local multiplayer",
        author: "LanTerm Team"
    },
    rand_num => {
        types: (NumberGame, GuessAction, NumberState),
        id: "rand_num",
        name: "Number Guessing",
        description: "Guess the random number - turn-based strategy game",
        author: "LanTerm Team"
    }
}