use crate::{Context, Game};
use ratatui::{widgets::{Paragraph, Block, Borders}, layout::{Rect, Alignment}};
use serde::{Deserialize, Serialize};
use crossterm::event::KeyCode;
use iroh::EndpointId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PongAction {
    Player1MoveUp,
    Player1MoveDown,
    Player2MoveUp,
    Player2MoveDown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PongState {
    pub ball_x: i16,
    pub ball_y: i16,
    pub ball_dx: i16,
    pub ball_dy: i16,
    pub paddle1_y: i16,  // Left paddle (host)
    pub paddle2_y: i16,  // Right paddle (client)
    pub score: u32,
}

impl Default for PongState {
    fn default() -> Self {
        Self {
            ball_x: 30,
            ball_y: 10,
            ball_dx: 1,
            ball_dy: 1,
            paddle1_y: 8,
            paddle2_y: 8,
            score: 0,
        }
    }
}

pub struct PongGame {
    is_host: bool,
}

impl PongGame {
    pub fn new(is_host: bool) -> Self {
        Self { is_host }
    }
}

impl Game for PongGame {
    type Action = PongAction;
    type State = PongState;

    fn handle_input(&mut self, event: crossterm::event::KeyEvent, ctx: &Context<Self::Action>, _me: EndpointId) {
        match event.code {
            KeyCode::Up => {
                if self.is_host {
                    ctx.send_action(PongAction::Player1MoveUp);
                } else {
                    ctx.send_action(PongAction::Player2MoveUp);
                }
            }
            KeyCode::Down => {
                if self.is_host {
                    ctx.send_action(PongAction::Player1MoveDown);
                } else {
                    ctx.send_action(PongAction::Player2MoveDown);
                }
            }
            _ => {}
        }
    }

    fn handle_action(&self, action: Self::Action, state: &mut Self::State, _player: EndpointId) {
        match action {
            PongAction::Player1MoveUp => state.paddle1_y = (state.paddle1_y - 1).max(0),
            PongAction::Player1MoveDown => state.paddle1_y = (state.paddle1_y + 1).min(16),
            PongAction::Player2MoveUp => state.paddle2_y = (state.paddle2_y - 1).max(0),
            PongAction::Player2MoveDown => state.paddle2_y = (state.paddle2_y + 1).min(16),
        }
    }

    fn on_tick(&self, _dt: u32, state: &mut Self::State) {
        // Physics logic (Host Only)
        state.ball_x += state.ball_dx;
        state.ball_y += state.ball_dy;

        // Bounce off top/bottom
        if state.ball_y <= 0 || state.ball_y >= 20 {
            state.ball_dy *= -1;
        }

        // Left Paddle Collision (Player 1)
        if state.ball_x <= 2 && (state.ball_y >= state.paddle1_y && state.ball_y <= state.paddle1_y + 4) {
            state.ball_dx = state.ball_dx.abs();
            state.score += 1;
        }

        // Right Paddle Collision (Player 2) - use position relative to expected game width
        // The right paddle is rendered at area.width - 3, so collision at area.width - 4
        const GAME_WIDTH: i16 = 60;
        if state.ball_x >= GAME_WIDTH - 3 && (state.ball_y >= state.paddle2_y && state.ball_y <= state.paddle2_y + 4) {
            state.ball_dx = -state.ball_dx.abs();
            state.score += 1;
        }

        // Reset if missed (use game width as boundary)
        if state.ball_x < 0 || state.ball_x > GAME_WIDTH {
            state.ball_x = 30;
            state.ball_y = 10;
            state.ball_dx = if state.ball_x < 0 { 1 } else { -1 };
            state.ball_dy = 1;
        }
    }

    fn render(&self, frame: &mut ratatui::Frame, state: &Self::State) {
        let area = frame.area();
        
        // Render Left Paddle (Player 1)
        let paddle1 = Rect::new(2, state.paddle1_y as u16, 1, 4);
        frame.render_widget(Block::default().borders(Borders::ALL), paddle1);

        // Render Right Paddle (Player 2) at fixed position 57
        let paddle2 = Rect::new(57, state.paddle2_y as u16, 1, 4);
        frame.render_widget(Block::default().borders(Borders::ALL), paddle2);

        // Render Ball
        let ball = Rect::new(state.ball_x as u16, state.ball_y as u16, 1, 1);
        frame.render_widget(Paragraph::new("O"), ball);

        // Render Score
        let score_text = format!("Score: {}", state.score);
        frame.render_widget(
            Paragraph::new(score_text).alignment(Alignment::Center), 
            Rect::new(0, 0, area.width, 1)
        );
    }
}
