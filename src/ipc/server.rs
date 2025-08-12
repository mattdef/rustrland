use anyhow::Result;
use tokio::net::{UnixListener, UnixStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{info, debug, warn, error};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::ipc::protocol::{ClientMessage, DaemonResponse, get_socket_path};
use crate::core::plugin_manager::PluginManager;
use crate::core::hot_reload::HotReloadable;

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
                let mut pm = plugin_manager.lock().await;
                
                match pm.handle_command("expose", "toggle", &[]).await {
                    Ok(result) => DaemonResponse::Success { message: result },
                    Err(e) => DaemonResponse::Error { message: e.to_string() },
                }
            }
            
            ClientMessage::ExposeAction { action } => {
                debug!("🪟 Processing expose action: {}", action);
                let mut pm = plugin_manager.lock().await;
                
                match pm.handle_command("expose", &action, &[]).await {
                    Ok(result) => DaemonResponse::Success { message: result },
                    Err(e) => DaemonResponse::Error { message: e.to_string() },
                }
            }
            
            ClientMessage::WorkspaceAction { action, arg } => {
                debug!("🏢 Processing workspace action: {} {:?}", action, arg);
                let mut pm = plugin_manager.lock().await;
                
                let args: Vec<&str> = arg.as_ref().map(|s| vec![s.as_str()]).unwrap_or_default();
                match pm.handle_command("workspaces_follow_focus", &action, &args).await {
                    Ok(result) => DaemonResponse::Success { message: result },
                    Err(e) => DaemonResponse::Error { message: e.to_string() },
                }
            }
            
            ClientMessage::MagnifyAction { action, arg } => {
                debug!("🔍 Processing magnify action: {} {:?}", action, arg);
                let mut pm = plugin_manager.lock().await;
                
                let args: Vec<&str> = arg.as_ref().map(|s| vec![s.as_str()]).unwrap_or_default();
                match pm.handle_command("magnify", &action, &args).await {
                    Ok(result) => DaemonResponse::Success { message: result },
                    Err(e) => DaemonResponse::Error { message: e.to_string() },
                }
            }
            
            ClientMessage::Reload => {
                debug!("⚡ Processing reload command");
                let mut pm = plugin_manager.lock().await;
                
                match Self::handle_manual_reload(&mut pm).await {
                    Ok(message) => DaemonResponse::Success { message },
                    Err(e) => DaemonResponse::Error { message: e.to_string() },
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
    
    /// Handle manual reload request
    async fn handle_manual_reload(
        plugin_manager: &mut PluginManager,
    ) -> Result<String> {
        info!("🔄 Manual reload requested");
        
        // Find config file path (simplified - in real implementation would use the daemon's config path)
        let config_path = std::env::var("HOME")
            .map(|home| format!("{}/.config/hypr/rustrland.toml", home))
            .unwrap_or_else(|_| "rustrland.toml".to_string());
            
        // Read and parse new configuration
        let config_content = tokio::fs::read_to_string(&config_path).await
            .map_err(|e| anyhow::anyhow!("Failed to read config file '{}': {}", config_path, e))?;
            
        let config_value: toml::Value = toml::from_str(&config_content)
            .map_err(|e| anyhow::anyhow!("Failed to parse config: {}", e))?;
            
        let new_config = crate::config::Config::from_toml_value(config_value)
            .map_err(|e| anyhow::anyhow!("Invalid configuration: {}", e))?;
        
        // Get current plugins for comparison
        let current_plugins = plugin_manager.get_loaded_plugins();
        let new_plugins = new_config.get_plugins();
        
        info!("🔍 Comparing configurations:");
        info!("   Current plugins: {:?}", current_plugins);
        info!("   New plugins: {:?}", new_plugins);
        
        // Perform smart reload
        let mut reloaded = Vec::new();
        let mut added = Vec::new();
        let mut removed = Vec::new();
        
        // Find removed plugins
        for plugin in &current_plugins {
            if !new_plugins.contains(plugin) {
                        plugin_manager.unload_plugin(plugin).await?;
                removed.push(plugin.clone());
            }
        }
        
        // Find added plugins
        for plugin in &new_plugins {
            if !current_plugins.contains(plugin) {
                plugin_manager.load_plugin(plugin, &new_config).await?;
                added.push(plugin.clone());
            }
        }
        
        // Reload existing plugins (simplified - doesn't check if config actually changed)
        for plugin in &new_plugins {
            if current_plugins.contains(plugin) {
                plugin_manager.reload_plugin(plugin, &new_config).await?;
                reloaded.push(plugin.clone());
            }
        }
        
        // Build result message
        let mut messages = Vec::new();
        
        if !removed.is_empty() {
            messages.push(format!("🗑️ Removed: {}", removed.join(", ")));
        }
        
        if !added.is_empty() {
            messages.push(format!("➕ Added: {}", added.join(", ")));
        }
        
        if !reloaded.is_empty() {
            messages.push(format!("🔄 Reloaded: {}", reloaded.join(", ")));
        }
        
        if messages.is_empty() {
            Ok("✅ Configuration up-to-date, no changes needed".to_string())
        } else {
            Ok(format!("✅ Reload complete: {}", messages.join("; ")))
        }
    }
}