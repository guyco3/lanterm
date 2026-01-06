use anyhow::Result;
use iroh::endpoint::{SendStream, RecvStream, Connection};
use serde::{Serialize, Deserialize, de::DeserializeOwned};
use tokio::io::{AsyncReadExt, AsyncWriteExt}; 
use std::marker::PhantomData;

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

pub struct NetworkManager<M> {
    pub send: SendStream,
    pub recv: RecvStream,
    pub conn: Connection,
    _msg_type: PhantomData<M>,
}

impl<M> NetworkManager<M> 
where 
    M: Serialize + DeserializeOwned + Send + 'static 
{
    pub fn new(send: SendStream, recv: RecvStream, conn: Connection) -> Self {
        Self { send, recv, conn, _msg_type: PhantomData }
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
        let conn = self.conn.clone(); // future owns the connection handle so we don't borrow self across await
        async move {
            let bytes = conn.read_datagram().await?;
            Ok(postcard::from_bytes(&bytes)?)
        }
    }
}