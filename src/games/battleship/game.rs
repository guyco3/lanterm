use crate::{Context, Game};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use serde::{Deserialize, Serialize};
use crossterm::event::KeyCode;
use iroh::EndpointId;

const GRID_SIZE: usize = 8;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BattleAction {
    MoveCursor { dx: i8, dy: i8, is_host: bool },
    Fire { is_host: bool },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
pub enum Tile {
    #[default]
    Empty,
    Ship,
    Hit,
    Miss,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BattleState {
    pub p1_board: [[Tile; GRID_SIZE]; GRID_SIZE],
    pub p2_board: [[Tile; GRID_SIZE]; GRID_SIZE],
    pub p1_cursor: (usize, usize),
    pub p2_cursor: (usize, usize),
    pub host_turn: bool,
    pub winner: Option<String>,
    pub last_message: String,
}

impl Default for BattleState {
    fn default() -> Self {
        let mut p1 = [[Tile::Empty; GRID_SIZE]; GRID_SIZE];
        let mut p2 = [[Tile::Empty; GRID_SIZE]; GRID_SIZE];
        
        // Default ship placement
        p1[1][1] = Tile::Ship; p1[1][2] = Tile::Ship;
        p2[5][5] = Tile::Ship; p2[5][6] = Tile::Ship;

        Self {
            p1_board: p1,
            p2_board: p2,
            p1_cursor: (0, 0),
            p2_cursor: (0, 0),
            host_turn: true,
            winner: None,
            last_message: "Game Start! Host (P1) moves first.".into(),
        }
    }
}

pub struct BattleshipGame {
    is_host: bool,
}

impl BattleshipGame {
    pub fn new(is_host: bool) -> Self {
        Self { is_host }
    }
}

impl Game for BattleshipGame {
    type Action = BattleAction;
    type State = BattleState;

    fn handle_input(&mut self, event: crossterm::event::KeyEvent, ctx: &Context<Self::Action>, _me: EndpointId) {
        let is_host = self.is_host;
        match event.code {
            KeyCode::Up => ctx.send_action(BattleAction::MoveCursor { dx: 0, dy: -1, is_host }),
            KeyCode::Down => ctx.send_action(BattleAction::MoveCursor { dx: 0, dy: 1, is_host }),
            KeyCode::Left => ctx.send_action(BattleAction::MoveCursor { dx: -1, dy: 0, is_host }),
            KeyCode::Right => ctx.send_action(BattleAction::MoveCursor { dx: 1, dy: 0, is_host }),
            KeyCode::Enter | KeyCode::Char(' ') => ctx.send_action(BattleAction::Fire { is_host }),
            _ => {}
        }
    }

    fn handle_action(&self, action: Self::Action, state: &mut Self::State, player: EndpointId) {
        if state.winner.is_some() { return; }

        match action {
            BattleAction::MoveCursor { dx, dy, is_host } => {
                // Players move their own targeting cursor
                let cursor = if is_host { &mut state.p1_cursor } else { &mut state.p2_cursor };
                cursor.0 = (cursor.0 as i8 + dx).clamp(0, GRID_SIZE as i8 - 1) as usize;
                cursor.1 = (cursor.1 as i8 + dy).clamp(0, GRID_SIZE as i8 - 1) as usize;
            }
            BattleAction::Fire { is_host } => {
                if is_host != state.host_turn {
                    state.last_message = "Wait for your turn!".into();
                    return;
                }

                let (cx, cy) = if is_host { state.p1_cursor } else { state.p2_cursor };
                let target_board = if is_host { &mut state.p2_board } else { &mut state.p1_board };

                match target_board[cy][cx] {
                    Tile::Ship => {
                        target_board[cy][cx] = Tile::Hit;
                        state.last_message = format!("Player {player} HIT a ship!");
                        
                        if !target_board.iter().flatten().any(|t| matches!(t, Tile::Ship)) {
                            state.winner = Some("Player ".to_string() + &player.to_string());
                        }
                    }
                    Tile::Empty => {
                        target_board[cy][cx] = Tile::Miss;
                        state.last_message = format!("Player {player} MISSED!");
                        state.host_turn = !state.host_turn;
                    }
                    _ => {
                        state.last_message = "Already fired there!".into();
                    }
                }
            }
        }
    }

    fn on_tick(&self, _dt: u32, _state: &mut Self::State) {}

    fn render(&self, frame: &mut ratatui::Frame, state: &Self::State) {
        let root = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(GRID_SIZE as u16 + 2),
                Constraint::Length(3),
                Constraint::Length(3),
            ])
            .split(frame.area());

        let boards_chunk = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(root[0]);

        // --- RENDER LOGIC UPDATE ---
        // If Host: Left is Your Board (P1), Right is Target Board (P2)
        // If Client: Right is Your Board (P2), Left is Target Board (P1)

        // Board 1 (Left)
        let (p1_active, p1_show_ships) = if self.is_host {
            (false, true) // Host's own board: no cursor, show ships
        } else {
            (state.host_turn == false, false) // Client targeting Host: cursor if turn, hide ships
        };
        self.draw_grid(frame, boards_chunk[0], if self.is_host { "YOUR BOARD (P1)" } else { "ENEMY BOARD (P1)" }, 
            &state.p1_board, p1_active, state.p2_cursor, p1_show_ships);

        // Board 2 (Right)
        let (p2_active, p2_show_ships) = if self.is_host {
            (state.host_turn == true, false) // Host targeting Client: cursor if turn, hide ships
        } else {
            (false, true) // Client's own board: no cursor, show ships
        };
        self.draw_grid(frame, boards_chunk[1], if self.is_host { "ENEMY BOARD (P2)" } else { "YOUR BOARD (P2)" }, 
            &state.p2_board, p2_active, state.p1_cursor, p2_show_ships);

        // Status
        let turn_text = if state.host_turn == self.is_host {
            Span::styled(" YOUR TURN ", Style::default().bg(Color::Green).fg(Color::Black).add_modifier(Modifier::BOLD))
        } else {
            Span::styled(" ENEMY TURN ", Style::default().bg(Color::Red).fg(Color::White))
        };

        frame.render_widget(
            Paragraph::new(Line::from(vec![turn_text, Span::raw(format!(" | {}", state.last_message))]))
                .block(Block::default().borders(Borders::ALL).title("Status")),
            root[1]
        );

        frame.render_widget(
            Paragraph::new("Arrows: Move Target Cursor | Enter/Space: Fire | Esc: Quit")
                .block(Block::default().borders(Borders::ALL).title("Controls"))
                .style(Style::default().fg(Color::DarkGray)),
            root[2]
        );

        if let Some(ref winner) = state.winner {
            let area = self.centered_rect(40, 3, frame.area());
            frame.render_widget(ratatui::widgets::Clear, area);
            frame.render_widget(
                Paragraph::new(format!("{} WINS!", winner))
                    .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Yellow)))
                    .alignment(ratatui::layout::Alignment::Center),
                area
            );
        }
    }
}

impl BattleshipGame {
    fn draw_grid(&self, frame: &mut ratatui::Frame, area: Rect, title: &str, board: &[[Tile; GRID_SIZE]; GRID_SIZE], active: bool, cursor: (usize, usize), show_ships: bool) {
        let mut lines = Vec::new();
        for y in 0..GRID_SIZE {
            let mut spans = Vec::new();
            for x in 0..GRID_SIZE {
                let is_cursor = active && cursor == (x, y);
                let symbol = match board[y][x] {
                    Tile::Empty => " . ",
                    Tile::Ship => if show_ships { " S " } else { " . " },
                    Tile::Hit => " X ",
                    Tile::Miss => " O ",
                };
                let style = if is_cursor {
                    Style::default().bg(Color::Yellow).fg(Color::Black)
                } else {
                    match board[y][x] {
                        Tile::Hit => Style::default().fg(Color::Red),
                        Tile::Miss => Style::default().fg(Color::White),
                        Tile::Ship if show_ships => Style::default().fg(Color::Green),
                        _ => Style::default().fg(Color::DarkGray),
                    }
                };
                spans.push(Span::styled(symbol, style));
            }
            lines.push(Line::from(spans));
        }
        let block = Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(if active { Style::default().fg(Color::Cyan) } else { Style::default() });
            
        frame.render_widget(Paragraph::new(lines).block(block), area);
    }

    fn centered_rect(&self, percent_x: u16, height: u16, r: Rect) -> Rect {
        let popup_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length((r.height.saturating_sub(height)) / 2),
                Constraint::Length(height),
                Constraint::Min(0),
            ])
            .split(r);

        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ])
            .split(popup_layout[1])[1]
    }
}