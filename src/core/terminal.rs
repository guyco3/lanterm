/// Terminal rendering context that handles all the low-level terminal stuff
use crossterm::{QueueableCommand, cursor, terminal, style::{SetForegroundColor, ResetColor, Color}};
use std::io::{stdout, Write, Stdout};

pub struct TerminalContext {
    out: Stdout,
}

impl TerminalContext {
    pub fn new() -> Self {
        let mut out = stdout();
        // Clear screen and position cursor
        out.queue(cursor::MoveTo(0, 0)).unwrap();
        out.queue(terminal::Clear(terminal::ClearType::All)).unwrap();
        
        Self { out }
    }

    /// Clear the entire screen and position cursor
    pub fn clear_screen(&mut self) {
        self.out.queue(cursor::MoveTo(0, 0)).unwrap();
        self.out.queue(terminal::Clear(terminal::ClearType::All)).unwrap();
    }

    /// Print a line with automatic carriage return - no more \r boilerplate!
    pub fn print_line(&mut self, text: &str) {
        writeln!(self.out, "{}\r", text).unwrap();
    }

    /// Print colored text with automatic reset
    pub fn print_colored(&mut self, text: &str, color: TerminalColor) {
        self.out.queue(SetForegroundColor(color.into())).unwrap();
        write!(self.out, "{}", text).unwrap();
        self.out.queue(ResetColor).unwrap();
    }

    /// Print a colored line
    pub fn print_colored_line(&mut self, text: &str, color: TerminalColor) {
        self.out.queue(SetForegroundColor(color.into())).unwrap();
        writeln!(self.out, "{}\r", text).unwrap();
        self.out.queue(ResetColor).unwrap();
    }

    /// Print empty line
    pub fn empty_line(&mut self) {
        writeln!(self.out, "\r").unwrap();
    }

    /// Print text without newline
    pub fn print(&mut self, text: &str) {
        write!(self.out, "{}", text).unwrap();
    }

    /// Flush all output at once - call this at the end of render
    pub fn flush(&mut self) {
        self.out.flush().unwrap();
    }
}

#[derive(Clone, Copy)]
pub enum TerminalColor {
    Red,
    Green, 
    Blue,
    Yellow,
    Cyan,
    White,
    Default,
}

impl From<TerminalColor> for Color {
    fn from(color: TerminalColor) -> Self {
        match color {
            TerminalColor::Red => Color::Red,
            TerminalColor::Green => Color::Green,
            TerminalColor::Blue => Color::Blue,
            TerminalColor::Yellow => Color::Yellow,
            TerminalColor::Cyan => Color::Cyan,
            TerminalColor::White => Color::White,
            TerminalColor::Default => Color::Reset,
        }
    }
}