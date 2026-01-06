pub mod pong;
pub mod rand_num;

use std::pin::Pin;
use anyhow::Result;
use iroh::endpoint::{SendStream, RecvStream, Connection};
use ratatui::DefaultTerminal;

/// Metadata about a game
#[derive(Clone, Debug)]
pub struct GameInfo {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub author: &'static str,
}

/// Game runner function signature
/// Takes network components and terminal, returns a future that runs the game
pub type GameRunner = fn(SendStream, RecvStream, Connection, bool, DefaultTerminal) 
    -> Pin<Box<dyn std::future::Future<Output = Result<()>> + Send>>;

/// Registry entry containing metadata and runner
pub struct GameRegistry {
    pub info: GameInfo,
    pub runner: GameRunner,
}

/// Get all available games with their metadata and runners
pub fn get_all_games() -> Vec<GameRegistry> {
    vec![
        GameRegistry {
            info: GameInfo {
                id: "pong",
                name: "Pong",
                description: "Classic Pong game - competitive local multiplayer",
                author: "LanTerm Team",
            },
            runner: |send, recv, conn, is_host, terminal| {
                Box::pin(pong::run_game(send, recv, conn, is_host, terminal))
            },
        },
        GameRegistry {
            info: GameInfo {
                id: "rand_num",
                name: "Number Guessing",
                description: "Guess the random number - turn-based strategy game",
                author: "LanTerm Team",
            },
            runner: |send, recv, conn, is_host, terminal| {
                Box::pin(rand_num::run_game(send, recv, conn, is_host, terminal))
            },
        },
    ]
}

/// Get a game by ID
pub fn get_game(id: &str) -> Option<GameRegistry> {
    get_all_games().into_iter().find(|g| g.info.id == id)
}