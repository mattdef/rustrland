use anyhow::Result;
use clap::{Parser, Subcommand};
use tokio::net::UnixStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

// Import the IPC protocol from the library
use rustrland::ipc::{ClientMessage, DaemonResponse, protocol::get_socket_path};

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
    Expose {
        /// Expose sub-command (toggle, next, prev, exit, status)
        #[arg(default_value = "toggle")]
        action: String,
    },
    /// Reload configuration
    Reload,
    /// Show daemon status
    Status,
    /// List available scratchpads
    List,
    /// Workspace management
    Workspace {
        /// Workspace command (switch, change, list, status)
        #[arg()]
        action: String,
        /// Optional argument (workspace ID, offset, etc.)
        #[arg()]
        arg: Option<String>,
    },
    /// Magnify/zoom controls
    Magnify {
        /// Magnify command (toggle, set, in, out, reset, status)
        #[arg()]
        action: String,
        /// Optional argument (zoom level, delta, etc.)
        #[arg()]
        arg: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    let message = match cli.command {
        Commands::Toggle { name } => ClientMessage::Toggle { scratchpad: name },
        Commands::Expose { action } => ClientMessage::ExposeAction { action },
        Commands::Reload => ClientMessage::Reload,
        Commands::Status => ClientMessage::Status,
        Commands::List => ClientMessage::List,
        Commands::Workspace { action, arg } => ClientMessage::WorkspaceAction { action, arg },
        Commands::Magnify { action, arg } => ClientMessage::MagnifyAction { action, arg },
    };
    
    match send_command(message).await {
        Ok(response) => handle_response(response),
        Err(e) => {
            eprintln!("❌ Failed to communicate with daemon: {}", e);
            eprintln!("💡 Make sure the rustrland daemon is running");
            std::process::exit(1);
        }
    }
    
    Ok(())
}

async fn send_command(message: ClientMessage) -> Result<DaemonResponse> {
    let socket_path = get_socket_path();
    let mut stream = UnixStream::connect(&socket_path).await?;
    
    // Serialize the message
    let message_data = serde_json::to_vec(&message)?;
    
    // Send message length + message
    let msg_len = (message_data.len() as u32).to_le_bytes();
    stream.write_all(&msg_len).await?;
    stream.write_all(&message_data).await?;
    
    // Read response length
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf).await?;
    let response_len = u32::from_le_bytes(len_buf) as usize;
    
    // Read response
    let mut response_buf = vec![0u8; response_len];
    stream.read_exact(&mut response_buf).await?;
    
    // Deserialize response
    let response: DaemonResponse = serde_json::from_slice(&response_buf)?;
    Ok(response)
}

fn handle_response(response: DaemonResponse) {
    match response {
        DaemonResponse::Success { message } => {
            println!("✅ {}", message);
        }
        DaemonResponse::Error { message } => {
            eprintln!("❌ Error: {}", message);
            std::process::exit(1);
        }
        DaemonResponse::Status { version, uptime_seconds, plugins_loaded } => {
            println!("📊 Rustrland Status");
            println!("   Version: {}", version);
            println!("   Uptime: {} seconds", uptime_seconds);
            println!("   Plugins loaded: {}", plugins_loaded);
        }
        DaemonResponse::List { items } => {
            if items.is_empty() {
                println!("📋 No items available");
            } else {
                println!("📋 Available items:");
                for item in items {
                    println!("   • {}", item);
                }
            }
        }
    }
}
