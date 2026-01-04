mod cli;
mod core;
mod client;
mod games;

use cli::run_cli;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    run_cli().await
}