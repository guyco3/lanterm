pub mod hangman;
pub mod battleship;

use crate::core::registry::GameRegistry;
use crate::games::hangman::{HangmanGame, HangmanRenderer};
use crate::games::battleship::{BattleshipGame, BattleshipRenderer};

/// Create registry with auto-injected games - game controls input parsing!
pub fn create_default_registry() -> GameRegistry {
    let mut registry = GameRegistry::new();
    
    // Auto-inject renderer only - game controls input parsing!
    registry.register_game::<HangmanGame, HangmanRenderer>();
    registry.register_game::<BattleshipGame, BattleshipRenderer>();
    
    registry
}
