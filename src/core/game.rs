/// Core game interface for the lanterm framework
use serde::Serialize;

/// Main game trait that all games must implement
/// This provides a standard interface for WebSocket-based multiplayer games
pub trait WebSocketGame: Clone + Send + Sync + 'static {
    /// Game state that gets serialized and sent to clients
    type State: Clone + Send + Sync + Serialize + for<'de> serde::Deserialize<'de> + std::fmt::Debug + 'static;
    
    /// Input type that clients send to the server
    type Input: Clone + Send + Sync + Serialize + for<'de> serde::Deserialize<'de> + std::fmt::Debug + 'static;
    
    // Metadata as associated constants - no factory needed!
    const NAME: &'static str;
    const DESCRIPTION: &'static str;
    const MIN_PLAYERS: usize;
    const MAX_PLAYERS: usize;
    
    /// Create a new game instance with initial state
    fn new_game() -> Self::State;
    
    /// Handle player input and update game state
    /// Returns a message to send back to the player
    fn handle_input(input: &Self::Input, state: &mut Self::State, player_name: &str) -> String;
    
    /// Parse line input into game commands - game developer controls this
    fn parse_line(line: &str) -> Option<Self::Input>;
}
