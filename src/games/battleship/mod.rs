use crate::core::game::{LantermGame, LantermContext, NodeId, LantermRenderer};
use serde::{Serialize, Deserialize};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::layout::{Layout, Constraint, Direction};
use ratatui::style::{Style, Color};

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Debug)]
pub enum Cell { Empty, Ship, Hit, Miss }

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub enum GamePhase {
    Lobby,     // Waiting for players to join
    Setup,     // Placing ships
    Waiting,   // Waiting for opponent to be ready
    Playing,   // Game in progress
    Ended,     // Game over
}

#[derive(Clone, Serialize, Deserialize)]
pub struct BattleshipState {
    pub my_board: [[Cell; 10]; 10],
    pub opponent_view: [[Cell; 10]; 10],
    pub is_my_turn: bool,
    pub status_message: String,
    pub phase: GamePhase,
    pub opponent_id: Option<NodeId>,
    pub my_ships_remaining: usize,
    pub opponent_ships_remaining: usize,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum BattleshipInput {
    Fire { x: usize, y: usize },
    Report { x: usize, y: usize, hit: bool },
    PlaceShip { x: usize, y: usize, horizontal: bool, length: usize },
    ReadyForGame,
}

#[derive(Clone)]
pub struct Battleship;

impl LantermGame for Battleship {
    type State = BattleshipState;
    type Input = BattleshipInput;

    fn new_game() -> Self::State {
        BattleshipState {
            my_board: [[Cell::Empty; 10]; 10],
            opponent_view: [[Cell::Empty; 10]; 10],
            is_my_turn: false,
            status_message: "Lobby: Waiting for opponent to join...".to_string(),
            phase: GamePhase::Lobby,
            opponent_id: None,
            my_ships_remaining: 5, // 5 ships total
            opponent_ships_remaining: 5,
        }
    }

    fn parse_command(cmd: &str) -> Option<Self::Input> {
        let parts: Vec<&str> = cmd.trim().split_whitespace().collect();
        if parts.is_empty() {
            return None;
        }

        match parts[0].to_lowercase().as_str() {
            "place" | "p" => {
                if parts.len() >= 4 {
                    let x = parts[1].parse::<usize>().ok()?;
                    let y = parts[2].parse::<usize>().ok()?;
                    let horizontal = parts[3].to_lowercase().starts_with('h');
                    // Auto-detect length: try 5,4,3,2 in order (standard battleship sizes)
                    let length = if parts.len() >= 5 {
                        parts[4].parse::<usize>().ok()?
                    } else {
                        5  // default to carrier (5)
                    };
                    Some(BattleshipInput::PlaceShip { x, y, horizontal, length })
                } else {
                    None
                }
            }
            "fire" | "f" => {
                if parts.len() >= 3 {
                    let x = parts[1].parse::<usize>().ok()?;
                    let y = parts[2].parse::<usize>().ok()?;
                    Some(BattleshipInput::Fire { x, y })
                } else {
                    None
                }
            }
            "ready" | "r" => Some(BattleshipInput::ReadyForGame),
            _ => None,
        }
    }

    fn handle_input(state: &mut Self::State, input: Self::Input, player: NodeId, ctx: &LantermContext<Self::Input>) {
        // Set opponent ID on first contact (if player is not local)
        if state.opponent_id.is_none() && player != ctx.local_node_id {
            state.opponent_id = Some(player);
        }

        match state.phase {
            GamePhase::Lobby => {
                // In lobby, we're just waiting for opponent
                // Don't process any game inputs yet
            }
            GamePhase::Setup => {
                // Only allow local player to place ships
                if player != ctx.local_node_id {
                    return;
                }
                
                match input {
                    BattleshipInput::PlaceShip { x, y, horizontal, length } => {
                        // Validate placement
                        if length < 2 || length > 5 {
                            state.status_message = "Invalid ship length! Use 2-5.".to_string();
                            return;
                        }
                        
                        if x >= 10 || y >= 10 {
                            state.status_message = "Coordinates out of bounds! Use 0-9.".to_string();
                            return;
                        }
                        
                        let end_x = x + if horizontal { length } else { 0 };
                        let end_y = y + if !horizontal { length } else { 0 };
                        
                        if end_x > 10 || end_y > 10 {
                            state.status_message = "Ship doesn't fit on board!".to_string();
                            return;
                        }
                        
                        // Check for overlaps
                        for i in 0..length {
                            let (px, py) = if horizontal { (x + i, y) } else { (x, y + i) };
                            if state.my_board[py][px] == Cell::Ship {
                                state.status_message = "Ships overlap! Try different position.".to_string();
                                return;
                            }
                        }
                        
                        // Place the ship
                        for i in 0..length {
                            let (px, py) = if horizontal { (x + i, y) } else { (x, y + i) };
                            state.my_board[py][px] = Cell::Ship;
                        }
                        
                        // Count ships placed
                        let ships_placed = state.my_board.iter()
                            .flat_map(|row| row.iter())
                            .filter(|&&cell| cell == Cell::Ship)
                            .count() / 5;  // Rough count (assumes 5-length ships)
                        
                        state.status_message = format!("✓ Ship placed at ({},{})! {} ships placed. Place more (e.g. 'p 2 3 v 4') or type 'ready'", x, y, ships_placed);
                    }
                    BattleshipInput::ReadyForGame => {
                        // Only local player can mark themselves ready
                        if player == ctx.local_node_id {
                            state.phase = GamePhase::Waiting;
                            state.status_message = "You are ready! Waiting for opponent to place their ships and type 'ready'...".to_string();
                            // Send ready message to opponent (non-blocking)
                            tokio::spawn({
                                let ctx_send = ctx.cmd_tx.clone();
                                async move {
                                    use crate::core::game::EngineCommand;
                                    let _ = ctx_send.send(EngineCommand::SendInput(BattleshipInput::ReadyForGame)).await;
                                }
                            });
                        }
                    }
                    _ => {}
                }
            }
            GamePhase::Waiting => {
                if let BattleshipInput::ReadyForGame = input {
                    state.phase = GamePhase::Playing;
                    state.is_my_turn = true;
                    state.status_message = "Both players ready! Game started! Your turn to fire. Type 'fire X Y'.".to_string();
                }
            }
            GamePhase::Playing => {
                match input {
                    BattleshipInput::Fire { x, y } => {
                        if state.my_board[y][x] == Cell::Ship {
                            state.my_board[y][x] = Cell::Hit;
                            state.opponent_ships_remaining = state.opponent_ships_remaining.saturating_sub(1);
                            state.status_message = format!("Opponent hit your ship at ({}, {})!", x, y);
                            let _ = ctx.send_input(BattleshipInput::Report { x, y, hit: true });
                        } else {
                            state.my_board[y][x] = Cell::Miss;
                            state.status_message = format!("Opponent missed at ({}, {})", x, y);
                            let _ = ctx.send_input(BattleshipInput::Report { x, y, hit: false });
                        }
                        state.is_my_turn = true;
                    }
                    BattleshipInput::Report { x, y, hit } => {
                        state.opponent_view[y][x] = if hit { Cell::Hit } else { Cell::Miss };
                        if hit {
                            state.status_message = "You HIT their ship!".to_string();
                            state.my_ships_remaining = state.my_ships_remaining.saturating_sub(1);
                        } else {
                            state.status_message = "You missed.".to_string();
                        }
                        state.is_my_turn = false;

                        if state.opponent_ships_remaining == 0 {
                            state.phase = GamePhase::Ended;
                            state.status_message = "You WON!".to_string();
                        } else if state.my_ships_remaining == 0 {
                            state.phase = GamePhase::Ended;
                            state.status_message = "You LOST!".to_string();
                        }
                    }
                    _ => {}
                }
            }
            GamePhase::Ended => {
                // Game over, nothing to do
            }
        }
    }

    fn handle_player_joined(state: &mut Self::State, player: NodeId, _ctx: &LantermContext<Self::Input>) {
        state.opponent_id = Some(player);
        
        // Transition from Lobby to Setup when a player joins
        if state.phase == GamePhase::Lobby {
            state.phase = GamePhase::Setup;
            state.status_message = "Opponent connected! Place ships: type 'p 0 0 h 5' (X Y horizontal/vertical length). Then 'ready'.".to_string();
        } else {
            state.status_message = format!("Player {} joined!", player);
        }
    }
}

pub struct BattleshipRenderer;

impl LantermRenderer<BattleshipState> for BattleshipRenderer {
    fn render(frame: &mut ratatui::Frame, state: &BattleshipState, local_node_id: NodeId, current_input: &str) -> Option<(u16, u16)> {
        // Show lobby screen if in lobby phase
        if state.phase == GamePhase::Lobby {
            Self::render_lobby(frame, local_node_id, &state.status_message);
            return None;
        }

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(70),
                Constraint::Percentage(20),
                Constraint::Percentage(10),
            ])
            .split(frame.area());

        // Main game area
        let game_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[0]);

        let render_board = |board: &[[Cell; 10]; 10]| {
            let mut s = String::new();
            for row in board {
                for cell in row {
                    let symbol = match cell {
                        Cell::Empty => ". ",
                        Cell::Ship => "S ",
                        Cell::Hit => "X ",
                        Cell::Miss => "O ",
                    };
                    s.push_str(symbol);
                }
                s.push('\n');
            }
            s
        };

        let my_board = Paragraph::new(render_board(&state.my_board))
            .block(Block::default().title(" MY SHIPS ").borders(Borders::ALL));
        
        let opp_board = Paragraph::new(render_board(&state.opponent_view))
            .block(Block::default().title(" ENEMY WATERS ").borders(Borders::ALL));

        frame.render_widget(my_board, game_chunks[0]);
        frame.render_widget(opp_board, game_chunks[1]);

        // Status area
        let status_style = match state.phase {
            GamePhase::Lobby => Style::default().fg(Color::White),
            GamePhase::Setup => Style::default().fg(Color::Yellow),
            GamePhase::Waiting => Style::default().fg(Color::Cyan),
            GamePhase::Playing => Style::default().fg(Color::Green),
            GamePhase::Ended => Style::default().fg(Color::Magenta),
        };

        let status_text = format!(
            "Phase: {:?} | {} | My Ships: {} | Enemy Ships: {} | Your Turn: {}",
            state.phase,
            state.status_message,
            state.my_ships_remaining,
            state.opponent_ships_remaining,
            state.is_my_turn
        );

        let status = Paragraph::new(status_text)
            .block(Block::default().borders(Borders::ALL))
            .style(status_style);
        
        frame.render_widget(status, chunks[1]);

        // Input area with helpful commands
        let input_help = match state.phase {
            GamePhase::Setup => " INPUT: Type 'p 0 0 h 5' (place ship at 0,0 horizontal length 5) or 'ready' ",
            GamePhase::Playing if state.is_my_turn => " INPUT: Type 'f 3 4' to fire at coordinates (3,4) ",
            _ => " INPUT ",
        };
        
        let input_area = Paragraph::new(format!("> {}", current_input))
            .block(Block::default().borders(Borders::ALL).title(input_help));
        
        frame.render_widget(input_area, chunks[2]);

        // Return cursor position in input area (after "> " prompt)
        Some((chunks[2].x + 3 + current_input.len() as u16, chunks[2].y + 1))
    }
}
impl BattleshipRenderer {
    fn render_lobby(frame: &mut ratatui::Frame, local_node_id: NodeId, status_message: &str) {
        use ratatui::layout::Alignment;
        
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(20),
                Constraint::Percentage(60),
                Constraint::Percentage(20),
            ])
            .split(frame.area());

        let title = Paragraph::new("BATTLESHIP LOBBY")
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::Cyan))
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(title, chunks[0]);

        let info_text = format!(
            "Your Node ID:\n\n{}\n\n\n{}",
            local_node_id,
            status_message
        );

        let info = Paragraph::new(info_text)
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).title(" WAITING FOR PLAYERS "))
            .style(Style::default().fg(Color::Yellow));
        frame.render_widget(info, chunks[1]);

        let instructions = Paragraph::new("Share your Node ID with other players so they can join.\nPress Esc to exit.")
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::Gray));
        frame.render_widget(instructions, chunks[2]);
    }
}