/// Generic game renderer trait for dependency injection
use crate::core::terminal::TerminalContext;

/// Auto-injected renderer trait - framework discovers this automatically
pub trait GameRenderer<State>: Send + Sync {
    /// Create a new renderer instance with injected dependencies
    fn new(player_name: String) -> Self where Self: Sized;
    
    /// Render game state using terminal context - no more manual \r handling!
    fn render(&self, state: &State, ctx: &mut TerminalContext);
}
