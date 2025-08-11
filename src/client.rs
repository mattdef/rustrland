use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "rustr")]
#[command(about = "Rustrland client - send commands to running daemon")]
#[command(version = env!("CARGO_PKG_VERSION"))]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Toggle a scratchpad
    Toggle {
        /// Scratchpad name
        name: String,
    },
    /// Show all windows (expose)
    Expose,
    /// Reload configuration
    Reload,
    /// Show daemon status
    Status,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Toggle { name } => {
            println!("🔄 Toggling scratchpad: {}", name);
            // TODO: Send IPC command to daemon
        }
        Commands::Expose => {
            println!("🪟 Exposing all windows");
            // TODO: Send expose command
        }
        Commands::Reload => {
            println!("⚡ Reloading configuration");
            // TODO: Send reload command
        }
        Commands::Status => {
            println!("📊 Daemon status");
            // TODO: Query daemon status
        }
    }
    
    Ok(())
}
