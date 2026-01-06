use anyhow::{anyhow, Result};
use iroh::endpoint::{Endpoint, RecvStream, SendStream};
use iroh::EndpointId;

const ALPN: &[u8] = b"lanterm-proto";

pub async fn connect_iroh(is_host: bool, ticket_str: Option<String>) -> Result<(SendStream, RecvStream)> {
    let endpoint = Endpoint::builder()
        .alpns(vec![ALPN.to_vec()])
        .bind()
        .await?;
    
    if is_host {
        let id = endpoint.id();
        println!("Your Endpoint ID: {}", id);
        println!("Waiting for client to connect...");
        
        let incoming = endpoint
            .accept()
            .await
            .ok_or_else(|| anyhow!("Endpoint closed while waiting for client"))?;
        let connection = incoming.accept()?.await?;
        println!("Client connected.");
        let (send, recv) = connection.accept_bi().await?;
        Ok((send, recv))
    } else {
        let peer: EndpointId = ticket_str
            .ok_or_else(|| anyhow!("Missing peer ID"))?
            .parse()?;
        println!("Connecting to host {}...", peer);
        let connection = endpoint.connect(peer, ALPN).await?;
        println!("Connected to host.");
        let (send, recv) = connection.open_bi().await?;
        Ok((send, recv))
    }
}
