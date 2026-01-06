pub mod core {
	pub mod engine;
	pub mod network;
	pub mod lobby;
	pub mod game;
}

pub mod games;

// Re-export for convenience
pub use crate::core::game::{Context, Game};
