use crate::core::network::NetworkManager;
use crate::{Context, Game};
use anyhow::Result;
use ratatui::DefaultTerminal;
use std::time::Instant;

pub struct Engine<G: Game> {
    game: G,
    network: NetworkManager<G::Message>,
}

impl<G: Game> Engine<G> {
    pub fn new(game: G, network: NetworkManager<G::Message>) -> Self {
        Self { game, network }
    }

    pub async fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        let mut last_tick = Instant::now();
        
        // set up channels for outgoing + incoming network messages
        let (outbox_tx, mut outbox_rx) = tokio::sync::mpsc::unbounded_channel::<G::Message>();
        let ctx = Context { tx: outbox_tx };

        loop {
            terminal.draw(|f| self.game.render(f))?;

            // INPUT (Non-blocking)
            if crossterm::event::poll(std::time::Duration::from_millis(0))? {
                if let crossterm::event::Event::Key(key) = crossterm::event::read()? {
                    if key.code == crossterm::event::KeyCode::Esc { break; }
                    self.game.handle_input(key, &ctx);
                }
            }

            // Always wake the loop periodically so input keeps getting polled even when
            // the game does not use ticks. For games without ticks we use a small sleep
            // to avoid a tight loop while still letting input through.
            let tick_rate = self.game.tick_rate();
            let tick_sleep = tick_rate.unwrap_or(std::time::Duration::from_millis(16));
            let tick_fused = tokio::time::sleep(tick_sleep);

            tokio::select! {
                // 2. SEND: If the user called ctx.send_network_event(), handle it here
                Some(msg_to_send) = outbox_rx.recv() => {
                    self.network.send_msg(msg_to_send).await?;
                }

                // 3. RECEIVE: If a message comes from the other player
                Ok(incoming_msg) = self.network.next_msg() => {
                    self.game.handle_network(incoming_msg, &ctx);
                }

                // 4. TICK: Game heartbeat
                _ = tick_fused => {
                    if tick_rate.is_some() {
                        let dt = last_tick.elapsed().as_millis() as u32;
                        last_tick = Instant::now();
                        self.game.on_tick(dt, &ctx);
                    }
                }
            }
        }
        
        ratatui::restore();
        Ok(())
    }
}