mod core;
mod games;
mod game_runner;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use iroh::Endpoint;
use ratatui::{prelude::*, widgets::*};
use std::{io::{stdout}, time::{Duration, Instant}};

use core::game::NodeId;
use game_runner::GameRunner;
use games::battleship::{BattleshipGame, BattleshipRenderer, game::BattleshipInput};

#[derive(Debug, Clone, PartialEq)]
enum AppState {
    MainMenu,
    HostLobby,
    JoinGame,
    Playing,
}

#[derive(Debug, Clone, PartialEq)]
enum InputMode {
    Normal,
    EnteringNodeId,
    EnteringCoordinates { row_input: String, col_input: String, entering_col: bool },
}

#[derive(Debug)]
struct App {
    state: AppState,
    input_mode: InputMode,
    menu_index: usize,
    endpoint_id: NodeId,
    join_input: String,
    game_runner: Option<GameRunner<BattleshipGame, BattleshipRenderer>>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing to stderr to avoid corrupting the TUI on stdout
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .init();
    
    // --- 1. SETUP TERMINAL ---
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    terminal.clear()?;

    // --- 2. SETUP NETWORKING (IROH) ---
    let endpoint = Endpoint::builder()
        .alpns(vec![b"lanterm/battleship/0".to_vec()])
        .bind()
        .await?;
    let me = endpoint.id();
    
    let mut app = App {
        state: AppState::MainMenu,
        input_mode: InputMode::Normal,
        menu_index: 0,
        endpoint_id: me,
        join_input: String::new(),
        game_runner: None,
    };
    
    // --- 3. GAME LOOP ---
    let tick_rate = Duration::from_millis(16); // ~60 FPS
    let mut last_tick = Instant::now();

    let result = loop {
        // Check for auto-transition from lobby to playing
        if app.state == AppState::HostLobby {
            if let Some(runner) = &mut app.game_runner {
                // Try to accept a connection if we're the host
                if runner.is_host {
                    // accept_connection is internally time-bounded; call directly
                    let _ = runner.accept_connection().await;
                }
                
                if runner.game_state().players.len() == 2 || runner.runner_state() == &game_runner::GameRunnerState::Playing {
                    tracing::info!("host sees 2 players; entering Playing");
                    app.state = AppState::Playing;
                }
            }
        }

        // Client: poll for state syncs each tick
        if app.state == AppState::Playing {
            if let Some(runner) = &mut app.game_runner {
                if !runner.is_host {
                    let _ = runner.poll_state().await;
                }
            }
        }
        
        // Check if client in JoinGame state with game_runner (connecting)
        if app.state == AppState::JoinGame {
            if let Some(runner) = &app.game_runner {
                // Client is connected, check if host added us
                if runner.game_state().players.len() > 0 {
                    // Host has added us, transition to playing
                    app.state = AppState::Playing;
                    app.input_mode = InputMode::Normal;
                }
            }
        }
        
        // Draw UI
        terminal.draw(|f| ui(f, &app))?;

        // Handle Input
        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if handle_key_event(&mut app, key, &endpoint).await? {
                    break Ok(());
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
    };

    // --- 4. CLEANUP ---
    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;
    result
}

async fn handle_key_event(app: &mut App, key: event::KeyEvent, endpoint: &Endpoint) -> Result<bool> {
    match &app.input_mode {
        InputMode::Normal => handle_normal_mode(app, key, endpoint).await,
        InputMode::EnteringNodeId => handle_node_id_input(app, key, endpoint).await,
        InputMode::EnteringCoordinates { .. } => handle_coordinate_input(app, key).await,
    }
}

async fn handle_normal_mode(app: &mut App, key: event::KeyEvent, endpoint: &Endpoint) -> Result<bool> {
    match key.code {
        KeyCode::Char('q') => return Ok(true),
        KeyCode::Up => {
            if app.state == AppState::MainMenu && app.menu_index > 0 {
                app.menu_index -= 1;
            }
        },
        KeyCode::Down => {
            if app.state == AppState::MainMenu && app.menu_index < 2 {
                app.menu_index += 1;
            }
        },
        KeyCode::Enter => {
            match app.state {
                AppState::MainMenu => {
                    match app.menu_index {
                        0 => {
                            // Host game
                            let mut runner = GameRunner::new_host(endpoint.clone()).await?;
                            runner.add_local_player();
                            app.game_runner = Some(runner);
                            app.state = AppState::HostLobby;
                        },
                        1 => {
                            // Join game
                            app.state = AppState::JoinGame;
                            app.input_mode = InputMode::EnteringNodeId;
                            app.join_input.clear();
                        },
                        2 => return Ok(true),
                        _ => {}
                    }
                },
                _ => {}
            }
        },
        KeyCode::Char('f') => {
            // Fire command - only in playing state and on player's turn
            if app.state == AppState::Playing {
                if let Some(runner) = &app.game_runner {
                    if let Some(current_turn) = runner.game_state().current_turn_node {
                        if current_turn == app.endpoint_id {
                            // Start coordinate input
                            app.input_mode = InputMode::EnteringCoordinates {
                                row_input: String::new(),
                                col_input: String::new(),
                                entering_col: false,
                            };
                        }
                    }
                }
            }
        },
        KeyCode::Esc => {
            app.state = AppState::MainMenu;
            app.input_mode = InputMode::Normal;
            app.menu_index = 0;
            app.game_runner = None;
        },
        _ => {}
    }
    Ok(false)
}

async fn handle_node_id_input(app: &mut App, key: event::KeyEvent, endpoint: &Endpoint) -> Result<bool> {
    match key.code {
        KeyCode::Char(c) => {
            app.join_input.push(c);
        },
        KeyCode::Backspace => {
            app.join_input.pop();
        },
        KeyCode::Enter => {
            if !app.join_input.is_empty() {
                // Try to parse and trim whitespace
                let node_id_str = app.join_input.trim();
                
                match node_id_str.parse::<NodeId>() {
                    Ok(host_node_id) => {
                        match GameRunner::new_client(endpoint.clone(), host_node_id).await {
                            Ok(mut runner) => {
                                // For local testing: immediately set up 2-player game
                                // In real P2P, state would be synced from host
                                runner.add_remote_player(host_node_id); // Host as player 1
                                runner.add_local_player(); // Self as player 2
                                app.game_runner = Some(runner);
                                app.state = AppState::Playing;
                                app.input_mode = InputMode::Normal;
                            },
                            Err(_e) => {
                                // Connection failed
                                app.join_input = "Connection failed!".to_string();
                            }
                        }
                    },
                    Err(_e) => {
                        // Invalid node ID format
                        app.join_input = "Invalid Node ID!".to_string();
                    }
                }
            }
        },
        KeyCode::Esc => {
            app.state = AppState::MainMenu;
            app.input_mode = InputMode::Normal;
            app.join_input.clear();
        },
        _ => {}
    }
    Ok(false)
}

async fn handle_coordinate_input(app: &mut App, key: event::KeyEvent) -> Result<bool> {
    if let InputMode::EnteringCoordinates { ref mut row_input, ref mut col_input, ref mut entering_col } = app.input_mode {
        match key.code {
            KeyCode::Char(c) if c.is_ascii_digit() => {
                if *entering_col {
                    if col_input.len() < 1 {
                        col_input.push(c);
                    }
                } else {
                    if row_input.len() < 1 {
                        row_input.push(c);
                    }
                }
            },
            KeyCode::Backspace => {
                if *entering_col {
                    col_input.pop();
                } else {
                    row_input.pop();
                }
            },
            KeyCode::Enter | KeyCode::Char(' ') => {
                if !*entering_col && !row_input.is_empty() {
                    // Move to column input
                    *entering_col = true;
                } else if *entering_col && !col_input.is_empty() {
                    // Parse and fire
                    if let (Ok(row), Ok(col)) = (row_input.parse::<usize>(), col_input.parse::<usize>()) {
                        if row < 10 && col < 10 {
                            if let Some(runner) = &mut app.game_runner {
                                runner.handle_input(BattleshipInput::Fire { row, col });
                                if runner.is_host {
                                    let _ = runner.send_state().await;
                                }
                            }
                        }
                    }
                    app.input_mode = InputMode::Normal;
                }
            },
            KeyCode::Esc => {
                app.input_mode = InputMode::Normal;
            },
            _ => {}
        }
    }
    Ok(false)
}

fn ui(f: &mut Frame, app: &App) {
    match app.state {
        AppState::MainMenu => render_main_menu(f, app),
        AppState::HostLobby => render_host_lobby(f, app),
        AppState::JoinGame => render_join_game(f, app),
        AppState::Playing => render_game(f, app),
    }
}

fn render_main_menu(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            Constraint::Min(8),
            Constraint::Length(3),
        ])
        .split(f.area());

    // Header
    let short_id = format!("{}...", &app.endpoint_id.to_string()[0..16.min(app.endpoint_id.to_string().len())]);
    let header = Paragraph::new(vec![
        "🕹️ LANTERM v2 - Battleship".into(),
        format!("Node ID: {}", short_id).into(),
    ])
        .block(Block::default().borders(Borders::ALL).title(" Status "))
        .style(Style::default().fg(Color::Green));
    f.render_widget(header, chunks[0]);

    // Menu
    let menu_items = vec!["🎮 Host Battleship Game", "🔌 Join Game", "❌ Quit"];
    let items: Vec<ListItem> = menu_items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let style = if i == app.menu_index {
                Style::default().bg(Color::Blue).fg(Color::White)
            } else {
                Style::default().fg(Color::Gray)
            };
            ListItem::new(*item).style(style)
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(" Main Menu "))
        .style(Style::default().fg(Color::White));
    f.render_widget(list, chunks[1]);

    // Footer
    let footer = Paragraph::new("Arrow keys to navigate • Enter to select • 'q' to quit")
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).title(" Controls "))
        .style(Style::default().fg(Color::Yellow));
    f.render_widget(footer, chunks[2]);
}

fn render_host_lobby(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            Constraint::Min(8),
            Constraint::Length(3),
        ])
        .split(f.area());

    // Header
    let header = Paragraph::new("🚢 BATTLESHIP - Host Lobby")
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center);
    f.render_widget(header, chunks[0]);

    // Lobby info
    let player_count = if let Some(runner) = &app.game_runner {
        runner.game_state().players.len()
    } else {
        0
    };

    let text = vec![
        "".into(),
        "Waiting for players to connect...".into(),
        "".into(),
        format!("Players: {}/2", player_count).into(),
        "".into(),
        "📋 YOUR NODE ID (share this with the other player):".into(),
        "".into(),
        app.endpoint_id.to_string().into(),
        "".into(),
        "The game will start automatically when 2 players join".into(),
    ];

    let paragraph = Paragraph::new(text)
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).title(" Lobby "))
        .style(Style::default().fg(Color::Cyan));
    f.render_widget(paragraph, chunks[1]);

    // Check if we should transition to playing
    if player_count == 2 {
        // This will be handled in the next frame
    }

    // Footer
    let footer = Paragraph::new("ESC to return to main menu • 'q' to quit")
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).title(" Controls "))
        .style(Style::default().fg(Color::Yellow));
    f.render_widget(footer, chunks[2]);
}

fn render_join_game(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            Constraint::Min(8),
            Constraint::Length(3),
        ])
        .split(f.area());

    // Header
    let header = Paragraph::new("🔌 Join Battleship Game")
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center);
    f.render_widget(header, chunks[0]);

    // Input field
    let input_display = if app.join_input.is_empty() {
        "█".to_string()
    } else {
        format!("{}█", app.join_input)
    };
    
    let text = vec![
        "".into(),
        "".into(),
        "Enter the Host's Node ID to connect:".into(),
        "".into(),
        input_display.into(),
        "".into(),
        "".into(),
        "Paste the Node ID from the host and press Enter".into(),
    ];

    let paragraph = Paragraph::new(text)
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).title(" Connect "))
        .style(Style::default().fg(Color::Yellow));
    f.render_widget(paragraph, chunks[1]);

    // Footer
    let footer = Paragraph::new("Type Node ID • Enter to connect • ESC to go back")
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).title(" Controls "))
        .style(Style::default().fg(Color::Yellow));
    f.render_widget(footer, chunks[2]);
}

fn render_game(f: &mut Frame, app: &App) {
    if let Some(runner) = &app.game_runner {
        // Render the game (it has its own layout internally)
        runner.render(f);
        
        // Overlay footer for input at the bottom
        let footer_text = match &app.input_mode {
            InputMode::EnteringCoordinates { row_input, col_input, entering_col } => {
                if *entering_col {
                    format!("🎯 Fire at Row: {} Col: {}█  (Enter to confirm, ESC to cancel)", row_input, col_input)
                } else {
                    format!("🎯 Fire at Row: {}█  (0-9, Enter for next, ESC to cancel)", row_input)
                }
            },
            _ => {
                if let Some(current_turn) = runner.game_state().current_turn_node {
                    if current_turn == app.endpoint_id {
                        "🎯 Your turn! Press 'f' to fire • ESC for menu • 'q' to quit".to_string()
                    } else {
                        "⏳ Opponent's turn... • ESC for menu • 'q' to quit".to_string()
                    }
                } else {
                    "ESC for menu • 'q' to quit".to_string()
                }
            }
        };

        // Render footer at the very bottom, overlaying the game's footer
        let footer_area = Rect {
            x: 0,
            y: f.area().height.saturating_sub(3),
            width: f.area().width,
            height: 3,
        };

        let footer = Paragraph::new(footer_text)
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).title(" Input "))
            .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
        f.render_widget(footer, footer_area);
    }
}