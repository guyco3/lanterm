// WebSocket Battleship example
use lanterm::{
    core::{
        websocket_host::WebSocketGameHost,
        renderer::GameRenderer,
    },
    games::battleship::{BattleshipGame, BattleshipRenderer},
};
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚢 Starting Battleship WebSocket Server...");
    
    // Create game host with dependency injection
    let renderer = BattleshipRenderer;
    let host = WebSocketGameHost::<BattleshipGame>::new("127.0.0.1:8081", renderer).await?;
    
    println!("🌐 Battleship server running on ws://127.0.0.1:8081");
    println!("⚔️  Players can connect and battle!");
    println!("💡 Send coordinates like '3,4' to fire at row 3, column 4");
    
    // Run the game host
    host.run().await?;
    
    Ok(())
}