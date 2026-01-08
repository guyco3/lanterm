pub mod macros;
pub mod pong;
pub mod rand_num;
pub mod battleship;
pub mod poker;

use std::future::Future;
use std::pin::Pin;
use anyhow::Result;
use ratatui::DefaultTerminal;
use crate::core::network::{InLobby, NetworkManager};
use crate::register_games;

/// Metadata about a game
#[derive(Clone, Debug)]
pub struct GameInfo {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub author: &'static str,
}

/// Game initializer function - creates and runs the game, returning the lobby-ready manager
pub type GameInitializer = for<'a> fn(NetworkManager<InLobby>, bool, &'a mut DefaultTerminal)
    -> Pin<Box<dyn Future<Output = Result<NetworkManager<InLobby>>> + Send + 'a>>;

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
    },
    battleship => {
        types: (BattleshipGame, BattleAction, BattleState),
        id: "battleship",
        name: "Battleship",
        description: "Strategy naval combat - turn-based P2P",
        author: "LanTerm Team"
    },
    poker => {
        types: (PokerGame, PokerAction, PokerState),
        id: "poker",
        name: "Texas Hold'em",
        description: "N-player P2P Poker. Bluffs, bets, and cards.",
        author: "LanTerm Team"
    }
}