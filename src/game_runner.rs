/// Game runner that handles P2P networking, lobby, and game state management
use anyhow::Result;
use iroh::Endpoint;
use rand::rng;
use tracing::{info, warn};
use ratatui::prelude::*;
use serde::{Serialize, Deserialize};

use crate::core::game::{LantermGame, LantermRenderer, NodeId};
use crate::games::battleship::{BattleshipGame, BattleshipRenderer};

const ALPN: &[u8] = b"lanterm/battleship/0";

#[derive(Debug, Clone, Serialize, Deserialize)]
enum NetworkMessage {
    PlayerJoined { node_id: NodeId },
    GameInput { row: usize, col: usize },
    StateSync { state: Vec<u8> },
}

#[derive(Debug, Clone, PartialEq)]
pub enum GameRunnerState {
    Lobby { player_count: usize },
    Playing,
    Finished,
}

#[derive(Debug)]
pub struct GameRunner<G: LantermGame, R: LantermRenderer<G::State>> {
    pub endpoint: Endpoint,
    pub local_node_id: NodeId,
    pub is_host: bool,
    state: GameRunnerState,
    game_state: G::State,
    pub remote_node_id: Option<NodeId>,
    _phantom: std::marker::PhantomData<(G, R)>,
}

impl GameRunner<BattleshipGame, BattleshipRenderer> {
    pub async fn new_host(endpoint: Endpoint) -> Result<Self> {
        let local_node_id = endpoint.id();
        let game_state = BattleshipGame::new_game();
        
        Ok(Self {
            endpoint,
            local_node_id,
            is_host: true,
            state: GameRunnerState::Lobby { player_count: 0 },
            game_state,
            remote_node_id: None,
            _phantom: std::marker::PhantomData,
        })
    }
    
    pub async fn new_client(endpoint: Endpoint, host_node_id: NodeId) -> Result<Self> {
        let local_node_id = endpoint.id();
        let game_state = BattleshipGame::new_game();
        
        // Attempt to connect to host
        let conn = endpoint.connect(host_node_id, ALPN).await?;
        let (mut send, _recv) = conn.open_bi().await?;
        info!(host=?host_node_id, "client connected to host");
        
        // Send join message
        let join_msg = NetworkMessage::PlayerJoined { node_id: local_node_id };
        let msg_bytes = bincode::serialize(&join_msg)?;
        send.write_all(&(msg_bytes.len() as u32).to_le_bytes()).await?;
        send.write_all(&msg_bytes).await?;
        send.finish()?;
        info!(local=?local_node_id, "client sent PlayerJoined");
        
        Ok(Self {
            endpoint,
            local_node_id,
            is_host: false,
            state: GameRunnerState::Lobby { player_count: 0 },
            game_state,
            remote_node_id: Some(host_node_id),
            _phantom: std::marker::PhantomData,
        })
    }
    
    pub async fn accept_connection(&mut self) -> Result<()> {
        if !self.is_host {
            return Ok(());
        }
        
        // Accept incoming connection
        let incoming = self.endpoint.accept().await;
        if let Some(connecting) = incoming {
            info!("host accepted incoming connection");
            let conn = connecting.await?;
            let (_send, mut recv) = conn.accept_bi().await?;

            // Read join message
            let mut len_bytes = [0u8; 4];
            let read_len = recv.read_exact(&mut len_bytes).await;
            if read_len.is_err() {
                warn!("failed to read length; adding remote anyway");
                let mut rng = rng();
                let sk = iroh::SecretKey::generate(&mut rng);
                let node_id = sk.public();
                self.remote_node_id = Some(node_id);
                self.add_remote_player(node_id);
                return Ok(());
            }
            let len = u32::from_le_bytes(len_bytes) as usize;
            
            let mut msg_bytes = vec![0u8; len];
            if let Err(e) = recv.read_exact(&mut msg_bytes).await {
                warn!(error=?e, "failed to read join payload; adding remote anyway");
                let mut rng = rng();
                let sk = iroh::SecretKey::generate(&mut rng);
                let node_id = sk.public();
                self.remote_node_id = Some(node_id);
                self.add_remote_player(node_id);
                return Ok(());
            }
            
            match bincode::deserialize(&msg_bytes) {
                Ok(NetworkMessage::PlayerJoined { node_id }) => {
                    info!(remote=?node_id, "host received PlayerJoined");
                    self.remote_node_id = Some(node_id);
                    self.add_remote_player(node_id);
                },
                Ok(other) => {
                    warn!(?other, "unexpected network message in lobby; adding placeholder");
                    let mut rng = rng();
                    let sk = iroh::SecretKey::generate(&mut rng);
                    let node_id = sk.public();
                    self.remote_node_id = Some(node_id);
                    self.add_remote_player(node_id);
                },
                Err(e) => {
                    warn!(error=?e, "failed to deserialize network message; adding placeholder");
                    let mut rng = rng();
                    let sk = iroh::SecretKey::generate(&mut rng);
                    let node_id = sk.public();
                    self.remote_node_id = Some(node_id);
                    self.add_remote_player(node_id);
                }
            }
        }
        
        Ok(())
    }
    
    #[allow(dead_code)]
    pub async fn send_input(&mut self, row: usize, col: usize) -> Result<()> {
        if let Some(remote_id) = self.remote_node_id {
            let conn = self.endpoint.connect(remote_id, ALPN).await?;
            let (mut send, _recv) = conn.open_bi().await?;
            
            let msg = NetworkMessage::GameInput { row, col };
            let msg_bytes = bincode::serialize(&msg)?;
            send.write_all(&(msg_bytes.len() as u32).to_le_bytes()).await?;
            send.write_all(&msg_bytes).await?;
            send.finish()?;
        }
        Ok(())
    }
    
    pub fn add_local_player(&mut self) {
        self.game_state.add_player(self.local_node_id);
        if let GameRunnerState::Lobby { ref mut player_count } = self.state {
            *player_count += 1;
            if *player_count == 2 {
                self.state = GameRunnerState::Playing;
            }
        }
    }
    
    pub fn add_remote_player(&mut self, node_id: NodeId) {
        self.game_state.add_player(node_id);
        if let GameRunnerState::Lobby { ref mut player_count } = self.state {
            *player_count += 1;
            if *player_count == 2 {
                self.state = GameRunnerState::Playing;
            }
        }
    }
    
    pub fn handle_input(&mut self, input: <BattleshipGame as LantermGame>::Input) {
        BattleshipGame::handle_input(&mut self.game_state, input, self.local_node_id);
        
        if self.game_state.finished {
            self.state = GameRunnerState::Finished;
        }
    }
    
    pub fn render(&self, frame: &mut Frame) {
        BattleshipRenderer::render(frame, &self.game_state, self.local_node_id);
    }
    
    pub fn game_state(&self) -> &<BattleshipGame as LantermGame>::State {
        &self.game_state
    }
}
