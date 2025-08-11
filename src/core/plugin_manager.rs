use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{info, warn, error};

use crate::plugins::{Plugin, PluginBox};
use crate::plugins::scratchpads::ScratchpadsPlugin;
use crate::plugins::expose::ExposePlugin;
use crate::plugins::workspaces_follow_focus::WorkspacesFollowFocusPlugin;
use crate::plugins::magnify::MagnifyPlugin;
use crate::config::Config;
use crate::ipc::{HyprlandEvent, HyprlandClient};

pub struct PluginManager {
    plugins: HashMap<String, PluginBox>,
}

impl PluginManager {
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
        }
    }
    
    pub async fn load_plugins(&mut self, config: &Config, hyprland_client: Arc<HyprlandClient>) -> Result<()> {
        let plugins = config.get_plugins();
        info!("🔌 Loading {} plugins", plugins.len());
        
        for plugin_name in &plugins {
            if let Err(e) = self.load_single_plugin(plugin_name, config, Arc::clone(&hyprland_client)).await {
                error!("❌ Failed to load plugin '{}': {}", plugin_name, e);
            }
        }
        
        info!("✅ Loaded {} plugins successfully", self.plugins.len());
        Ok(())
    }
    
    async fn load_single_plugin(&mut self, plugin_name: &str, config: &Config, hyprland_client: Arc<HyprlandClient>) -> Result<()> {
        info!("📦 Loading plugin: {}", plugin_name);
        
        let mut plugin: PluginBox = match plugin_name {
            "scratchpads" => {
                let scratchpads_plugin = ScratchpadsPlugin::new();
                scratchpads_plugin.set_hyprland_client(Arc::clone(&hyprland_client)).await;
                Box::new(scratchpads_plugin)
            }
            "expose" => {
                Box::new(ExposePlugin::new())
            }
            "workspaces_follow_focus" => {
                Box::new(WorkspacesFollowFocusPlugin::new())
            }
            "magnify" => {
                Box::new(MagnifyPlugin::new())
            }
            // Add more plugins here as they're implemented
            _ => {
                warn!("⚠️  Unknown plugin: {}", plugin_name);
                return Ok(());
            }
        };
        
        // Get plugin-specific config
        let plugin_config = config.plugins.get(plugin_name)
            .cloned()
            .unwrap_or(toml::Value::Table(toml::map::Map::new()));
        
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
            
            let combined = toml::Value::Table(combined_config);
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
    
    pub async fn handle_command(&mut self, plugin_name: &str, command: &str, args: &[&str]) -> Result<String> {
        if let Some(plugin) = self.plugins.get_mut(plugin_name) {
            plugin.handle_command(command, args).await
        } else {
            Err(anyhow::anyhow!("Plugin '{}' not found", plugin_name))
        }
    }
    
    pub fn get_plugin_count(&self) -> usize {
        self.plugins.len()
    }
}
