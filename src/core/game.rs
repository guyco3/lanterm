use ratatui::Frame;
use serde::{Serialize, de::DeserializeOwned};
use std::time::Duration;
use tokio::sync::mpsc::UnboundedSender;

pub struct Context<M> {
    pub(crate) tx: UnboundedSender<M>,
}

impl<M> Context<M> {
    pub fn send_network_event(&self, msg: M) {
        let _ = self.tx.send(msg);
    }
}

pub trait Game: Send + Sync + 'static {
    type Message: Serialize + DeserializeOwned + Send + Clone + std::fmt::Debug;

    // User-implemented logic hooks
    fn handle_input(&mut self, event: crossterm::event::KeyEvent, ctx: &Context<Self::Message>);
    fn handle_network(&mut self, msg: Self::Message, ctx: &Context<Self::Message>);
    fn on_tick(&mut self, dt: u32, ctx: &Context<Self::Message>);
    fn render(&self, frame: &mut Frame);

    // Optional: Defaults to 60fps
    fn tick_rate(&self) -> Option<Duration> {
        Some(Duration::from_millis(16))
    }
}