
use crate::core::renderer::GameRenderer;
use crate::core::terminal::{TerminalContext, TerminalColor};
use super::game::{BattleshipState, CellState};

const BOARD_SIZE: usize = 10;

pub struct BattleshipRenderer {
    player_name: String,
}

impl GameRenderer<BattleshipState> for BattleshipRenderer {
    fn new(player_name: String) -> Self {
        Self { player_name }
    }

    fn render(&self, state: &BattleshipState, ctx: &mut TerminalContext) {
        // Much cleaner - no manual terminal handling!
        ctx.print_line("🚢 ═══ BATTLESHIP ═══ 🚢");
        ctx.print_line(&format!("Player: {}", self.player_name));
        ctx.empty_line();
        
        // Show game status
        if state.players.len() < 2 {
            ctx.print_colored_line(&state.message, TerminalColor::Yellow);
            ctx.print_line(&format!("Players: {}/2", state.players.len()));
            ctx.flush();
            return;
        }

        // Show players
        ctx.print_line(&format!("⚔️  {} vs {}", state.players[0], state.players[1]));
        ctx.empty_line();

        // Show current turn or winner
        if state.finished {
            if let Some(ref winner) = state.winner {
                ctx.print_colored_line(&format!("🏆 {} is victorious!", winner), TerminalColor::Green);
            }
        } else {
            let current_player = &state.players[state.current_turn];
            ctx.print_colored_line(&format!("🎯 {}'s turn to fire", current_player), TerminalColor::Yellow);
        }
        
        ctx.empty_line();
        ctx.print_line(&state.message);
        ctx.empty_line();

        // Show both boards side by side
        self.render_boards_side_by_side(state, ctx);
        
        if !state.finished {
            ctx.empty_line();
            ctx.print_line("💡 Enter coordinates to fire (row,col):");
            ctx.print_line("   Example: '3,4' or '3 4' to fire at row 3, column 4");
        }

        ctx.flush();
    }
}

impl BattleshipRenderer {
    fn render_boards_side_by_side(&self, state: &BattleshipState, ctx: &mut TerminalContext) {
        if state.player_boards.len() != 2 {
            return;
        }

        // Headers - much simpler with terminal context!
        ctx.print(&format!("{:<25}", format!("🛡️  {}'s Fleet", state.players[0])));
        ctx.print_line(&format!("🎯 {}'s Targets", state.players[1]));
        ctx.empty_line();

        // Column headers for both boards
        ctx.print("   ");
        for i in 0..BOARD_SIZE { ctx.print(&format!(" {} ", i)); }
        ctx.print("     "); // Space between boards
        ctx.print("   ");
        for i in 0..BOARD_SIZE { ctx.print(&format!(" {} ", i)); }
        ctx.empty_line();

        // Render rows
        for row in 0..BOARD_SIZE {
            // Player 0's board (own ships visible)
            ctx.print(&format!("{:2} ", row));
            for col in 0..BOARD_SIZE {
                let cell = state.player_boards[0].grid()[row][col];
                self.render_cell(cell, false, ctx);
            }
            
            ctx.print("     "); // Space between boards
            
            // Player 1's board from player 0's perspective (opponent view - ships hidden)
            ctx.print(&format!("{:2} ", row));
            for col in 0..BOARD_SIZE {
                let cell = state.player_boards[1].grid()[row][col];
                self.render_cell(cell, true, ctx);
            }
            ctx.empty_line();
        }

        ctx.empty_line();
        ctx.print_line("Legend: ■ Ship  ● Hit  · Miss  □ Water");
    }

    fn render_cell(&self, cell: CellState, hide_ships: bool, ctx: &mut TerminalContext) {
        match cell {
            CellState::Empty => {
                if hide_ships {
                    ctx.print("   ");
                } else {
                    ctx.print_colored(" □ ", TerminalColor::Blue);
                }
            },
            CellState::Ship => {
                if hide_ships {
                    ctx.print("   ");
                } else {
                    ctx.print_colored(" ■ ", TerminalColor::White);
                }
            },
            CellState::Hit => {
                ctx.print_colored(" ● ", TerminalColor::Red);
            },
            CellState::Miss => {
                ctx.print_colored(" · ", TerminalColor::Cyan);
            },
        }
    }
}