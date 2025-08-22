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
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

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
                // Try to watch the file directly first
                match watcher.watch(path, RecursiveMode::NonRecursive) {
                    Ok(()) => {
                        info!("👀 Watching file directly: {:?}", path);
                    }
                    Err(_) => {
                        // If watching the file directly fails, watch the parent directory
                        if let Some(parent) = path.parent() {
                            watcher.watch(parent, RecursiveMode::NonRecursive)?;
                            info!("👀 Watching directory: {:?}", parent);
                        }
                    }
                }
            } else {
                error!("⚠️ Config file does not exist: {:?}", path);
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
                // Use tokio async recv
                match rx.recv().await {
                    Some(event) => {
                        debug!("🔍 File event received: {:?}", event);

                        // Handle various event types that indicate file changes
                        match event.kind {
                            EventKind::Modify(_) | EventKind::Create(_) | EventKind::Access(_) => {
                                // Check if this is one of our config files
                                for path in event.paths {
                                    debug!(
                                        "🔍 Checking path: {:?} against {:?}",
                                        path, config_paths
                                    );

                                    // Check both exact match and filename match
                                    let is_config_file = config_paths.iter().any(|cp| {
                                        cp == &path
                                            || (path.file_name() == cp.file_name()
                                                && cp.file_name().is_some())
                                    });

                                    if is_config_file {
                                        let now = Instant::now();

                                        // Debounce rapid file changes
                                        if let Some(last_time) = last_event_time {
                                            if now.duration_since(last_time) < debounce_duration {
                                                debug!(
                                                    "🔍 Event debounced, too soon after last event"
                                                );
                                                continue;
                                            }
                                        }

                                        last_event_time = Some(now);
                                        info!("📁 Config file changed: {:?}", path);

                                        let _ = event_sender
                                            .send(ReloadEvent::ConfigChanged(path.clone()));
                                        break;
                                    }
                                }
                            }
                            _ => {
                                debug!("🔍 Ignoring event type: {:?}", event.kind);
                            }
                        }
                    }
                    None => {
                        debug!("🔍 File watcher channel closed");
                        break;
                    }
                }
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
            Self::create_config_backup(config_path).await?;
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

        // Apply new configuration with error handling and rollback
        debug!("🔒 Acquiring plugin manager write lock...");
        let reload_result = {
            let mut pm = plugin_manager.write().await;
            debug!("✅ Plugin manager lock acquired");

            if config.partial_reload {
                info!("🔄 Applying partial reload");
                Self::apply_partial_reload(&mut pm, &new_config, &preserved_states).await
            } else {
                info!("🔄 Applying full reload");
                Self::apply_full_reload(&mut pm, &new_config, &preserved_states).await
            }
        };

        // Handle reload results with automatic recovery
        match reload_result {
            Ok(()) => {
                info!("✅ Config change handled successfully");

                // Cleanup old backups (keep last 5)
                if config.backup_on_reload {
                    if let Err(e) = Self::cleanup_old_backups(config_path, 5).await {
                        warn!("Failed to cleanup old backups: {}", e);
                    }
                }

                let _ = event_sender.send(ReloadEvent::ReloadComplete);
                Ok(())
            }
            Err(reload_error) => {
                error!("❌ Configuration reload failed: {}", reload_error);
                let _ = event_sender.send(ReloadEvent::ValidationError(format!(
                    "Reload failed: {}",
                    reload_error
                )));

                // Attempt automatic rollback if backup is enabled
                if config.backup_on_reload {
                    match Self::handle_reload_failure(
                        plugin_manager,
                        config_path,
                        &reload_error,
                        &preserved_states,
                    )
                    .await
                    {
                        Ok(()) => {
                            warn!("⚠️ Original reload failed but automatic recovery succeeded");
                            let _ = event_sender.send(ReloadEvent::ReloadComplete);
                            Ok(())
                        }
                        Err(recovery_error) => {
                            error!("💥 Both reload and recovery failed: {}", recovery_error);
                            Err(recovery_error)
                        }
                    }
                } else {
                    error!(
                        "❌ Reload failed and backup is disabled - no automatic recovery available"
                    );
                    Err(reload_error)
                }
            }
        }
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

    // ============================================================================
    // CONFIGURATION BACKUP AND RECOVERY SYSTEM
    // ============================================================================

    /// Create backup of current configuration before making changes
    async fn create_config_backup(config_path: &Path) -> Result<()> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let backup_filename = format!(
            "{}.backup.{}",
            config_path
                .file_name()
                .unwrap_or_default()
                .to_str()
                .unwrap_or("config"),
            timestamp
        );

        let backup_path = config_path
            .parent()
            .unwrap_or(Path::new("/tmp"))
            .join(backup_filename);

        tokio::fs::copy(config_path, &backup_path).await?;

        info!("💾 Configuration backup created: {:?}", backup_path);
        Ok(())
    }

    /// Restore configuration from the most recent backup
    async fn restore_config_backup(config_path: &Path) -> Result<()> {
        let parent_dir = config_path.parent().unwrap_or(Path::new("/tmp"));

        let config_name = config_path
            .file_name()
            .unwrap_or_default()
            .to_str()
            .unwrap_or("config");

        // Find the most recent backup file
        let mut backup_files = Vec::new();
        let mut dir_entries = tokio::fs::read_dir(parent_dir).await?;

        while let Some(entry) = dir_entries.next_entry().await? {
            if let Some(name) = entry.file_name().to_str() {
                if name.starts_with(&format!("{}.backup.", config_name)) {
                    if let Some(timestamp_str) = name.split('.').next_back() {
                        if let Ok(timestamp) = timestamp_str.parse::<u64>() {
                            backup_files.push((timestamp, entry.path()));
                        }
                    }
                }
            }
        }

        if backup_files.is_empty() {
            return Err(anyhow::anyhow!("No backup files found for configuration"));
        }

        // Sort by timestamp (newest first)
        backup_files.sort_by(|a, b| b.0.cmp(&a.0));
        let most_recent_backup = &backup_files[0].1;

        // Restore the backup
        tokio::fs::copy(most_recent_backup, config_path).await?;

        warn!(
            "🔄 Configuration restored from backup: {:?}",
            most_recent_backup
        );
        Ok(())
    }

    /// Cleanup old backup files, keeping only the most recent N backups
    async fn cleanup_old_backups(config_path: &Path, keep_count: usize) -> Result<()> {
        let parent_dir = config_path.parent().unwrap_or(Path::new("/tmp"));

        let config_name = config_path
            .file_name()
            .unwrap_or_default()
            .to_str()
            .unwrap_or("config");

        let mut backup_files = Vec::new();
        let mut dir_entries = tokio::fs::read_dir(parent_dir).await?;

        while let Some(entry) = dir_entries.next_entry().await? {
            if let Some(name) = entry.file_name().to_str() {
                if name.starts_with(&format!("{}.backup.", config_name)) {
                    if let Some(timestamp_str) = name.split('.').next_back() {
                        if let Ok(timestamp) = timestamp_str.parse::<u64>() {
                            backup_files.push((timestamp, entry.path()));
                        }
                    }
                }
            }
        }

        if backup_files.len() <= keep_count {
            return Ok(()); // No cleanup needed
        }

        // Sort by timestamp (newest first)
        backup_files.sort_by(|a, b| b.0.cmp(&a.0));

        // Remove old backups beyond keep_count
        let to_remove = backup_files.split_off(keep_count);
        for (_timestamp, backup_path) in to_remove {
            if let Err(e) = tokio::fs::remove_file(&backup_path).await {
                warn!("Failed to remove old backup {:?}: {}", backup_path, e);
            } else {
                debug!("🗑️ Removed old backup: {:?}", backup_path);
            }
        }

        if backup_files.len() > keep_count {
            info!(
                "🧹 Cleaned up {} old backup files",
                backup_files.len() - keep_count
            );
        }

        Ok(())
    }

    /// Handle configuration reload failure with automatic rollback
    async fn handle_reload_failure(
        plugin_manager: &Arc<RwLock<PluginManager>>,
        config_path: &Path,
        error: &anyhow::Error,
        preserved_states: &HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        error!("❌ Configuration reload failed: {}", error);
        error!("🔄 Attempting automatic rollback to previous configuration...");

        // Attempt to restore from backup
        match Self::restore_config_backup(config_path).await {
            Ok(()) => {
                info!("✅ Configuration restored from backup");

                // Try to reload with restored configuration
                match Self::reload_from_restored_config(
                    plugin_manager,
                    config_path,
                    preserved_states,
                )
                .await
                {
                    Ok(()) => {
                        info!("✅ Successfully recovered from reload failure");
                        Ok(())
                    }
                    Err(rollback_error) => {
                        error!("❌ Rollback also failed: {}", rollback_error);
                        error!(
                            "💥 System is in an inconsistent state - manual intervention required"
                        );
                        Err(anyhow::anyhow!("Both reload and rollback failed: original error: {}, rollback error: {}", error, rollback_error))
                    }
                }
            }
            Err(backup_error) => {
                error!("❌ Failed to restore backup: {}", backup_error);
                Err(anyhow::anyhow!(
                    "Reload failed and backup restoration failed: {}",
                    backup_error
                ))
            }
        }
    }

    /// Reload configuration from restored backup file
    async fn reload_from_restored_config(
        plugin_manager: &Arc<RwLock<PluginManager>>,
        config_path: &Path,
        preserved_states: &HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        debug!("🔄 Reloading from restored configuration...");

        let config_content = tokio::fs::read_to_string(config_path).await?;
        let restored_config = Self::validate_config(&config_content).await?;

        let mut pm = plugin_manager.write().await;

        // Full reload with preserved states
        pm.unload_all_plugins().await?;
        pm.load_from_config(&restored_config).await?;

        // Restore preserved states
        for (plugin_name, state) in preserved_states {
            if let Err(e) = pm.restore_plugin_state(plugin_name, state.clone()).await {
                warn!(
                    "Failed to restore state for plugin {} during rollback: {}",
                    plugin_name, e
                );
            }
        }

        info!("✅ Configuration successfully reloaded from backup");
        Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tempfile::tempdir;
    use tokio::fs;
    use tokio::sync::RwLock;
    use tokio_test;

    // Mock PluginManager for testing
    struct MockPluginManager {
        states: std::collections::HashMap<String, serde_json::Value>,
        config: Option<crate::config::Config>,
        plugins: Vec<String>,
    }

    impl MockPluginManager {
        fn new() -> Self {
            Self {
                states: std::collections::HashMap::new(),
                config: None,
                plugins: vec!["scratchpads".to_string(), "expose".to_string()],
            }
        }
    }

    impl HotReloadable for MockPluginManager {
        async fn get_plugin_state(&self, plugin_name: &str) -> Result<serde_json::Value> {
            Ok(serde_json::json!({
                "plugin_name": plugin_name,
                "loaded": true,
                "timestamp": 1234567890
            }))
        }

        async fn preserve_plugin_state(
            &self,
            _plugin_name: &str,
            _state: serde_json::Value,
        ) -> Result<()> {
            Ok(())
        }

        async fn restore_plugin_state(
            &self,
            _plugin_name: &str,
            _state: serde_json::Value,
        ) -> Result<()> {
            Ok(())
        }

        async fn reload_plugin(
            &mut self,
            _plugin_name: &str,
            _config: &crate::config::Config,
        ) -> Result<()> {
            Ok(())
        }

        async fn unload_plugin(&mut self, _plugin_name: &str) -> Result<()> {
            Ok(())
        }

        async fn unload_all_plugins(&mut self) -> Result<()> {
            Ok(())
        }

        async fn load_plugin(
            &mut self,
            _plugin_name: &str,
            _config: &crate::config::Config,
        ) -> Result<()> {
            Ok(())
        }

        async fn load_from_config(&mut self, config: &crate::config::Config) -> Result<()> {
            self.config = Some(config.clone());
            Ok(())
        }

        fn get_loaded_plugins(&self) -> Vec<String> {
            self.plugins.clone()
        }

        fn get_plugin_config(&self, _plugin_name: &str) -> Result<toml::Value> {
            Ok(toml::Value::Table(toml::Table::new()))
        }
    }

    #[tokio::test]
    async fn test_hot_reload_config_validation() {
        // Test valid configuration
        let valid_config = r#"
[rustrland]
plugins = ["scratchpads", "expose"]

[hot_reload]
auto_reload = true
debounce_ms = 500
"#;

        let result = HotReloadManager::validate_config(valid_config).await;
        assert!(result.is_ok(), "Valid config should pass validation");

        // Test invalid TOML
        let invalid_toml = r#"
[rustrland
plugins = ["scratchpads"  # Missing closing bracket
"#;

        let result = HotReloadManager::validate_config(invalid_toml).await;
        assert!(result.is_err(), "Invalid TOML should fail validation");
    }

    #[tokio::test]
    async fn test_config_backup_and_restore() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("test_config.toml");

        // Create initial config file
        let initial_config = r#"
[rustrland]
plugins = ["scratchpads"]

[hot_reload]
auto_reload = true
"#;
        fs::write(&config_path, initial_config).await.unwrap();

        // Test backup creation
        let result = HotReloadManager::create_config_backup(&config_path).await;
        assert!(result.is_ok(), "Backup creation should succeed");

        // Verify backup file exists
        let mut backup_files = Vec::new();
        let mut dir_entries = fs::read_dir(temp_dir.path()).await.unwrap();
        while let Some(entry) = dir_entries.next_entry().await.unwrap() {
            if let Some(name) = entry.file_name().to_str() {
                if name.starts_with("test_config.toml.backup.") {
                    backup_files.push(entry.path());
                }
            }
        }

        assert!(!backup_files.is_empty(), "Backup file should be created");

        // Modify original config to simulate corruption
        fs::write(&config_path, "CORRUPTED CONFIG").await.unwrap();

        // Test restore
        let result = HotReloadManager::restore_config_backup(&config_path).await;
        assert!(result.is_ok(), "Restore should succeed");

        // Verify original content is restored
        let restored_content = fs::read_to_string(&config_path).await.unwrap();
        assert!(
            restored_content.contains("plugins = [\"scratchpads\"]"),
            "Original content should be restored"
        );
    }

    #[tokio::test]
    async fn test_backup_cleanup() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("test_config.toml");

        // Create config file
        fs::write(&config_path, "[rustrland]\nplugins = []")
            .await
            .unwrap();

        // Create multiple backup files
        for i in 1..=10 {
            let backup_path = temp_dir
                .path()
                .join(format!("test_config.toml.backup.{}", i));
            fs::write(&backup_path, format!("backup {}", i))
                .await
                .unwrap();
        }

        // Test cleanup (keep 5 most recent)
        let result = HotReloadManager::cleanup_old_backups(&config_path, 5).await;
        assert!(result.is_ok(), "Cleanup should succeed");

        // Count remaining backup files
        let mut backup_count = 0;
        let mut dir_entries = fs::read_dir(temp_dir.path()).await.unwrap();
        while let Some(entry) = dir_entries.next_entry().await.unwrap() {
            if let Some(name) = entry.file_name().to_str() {
                if name.starts_with("test_config.toml.backup.") {
                    backup_count += 1;
                }
            }
        }

        assert_eq!(backup_count, 5, "Should keep exactly 5 backup files");
    }

    #[tokio::test]
    async fn test_plugin_state_capture_and_restore() {
        let mock_pm = MockPluginManager::new();

        // Test individual state capture
        let state = mock_pm.get_plugin_state("scratchpads").await.unwrap();
        assert_eq!(state["plugin_name"], "scratchpads");
        assert_eq!(state["loaded"], true);
        assert_eq!(state["timestamp"], 1234567890);

        // Test state preservation
        let result = mock_pm
            .preserve_plugin_state("scratchpads", state.clone())
            .await;
        assert!(result.is_ok(), "State preservation should succeed");

        // Test state restoration
        let result = mock_pm.restore_plugin_state("scratchpads", state).await;
        assert!(result.is_ok(), "State restoration should succeed");
    }

    #[tokio::test]
    async fn test_hot_reload_config_default_values() {
        let config = HotReloadConfig::default();

        assert_eq!(config.auto_reload, true);
        assert_eq!(config.debounce_ms, 500);
        assert_eq!(config.validate_before_apply, true);
        assert_eq!(config.backup_on_reload, true);
        assert_eq!(config.preserve_plugin_state, true);
        assert_eq!(config.partial_reload, true);
    }

    #[tokio::test]
    async fn test_hot_reload_events() {
        // Test event creation
        let event = ReloadEvent::ConfigChanged(std::path::PathBuf::from("/test/path"));
        match event {
            ReloadEvent::ConfigChanged(path) => {
                assert_eq!(path.to_str().unwrap(), "/test/path");
            }
            _ => panic!("Wrong event type"),
        }

        let validation_event = ReloadEvent::ValidationError("Test error".to_string());
        match validation_event {
            ReloadEvent::ValidationError(msg) => {
                assert_eq!(msg, "Test error");
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[tokio::test]
    async fn test_partial_vs_full_reload_logic() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("test_config.toml");

        // Create initial config
        let config_content = r#"
[rustrland]
plugins = ["scratchpads", "expose"]

[hot_reload]
auto_reload = true
partial_reload = true
"#;
        fs::write(&config_path, config_content).await.unwrap();

        // Test that validation works for both partial and full reload scenarios
        let config = HotReloadManager::validate_config(config_content).await;
        assert!(config.is_ok(), "Config should be valid for reload testing");

        let parsed_config = config.unwrap();
        assert!(parsed_config
            .get_plugins()
            .contains(&"scratchpads".to_string()));
        assert!(parsed_config.get_plugins().contains(&"expose".to_string()));
    }

    #[test]
    fn test_hot_reload_stats() {
        // Test stats structure
        let stats = HotReloadStats {
            auto_reload_enabled: true,
            watched_paths: 2,
            last_reload: Some(Instant::now()),
            backup_count: 5,
            preserved_states_count: 3,
        };

        assert_eq!(stats.auto_reload_enabled, true);
        assert_eq!(stats.watched_paths, 2);
        assert_eq!(stats.backup_count, 5);
        assert_eq!(stats.preserved_states_count, 3);
        assert!(stats.last_reload.is_some());
    }

    #[tokio::test]
    async fn test_file_watching_configuration() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("test_config.toml");

        // Create config file
        fs::write(&config_path, "[rustrland]\nplugins = []")
            .await
            .unwrap();

        // Test that config file can be read and parsed
        let content = fs::read_to_string(&config_path).await.unwrap();
        assert!(
            content.contains("[rustrland]"),
            "Config should contain rustrland section"
        );

        // Test HotReloadConfig structure
        let hot_reload_config = HotReloadConfig {
            auto_reload: true,
            debounce_ms: 1000,
            validate_before_apply: true,
            backup_on_reload: false,
            preserve_plugin_state: false,
            partial_reload: false,
        };

        assert_eq!(hot_reload_config.auto_reload, true);
        assert_eq!(hot_reload_config.debounce_ms, 1000);
        assert_eq!(hot_reload_config.backup_on_reload, false);
    }
}
