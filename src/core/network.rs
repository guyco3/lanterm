use anyhow::{anyhow, Result};
use iroh::endpoint::{Connection, Endpoint, RecvStream, SendStream};
use iroh::EndpointId;
use serde::{Serialize, Deserialize, de::DeserializeOwned};
use std::marker::PhantomData;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const ALPN: &[u8] = b"lanterm-proto";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SyncPacket<S> {
    pub seq: u64,
    pub state: S,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum InternalMsg<A, S> {
    Action(A),
    Sync(SyncPacket<S>),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum LobbySignal {
    StartGame,
}

pub struct InLobby;
pub struct Active<M> {
    pub _pd: PhantomData<M>,
}

pub struct NetworkManager<S> {
    conn: Connection,
    send: SendStream,
    recv: RecvStream,
    local_endpoint_id: EndpointId,
    game_id: Option<String>,
    _state: S,
}

impl<S> NetworkManager<S> {
    /// Gets our own Endpoint ID
    pub fn local_id(&self) -> EndpointId {
        self.local_endpoint_id
    }

    /// Gets the Endpoint ID of the person on the other side of this connection
    pub fn remote_id(&self) -> EndpointId {
        self.conn.remote_id()
    }

    /// Clones the underlying connection handle for read-only tasks without borrowing self across awaits.
    pub fn conn_clone(&self) -> Connection {
        self.conn.clone()
    }

    /// Best-effort abort of the underlying QUIC connection
    pub fn abort(self) {
        let _ = self.conn.close(0u32.into(), b"abort");
    }

    /// Returns the last negotiated game identifier, if any
    pub fn game_id(&self) -> Option<&str> {
        self.game_id.as_deref()
    }
}

impl NetworkManager<InLobby> {
    /// Establish a connection to a peer and return a manager locked to the lobby state.
    pub async fn connect(is_host: bool, peer: Option<String>) -> Result<Self> {
        let endpoint = Endpoint::builder()
            .alpns(vec![ALPN.to_vec()])
            .bind()
            .await?;

        let local_endpoint_id = endpoint.id();

        let conn = if is_host {
            println!("Your Endpoint ID: {local_endpoint_id}");
            println!("Waiting for a client to connect...");
            let incoming = endpoint.accept().await.ok_or_else(|| anyhow!("Closed"))?;
            let conn = incoming.accept()?.await?;
            println!("Client connected!");
            conn
        } else {
            let peer_id: EndpointId = peer.ok_or_else(|| anyhow!("Missing peer ID"))?.parse()?;
            println!("Connecting to host {peer_id}...");
            let conn = endpoint.connect(peer_id, ALPN).await?;
            println!("Connected to host!");
            conn
        };

        // Deterministic roles: host opens, client accepts to avoid the accept wait seen earlier.
        let (send, recv) = if is_host {
            println!("Opening bidirectional stream to client...");
            let s = conn.open_bi().await?;
            println!("Bidirectional stream established (host).");
            s
        } else {
            println!("Waiting for host to open bidirectional stream...");
            let s = conn.accept_bi().await?;
            println!("Bidirectional stream established (client).");
            s
        };

        Ok(Self {
            conn,
            send,
            recv,
            local_endpoint_id,
            game_id: None,
            _state: InLobby,
        })
    }

    /// Exchanges the game identifier via the reliable stream to avoid stray datagrams.
    pub async fn handshake(mut self, requested_game_id: Option<String>, is_host: bool) -> Result<(Self, String)> {
        let chosen = if is_host {
            let gid = requested_game_id.ok_or_else(|| anyhow!("Host must provide a game id"))?;
            let bytes = gid.as_bytes();
            self.send.write_u32(bytes.len() as u32).await?;
            self.send.write_all(bytes).await?;
            self.send.flush().await?;
            println!("Handshake: sent game id '{gid}'.");
            gid.to_string()
        } else {
            let len = self.recv.read_u32().await? as usize;
            let mut buf = vec![0u8; len];
            self.recv.read_exact(&mut buf).await?;
            let gid = String::from_utf8(buf)?;
            println!("Handshake: received game id '{gid}'.");
            gid
        };

        self.game_id = Some(chosen.clone());
        Ok((self, chosen))
    }

    /// Host: Sends the start signal to the peer.
    pub async fn send_start_signal(&self) -> Result<()> {
        let msg = postcard::to_stdvec(&LobbySignal::StartGame)?;
        self.conn.send_datagram(msg.into())?;
        Ok(())
    }

    /// Client: Resolves when the start signal is received.
    pub async fn recv_start_signal(&self) -> Result<()> {
        loop {
            let dg = self.conn.read_datagram().await?;
            if let Ok(LobbySignal::StartGame) = postcard::from_bytes(&dg) {
                return Ok(());
            }
        }
    }

    /// Upgrade from lobby state into the typed active state.
    pub fn upgrade<M>(self) -> NetworkManager<Active<M>> {
        NetworkManager {
            conn: self.conn,
            send: self.send,
            recv: self.recv,
            local_endpoint_id: self.local_endpoint_id,
            game_id: self.game_id,
            _state: Active { _pd: PhantomData },
        }
    }
}

impl<M> NetworkManager<Active<M>>
where
    M: Serialize + DeserializeOwned + Send + 'static,
{
    /// Step 4: Reset. Downgrades from Active back to Lobby.
    pub fn reset(self) -> NetworkManager<InLobby> {
        NetworkManager {
            conn: self.conn,
            send: self.send,
            recv: self.recv,
            local_endpoint_id: self.local_endpoint_id,
            game_id: None,
            _state: InLobby,
        }
    }

    pub async fn send_reliable(&mut self, msg: M) -> Result<()> {
        let bytes = postcard::to_stdvec(&msg)?;
        self.send.write_u32(bytes.len() as u32).await?;
        self.send.write_all(&bytes).await?;
        self.send.flush().await?;
        Ok(())
    }

    pub fn send_unreliable(&self, msg: M) -> Result<()> {
        let bytes = postcard::to_stdvec(&msg)?;
        self.conn.send_datagram(bytes.into())?;
        Ok(())
    }

    pub async fn next_reliable(&mut self) -> Result<M> {
        let len = self.recv.read_u32().await? as usize;
        let mut buf = vec![0u8; len];
        self.recv.read_exact(&mut buf).await?;
        Ok(postcard::from_bytes(&buf)?)
    }

    pub fn next_unreliable(&self) -> impl std::future::Future<Output = Result<M>> + Send + 'static {
        let conn = self.conn.clone();
        async move {
            let bytes = conn.read_datagram().await?;
            Ok(postcard::from_bytes(&bytes)?)
        }
    }
}