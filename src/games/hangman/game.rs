use serde::{Deserialize, Serialize};
use crate::core::game::WebSocketGame;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HangmanState {
    pub word: String,
    pub masked_word: String,
    pub guessed: Vec<char>,
    pub wrong: Vec<char>,
    pub remaining_tries: u8,
    pub players: Vec<String>,
    pub current_turn: usize,
    pub message: String,
    pub finished: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum HangmanInput {
    Guess(char),
}

impl HangmanState {
    pub fn new(word: String) -> Self {
        let word = word.to_uppercase();
        let masked_word = word.chars().map(|c| if c.is_ascii_alphabetic() { '_' } else { c }).collect();
        
        Self {
            word,
            masked_word,
            guessed: Vec::new(),
            wrong: Vec::new(),
            remaining_tries: 6,
            players: Vec::new(),
            current_turn: 0,
            message: "Welcome to Hangman!".to_string(),
            finished: false,
        }
    }
    
    pub fn add_player(&mut self, name: String) {
        if !self.players.contains(&name) {
            self.players.push(name);
            if self.players.len() == 2 {
                self.message = format!("Game started! {} goes first.", self.players[0]);
            }
        }
    }
    
    fn update_masked_word(&mut self) {
        self.masked_word = self.word.chars()
            .map(|c| {
                if c.is_ascii_alphabetic() {
                    if self.guessed.contains(&c) { c } else { '_' }
                } else { c }
            })
            .collect();
    }
    
    fn is_word_complete(&self) -> bool {
        self.word.chars()
            .filter(|c| c.is_ascii_alphabetic())
            .all(|c| self.guessed.contains(&c))
    }
    
    pub fn guess(&mut self, letter: char, player_name: &str) -> Result<bool, String> {
        if self.players.len() < 2 {
            return Err("Need at least 2 players to start".to_string());
        }
        
        if self.finished {
            return Err("Game is finished".to_string());
        }
        
        let current_player = self.players.get(self.current_turn).unwrap();
        if current_player != player_name {
            return Err(format!("Not your turn! It's {}'s turn.", current_player));
        }
        
        let letter = letter.to_ascii_uppercase();
        
        if !letter.is_ascii_alphabetic() {
            return Err("Please guess a letter A-Z".to_string());
        }
        
        if self.guessed.contains(&letter) || self.wrong.contains(&letter) {
            return Err(format!("Letter '{}' already guessed", letter));
        }
        
        let is_correct = self.word.contains(letter);
        
        if is_correct {
            self.guessed.push(letter);
            self.update_masked_word();
            
            if self.is_word_complete() {
                self.finished = true;
                self.message = format!("🎉 {} won! The word was '{}'.", player_name, self.word);
                return Ok(true);
            }
            
            self.message = format!("Good guess! '{}' is in the word.", letter);
        } else {
            self.wrong.push(letter);
            self.remaining_tries -= 1;
            
            if self.remaining_tries == 0 {
                self.finished = true;
                self.message = format!("💀 Game over! The word was '{}'.", self.word);
                return Ok(false);
            }
            
            self.message = format!("Sorry, '{}' is not in the word.", letter);
        }
        
        self.current_turn = (self.current_turn + 1) % self.players.len().min(2);
        
        if !self.finished {
            let next_player = &self.players[self.current_turn];
            self.message = format!("{}  Next: {}", self.message, next_player);
        }
        
        Ok(is_correct)
    }
}

/// Pure game implementation - no UI or transport concerns
#[derive(Clone)]
pub struct HangmanGame;

impl WebSocketGame for HangmanGame {
    type State = HangmanState;
    type Input = HangmanInput;
    
    // Metadata directly in game - no factory needed!
    const NAME: &'static str = "Hangman";
    const DESCRIPTION: &'static str = "Guess the word letter by letter using WebSocket";
    const MIN_PLAYERS: usize = 1;
    const MAX_PLAYERS: usize = 4;
    
    fn new_game() -> Self::State {
        let words = ["EXAMPLE", "WEBSOCKET", "RUST", "ASYNC", "TOKIO", "HANGMAN", "TERMINAL", "NETWORK", "SOCKET"];
        let random_word = words[rand::random::<usize>() % words.len()];
        
        println!("🎯 Secret word: {} (for demo)", random_word);
        HangmanState::new(random_word.to_string())
    }
    
    fn handle_input(input: &Self::Input, state: &mut Self::State, player_name: &str) -> String {
        match input {
            HangmanInput::Guess(letter) => {
                if !state.players.contains(&player_name.to_string()) {
                    state.add_player(player_name.to_string());
                }
                
                match state.guess(*letter, player_name) {
                    Ok(correct) => {
                        if correct {
                            format!("✅ Good guess! '{}' is in the word.", letter)
                        } else {
                            format!("❌ Sorry, '{}' is not in the word.", letter)
                        }
                    }
                    Err(e) => e,
                }
            }
        }
    }
    
    /// Game developer controls input parsing - no framework interference
    fn parse_line(line: &str) -> Option<Self::Input> {
        // Extract first alphabetic character from line
        line.chars().find(|c| c.is_ascii_alphabetic())
            .map(HangmanInput::Guess)
    }
}
