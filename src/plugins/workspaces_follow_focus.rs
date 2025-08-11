use anyhow::Result;
use async_trait::async_trait;
use tracing::{info, debug, warn, error};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use serde::{Deserialize, Serialize};

use crate::plugins::Plugin;
use crate::ipc::{HyprlandEvent, HyprlandClient};
use hyprland::data::{Monitors, Workspaces, Clients};
use hyprland::shared::{HyprData, HyprDataVec};
use hyprland::dispatch::{Dispatch, DispatchType, WorkspaceIdentifierWithSpecial, WorkspaceIdentifier, MonitorIdentifier};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WorkspacesFollowFocusConfig {
    /// Auto-switch workspace when focusing a window on different monitor (default: true)
    #[serde(default = "default_true")]
    pub follow_window_focus: bool,
    
    /// Auto-move workspaces to focused monitor (default: true)  
    #[serde(default = "default_true")]
    pub follow_workspace_request: bool,
    
    /// Allow switching to workspaces on other monitors (default: true)
    #[serde(default = "default_true")]
    pub allow_cross_monitor_switch: bool,
    
    /// Log workspace switching events (default: false)
    #[serde(default)]
    pub debug_logging: bool,
}

fn default_true() -> bool { true }

impl Default for WorkspacesFollowFocusConfig {
    fn default() -> Self {
        Self {
            follow_window_focus: true,
            follow_workspace_request: true,
            allow_cross_monitor_switch: true,
            debug_logging: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MonitorInfo {
    pub id: i128,
    pub name: String,
    pub focused: bool,
    pub active_workspace: i32,
    pub width: u16,
    pub height: u16,
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone)]
pub struct WorkspaceInfo {
    pub id: i32,
    pub name: String,
    pub monitor: String,
    pub windows: u16,
    pub last_window_addr: String,
}

pub struct WorkspacesFollowFocusPlugin {
    config: WorkspacesFollowFocusConfig,
    monitors: HashMap<String, MonitorInfo>,
    workspaces: HashMap<i32, WorkspaceInfo>,
    focused_monitor: Option<String>,
    hyprland_client: Arc<Mutex<Option<Arc<HyprlandClient>>>>,
}

impl WorkspacesFollowFocusPlugin {
    pub fn new() -> Self {
        Self {
            config: WorkspacesFollowFocusConfig::default(),
            monitors: HashMap::new(),
            workspaces: HashMap::new(),
            focused_monitor: None,
            hyprland_client: Arc::new(Mutex::new(None)),
        }
    }
    
    /// Update monitor information from Hyprland
    async fn update_monitors(&mut self) -> Result<()> {
        let monitors = tokio::task::spawn_blocking(|| Monitors::get()).await??;
        let monitor_vec = monitors.to_vec();
        
        self.monitors.clear();
        
        for monitor in monitor_vec {
            let monitor_info = MonitorInfo {
                id: monitor.id,
                name: monitor.name.clone(),
                focused: monitor.focused,
                active_workspace: monitor.active_workspace.id,
                width: monitor.width,
                height: monitor.height,
                x: monitor.x,
                y: monitor.y,
            };
            
            if monitor.focused {
                self.focused_monitor = Some(monitor.name.clone());
            }
            
            self.monitors.insert(monitor.name, monitor_info);
        }
        
        if self.config.debug_logging {
            debug!("🖥️  Updated {} monitors, focused: {:?}", 
                self.monitors.len(), 
                self.focused_monitor
            );
        }
        
        Ok(())
    }
    
    /// Update workspace information from Hyprland  
    async fn update_workspaces(&mut self) -> Result<()> {
        let workspaces = tokio::task::spawn_blocking(|| Workspaces::get()).await??;
        let workspace_vec = workspaces.to_vec();
        
        self.workspaces.clear();
        
        for workspace in workspace_vec {
            let workspace_info = WorkspaceInfo {
                id: workspace.id,
                name: workspace.name.clone(),
                monitor: workspace.monitor.clone(),
                windows: workspace.windows,
                last_window_addr: workspace.last_window.to_string(),
            };
            
            self.workspaces.insert(workspace.id, workspace_info);
        }
        
        if self.config.debug_logging {
            debug!("🏢 Updated {} workspaces", self.workspaces.len());
        }
        
        Ok(())
    }
    
    /// Get the monitor that a workspace is currently on
    fn get_workspace_monitor(&self, workspace_id: i32) -> Option<String> {
        self.workspaces.get(&workspace_id).map(|ws| ws.monitor.clone())
    }
    
    /// Get the currently focused monitor
    fn get_focused_monitor(&self) -> Option<String> {
        self.focused_monitor.clone()
    }
    
    /// Switch to a workspace, potentially moving it to the focused monitor
    async fn switch_workspace(&mut self, workspace_id: i32) -> Result<String> {
        // Update current state
        self.update_monitors().await?;
        self.update_workspaces().await?;
        
        let focused_monitor = match self.get_focused_monitor() {
            Some(monitor) => monitor,
            None => return Err(anyhow::anyhow!("No focused monitor found")),
        };
        
        let workspace_monitor = self.get_workspace_monitor(workspace_id);
        
        if self.config.debug_logging {
            debug!("🔄 Switching to workspace {} (currently on {:?}), focused monitor: {}", 
                workspace_id, workspace_monitor, focused_monitor);
        }
        
        // If workspace is on a different monitor and we allow cross-monitor switching
        if let Some(ws_monitor) = workspace_monitor {
            if ws_monitor != focused_monitor && self.config.allow_cross_monitor_switch {
                info!("📱 Moving workspace {} from monitor {} to focused monitor {}", 
                    workspace_id, ws_monitor, focused_monitor);
                
                // Move workspace to focused monitor
                let workspace_identifier = WorkspaceIdentifier::Id(workspace_id);
                let monitor_name = focused_monitor.clone();
                tokio::task::spawn_blocking(move || {
                    let monitor_identifier = MonitorIdentifier::Name(&monitor_name);
                    Dispatch::call(DispatchType::MoveWorkspaceToMonitor(
                        workspace_identifier, 
                        monitor_identifier
                    ))
                }).await??;
            }
        }
        
        // Switch to the workspace
        let workspace_identifier = WorkspaceIdentifierWithSpecial::Id(workspace_id);
        tokio::task::spawn_blocking(move || {
            Dispatch::call(DispatchType::Workspace(workspace_identifier))
        }).await??;
        
        Ok(format!("Switched to workspace {} on monitor {}", workspace_id, focused_monitor))
    }
    
    /// Handle workspace change with relative offset (+1, -1, etc.)
    async fn change_workspace(&mut self, offset: i32) -> Result<String> {
        // Update current state
        self.update_monitors().await?;
        self.update_workspaces().await?;
        
        let focused_monitor = match self.get_focused_monitor() {
            Some(monitor) => monitor,
            None => return Err(anyhow::anyhow!("No focused monitor found")),
        };
        
        // Get current workspace on focused monitor
        let current_workspace = self.monitors.get(&focused_monitor)
            .map(|m| m.active_workspace)
            .unwrap_or(1);
        
        let target_workspace = current_workspace + offset;
        
        // Ensure target workspace exists (create if needed in range 1-10)
        if target_workspace < 1 || target_workspace > 10 {
            return Err(anyhow::anyhow!("Workspace {} out of range (1-10)", target_workspace));
        }
        
        if self.config.debug_logging {
            debug!("🔄 Changing workspace by {} (from {} to {}) on monitor {}", 
                offset, current_workspace, target_workspace, focused_monitor);
        }
        
        self.switch_workspace(target_workspace).await
    }
    
    /// List workspaces with their monitor assignments
    async fn list_workspaces(&mut self) -> Result<String> {
        self.update_monitors().await?;
        self.update_workspaces().await?;
        
        let mut output = String::from("🏢 Workspaces:\n");
        
        let mut workspace_list: Vec<_> = self.workspaces.values().collect();
        workspace_list.sort_by_key(|ws| ws.id);
        
        for workspace in workspace_list {
            let is_active = self.monitors.values()
                .any(|m| m.active_workspace == workspace.id);
            
            let active_marker = if is_active { "🎯" } else { "  " };
            
            output.push_str(&format!(
                "{} Workspace {}: {} windows on monitor {} ({})\n",
                active_marker,
                workspace.id,
                workspace.windows,
                workspace.monitor,
                workspace.name
            ));
        }
        
        // Add monitor info
        output.push_str("\n🖥️  Monitors:\n");
        let mut monitor_list: Vec<_> = self.monitors.values().collect();
        monitor_list.sort_by_key(|m| &m.name);
        
        for monitor in monitor_list {
            let focused_marker = if monitor.focused { "🎯" } else { "  " };
            output.push_str(&format!(
                "{} {}: {}x{} @ ({},{}) - Workspace {}\n",
                focused_marker,
                monitor.name,
                monitor.width, monitor.height,
                monitor.x, monitor.y,
                monitor.active_workspace
            ));
        }
        
        Ok(output)
    }
    
    /// Get status of workspaces_follow_focus plugin
    async fn get_status(&mut self) -> Result<String> {
        self.update_monitors().await?;
        self.update_workspaces().await?;
        
        let monitor_count = self.monitors.len();
        let workspace_count = self.workspaces.len();
        let focused_monitor = self.get_focused_monitor().unwrap_or("None".to_string());
        
        Ok(format!(
            "WorkspacesFollowFocus: {} monitors, {} workspaces\nFocused: {}\nConfig: follow_window_focus={}, cross_monitor_switch={}",
            monitor_count,
            workspace_count,
            focused_monitor,
            self.config.follow_window_focus,
            self.config.allow_cross_monitor_switch
        ))
    }
}

#[async_trait]
impl Plugin for WorkspacesFollowFocusPlugin {
    fn name(&self) -> &str {
        "workspaces_follow_focus"
    }
    
    async fn init(&mut self, config: &toml::Value) -> Result<()> {
        info!("🏢 Initializing workspaces_follow_focus plugin");
        
        if let Some(plugin_config) = config.get("workspaces_follow_focus") {
            match plugin_config.clone().try_into() {
                Ok(config) => self.config = config,
                Err(e) => return Err(anyhow::anyhow!("Invalid workspaces_follow_focus configuration: {}", e)),
            }
        }
        
        debug!("WorkspacesFollowFocus config: {:?}", self.config);
        
        // Initialize monitor and workspace state
        self.update_monitors().await?;
        self.update_workspaces().await?;
        
        info!("✅ WorkspacesFollowFocus plugin initialized with {} monitors, {} workspaces", 
            self.monitors.len(), self.workspaces.len());
        
        Ok(())
    }
    
    async fn handle_event(&mut self, event: &HyprlandEvent) -> Result<()> {
        match event {
            HyprlandEvent::WorkspaceChanged { workspace } => {
                if self.config.debug_logging {
                    debug!("🔄 Workspace changed to: {}", workspace);
                }
                
                // Update our state when workspace changes
                self.update_monitors().await?;
                self.update_workspaces().await?;
            }
            
            HyprlandEvent::WindowOpened { window } => {
                if self.config.follow_window_focus {
                    debug!("🪟 New window opened: {}", window);
                    // Could implement logic to follow window focus across monitors
                    self.update_workspaces().await?;
                }
            }
            
            HyprlandEvent::WindowClosed { window } => {
                if self.config.debug_logging {
                    debug!("🚪 Window closed: {}", window);
                }
                self.update_workspaces().await?;
            }
            
            HyprlandEvent::WindowMoved { window } => {
                if self.config.follow_window_focus {
                    debug!("📱 Window moved: {}", window);
                    self.update_monitors().await?;
                    self.update_workspaces().await?;
                }
            }
            
            _ => {}
        }
        
        Ok(())
    }
    
    async fn handle_command(&mut self, command: &str, args: &[&str]) -> Result<String> {
        debug!("🏢 WorkspacesFollowFocus command: {} {:?}", command, args);
        
        match command {
            "switch" => {
                if let Some(workspace_str) = args.first() {
                    let workspace_id: i32 = workspace_str.parse()
                        .map_err(|_| anyhow::anyhow!("Invalid workspace ID: {}", workspace_str))?;
                    self.switch_workspace(workspace_id).await
                } else {
                    Err(anyhow::anyhow!("Switch command requires workspace ID"))
                }
            }
            
            "change" => {
                if let Some(offset_str) = args.first() {
                    let offset: i32 = offset_str.parse()
                        .map_err(|_| anyhow::anyhow!("Invalid offset: {}", offset_str))?;
                    self.change_workspace(offset).await
                } else {
                    Err(anyhow::anyhow!("Change command requires offset (+1, -1, etc.)"))
                }
            }
            
            "list" => self.list_workspaces().await,
            "status" => self.get_status().await,
            
            _ => Ok(format!("Unknown workspaces_follow_focus command: {}", command)),
        }
    }
}