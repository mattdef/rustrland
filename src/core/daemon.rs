use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::signal;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use crate::config::Config;
use crate::core::event_handler::EventHandler;
use crate::core::hot_reload::{HotReloadConfig, HotReloadManager};
use crate::core::plugin_manager::PluginManager;
use crate::ipc::{server::IpcServer, HyprlandClient};

pub struct Daemon {
    config: Config,
    config_path: String,
    hyprland_client: HyprlandClient,
    plugin_manager: Arc<RwLock<PluginManager>>,
    event_handler: EventHandler,
    hot_reload_manager: Option<HotReloadManager>,
}

impl Daemon {
    pub async fn new(config_path: &str) -> Result<Self> {
        info!("📄 Loading configuration from: {}", config_path);
        let config = Config::load(config_path).await?;

        info!("🔌 Connecting to Hyprland IPC");
        let hyprland_client = HyprlandClient::new().await?;

        info!("🔧 Initializing plugin manager");
        let mut plugin_manager = PluginManager::new();
        plugin_manager
            .load_plugins(&config, Arc::new(hyprland_client.clone()))
            .await?;
        let plugin_manager = Arc::new(RwLock::new(plugin_manager));

        info!("📡 Setting up event handler");
        let event_handler = EventHandler::new();

        // Initialize hot reload manager
        let hot_reload_manager = HotReloadManager::new(Arc::clone(&plugin_manager));

        Ok(Self {
            config,
            config_path: config_path.to_string(),
            hyprland_client,
            plugin_manager,
            event_handler,
            hot_reload_manager: Some(hot_reload_manager),
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

        // Start hot reload manager if configured
        // Parse hot reload configuration from config first
        let hot_reload_config = self.parse_hot_reload_config();

        if let Some(ref mut hot_reload_manager) = self.hot_reload_manager {
            if hot_reload_config.auto_reload {
                let config_paths = vec![PathBuf::from(&self.config_path)];

                if let Err(e) = hot_reload_manager
                    .start(config_paths, hot_reload_config)
                    .await
                {
                    error!("❌ Failed to start hot reload manager: {}", e);
                } else {
                    info!("🔥 Hot reload manager started successfully");
                }
            } else {
                debug!("🔥 Hot reload auto_reload is disabled");
            }
        }

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

    /// Parse hot reload configuration from config file
    fn parse_hot_reload_config(&self) -> HotReloadConfig {
        // Check if hot_reload section exists in config
        if let Some(hot_reload_value) = self.config.plugins.get("hot_reload") {
            // Try to parse the hot_reload configuration
            if let Ok(config) = hot_reload_value.clone().try_into::<HotReloadConfig>() {
                debug!(
                    "🔥 Parsed hot reload config: auto_reload={}, debounce_ms={}",
                    config.auto_reload, config.debounce_ms
                );
                return config;
            } else {
                warn!("⚠️ Invalid hot_reload configuration, using defaults");
            }
        } else {
            debug!("🔥 No hot_reload section found, using defaults");
        }

        // Return default configuration
        HotReloadConfig::default()
    }
}
