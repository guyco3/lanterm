/// Hangman game module - clean and simple!
pub mod game;
pub mod renderer;

// Clean exports - game controls its own input parsing!
pub use game::{HangmanGame, HangmanState};
pub use renderer::HangmanRenderer;
