use anyhow::{anyhow, Result};
use iroh::endpoint::{Endpoint, RecvStream, SendStream, Connection};
use iroh::EndpointId;

const ALPN: &[u8] = b"lanterm-proto";

pub async fn connect_iroh(is_host: bool, ticket_str: Option<String>) -> Result<(SendStream, RecvStream, Connection)> {
    let endpoint = Endpoint::builder()
        .alpns(vec![ALPN.to_vec()])
        .bind()
        .await?;
    
    let connection = if is_host {
        println!("Your Endpoint ID: {}", endpoint.id());
        let incoming = endpoint.accept().await.ok_or_else(|| anyhow!("Closed"))?;
        incoming.accept()?.await?
    } else {
        let peer: EndpointId = ticket_str.ok_or_else(|| anyhow!("Missing ID"))?.parse()?;
        endpoint.connect(peer, ALPN).await?
    };

    let (send, recv) = if is_host {
        connection.accept_bi().await?
    } else {
        connection.open_bi().await?
    };

    Ok((send, recv, connection))
}