/// Generic WebSocket game hosting functionality
use std::marker::PhantomData;
use crate::core::websocket::{WebSocketGameServer, GameMetadata};
use crate::core::game::WebSocketGame;

/// Generic WebSocket game host
pub struct WebSocketGameHost<G: WebSocketGame> {
    _phantom: PhantomData<G>,
}

impl<G: WebSocketGame> WebSocketGameHost<G> {
    pub async fn start(addr: &str, game_name: &str, description: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let initial_state = G::new_game();
        
        let metadata = GameMetadata {
            name: game_name.to_string(),
            version: "1.0.0".to_string(),
            description: description.to_string(),
        };
        
        let mut server = WebSocketGameServer::new(addr, initial_state, metadata).await?;
        
        println!("✅ {} WebSocket server running on ws://{}", game_name, addr);
        
        server.run(|input: &G::Input, state: &mut G::State, player_name: &str| {
            G::handle_input(input, state, player_name)
        }).await
    }
}