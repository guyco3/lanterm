# 🕹️ LANTERM v2: Design Specification

**LANTERM** is a Rust framework for building peer-to-peer (P2P) multiplayer games that run entirely in the terminal. It leverages **Iroh** for seamless connectivity (even through firewalls) and **Ratatui** for high-performance terminal rendering.

*LANTERM = Latency-optimized Agnostic Network Terminal*

## 🏗️ System Architecture

The framework is divided into three distinct layers:

### 1. The Transport Layer (Iroh)

- **Identity**: Every player has a persistent `NodeId` (PublicKey).
- **Connectivity**: Uses QUIC over UDP. Supports NAT hole-punching and relaying (DERP) so games work over the internet without port forwarding.
- **Protocol**: Custom ALPN (Application-Layer Protocol Negotiation) `lanterm/v2`.

### 2. The Logic Layer (State Machine)

- **Authoritative Host**: One node acts as the "Host" running the `update()` loop.
- **Deterministic State**: The `GameState` is serialized (using Bincode or JSON) and synchronized across peers.

### 3. The View Layer (Ratatui)

- **Immediate Mode UI**: The framework provides a `Frame` to the game's renderer.
- **Double Buffering**: Ratatui calculates the "diff" between frames, sending only changed characters to the terminal to eliminate flickering.

## 🔧 The Core Trait API

This is what a developer implements to create a game.

```rust
pub trait LantermGame: Clone + Send + Sync + 'static {
    type State: Serialize + DeserializeOwned + Clone + Send + Sync + 'static;
    type Input: Serialize + DeserializeOwned + Clone + Send + Sync + 'static;

    const NAME: &'static str;
    const TICK_RATE_HZ: u32; // 0 for turn-based, 60 for real-time (Pong)

    /// Initialize the game state
    fn new_game() -> Self::State;

    /// Server-side: Update state based on time (Physics/Logic)
    fn update(state: &mut Self::State, delta_ms: u64);

    /// Server-side: Update state based on player input
    fn handle_input(state: &mut Self::State, input: Self::Input, player: NodeId);

    /// Client-side: Convert keystrokes to Game Input
    fn map_key(key: KeyEvent) -> Option<Self::Input>;
}

pub trait LantermRenderer<S> {
    /// Render the current state into the Ratatui Frame
    fn render(frame: &mut Frame, state: &S, local_node_id: NodeId);
}
```

## 🔄 The Game Loop (Tick Flow)

1. **Input**: Client captures `KeyEvent` → `map_key()` → Sends `Input` to Host via Iroh stream.
2. **Process**: Host receives `Input` → `handle_input()` updates the Master State.
3. **Tick**: Every `1/TICK_RATE_HZ`, the Host calls `update()` (for ball physics, timers, etc.).
4. **Sync**: Host broadcasts the updated `State` to all connected `NodeIds`.
5. **Draw**: Client receives `State` → `render()` draws the frame using Ratatui.

## 📦 Updated Tech Stack

| Component | Tool | Why? |
|-----------|------|------|
| **Networking** | Iroh | P2P hole-punching, QUIC performance, and encrypted Node Identities. |
| **Rendering** | Ratatui | Industry-standard TUI widgets, layouts, and flicker-free rendering. |
| **Async Runtime** | Tokio | Handles the Iroh event loop and game timers concurrently. |
| **Serialization** | Bincode | Faster and smaller than JSON for real-time state synchronization. |
| **CLI Inputs** | Crossterm | Integrated into Ratatui for raw-mode keyboard event handling. |

## 🛤️ Implementation Roadmap

### Phase 1: The P2P Handshake
Implement the Host and Join commands using Iroh. Instead of an IP address, the user will share a `NodeId` or a "Ticket" (a short string Iroh generates that encodes the connection info).

### Phase 2: Ratatui Integration
Create a `TerminalContext` that initializes stdout into Raw Mode and sets up the Ratatui backend. Create a "Lobby" widget that shows players' Node IDs while waiting for a game to start.

### Phase 3: The "Pong" Test
Build Pong as the first v2 game. This will stress-test the `update()` loop and the Iroh latency. If the ball moves smoothly between two terminals over different WiFi networks, the framework is a success.

## 🚢 Battleship in LANTERM v2

Battleship becomes even better with this architecture because the framework now handles the "boring" parts (lobby, connection, UI layout) much more effectively. Turn-based games like Battleship work as a "subset" of the high-speed architecture.

### How Battleship fits into the new "Tick" Architecture

- Set `TICK_RATE_HZ` to `0` (or a very low number)
- **Turn-Based Logic**: Since `update()` won't be moving physics objects, the game state only changes when `handle_input()` is called
- **The "Tick" becomes a "Sync"**: Every time a player fires, the Host updates the state and broadcasts it

### 1. The Logic (Simplified)

```rust
impl LantermGame for Battleship {
    const TICK_RATE_HZ: u32 = 0; // Turn-based

    fn handle_input(state: &mut Self::State, input: Self::Input, player_id: NodeId) {
        match input {
            BattleshipInput::Fire { row, col } => {
                if state.current_turn_node == player_id {
                    // 1. Process the shot
                    let result = state.opponent_board(player_id).fire(row, col);
                    // 2. Update message
                    state.last_action = format!("Node {} fired at {},{}", player_id, row, col);
                    // 3. Switch turns
                    state.switch_turn();
                }
            }
        }
    }
}
```

### 2. The Renderer (Professional)

Instead of manual loops and `print!` calls, use Ratatui's `Table` or `Canvas` widgets:

```rust
impl LantermRenderer<BattleshipState> for BattleshipRenderer {
    fn render(frame: &mut Frame, state: &BattleshipState, local_id: NodeId) {
        let chunks = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(frame.area());

        // Render your board on the left
        let my_board = self.build_board_widget(&state.my_board(local_id));
        frame.render_widget(my_board, chunks[0]);

        // Render opponent's board (fog of war) on the right
        let opponent_board = self.build_board_widget(&state.opponent_view(local_id));
        frame.render_widget(opponent_board, chunks[1]);
    }
}
```

### Why v2 Architecture is Better for Battleship

| Problem in v1 | Solution in v2 (Iroh + Ratatui) |
|----------------|----------------------------------|
| **Connection Jitter**: Manual TCP connections can drop or hang. | Iroh uses QUIC, which is much more resilient to "flaky" WiFi. |
| **Screen Tearing**: Clearing the screen with `\x1b[2J` causes a flicker. | Ratatui uses a buffer; it only updates the specific cells that change. |
| **Discovery**: You have to type `192.168.x.x`. | You use a `NodeID` or Iroh's discovery to find your friend automatically. |
| **UI Constraints**: Hard to add a side-chat or a log window. | Ratatui Layouts make it easy to add a "Battle Log" at the bottom. |

## 📦 Next Steps: Cargo.toml

Here is the Cargo.toml and the core "Skeleton" for LANTERM v2:

### Cargo.toml

```toml
[package]
name = "lanterm"
version = "0.2.0"
edition = "2021"

[dependencies]
# Networking
iroh = "0.29"
tokio = { version = "1.0", features = ["full"] }
futures-util = "0.3"

# UI & Terminal
ratatui = "0.26"
crossterm = { version = "0.27", features = ["event"] }

# Serialization
serde = { version = "1.0", features = ["derive"] }
bincode = "1.3"
serde_json = "1.0"

# Utilities
anyhow = "1.0"
tracing = "0.1"
tracing-subscriber = "0.3"
```

### The Skeleton (main.rs)

This code initializes the terminal into "Raw Mode", sets up an Iroh Endpoint, and prepares the "Tick" loop:

```rust
use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use iroh::Endpoint;
use ratatui::{prelude::*, widgets::*};
use std::{io::{self, stdout}, time::{Duration, Instant}};

#[tokio::main]
async fn main() -> Result<()> {
    // --- 1. SETUP TERMINAL ---
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    // --- 2. SETUP NETWORKING (IROH) ---
    // This creates your unique identity (NodeId)
    let endpoint = Endpoint::builder()
        .discovery_n0() // Allows P2P hole-punching
        .bind()
        .await?;
    
    let me = endpoint.node_id();
    
    // --- 3. GAME LOOP ---
    let tick_rate = Duration::from_millis(16); // ~60 FPS
    let mut last_tick = Instant::now();

    loop {
        // Draw UI
        terminal.draw(|f| ui(f, me))?;

        // Handle Input
        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => break, // Quit
                    _ => {
                        // Send key to your Game Logic handle_input here
                    }
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            // Run your Game Logic update() here
            last_tick = Instant::now();
        }
    }

    // --- 4. CLEANUP ---
    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}

fn ui(f: &mut Frame, node_id: iroh::NodeId) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(f.area());

    let header = Paragraph::new(format!("LANTERM v2 | My Node ID: {}", node_id))
        .block(Block::default().borders(Borders::ALL).title("Status"))
        .style(Style::default().fg(Color::Cyan));
    
    f.render_widget(header, chunks[0]);

    let body = Paragraph::new("Press 'q' to quit. Waiting for peers...")
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).title("Game Console"));
    
    f.render_widget(body, chunks[1]);
}
```

## 🎯 How to Handle Battleship in This New Setup

To move your Battleship game into this:

1. **State Sync**: When a player fires, the "Host" node will use `endpoint.connect(peer_node_id)` to open a stream and send the updated `BattleshipState` serialized with Bincode.

2. **Rendering**: Instead of the `ui` function above just showing "Waiting for peers," you would call `render_battleship_board(f, &state, chunks[1])`.

3. **Discovery**: You can use Iroh's "Gossip" or just copy-paste the `NodeId` (the long string in the header) to your friend.

## 🚀 Why This is a "v2"

- **The UI is stable**: No more flickering or weird spaces because Ratatui controls the drawing buffer.
- **The Network is P2P**: You can play with someone in a different city as long as they have your NodeId.
- **Error Handling**: If the network drops, Iroh handles the retry logic internally so your code stays clean.

---

*Ready to implement the Iroh "Connect" logic so one terminal can actually send a "Fire" command to the other?*