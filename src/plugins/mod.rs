use anyhow::Result;
use async_trait::async_trait;

use crate::ipc::HyprlandEvent;

pub mod scratchpads;

#[async_trait]
pub trait Plugin: Send + Sync {
    /// Plugin name
    fn name(&self) -> &str;
    
    /// Initialize plugin with configuration
    async fn init(&mut self, config: &toml::Value) -> Result<()>;
    
    /// Handle Hyprland events
    async fn handle_event(&mut self, event: &HyprlandEvent) -> Result<()>;
    
    /// Handle commands from client
    async fn handle_command(&mut self, command: &str, args: &[&str]) -> Result<String>;
}

pub type PluginBox = Box<dyn Plugin>;
