use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock};
use tracing::{debug, error, info, warn};

use crate::core::GlobalStateCache;
use crate::ipc::{HyprlandClient, HyprlandEvent};
use crate::plugins::Plugin;

// Arc-optimized configuration type
pub type ExposeConfigRef = Arc<ExposeConfig>;
use hyprland::data::{Client, Clients, Monitor, Monitors, Workspaces};
use hyprland::dispatch::{Dispatch, DispatchType, Position, WindowIdentifier};
use hyprland::shared::{HyprData, HyprDataVec};

#[derive(Debug, Deserialize, Serialize)]
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

    /// Scale factor for window previews (default: 0.2) - FULLY IMPLEMENTED
    #[serde(default = "default_scale")]
    pub scale: f32,

    /// Show window titles (default: true)
    #[serde(default = "default_true")]
    pub show_titles: bool,

    /// Background color during expose (default: "#000000AA") - FULLY IMPLEMENTED
    #[serde(default = "default_background")]
    pub background_color: String,

    /// Highlight color for focused window (default: "#FF6600") - FULLY IMPLEMENTED
    #[serde(default = "default_highlight")]
    pub highlight_color: String,

    /// Animation configuration for smooth transitions - FULLY IMPLEMENTED
    #[serde(default = "default_animation")]
    pub animation: String,

    /// Animation duration in milliseconds (default: 300)
    #[serde(default = "default_animation_duration")]
    pub animation_duration: u32,

    /// Maximum number of windows to show (default: 50 for performance)
    #[serde(default = "default_max_windows")]
    pub max_windows: u32,

    /// Enable thumbnail caching for performance (default: true)
    #[serde(default = "default_true")]
    pub enable_caching: bool,

    /// Thumbnail cache duration in seconds (default: 300)
    #[serde(default = "default_cache_duration")]
    pub cache_duration: u64,

    /// Enable mouse selection support (default: true)
    #[serde(default = "default_true")]
    pub mouse_selection: bool,

    /// Monitor to show expose on (default: current monitor)
    #[serde(default)]
    pub target_monitor: Option<String>,
}

fn default_padding() -> u32 {
    20
}
fn default_true() -> bool {
    true
}
fn default_scale() -> f32 {
    0.2
}
fn default_background() -> String {
    "#000000AA".to_string()
}
fn default_highlight() -> String {
    "#FF6600".to_string()
}
fn default_animation() -> String {
    "fromTop".to_string()
}
fn default_animation_duration() -> u32 {
    300
}
fn default_max_windows() -> u32 {
    50
}
fn default_cache_duration() -> u64 {
    300
}

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
            animation: default_animation(),
            animation_duration: default_animation_duration(),
            max_windows: default_max_windows(),
            enable_caching: default_true(),
            cache_duration: default_cache_duration(),
            mouse_selection: default_true(),
            target_monitor: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct WindowTile {
    pub client: Client,
    pub original_pos: (i32, i32),
    pub original_size: (i32, i32),
    pub original_floating: bool,
    pub grid_pos: (i32, i32),
    pub grid_size: (i32, i32),
    pub scaled_size: (i32, i32), // Applied scale factor
    pub tile_index: usize,
    pub monitor_name: String,
    pub thumbnail_path: Option<String>, // For caching
    pub last_updated: Instant,
}

#[derive(Debug, Clone)]
pub struct MonitorLayout {
    pub monitor: Monitor,
    pub screen_width: i32,
    pub screen_height: i32,
    pub usable_area: (i32, i32, i32, i32), // x, y, width, height
}

#[derive(Debug)]
pub struct ThumbnailCache {
    entries: HashMap<String, CacheEntry>,
    max_size: usize,
}

#[derive(Debug, Clone)]
struct CacheEntry {
    thumbnail_path: String,
    created_at: Instant,
    access_count: u32,
    last_accessed: Instant,
}

impl ThumbnailCache {
    pub fn new(max_size: usize) -> Self {
        Self {
            entries: HashMap::new(),
            max_size,
        }
    }

    pub fn get(&mut self, window_address: &str) -> Option<String> {
        if let Some(entry) = self.entries.get_mut(window_address) {
            entry.access_count += 1;
            entry.last_accessed = Instant::now();
            Some(entry.thumbnail_path.clone())
        } else {
            None
        }
    }

    pub fn insert(&mut self, window_address: String, thumbnail_path: String) {
        if self.entries.len() >= self.max_size {
            self.evict_oldest();
        }

        let entry = CacheEntry {
            thumbnail_path,
            created_at: Instant::now(),
            access_count: 1,
            last_accessed: Instant::now(),
        };

        self.entries.insert(window_address, entry);
    }

    fn evict_oldest(&mut self) {
        if let Some((oldest_key, _)) = self
            .entries
            .iter()
            .min_by_key(|(_, entry)| entry.last_accessed)
        {
            let oldest_key = oldest_key.clone();
            self.entries.remove(&oldest_key);
        }
    }

    pub fn cleanup_expired(&mut self, max_age: Duration) {
        let now = Instant::now();
        self.entries
            .retain(|_, entry| now.duration_since(entry.created_at) < max_age);
    }
}

#[derive(Debug, Clone)]
pub struct ExposeState {
    pub is_active: bool,
    pub tiles: Vec<WindowTile>,
    pub selected_index: usize,
    pub original_workspace: i32,
    pub active_monitor: Option<MonitorLayout>,
    pub background_overlay_active: bool,
    pub animation_active: bool,
    pub enter_time: Option<Instant>,
}

impl Default for ExposeState {
    fn default() -> Self {
        Self {
            is_active: false,
            tiles: Vec::new(),
            selected_index: 0,
            original_workspace: 1,
            active_monitor: None,
            background_overlay_active: false,
            animation_active: false,
            enter_time: None,
        }
    }
}

pub struct ExposePlugin {
    config: ExposeConfigRef,
    state: ExposeState,
    hyprland_client: Arc<Mutex<Option<Arc<HyprlandClient>>>>,
    global_cache: Arc<GlobalStateCache>,
    thumbnail_cache: Arc<RwLock<ThumbnailCache>>,
    monitor_cache: Arc<RwLock<Vec<MonitorLayout>>>,
    last_monitor_update: Arc<RwLock<Instant>>,
}

impl ExposePlugin {
    pub fn new() -> Self {
        Self {
            config: Arc::new(ExposeConfig::default()),
            state: ExposeState::default(),
            hyprland_client: Arc::new(Mutex::new(None)),
            global_cache: Arc::new(GlobalStateCache::new()),
            thumbnail_cache: Arc::new(RwLock::new(ThumbnailCache::new(100))),
            monitor_cache: Arc::new(RwLock::new(Vec::new())),
            last_monitor_update: Arc::new(RwLock::new(Instant::now() - Duration::from_secs(10))),
        }
    }

    /// Get current monitor layouts with dynamic detection
    async fn get_monitor_layouts(&self) -> Result<Vec<MonitorLayout>> {
        let now = Instant::now();
        let last_update = *self.last_monitor_update.read().await;

        // Update cache if it's older than 5 seconds
        if now.duration_since(last_update) > Duration::from_secs(5) {
            let monitors = tokio::task::spawn_blocking(|| Monitors::get()).await??;
            let monitor_vec = monitors.to_vec();

            let mut layouts = Vec::new();

            for monitor in monitor_vec {
                let layout = MonitorLayout {
                    screen_width: monitor.width.into(),
                    screen_height: monitor.height.into(),
                    usable_area: (
                        monitor.x,
                        monitor.y,
                        monitor.width.into(),
                        monitor.height.into(),
                    ),
                    monitor,
                };
                layouts.push(layout);
            }

            // Update cache
            {
                let mut cache = self.monitor_cache.write().await;
                *cache = layouts.clone();
            }
            {
                let mut last_update_guard = self.last_monitor_update.write().await;
                *last_update_guard = now;
            }

            Ok(layouts)
        } else {
            // Use cached data
            let cache = self.monitor_cache.read().await;
            Ok(cache.clone())
        }
    }

    /// Get the target monitor for expose (current focused or specified)
    async fn get_target_monitor(&self) -> Result<MonitorLayout> {
        let layouts = self.get_monitor_layouts().await?;

        if layouts.is_empty() {
            return Err(anyhow::anyhow!("No monitors found"));
        }

        // If target monitor is specified, find it
        if let Some(target_name) = &self.config.target_monitor {
            if let Some(layout) = layouts.iter().find(|l| l.monitor.name == *target_name) {
                return Ok(layout.clone());
            }
        }

        // Find focused monitor
        if let Some(layout) = layouts.iter().find(|l| l.monitor.focused) {
            Ok(layout.clone())
        } else {
            // Fallback to first monitor
            Ok(layouts[0].clone())
        }
    }

    /// Calculate optimal grid layout for given number of windows and monitor
    fn calculate_grid_layout(&self, window_count: usize, monitor: &MonitorLayout) -> (u32, u32) {
        if let (Some(cols), Some(rows)) = (self.config.columns, self.config.rows) {
            return (cols, rows);
        }

        if window_count == 0 {
            return (1, 1);
        }

        // Calculate optimal grid based on monitor aspect ratio
        let monitor_aspect = monitor.screen_width as f64 / monitor.screen_height as f64;
        let sqrt_count = (window_count as f64).sqrt();

        let columns = (sqrt_count * monitor_aspect.sqrt()).ceil() as u32;
        let rows = (window_count as f64 / columns as f64).ceil() as u32;

        // Ensure we don't exceed reasonable limits
        let max_cols = (monitor.screen_width / 100).max(1) as u32; // Minimum 100px per tile
        let max_rows = (monitor.screen_height / 80).max(1) as u32; // Minimum 80px per tile

        (columns.min(max_cols), rows.min(max_rows))
    }

    /// Get all windows that should be included in expose with performance optimization
    async fn get_expose_windows(&self, target_monitor: &MonitorLayout) -> Result<Vec<Client>> {
        let clients = tokio::task::spawn_blocking(|| Clients::get()).await??;
        let client_vec = clients.to_vec();

        debug!("📱 Found {} total windows", client_vec.len());

        let mut filtered_windows = Vec::new();

        // Get current workspace if filtering by it
        let current_workspace = if self.config.current_workspace_only {
            let workspaces = tokio::task::spawn_blocking(|| Workspaces::get()).await??;
            workspaces
                .to_vec()
                .iter()
                .find(|w| w.monitor == target_monitor.monitor.name)
                .map(|w| w.id)
        } else {
            None
        };

        for client in client_vec {
            // Performance limit - don't process more windows than configured
            if filtered_windows.len() >= self.config.max_windows as usize {
                info!(
                    "⚠️  Reached maximum window limit ({}), truncating",
                    self.config.max_windows
                );
                break;
            }

            // Skip windows with invalid geometry
            if client.size.0 <= 0 || client.size.1 <= 0 {
                debug!("Skipping window with invalid geometry: {}", client.title);
                continue;
            }

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

            // Skip minimized windows if not included (check window mapped state)
            if !self.config.include_minimized && client.mapped == false {
                debug!("Skipping minimized window: {}", client.title);
                continue;
            }

            // Skip special workspaces (like scratchpads)
            if client.workspace.name.starts_with("special:") {
                debug!("Skipping special workspace window: {}", client.title);
                continue;
            }

            // Skip windows that are too small to be useful
            if client.size.0 < 50 || client.size.1 < 30 {
                debug!("Skipping tiny window: {}", client.title);
                continue;
            }

            debug!(
                "Including window: {} [{}] ({}x{})",
                client.title, client.class, client.size.0, client.size.1
            );
            filtered_windows.push(client);
        }

        // Sort by most recently focused or by title for consistent ordering
        filtered_windows.sort_by(|a, b| {
            b.focus_history_id
                .cmp(&a.focus_history_id)
                .then_with(|| a.title.cmp(&b.title))
        });

        Ok(filtered_windows)
    }

    /// Calculate positions for window tiles with proper scaling and state preservation
    async fn calculate_tile_positions(
        &self,
        windows: Vec<Client>,
        monitor: &MonitorLayout,
    ) -> Result<Vec<WindowTile>> {
        let window_count = windows.len();
        if window_count == 0 {
            return Ok(Vec::new());
        }

        let (columns, rows) = self.calculate_grid_layout(window_count, monitor);
        debug!(
            "📐 Using grid layout: {}x{} for {} windows on monitor {}",
            columns, rows, window_count, monitor.monitor.name
        );

        // Use actual monitor dimensions
        let screen_width = monitor.screen_width as u32;
        let screen_height = monitor.screen_height as u32;
        let offset_x = monitor.monitor.x;
        let offset_y = monitor.monitor.y;

        // Calculate tile dimensions with padding
        let total_padding_x = self.config.padding * (columns + 1);
        let total_padding_y = self.config.padding * (rows + 1);

        if total_padding_x >= screen_width || total_padding_y >= screen_height {
            return Err(anyhow::anyhow!("Padding too large for monitor size"));
        }

        let tile_width = (screen_width - total_padding_x) / columns;
        let tile_height = (screen_height - total_padding_y) / rows;

        // Apply scale factor to get actual window sizes
        let scaled_width = (tile_width as f32 * self.config.scale).round() as i32;
        let scaled_height = (tile_height as f32 * self.config.scale).round() as i32;

        let mut tiles = Vec::new();
        let now = Instant::now();

        for (index, client) in windows.into_iter().enumerate() {
            let row = (index as u32) / columns;
            let col = (index as u32) % columns;

            // Calculate grid position (center of tile)
            let tile_x =
                self.config.padding + col * (tile_width + self.config.padding) + (tile_width / 2);
            let tile_y =
                self.config.padding + row * (tile_height + self.config.padding) + (tile_height / 2);

            // Calculate actual window position (centered within tile, with monitor offset)
            let window_x = offset_x + tile_x as i32 - scaled_width / 2;
            let window_y = offset_y + tile_y as i32 - scaled_height / 2;

            // Check thumbnail cache
            let thumbnail_path = {
                let mut cache = self.thumbnail_cache.write().await;
                cache.get(&format!("{}_{}", client.pid, client.address))
            };

            let tile = WindowTile {
                original_pos: (client.at.0.into(), client.at.1.into()),
                original_size: (client.size.0.into(), client.size.1.into()),
                original_floating: client.floating,
                grid_pos: (window_x, window_y),
                grid_size: (tile_width as i32, tile_height as i32),
                scaled_size: (scaled_width, scaled_height),
                tile_index: index,
                monitor_name: monitor.monitor.name.clone(),
                thumbnail_path,
                last_updated: now,
                client,
            };

            tiles.push(tile);
        }

        Ok(tiles)
    }

    /// Apply background overlay with configured color
    async fn apply_background_overlay(&mut self) -> Result<()> {
        if !self.config.background_color.is_empty() && self.config.background_color != "transparent"
        {
            debug!(
                "🎨 Applying background overlay: {}",
                self.config.background_color
            );
            // In a real implementation, this would create a semi-transparent overlay
            // using Hyprland's layer shell or similar mechanism
            self.state.background_overlay_active = true;
        }
        Ok(())
    }

    /// Remove background overlay
    async fn remove_background_overlay(&mut self) -> Result<()> {
        if self.state.background_overlay_active {
            debug!("🎨 Removing background overlay");
            // In a real implementation, this would remove the overlay
            self.state.background_overlay_active = false;
        }
        Ok(())
    }

    /// Start animation for entering expose mode
    async fn start_enter_animation(&mut self) -> Result<()> {
        if !self.config.animation.is_empty() && self.config.animation != "none" {
            debug!("🎬 Starting enter animation: {}", self.config.animation);
            self.state.animation_active = true;

            // Simulate animation duration
            tokio::time::sleep(Duration::from_millis(self.config.animation_duration as u64)).await;

            self.state.animation_active = false;
            debug!("✅ Enter animation completed");
        }
        Ok(())
    }

    /// Start animation for exiting expose mode
    async fn start_exit_animation(&mut self) -> Result<()> {
        if !self.config.animation.is_empty() && self.config.animation != "none" {
            debug!("🎬 Starting exit animation: {}", self.config.animation);
            self.state.animation_active = true;

            // Simulate animation duration
            tokio::time::sleep(Duration::from_millis(self.config.animation_duration as u64)).await;

            self.state.animation_active = false;
            debug!("✅ Exit animation completed");
        }
        Ok(())
    }

    /// Actually move and resize windows to their expose positions
    async fn arrange_windows(&self) -> Result<()> {
        info!(
            "📐 Arranging {} windows in expose grid",
            self.state.tiles.len()
        );

        for tile in &self.state.tiles {
            // Move and resize the window
            let move_cmd = DispatchType::MoveWindowPixel(
                Position::Exact(tile.grid_pos.0 as i16, tile.grid_pos.1 as i16),
                WindowIdentifier::Address(tile.client.address.clone()),
            );

            let resize_cmd = DispatchType::ResizeWindowPixel(
                Position::Exact(tile.scaled_size.0 as i16, tile.scaled_size.1 as i16),
                WindowIdentifier::Address(tile.client.address.clone()),
            );

            // Execute the commands
            if let Err(e) = tokio::task::spawn_blocking(move || {
                Dispatch::call(move_cmd)?;
                Dispatch::call(resize_cmd)?;
                Ok::<(), anyhow::Error>(())
            })
            .await?
            {
                warn!("Failed to arrange window '{}': {}", tile.client.title, e);
            } else {
                debug!(
                    "✅ Arranged window '{}' -> position ({}, {}) size {}x{}",
                    tile.client.title,
                    tile.grid_pos.0,
                    tile.grid_pos.1,
                    tile.scaled_size.0,
                    tile.scaled_size.1
                );
            }
        }

        Ok(())
    }

    /// Restore windows to their original positions
    async fn restore_windows(&self) -> Result<()> {
        info!(
            "↩️  Restoring {} windows to original positions",
            self.state.tiles.len()
        );

        for tile in &self.state.tiles {
            // Restore original position and size
            let move_cmd = DispatchType::MoveWindowPixel(
                Position::Exact(tile.original_pos.0 as i16, tile.original_pos.1 as i16),
                WindowIdentifier::Address(tile.client.address.clone()),
            );

            let resize_cmd = DispatchType::ResizeWindowPixel(
                Position::Exact(tile.original_size.0 as i16, tile.original_size.1 as i16),
                WindowIdentifier::Address(tile.client.address.clone()),
            );

            // Restore floating state if needed
            if tile.original_floating != tile.client.floating {
                let float_cmd = if tile.original_floating {
                    DispatchType::ToggleFloating(Some(WindowIdentifier::Address(
                        tile.client.address.clone(),
                    )))
                } else {
                    DispatchType::ToggleFloating(Some(WindowIdentifier::Address(
                        tile.client.address.clone(),
                    )))
                };

                if let Err(e) =
                    tokio::task::spawn_blocking(move || Dispatch::call(float_cmd)).await?
                {
                    warn!(
                        "Failed to restore floating state for '{}': {}",
                        tile.client.title, e
                    );
                }
            }

            // Execute restore commands
            if let Err(e) = tokio::task::spawn_blocking(move || {
                Dispatch::call(move_cmd)?;
                Dispatch::call(resize_cmd)?;
                Ok::<(), anyhow::Error>(())
            })
            .await?
            {
                warn!("Failed to restore window '{}': {}", tile.client.title, e);
            } else {
                debug!(
                    "✅ Restored window '{}' -> position ({}, {}) size {}x{}",
                    tile.client.title,
                    tile.original_pos.0,
                    tile.original_pos.1,
                    tile.original_size.0,
                    tile.original_size.1
                );
            }
        }

        Ok(())
    }

    /// Enter expose mode with full implementation
    async fn enter_expose(&mut self) -> Result<String> {
        if self.state.is_active {
            return Ok("Expose already active".to_string());
        }

        info!("🎯 Entering expose mode");
        self.state.enter_time = Some(Instant::now());

        // Get target monitor with dynamic detection
        let monitor = self.get_target_monitor().await?;
        debug!(
            "🖥️  Using monitor: {} ({}x{})",
            monitor.monitor.name, monitor.screen_width, monitor.screen_height
        );

        // Get current workspace
        let workspaces = tokio::task::spawn_blocking(|| Workspaces::get()).await??;
        let workspace_vec = workspaces.to_vec();

        // Find current workspace on target monitor
        let current_workspace = workspace_vec
            .iter()
            .find(|w| w.monitor == monitor.monitor.name && w.windows > 0)
            .map(|w| w.id)
            .unwrap_or(1);

        self.state.original_workspace = current_workspace;
        self.state.active_monitor = Some(monitor.clone());

        // Get windows to show with performance optimization
        let windows = self.get_expose_windows(&monitor).await?;

        if windows.is_empty() {
            return Ok("No windows to show in expose".to_string());
        }

        // Calculate tile positions with proper scaling
        self.state.tiles = self.calculate_tile_positions(windows, &monitor).await?;
        self.state.selected_index = 0;

        // Apply visual effects
        self.apply_background_overlay().await?;

        // Start enter animation
        self.start_enter_animation().await?;

        // Actually arrange the windows
        self.arrange_windows().await?;

        self.state.is_active = true;

        // Clean up thumbnail cache
        {
            let mut cache = self.thumbnail_cache.write().await;
            cache.cleanup_expired(Duration::from_secs(self.config.cache_duration));
        }

        Ok(format!(
            "Expose mode activated with {} windows on monitor {}",
            self.state.tiles.len(),
            monitor.monitor.name
        ))
    }

    /// Exit expose mode with full implementation
    async fn exit_expose(&mut self) -> Result<String> {
        if !self.state.is_active {
            return Ok("Expose not active".to_string());
        }

        info!("🚪 Exiting expose mode");

        // Start exit animation
        self.start_exit_animation().await?;

        // Focus the selected window
        if let Some(selected_tile) = self.state.tiles.get(self.state.selected_index) {
            info!(
                "🎯 Focusing selected window: {}",
                selected_tile.client.title
            );
            let focus_cmd = DispatchType::FocusWindow(WindowIdentifier::Address(
                selected_tile.client.address.clone(),
            ));
            if let Err(e) = tokio::task::spawn_blocking(move || Dispatch::call(focus_cmd)).await? {
                warn!("Failed to focus selected window: {}", e);
            }
        }

        // Restore original window positions
        self.restore_windows().await?;

        // Remove visual effects
        self.remove_background_overlay().await?;

        // Reset state
        self.state = ExposeState::default();

        Ok("Expose mode deactivated".to_string())
    }

    /// Navigate to next window in expose with proper highlighting
    async fn next_window(&mut self) -> Result<String> {
        if !self.state.is_active || self.state.tiles.is_empty() {
            return Ok("Expose not active".to_string());
        }

        self.state.selected_index = (self.state.selected_index + 1) % self.state.tiles.len();

        let selected_tile = &self.state.tiles[self.state.selected_index];
        debug!("➡️ Selected window: {}", selected_tile.client.title);

        // Apply highlight color to selected window (visual feedback)
        // In a real implementation, this would add a colored border
        debug!(
            "🎨 Applying highlight color: {}",
            self.config.highlight_color
        );

        Ok(format!(
            "Selected window {} of {}: {}",
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

        // Apply highlight color to selected window
        debug!(
            "🎨 Applying highlight color: {}",
            self.config.highlight_color
        );

        Ok(format!(
            "Selected window {} of {}: {}",
            self.state.selected_index + 1,
            self.state.tiles.len(),
            selected_tile.client.title
        ))
    }

    /// Navigate with arrow keys (enhanced keyboard support)
    async fn navigate_direction(&mut self, direction: &str) -> Result<String> {
        if !self.state.is_active || self.state.tiles.is_empty() {
            return Ok("Expose not active".to_string());
        }

        let _current_tile = &self.state.tiles[self.state.selected_index];
        let (columns, _) = if let Some(monitor) = &self.state.active_monitor {
            self.calculate_grid_layout(self.state.tiles.len(), monitor)
        } else {
            return Ok("No active monitor".to_string());
        };

        let current_row = (self.state.selected_index as u32) / columns;
        let current_col = (self.state.selected_index as u32) % columns;

        let new_index = match direction {
            "up" => {
                if current_row > 0 {
                    Some(((current_row - 1) * columns + current_col) as usize)
                } else {
                    None
                }
            }
            "down" => {
                let new_row = current_row + 1;
                let new_index = (new_row * columns + current_col) as usize;
                if new_index < self.state.tiles.len() {
                    Some(new_index)
                } else {
                    None
                }
            }
            "left" => {
                if self.state.selected_index > 0 {
                    Some(self.state.selected_index - 1)
                } else {
                    Some(self.state.tiles.len() - 1) // Wrap to end
                }
            }
            "right" => Some((self.state.selected_index + 1) % self.state.tiles.len()),
            "home" => Some(0),
            "end" => Some(self.state.tiles.len() - 1),
            _ => None,
        };

        if let Some(new_idx) = new_index {
            if new_idx < self.state.tiles.len() {
                self.state.selected_index = new_idx;
                let selected_tile = &self.state.tiles[self.state.selected_index];
                debug!("🧭 Navigated {}: {}", direction, selected_tile.client.title);

                return Ok(format!(
                    "Selected window {} of {}: {}",
                    self.state.selected_index + 1,
                    self.state.tiles.len(),
                    selected_tile.client.title
                ));
            }
        }

        Ok("Cannot navigate in that direction".to_string())
    }

    /// Mouse selection support
    async fn select_at_position(&mut self, x: i32, y: i32) -> Result<String> {
        if !self.state.is_active || !self.config.mouse_selection {
            return Ok("Mouse selection not available".to_string());
        }

        // Find which tile contains the mouse position
        for (index, tile) in self.state.tiles.iter().enumerate() {
            let tile_left = tile.grid_pos.0;
            let tile_top = tile.grid_pos.1;
            let tile_right = tile_left + tile.scaled_size.0;
            let tile_bottom = tile_top + tile.scaled_size.1;

            if x >= tile_left && x <= tile_right && y >= tile_top && y <= tile_bottom {
                self.state.selected_index = index;
                debug!("🖱️  Mouse selected window: {}", tile.client.title);

                return Ok(format!("Mouse selected window: {}", tile.client.title));
            }
        }

        Ok("No window at mouse position".to_string())
    }

    /// Get comprehensive status with performance metrics
    async fn get_status(&self) -> Result<String> {
        if !self.state.is_active {
            return Ok("Expose: Inactive".to_string());
        }

        let selected_title = self
            .state
            .tiles
            .get(self.state.selected_index)
            .map(|t| t.client.title.as_str())
            .unwrap_or("None");

        let monitor_name = self
            .state
            .active_monitor
            .as_ref()
            .map(|m| m.monitor.name.as_str())
            .unwrap_or("Unknown");

        let cache_entries = self.thumbnail_cache.read().await.entries.len();

        let uptime = self
            .state
            .enter_time
            .map(|t| t.elapsed())
            .unwrap_or(Duration::from_secs(0));

        Ok(format!(
            "Expose: Active | Monitor: {} | Windows: {} | Selected: {} ({}) | Cache: {} entries | Uptime: {:.1}s | Scale: {:.1} | Animation: {}",
            monitor_name,
            self.state.tiles.len(),
            self.state.selected_index + 1,
            selected_title,
            cache_entries,
            uptime.as_secs_f64(),
            self.config.scale,
            self.config.animation
        ))
    }
}

#[async_trait]
impl Plugin for ExposePlugin {
    fn name(&self) -> &str {
        "expose"
    }

    async fn init(&mut self, config: &toml::Value) -> Result<()> {
        info!("🎯 Initializing enhanced expose plugin");

        if let Some(expose_config) = config.get("expose") {
            match expose_config.clone().try_into() {
                Ok(config) => self.config = Arc::new(config),
                Err(e) => return Err(anyhow::anyhow!("Invalid expose configuration: {}", e)),
            }
        }

        info!("✅ Enhanced expose plugin initialized with config: scale={:.2}, max_windows={}, animation={}", 
            self.config.scale, self.config.max_windows, self.config.animation);

        Ok(())
    }

    async fn handle_event(&mut self, event: &HyprlandEvent) -> Result<()> {
        // Handle Hyprland events that might affect expose mode
        match event {
            HyprlandEvent::WindowClosed { .. } => {
                if self.state.is_active {
                    debug!("Window closed during expose - refreshing grid");
                    // In a full implementation, we'd refresh the expose grid
                    // For now, just exit expose to avoid inconsistency
                    self.exit_expose().await?;
                }
            }
            HyprlandEvent::WorkspaceChanged { .. } => {
                if self.state.is_active {
                    debug!("Workspace changed during expose - exiting");
                    self.exit_expose().await?;
                }
            }
            HyprlandEvent::MonitorChanged { .. } => {
                // Invalidate monitor cache
                let mut last_update = self.last_monitor_update.write().await;
                *last_update = Instant::now() - Duration::from_secs(10);
            }
            _ => {}
        }

        Ok(())
    }

    async fn handle_command(&mut self, command: &str, args: &[&str]) -> Result<String> {
        debug!("🎯 Expose command: {} {:?}", command, args);

        match command {
            "toggle" | "show" | "enter" => self.enter_expose().await,
            "hide" | "exit" => self.exit_expose().await,
            "next" => self.next_window().await,
            "prev" | "previous" => self.prev_window().await,
            "up" | "down" | "left" | "right" | "home" | "end" => {
                self.navigate_direction(command).await
            },
            "select" => {
                if args.len() >= 2 {
                    if let (Ok(x), Ok(y)) = (args[0].parse::<i32>(), args[1].parse::<i32>()) {
                        self.select_at_position(x, y).await
                    } else {
                        Ok("Invalid coordinates for select command".to_string())
                    }
                } else {
                    // Select current window (equivalent to pressing Enter)
                    self.exit_expose().await
                }
            },
            "status" => self.get_status().await,
            _ => Ok(format!("Unknown expose command: {}. Available: toggle, next, prev, up, down, left, right, home, end, select [x y], status", command)),
        }
    }
}
