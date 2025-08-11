use anyhow::Result;
use tokio::net::{UnixListener, UnixStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{info, debug, warn, error};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::ipc::protocol::{ClientMessage, DaemonResponse, get_socket_path};
use crate::core::plugin_manager::PluginManager;

pub struct IpcServer {
    plugin_manager: Arc<Mutex<PluginManager>>,
    start_time: std::time::Instant,
}

impl IpcServer {
    pub fn new(plugin_manager: Arc<Mutex<PluginManager>>) -> Self {
        Self {
            plugin_manager,
            start_time: std::time::Instant::now(),
        }
    }
    
    pub async fn start(&self) -> Result<()> {
        let socket_path = get_socket_path();
        
        // Remove existing socket file if it exists
        if std::path::Path::new(&socket_path).exists() {
            std::fs::remove_file(&socket_path)?;
        }
        
        let listener = UnixListener::bind(&socket_path)?;
        info!("🔌 IPC server listening on: {}", socket_path);
        
        loop {
            match listener.accept().await {
                Ok((stream, _)) => {
                    let plugin_manager = Arc::clone(&self.plugin_manager);
                    let start_time = self.start_time;
                    
                    tokio::spawn(async move {
                        if let Err(e) = Self::handle_client(stream, plugin_manager, start_time).await {
                            warn!("⚠️  Error handling client: {}", e);
                        }
                    });
                }
                Err(e) => {
                    error!("❌ Failed to accept connection: {}", e);
                }
            }
        }
    }
    
    async fn handle_client(
        mut stream: UnixStream,
        plugin_manager: Arc<Mutex<PluginManager>>,
        start_time: std::time::Instant,
    ) -> Result<()> {
        debug!("📞 New client connection");
        
        // Read message length first (4 bytes)
        let mut len_buf = [0u8; 4];
        stream.read_exact(&mut len_buf).await?;
        let msg_len = u32::from_le_bytes(len_buf) as usize;
        
        // Read the actual message
        let mut msg_buf = vec![0u8; msg_len];
        stream.read_exact(&mut msg_buf).await?;
        
        // Deserialize the message
        let message: ClientMessage = serde_json::from_slice(&msg_buf)?;
        debug!("📨 Received message: {:?}", message);
        
        // Process the message
        let response = Self::process_message(message, plugin_manager, start_time).await;
        
        // Serialize response
        let response_data = serde_json::to_vec(&response)?;
        
        // Send response length + response
        let response_len = (response_data.len() as u32).to_le_bytes();
        stream.write_all(&response_len).await?;
        stream.write_all(&response_data).await?;
        
        debug!("📤 Sent response: {:?}", response);
        Ok(())
    }
    
    async fn process_message(
        message: ClientMessage,
        plugin_manager: Arc<Mutex<PluginManager>>,
        start_time: std::time::Instant,
    ) -> DaemonResponse {
        match message {
            ClientMessage::Toggle { scratchpad } => {
                debug!("🔄 Processing toggle for scratchpad: {}", scratchpad);
                let mut pm = plugin_manager.lock().await;
                
                match pm.handle_command("scratchpads", "toggle", &[&scratchpad]).await {
                    Ok(result) => DaemonResponse::Success { message: result },
                    Err(e) => DaemonResponse::Error { message: e.to_string() },
                }
            }
            
            ClientMessage::Expose => {
                debug!("🪟 Processing expose command");
                // TODO: Implement expose functionality
                DaemonResponse::Error { 
                    message: "Expose functionality not yet implemented".to_string() 
                }
            }
            
            ClientMessage::Reload => {
                debug!("⚡ Processing reload command");
                // TODO: Implement config reload
                DaemonResponse::Error { 
                    message: "Reload functionality not yet implemented".to_string() 
                }
            }
            
            ClientMessage::Status => {
                debug!("📊 Processing status command");
                let uptime = start_time.elapsed().as_secs();
                let plugins_loaded = {
                    let pm = plugin_manager.lock().await;
                    pm.get_plugin_count()
                };
                
                DaemonResponse::Status {
                    version: env!("CARGO_PKG_VERSION").to_string(),
                    uptime_seconds: uptime,
                    plugins_loaded,
                }
            }
            
            ClientMessage::List => {
                debug!("📋 Processing list command");
                let mut pm = plugin_manager.lock().await;
                
                match pm.handle_command("scratchpads", "list", &[]).await {
                    Ok(result) => {
                        // Parse the result to extract just the scratchpad names
                        let items = if result.starts_with("Available scratchpads: ") {
                            result.replace("Available scratchpads: ", "")
                                .split(", ")
                                .map(|s| s.to_string())
                                .collect()
                        } else {
                            vec![result]
                        };
                        DaemonResponse::List { items }
                    }
                    Err(e) => DaemonResponse::Error { message: e.to_string() },
                }
            }
        }
    }
}