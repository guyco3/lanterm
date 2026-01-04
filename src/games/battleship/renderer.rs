use crate::core::game::{LantermRenderer, NodeId};
use super::game::{BattleshipState, CellState, Board};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph, Table, Row, Cell},
    layout::{Layout, Constraint, Direction}
};

const BOARD_SIZE: usize = 10;

#[derive(Debug)]
pub struct BattleshipRenderer;

impl LantermRenderer<BattleshipState> for BattleshipRenderer {
    fn render(frame: &mut Frame, state: &BattleshipState, local_node_id: NodeId) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Length(3), // Status
                Constraint::Min(0),    // Game area
                Constraint::Length(3), // Footer
            ])
            .split(frame.area());

        // Header
        let header = Paragraph::new("🚢 ═══ BATTLESHIP v2 ═══ 🚢")
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            .alignment(Alignment::Center);
        frame.render_widget(header, chunks[0]);

        // Status
        let status_text = if state.players.len() < 2 {
            format!("Waiting for players... ({}/2)", state.players.len())
        } else if state.finished {
            if let Some(winner) = state.winner {
                if winner == local_node_id {
                    "🏆 Victory! You sunk all enemy ships!".to_string()
                } else {
                    "💀 Defeat! Your fleet has been destroyed.".to_string()
                }
            } else {
                "Game finished".to_string()
            }
        } else {
            if Some(local_node_id) == state.current_turn_node {
                "🎯 Your turn! Press 'f' to fire!".to_string()
            } else {
                "⏳ Waiting for opponent...".to_string()
            }
        };

        let status = Paragraph::new(status_text)
            .block(Block::default().borders(Borders::ALL).title("Status"))
            .style(Style::default().fg(Color::Yellow));
        frame.render_widget(status, chunks[1]);

        // Game area - split into two boards
        if state.players.len() == 2 {
            let game_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(chunks[2]);

            // My board (left side)
            if let Some(my_board) = state.my_board(local_node_id) {
                let my_board_widget = Self::build_board_widget(my_board, "My Fleet", true);
                frame.render_widget(my_board_widget, game_chunks[0]);
            }

            // Enemy board (right side) - with fog of war
            if let Some(enemy_board) = state.opponent_view(local_node_id) {
                let enemy_board_widget = Self::build_board_widget(&enemy_board, "Enemy Waters", false);
                frame.render_widget(enemy_board_widget, game_chunks[1]);
            }
        }

        // Footer with last action
        let footer = Paragraph::new(state.last_action.clone())
            .block(Block::default().borders(Borders::ALL).title("Battle Log"))
            .style(Style::default().fg(Color::White));
        frame.render_widget(footer, chunks[3]);
    }
}

impl BattleshipRenderer {
    fn build_board_widget<'a>(board: &'a Board, title: &'a str, show_ships: bool) -> Table<'a> {
        let mut rows = vec![
            Row::new((0..BOARD_SIZE).map(|i| Cell::from(format!("{}", i))).collect::<Vec<_>>())
        ];

        for (row_idx, row) in board.grid().iter().enumerate() {
            let mut cells = vec![Cell::from(format!("{}", row_idx))];
            
            for &cell in row.iter() {
                let cell_char = match cell {
                    CellState::Empty => "~",
                    CellState::Ship if show_ships => "■",
                    CellState::Ship => "~", // Hide enemy ships
                    CellState::Hit => "💥",
                    CellState::Miss => "💦",
                };
                
                let cell_style = match cell {
                    CellState::Hit => Style::default().fg(Color::Red),
                    CellState::Miss => Style::default().fg(Color::Blue),
                    CellState::Ship if show_ships => Style::default().fg(Color::Green),
                    _ => Style::default().fg(Color::Cyan),
                };
                
                cells.push(Cell::from(cell_char).style(cell_style));
            }
            
            rows.push(Row::new(cells));
        }

        Table::new(
            rows,
            std::iter::repeat(Constraint::Length(3)).take(BOARD_SIZE + 1).collect::<Vec<_>>()
        )
            .block(Block::default().borders(Borders::ALL).title(title))
            .style(Style::default().fg(Color::White))
    }
}