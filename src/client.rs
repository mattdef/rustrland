use anyhow::Result;
use clap::{Parser, Subcommand};
use tokio::net::UnixStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{warn, error};

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
    use tokio::time::{timeout, Duration, sleep};
    
    const IPC_TIMEOUT: Duration = Duration::from_secs(10);
    const MAX_RETRIES: u32 = 3;
    const RETRY_DELAY: Duration = Duration::from_millis(100);
    
    let mut last_error = None;
    
    // Retry loop with exponential backoff
    for attempt in 1..=MAX_RETRIES {
        match send_command_once(&message, IPC_TIMEOUT).await {
            Ok(response) => return Ok(response),
            Err(e) => {
                last_error = Some(e);
                
                // Check if this is a recoverable error
                if attempt < MAX_RETRIES {
                    let delay = RETRY_DELAY * attempt;
                    warn!("Command failed on attempt {}, retrying in {:?}: {}", attempt, delay, last_error.as_ref().unwrap());
                    sleep(delay).await;
                } else {
                    error!("Command failed after {} attempts", MAX_RETRIES);
                }
            }
        }
    }
    
    Err(last_error.unwrap())
}

async fn send_command_once(message: &ClientMessage, timeout_duration: tokio::time::Duration) -> Result<DaemonResponse> {
    use tokio::time::timeout;
    
    let socket_path = get_socket_path();
    let mut stream = timeout(timeout_duration, UnixStream::connect(&socket_path)).await
        .map_err(|_| anyhow::anyhow!("Connection timeout after {:?}", timeout_duration))??;
    
    // Serialize the message
    let message_data = serde_json::to_vec(&message)?;
    
    // Send message length + message with timeout
    let msg_len = (message_data.len() as u32).to_le_bytes();
    timeout(timeout_duration, stream.write_all(&msg_len)).await
        .map_err(|_| anyhow::anyhow!("Write timeout after {:?}", timeout_duration))??;
    timeout(timeout_duration, stream.write_all(&message_data)).await
        .map_err(|_| anyhow::anyhow!("Write timeout after {:?}", timeout_duration))??;
    
    // Read response length with timeout
    let mut len_buf = [0u8; 4];
    timeout(timeout_duration, stream.read_exact(&mut len_buf)).await
        .map_err(|_| anyhow::anyhow!("Read timeout while waiting for response length after {:?}", timeout_duration))??;
    let response_len = u32::from_le_bytes(len_buf) as usize;
    
    // Validate response length to prevent DoS
    if response_len > 1024 * 1024 {  // 1MB limit
        return Err(anyhow::anyhow!("Response too large: {} bytes", response_len));
    }
    
    // Read response with timeout
    let mut response_buf = vec![0u8; response_len];
    timeout(timeout_duration, stream.read_exact(&mut response_buf)).await
        .map_err(|_| anyhow::anyhow!("Read timeout while waiting for response data after {:?}", timeout_duration))??;
    
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
