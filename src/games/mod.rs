pub mod pong;
pub mod rand_num;

use crate::core::engine::Engine;
use crate::core::network::NetworkManager;
use iroh::endpoint::{SendStream, RecvStream, Connection};
use ratatui::DefaultTerminal;
use anyhow::Result;
use std::collections::HashMap;

// Launcher type alias for readability
type Launcher = fn(SendStream, RecvStream, Connection, bool, DefaultTerminal) -> 
    std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>>>>;

pub fn get_registry() -> HashMap<&'static str, Launcher> {
    let mut m: HashMap<&'static str, Launcher> = HashMap::new();

    // Pong Registration
    m.insert("pong", |s, r, c, h, t| Box::pin(async move {
        // Pass 'h' (is_host) into PongGame::new()
        let game = pong::PongGame::new(h); 
        let network = NetworkManager::new(s, r, c);
        Engine::new(game, network, h).run(t).await
    }));

    // Random Number Registration
    m.insert("rand", |s, r, c, h, t| Box::pin(async move {
        // Assuming NumberGame::new() also takes is_host based on previous turns
        let game = rand_num::NumberGame::new(); 
        let network = NetworkManager::new(s, r, c);
        Engine::new(game, network, h).run(t).await
    }));

    m
}