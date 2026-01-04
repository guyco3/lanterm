use clap::{Parser, Subcommand};
use dialoguer::{theme::ColorfulTheme, Select};

use crate::games::create_default_registry;

#[derive(Parser)]
#[command(name = "lanterm")]
#[command(about = "🕹️ A Rust framework for multiplayer terminal games")]
#[command(version = "0.1.0")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Host a game server
    Host {
        /// Address to bind the server to (e.g., "0.0.0.0:4000")
        #[arg(short, long, default_value = "0.0.0.0:4000")]
        addr: String,
        
        /// Game to host (if not specified, will show selection)
        #[arg(short, long)]
        game: Option<String>,
    },
    /// Join a game server
    Join {
        /// Server address to connect to (e.g., "127.0.0.1:4000")
        addr: String,
        
        /// Player name
        #[arg(short, long, default_value = "Player")]
        name: String,
    },
    /// List available games
    List,
}

pub async fn run_cli() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Host { addr, game }) => {
            let registry = create_default_registry();
            
            let game_name = if let Some(game_name) = game {
                if !registry.has_game(&game_name) {
                    eprintln!("❌ Game '{}' not found", game_name);
                    eprintln!("Available games:");
                    for metadata in registry.list_games() {
                        eprintln!("  • {} - {}", metadata.name, metadata.description);
                    }
                    std::process::exit(1);
                }
                game_name
            } else {
                select_game(&registry)?
            };
            
            println!("🚀 Starting {} server on {}", game_name, addr);
            registry.start_game(&game_name, &addr).await?;
        }
        
        Some(Commands::Join { addr, name }) => {
            println!("🔗 Connecting to {} as '{}'...", addr, name);
            
            // Auto-detect game type from server metadata
            let registry = create_default_registry();
            registry.auto_detect_and_join(&addr, name).await?;
        }
        
        Some(Commands::List) => {
            let registry = create_default_registry();
            println!("🎮 Available games:");
            println!();
            
            for metadata in registry.list_games() {
                println!("📦 {}", metadata.name);
                println!("   {}", metadata.description);
                println!("   Players: {}-{}", metadata.min_players, metadata.max_players);
                println!();
            }
        }
        
        None => {
            // No subcommand provided - show interactive menu
            show_main_menu().await?;
        }
    }

    Ok(())
}

fn select_game(registry: &crate::core::registry::GameRegistry) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let games = registry.list_games();
    
    if games.is_empty() {
        return Err("No games available".into());
    }

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("🎮 Select a game to host")
        .items(&games.iter().map(|g| format!("{} - {}", g.name, g.description)).collect::<Vec<_>>())
        .interact()?;

    Ok(games[selection].name.clone())
}

async fn show_main_menu() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("🕹️  Welcome to Lanterm!");
    println!("   A Rust framework for multiplayer terminal games");
    println!();

    let options = vec![
        "🏠 Host a game",
        "🔗 Join a game",
        "📋 List available games",
        "🚪 Exit",
    ];

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("What would you like to do?")
        .items(&options)
        .interact()?;

    match selection {
        0 => {
            // Host a game
            let registry = create_default_registry();
            let game_name = select_game(&registry)?;
            
            let addr = dialoguer::Input::with_theme(&ColorfulTheme::default())
                .with_prompt("Server address")
                .default("0.0.0.0:4000".to_string())
                .interact_text()?;

            println!("🚀 Starting {} server on {}", game_name, addr);
            registry.start_game(&game_name, &addr).await?;
        }
        1 => {
            // Join a game
            let addr = dialoguer::Input::with_theme(&ColorfulTheme::default())
                .with_prompt("Server address")
                .default("127.0.0.1:4000".to_string())
                .interact_text()?;

            let name = dialoguer::Input::with_theme(&ColorfulTheme::default())
                .with_prompt("Your name")
                .default("Player".to_string())
                .interact_text()?;

            println!("🔗 Connecting to {} as '{}'...", addr, name);
            
            // Auto-detect game from server metadata
            let registry = create_default_registry();
            registry.auto_detect_and_join(&addr, name).await?;
        }
        2 => {
            // List games
            let registry = create_default_registry();
            println!();
            println!("🎮 Available games:");
            println!();
            
            for metadata in registry.list_games() {
                println!("📦 {}", metadata.name);
                println!("   {}", metadata.description);
                println!("   Players: {}-{}", metadata.min_players, metadata.max_players);
                println!();
            }
        }
        3 => {
            // Exit
            println!("👋 Goodbye!");
        }
        _ => unreachable!(),
    }

    Ok(())
}