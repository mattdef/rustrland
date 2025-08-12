use anyhow::Result;
use tracing::{debug, trace};

use crate::core::plugin_manager::PluginManager;
use crate::ipc::HyprlandEvent;

pub struct EventHandler {
    // Could store event filtering, rate limiting, etc.
}

impl EventHandler {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn handle_event(
        &self,
        event: &HyprlandEvent,
        plugin_manager: &mut PluginManager,
    ) -> Result<()> {
        trace!("📨 Handling event: {:?}", event);

        // Filter or transform events here if needed

        // Forward to all plugins
        plugin_manager.handle_event(event).await?;

        debug!("✅ Event handled successfully");
        Ok(())
    }
}
