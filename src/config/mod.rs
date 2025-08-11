use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::fs;
use tracing::{info, debug};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    pub pyprland: PyprlandConfig,
    
    #[serde(flatten)]
    pub plugins: HashMap<String, toml::Value>,
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
        
        let content = fs::read_to_string(expanded_path.as_ref()).await
            .map_err(|e| anyhow::anyhow!("Failed to read config file '{}': {}", expanded_path, e))?;
        
        let config: Config = toml::from_str(&content)
            .map_err(|e| anyhow::anyhow!("Failed to parse config: {}", e))?;
        
        debug!("📋 Config loaded: {} plugins", config.pyprland.plugins.len());
        Ok(config)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            pyprland: PyprlandConfig {
                plugins: vec!["scratchpads".to_string()],
                variables: HashMap::new(),
            },
            plugins: HashMap::new(),
        }
    }
}
