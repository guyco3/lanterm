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

┌─────────────────────────────────────────────────────┐
│ main.rs: Creates Engine + Injects Game Type        │
│   engine.run::<Battleship, BattleshipRenderer>()   │
└────────────────────┬────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────┐
│ runner.rs: Main Event Loop (tokio::select!)        │
│                                                     │
│  ┌──────────────┐   ┌──────────────┐              │
│  │ Network Rx   │   │  User Input  │              │
│  │ (from peer)  │   │  (keyboard)  │              │
│  └──────┬───────┘   └──────┬───────┘              │
│         │                   │                       │
│         ├───────┐  ┌────────┤                      │
│         ▼       ▼  ▼        ▼                       │
│    ┌────────────────────────────┐                  │
│    │  G::handle_input()         │                  │
│    │  (Game trait method)       │                  │
│    └────────────┬───────────────┘                  │
│                 │                                   │
│                 ▼                                   │
│    Updates local game state                        │
│    (state lives in runner.rs)                      │
│                 │                                   │
│                 ▼                                   │
│    ┌────────────────────────────┐                  │
│    │  R::render()               │                  │
│    │  (Renderer draws UI)       │                  │
│    └────────────────────────────┘                  │
└─────────────────────────────────────────────────────┘

## Contributing
See `games/` for examples. The framework handles all the networking and client management - you just focus on your game logic!




more ideas

Yes, a shared-state/event model simplifies the mental load:

Treat the game as: (a) local input → events, (b) authoritative reducer updates shared state, (c) state snapshots/patches broadcast to all. The terminal just renders the latest state.
Keep a monotonic state_version (or turn counter). Every event carries the expected previous version; the authority applies in order and bumps the version, then broadcasts the new version plus the delta/result.
Peers: render from state, send events when allowed. If a peer has an older version, it drops or defers events until it catches up. If it receives a newer snapshot, it replaces its local state.
Authority choice: simplest is host-authoritative. Full peer-to-peer with vector clocks is possible but adds complexity; start with a single authority.
Message types: Event (e.g., FireShot/PlaceShip), Result/Delta (resolved outcome), optional periodic StateSnapshot (state + state_version) for resync.
Rendering: no need to juggle “local terminal vs network”; you just update state and call render(state) each loop.
Input gating: derive “can I act?” from state (whose turn, phase) rather than booleans like is_host.




also, ensure exactly once semantics for delivery becauyse

i guess it makes sense to broadcast and use mono increaing seq number because wheneerv action -> result

A sends action to B, B proresces aciton sends result to A, but A also sends result to irself , so might apply it twice if not careful


Yes, a shared-state/event model simplifies the mental load:

Treat the game as: (a) local input → events, (b) authoritative reducer updates shared state, (c) state snapshots/patches broadcast to all. The terminal just renders the latest state.
Keep a monotonic state_version (or turn counter). Every event carries the expected previous version; the authority applies in order and bumps the version, then broadcasts the new version plus the delta/result.
Peers: render from state, send events when allowed. If a peer has an older version, it drops or defers events until it catches up. If it receives a newer snapshot, it replaces its local state.
Authority choice: simplest is host-authoritative. Full peer-to-peer with vector clocks is possible but adds complexity; start with a single authority.
Message types: Event (e.g., FireShot/PlaceShip), Result/Delta (resolved outcome), optional periodic StateSnapshot (state + state_version) for resync.
Rendering: no need to juggle “local terminal vs network”; you just update state and call render(state) each loop.
Input gating: derive “can I act?” from state (whose turn, phase) rather than booleans like is_host.

QUIC (and iroh on top of it) already gives you reliability, ordering, congestion control, and retransmissions, so your current stack is fine. You’ll still want app-level idempotency (seq/turn numbers) to guard against retries/duplicates and to reject out-of-order actions. No need to switch protocols for “retries”; just keep your monotonic seq and idempotent apply logic