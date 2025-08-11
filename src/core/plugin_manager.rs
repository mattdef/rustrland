use anyhow::Result;
use std::collections::HashMap;
use tracing::{info, warn, error};

use crate::plugins::{Plugin, PluginBox};
use crate::plugins::scratchpads::ScratchpadsPlugin;
use crate::config::Config;
use crate::ipc::HyprlandEvent;

pub struct PluginManager {
    plugins: HashMap<String, PluginBox>,
}

impl PluginManager {
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
        }
    }
    
    pub async fn load_plugins(&mut self, config: &Config) -> Result<()> {
        info!("🔌 Loading {} plugins", config.pyprland.plugins.len());
        
        for plugin_name in &config.pyprland.plugins {
            if let Err(e) = self.load_single_plugin(plugin_name, config).await {
                error!("❌ Failed to load plugin '{}': {}", plugin_name, e);
            }
        }
        
        info!("✅ Loaded {} plugins successfully", self.plugins.len());
        Ok(())
    }
    
    async fn load_single_plugin(&mut self, plugin_name: &str, config: &Config) -> Result<()> {
        info!("📦 Loading plugin: {}", plugin_name);
        
        let mut plugin: PluginBox = match plugin_name {
            "scratchpads" => Box::new(ScratchpadsPlugin::new()),
            // "magnify" => Box::new(MagnifyPlugin::new()),
            // "expose" => Box::new(ExposePlugin::new()),
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
        
        // Initialize plugin
        plugin.init(&plugin_config).await?;
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
}
