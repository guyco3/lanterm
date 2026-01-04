pub mod network;
pub mod runner;

pub use runner::LantermEngine;

/// Internal engine events that bridge network and game logic
pub(crate) enum EngineEvent<I> {
    InputReceived(crate::core::game::NodeId, I),
    PlayerJoined(crate::core::game::NodeId),
    PlayerLeft(crate::core::game::NodeId),
}