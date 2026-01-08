use anyhow::{anyhow, Result};
use crossterm::event::{self, Event, KeyCode};
use lanterm::core::{menu::{Menu, MenuChoice}, network::{InLobby, NetworkManager, LobbySignal}};
use lanterm::games;
use ratatui::{
    layout::Alignment,
    widgets::{Block, Borders, Paragraph},
    DefaultTerminal,
};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    // Menu in its own terminal
    let mut menu_terminal = ratatui::init();
    let mut menu = Menu::new();
    let (choice, game_id, join_config) = menu.run(&mut menu_terminal)?;
    ratatui::restore();

    let is_host = matches!(choice, MenuChoice::Host);
    let peer_id = join_config.as_ref().map(|c| c.node_id.clone());

    // Connect: establishes bidirectional communication
    let net_manager = NetworkManager::<InLobby>::connect(is_host, peer_id).await?;

    // Handshake: agree on which game
    let (net_manager, negotiated_game_id) = net_manager.handshake(Some(game_id), is_host).await?;

    // Lobby UI controls start; returns when host signals start
    let mut game_terminal = ratatui::init();
    let net_manager = run_lobby(&mut game_terminal, net_manager, is_host).await?;

    // Get game and run it
    let game = games::get_game(&negotiated_game_id)
        .ok_or_else(|| anyhow!("Game '{}' not found", negotiated_game_id))?;

    let _net_after_game = (game.initializer)(net_manager, is_host, &mut game_terminal).await?;
    ratatui::restore();

    println!("Game finished. Goodbye!");
    Ok(())
}

async fn run_lobby(
    terminal: &mut DefaultTerminal,
    net: NetworkManager<InLobby>,
    is_host: bool,
) -> Result<NetworkManager<InLobby>> {
    let local_id = net.local_id();
    let remote_id = net.remote_id();
    let game_id = net.game_id().unwrap_or("unknown").to_string();

    let mut start_signal_fut = if !is_host {
        let conn = net.conn_clone();
        Some(Box::pin(async move {
            loop {
                let dg = conn.read_datagram().await?;
                if let Ok(LobbySignal::StartGame) = postcard::from_bytes(&dg) {
                    return Ok::<(), anyhow::Error>(());
                }
            }
        }))
    } else {
        None
    };

    loop {
        terminal.draw(|f| {
            let area = f.area();
            let text = if is_host {
                format!(
                    "HOSTING: {game_id}\nYour ID: {local_id}\nPeer: {remote_id}\n\nPress 's' to start"
                )
            } else {
                format!(
                    "JOINED: {game_id}\nYour ID: {local_id}\nHost: {remote_id}\n\nWaiting for host to start..."
                )
            };

            f.render_widget(
                Paragraph::new(text)
                    .block(Block::default().title("Lobby").borders(Borders::ALL))
                    .alignment(Alignment::Center),
                area,
            );
        })?;

        tokio::select! {
            // Client waits for host signal
            _ = async {
                if let Some(fut) = start_signal_fut.as_mut() {
                    fut.await
                } else {
                    std::future::pending().await
                }
            }, if !is_host => {
                // drop future before moving net
                let _ = start_signal_fut.take();
                return Ok(net);
            }

            _ = tokio::time::sleep(Duration::from_millis(50)) => {
                if event::poll(Duration::from_millis(0))? {
                    if let Event::Key(key) = event::read()? {
                        if key.code == KeyCode::Esc {
                            return Err(anyhow!("Lobby exited"));
                        }
                        if is_host && key.code == KeyCode::Char('s') {
                            net.send_start_signal().await?;
                            return Ok(net);
                        }
                    }
                }
            }
        }
    }
}