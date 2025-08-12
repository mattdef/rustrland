use serde::{Deserialize, Serialize};

/// Messages sent from client to daemon
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientMessage {
    /// Toggle a scratchpad
    Toggle { scratchpad: String },
    /// Show all windows (expose)
    Expose,
    /// Expose with action
    ExposeAction { action: String },
    /// Workspace management action
    WorkspaceAction { action: String, arg: Option<String> },
    /// Magnify/zoom action
    MagnifyAction { action: String, arg: Option<String> },
    /// Reload configuration
    Reload,
    /// Get daemon status
    Status,
    /// List available scratchpads
    List,
}

/// Responses sent from daemon to client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DaemonResponse {
    /// Command executed successfully
    Success { message: String },
    /// Command failed with error
    Error { message: String },
    /// Status information
    Status {
        version: String,
        uptime_seconds: u64,
        plugins_loaded: usize,
    },
    /// List of available items
    List { items: Vec<String> },
}

impl ClientMessage {
    /// Parse command line arguments into a ClientMessage
    pub fn from_args(command: &str, args: &[String]) -> anyhow::Result<Self> {
        match command {
            "toggle" => {
                if let Some(scratchpad) = args.first() {
                    Ok(ClientMessage::Toggle {
                        scratchpad: scratchpad.clone(),
                    })
                } else {
                    Err(anyhow::anyhow!("Toggle command requires scratchpad name"))
                }
            }
            "expose" => {
                if args.is_empty() {
                    Ok(ClientMessage::Expose)
                } else {
                    Ok(ClientMessage::ExposeAction {
                        action: args
                            .first()
                            .ok_or_else(|| anyhow::anyhow!("Missing expose action"))?
                            .clone(),
                    })
                }
            }
            "workspace" => {
                if let Some(action) = args.first() {
                    Ok(ClientMessage::WorkspaceAction {
                        action: action.clone(),
                        arg: args.get(1).cloned(),
                    })
                } else {
                    Err(anyhow::anyhow!("Workspace command requires action"))
                }
            }
            "magnify" => {
                if let Some(action) = args.first() {
                    Ok(ClientMessage::MagnifyAction {
                        action: action.clone(),
                        arg: args.get(1).cloned(),
                    })
                } else {
                    Err(anyhow::anyhow!("Magnify command requires action"))
                }
            }
            "reload" => Ok(ClientMessage::Reload),
            "status" => Ok(ClientMessage::Status),
            "list" => Ok(ClientMessage::List),
            _ => Err(anyhow::anyhow!("Unknown command: {}", command)),
        }
    }
}

/// IPC socket path - uses runtime directory or falls back to /tmp
pub fn get_socket_path() -> String {
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".to_string());
    format!("{}/rustrland.sock", runtime_dir)
}
