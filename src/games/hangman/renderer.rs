/// Hangman-specific renderer - injected dependency
use std::io::{stdout, Write};
use crossterm::{QueueableCommand, cursor, terminal};
use crate::core::renderer::GameRenderer;
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
    fn render(&self, state: &HangmanState) {
        let mut out = stdout();

        // Move cursor and clear with proper buffering
        out.queue(cursor::MoveTo(0, 0)).unwrap();
        out.queue(terminal::Clear(terminal::ClearType::All)).unwrap();
        
        // Render with carriage returns
        writeln!(out, "🎩 HANGMAN (WebSocket) - Player: {}\r", self.player_name).unwrap();
        writeln!(out, "══════════════════════════════════\r").unwrap();
        writeln!(out, "\r").unwrap();
        writeln!(out, "Word: {}\r", state.masked_word).unwrap();
        writeln!(out, "\r").unwrap();
        writeln!(out, "Tries left: {} {}\r", state.remaining_tries, "❤".repeat(state.remaining_tries as usize)).unwrap();
        
        if !state.guessed.is_empty() {
            println!("Correct: {}", state.guessed.iter().collect::<String>());
        }
        if !state.wrong.is_empty() {
            println!("Wrong: {}", state.wrong.iter().collect::<String>());
        }
        
        writeln!(out, "\r").unwrap();
        writeln!(out, "Players: {}\r", state.players.join(", ")).unwrap();
        
        if state.players.len() >= 2 && !state.finished {
            writeln!(out, "Current turn: {}\r", state.players.get(state.current_turn).unwrap_or(&"?".to_string())).unwrap();
        }
        
        writeln!(out, "\r").unwrap();
        writeln!(out, "📢 {}\r", state.message).unwrap();
        writeln!(out, "\r").unwrap();
        
        if !state.finished && state.players.len() >= 2 {
            writeln!(out, "💡 Type a letter to guess, or 'q' to quit\r").unwrap();
        } else if state.finished {
            writeln!(out, "🏁 Game over! Press 'q' to quit\r").unwrap();
        } else {
            writeln!(out, "⏳ Waiting for more players...\r").unwrap();
        }
        
        // Flush all output at once
        out.flush().unwrap();
    }
}