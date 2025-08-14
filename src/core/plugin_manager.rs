use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{error, info, warn};

use crate::config::Config;
use crate::core::global_cache::GlobalStateCache;
use crate::ipc::{HyprlandClient, HyprlandEvent};
use crate::plugins::expose::ExposePlugin;
use crate::plugins::magnify::MagnifyPlugin;
use crate::plugins::monitors::MonitorsPlugin;
use crate::plugins::scratchpads::ScratchpadsPlugin;
use crate::plugins::shift_monitors::ShiftMonitorsPlugin;
use crate::plugins::system_notifier::SystemNotifier;
use crate::plugins::toggle_special::ToggleSpecialPlugin;
use crate::plugins::wallpapers::WallpapersPlugin;
use crate::plugins::workspaces_follow_focus::WorkspacesFollowFocusPlugin;
use crate::plugins::{Plugin, PluginBox};

pub struct PluginManager {
    plugins: HashMap<String, PluginBox>,
    global_cache: Arc<GlobalStateCache>,
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginManager {
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
            global_cache: Arc::new(GlobalStateCache::new()),
        }
    }

    pub async fn load_plugins(
        &mut self,
        config: &Config,
        hyprland_client: Arc<HyprlandClient>,
    ) -> Result<()> {
        let plugins = config.get_plugins();
        info!("🔌 Loading {} plugins", plugins.len());

        for plugin_name in &plugins {
            if let Err(e) = self
                .load_single_plugin(plugin_name, config, Arc::clone(&hyprland_client))
                .await
            {
                error!("❌ Failed to load plugin '{}': {}", plugin_name, e);
            }
        }

        info!("✅ Loaded {} plugins successfully", self.plugins.len());
        Ok(())
    }

    async fn load_single_plugin(
        &mut self,
        plugin_name: &str,
        config: &Config,
        hyprland_client: Arc<HyprlandClient>,
    ) -> Result<()> {
        info!("📦 Loading plugin: {}", plugin_name);

        let mut plugin: PluginBox = match plugin_name {
            "scratchpads" => {
                let scratchpads_plugin = ScratchpadsPlugin::new();
                scratchpads_plugin
                    .set_hyprland_client(Arc::clone(&hyprland_client))
                    .await;
                Box::new(scratchpads_plugin)
            }
            "expose" => Box::new(ExposePlugin::new()),
            "workspaces_follow_focus" => Box::new(WorkspacesFollowFocusPlugin::new()),
            "magnify" => Box::new(MagnifyPlugin::new()),
            "shift_monitors" => Box::new(ShiftMonitorsPlugin::new()),
            "system_notifier" => Box::new(SystemNotifier::new()),
            "toggle_special" => Box::new(ToggleSpecialPlugin::new()),
            "monitors" => Box::new(MonitorsPlugin::new()),
            "wallpapers" => Box::new(WallpapersPlugin::new()),
            // Add more plugins here as they're implemented
            _ => {
                warn!("⚠️  Unknown plugin: {}", plugin_name);
                return Ok(());
            }
        };

        // Get plugin-specific config and wrap in Arc
        let plugin_config = config
            .plugins
            .get(plugin_name)
            .cloned()
            .unwrap_or(toml::Value::Table(toml::map::Map::new()));
        let plugin_config_arc = Arc::new(plugin_config.clone());

        // Store config in global cache for sharing
        self.global_cache
            .store_config(plugin_name.to_string(), plugin_config_arc.clone())
            .await;

        // For scratchpads, we need to pass both the plugin config and global variables
        if plugin_name == "scratchpads" {
            // Create a combined config with both scratchpad settings and variables
            let mut combined_config = toml::map::Map::new();

            // Add plugin-specific config
            if let toml::Value::Table(plugin_table) = plugin_config {
                for (key, value) in plugin_table {
                    combined_config.insert(key, value);
                }
            }

            // Add variables section (merged from both pyprland and rustrland)
            let merged_variables = config.get_variables();
            let variables_value = toml::Value::try_from(&merged_variables)
                .unwrap_or(toml::Value::Table(toml::map::Map::new()));
            combined_config.insert("variables".to_string(), variables_value);

            // Store variables in global cache
            self.global_cache.store_variables(merged_variables).await;

            let combined = toml::Value::Table(combined_config);
            let combined_arc = Arc::new(combined.clone());

            // Store combined config in cache and initialize
            self.global_cache
                .store_config(format!("{}_combined", plugin_name), combined_arc.clone())
                .await;
            plugin.init(&combined).await?;
        } else {
            // Initialize plugin normally
            plugin.init(&plugin_config).await?;
        }
        self.plugins.insert(plugin_name.to_string(), plugin);

        info!("✅ Plugin '{}' loaded successfully", plugin_name);
        Ok(())
    }

    pub async fn handle_event(&mut self, event: &HyprlandEvent) -> Result<()> {
        for (name, plugin) in &mut self.plugins {
            if let Err(e) = plugin.handle_event(event).await {
                warn!("⚠️  Plugin '{}' error handling event: {}", name, e);
            }
        }
        Ok(())
    }

    pub async fn handle_command(
        &mut self,
        plugin_name: &str,
        command: &str,
        args: &[&str],
    ) -> Result<String> {
        if let Some(plugin) = self.plugins.get_mut(plugin_name) {
            plugin.handle_command(command, args).await
        } else {
            Err(anyhow::anyhow!("Plugin '{}' not found", plugin_name))
        }
    }

    pub fn get_plugin_count(&self) -> usize {
        self.plugins.len()
    }

    pub fn get_global_cache(&self) -> Arc<GlobalStateCache> {
        Arc::clone(&self.global_cache)
    }
}

// Implementation of HotReloadable trait for PluginManager
impl super::hot_reload::HotReloadable for PluginManager {
    async fn get_plugin_state(&self, plugin_name: &str) -> Result<serde_json::Value> {
        // For now, return empty JSON object - plugins can implement their own state serialization
        if self.plugins.contains_key(plugin_name) {
            Ok(serde_json::json!({}))
        } else {
            Err(anyhow::anyhow!("Plugin '{}' not found", plugin_name))
        }
    }

    async fn preserve_plugin_state(
        &self,
        _plugin_name: &str,
        _state: serde_json::Value,
    ) -> Result<()> {
        // Plugin state preservation would be implemented here
        Ok(())
    }

    async fn restore_plugin_state(
        &self,
        _plugin_name: &str,
        _state: serde_json::Value,
    ) -> Result<()> {
        // Plugin state restoration would be implemented here
        Ok(())
    }

    async fn reload_plugin(&mut self, plugin_name: &str, config: &Config) -> Result<()> {
        info!("🔄 Reloading plugin: {}", plugin_name);

        // Remove existing plugin
        self.plugins.remove(plugin_name);

        // Load fresh plugin
        // Note: This is a simplified implementation - a full version would preserve the hyprland client
        let hyprland_client = Arc::new(crate::ipc::HyprlandClient::new().await?);
        self.load_single_plugin(plugin_name, config, hyprland_client)
            .await?;

        Ok(())
    }

    async fn unload_plugin(&mut self, plugin_name: &str) -> Result<()> {
        if let Some(_plugin) = self.plugins.remove(plugin_name) {
            info!("🗑️ Unloaded plugin: {}", plugin_name);
        }
        Ok(())
    }

    async fn unload_all_plugins(&mut self) -> Result<()> {
        let count = self.plugins.len();
        self.plugins.clear();
        info!("🗑️ Unloaded all {} plugins", count);
        Ok(())
    }

    async fn load_plugin(&mut self, plugin_name: &str, config: &Config) -> Result<()> {
        let hyprland_client = Arc::new(crate::ipc::HyprlandClient::new().await?);
        self.load_single_plugin(plugin_name, config, hyprland_client)
            .await
    }

    async fn load_from_config(&mut self, config: &Config) -> Result<()> {
        let hyprland_client = Arc::new(crate::ipc::HyprlandClient::new().await?);
        self.load_plugins(config, hyprland_client).await
    }

    fn get_loaded_plugins(&self) -> Vec<String> {
        self.plugins.keys().cloned().collect()
    }

    fn get_plugin_config(&self, plugin_name: &str) -> Result<toml::Value> {
        // This would return the plugin's configuration from the config
        // For now, return empty config
        if self.plugins.contains_key(plugin_name) {
            Ok(toml::Value::Table(toml::Table::new()))
        } else {
            Err(anyhow::anyhow!("Plugin '{}' not found", plugin_name))
        }
    }
}
