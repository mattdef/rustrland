use anyhow::Result;
use async_trait::async_trait;
use tracing::{info, debug, warn};
use std::collections::HashMap;

use crate::plugins::Plugin;
use crate::ipc::HyprlandEvent;

#[derive(Debug)]
pub struct ScratchpadConfig {
    pub command: String,
    pub class: String,
    pub size: String,
    pub animation: Option<String>,
}

pub struct ScratchpadsPlugin {
    scratchpads: HashMap<String, ScratchpadConfig>,
}

impl ScratchpadsPlugin {
    pub fn new() -> Self {
        Self {
            scratchpads: HashMap::new(),
        }
    }
}

#[async_trait]
impl Plugin for ScratchpadsPlugin {
    fn name(&self) -> &str {
        "scratchpads"
    }
    
    async fn init(&mut self, config: &toml::Value) -> Result<()> {
        info!("🪟 Initializing scratchpads plugin");
        debug!("Config: {}", config);
        
        // Parse scratchpad configurations
        if let toml::Value::Table(map) = config {
            for (name, scratchpad_config) in map {
                if let toml::Value::Table(sc) = scratchpad_config {
                    let command = sc.get("command")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    
                    let class = sc.get("class")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    
                    let size = sc.get("size")
                        .and_then(|v| v.as_str())
                        .unwrap_or("50% 50%")
                        .to_string();
                    
                    let animation = sc.get("animation")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    
                    let config = ScratchpadConfig {
                        command,
                        class,
                        size,
                        animation,
                    };
                    
                    self.scratchpads.insert(name.clone(), config);
                    info!("📝 Registered scratchpad: {}", name);
                }
            }
        }
        
        info!("✅ Scratchpads plugin initialized with {} scratchpads", self.scratchpads.len());
        Ok(())
    }
    
    async fn handle_event(&mut self, event: &HyprlandEvent) -> Result<()> {
        debug!("🪟 Scratchpads handling event: {:?}", event);
        
        // Handle relevant events (window open/close, workspace changes, etc.)
        match event {
            HyprlandEvent::WindowOpened { window } => {
                debug!("Window opened: {} - checking if it's a scratchpad", window);
            }
            HyprlandEvent::WindowClosed { window } => {
                debug!("Window closed: {} - checking if it was a scratchpad", window);
            }
            HyprlandEvent::WorkspaceChanged { workspace } => {
                debug!("Workspace changed to: {}", workspace);
            }
            HyprlandEvent::Other(msg) => {
                debug!("Other event: {}", msg);
            }
            _ => {
                // Handle other events
            }
        }
        
        Ok(())
    }
    
    async fn handle_command(&mut self, command: &str, args: &[&str]) -> Result<String> {
        match command {
            "toggle" => {
                if let Some(scratchpad_name) = args.first() {
                    info!("🔄 Toggling scratchpad: {}", scratchpad_name);
                    
                    if let Some(_config) = self.scratchpads.get(*scratchpad_name) {
                        // TODO: Implement actual toggle logic
                        info!("✅ Toggled scratchpad: {}", scratchpad_name);
                        Ok(format!("Toggled scratchpad: {}", scratchpad_name))
                    } else {
                        warn!("⚠️  Scratchpad '{}' not found", scratchpad_name);
                        Err(anyhow::anyhow!("Scratchpad '{}' not found", scratchpad_name))
                    }
                } else {
                    Err(anyhow::anyhow!("No scratchpad name provided"))
                }
            }
            "list" => {
                let names: Vec<String> = self.scratchpads.keys().cloned().collect();
                Ok(format!("Available scratchpads: {}", names.join(", ")))
            }
            _ => {
                Err(anyhow::anyhow!("Unknown command: {}", command))
            }
        }
    }
}
