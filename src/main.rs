use anyhow::Result;
use clap::Parser;
use tracing::{info, error};

mod core;
mod config;
mod ipc;
mod plugins;

use crate::core::daemon::Daemon;

#[derive(Parser)]
#[command(name = "rustrland")]
#[command(about = "A Rust implementation of Pyprland for Hyprland")]
#[command(version = env!("CARGO_PKG_VERSION"))]
struct Cli {
    /// Configuration file path
    #[arg(short, long, default_value = "~/.config/hypr/rustrland.toml")]
    config: String,
    
    /// Enable debug logging
    #[arg(short, long)]
    debug: bool,
    
    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,
    
    /// Run in foreground (don't daemonize)
    #[arg(short, long)]
    foreground: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    // Setup logging
    let log_level = if cli.debug {
        "debug"
    } else if cli.verbose {
        "info"
    } else {
        "warn"
    };
    
    tracing_subscriber::fmt()
        .with_env_filter(format!("rustrland={log_level}"))
        .with_target(false)
        .init();
    
    info!("🦀 Starting Rustrland v{}", env!("CARGO_PKG_VERSION"));
    
    // Verify Hyprland is running
    if std::env::var("HYPRLAND_INSTANCE_SIGNATURE").is_err() {
        error!("❌ Hyprland not detected. HYPRLAND_INSTANCE_SIGNATURE not set.");
        std::process::exit(1);
    }
    
    // Create and run daemon
    match Daemon::new(&cli.config).await {
        Ok(mut daemon) => {
            if let Err(e) = daemon.run().await {
                error!("❌ Daemon error: {}", e);
                std::process::exit(1);
            }
        }
        Err(e) => {
            error!("❌ Failed to create daemon: {}", e);
            std::process::exit(1);
        }
    }
    
    Ok(())
}
