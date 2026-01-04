use std::collections::HashMap;

use async_trait::async_trait;
use futures_util::StreamExt;
use serde::{Serialize, Deserialize};

use crate::core::game::WebSocketGame;
use crate::core::renderer::GameRenderer;
use crate::core::websocket_host::WebSocketGameHost;
use crate::client::websocket_client::WebSocketGameClient;

/// Metadata about a game - extracted from game trait constants
#[derive(Debug, Clone)]
pub struct GameMetadata {
    pub name: String,
    pub description: String,
    pub min_players: usize,
    pub max_players: usize,
}

/// WebSocket game message for detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DetectionMessage {
    GameMetadata { name: String, version: String, description: String },
    PlayerJoin { name: String },
    #[serde(other)]
    Other,
}

/// Auto-injected game registration - framework discovers everything from traits
#[async_trait]
pub trait GameRegistration: Send + Sync {
    /// Get metadata about this game
    fn metadata(&self) -> GameMetadata;
    
    /// Start hosting this game - framework auto-injects
    async fn start_host(&self, addr: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    
    /// Start client for this game - framework auto-injects renderer
    async fn start_client(&self, addr: &str, name: String) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
}

/// Generic game registration that auto-injects renderer only
pub struct GenericGameRegistration<G, R> {
    _phantom: std::marker::PhantomData<(G, R)>,
}

impl<G, R> GenericGameRegistration<G, R>
where
    G: WebSocketGame + Send + Sync + 'static,
    R: GameRenderer<G::State> + 'static,
{
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}

#[async_trait]
impl<G, R> GameRegistration for GenericGameRegistration<G, R>
where
    G: WebSocketGame + Send + Sync + 'static,
    R: GameRenderer<G::State> + 'static,
{
    /// Extract metadata from game trait constants - no factory needed!
    fn metadata(&self) -> GameMetadata {
        GameMetadata {
            name: G::NAME.to_string(),
            description: G::DESCRIPTION.to_string(),
            min_players: G::MIN_PLAYERS,
            max_players: G::MAX_PLAYERS,
        }
    }
    
    /// Framework auto-injects game hosting using trait constants
    async fn start_host(&self, addr: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        println!("🚀 Starting {} WebSocket server...", G::NAME);
        WebSocketGameHost::<G>::start(
            addr, 
            G::NAME, 
            G::DESCRIPTION
        ).await
    }
    
    /// Framework auto-injects client with renderer - game controls input parsing
    async fn start_client(&self, addr: &str, name: String) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let server_url = if addr.starts_with("ws://") || addr.starts_with("wss://") {
            addr.to_string()
        } else {
            format!("ws://{}", addr)
        };
        
        let mut client = WebSocketGameClient::new(name.clone());
        
        // Auto-inject renderer only - game controls input parsing!
        let renderer = R::new(name.clone());
        
        println!("🎮 Connecting to {} WebSocket game! Press 'q' to quit.", G::NAME);
        
        client.connect_and_play::<G::State, G::Input, _, _>(
            &server_url,
            move |state: &G::State| {
                // Auto-injected renderer handles all UI
                renderer.render(state);
            },
            |line: &str| -> Option<G::Input> {
                // Game developer controls input parsing
                G::parse_line(line)
            }
        ).await
    }
}

/// Registry of auto-injected games - no factories!
pub struct GameRegistry {
    games: HashMap<String, Box<dyn GameRegistration>>,
}

impl GameRegistry {
    pub fn new() -> Self {
        Self {
            games: HashMap::new(),
        }
    }

    /// Register a game with auto-injection - only renderer, game controls input parsing!
    pub fn register_game<G, R>(&mut self)
    where
        G: WebSocketGame + Send + Sync + 'static,
        R: GameRenderer<G::State> + 'static,
    {
        let registration = GenericGameRegistration::<G, R>::new();
        let name = G::NAME.to_string(); // Get name from trait constant
        self.games.insert(name, Box::new(registration));
    }

    /// Get all available games
    pub fn list_games(&self) -> Vec<GameMetadata> {
        self.games.values().map(|r| r.metadata()).collect()
    }

    /// Start a game - framework auto-injects hosting
    pub async fn start_game(&self, name: &str, addr: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        match self.games.get(name) {
            Some(registration) => registration.start_host(addr).await,
            None => Err(format!("Game '{}' not found", name).into()),
        }
    }
    
    /// Join game - framework auto-injects client with renderer
    pub async fn join_game(&self, name: &str, addr: &str, player_name: String) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        match self.games.get(name) {
            Some(registration) => registration.start_client(addr, player_name).await,
            None => Err(format!("Game '{}' not found", name).into()),
        }
    }

    /// Auto-detect and join - framework handles everything
    pub async fn auto_detect_and_join(&self, addr: &str, player_name: String) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let ws_url = if addr.starts_with("ws://") || addr.starts_with("wss://") {
            addr.to_string()
        } else {
            format!("ws://{}", addr)
        };
        
        println!("🔍 Detecting game type at {}...", ws_url);
        
        let (ws_stream, _) = tokio_tungstenite::connect_async(&ws_url).await
            .map_err(|e| format!("Failed to connect to {}: {}", ws_url, e))?;
            
        let (mut _ws_sender, mut ws_receiver) = ws_stream.split();
        
        if let Some(msg_result) = ws_receiver.next().await {
            match msg_result {
                Ok(tokio_tungstenite::tungstenite::protocol::Message::Text(text)) => {
                    if let Ok(detection_msg) = serde_json::from_str::<DetectionMessage>(&text) {
                        if let DetectionMessage::GameMetadata { name, version, description } = detection_msg {
                            println!("✨ Detected game: {} v{} - {}", name, version, description);
                            
                            // Auto-inject client for detected game
                            return self.join_game(&name, addr, player_name).await;
                        }
                    }
                }
                Ok(_) => {},
                Err(e) => return Err(format!("WebSocket error: {}", e).into()),
            }
        }
        
        Err("Failed to detect game type from server".into())
    }

    /// Check if a game exists
    pub fn has_game(&self, name: &str) -> bool {
        self.games.contains_key(name)
    }
}

impl Default for GameRegistry {
    fn default() -> Self {
        Self::new()
    }
}