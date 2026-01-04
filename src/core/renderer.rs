/// Generic game renderer trait for dependency injection

/// Auto-injected renderer trait - framework discovers this automatically
pub trait GameRenderer<State>: Send + Sync {
    /// Create a new renderer instance with injected dependencies
    fn new(player_name: String) -> Self where Self: Sized;
    
    /// Render game state - pure UI concerns
    fn render(&self, state: &State);
}
