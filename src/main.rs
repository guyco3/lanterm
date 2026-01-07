use anyhow::Result;
use lanterm::core::{lobby, menu::{Menu, MenuChoice}};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize terminal for menu
    let mut terminal = ratatui::init();

    // Show interactive menu
    let mut menu = Menu::new();
    let (choice, game_id, join_config) = menu.run(&mut terminal)?;

    // Restore terminal before proceeding
    ratatui::restore();

    // Determine if host or join
    let is_host = matches!(choice, MenuChoice::Host);
    
    // Get peer_id if joining
    let peer_id = join_config.as_ref().map(|c| c.node_id.clone());

    // 1. LOBBY: Connect to Iroh and perform handshake
    // The handshake is handled at connection level (via datagrams) - clean and simple!
    println!("--- LANTERM LOBBY ---");
    let (send, recv, conn, discovered_game_id, local_id) = lobby::connect_iroh(is_host, peer_id, game_id).await?;
    println!("Lobby connection established!");

    // 2. Get the game from registry (client now knows which game from handshake)
    let game = lanterm::games::get_game(&discovered_game_id)
        .ok_or_else(|| anyhow::anyhow!("Game '{}' not found", discovered_game_id))?;

    println!("Starting game: {}", game.info.name);

    // 3. Initialize terminal for game rendering
    let terminal = ratatui::init();

    // 4. Call the game's initializer (no more handshake needed - already done!)
    (game.initializer)(send, recv, conn, is_host, terminal, local_id).await?;

    Ok(())
}