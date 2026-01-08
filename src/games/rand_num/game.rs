use crate::{Context, Game};
use ratatui::widgets::Paragraph;
use serde::{Deserialize, Serialize};
use crossterm::event::KeyCode;
use iroh::EndpointId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GuessAction {
    Submit(u32),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NumberState {
    pub message: String,
    pub over: bool,
}

pub struct NumberGame {
    local_input: String,
    #[allow(dead_code)]
    is_host: bool,
    #[allow(dead_code)]
    my_id: EndpointId,
}

impl NumberGame {
    pub fn new(is_host: bool, my_id: EndpointId) -> Self {
        Self { 
            local_input: String::new(),
            is_host,
            my_id,
        }
    }
}

impl Game for NumberGame {
    type Action = GuessAction;
    type State = NumberState;

    fn handle_input(&mut self, event: crossterm::event::KeyEvent, ctx: &Context<Self::Action>, _me: EndpointId) {
        match event.code {
            KeyCode::Char(c) if c.is_digit(10) => self.local_input.push(c),
            KeyCode::Backspace => { self.local_input.pop(); },
            KeyCode::Enter => {
                if let Ok(val) = self.local_input.parse() {
                    ctx.send_action(GuessAction::Submit(val));
                }
                self.local_input.clear();
            }
            _ => {}
        }
    }

    fn handle_action(&self, action: Self::Action, state: &mut Self::State, _player: EndpointId) {
        match action {
            GuessAction::Submit(val) => {
                state.message = format!("Last guess was: {}", val);
                if val == 42 { // Simple win condition
                    state.over = true;
                    state.message = "42! You win!".into();
                }
            }
        }
    }

    fn on_tick(&self, _dt: u32, _state: &mut Self::State) {}

    fn render(&self, frame: &mut ratatui::Frame, state: &Self::State) {
        let text = format!(
            "--- SHARED WORLD ---\nStatus: {}\nYour Typing: {}\nGame Over: {}",
            state.message, self.local_input, state.over
        );
        frame.render_widget(Paragraph::new(text), frame.area());
    }
}