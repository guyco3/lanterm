pub mod game;
pub use game::{NumberGame, NumberState, GuessAction};

use crate::core::{engine::Engine, network::NetworkManager};
use crate::core::network::InternalMsg;
use anyhow::Result;
use iroh::endpoint::{SendStream, RecvStream, Connection};
use ratatui::DefaultTerminal;

/// Game runner for Number Guessing Game
pub async fn run_game(
    send: SendStream,
    recv: RecvStream,
    conn: Connection,
    is_host: bool,
    terminal: DefaultTerminal,
) -> Result<()> {
    let game = NumberGame::new(is_host);
    let network = NetworkManager::<InternalMsg<GuessAction, NumberState>>::new(send, recv, conn);
    let engine = Engine::new(game, network, is_host);
    engine.run(terminal).await
}