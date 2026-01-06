use anyhow::Result;
use lanterm::core::{engine::Engine, lobby, network::NetworkManager};
use lanterm::games::rand_num::NumberGame;
use lanterm::games::pong::PongGame;
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    
    let role = args.get(1).map(|s| s.as_str()).unwrap_or("host");
    let is_host = matches!(role, "host" | "--host");
    let peer_id = if is_host { None } else { args.get(2).cloned() };

    // 1. LOBBY: Updated to destructure 3 elements (send, recv, conn)
    println!("--- LANTERM LOBBY ---");
    let (send, recv, conn) = lobby::connect_iroh(is_host, peer_id).await?;

    // 2. SETUP: Pass the connection into the NetworkManager
    let network = NetworkManager::new(send, recv, conn);
    
    // The Shared World model doesn't need to know if it's a host during init anymore
    let game = PongGame::new(is_host);
    let terminal = ratatui::init();

    // 3. RUN: Pass is_host to the Engine so it knows whether to act as Authority
    let engine = Engine::new(game, network, is_host);
    engine.run(terminal).await?;

    Ok(())
}