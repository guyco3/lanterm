use crate::{Context, Game};
use rand::Rng;
use ratatui::widgets::Paragraph;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GuessMsg {
    PlayerGuessed(u32),   
    Feedback(String),      
    GameOver(u32),
}

pub struct NumberGame {
    is_host: bool,
    secret_number: u32,
    last_feedback: String,
    current_input: String,
    game_over: bool,
}

impl NumberGame {
    pub fn new(is_host: bool) -> Self {
        let mut rng = rand::rng();
        let secret = if is_host { rng.random_range(1..100) } else { 0 };
        Self {
            is_host,
            secret_number: secret,
            last_feedback: if is_host { "Waiting for guess...".into() } else { "Enter a guess!".into() },
            current_input: String::new(),
            game_over: false,
        }
    }
}

impl Game for NumberGame {
    type Message = GuessMsg;

    // We don't need a tick for a guessing game!
    fn tick_rate(&self) -> Option<std::time::Duration> { None }
    fn on_tick(&mut self, _dt: u32, _ctx: &Context<Self::Message>) {}

    fn handle_input(&mut self, event: crossterm::event::KeyEvent, ctx: &Context<Self::Message>) {
        if self.game_over { return; }

        match event.code {
            crossterm::event::KeyCode::Char(c) => {
                if c.is_ascii_digit() {
                    self.current_input.push(c);
                }
            }
            crossterm::event::KeyCode::Backspace => {
                self.current_input.pop();
            }
            crossterm::event::KeyCode::Enter => {
                if let Ok(guess) = self.current_input.parse::<u32>() {
                    if !self.is_host {
                        // CLIENT: Send the guess to the host
                        ctx.send_network_event(GuessMsg::PlayerGuessed(guess));
                        self.last_feedback = format!("You guessed {}. Waiting for feedback...", guess);
                    }
                }
                self.current_input.clear();
            }
            _ => {}
        }
    }

    fn handle_network(&mut self, msg: Self::Message, ctx: &Context<Self::Message>) {
        match msg {
            GuessMsg::PlayerGuessed(guess) => {
                if self.is_host {
                    // HOST: Check the guess and send feedback
                    println!("Client guessed: {}", guess);
                    if guess < self.secret_number {
                        ctx.send_network_event(GuessMsg::Feedback("Too low!".into()));
                    } else if guess > self.secret_number {
                        ctx.send_network_event(GuessMsg::Feedback("Too high!".into()));
                    } else {
                        ctx.send_network_event(GuessMsg::GameOver(guess));
                        self.game_over = true;
                        self.last_feedback = format!("Player won! Number was {}", guess);
                    }
                }
            }
            GuessMsg::Feedback(text) => {
                self.last_feedback = text;
            }
            GuessMsg::GameOver(num) => {
                self.game_over = true;
                self.last_feedback = format!("Correct! The number was {}", num);
            }
        }
    }

    fn render(&self, frame: &mut ratatui::Frame) {
        let text = format!(
            "{}\n\nInput (0-99, Backspace to erase, Enter to send, Esc to quit):\n{}\n\nFeedback:\n{}", 
            if self.is_host { "HOST MODE - Waiting for guess..." } else { "CLIENT MODE - Type digits (0-99)" },
            self.current_input,
            self.last_feedback
        );
        frame.render_widget(Paragraph::new(text), frame.area());
    }
}