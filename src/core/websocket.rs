/// WebSocket-based game transport - much simpler than TCP!
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{
    accept_async,
    tungstenite::protocol::Message
};
use futures_util::{SinkExt, StreamExt};
use serde::{Serialize, Deserialize};
use uuid::Uuid;

/// WebSocket game messages - much cleaner than custom protocol
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GameMessage<State, Input> {
    /// Server metadata (sent first on connection)
    GameMetadata { name: String, version: String, description: String },
    /// Player joining the game
    PlayerJoin { name: String },
    /// Player leaving the game  
    PlayerLeave,
    /// Input from player
    PlayerInput(Input),
    /// Game state update
    StateUpdate(State),
    /// Chat message or notification
    Message(String),
    /// Error message
    Error(String),
}

/// Player session info
#[derive(Debug, Clone)]
pub struct PlayerSession {
    pub name: String,
    pub sender: broadcast::Sender<String>,
}

/// WebSocket game server - event-driven by design!
pub struct WebSocketGameServer<State, Input> {
    listener: TcpListener,
    sessions: Arc<RwLock<HashMap<String, PlayerSession>>>,
    game_state: Arc<RwLock<State>>,
    state_broadcast: broadcast::Sender<State>,
    input_broadcast: broadcast::Sender<(String, Input)>, // (player_id, input)
    game_metadata: GameMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameMetadata {
    pub name: String,
    pub version: String,
    pub description: String,
}

impl<State, Input> WebSocketGameServer<State, Input>
where
    State: Clone + Send + Sync + Serialize + for<'de> Deserialize<'de> + 'static,
    Input: Clone + Send + Sync + Serialize + for<'de> Deserialize<'de> + 'static,
{
    pub async fn new(addr: &str, initial_state: State, metadata: GameMetadata) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let listener = TcpListener::bind(addr).await?;
        let (state_broadcast, _) = broadcast::channel(100);
        let (input_broadcast, _) = broadcast::channel(100);
        
        Ok(Self {
            listener,
            sessions: Arc::new(RwLock::new(HashMap::new())),
            game_state: Arc::new(RwLock::new(initial_state)),
            state_broadcast,
            input_broadcast,
            game_metadata: metadata,
        })
    }

    /// Start the WebSocket server - naturally event-driven!
    pub async fn run<F>(&mut self, mut input_handler: F) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
    where
        F: FnMut(&Input, &mut State, &str) -> String + Send + 'static,
    {
        println!("WebSocket game server listening on {}", self.listener.local_addr()?);
        
        // Spawn input handler task
        let mut input_rx = self.input_broadcast.subscribe();
        let game_state = Arc::clone(&self.game_state);
        let state_broadcast = self.state_broadcast.clone();
        let sessions = Arc::clone(&self.sessions);
        
        tokio::spawn(async move {
            while let Ok((player_id, input)) = input_rx.recv().await {
                let player_name = {
                    let sessions_guard = sessions.read().await;
                    sessions_guard.get(&player_id).map(|s| s.name.clone())
                };
                
                if let Some(name) = player_name {
                    let response = {
                        let mut state = game_state.write().await;
                        input_handler(&input, &mut *state, &name)
                    };
                    
                    // Broadcast updated state to all players
                    let current_state = game_state.read().await.clone();
                    let _ = state_broadcast.send(current_state);
                    
                    // Send response back to player if needed
                    if !response.is_empty() {
                        let sessions_guard = sessions.read().await;
                        if let Some(session) = sessions_guard.get(&player_id) {
                            let msg = GameMessage::<State, Input>::Message(response);
                            let json = serde_json::to_string(&msg).unwrap_or_default();
                            let _ = session.sender.send(json);
                        }
                    }
                }
            }
        });
        
        // Accept WebSocket connections
        while let Ok((stream, addr)) = self.listener.accept().await {
            println!("New connection from {}", addr);
            
            let sessions = Arc::clone(&self.sessions);
            let state_broadcast = self.state_broadcast.clone();
            let input_broadcast = self.input_broadcast.clone();
            let game_state = Arc::clone(&self.game_state);
            
            tokio::spawn(Self::handle_connection(
                stream, sessions, state_broadcast, input_broadcast, game_state, self.game_metadata.clone()
            ));
        }
        
        Ok(())
    }

    /// Handle individual WebSocket connection - pure events!
    async fn handle_connection(
        stream: TcpStream,
        sessions: Arc<RwLock<HashMap<String, PlayerSession>>>,
        state_broadcast: broadcast::Sender<State>,
        input_broadcast: broadcast::Sender<(String, Input)>,
        game_state: Arc<RwLock<State>>,
        metadata: GameMetadata,
    ) {
        let ws_stream = match accept_async(stream).await {
            Ok(ws) => ws,
            Err(e) => {
                eprintln!("Failed to accept WebSocket: {}", e);
                return;
            }
        };

        let (mut ws_sender, mut ws_receiver) = ws_stream.split();
        let player_id = Uuid::new_v4().to_string();
        let mut player_name: Option<String> = None;
        
        // Create broadcast channel for this player
        let (player_sender, mut player_receiver) = broadcast::channel::<String>(100);
        
        // Send game metadata first thing
        let metadata_msg = GameMessage::<State, Input>::GameMetadata {
            name: metadata.name.clone(),
            version: metadata.version.clone(),
            description: metadata.description.clone(),
        };
        if let Ok(json) = serde_json::to_string(&metadata_msg) {
            let _ = ws_sender.send(Message::Text(json)).await;
        }
        
        // Subscribe to state updates
        let mut state_rx = state_broadcast.subscribe();
        
        // Spawn task to forward state updates to this player
        let player_sender_clone = player_sender.clone();
        let state_task = tokio::spawn(async move {
            while let Ok(state) = state_rx.recv().await {
                let msg = GameMessage::<State, Input>::StateUpdate(state);
                if let Ok(json) = serde_json::to_string(&msg) {
                    let _ = player_sender_clone.send(json);
                }
            }
        });
        
        // Spawn task to send messages to WebSocket
        let sender_task = tokio::spawn(async move {
            while let Ok(msg) = player_receiver.recv().await {
                if ws_sender.send(Message::Text(msg)).await.is_err() {
                    break;
                }
            }
        });

        // Handle incoming WebSocket messages - event-driven!
        while let Some(msg_result) = ws_receiver.next().await {
            match msg_result {
                Ok(Message::Text(text)) => {
                    if let Ok(game_msg) = serde_json::from_str::<GameMessage<State, Input>>(&text) {
                        match game_msg {
                            GameMessage::PlayerJoin { name } => {
                                println!("Player '{}' joined", name);
                                
                                // Store session
                                let session = PlayerSession {
                                    name: name.clone(),
                                    sender: player_sender.clone(),
                                };
                                
                                sessions.write().await.insert(player_id.clone(), session);
                                player_name = Some(name);
                                
                                // For games like Battleship that need player joining to trigger state updates,
                                // auto-send a dummy input to trigger the game's add_player logic
                                // This is a temporary hack - in the future we might add a proper join hook
                                let dummy_input_json = r#"{"Fire":{"row":99,"col":99}}"#; // Invalid coords that will just add player
                                if let Ok(dummy_input) = serde_json::from_str(&dummy_input_json) {
                                    let _ = input_broadcast.send((player_id.clone(), dummy_input));
                                }
                                
                                // Send current state to new player
                                let current_state = game_state.read().await.clone();
                                let state_msg = GameMessage::<State, Input>::StateUpdate(current_state);
                                if let Ok(json) = serde_json::to_string(&state_msg) {
                                    let _ = player_sender.send(json);
                                }
                            }
                            
                            GameMessage::PlayerInput(input) => {
                                // Forward to input handler
                                let _ = input_broadcast.send((player_id.clone(), input));
                            }
                            
                            GameMessage::PlayerLeave => {
                                break;
                            }
                            
                            _ => {
                                // Handle other message types
                            }
                        }
                    }
                }
                Ok(Message::Close(_)) => {
                    break;
                }
                Err(e) => {
                    eprintln!("WebSocket error: {}", e);
                    break;
                }
                _ => {}
            }
        }
        
        // Cleanup when connection closes
        if let Some(name) = player_name {
            println!("Player '{}' disconnected", name);
            sessions.write().await.remove(&player_id);
        }
        
        state_task.abort();
        sender_task.abort();
    }
}