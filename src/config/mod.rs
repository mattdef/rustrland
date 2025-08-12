use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::fs;
use tracing::{debug, info};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    #[serde(default)]
    pub pyprland: Option<PyprlandConfig>,

    #[serde(default)]
    pub rustrland: Option<RustrlandConfig>,

    #[serde(flatten)]
    pub plugins: HashMap<String, toml::Value>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RustrlandConfig {
    pub plugins: Vec<String>,

    #[serde(default)]
    pub variables: HashMap<String, String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PyprlandConfig {
    pub plugins: Vec<String>,

    #[serde(default)]
    pub variables: HashMap<String, String>,
}

impl Config {
    pub async fn load(path: &str) -> Result<Self> {
        let expanded_path = shellexpand::tilde(path);
        info!("📄 Reading config from: {}", expanded_path);

        let content = fs::read_to_string(expanded_path.as_ref())
            .await
            .map_err(|e| {
                anyhow::anyhow!("Failed to read config file '{}': {}", expanded_path, e)
            })?;

        let config: Config = toml::from_str(&content)
            .map_err(|e| anyhow::anyhow!("Failed to parse config: {}", e))?;

        let plugin_count = config.get_plugins().len();
        debug!("📋 Config loaded: {} plugins", plugin_count);

        // Log which configuration sections were found
        match (&config.pyprland, &config.rustrland) {
            (Some(_), Some(_)) => {
                info!("📋 Found both [pyprland] and [rustrland] configurations, merging them")
            }
            (Some(_), None) => info!("📋 Found [pyprland] configuration"),
            (None, Some(_)) => info!("📋 Found [rustrland] configuration"),
            (None, None) => info!("📋 No main configuration section found, using defaults"),
        }

        Ok(config)
    }

    /// Get merged list of plugins from both pyprland and rustrland sections
    pub fn get_plugins(&self) -> Vec<String> {
        let mut plugins = Vec::new();

        // Add pyprland plugins
        if let Some(ref pyprland) = self.pyprland {
            plugins.extend(pyprland.plugins.clone());
        }

        // Add rustrland plugins (avoiding duplicates)
        if let Some(ref rustrland) = self.rustrland {
            for plugin in &rustrland.plugins {
                if !plugins.contains(plugin) {
                    plugins.push(plugin.clone());
                }
            }
        }

        // If no plugins defined anywhere, use default
        if plugins.is_empty() {
            plugins.push("scratchpads".to_string());
        }

        plugins
    }

    /// Get merged variables from both pyprland and rustrland sections
    /// Rustrland variables take precedence over pyprland variables
    pub fn get_variables(&self) -> HashMap<String, String> {
        let mut variables = HashMap::new();

        // Add pyprland variables first
        if let Some(ref pyprland) = self.pyprland {
            variables.extend(pyprland.variables.clone());
        }

        // Add rustrland variables (they override pyprland ones)
        if let Some(ref rustrland) = self.rustrland {
            variables.extend(rustrland.variables.clone());
        }

        variables
    }

    /// Check if a configuration uses the new rustrland format
    pub fn uses_rustrland_config(&self) -> bool {
        self.rustrland.is_some()
    }

    /// Check if a configuration uses the legacy pyprland format
    pub fn uses_pyprland_config(&self) -> bool {
        self.pyprland.is_some()
    }

    /// Create config from TOML value (for hot reload)
    pub fn from_toml_value(value: toml::Value) -> Result<Self> {
        let config: Config = value.try_into()?;
        Ok(config)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            pyprland: Some(PyprlandConfig {
                plugins: vec!["scratchpads".to_string()],
                variables: HashMap::new(),
            }),
            rustrland: None,
            plugins: HashMap::new(),
        }
    }
}

// Implementation of ConfigExt trait for Config
impl super::core::hot_reload::ConfigExt for Config {
    fn get_plugin_names(&self) -> Vec<String> {
        let mut plugin_names = Vec::new();

        // Get plugins from pyprland config
        if let Some(pyprland) = &self.pyprland {
            plugin_names.extend_from_slice(&pyprland.plugins);
        }

        // Get plugins from rustrland config (takes precedence)
        if let Some(rustrland) = &self.rustrland {
            for plugin in &rustrland.plugins {
                if !plugin_names.contains(plugin) {
                    plugin_names.push(plugin.clone());
                }
            }
        }

        plugin_names
    }

    fn get_plugin_config(&self, plugin_name: &str) -> Result<toml::Value> {
        if let Some(config) = self.plugins.get(plugin_name) {
            Ok(config.clone())
        } else {
            Ok(toml::Value::Table(toml::Table::new()))
        }
    }

    fn from_toml_value(value: toml::Value) -> Result<Self> {
        let config: Config = value.try_into()?;
        Ok(config)
    }
}
