use anyhow::{Context, Result};
use iroh::endpoint::{SendStream, RecvStream};
use serde::{Serialize, de::DeserializeOwned};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::marker::PhantomData;

pub struct NetworkManager<M> {
    // These are the raw bi-directional streams from your Iroh connection
    pub send: SendStream,
    pub recv: RecvStream,
    // PhantomData just tells Rust: "This manager is for Message type M"
    _msg_type: PhantomData<M>,
}

impl<M> NetworkManager<M> 
where 
    M: Serialize + DeserializeOwned + Send + 'static 
{
    pub fn new(send: SendStream, recv: RecvStream) -> Self {
        Self {
            send,
            recv,
            _msg_type: PhantomData,
        }
    }

    /// Sends a message: [Length (4 bytes)] + [Postcard Data]
    pub async fn send_msg(&mut self, msg: M) -> Result<()> {
        // 1. Turn the enum/struct into tiny binary bytes
        let bytes = postcard::to_stdvec(&msg)
            .context("Failed to serialize message")?;

        // 2. Send the size first so the other player knows how much to read
        self.send.write_u32(bytes.len() as u32).await?;
        
        // 3. Send the actual data
        self.send.write_all(&bytes).await?;
        
        // Ensure it's actually sent over the wire
        self.send.flush().await?;
        Ok(())
    }

    /// Receives a message: Reads Length, then reads that many bytes
    pub async fn next_msg(&mut self) -> Result<M> {
        // 1. Read the 4-byte length prefix
        let len = self.recv.read_u32().await? as usize;

        // 2. Prepare a buffer of exactly that size
        let mut buf = vec![0u8; len];

        // 3. Read the exact number of bytes
        self.recv.read_exact(&mut buf).await?;

        // 4. Turn the binary bytes back into your Message type
        let msg = postcard::from_bytes(&buf)
            .context("Failed to deserialize message")?;
            
        Ok(msg)
    }
}