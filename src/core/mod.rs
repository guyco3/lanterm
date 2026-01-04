pub mod game;
pub mod registry;
pub mod renderer;  // Auto-injection traits
pub mod terminal;  // Terminal context wrapper

// WebSocket-based architecture (clean and event-driven!)
pub mod websocket;
pub mod websocket_host;
