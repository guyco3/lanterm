/// Hangman-specific renderer - injected dependency
use crate::core::renderer::GameRenderer;
use crate::core::terminal::{TerminalContext, TerminalColor};
use crate::games::hangman::HangmanState;

/// Dependency-injected renderer for Hangman
pub struct HangmanRenderer {
    player_name: String,
}

impl GameRenderer<HangmanState> for HangmanRenderer {
    fn new(player_name: String) -> Self {
        Self { player_name }
    }
    
    /// Pure rendering function - no game logic
    fn render(&self, state: &HangmanState, ctx: &mut TerminalContext) {
        ctx.clear_screen();
        
        // Header
        ctx.print_colored(&format!("🎩 HANGMAN (WebSocket) - Player: {}", self.player_name), TerminalColor::Cyan);
        ctx.print_line("══════════════════════════════════");
        ctx.print_line("");
        
        // Game state
        ctx.print_line(&format!("Word: {}", state.masked_word));
        ctx.print_line("");
        ctx.print_line(&format!("Tries left: {} {}", state.remaining_tries, "❤".repeat(state.remaining_tries as usize)));
        
        if !state.guessed.is_empty() {
            ctx.print_colored(&format!("Correct: {}", state.guessed.iter().collect::<String>()), TerminalColor::Green);
        }
        if !state.wrong.is_empty() {
            ctx.print_colored(&format!("Wrong: {}", state.wrong.iter().collect::<String>()), TerminalColor::Red);
        }
        
        ctx.print_line("");
        ctx.print_line(&format!("Players: {}", state.players.join(", ")));
        
        if state.players.len() >= 2 && !state.finished {
            ctx.print_line(&format!("Current turn: {}", state.players.get(state.current_turn).unwrap_or(&"?".to_string())));
        }
        
        ctx.print_line("");
        ctx.print_colored(&format!("📢 {}", state.message), TerminalColor::Yellow);
        ctx.print_line("");
        
        if !state.finished && state.players.len() >= 2 {
            ctx.print_line("💡 Type a letter to guess, or 'q' to quit");
        } else if state.finished {
            ctx.print_line("🏁 Game over! Press 'q' to quit");
        } else {
            ctx.print_line("⏳ Waiting for more players...");
        }
        
        ctx.flush();
    }
}