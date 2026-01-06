pub mod game;

pub use game::{PongGame, PongState, PongAction};

use crate::core::{engine::Engine, network::NetworkManager};
use crate::core::network::InternalMsg;
use anyhow::Result;
use iroh::endpoint::{SendStream, RecvStream, Connection};
use ratatui::DefaultTerminal;

/// Game runner for Pong
pub async fn run_game(
    send: SendStream,
    recv: RecvStream,
    conn: Connection,
    is_host: bool,
    terminal: DefaultTerminal,
) -> Result<()> {
    let game = PongGame::new(is_host);
    let network = NetworkManager::<InternalMsg<PongAction, PongState>>::new(send, recv, conn);
    let engine = Engine::new(game, network, is_host);
    engine.run(terminal).await
}