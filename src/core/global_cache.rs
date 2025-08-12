use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use crate::ipc::MonitorInfo;
use crate::plugins::workspaces_follow_focus::{WorkspaceInfo, MonitorInfoRef, WorkspaceInfoRef, MonitorCache, WorkspaceCache};

/// Global state cache shared across all plugins for memory optimization
/// This implements the Arc-based shared state pattern to reduce memory usage
#[derive(Debug)]
pub struct GlobalStateCache {
    /// Cached monitor information shared across all plugins
    monitors: MonitorCache,
    
    /// Cached workspace information shared across all plugins  
    workspaces: WorkspaceCache,
    
    /// Last time the cache was updated
    last_update: Arc<RwLock<Instant>>,
    
    /// Configuration cache shared across plugins
    configs: Arc<RwLock<HashMap<String, Arc<toml::Value>>>>,
    
    /// Variables shared across plugins
    variables: Arc<RwLock<HashMap<String, String>>>,
    
    /// Cache validity duration (default: 2 seconds)
    cache_duration: std::time::Duration,
}

impl GlobalStateCache {
    pub fn new() -> Self {
        Self {
            monitors: Arc::new(RwLock::new(HashMap::new())),
            workspaces: Arc::new(RwLock::new(HashMap::new())),
            last_update: Arc::new(RwLock::new(Instant::now())),
            configs: Arc::new(RwLock::new(HashMap::new())),
            variables: Arc::new(RwLock::new(HashMap::new())),
            cache_duration: std::time::Duration::from_secs(2),
        }
    }

    /// Get monitor info with Arc sharing (no data duplication)
    pub async fn get_monitor(&self, name: &str) -> Option<MonitorInfoRef> {
        let monitors = self.monitors.read().await;
        monitors.get(name).cloned() // Only clones the Arc, not the data
    }

    /// Get workspace info with Arc sharing
    pub async fn get_workspace(&self, id: i32) -> Option<WorkspaceInfoRef> {
        let workspaces = self.workspaces.read().await;
        workspaces.get(&id).cloned() // Only clones the Arc, not the data
    }

    /// Update monitor information efficiently
    pub async fn update_monitors(&self, new_monitors: Vec<crate::ipc::MonitorInfo>) -> Result<()> {
        let mut monitors = self.monitors.write().await;
        
        // Clear old monitors
        monitors.clear();
        
        // Convert and add new monitors as Arc<RwLock<T>>
        for monitor in new_monitors {
            let _workspace_info = WorkspaceInfo {
                id: 1, // Default workspace - would be populated from Hyprland data
                name: "workspace_1".to_string(),
                monitor: monitor.name.clone(),
                windows: 0, // This would be populated from Hyprland data
                last_window_addr: String::new(),
            };

            let monitor_info = crate::plugins::workspaces_follow_focus::MonitorInfo {
                id: 1, // Default id - would be populated from Hyprland data
                name: monitor.name.clone(),
                focused: monitor.is_focused,
                active_workspace: 1, // Default active workspace
                width: monitor.width as u16,
                height: monitor.height as u16,
                x: monitor.x,
                y: monitor.y,
            };

            let monitor_ref = Arc::new(RwLock::new(monitor_info));
            monitors.insert(monitor.name, monitor_ref);
        }
        
        // Update timestamp
        {
            let mut last_update = self.last_update.write().await;
            *last_update = Instant::now();
        }
        
        Ok(())
    }

    /// Check if cache is still valid
    pub async fn is_cache_valid(&self) -> bool {
        let last_update = self.last_update.read().await;
        last_update.elapsed() < self.cache_duration
    }

    /// Get monitor cache reference for sharing with plugins
    pub fn get_monitor_cache(&self) -> MonitorCache {
        Arc::clone(&self.monitors)
    }

    /// Get workspace cache reference for sharing with plugins
    pub fn get_workspace_cache(&self) -> WorkspaceCache {
        Arc::clone(&self.workspaces)
    }

    /// Store configuration with Arc sharing
    pub async fn store_config(&self, plugin_name: String, config: Arc<toml::Value>) {
        let mut configs = self.configs.write().await;
        configs.insert(plugin_name, config);
    }

    /// Get shared configuration
    pub async fn get_config(&self, plugin_name: &str) -> Option<Arc<toml::Value>> {
        let configs = self.configs.read().await;
        configs.get(plugin_name).cloned()
    }

    /// Store variables with Arc sharing
    pub async fn store_variables(&self, variables: HashMap<String, String>) {
        let mut vars = self.variables.write().await;
        *vars = variables;
    }

    /// Get shared variables reference
    pub fn get_variables(&self) -> Arc<RwLock<HashMap<String, String>>> {
        Arc::clone(&self.variables)
    }

    /// Get memory usage statistics
    pub async fn get_memory_stats(&self) -> MemoryStats {
        let monitors = self.monitors.read().await;
        let workspaces = self.workspaces.read().await;
        let configs = self.configs.read().await;
        let vars = self.variables.read().await;
        
        MemoryStats {
            monitor_count: monitors.len(),
            workspace_count: workspaces.len(),
            config_count: configs.len(),
            variable_count: vars.len(),
            total_arc_refs: monitors.len() + workspaces.len() + configs.len() + 1, // +1 for variables
        }
    }
}

#[derive(Debug)]
pub struct MemoryStats {
    pub monitor_count: usize,
    pub workspace_count: usize,
    pub config_count: usize,
    pub variable_count: usize,
    pub total_arc_refs: usize,
}

impl Default for GlobalStateCache {
    fn default() -> Self {
        Self::new()
    }
}