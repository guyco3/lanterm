/// Core game interface for the lanterm v2 framework
use serde::{Serialize, Deserialize};

use iroh::EndpointId;

// Type alias for better readability
pub type NodeId = EndpointId;

/// Main game trait that all games must implement
/// This provides a standard interface for P2P multiplayer games using Iroh and Ratatui
pub trait LantermGame: Clone + Send + Sync + 'static {
    /// Game state that gets serialized and synchronized across peers
    type State: Serialize + for<'de> Deserialize<'de> + Clone + Send + Sync + 'static;
    
    /// Input type that clients send to the host
    type Input: Serialize + for<'de> Deserialize<'de> + Clone + Send + Sync + 'static;
    
    /// Initialize the game state
    fn new_game() -> Self::State;
    
    /// Server-side: Update state based on player input
    fn handle_input(state: &mut Self::State, input: Self::Input, player: NodeId);
}

/// Renderer trait for drawing game state using Ratatui
pub trait LantermRenderer<S> {
    /// Render the current state into the Ratatui Frame
    fn render(frame: &mut ratatui::Frame, state: &S, local_node_id: NodeId);
}
