use crate::core::game::{NodeId};
use crate::engine::EngineEvent;
use iroh::Endpoint;
use serde::{Serialize, de::DeserializeOwned};
use tokio::sync::mpsc;

pub struct NetworkManager {
    pub endpoint: Endpoint,
}

impl NetworkManager {
    pub async fn new() -> anyhow::Result<Self> {
        // Bind to a random port and start listening
        // Configure ALPN for the battleship game protocol
        let endpoint = iroh::Endpoint::builder()
            .alpns(vec![b"lanterm-battleship".to_vec()])
            .bind()
            .await?;
        Ok(Self { endpoint })
    }

    /// Background task to accept incoming P2P connections
    pub async fn start_accept_loop<I>(
        endpoint: Endpoint,
        event_tx: mpsc::Sender<EngineEvent<I>>,
        conn_tx: mpsc::Sender<iroh::endpoint::Connection>,
    ) -> anyhow::Result<()> 
    where I: Serialize + DeserializeOwned + Send + 'static 
    {
        while let Some(incoming) = endpoint.accept().await {
            let event_tx = event_tx.clone();
            let conn_tx = conn_tx.clone();
            tokio::spawn(async move {
                if let Ok(conn) = incoming.await {
                    let player_id: NodeId = conn.remote_id();
                    
                    // Send connection to runner for outgoing messages
                    let _ = conn_tx.send(conn.clone()).await;
                    
                    let _ = event_tx.send(EngineEvent::PlayerJoined(player_id)).await;
                    
                    // Handle bi-directional game stream
                    if let Ok((_send, mut recv)) = conn.accept_bi().await {
                        let mut buf = vec![0u8; 1024]; 
                        loop {
                            match recv.read(&mut buf).await {
                                Ok(Some(n)) => {
                                    // Deserializing the network bytes into the Game's Input type
                                    if let Ok(input) = postcard::from_bytes::<I>(&buf[..n]) {
                                        let _ = event_tx.send(EngineEvent::InputReceived(player_id, input)).await;
                                    }
                                }
                                Ok(None) => break,
                                Err(_) => break,
                            }
                        }
                    }

                    let _ = event_tx.send(EngineEvent::PlayerLeft(player_id)).await;
                }
            });
        }
        Ok(())
    }
}
