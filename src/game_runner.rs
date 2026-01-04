/// Game runner that handles P2P networking, lobby, and game state management
use anyhow::Result;
use iroh::Endpoint;
use rand::rng;
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, time::{timeout, Duration}};
use tracing::{info, warn};
use ratatui::prelude::*;
use serde::{Serialize, Deserialize};

use crate::core::game::{LantermGame, LantermRenderer, NodeId};
use crate::games::battleship::{BattleshipGame, BattleshipRenderer, game::BattleshipState};

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

    /// Replace placeholder remote with real node id if present; otherwise add as new remote.
    fn upsert_remote_player(&mut self, node_id: NodeId) {
        if let Some(pos) = self.game_state.players.iter().position(|&id| id != self.local_node_id) {
            if self.game_state.players[pos] != node_id {
                self.game_state.players[pos] = node_id;
            }
            self.remote_node_id = Some(node_id);
            if let Some(current) = self.game_state.current_turn_node {
                if current != self.local_node_id {
                    self.game_state.current_turn_node = Some(node_id);
                }
            }
        } else {
            self.remote_node_id = Some(node_id);
            self.add_remote_player(node_id);
        }
    }

    fn add_placeholder_remote(&mut self) {
        let mut rng = rng();
        let sk = iroh::SecretKey::generate(&mut rng);
        let node_id = sk.public();
        if self.remote_node_id.is_none() {
            self.remote_node_id = Some(node_id);
        }
        self.add_remote_player(node_id);
    }
    
    pub async fn accept_connection(&mut self) -> Result<()> {
        if !self.is_host {
            return Ok(());
        }

        // Bounded, non-blocking accept to keep UI responsive
        let incoming = match timeout(Duration::from_millis(50), self.endpoint.accept()).await {
            Ok(opt) => opt,
            Err(_) => return Ok(()),
        };

        if let Some(connecting) = incoming {
            info!("host accepted incoming connection");

            let conn = match timeout(Duration::from_millis(2000), connecting).await {
                Ok(c) => c?,
                Err(_) => {
                    warn!("timeout awaiting connecting");
                    return Ok(());
                }
            };
            info!("host got connection future resolved");

            let (_send, mut recv) = match timeout(Duration::from_millis(2000), conn.accept_bi()).await {
                Ok(res) => res?,
                Err(_) => {
                    warn!("timeout awaiting bi stream");
                    return Ok(());
                }
            };
            info!("host accepted bi stream");

            // Read join message length
            let mut len_bytes = [0u8; 4];
            match timeout(Duration::from_millis(2000), recv.read_exact(&mut len_bytes)).await {
                Ok(Ok(_)) => {}
                Ok(Err(e)) => {
                    warn!(error=?e, "failed to read length; adding remote anyway");
                    self.add_placeholder_remote();
                    return Ok(());
                }
                Err(_) => {
                    warn!("timeout reading length; adding remote anyway");
                    self.add_placeholder_remote();
                    return Ok(());
                }
            }
            let len = u32::from_le_bytes(len_bytes) as usize;
            info!(len, "host read message length");

            // Read join message payload
            let mut msg_bytes = vec![0u8; len];
            match timeout(Duration::from_millis(2000), recv.read_exact(&mut msg_bytes)).await {
                Ok(Ok(_)) => {}
                Ok(Err(e)) => {
                    warn!(error=?e, "failed to read join payload; adding remote anyway");
                    self.add_placeholder_remote();
                    return Ok(());
                }
                Err(_) => {
                    warn!("timeout reading join payload; adding remote anyway");
                    self.add_placeholder_remote();
                    return Ok(());
                }
            }
            info!(bytes=len, "host read message payload");

            match bincode::deserialize(&msg_bytes) {
                Ok(NetworkMessage::PlayerJoined { node_id }) => {
                    info!(remote=?node_id, "host received PlayerJoined");
                    self.upsert_remote_player(node_id);
                    self.send_state().await.ok();
                }
                Ok(other) => {
                    warn!(?other, "unexpected network message in lobby; adding placeholder");
                    self.add_placeholder_remote();
                }
                Err(e) => {
                    warn!(error=?e, "failed to deserialize network message; adding placeholder");
                    self.add_placeholder_remote();
                }
            }
        }

        Ok(())
    }
    
    pub fn add_local_player(&mut self) {
        self.game_state.add_player(self.local_node_id);
        info!(players=self.game_state.players.len(), "added local player");
        if let GameRunnerState::Lobby { ref mut player_count } = self.state {
            *player_count += 1;
            if *player_count == 2 {
                self.state = GameRunnerState::Playing;
            }
        }
    }
    
    pub fn add_remote_player(&mut self, node_id: NodeId) {
        self.game_state.add_player(node_id);
        info!(players=self.game_state.players.len(), remote=?node_id, "added remote player");
        if let GameRunnerState::Lobby { ref mut player_count } = self.state {
            *player_count += 1;
            if *player_count == 2 {
                self.state = GameRunnerState::Playing;
            }
        }
    }
    
    pub fn handle_input(&mut self, input: <BattleshipGame as LantermGame>::Input) {
        let shooter = self.local_node_id;
        BattleshipGame::handle_input(&mut self.game_state, input, shooter);
        
        if self.game_state.finished {
            self.state = GameRunnerState::Finished;
        }
    }

    /// Host: send full state to remote
    pub async fn send_state(&self) -> Result<()> {
        if !self.is_host {
            return Ok(());
        }
        if let Some(remote_id) = self.remote_node_id {
            let conn = self.endpoint.connect(remote_id, ALPN).await?;
            let (mut send, _recv) = conn.open_bi().await?;
            let msg = NetworkMessage::StateSync { state: bincode::serialize(&self.game_state)? };
            let msg_bytes = bincode::serialize(&msg)?;
            send.write_all(&(msg_bytes.len() as u32).to_le_bytes()).await?;
            send.write_all(&msg_bytes).await?;
            send.finish()?;
        }
        Ok(())
    }

    /// Client: poll for incoming state sync from host
    pub async fn poll_state(&mut self) -> Result<()> {
        if self.is_host {
            return Ok(());
        }
        let incoming = match timeout(Duration::from_millis(10), self.endpoint.accept()).await {
            Ok(opt) => opt,
            Err(_) => return Ok(()),
        };
        if let Some(connecting) = incoming {
            let conn = match timeout(Duration::from_millis(2000), connecting).await {
                Ok(c) => c?,
                Err(_) => return Ok(()),
            };
            let (_send, mut recv) = match timeout(Duration::from_millis(2000), conn.accept_bi()).await {
                Ok(res) => res?,
                Err(_) => return Ok(()),
            };
            let mut len_bytes = [0u8; 4];
            if timeout(Duration::from_millis(2000), recv.read_exact(&mut len_bytes)).await?.is_err() {
                return Ok(());
            }
            let len = u32::from_le_bytes(len_bytes) as usize;
            let mut msg_bytes = vec![0u8; len];
            if timeout(Duration::from_millis(2000), recv.read_exact(&mut msg_bytes)).await?.is_err() {
                return Ok(());
            }
            if let Ok(NetworkMessage::StateSync { state }) = bincode::deserialize(&msg_bytes) {
                if let Ok(new_state) = bincode::deserialize::<BattleshipState>(&state) {
                    self.game_state = new_state;
                }
            }
        }
        Ok(())
    }
    
    pub fn render(&self, frame: &mut Frame) {
        BattleshipRenderer::render(frame, &self.game_state, self.local_node_id);
    }
    
    pub fn game_state(&self) -> &<BattleshipGame as LantermGame>::State {
        &self.game_state
    }

    pub fn runner_state(&self) -> &GameRunnerState {
        &self.state
    }
}
