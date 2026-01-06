use anyhow::{Result, anyhow};
use lanterm::core::lobby;
use lanterm::games;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    
    // ... Networking Setup (is_host, peer_id) ...
    let role = args.get(1).map(|s| s.as_str()).unwrap_or("host");
    let is_host = matches!(role, "host" | "--host");
    let peer_id = if is_host { None } else { args.get(2).cloned() };

    // 1. Get the game name from the user (via CLI for now)
    let selected_id = args.get(if is_host { 2 } else { 3 }).map(|s| s.as_str()).unwrap_or("pong");

    // 2. Look up the "Recipe" in the HashMap
    let registry = games::get_registry();
    let launcher = registry.get(selected_id)
        .ok_or_else(|| anyhow!("Game '{}' not found in registry!", selected_id))?;

    // 3. Connect and Launch
    let (send, recv, conn) = lobby::connect_iroh(is_host, peer_id).await?;
    let terminal = ratatui::init();

    // Run the chosen recipe
    let result = launcher(send, recv, conn, is_host, terminal).await;

    ratatui::restore();
    result
}