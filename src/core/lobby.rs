use anyhow::{anyhow, Result};
use iroh::endpoint::{Endpoint, RecvStream, SendStream, Connection};
use iroh::EndpointId;

const ALPN: &[u8] = b"lanterm-proto";

/// Connect to Iroh and perform the game ID handshake
/// Returns (send, recv, conn, game_id, local_endpoint_id)
/// For host: game_id is the one they selected
/// For client: game_id is received from the host
pub async fn connect_iroh(is_host: bool, ticket_str: Option<String>, game_id: String) -> Result<(SendStream, RecvStream, Connection, String, EndpointId)> {
    let endpoint = Endpoint::builder()
        .alpns(vec![ALPN.to_vec()])
        .bind()
        .await?;
    
    let local_id = endpoint.id();
    
    let connection = if is_host {
        println!("Your Endpoint ID: {}", local_id);
        println!("Waiting for client to connect...");
        let incoming = endpoint.accept().await.ok_or_else(|| anyhow!("Closed"))?;
        let conn = incoming.accept()?.await?;
        println!("Client connected!");
        conn
    } else {
        let peer: EndpointId = ticket_str.ok_or_else(|| anyhow!("Missing ID"))?.parse()?;
        println!("Connecting to host...");
        let conn = endpoint.connect(peer, ALPN).await?;
        println!("Connected to host!");
        conn
    };

    // HANDSHAKE PROTOCOL via datagrams (untyped, no network manager needed)
    let received_game_id = if is_host {
        // Host sends the game ID to client via datagram
        println!("Host: Sending game ID '{}' to client...", game_id);
        connection.send_datagram(game_id.as_bytes().to_vec().into())?;
        game_id // Host keeps its own game_id
    } else {
        // Client receives the game ID from host
        println!("Client: Waiting for host to announce game...");
        let datagram = connection.read_datagram().await?;
        let received_id = String::from_utf8(datagram.to_vec())?;
        println!("Host is running: {}", received_id);
        received_id
    };

    // Now establish the bidirectional stream for game communication
    let (send, recv) = if is_host {
        println!("Host: Waiting for client to open stream...");
        connection.accept_bi().await?
    } else {
        println!("Client: Opening stream to host...");
        connection.open_bi().await?
    };

    println!("Stream established!");
    Ok((send, recv, connection, received_game_id, local_id))
}