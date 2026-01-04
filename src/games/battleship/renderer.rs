
use crossterm::{QueueableCommand, cursor, terminal};
use std::io::{stdout, Write};

use crate::core::renderer::GameRenderer;
use super::game::{BattleshipState, CellState};

const BOARD_SIZE: usize = 10;

pub struct BattleshipRenderer {
    player_name: String,
}

impl GameRenderer<BattleshipState> for BattleshipRenderer {
    fn new(player_name: String) -> Self {
        Self { player_name }
    }

    fn render(&self, state: &BattleshipState) {
        let mut out = stdout();

        // 1. Move cursor to 0,0 and clear instead of raw ANSI
        out.queue(cursor::MoveTo(0, 0)).unwrap();
        out.queue(terminal::Clear(terminal::ClearType::All)).unwrap();
        
        // 2. Use writeln! with carriage returns
        writeln!(out, "🚢 ═══ BATTLESHIP ═══ 🚢\r").unwrap();
        writeln!(out, "Player: {}\r", self.player_name).unwrap();
        writeln!(out, "\r").unwrap();
        
        // Show game status
        if state.players.len() < 2 {
            writeln!(out, "⏳ {}\r", state.message).unwrap();
            writeln!(out, "Players: {}/2\r", state.players.len()).unwrap();
            out.flush().unwrap();
            return;
        }

        // Show players
        writeln!(out, "⚔️  {} vs {}\r", state.players[0], state.players[1]).unwrap();
        writeln!(out, "\r").unwrap();

        // Show current turn or winner
        if state.finished {
            if let Some(ref winner) = state.winner {
                out.queue(crossterm::style::SetForegroundColor(crossterm::style::Color::Green)).unwrap();
                writeln!(out, "🏆 {} is victorious!\r", winner).unwrap();
                out.queue(crossterm::style::ResetColor).unwrap();
            }
        } else {
            let current_player = &state.players[state.current_turn];
            out.queue(crossterm::style::SetForegroundColor(crossterm::style::Color::Yellow)).unwrap();
            writeln!(out, "🎯 {}'s turn to fire\r", current_player).unwrap();
            out.queue(crossterm::style::ResetColor).unwrap();
        }
        
        writeln!(out, "\r").unwrap();
        writeln!(out, "{}\r", state.message).unwrap();
        writeln!(out, "\r").unwrap();

        // Show both boards side by side
        self.render_boards_side_by_side(state, &mut out);
        
        if !state.finished {
            writeln!(out, "\r").unwrap();
            writeln!(out, "💡 Enter coordinates to fire (row,col):\r").unwrap();
            writeln!(out, "   Example: '3,4' or '3 4' to fire at row 3, column 4\r").unwrap();
        }

        // 3. Flush everything at once to prevent flickering
        out.flush().unwrap();
    }
}

impl BattleshipRenderer {
    fn render_boards_side_by_side(&self, state: &BattleshipState, out: &mut std::io::Stdout) {
        if state.player_boards.len() != 2 {
            return;
        }

        // Headers
        write!(out, "{:<25}", format!("🛡️  {}'s Fleet", state.players[0])).unwrap();
        writeln!(out, "🎯 {}'s Targets\r", state.players[1]).unwrap();
        writeln!(out, "\r").unwrap();

        // Column headers for both boards
        write!(out, "   ").unwrap();
        for i in 0..BOARD_SIZE { write!(out, " {} ", i).unwrap(); }
        write!(out, "     ").unwrap(); // Space between boards
        write!(out, "   ").unwrap();
        for i in 0..BOARD_SIZE { write!(out, " {} ", i).unwrap(); }
        writeln!(out, "\r").unwrap();

        // Render rows
        for row in 0..BOARD_SIZE {
            // Player 0's board (own ships visible)
            write!(out, "{:2} ", row).unwrap();
            for col in 0..BOARD_SIZE {
                let cell = state.player_boards[0].grid()[row][col];
                self.render_cell(cell, false, out);
            }
            
            write!(out, "     ").unwrap(); // Space between boards
            
            // Player 1's board from player 0's perspective (opponent view - ships hidden)
            write!(out, "{:2} ", row).unwrap();
            for col in 0..BOARD_SIZE {
                let cell = state.player_boards[1].grid()[row][col];
                self.render_cell(cell, true, out);
            }
            writeln!(out, "\r").unwrap();
        }

        writeln!(out, "\r").unwrap();
        writeln!(out, "Legend: ■ Ship  ● Hit  · Miss  □ Water\r").unwrap();
    }

    fn render_cell(&self, cell: CellState, hide_ships: bool, out: &mut std::io::Stdout) {
        match cell {
            CellState::Empty => {
                if hide_ships {
                    write!(out, "   ").unwrap();
                } else {
                    out.queue(crossterm::style::SetForegroundColor(crossterm::style::Color::Blue)).unwrap();
                    write!(out, " □ ").unwrap();
                    out.queue(crossterm::style::ResetColor).unwrap();
                }
            },
            CellState::Ship => {
                if hide_ships {
                    write!(out, "   ").unwrap();
                } else {
                    out.queue(crossterm::style::SetForegroundColor(crossterm::style::Color::White)).unwrap();
                    write!(out, " ■ ").unwrap();
                    out.queue(crossterm::style::ResetColor).unwrap();
                }
            },
            CellState::Hit => {
                out.queue(crossterm::style::SetForegroundColor(crossterm::style::Color::Red)).unwrap();
                write!(out, " ● ").unwrap();
                out.queue(crossterm::style::ResetColor).unwrap();
            },
            CellState::Miss => {
                out.queue(crossterm::style::SetForegroundColor(crossterm::style::Color::Cyan)).unwrap();
                write!(out, " · ").unwrap();
                out.queue(crossterm::style::ResetColor).unwrap();
            },
        }
    }
}