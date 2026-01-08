use crate::core::network::{Active, InternalMsg, NetworkManager, SyncPacket};
use crate::{Context, Game};
use anyhow::Result;
use ratatui::DefaultTerminal;
use std::time::{Instant, Duration};
use crossterm::event::{self, Event, KeyCode};

pub struct Engine<G: Game> {
    game: G,
    network: NetworkManager<Active<InternalMsg<G::Action, G::State>>>,
    is_host: bool,
    state: G::State,
}

impl<G: Game> Engine<G> {
    pub fn new(game: G, network: NetworkManager<Active<InternalMsg<G::Action, G::State>>>, is_host: bool) -> Self {
        Self { game, network, is_host, state: G::State::default() }
    }

    pub async fn run(mut self, terminal: &mut DefaultTerminal) -> Result<NetworkManager<Active<InternalMsg<G::Action, G::State>>>> {

        let my_id = self.network.local_id(); // WHO AM I?
        let remote_id = self.network.remote_id(); // WHO IS THE OTHER?
        let mut last_tick = Instant::now();
        let mut sequence_counter: u64 = 0;
        let mut last_seen_seq: u64 = 0;

        let (action_tx, mut action_rx) = tokio::sync::mpsc::unbounded_channel::<G::Action>();
        let ctx = Context { tx: action_tx };

        loop {
            terminal.draw(|f| self.game.render(f, &self.state))?;

            // Input Handling (Non-blocking)
            if event::poll(Duration::from_millis(0))? {
                if let Event::Key(key) = event::read()? {
                    if key.code == KeyCode::Esc { break; }
                    self.game.handle_input(key, &ctx, my_id);
                }
            }

            let tick_rate = self.game.tick_rate().unwrap_or(Duration::from_millis(16));
            let conn = self.network.conn_clone();

            tokio::select! {
                Some(action) = action_rx.recv() => {
                    if self.is_host {
                        self.game.handle_action(action, &mut self.state, my_id);
                    } else {
                        self.network.send_reliable(InternalMsg::Action(action)).await?;
                    }
                }

                Ok(msg) = self.network.next_reliable() => {
                    if let InternalMsg::Action(a) = msg {
                        if self.is_host {
                            self.game.handle_action(a, &mut self.state, remote_id);
                        }
                    }
                }

                Ok(msg) = async {
                    let bytes = conn.read_datagram().await?;
                    let msg = postcard::from_bytes(&bytes)?;
                    Ok::<InternalMsg<G::Action, G::State>, anyhow::Error>(msg)
                } => {
                    if let InternalMsg::Sync(packet) = msg {
                        if !self.is_host && packet.seq > last_seen_seq {
                            self.state = packet.state;
                            last_seen_seq = packet.seq;
                        }
                    }
                }

                _ = tokio::time::sleep(tick_rate) => {
                    let dt = last_tick.elapsed().as_millis() as u32;
                    last_tick = Instant::now();

                    if self.is_host {
                        self.game.on_tick(dt, &mut self.state);
                        
                        sequence_counter += 1;
                        let packet = SyncPacket {
                            seq: sequence_counter,
                            state: self.state.clone(),
                        };
                        let _ = self.network.send_unreliable(InternalMsg::Sync(packet));
                    }
                }
            }
        }
        Ok(self.network)
    }
}