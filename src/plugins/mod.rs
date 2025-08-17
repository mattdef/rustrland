use anyhow::Result;
use async_trait::async_trait;

use crate::ipc::HyprlandEvent;

pub mod expose;
pub mod lost_windows;
pub mod magnify;
pub mod monitors;
pub mod scratchpads;
pub mod shift_monitors;
pub mod system_notifier;
pub mod toggle_special;
pub mod wallpapers;
pub mod workspaces_follow_focus;

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

    /// Cleanup plugin resources (background tasks, timers, etc.)
    async fn cleanup(&mut self) -> Result<()> {
        // Default implementation does nothing
        Ok(())
    }
}

pub type PluginBox = Box<dyn Plugin>;
