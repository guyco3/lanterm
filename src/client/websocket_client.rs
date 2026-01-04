/// WebSocket-based game client - event-driven and clean!
use std::io::{self, Write};
use crossterm::{
    event::{self, KeyCode, KeyEvent},
    terminal::{self},
};
use tokio::time::Duration;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures_util::{SinkExt, StreamExt};
use serde::{Serialize, Deserialize};

use crate::core::websocket::GameMessage;

/// WebSocket game client
pub struct WebSocketGameClient {
    player_name: String,
}

impl WebSocketGameClient {
    pub fn new(name: String) -> Self {
        Self {
            player_name: name,
        }
    }

    /// Connect and run the event-driven game loop
    pub async fn connect_and_play<State, Input, F, I>(
        &mut self,
        url: &str,
        mut render_fn: F,
        mut input_fn: I,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
    where
        State: for<'de> Deserialize<'de> + Serialize + Send + Sync + std::fmt::Debug + 'static,
        Input: Serialize + for<'de> Deserialize<'de> + Send + Sync + 'static,
        F: FnMut(&State),
        I: FnMut(&str) -> Option<Input>,
    {
        // Connect without printing - let game handle all output
        let (ws_stream, _) = connect_async(url).await?;
        let (mut ws_sender, mut ws_receiver) = ws_stream.split();
        
        // Send join message
        let join_json = format!("{{\"PlayerJoin\":{{\"name\":\"{}\"}}}}", self.player_name);
        ws_sender.send(Message::Text(join_json)).await?;
        
        // Enable raw mode for input
        terminal::enable_raw_mode()?;
        
        let result = self.run_game_loop(&mut ws_sender, &mut ws_receiver, &mut render_fn, &mut input_fn).await;
        
        // Always disable raw mode
        terminal::disable_raw_mode()?;
        
        result
    }

    async fn run_game_loop<State, Input, F, I>(
        &mut self,
        ws_sender: &mut futures_util::stream::SplitSink<tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>, Message>,
        ws_receiver: &mut futures_util::stream::SplitStream<tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>>,
        render_fn: &mut F,
        input_fn: &mut I,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
    where
        State: for<'de> Deserialize<'de> + Send + Sync + std::fmt::Debug + 'static,
        Input: Serialize + for<'de> Deserialize<'de> + Send + Sync + 'static,
        F: FnMut(&State),
        I: FnMut(&str) -> Option<Input>,
    {
        let mut last_state: Option<String> = None;
        let mut input_line = String::new();

        loop {
            tokio::select! {
                // Handle keyboard input - line-based for coordinates!
                _ = tokio::time::sleep(Duration::from_millis(50)) => {
                    if event::poll(Duration::from_millis(1))? {
                        if let event::Event::Key(KeyEvent { code, .. }) = event::read()? {
                            match code {
                                KeyCode::Char('q') => {
                                    // Just close the connection, server will handle cleanup
                                    break;
                                }
                                KeyCode::Enter => {
                                    // Process the complete line
                                    if let Some(input) = input_fn(&input_line) {
                                        if let Ok(input_json) = serde_json::to_string(&input) {
                                            let msg = format!("{{\"PlayerInput\":{}}}", input_json);
                                            let _ = ws_sender.send(Message::Text(msg)).await;
                                        }
                                    }
                                    input_line.clear();
                                }
                                KeyCode::Backspace => {
                                    if !input_line.is_empty() {
                                        input_line.pop();
                                    }
                                }
                                KeyCode::Char(c) => {
                                    input_line.push(c);
                                }
                                _ => {}
                            }
                        }
                    }
                }
                
                // Handle WebSocket messages - pure events!
                msg_result = ws_receiver.next() => {
                    match msg_result {
                        Some(Ok(Message::Text(text))) => {
                            if let Ok(game_msg) = serde_json::from_str::<GameMessage<State, Input>>(&text) {
                                match game_msg {
                                    GameMessage::StateUpdate(state) => {
                                        // Only render if state changed
                                        let state_str = format!("{:?}", state);
                                        if last_state.as_ref() != Some(&state_str) {
                                            render_fn(&state);
                                            last_state = Some(state_str);
                                        }
                                    }
                                    GameMessage::Message(msg) => {
                                        // Don't print messages here - they interfere with game rendering
                                        // The game renderer handles all output
                                    }
                                    GameMessage::Error(err) => {
                                        // Only print critical errors that need immediate attention
                                        if err.contains("disconnect") || err.contains("connection") {
                                            eprintln!("❌ {}", err);
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                        Some(Ok(Message::Close(_))) => {
                            break;
                        }
                        Some(Err(e)) => {
                            // Only log to stderr without newlines to avoid terminal interference
                            eprintln!("WebSocket error: {}", e);
                            break;
                        }
                        None => {
                            break;
                        }
                        _ => {}
                    }
                }
            }
        }
        
        Ok(())
    }
}