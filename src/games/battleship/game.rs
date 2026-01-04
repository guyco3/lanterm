use serde::{Deserialize, Serialize};
use crate::core::game::WebSocketGame;
use rand::Rng;

const BOARD_SIZE: usize = 10;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum CellState {
    Empty,
    Ship,
    Hit,
    Miss,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Board {
    grid: [[CellState; BOARD_SIZE]; BOARD_SIZE],
    ships: Vec<(usize, usize)>,
}

impl Board {
    fn new() -> Self {
        Board {
            grid: [[CellState::Empty; BOARD_SIZE]; BOARD_SIZE],
            ships: Vec::new(),
        }
    }

    fn place_ship(&mut self, size: usize) {
        let mut rng = rand::thread_rng();

        loop {
            let row = rng.gen_range(0..BOARD_SIZE);
            let col = rng.gen_range(0..BOARD_SIZE);
            let direction = rng.gen::<bool>();

            if self.can_place_ship(row, col, size, direction) {
                for i in 0..size {
                    let (r, c) = if direction { (row, col + i) } else { (row + i, col) };
                    self.grid[r][c] = CellState::Ship;
                    self.ships.push((r, c));
                }
                break;
            }
        }
    }

    fn can_place_ship(&self, row: usize, col: usize, size: usize, direction: bool) -> bool {
        if direction {
            if col + size > BOARD_SIZE { return false; }
            for i in 0..size {
                if self.grid[row][col + i] != CellState::Empty { return false; }
            }
        } else {
            if row + size > BOARD_SIZE { return false; }
            for i in 0..size {
                if self.grid[row + i][col] != CellState::Empty { return false; }
            }
        }
        true
    }

    fn fire(&mut self, row: usize, col: usize) -> CellState {
        match self.grid[row][col] {
            CellState::Empty => {
                self.grid[row][col] = CellState::Miss;
                CellState::Miss
            },
            CellState::Ship => {
                self.grid[row][col] = CellState::Hit;
                CellState::Hit
            },
            _ => CellState::Miss,
        }
    }

    fn is_game_over(&self) -> bool {
        self.ships.iter().all(|&(r, c)| self.grid[r][c] == CellState::Hit)
    }

    pub fn grid(&self) -> &[[CellState; BOARD_SIZE]; BOARD_SIZE] {
        &self.grid
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BattleshipState {
    pub players: Vec<String>,
    pub player_boards: Vec<Board>,
    pub current_turn: usize,
    pub message: String,
    pub finished: bool,
    pub winner: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum BattleshipInput {
    Fire { row: usize, col: usize },
}

impl BattleshipState {
    pub fn new() -> Self {
        Self {
            players: Vec::new(),
            player_boards: Vec::new(),
            current_turn: 0,
            message: "Welcome to Battleship! Waiting for 2 players...".to_string(),
            finished: false,
            winner: None,
        }
    }

    pub fn add_player(&mut self, name: String) {
        if !self.players.contains(&name) && self.players.len() < 2 {
            self.players.push(name.clone());
            
            // Create and set up board for new player
            let mut board = Board::new();
            // Place ships: Aircraft Carrier(5), Battleship(4), Cruiser(3), Submarine(3), Destroyer(2)
            board.place_ship(5);
            board.place_ship(4);
            board.place_ship(3);
            board.place_ship(3);
            board.place_ship(2);
            self.player_boards.push(board);
            
            if self.players.len() == 2 {
                self.message = format!("🚢 Battle begins! {} fires first.", self.players[0]);
            } else {
                self.message = format!("{} joined! Waiting for one more player...", name);
            }
        }
    }

    pub fn fire(&mut self, row: usize, col: usize, player_name: &str) -> Result<String, String> {
        if self.players.len() < 2 {
            return Err("Need 2 players to start battle!".to_string());
        }

        if self.finished {
            return Err("Battle is over!".to_string());
        }

        // Validate it's the player's turn
        let current_player = self.players.get(self.current_turn).unwrap();
        if current_player != player_name {
            return Err(format!("Not your turn! It's {}'s turn.", current_player));
        }

        // Validate coordinates
        if row >= BOARD_SIZE || col >= BOARD_SIZE {
            return Err(format!("Invalid coordinates! Use 0-{}", BOARD_SIZE - 1));
        }

        // Fire at opponent's board (opposite player)
        let opponent_idx = 1 - self.current_turn;
        let result = self.player_boards[opponent_idx].fire(row, col);
        
        let result_message = match result {
            CellState::Hit => {
                // Check if opponent is defeated
                if self.player_boards[opponent_idx].is_game_over() {
                    self.finished = true;
                    self.winner = Some(player_name.to_string());
                    format!("🎯 Direct hit! 🏆 {} wins the battle!", player_name)
                } else {
                    "🎯 Direct hit! Enemy ship damaged!".to_string()
                }
            },
            CellState::Miss => "💦 Missed! Your shot splashed harmlessly.".to_string(),
            _ => "🔄 Already fired here!".to_string(),
        };

        // Switch turns only if it was a miss or if game is over
        if result == CellState::Miss || self.finished {
            self.current_turn = (self.current_turn + 1) % 2;
        }

        if !self.finished && result != CellState::Miss {
            self.message = format!("{}  {} gets another turn!", result_message, player_name);
        } else if !self.finished {
            let next_player = &self.players[self.current_turn];
            self.message = format!("{}  {}'s turn to fire!", result_message, next_player);
        } else {
            self.message = result_message;
        }

        Ok(self.message.clone())
    }
}

/// Pure battleship game implementation
#[derive(Clone)]
pub struct BattleshipGame;

impl WebSocketGame for BattleshipGame {
    type State = BattleshipState;
    type Input = BattleshipInput;
    
    const NAME: &'static str = "Battleship";
    const DESCRIPTION: &'static str = "Naval combat - sink your opponent's fleet!";
    const MIN_PLAYERS: usize = 2;
    const MAX_PLAYERS: usize = 2;
    
    fn new_game() -> Self::State {
        BattleshipState::new()
    }
    
    fn handle_input(input: &Self::Input, state: &mut Self::State, player_name: &str) -> String {
        match input {
            BattleshipInput::Fire { row, col } => {
                // Ensure player is in game before firing
                if !state.players.contains(&player_name.to_string()) {
                    return "You must join the game first!".to_string();
                }
                
                match state.fire(*row, *col, player_name) {
                    Ok(message) => message,
                    Err(error) => error,
                }
            }
        }
    }
    
    /// Explicit join handling - much cleaner than magic coordinates!
    fn on_player_join(state: &mut Self::State, player_name: &str) -> String {
        if !state.players.contains(&player_name.to_string()) {
            state.add_player(player_name.to_string());
            format!("{} joined the battle!", player_name)
        } else {
            "You're already in the battle!".to_string()
        }
    }
    
    /// Parse coordinates from line input like "3,4" or "3 4"
    fn parse_line(line: &str) -> Option<Self::Input> {
        let coords: Result<Vec<usize>, _> = line
            .trim()
            .split(|c: char| c == ',' || c.is_whitespace())
            .filter(|s| !s.is_empty())
            .map(|s| s.parse())
            .collect();
        
        if let Ok(coords) = coords {
            if coords.len() == 2 && coords[0] < 10 && coords[1] < 10 {
                return Some(Self::Input::Fire { 
                    row: coords[0], 
                    col: coords[1] 
                });
            }
        }
        None
    }
}

