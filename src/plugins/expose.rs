use anyhow::Result;
use async_trait::async_trait;
use tracing::{info, debug, warn, error};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use serde::{Deserialize, Serialize};

use crate::plugins::Plugin;
use crate::ipc::{HyprlandEvent, HyprlandClient};
use hyprland::data::{Client, Clients, Workspaces};
use hyprland::shared::{HyprData, HyprDataVec};
use hyprland::dispatch::{Dispatch, DispatchType};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExposeConfig {
    /// Grid columns (default: auto-calculate)
    #[serde(default)]
    pub columns: Option<u32>,
    
    /// Grid rows (default: auto-calculate)
    #[serde(default)]
    pub rows: Option<u32>,
    
    /// Padding between windows (default: 20px)
    #[serde(default = "default_padding")]
    pub padding: u32,
    
    /// Include floating windows (default: true)
    #[serde(default = "default_true")]
    pub include_floating: bool,
    
    /// Include minimized windows (default: false)
    #[serde(default)]
    pub include_minimized: bool,
    
    /// Only show windows from current workspace (default: false)
    #[serde(default)]
    pub current_workspace_only: bool,
    
    /// Scale factor for window previews (default: 0.2)
    #[serde(default = "default_scale")]
    pub scale: f32,
    
    /// Show window titles (default: true)
    #[serde(default = "default_true")]
    pub show_titles: bool,
    
    /// Background color during expose (default: "#000000AA")
    #[serde(default = "default_background")]
    pub background_color: String,
    
    /// Highlight color for focused window (default: "#FF6600")
    #[serde(default = "default_highlight")]
    pub highlight_color: String,
}

fn default_padding() -> u32 { 20 }
fn default_true() -> bool { true }
fn default_scale() -> f32 { 0.2 }
fn default_background() -> String { "#000000AA".to_string() }
fn default_highlight() -> String { "#FF6600".to_string() }

impl Default for ExposeConfig {
    fn default() -> Self {
        Self {
            columns: None,
            rows: None,
            padding: default_padding(),
            include_floating: default_true(),
            include_minimized: false,
            current_workspace_only: false,
            scale: default_scale(),
            show_titles: default_true(),
            background_color: default_background(),
            highlight_color: default_highlight(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct WindowTile {
    pub client: Client,
    pub original_pos: (i32, i32),
    pub original_size: (i32, i32),
    pub grid_pos: (i32, i32),
    pub grid_size: (i32, i32),
    pub tile_index: usize,
}

#[derive(Debug, Clone)]
pub struct ExposeState {
    pub is_active: bool,
    pub tiles: Vec<WindowTile>,
    pub selected_index: usize,
    pub original_workspace: i32,
}

impl Default for ExposeState {
    fn default() -> Self {
        Self {
            is_active: false,
            tiles: Vec::new(),
            selected_index: 0,
            original_workspace: 1,
        }
    }
}

pub struct ExposePlugin {
    config: ExposeConfig,
    state: ExposeState,
    hyprland_client: Arc<Mutex<Option<Arc<HyprlandClient>>>>,
}

impl ExposePlugin {
    pub fn new() -> Self {
        Self {
            config: ExposeConfig::default(),
            state: ExposeState::default(),
            hyprland_client: Arc::new(Mutex::new(None)),
        }
    }
    
    /// Calculate optimal grid layout for given number of windows
    fn calculate_grid_layout(&self, window_count: usize) -> (u32, u32) {
        if let (Some(cols), Some(rows)) = (self.config.columns, self.config.rows) {
            return (cols, rows);
        }
        
        if window_count == 0 {
            return (1, 1);
        }
        
        // Calculate square-ish grid
        let sqrt_count = (window_count as f64).sqrt().ceil() as u32;
        let columns = sqrt_count;
        let rows = (window_count as f64 / columns as f64).ceil() as u32;
        
        (columns, rows)
    }
    
    /// Get all windows that should be included in expose
    async fn get_expose_windows(&self) -> Result<Vec<Client>> {
        let clients = tokio::task::spawn_blocking(|| Clients::get()).await??;
        let client_vec = clients.to_vec();
        
        debug!("📱 Found {} total windows", client_vec.len());
        
        let mut filtered_windows = Vec::new();
        
        // Get current workspace if filtering by it
        let current_workspace = if self.config.current_workspace_only {
            // For now, we'll assume workspace 1. In a real implementation,
            // we'd get the current workspace from Hyprland
            Some(1)
        } else {
            None
        };
        
        for client in client_vec {
            // Skip if not including floating windows
            if client.floating && !self.config.include_floating {
                debug!("Skipping floating window: {}", client.title);
                continue;
            }
            
            // Skip if filtering by current workspace
            if let Some(workspace) = current_workspace {
                if client.workspace.id != workspace {
                    debug!("Skipping window from different workspace: {}", client.title);
                    continue;
                }
            }
            
            // Skip minimized windows if not included
            if !self.config.include_minimized {
                // For now, we'll assume all windows are visible
                // In a real implementation, we'd check the window state
            }
            
            // Skip special workspaces (like scratchpads)
            if client.workspace.name.starts_with("special:") {
                debug!("Skipping special workspace window: {}", client.title);
                continue;
            }
            
            debug!("Including window: {} [{}]", client.title, client.class);
            filtered_windows.push(client);
        }
        
        Ok(filtered_windows)
    }
    
    /// Calculate positions for window tiles in the grid
    fn calculate_tile_positions(&self, windows: Vec<Client>) -> Vec<WindowTile> {
        let window_count = windows.len();
        if window_count == 0 {
            return Vec::new();
        }
        
        let (columns, rows) = self.calculate_grid_layout(window_count);
        debug!("📐 Using grid layout: {}x{} for {} windows", columns, rows, window_count);
        
        // Get screen dimensions (hardcoded for now - should get from Hyprland)
        let screen_width = 1920;
        let screen_height = 1080;
        
        // Calculate tile dimensions
        let total_padding_x = self.config.padding * (columns + 1);
        let total_padding_y = self.config.padding * (rows + 1);
        
        let tile_width = (screen_width - total_padding_x) / columns;
        let tile_height = (screen_height - total_padding_y) / rows;
        
        let mut tiles = Vec::new();
        
        for (index, client) in windows.into_iter().enumerate() {
            let row = (index as u32) / columns;
            let col = (index as u32) % columns;
            
            let x = self.config.padding + col * (tile_width + self.config.padding);
            let y = self.config.padding + row * (tile_height + self.config.padding);
            
            let tile = WindowTile {
                original_pos: (client.at.0.into(), client.at.1.into()),
                original_size: (client.size.0.into(), client.size.1.into()),
                grid_pos: (x as i32, y as i32),
                grid_size: (tile_width as i32, tile_height as i32),
                tile_index: index,
                client,
            };
            
            tiles.push(tile);
        }
        
        tiles
    }
    
    /// Enter expose mode
    async fn enter_expose(&mut self) -> Result<String> {
        if self.state.is_active {
            return Ok("Expose already active".to_string());
        }
        
        info!("🎯 Entering expose mode");
        
        // Get current workspace
        let workspaces = tokio::task::spawn_blocking(|| Workspaces::get()).await??;
        let workspace_vec = workspaces.to_vec();
        
        // Find current workspace (simplified - get first non-empty workspace)
        let current_workspace = workspace_vec
            .iter()
            .find(|w| w.windows > 0)
            .map(|w| w.id)
            .unwrap_or(1);
        
        self.state.original_workspace = current_workspace;
        
        // Get windows to show
        let windows = self.get_expose_windows().await?;
        
        if windows.is_empty() {
            return Ok("No windows to show in expose".to_string());
        }
        
        // Calculate tile positions
        self.state.tiles = self.calculate_tile_positions(windows);
        self.state.selected_index = 0;
        self.state.is_active = true;
        
        // Here we would actually move and resize windows
        // For now, just log what we would do
        info!("📐 Would arrange {} windows in expose grid", self.state.tiles.len());
        for (i, tile) in self.state.tiles.iter().enumerate() {
            debug!("  Window {}: '{}' -> position ({}, {}) size {}x{}", 
                i, 
                tile.client.title.chars().take(20).collect::<String>(),
                tile.grid_pos.0, tile.grid_pos.1,
                tile.grid_size.0, tile.grid_size.1
            );
        }
        
        Ok(format!("Expose mode activated with {} windows", self.state.tiles.len()))
    }
    
    /// Exit expose mode
    async fn exit_expose(&mut self) -> Result<String> {
        if !self.state.is_active {
            return Ok("Expose not active".to_string());
        }
        
        info!("🚪 Exiting expose mode");
        
        // Here we would restore original window positions
        // For now, just log what we would do
        for tile in &self.state.tiles {
            debug!("  Restoring window '{}' -> position ({}, {}) size {}x{}", 
                tile.client.title.chars().take(20).collect::<String>(),
                tile.original_pos.0, tile.original_pos.1,
                tile.original_size.0, tile.original_size.1
            );
        }
        
        // Focus the selected window
        if let Some(selected_tile) = self.state.tiles.get(self.state.selected_index) {
            info!("🎯 Focusing selected window: {}", selected_tile.client.title);
            // Here we would actually focus the window
        }
        
        // Reset state
        self.state = ExposeState::default();
        
        Ok("Expose mode deactivated".to_string())
    }
    
    /// Navigate to next window in expose
    async fn next_window(&mut self) -> Result<String> {
        if !self.state.is_active || self.state.tiles.is_empty() {
            return Ok("Expose not active".to_string());
        }
        
        self.state.selected_index = (self.state.selected_index + 1) % self.state.tiles.len();
        
        let selected_tile = &self.state.tiles[self.state.selected_index];
        debug!("➡️ Selected window: {}", selected_tile.client.title);
        
        Ok(format!("Selected window {} of {}: {}", 
            self.state.selected_index + 1, 
            self.state.tiles.len(),
            selected_tile.client.title
        ))
    }
    
    /// Navigate to previous window in expose
    async fn prev_window(&mut self) -> Result<String> {
        if !self.state.is_active || self.state.tiles.is_empty() {
            return Ok("Expose not active".to_string());
        }
        
        self.state.selected_index = if self.state.selected_index == 0 {
            self.state.tiles.len() - 1
        } else {
            self.state.selected_index - 1
        };
        
        let selected_tile = &self.state.tiles[self.state.selected_index];
        debug!("⬅️ Selected window: {}", selected_tile.client.title);
        
        Ok(format!("Selected window {} of {}: {}", 
            self.state.selected_index + 1, 
            self.state.tiles.len(),
            selected_tile.client.title
        ))
    }
    
    /// Get current expose status
    async fn get_status(&self) -> Result<String> {
        if !self.state.is_active {
            return Ok("Expose: Inactive".to_string());
        }
        
        let selected_title = self.state.tiles
            .get(self.state.selected_index)
            .map(|t| t.client.title.as_str())
            .unwrap_or("None");
        
        Ok(format!(
            "Expose: Active | Windows: {} | Selected: {} ({})",
            self.state.tiles.len(),
            self.state.selected_index + 1,
            selected_title
        ))
    }
}

#[async_trait]
impl Plugin for ExposePlugin {
    fn name(&self) -> &str {
        "expose"
    }
    
    async fn init(&mut self, config: &toml::Value) -> Result<()> {
        info!("🎯 Initializing expose plugin");
        
        if let Some(expose_config) = config.get("expose") {
            match expose_config.clone().try_into() {
                Ok(config) => self.config = config,
                Err(e) => return Err(anyhow::anyhow!("Invalid expose configuration: {}", e)),
            }
        }
        
        debug!("Expose config: {:?}", self.config);
        
        Ok(())
    }
    
    async fn handle_event(&mut self, event: &HyprlandEvent) -> Result<()> {
        // Handle Hyprland events that might affect expose mode
        match event {
            HyprlandEvent::WindowClosed { .. } => {
                if self.state.is_active {
                    debug!("Window closed during expose - might need to refresh");
                    // In a full implementation, we'd refresh the expose grid
                }
            }
            HyprlandEvent::WorkspaceChanged { .. } => {
                if self.state.is_active {
                    debug!("Workspace changed during expose - exiting");
                    self.exit_expose().await?;
                }
            }
            _ => {}
        }
        
        Ok(())
    }
    
    async fn handle_command(&mut self, command: &str, args: &[&str]) -> Result<String> {
        debug!("🎯 Expose command: {} {:?}", command, args);
        
        match command {
            "toggle" | "show" => self.enter_expose().await,
            "hide" | "exit" => self.exit_expose().await,
            "next" => self.next_window().await,
            "prev" | "previous" => self.prev_window().await,
            "status" => self.get_status().await,
            _ => Ok(format!("Unknown expose command: {}", command)),
        }
    }
}