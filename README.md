# lanterm 🕹️

**lanterm** is a Rust framework for building **multiplayer terminal games**.

It provides:
- Clean abstractions for game logic, networking, and rendering
- TCP transport using Tokio for real-time multiplayer
- Interactive CLI with game selection (like Vite project selection)
- Easy game development - just implement the `Game` trait
- Terminal client helpers for smooth gameplay

## Quick Start

### Interactive Mode (Recommended)
```bash
cargo run
```
This will show you an interactive menu where you can:
- 🏠 Host a game (with game selection)
- 🔗 Join a game 
- 📋 List available games

### Command Line Mode

1) **List available games**
```bash
cargo run -- list
```

2) **Host a game** (with interactive selection)
```bash
cargo run -- host
```

3) **Host a specific game**
```bash
cargo run -- host --game hangman --addr 0.0.0.0:4000
```

4) **Join a game**
```bash
cargo run -- join 127.0.0.1:4000 --name alice
```

### Example: Playing Hangman

1) **Host**: In one terminal, run:
```bash
cargo run -- host --game hangman
# or just: cargo run
```

2) **Join**: In other terminals:
```bash
cargo run -- join 127.0.0.1:4000 --name alice
cargo run -- join 127.0.0.1:4000 --name bob
```

### Controls:
- Type letters to guess in Hangman
- `q` to quit any game

## Adding New Games

1. Create a new module in `src/games/your_game/`
2. Implement the `Game` trait:
```rust
use crate::core::game::Game;

pub struct YourGame {
    // game state
}

impl Game for YourGame {
    type State = YourGameState; // Serializable game state
    type Input = YourGameInput; // User input type
    
    fn new() -> Self { /* initialize */ }
    fn on_join(&mut self, player: PlayerId, name: String) { /* handle join */ }
    fn on_input(&mut self, player: PlayerId, input: Self::Input) { /* handle input */ }
    fn state(&self) -> Self::State { /* return current state */ }
    // ... other methods
}
```

3. Create a `GameFactory`:
```rust
use crate::core::registry::{GameFactory, GameMetadata};

pub struct YourGameFactory;

impl GameFactory for YourGameFactory {
    fn metadata(&self) -> GameMetadata {
        GameMetadata {
            name: "your-game".to_string(),
            description: "Description of your game".to_string(),
            min_players: 2,
            max_players: 4,
        }
    }
    
    async fn start_host(&self, addr: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Start your game server
    }
}
```

4. Register it in `src/games/mod.rs`:
```rust
pub fn create_default_registry() -> GameRegistry {
    let mut registry = GameRegistry::new();
    registry.register(HangmanFactory);
    registry.register(YourGameFactory); // Add this line
    registry
}
```

## Architecture

- **Core Framework**: Game trait, Transport trait, Host orchestrator
- **TCP Transport**: Real-time networking with JSON protocol  
- **Game Registry**: Dynamic game discovery and instantiation
- **Client Library**: Terminal UI helpers and networking
- **CLI Interface**: User-friendly game selection and management

## Contributing
See `games/` for examples. The framework handles all the networking and client management - you just focus on your game logic!