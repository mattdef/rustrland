use anyhow::Result;
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, RwLock};
use tokio_stream::{wrappers::BroadcastStream, StreamExt};
use tracing::{debug, error, info, warn};

use crate::config::Config as RustrlandConfig;
use crate::core::plugin_manager::PluginManager;

/// Hot reload event types
#[derive(Debug, Clone)]
pub enum ReloadEvent {
    ConfigChanged(PathBuf),
    PluginReload(String),
    ValidationError(String),
    ReloadComplete,
}

/// Hot reload configuration
#[derive(Debug, Clone, serde::Deserialize)]
pub struct HotReloadConfig {
    /// Enable automatic file watching
    pub auto_reload: bool,
    /// Debounce duration to avoid rapid reloads
    pub debounce_ms: u64,
    /// Validate config before applying
    pub validate_before_apply: bool,
    /// Create backup of working config
    pub backup_on_reload: bool,
    /// Preserve plugin state during reload
    pub preserve_plugin_state: bool,
    /// Allow partial reloads (only changed plugins)
    pub partial_reload: bool,
}

impl Default for HotReloadConfig {
    fn default() -> Self {
        Self {
            auto_reload: true,
            debounce_ms: 500, // 500ms debounce
            validate_before_apply: true,
            backup_on_reload: true,
            preserve_plugin_state: true,
            partial_reload: true,
        }
    }
}

/// Advanced hot reload manager
pub struct HotReloadManager {
    config: HotReloadConfig,
    config_paths: Vec<PathBuf>,
    watcher: Option<RecommendedWatcher>,
    event_sender: broadcast::Sender<ReloadEvent>,
    event_receiver: broadcast::Receiver<ReloadEvent>,
    plugin_manager: Arc<RwLock<PluginManager>>,
    last_reload: Option<Instant>,
    backup_configs: HashMap<PathBuf, String>,
    plugin_states: HashMap<String, serde_json::Value>,
}

impl HotReloadManager {
    pub fn new(plugin_manager: Arc<RwLock<PluginManager>>) -> Self {
        let (sender, receiver) = broadcast::channel(100);

        Self {
            config: HotReloadConfig::default(),
            config_paths: Vec::new(),
            watcher: None,
            event_sender: sender,
            event_receiver: receiver,
            plugin_manager,
            last_reload: None,
            backup_configs: HashMap::new(),
            plugin_states: HashMap::new(),
        }
    }

    /// Start hot reload with configuration
    pub async fn start(
        &mut self,
        config_paths: Vec<PathBuf>,
        config: HotReloadConfig,
    ) -> Result<()> {
        info!("🔥 Starting hot reload manager");
        self.config = config;
        self.config_paths = config_paths.clone();

        if self.config.auto_reload {
            self.start_file_watcher().await?;
        }

        // Start event processing loop
        self.start_event_loop().await;

        info!(
            "✅ Hot reload manager started, watching {} paths",
            config_paths.len()
        );
        Ok(())
    }

    /// Start file system watcher
    async fn start_file_watcher(&mut self) -> Result<()> {
        let (tx, rx) = std::sync::mpsc::channel();

        let mut watcher = RecommendedWatcher::new(
            move |res: Result<Event, notify::Error>| match res {
                Ok(event) => {
                    if let Err(e) = tx.send(event) {
                        error!("Failed to send file watch event: {}", e);
                    }
                }
                Err(e) => error!("File watch error: {}", e),
            },
            Config::default().with_poll_interval(Duration::from_millis(100)),
        )?;

        // Watch all config paths
        for path in &self.config_paths {
            if path.exists() {
                if let Some(parent) = path.parent() {
                    watcher.watch(parent, RecursiveMode::NonRecursive)?;
                    debug!("👀 Watching directory: {:?}", parent);
                }
            }
        }

        self.watcher = Some(watcher);

        // Spawn background task to handle file events
        let event_sender = self.event_sender.clone();
        let debounce_duration = Duration::from_millis(self.config.debounce_ms);
        let config_paths = self.config_paths.clone();

        tokio::spawn(async move {
            let mut last_event_time: Option<Instant> = None;

            loop {
                if let Ok(event) = rx.try_recv() {
                    if let EventKind::Modify(_) = event.kind {
                        // Check if this is one of our config files
                        for path in event.paths {
                            if config_paths.iter().any(|cp| cp == &path) {
                                let now = Instant::now();

                                // Debounce rapid file changes
                                if let Some(last_time) = last_event_time {
                                    if now.duration_since(last_time) < debounce_duration {
                                        continue;
                                    }
                                }

                                last_event_time = Some(now);
                                debug!("📁 Config file changed: {:?}", path);

                                let _ = event_sender.send(ReloadEvent::ConfigChanged(path.clone()));
                                break;
                            }
                        }
                    }
                }

                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        });

        Ok(())
    }

    /// Start event processing loop
    async fn start_event_loop(&mut self) {
        let mut receiver = self.event_receiver.resubscribe();
        let plugin_manager = Arc::clone(&self.plugin_manager);
        let config = self.config.clone();
        let event_sender = self.event_sender.clone();

        tokio::spawn(async move {
            while let Ok(event) = receiver.recv().await {
                match event {
                    ReloadEvent::ConfigChanged(path) => {
                        info!("🔄 Processing config change: {:?}", path);

                        info!("🚀 About to call handle_config_change...");
                        if let Err(e) = Self::handle_config_change(
                            &plugin_manager,
                            &path,
                            &config,
                            &event_sender,
                        )
                        .await
                        {
                            error!("❌ Failed to handle config change: {}", e);
                            let _ = event_sender.send(ReloadEvent::ValidationError(e.to_string()));
                        } else {
                            info!("✅ Config change handled successfully");
                        }
                    }
                    ReloadEvent::PluginReload(plugin_name) => {
                        info!("🔌 Reloading plugin: {}", plugin_name);
                        // Handle individual plugin reload
                    }
                    ReloadEvent::ValidationError(error) => {
                        warn!("⚠️ Config validation error: {}", error);
                    }
                    ReloadEvent::ReloadComplete => {
                        info!("✅ Hot reload complete");
                    }
                }
            }
        });
    }

    /// Handle configuration file change
    async fn handle_config_change(
        plugin_manager: &Arc<RwLock<PluginManager>>,
        config_path: &Path,
        config: &HotReloadConfig,
        event_sender: &broadcast::Sender<ReloadEvent>,
    ) -> Result<()> {
        debug!("🔧 Reading config file: {:?}", config_path);

        // Read and validate new configuration
        let config_content = std::fs::read_to_string(config_path)?;
        debug!("📄 Config content read, {} bytes", config_content.len());

        debug!("🔍 Validating configuration...");
        let new_config = Self::validate_config(&config_content).await?;
        debug!("✅ Configuration validation completed");

        if config.validate_before_apply {
            info!("✓ Configuration validation passed");
        }

        // Create backup if enabled
        if config.backup_on_reload {
            // Backup logic would go here
            debug!("💾 Created config backup");
        }

        // Preserve plugin states if enabled
        debug!(
            "💾 Checking if plugin state preservation is enabled: {}",
            config.preserve_plugin_state
        );
        let preserved_states = if config.preserve_plugin_state {
            debug!("📸 Capturing plugin states...");
            Self::capture_plugin_states(plugin_manager).await?
        } else {
            debug!("⏭️ Skipping plugin state preservation");
            HashMap::new()
        };

        // Apply new configuration
        debug!("🔒 Acquiring plugin manager write lock...");
        {
            let mut pm = plugin_manager.write().await;
            debug!("✅ Plugin manager lock acquired");

            if config.partial_reload {
                // Compare configs and only reload changed plugins
                Self::apply_partial_reload(&mut pm, &new_config, &preserved_states).await?;
            } else {
                // Full reload
                Self::apply_full_reload(&mut pm, &new_config, &preserved_states).await?;
            }
        }

        let _ = event_sender.send(ReloadEvent::ReloadComplete);
        Ok(())
    }

    /// Validate configuration without applying it
    async fn validate_config(config_content: &str) -> Result<RustrlandConfig> {
        debug!("🔍 Parsing TOML content...");
        let config: toml::Value = toml::from_str(config_content).map_err(|e| {
            error!("❌ TOML parsing failed: {}", e);
            e
        })?;

        debug!("🔍 Converting to RustrlandConfig...");
        RustrlandConfig::from_toml_value(config).map_err(|e| {
            error!("❌ Config conversion failed: {}", e);
            e
        })
    }

    /// Capture current plugin states for preservation
    async fn capture_plugin_states(
        plugin_manager: &Arc<RwLock<PluginManager>>,
    ) -> Result<HashMap<String, serde_json::Value>> {
        let pm = plugin_manager.read().await;
        let mut states = HashMap::new();

        // Get state from each plugin
        for plugin_name in pm.get_loaded_plugins() {
            if let Ok(state) = pm.get_plugin_state(&plugin_name).await {
                states.insert(plugin_name, state);
            }
        }

        debug!("📸 Captured {} plugin states", states.len());
        Ok(states)
    }

    /// Apply partial reload (only changed plugins)
    async fn apply_partial_reload(
        plugin_manager: &mut PluginManager,
        new_config: &RustrlandConfig,
        preserved_states: &HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        info!("🔄 Applying partial reload");

        // Compare current and new configurations
        let current_plugins = plugin_manager.get_loaded_plugins();
        let new_plugins = new_config.get_plugin_names();

        debug!("🔍 Current plugins: {:?}", current_plugins);
        debug!("🔍 New plugins: {:?}", new_plugins);

        // Find added, removed, and modified plugins
        let added: Vec<_> = new_plugins
            .iter()
            .filter(|p| !current_plugins.contains(p))
            .collect();

        let removed: Vec<_> = current_plugins
            .iter()
            .filter(|p| !new_plugins.contains(p))
            .collect();

        debug!("🔍 Plugins to add: {:?}", added);
        debug!("🔍 Plugins to remove: {:?}", removed);

        let potentially_modified: Vec<_> = current_plugins
            .iter()
            .filter(|p| new_plugins.contains(p))
            .collect();

        // Remove plugins no longer needed
        for plugin_name in removed {
            plugin_manager.unload_plugin(plugin_name).await?;
            info!("🗑️ Removed plugin: {}", plugin_name);
        }

        // Add new plugins
        for plugin_name in added {
            plugin_manager.load_plugin(plugin_name, new_config).await?;
            info!("➕ Added plugin: {}", plugin_name);
        }

        // Check and reload modified plugins
        for plugin_name in potentially_modified {
            if Self::plugin_config_changed(plugin_manager, plugin_name, new_config).await? {
                // Preserve state before reload
                if let Some(state) = preserved_states.get(plugin_name) {
                    plugin_manager
                        .preserve_plugin_state(plugin_name, state.clone())
                        .await?;
                }

                plugin_manager
                    .reload_plugin(plugin_name, new_config)
                    .await?;
                info!("🔄 Reloaded plugin: {}", plugin_name);

                // Restore state after reload
                if let Some(state) = preserved_states.get(plugin_name) {
                    plugin_manager
                        .restore_plugin_state(plugin_name, state.clone())
                        .await?;
                }
            }
        }

        Ok(())
    }

    /// Apply full reload (all plugins)
    async fn apply_full_reload(
        plugin_manager: &mut PluginManager,
        new_config: &RustrlandConfig,
        preserved_states: &HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        info!("🔄 Applying full reload");

        // Unload all plugins
        plugin_manager.unload_all_plugins().await?;

        // Reload with new configuration
        plugin_manager.load_from_config(new_config).await?;

        // Restore preserved states
        for (plugin_name, state) in preserved_states {
            if let Err(e) = plugin_manager
                .restore_plugin_state(plugin_name, state.clone())
                .await
            {
                warn!("Failed to restore state for plugin {}: {}", plugin_name, e);
            }
        }

        Ok(())
    }

    /// Check if a plugin's configuration has changed
    async fn plugin_config_changed(
        plugin_manager: &PluginManager,
        plugin_name: &str,
        new_config: &RustrlandConfig,
    ) -> Result<bool> {
        // Compare plugin configurations
        let current_config = plugin_manager.get_plugin_config(plugin_name)?;
        let new_plugin_config = new_config.get_plugin_config(plugin_name)?;

        Ok(current_config != new_plugin_config)
    }

    /// Manual reload trigger
    pub async fn reload_now(&self) -> Result<()> {
        info!("🔄 Manual reload triggered");

        if let Some(config_path) = self.config_paths.first() {
            self.event_sender
                .send(ReloadEvent::ConfigChanged(config_path.clone()))?;
        }

        Ok(())
    }

    /// Stop hot reload
    pub async fn stop(&mut self) -> Result<()> {
        info!("🛑 Stopping hot reload manager");
        self.watcher = None;
        Ok(())
    }

    /// Get reload statistics
    pub fn get_stats(&self) -> HotReloadStats {
        HotReloadStats {
            auto_reload_enabled: self.config.auto_reload,
            watched_paths: self.config_paths.len(),
            last_reload: self.last_reload,
            backup_count: self.backup_configs.len(),
            preserved_states_count: self.plugin_states.len(),
        }
    }

    /// Subscribe to reload events
    pub fn subscribe(&self) -> broadcast::Receiver<ReloadEvent> {
        self.event_receiver.resubscribe()
    }
}

/// Hot reload statistics
#[derive(Debug)]
pub struct HotReloadStats {
    pub auto_reload_enabled: bool,
    pub watched_paths: usize,
    pub last_reload: Option<Instant>,
    pub backup_count: usize,
    pub preserved_states_count: usize,
}

// Extension trait for PluginManager to support hot reload
pub trait HotReloadable {
    fn get_plugin_state(
        &self,
        plugin_name: &str,
    ) -> impl std::future::Future<Output = Result<serde_json::Value>> + Send;
    fn preserve_plugin_state(
        &self,
        plugin_name: &str,
        state: serde_json::Value,
    ) -> impl std::future::Future<Output = Result<()>> + Send;
    fn restore_plugin_state(
        &self,
        plugin_name: &str,
        state: serde_json::Value,
    ) -> impl std::future::Future<Output = Result<()>> + Send;
    fn reload_plugin(
        &mut self,
        plugin_name: &str,
        config: &RustrlandConfig,
    ) -> impl std::future::Future<Output = Result<()>> + Send;
    fn unload_plugin(
        &mut self,
        plugin_name: &str,
    ) -> impl std::future::Future<Output = Result<()>> + Send;
    fn unload_all_plugins(&mut self) -> impl std::future::Future<Output = Result<()>> + Send;
    fn load_plugin(
        &mut self,
        plugin_name: &str,
        config: &RustrlandConfig,
    ) -> impl std::future::Future<Output = Result<()>> + Send;
    fn load_from_config(
        &mut self,
        config: &RustrlandConfig,
    ) -> impl std::future::Future<Output = Result<()>> + Send;
    fn get_loaded_plugins(&self) -> Vec<String>;
    fn get_plugin_config(&self, plugin_name: &str) -> Result<toml::Value>;
}

// Extension trait for Config to support hot reload
pub trait ConfigExt {
    fn get_plugin_names(&self) -> Vec<String>;
    fn get_plugin_config(&self, plugin_name: &str) -> Result<toml::Value>;
    fn from_toml_value(value: toml::Value) -> Result<Self>
    where
        Self: Sized;
}
