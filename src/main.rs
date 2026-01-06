use anyhow::Result;
use lanterm::core::{engine::Engine, lobby, network::NetworkManager};
use lanterm::games::rand_num::NumberGame;
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
	let args: Vec<String> = env::args().collect();
    
	// Simple Argument Parsing: host/client/join, or default to host
	let role = args.get(1).map(|s| s.as_str()).unwrap_or("host");
	let is_host = matches!(role, "host" | "--host");
	let peer_id = if is_host {
		None
	} else {
		args.get(2).cloned()
	};

	// 1. LOBBY: Connect via Iroh
	println!("--- LANTERM LOBBY ---");
	let (send, recv) = lobby::connect_iroh(is_host, peer_id).await?;

	// 2. SETUP: Init Framework
	let network = NetworkManager::new(send, recv);
	let game = NumberGame::new(is_host);
	let terminal = ratatui::init();

	// 3. RUN: Start Engine
	let engine = Engine::new(game, network);
	engine.run(terminal).await?;

	Ok(())
}
