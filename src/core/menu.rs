use ratatui::{
    widgets::{Block, Borders, List, ListItem, Paragraph},
    layout::{Layout, Constraint, Direction},
    style::{Style, Color, Modifier},
    Frame,
};
use crossterm::event::{self, Event, KeyCode};
use std::time::Duration;
use anyhow::Result;

pub enum LobbyResult {
    Host(String), // Game ID
    Join(String), // Peer ID
    Quit,
}

pub struct LobbyManager {
    pub selected_game_index: usize,
    pub input_mode: bool,
    pub peer_id_input: String,
}

impl LobbyManager {
    pub fn new() -> Self {
        Self {
            selected_game_index: 0,
            input_mode: false,
            peer_id_input: String::new(),
        }
    }

    pub fn run(&mut self, terminal: &mut ratatui::DefaultTerminal, games: &[crate::games::GameInfo]) -> Result<LobbyResult> {
        loop {
            terminal.draw(|f| self.render(f, games))?;

            if event::poll(Duration::from_millis(16))? {
                if let Event::Key(key) = event::read()? {
                    if self.input_mode {
                        match key.code {
                            KeyCode::Enter => return Ok(LobbyResult::Join(self.peer_id_input.clone())),
                            KeyCode::Esc => self.input_mode = false,
                            KeyCode::Char(c) => self.peer_id_input.push(c),
                            KeyCode::Backspace => { self.peer_id_input.pop(); }
                            _ => {}
                        }
                    } else {
                        match key.code {
                            KeyCode::Char('h') => {
                                let id = games[self.selected_game_index].id;
                                return Ok(LobbyResult::Host(id.to_string()));
                            }
                            KeyCode::Char('j') => self.input_mode = true,
                            KeyCode::Up => self.selected_game_index = self.selected_game_index.saturating_sub(1),
                            KeyCode::Down => self.selected_game_index = (self.selected_game_index + 1).min(games.len() - 1),
                            KeyCode::Char('q') | KeyCode::Esc => return Ok(LobbyResult::Quit),
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    fn render(&self, f: &mut Frame, games: &[crate::games::GameInfo]) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(2)
            .constraints([Constraint::Length(3), Constraint::Min(0), Constraint::Length(3)])
            .split(f.area());

        f.render_widget(
            Paragraph::new(" LANTERM P2P ARCADE ")
                .block(Block::default().borders(Borders::ALL))
                .alignment(ratatui::layout::Alignment::Center), 
            chunks[0]
        );

        if self.input_mode {
            f.render_widget(
                Paragraph::new(format!("Enter Host Node ID (PeerID):\n\n > {}", self.peer_id_input))
                    .block(Block::default().title(" JOIN SESSION ").borders(Borders::ALL)),
                chunks[1]
            );
        } else {
            let items: Vec<ListItem> = games.iter().enumerate().map(|(i, g)| {
                let style = if i == self.selected_game_index {
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                ListItem::new(format!(" » {} : {}", g.name, g.description)).style(style)
            }).collect();

            f.render_widget(
                List::new(items).block(Block::default().title(" AVAILABLE GAMES ").borders(Borders::ALL)), 
                chunks[1]
            );
            
            f.render_widget(
                Paragraph::new("[↑/↓] Navigate  [H] Host Selected  [J] Join by ID  [Q] Quit")
                    .alignment(ratatui::layout::Alignment::Center),
                chunks[2]
            );
        }
    }
}