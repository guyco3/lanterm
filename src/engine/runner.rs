use crate::core::game::{LantermGame, LantermContext, EngineCommand, LantermRenderer};
use crate::engine::network::NetworkManager;
use crate::engine::EngineEvent;
use tokio::sync::mpsc;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io;
use std::time::Duration;
use crossterm::event::{self, Event, KeyCode};

pub enum EngineMode {
    Menu,
    HostingWaitingForPlayers,
    JoiningWaitingForAccept,
    GameRunning,
}

pub struct LantermEngine {
    pub(crate) network: NetworkManager,
}

impl LantermEngine {
    pub async fn new() -> anyhow::Result<Self> {
        Ok(Self { network: NetworkManager::new().await? })
    }

    pub async fn run<G, R>(&self) -> anyhow::Result<()> 
    where 
        G: LantermGame,
        R: LantermRenderer<G::State>
    {
        let mut stdout = io::stdout();
        crossterm::terminal::enable_raw_mode()?;
        crossterm::execute!(stdout, crossterm::terminal::EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // Show menu first
        let mode = self.show_menu(&mut terminal)?;
        
        let mut state = G::new_game();
        let mut local_input_buffer = String::new();
        let mut game_mode = EngineMode::Menu;
        
        let endpoint = self.network.endpoint.clone();
        
        // Get the real node ID from the endpoint's secret key
        // In Iroh, each endpoint has a unique identity
        let local_node_id = endpoint.secret_key().public();
        
        let (cmd_tx, mut cmd_rx) = mpsc::channel::<EngineCommand<G::Input>>(100);
        let (event_tx, mut event_rx) = mpsc::channel::<EngineEvent<G::Input>>(100);
        let (conn_tx, mut conn_rx) = mpsc::channel::<iroh::endpoint::Connection>(10);
        
        let ctx = LantermContext { 
            local_node_id,
            cmd_tx: cmd_tx.clone(), 
            _input_type: std::marker::PhantomData 
        };

        let mut interval = G::tick_rate().map(|d| tokio::time::interval(d));
        let mut render_timer = tokio::time::interval(Duration::from_millis(33));
        let mut connections: Vec<iroh::endpoint::Connection> = Vec::new();
        
        match mode {
            EngineMode::Menu => {
                crossterm::terminal::disable_raw_mode()?;
                crossterm::execute!(io::stdout(), crossterm::terminal::LeaveAlternateScreen)?;
                return Ok(());
            }
            EngineMode::HostingWaitingForPlayers => {
                tokio::spawn(NetworkManager::start_accept_loop(endpoint.clone(), event_tx.clone(), conn_tx.clone()));
            }
            EngineMode::JoiningWaitingForAccept => {
                // Show input screen to get peer's node ID
                let peer_node_id = self.show_join_input(&mut terminal)?;
                
                // Connect to the peer
                let addr = iroh::EndpointAddr::from_parts(
                    peer_node_id,
                    vec![] // No direct addresses yet - will use relay
                );
                
                // Try to establish connection
                match endpoint.connect(addr, b"lanterm-battleship").await {
                    Ok(conn) => {
                        // Connection established
                        // Store connection for sending messages
                        connections.push(conn.clone());
                        
                        // Notify the game that we've joined (treat as if the host "joined" us)
                        let _ = event_tx.send(EngineEvent::PlayerJoined(peer_node_id)).await;
                        
                        // Spawn handler for the connection
                        tokio::spawn(async move {
                            // Handle the connection
                            if let Ok((send, recv)) = conn.open_bi().await {
                                // Connection ready
                            }
                        });
                    }
                    Err(e) => {
                        // Connection failed, show error and exit
                        crossterm::terminal::disable_raw_mode()?;
                        crossterm::execute!(io::stdout(), crossterm::terminal::LeaveAlternateScreen)?;
                        return Err(anyhow::anyhow!("Failed to connect: {}", e));
                    }
                }
            }
            EngineMode::GameRunning => {
                // Game is running
            }
        }

        let mut players_connected: i32 = 0;
        
        loop {
            // Non-blocking check for keyboard input
            let mut key_event = None;
            if event::poll(Duration::from_millis(0))? {
                if let Ok(Event::Key(key)) = event::read() {
                    key_event = Some(key);
                }
            }

            tokio::select! {
                Some(conn) = conn_rx.recv() => {
                    connections.push(conn);
                }
                
                Some(cmd) = cmd_rx.recv() => {
                    match cmd {
                        EngineCommand::SendInput(input) => {
                            // Serialize and send to all connections
                            if let Ok(bytes) = postcard::to_allocvec(&input) {
                                for conn in &connections {
                                    if let Ok(mut send_stream) = conn.open_uni().await {
                                        let _ = send_stream.write_all(&bytes).await;
                                        let _ = send_stream.finish();
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }

                Some(event) = event_rx.recv() => {
                    match event {
                        EngineEvent::InputReceived(pid, input) => G::handle_input(&mut state, input, pid, &ctx),
                        EngineEvent::PlayerJoined(pid) => {
                            players_connected += 1;
                            if players_connected >= 2 && matches!(game_mode, EngineMode::HostingWaitingForPlayers) {
                                game_mode = EngineMode::GameRunning;
                            }
                            G::handle_player_joined(&mut state, pid, &ctx);
                        }
                        EngineEvent::PlayerLeft(pid) => {
                            players_connected = players_connected.saturating_sub(1);
                            G::handle_player_left(&mut state, pid, &ctx);
                        }
                    }
                }

                _ = render_timer.tick() => {
                    let mut has_input = false;
                    terminal.draw(|f| {
                        let cursor_pos = R::render(f, &state, local_node_id, &local_input_buffer);
                        if let Some((x, y)) = cursor_pos {
                            f.set_cursor_position((x, y));
                            has_input = true;
                        }
                    })?;
                    
                    // Show/hide cursor based on whether we have input area
                    if has_input {
                        crossterm::execute!(terminal.backend_mut(), crossterm::cursor::Show)?;
                    } else {
                        crossterm::execute!(terminal.backend_mut(), crossterm::cursor::Hide)?;
                    }
                }

                _ = async { 
                    if let Some(ref mut i) = interval { i.tick().await; } 
                    else { std::future::pending::<()>().await; } 
                } => {
                    G::on_tick(&mut state, &ctx);
                }
            }
            
            // Handle keyboard input AFTER tokio::select!
            if let Some(key) = key_event {
                match key.code {
                    KeyCode::Esc => break,
                    KeyCode::Enter => {
                        let cmd = std::mem::take(&mut local_input_buffer);
                        if !cmd.is_empty() {
                            // Parse text command using the game's parser
                            if let Some(input) = G::parse_command(&cmd) {
                                G::handle_input(&mut state, input, local_node_id, &ctx);
                            } else {
                                // Command not recognized - notify user (game-agnostic error)
                                // We can't update state here since it's game-specific
                                // Just do nothing - the game can handle invalid states
                            }
                        }
                    }
                    KeyCode::Backspace => {
                        local_input_buffer.pop();
                    }
                    KeyCode::Char(c) => {
                        local_input_buffer.push(c);
                    }
                    _ => {}
                }
            }
        }

        crossterm::terminal::disable_raw_mode()?;
        crossterm::execute!(io::stdout(), crossterm::terminal::LeaveAlternateScreen)?;
        Ok(())
    }

    fn show_menu(&self, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> anyhow::Result<EngineMode> {
        use ratatui::widgets::{Block, Borders, Paragraph};
        use ratatui::layout::{Layout, Constraint, Direction, Alignment};
        use ratatui::style::{Style, Color};

        let local_node_id = self.network.endpoint.secret_key().public();
        let mut selected = 0;
        
        loop {
            terminal.draw(|f| {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Percentage(20),
                        Constraint::Percentage(60),
                        Constraint::Percentage(20),
                    ])
                    .split(f.area());

                let title = Paragraph::new("Welcome to LANTERM")
                    .alignment(Alignment::Center)
                    .style(Style::default().fg(Color::Cyan));
                f.render_widget(title, chunks[0]);

                let _host_style = if selected == 0 { 
                    Style::default().bg(Color::Blue).fg(Color::White) 
                } else { 
                    Style::default() 
                };
                let _join_style = if selected == 1 { 
                    Style::default().bg(Color::Blue).fg(Color::White) 
                } else { 
                    Style::default() 
                };

                let menu_text = format!(
                    "{} Host Game (your node ID: {})\n\n{} Join Game with Node ID",
                    if selected == 0 { "▶" } else { " " },
                    local_node_id,
                    if selected == 1 { "▶" } else { " " }
                );

                let menu = Paragraph::new(menu_text)
                    .block(Block::default().borders(Borders::ALL).title(" OPTIONS "))
                    .alignment(Alignment::Center);
                f.render_widget(menu, chunks[1]);

                let instructions = Paragraph::new("Use ↑↓ to select, Enter to confirm, Esc to exit")
                    .alignment(Alignment::Center)
                    .style(Style::default().fg(Color::Gray));
                f.render_widget(instructions, chunks[2]);
            })?;

            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Esc => return Ok(EngineMode::Menu),
                        KeyCode::Up => selected = (selected + 1) % 2,
                        KeyCode::Down => selected = (selected + 2) % 2,
                        KeyCode::Enter => {
                            if selected == 0 {
                                return Ok(EngineMode::HostingWaitingForPlayers);
                            } else {
                                return Ok(EngineMode::JoiningWaitingForAccept);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    fn show_join_input(&self, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> anyhow::Result<iroh::EndpointId> {
        use ratatui::widgets::{Block, Borders, Paragraph};
        use ratatui::layout::{Layout, Constraint, Direction, Alignment};
        use ratatui::style::{Style, Color};

        let mut input_buffer = String::new();
        
        loop {
            terminal.draw(|f| {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Percentage(20),
                        Constraint::Percentage(60),
                        Constraint::Percentage(20),
                    ])
                    .split(f.area());

                let title = Paragraph::new("Join Game")
                    .alignment(Alignment::Center)
                    .style(Style::default().fg(Color::Cyan));
                f.render_widget(title, chunks[0]);

                let prompt_text = format!(
                    "Enter the Node ID of the host:\n\n{}",
                    input_buffer
                );

                let prompt = Paragraph::new(prompt_text)
                    .block(Block::default().borders(Borders::ALL).title(" NODE ID "))
                    .alignment(Alignment::Center);
                f.render_widget(prompt, chunks[1]);

                let instructions = Paragraph::new("Type the Node ID and press Enter. Esc to cancel.")
                    .alignment(Alignment::Center)
                    .style(Style::default().fg(Color::Gray));
                f.render_widget(instructions, chunks[2]);
            })?;

            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Esc => {
                            return Err(anyhow::anyhow!("Join cancelled"));
                        }
                        KeyCode::Enter => {
                            // Try to parse the node ID - trim whitespace first
                            let trimmed = input_buffer.trim();
                            if trimmed.is_empty() {
                                continue;
                            }
                            
                            match trimmed.parse::<iroh::EndpointId>() {
                                Ok(node_id) => {
                                    return Ok(node_id);
                                }
                                Err(e) => {
                                    // Show error in the input buffer
                                    input_buffer = format!("ERROR: Invalid Node ID - {}", e);
                                    // Wait a moment so user can see the error
                                    std::thread::sleep(Duration::from_secs(2));
                                    input_buffer.clear();
                                }
                            }
                        }
                        KeyCode::Backspace => {
                            input_buffer.pop();
                        }
                        KeyCode::Char(c) => {
                            input_buffer.push(c);
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}
