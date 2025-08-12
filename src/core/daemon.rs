use anyhow::Result;
use tracing::{info, error, warn};
use tokio::signal;
use std::time::Duration;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::config::Config;
use crate::ipc::{HyprlandClient, server::IpcServer};
use crate::core::plugin_manager::PluginManager;
use crate::core::event_handler::EventHandler;

pub struct Daemon {
    config: Config,
    hyprland_client: HyprlandClient,
    plugin_manager: Arc<RwLock<PluginManager>>,
    event_handler: EventHandler,
}

impl Daemon {
    pub async fn new(config_path: &str) -> Result<Self> {
        info!("📄 Loading configuration from: {}", config_path);
        let config = Config::load(config_path).await?;
        
        info!("🔌 Connecting to Hyprland IPC");
        let hyprland_client = HyprlandClient::new().await?;
        
        info!("🔧 Initializing plugin manager");
        let mut plugin_manager = PluginManager::new();
        plugin_manager.load_plugins(&config, Arc::new(hyprland_client.clone())).await?;
        let plugin_manager = Arc::new(RwLock::new(plugin_manager));
        
        info!("📡 Setting up event handler");
        let event_handler = EventHandler::new();
        
        Ok(Self {
            config,
            hyprland_client,
            plugin_manager,
            event_handler,
        })
    }
    
    pub async fn run(&mut self) -> Result<()> {
        info!("🚀 Starting Rustrland daemon");
        
        // Test Hyprland connection
        if let Err(e) = self.hyprland_client.test_connection().await {
            error!("❌ Failed to connect to Hyprland: {}", e);
            return Err(e);
        }
        
        info!("✅ Connected to Hyprland successfully");
        
        // Start IPC server
        let ipc_server = IpcServer::new(Arc::clone(&self.plugin_manager));
        tokio::spawn(async move {
            if let Err(e) = ipc_server.start().await {
                error!("❌ IPC server error: {}", e);
            }
        });
        
        // Start event loop  
        self.hyprland_client.create_event_listener().await?;
        let mut reload_interval = tokio::time::interval(Duration::from_secs(1));
        
        info!("🔄 Starting event loop");
        
        loop {
            tokio::select! {
                // Handle Hyprland events
                event_result = self.hyprland_client.get_next_event() => {
                    match event_result {
                        Ok(event) => {
                            let mut pm = self.plugin_manager.write().await;
                            if let Err(e) = self.event_handler.handle_event(&event, &mut pm).await {
                                warn!("⚠️  Error handling event: {}", e);
                            }
                        }
                        Err(e) => {
                            error!("❌ Error receiving event: {}", e);
                            // Try to reconnect
                            tokio::time::sleep(Duration::from_secs(1)).await;
                        }
                    }
                }
                
                // Periodic maintenance
                _ = reload_interval.tick() => {
                    // Could check for config changes, cleanup, etc.
                }
                
                // Handle shutdown signal
                _ = signal::ctrl_c() => {
                    info!("🛑 Received shutdown signal");
                    break;
                }
            }
        }
        
        info!("👋 Shutting down Rustrland");
        Ok(())
    }
}
