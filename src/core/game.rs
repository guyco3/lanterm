use ratatui::Frame;
use serde::{Serialize, de::DeserializeOwned};
use std::time::Duration;

pub struct Context<A> {
    pub(crate) tx: tokio::sync::mpsc::UnboundedSender<A>,
}

impl<A> Context<A> {
    pub fn send_action(&self, action: A) {
        let _ = self.tx.send(action);
    }
}

pub trait Game: Send + Sync + 'static {
    type Action: Serialize + DeserializeOwned + Send + Clone + std::fmt::Debug;
    // Added DeserializeOwned here
    type State: Serialize + DeserializeOwned + Send + Clone + Default;

    fn handle_input(&mut self, event: crossterm::event::KeyEvent, ctx: &Context<Self::Action>);
    fn handle_action(&self, action: Self::Action, state: &mut Self::State);
    fn on_tick(&self, dt: u32, state: &mut Self::State);
    fn render(&self, frame: &mut Frame, state: &Self::State);

    fn tick_rate(&self) -> Option<Duration> {
        Some(Duration::from_millis(64))
    }
}