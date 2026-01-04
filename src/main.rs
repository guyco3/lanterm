mod core;
mod engine;
mod games;

use crate::engine::LantermEngine;
use crate::games::battleship::{Battleship, BattleshipRenderer};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let engine = LantermEngine::new().await?;
    // Inject the Battleship logic and its Renderer into the Engine
    engine.run::<Battleship, BattleshipRenderer>().await
}
