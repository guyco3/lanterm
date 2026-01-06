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

    // 1. LOBBY: Connect to Iroh
    println!("--- LANTERM LOBBY ---");
    let (send, recv, conn) = lobby::connect_iroh(is_host, peer_id).await?;

    // 2. Get the game from registry
    let game = lanterm::games::get_game(&game_id)
        .ok_or_else(|| anyhow::anyhow!("Game '{}' not found", game_id))?;

    // 3. Initialize terminal for game rendering
    let terminal = ratatui::init();

    // 4. Run the game (type information handled inside game module)
    (game.runner)(send, recv, conn, is_host, terminal).await?;

    Ok(())
}