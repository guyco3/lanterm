use serde::{Deserialize, Serialize};
use crate::core::game::{LantermGame, NodeId};
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
        let mut rng = rand::rng();

        loop {
            let row = rng.random_range(0..BOARD_SIZE);
            let col = rng.random_range(0..BOARD_SIZE);
            let direction = rng.random::<bool>();

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
    pub players: Vec<NodeId>,
    pub player_boards: Vec<Board>,
    pub current_turn: usize,
    pub current_turn_node: Option<NodeId>,
    pub last_action: String,
    pub finished: bool,
    pub winner: Option<NodeId>,
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
            current_turn_node: None,
            last_action: "Welcome to Battleship! Waiting for 2 players...".to_string(),
            finished: false,
            winner: None,
        }
    }

    pub fn add_player(&mut self, node_id: NodeId) {
        if !self.players.contains(&node_id) && self.players.len() < 2 {
            self.players.push(node_id);
            
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
                self.current_turn_node = Some(self.players[0]);
                self.last_action = format!("🚢 Battle begins! Player {} fires first.", self.players[0]);
            } else {
                self.last_action = format!("Player {} joined! Waiting for one more player...", node_id);
            }
        }
    }

    pub fn switch_turn(&mut self) {
        if self.players.len() == 2 && !self.finished {
            self.current_turn = (self.current_turn + 1) % 2;
            self.current_turn_node = Some(self.players[self.current_turn]);
        }
    }

    pub fn my_board(&self, node_id: NodeId) -> Option<&Board> {
        self.players.iter().position(|&id| id == node_id)
            .and_then(|idx| self.player_boards.get(idx))
    }

    pub fn opponent_view(&self, node_id: NodeId) -> Option<Board> {
        let opponent_idx = if self.players.get(0) == Some(&node_id) { 1 } else { 0 };
        self.player_boards.get(opponent_idx).map(|board| {
            let mut view = board.clone();
            // Hide ships that haven't been hit
            for row in view.grid.iter_mut() {
                for cell in row.iter_mut() {
                    if *cell == CellState::Ship {
                        *cell = CellState::Empty;
                    }
                }
            }
            view
        })
    }
}

/// Pure battleship game implementation
#[derive(Clone, Debug)]
pub struct BattleshipGame;

impl LantermGame for BattleshipGame {
    type State = BattleshipState;
    type Input = BattleshipInput;
    
    fn new_game() -> Self::State {
        BattleshipState::new()
    }
    
    fn handle_input(state: &mut Self::State, input: Self::Input, player_id: NodeId) {
        match input {
            BattleshipInput::Fire { row, col } => {
                if Some(player_id) == state.current_turn_node {
                    // Process the shot
                    let opponent_idx = if state.players[0] == player_id { 1 } else { 0 };
                    let result = state.player_boards[opponent_idx].fire(row, col);
                    
                    // Update last action message
                    state.last_action = match result {
                        CellState::Hit => {
                            if state.player_boards[opponent_idx].is_game_over() {
                                state.finished = true;
                                state.winner = Some(player_id);
                                format!("🎯 Direct hit! 🏆 Player {} wins!", player_id)
                            } else {
                                format!("🎯 Direct hit! Player {} damaged enemy ship!", player_id)
                            }
                        },
                        CellState::Miss => format!("💦 Player {} missed!", player_id),
                        _ => format!("Player {} fired at already targeted location", player_id),
                    };
                    
                    // Switch turns if miss or game over
                    if result == CellState::Miss || state.finished {
                        state.switch_turn();
                    }
                }
            }
        }
    }
}

