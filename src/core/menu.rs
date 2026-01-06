use anyhow::Result;
use crossterm::event::{self, KeyCode};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame, Terminal,
};
use std::io::Stdout;

use crate::games;

#[derive(Debug, Clone, Copy)]
pub enum MenuChoice {
    Host,
    Join,
}

#[derive(Debug, Clone)]
pub struct JoinConfig {
    pub node_id: String,
}

pub struct Menu {
    selected_game_index: usize,
    input_buffer: String,
    mode: MenuMode,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum MenuMode {
    HostOrJoin,
    SelectGame,
    JoinNodeId,
}

impl Menu {
    pub fn new() -> Self {
        Self {
            selected_game_index: 0,
            input_buffer: String::new(),
            mode: MenuMode::HostOrJoin,
        }
    }

    /// Run the menu and return the user's choice and game ID
    pub fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<(MenuChoice, String, Option<JoinConfig>)> {
        loop {
            terminal.draw(|f| self.draw(f))?;

            if event::poll(std::time::Duration::from_millis(100))? {
                if let event::Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Esc | KeyCode::Char('q') => {
                            return Err(anyhow::anyhow!("Menu cancelled"));
                        }
                        _ => {
                            if let Some(result) = self.handle_input(key.code) {
                                return Ok(result);
                            }
                        }
                    }
                }
            }
        }
    }

    fn draw(&self, f: &mut Frame) {
        let area = f.area();

        match self.mode {
            MenuMode::HostOrJoin => self.draw_host_or_join(f, area),
            MenuMode::SelectGame => self.draw_select_game(f, area),
            MenuMode::JoinNodeId => self.draw_join_node_id(f, area),
        }
    }

    fn draw_host_or_join(&self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .title("LanTerm - Choose Mode")
            .borders(Borders::ALL);

        let text = vec![
            Line::from(""),
            Line::from("  1) Host a Game"),
            Line::from("  2) Join a Game"),
            Line::from(""),
            Line::from("  Press ESC to quit"),
        ];

        f.render_widget(Paragraph::new(text).block(block).alignment(Alignment::Center), area);
    }

    fn draw_select_game(&self, f: &mut Frame, area: Rect) {
        let games = games::get_all_games();
        let block = Block::default()
            .title("Select Game")
            .borders(Borders::ALL);

        let mut lines = vec![Line::from("")];

        for (idx, game) in games.iter().enumerate() {
            let prefix = if idx == self.selected_game_index {
                "▶ "
            } else {
                "  "
            };

            let style = if idx == self.selected_game_index {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            lines.push(Line::from(vec![
                Span::styled(format!("{}{}", prefix, game.info.name), style),
            ]));

            lines.push(Line::from(format!("     {}", game.info.description)));
            lines.push(Line::from(""));
        }

        lines.push(Line::from("  ↑/↓: Navigate | ENTER: Select | ESC: Back"));

        f.render_widget(Paragraph::new(lines).block(block), area);
    }

    fn draw_join_node_id(&self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .title("Join Game - Enter Node ID")
            .borders(Borders::ALL);

        let text = vec![
            Line::from(""),
            Line::from("Enter the host's node ID:"),
            Line::from(""),
            Line::from(self.input_buffer.clone()),
            Line::from(""),
            Line::from("ENTER: Connect | ESC: Back"),
        ];

        f.render_widget(Paragraph::new(text).block(block), area);
    }

    fn handle_input(&mut self, code: KeyCode) -> Option<(MenuChoice, String, Option<JoinConfig>)> {
        match self.mode {
            MenuMode::HostOrJoin => match code {
                KeyCode::Char('1') => {
                    self.mode = MenuMode::SelectGame;
                    None
                }
                KeyCode::Char('2') => {
                    self.mode = MenuMode::JoinNodeId;
                    None
                }
                _ => None,
            },
            MenuMode::SelectGame => match code {
                KeyCode::Up => {
                    if self.selected_game_index > 0 {
                        self.selected_game_index -= 1;
                    }
                    None
                }
                KeyCode::Down => {
                    let games = games::get_all_games();
                    if self.selected_game_index < games.len() - 1 {
                        self.selected_game_index += 1;
                    }
                    None
                }
                KeyCode::Enter => {
                    let games = games::get_all_games();
                    let game_id = games[self.selected_game_index].info.id.to_string();
                    Some((MenuChoice::Host, game_id, None))
                }
                KeyCode::Esc => {
                    self.mode = MenuMode::HostOrJoin;
                    None
                }
                _ => None,
            },
            MenuMode::JoinNodeId => match code {
                KeyCode::Enter => {
                    if !self.input_buffer.is_empty() {
                        let node_id = self.input_buffer.clone();
                        Some((MenuChoice::Join, "pong".to_string(), Some(JoinConfig { node_id })))
                    } else {
                        None
                    }
                }
                KeyCode::Backspace => {
                    self.input_buffer.pop();
                    None
                }
                KeyCode::Char(c) => {
                    if self.input_buffer.len() < 64 {
                        self.input_buffer.push(c);
                    }
                    None
                }
                KeyCode::Esc => {
                    self.mode = MenuMode::HostOrJoin;
                    self.input_buffer.clear();
                    None
                }
                _ => None,
            },
        }
    }
}
