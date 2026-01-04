use serde::{Serialize, Deserialize};
use std::time::Duration;
use iroh::EndpointId;
use tokio::sync::mpsc;

pub type NodeId = EndpointId;

pub trait LantermGame: Clone + Send + Sync + 'static {
    type State: Serialize + for<'de> Deserialize<'de> + Clone + Send + Sync + 'static;
    type Input: Serialize + for<'de> Deserialize<'de> + Clone + Send + Sync + 'static;
    
    fn new_game() -> Self::State;

    /// Parse a text command from console input into game input
    fn parse_command(cmd: &str) -> Option<Self::Input>;

    /// Handle input from either local console (from local_player_id) or network (from other players).
    /// Use `player == local_player_id` to determine if input is from local console.
    fn handle_input(state: &mut Self::State, input: Self::Input, player: NodeId, ctx: &LantermContext<Self::Input>);
    
    fn handle_player_joined(_state: &mut Self::State, _player: NodeId, _ctx: &LantermContext<Self::Input>) {}
    fn handle_player_left(_state: &mut Self::State, _player: NodeId, _ctx: &LantermContext<Self::Input>) {}
    fn on_tick(_state: &mut Self::State, _ctx: &LantermContext<Self::Input>) {}
    fn tick_rate() -> Option<Duration> { None }
}

pub trait LantermRenderer<S> {
    /// render now takes `current_input` so the UI can show what the user is typing.
    /// Returns cursor position (x, y) if input should show a cursor, None otherwise.
    fn render(frame: &mut ratatui::Frame, state: &S, local_node_id: NodeId, current_input: &str) -> Option<(u16, u16)>;
}

pub(crate) enum EngineCommand<I> {
    SendInput(I),
    SetTimer(String, Duration),
}

pub struct LantermContext<I> {
    pub local_node_id: NodeId,
    pub(crate) cmd_tx: mpsc::Sender<EngineCommand<I>>, 
    pub(crate) _input_type: std::marker::PhantomData<I>,
}

impl<I: Serialize + Send + 'static> LantermContext<I> {
    pub async fn send_input(&self, input: I) -> anyhow::Result<()> {
        self.cmd_tx.send(EngineCommand::SendInput(input)).await
            .map_err(|_| anyhow::anyhow!("Engine disconnected"))
    }

    pub fn set_timer(&self, name: &str, duration: Duration) {
        let tx = self.cmd_tx.clone();
        let name = name.to_string();
        tokio::spawn(async move {
            tokio::time::sleep(duration).await;
            let _ = tx.send(EngineCommand::SetTimer(name, duration)).await;
        });
    }
}
