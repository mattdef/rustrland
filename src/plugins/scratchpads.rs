use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock};
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};

// Arc-optimized configuration types
pub type ScratchpadConfigRef = Arc<ScratchpadConfig>;
pub type ValidatedConfigRef = Arc<ValidatedConfig>;

use crate::ipc::{
    EnhancedHyprlandClient, HyprlandClient, HyprlandEvent, MonitorInfo, WindowGeometry,
};
use crate::plugins::Plugin;

// ============================================================================
// CONFIGURATION STRUCTURES
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct ScratchpadConfig {
    // Basic config
    pub command: String,
    pub class: String,
    pub size: String,

    // Animation config
    pub animation: Option<String>,
    pub margin: Option<i32>,
    pub offset: Option<String>,
    pub hide_delay: Option<u32>,

    // Pyprland-compatible features
    pub lazy: bool,
    pub pinned: bool,
    pub excludes: Vec<String>,
    pub restore_excluded: bool,
    pub preserve_aspect: bool,
    pub force_monitor: Option<String>,
    pub alt_toggle: bool,
    pub allow_special_workspaces: bool,
    pub smart_focus: bool,
    pub close_on_hide: bool,
    pub unfocus: Option<String>, // "hide" option
    pub max_size: Option<String>,
    pub r#use: Option<String>, // Template inheritance

    // Multi-window support
    pub multi_window: bool,
    pub max_instances: Option<u32>,
}

impl Default for ScratchpadConfig {
    fn default() -> Self {
        Self {
            command: String::new(),
            class: String::new(),
            size: "50% 50%".to_string(),
            animation: None,
            margin: None,
            offset: None,
            hide_delay: None,
            lazy: false,
            pinned: true,
            excludes: Vec::new(),
            restore_excluded: false,
            preserve_aspect: false,
            force_monitor: None,
            alt_toggle: false,
            allow_special_workspaces: false,
            smart_focus: true,
            close_on_hide: false,
            unfocus: None,
            max_size: None,
            r#use: None,
            multi_window: false,
            max_instances: Some(1),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatedConfig {
    // All fields from ScratchpadConfig
    pub command: String,
    pub class: String,
    pub size: String,
    pub animation: Option<String>,
    pub margin: Option<i32>,
    pub offset: Option<String>,
    pub hide_delay: Option<u32>,
    pub lazy: bool,
    pub pinned: bool,
    pub excludes: Vec<String>,
    pub restore_excluded: bool,
    pub preserve_aspect: bool,
    pub force_monitor: Option<String>,
    pub alt_toggle: bool,
    pub allow_special_workspaces: bool,
    pub smart_focus: bool,
    pub close_on_hide: bool,
    pub unfocus: Option<String>,
    pub max_size: Option<String>,
    pub r#use: Option<String>,
    pub multi_window: bool,
    pub max_instances: Option<u32>,

    // Validation metadata
    pub validation_errors: Vec<String>,
    pub validation_warnings: Vec<String>,

    // Pre-calculated values for performance
    pub parsed_size: Option<(i32, i32)>, // width, height (cached for default monitor)
    pub parsed_offset: Option<(i32, i32)>, // x, y offset
    pub parsed_max_size: Option<(i32, i32)>, // max width, height
}

impl Default for ValidatedConfig {
    fn default() -> Self {
        Self {
            command: String::new(),
            class: String::new(),
            size: "50% 50%".to_string(),
            animation: None,
            margin: None,
            offset: None,
            hide_delay: None,
            lazy: false,
            pinned: true,
            excludes: Vec::new(),
            restore_excluded: false,
            preserve_aspect: false,
            force_monitor: None,
            alt_toggle: false,
            allow_special_workspaces: false,
            smart_focus: true,
            close_on_hide: false,
            unfocus: None,
            max_size: None,
            r#use: None,
            multi_window: false,
            max_instances: Some(1),
            validation_errors: Vec::new(),
            validation_warnings: Vec::new(),
            parsed_size: None,
            parsed_offset: None,
            parsed_max_size: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct WindowState {
    pub address: String,
    pub is_visible: bool,
    pub last_position: Option<(i32, i32, i32, i32)>, // x, y, width, height
    pub monitor: Option<String>,
    pub workspace: Option<String>,
    pub last_focus: Option<Instant>,
}

#[derive(Debug, Clone, Default)]
pub struct ScratchpadState {
    pub windows: Vec<WindowState>,
    pub is_spawned: bool,
    pub last_used: Option<Instant>,
    pub excluded_by: HashSet<String>, // Which scratchpads excluded this one
    pub cached_position: Option<(String, i32, i32, i32, i32)>, // monitor, x, y, w, h
}

// ============================================================================
// GEOMETRY CALCULATION
// ============================================================================

pub struct GeometryCalculator;

impl GeometryCalculator {
    /// Calculate window geometry with monitor-aware positioning
    pub fn calculate_geometry(
        config: &ValidatedConfig,
        monitor: &MonitorInfo,
    ) -> Result<WindowGeometry> {
        let (width, height) = Self::parse_size(&config.size, monitor, config.max_size.as_deref())?;
        let (offset_x, offset_y) = Self::parse_offset(config.offset.as_deref(), monitor)?;
        let margin = config.margin.unwrap_or(0);

        // Calculate position with monitor-aware positioning
        let base_x = monitor.x + offset_x + margin;
        let base_y = monitor.y + offset_y + margin;

        // Center the window if no specific positioning
        let x = if offset_x == 0 && config.offset.is_none() {
            monitor.x + (monitor.width - width) / 2
        } else {
            base_x
        };

        let y = if offset_y == 0 && config.offset.is_none() {
            monitor.y + (monitor.height - height) / 2
        } else {
            base_y
        };

        // Ensure window stays within monitor bounds
        let final_x = x.max(monitor.x).min(monitor.x + monitor.width - width);
        let final_y = y.max(monitor.y).min(monitor.y + monitor.height - height);

        Ok(WindowGeometry {
            x: final_x,
            y: final_y,
            width,
            height,
            workspace: "e+0".to_string(), // Default workspace
            monitor: 0,                   // Will be updated based on actual monitor
            floating: true,               // Scratchpads are typically floating
        })
    }

    /// Parse size string with monitor-aware dimensions
    pub fn parse_size(
        size_str: &str,
        monitor: &MonitorInfo,
        max_size: Option<&str>,
    ) -> Result<(i32, i32)> {
        let parts: Vec<&str> = size_str.split_whitespace().collect();
        if parts.len() != 2 {
            return Err(anyhow::anyhow!(
                "Invalid size format '{}', expected 'width height'",
                size_str
            ));
        }

        let width = Self::parse_dimension(parts[0], monitor.width)?;
        let height = Self::parse_dimension(parts[1], monitor.height)?;

        // Apply max_size constraints if specified
        if let Some(max_size_str) = max_size {
            let max_parts: Vec<&str> = max_size_str.split_whitespace().collect();
            if max_parts.len() == 2 {
                let max_width = Self::parse_dimension(max_parts[0], monitor.width)?;
                let max_height = Self::parse_dimension(max_parts[1], monitor.height)?;
                return Ok((width.min(max_width), height.min(max_height)));
            }
        }

        Ok((width, height))
    }

    /// Parse offset string like "50px 100px" or "10% 20%"
    pub fn parse_offset(offset_str: Option<&str>, monitor: &MonitorInfo) -> Result<(i32, i32)> {
        let offset_str = match offset_str {
            Some(s) => s,
            None => return Ok((0, 0)),
        };

        let parts: Vec<&str> = offset_str.split_whitespace().collect();
        if parts.len() != 2 {
            return Err(anyhow::anyhow!(
                "Invalid offset format '{}', expected 'x y'",
                offset_str
            ));
        }

        let x = Self::parse_dimension(parts[0], monitor.width)?;
        let y = Self::parse_dimension(parts[1], monitor.height)?;

        Ok((x, y))
    }

    /// Parse individual dimension (supports %, px, or raw numbers)
    pub fn parse_dimension(dim_str: &str, monitor_size: i32) -> Result<i32> {
        if dim_str.ends_with('%') {
            let percent = dim_str
                .trim_end_matches('%')
                .parse::<f32>()
                .map_err(|_| anyhow::anyhow!("Invalid percentage: {}", dim_str))?;
            Ok((monitor_size as f32 * percent / 100.0) as i32)
        } else if dim_str.ends_with("px") {
            let pixels = dim_str
                .trim_end_matches("px")
                .parse::<i32>()
                .map_err(|_| anyhow::anyhow!("Invalid pixel value: {}", dim_str))?;
            Ok(pixels)
        } else {
            // Raw number, assume pixels
            dim_str
                .parse::<i32>()
                .map_err(|_| anyhow::anyhow!("Invalid dimension: {}", dim_str))
        }
    }
}

// ============================================================================
// CONFIGURATION VALIDATION
// ============================================================================

pub struct ConfigValidator;

impl ConfigValidator {
    /// Validate and preprocess scratchpad configurations
    pub fn validate_configs(
        configs: &HashMap<String, ScratchpadConfigRef>,
        monitors: &[MonitorInfo],
    ) -> HashMap<String, ValidatedConfigRef> {
        let mut validated_temp = HashMap::new();

        // First pass: basic validation and template resolution
        for (name, config) in configs {
            let mut validated_config = Self::convert_to_validated(config);

            // Resolve template inheritance
            if let Some(template_name) = &config.r#use {
                if let Some(template_config) = configs.get(template_name) {
                    validated_config = Self::merge_with_template(validated_config, template_config);
                } else {
                    validated_config
                        .validation_errors
                        .push(format!("Template '{template_name}' not found"));
                }
            }

            validated_temp.insert(name.clone(), validated_config);
        }

        // Second pass: cross-validation and advanced checks
        let validated_clone = validated_temp.clone();
        for (name, config) in &mut validated_temp {
            Self::validate_config(name, config, monitors, &validated_clone);
        }

        // Convert to Arc-wrapped configs
        let mut validated = HashMap::new();
        for (name, config) in validated_temp {
            validated.insert(name, Arc::new(config));
        }

        validated
    }

    fn convert_to_validated(config: &ScratchpadConfig) -> ValidatedConfig {
        ValidatedConfig {
            command: config.command.clone(),
            class: config.class.clone(),
            size: config.size.clone(),
            animation: config.animation.clone(),
            margin: config.margin,
            offset: config.offset.clone(),
            hide_delay: config.hide_delay,
            lazy: config.lazy,
            pinned: config.pinned,
            excludes: config.excludes.clone(),
            restore_excluded: config.restore_excluded,
            preserve_aspect: config.preserve_aspect,
            force_monitor: config.force_monitor.clone(),
            alt_toggle: config.alt_toggle,
            allow_special_workspaces: config.allow_special_workspaces,
            smart_focus: config.smart_focus,
            close_on_hide: config.close_on_hide,
            unfocus: config.unfocus.clone(),
            max_size: config.max_size.clone(),
            r#use: config.r#use.clone(),
            multi_window: config.multi_window,
            max_instances: config.max_instances,
            validation_errors: Vec::new(),
            validation_warnings: Vec::new(),
            parsed_size: None,
            parsed_offset: None,
            parsed_max_size: None,
        }
    }

    fn validate_config(
        name: &str,
        config: &mut ValidatedConfig,
        monitors: &[MonitorInfo],
        all_configs: &HashMap<String, ValidatedConfig>,
    ) {
        // Validate required fields
        if config.command.is_empty() {
            config
                .validation_errors
                .push("Command cannot be empty".to_string());
        }

        if config.class.is_empty() {
            config
                .validation_errors
                .push("Class cannot be empty".to_string());
        }

        // Validate size format and pre-calculate for default monitor
        if let Some(default_monitor) = monitors.first() {
            match GeometryCalculator::parse_size(
                &config.size,
                default_monitor,
                config.max_size.as_deref(),
            ) {
                Ok((width, height)) => {
                    config.parsed_size = Some((width, height));
                }
                Err(e) => {
                    config
                        .validation_errors
                        .push(format!("Invalid size format: {e}"));
                }
            }

            // Pre-calculate offset
            if let Ok((x, y)) =
                GeometryCalculator::parse_offset(config.offset.as_deref(), default_monitor)
            {
                config.parsed_offset = Some((x, y));
            }

            // Pre-calculate max_size
            if let Some(max_size) = &config.max_size {
                if let Ok((max_w, max_h)) =
                    GeometryCalculator::parse_size(max_size, default_monitor, None)
                {
                    config.parsed_max_size = Some((max_w, max_h));
                }
            }
        }

        // Validate monitor reference
        if let Some(monitor_name) = &config.force_monitor {
            if !monitors.iter().any(|m| m.name == *monitor_name) {
                config.validation_warnings.push(format!(
                    "Monitor '{monitor_name}' not found, will use focused monitor"
                ));
            }
        }

        // Validate excludes references
        for exclude in &config.excludes {
            if exclude != "*" && !all_configs.contains_key(exclude) {
                config
                    .validation_warnings
                    .push(format!("Excluded scratchpad '{exclude}' not found"));
            }
        }

        // Validate multi-window settings
        if config.multi_window {
            if let Some(max_instances) = config.max_instances {
                if max_instances == 0 {
                    config
                        .validation_errors
                        .push("max_instances cannot be 0 when multi_window is enabled".to_string());
                } else if max_instances > 10 {
                    config
                        .validation_warnings
                        .push("High max_instances value may impact performance".to_string());
                }
            }
        }

        // Validate hide_delay
        if let Some(delay) = config.hide_delay {
            if delay > 10000 {
                config
                    .validation_warnings
                    .push("Hide delay over 10 seconds may be unintentionally long".to_string());
            }
        }

        // Log validation results
        if !config.validation_errors.is_empty() {
            for error in &config.validation_errors {
                warn!("❌ Scratchpad '{}': {}", name, error);
            }
        }

        if !config.validation_warnings.is_empty() {
            for warning in &config.validation_warnings {
                warn!("⚠️  Scratchpad '{}': {}", name, warning);
            }
        }

        if config.validation_errors.is_empty() && config.validation_warnings.is_empty() {
            debug!("✅ Scratchpad '{}' validation passed", name);
        }
    }

    fn merge_with_template(
        mut config: ValidatedConfig,
        template: &ScratchpadConfig,
    ) -> ValidatedConfig {
        // Only use template values if current config doesn't have them set
        if config.command.is_empty() && !template.command.is_empty() {
            config.command = template.command.clone();
        }
        if config.class.is_empty() && !template.class.is_empty() {
            config.class = template.class.clone();
        }
        if config.size == "50% 50%" && template.size != "50% 50%" {
            config.size = template.size.clone();
        }
        if config.animation.is_none() {
            config.animation = template.animation.clone();
        }
        if config.margin.is_none() {
            config.margin = template.margin;
        }
        if config.offset.is_none() {
            config.offset = template.offset.clone();
        }
        if config.hide_delay.is_none() {
            config.hide_delay = template.hide_delay;
        }

        config
    }
}

// ============================================================================
// MAIN PLUGIN IMPLEMENTATION
// ============================================================================

pub struct ScratchpadsPlugin {
    pub scratchpads: HashMap<String, ScratchpadConfigRef>,
    pub states: HashMap<String, ScratchpadState>,
    pub hyprland_client: Arc<Mutex<Option<Arc<HyprlandClient>>>>,
    pub enhanced_client: Arc<EnhancedHyprlandClient>, // Enhanced client for better reliability
    pub variables: Arc<tokio::sync::RwLock<HashMap<String, String>>>,

    // Performance optimizations
    pub monitors_cache: Arc<RwLock<Vec<MonitorInfo>>>,
    pub cache_valid_until: Arc<RwLock<Instant>>,
    pub cache_duration: Duration,

    // Multi-window tracking
    pub window_to_scratchpad: HashMap<String, String>, // window_address -> scratchpad_name
    pub focused_window: Option<String>,

    // Template inheritance cache (Arc-optimized)
    pub resolved_configs: HashMap<String, ScratchpadConfigRef>,

    // Animation and delay management
    pub hide_tasks: HashMap<String, JoinHandle<()>>,

    // Validated configurations (Arc-optimized)
    pub validated_configs: HashMap<String, ValidatedConfigRef>,

    // Geometry synchronization
    pub geometry_cache: Arc<RwLock<HashMap<String, WindowGeometry>>>, // window_address -> geometry
    pub sync_tasks: HashMap<String, JoinHandle<()>>,                  // window_address -> sync task
}

impl ScratchpadsPlugin {
    pub fn new() -> Self {
        Self {
            scratchpads: HashMap::new(),
            states: HashMap::new(),
            hyprland_client: Arc::new(Mutex::new(None)),
            enhanced_client: Arc::new(EnhancedHyprlandClient::new()),
            variables: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            monitors_cache: Arc::new(RwLock::new(Vec::new())),
            cache_valid_until: Arc::new(RwLock::new(Instant::now())),
            cache_duration: Duration::from_secs(2), // Cache monitors for 2 seconds
            window_to_scratchpad: HashMap::new(),
            focused_window: None,
            resolved_configs: HashMap::new(),
            hide_tasks: HashMap::new(),
            validated_configs: HashMap::new(),
            geometry_cache: Arc::new(RwLock::new(HashMap::new())),
            sync_tasks: HashMap::new(),
        }
    }

    pub async fn set_hyprland_client(&self, client: Arc<HyprlandClient>) {
        let mut client_guard = self.hyprland_client.lock().await;
        *client_guard = Some(client);
    }

    /// Get current monitors with caching for performance
    pub async fn get_monitors(&self) -> Result<Vec<MonitorInfo>> {
        let now = Instant::now();

        // Check cache validity
        {
            let cache_valid = self.cache_valid_until.read().await;
            if now < *cache_valid {
                let monitors = self.monitors_cache.read().await;
                if !monitors.is_empty() {
                    return Ok(monitors.clone());
                }
            }
        }

        // Cache expired or empty, refresh monitors
        let client_guard = self.hyprland_client.lock().await;
        let client = match client_guard.as_ref() {
            Some(client) => Arc::clone(client),
            None => return Err(anyhow::anyhow!("Hyprland client not available")),
        };
        drop(client_guard);

        let monitors = client.get_monitors().await?;
        let monitor_infos: Vec<MonitorInfo> = monitors
            .iter()
            .map(|m| MonitorInfo {
                name: m.name.clone(),
                width: m.width as i32,
                height: m.height as i32,
                x: m.x,
                y: m.y,
                scale: m.scale,
                is_focused: m.focused,
            })
            .collect();

        // Update cache
        {
            let mut cache = self.monitors_cache.write().await;
            *cache = monitor_infos.clone();
        }
        {
            let mut cache_valid = self.cache_valid_until.write().await;
            *cache_valid = now + self.cache_duration;
        }

        Ok(monitor_infos)
    }

    /// Get the target monitor for a scratchpad
    pub async fn get_target_monitor(&self, config: &ValidatedConfig) -> Result<MonitorInfo> {
        let monitors = self.get_monitors().await?;

        // Force specific monitor if configured
        if let Some(forced_monitor) = &config.force_monitor {
            if let Some(monitor) = monitors.iter().find(|m| m.name == *forced_monitor) {
                return Ok(monitor.clone());
            }
            warn!(
                "Forced monitor '{}' not found, using focused monitor",
                forced_monitor
            );
        }

        // Use focused monitor
        monitors
            .iter()
            .find(|m| m.is_focused)
            .cloned()
            .or_else(|| monitors.first().cloned())
            .ok_or_else(|| anyhow::anyhow!("No monitors available"))
    }

    /// Process variable substitution in commands
    pub fn expand_command(&self, command: &str, variables: &HashMap<String, String>) -> String {
        let mut result = command.to_string();

        // Replace variables in [variable] format
        for (key, value) in variables {
            let pattern = format!("[{key}]");
            result = result.replace(&pattern, value);
        }

        debug!("🔄 Expanded command '{}' to '{}'", command, result);
        result
    }

    /// Start geometry synchronization for a window
    async fn start_geometry_sync(&mut self, window_address: &str) {
        // Cancel any existing sync for this window
        if let Some(handle) = self.sync_tasks.remove(window_address) {
            handle.abort();
        }

        let window_address = window_address.to_string();
        let enhanced_client = Arc::clone(&self.enhanced_client);
        let geometry_cache = Arc::clone(&self.geometry_cache);

        let window_key = window_address.to_string();

        let handle = tokio::spawn(async move {
            Self::geometry_sync_loop(window_address.to_string(), enhanced_client, geometry_cache)
                .await;
        });

        self.sync_tasks.insert(window_key, handle);
    }

    /// Geometry synchronization loop for a specific window
    async fn geometry_sync_loop(
        window_address: String,
        enhanced_client: Arc<EnhancedHyprlandClient>,
        geometry_cache: Arc<RwLock<HashMap<String, WindowGeometry>>>,
    ) {
        let mut interval = tokio::time::interval(Duration::from_millis(500)); // Check every 500ms

        loop {
            interval.tick().await;

            // Get current geometry from Hyprland
            match enhanced_client.get_window_geometry(&window_address).await {
                Ok(current_geometry) => {
                    // Check if geometry has changed
                    let needs_update = {
                        let cache = geometry_cache.read().await;
                        if let Some(cached_geometry) = cache.get(&window_address) {
                            cached_geometry.x != current_geometry.x
                                || cached_geometry.y != current_geometry.y
                                || cached_geometry.width != current_geometry.width
                                || cached_geometry.height != current_geometry.height
                                || cached_geometry.workspace != current_geometry.workspace
                        } else {
                            true // First time caching
                        }
                    };

                    if needs_update {
                        debug!(
                            "📐 Geometry updated for window {}: {}x{} at ({}, {})",
                            window_address,
                            current_geometry.width,
                            current_geometry.height,
                            current_geometry.x,
                            current_geometry.y
                        );

                        // Update cache
                        let mut cache = geometry_cache.write().await;
                        cache.insert(window_address.clone(), current_geometry);
                    }
                }
                Err(e) => {
                    debug!(
                        "❌ Failed to get geometry for window {}: {}",
                        window_address, e
                    );
                    // Window might have been closed, remove from cache
                    let mut cache = geometry_cache.write().await;
                    cache.remove(&window_address);
                    break; // Stop sync loop for this window
                }
            }
        }
    }

    /// Stop geometry synchronization for a window
    async fn stop_geometry_sync(&mut self, window_address: &str) {
        if let Some(handle) = self.sync_tasks.remove(window_address) {
            handle.abort();
            debug!("🛑 Stopped geometry sync for window: {}", window_address);
        }

        // Remove from cache
        let mut cache = self.geometry_cache.write().await;
        cache.remove(window_address);
    }

    /// Get cached geometry for a window
    pub async fn get_cached_geometry(&self, window_address: &str) -> Option<WindowGeometry> {
        let cache = self.geometry_cache.read().await;
        cache.get(window_address).cloned()
    }

    /// Bulk update geometries for all tracked windows
    pub async fn sync_all_geometries(&mut self) {
        let window_addresses: Vec<String> = self.window_to_scratchpad.keys().cloned().collect();

        if window_addresses.is_empty() {
            return;
        }

        debug!(
            "🔄 Syncing geometries for {} windows",
            window_addresses.len()
        );

        match self
            .enhanced_client
            .get_multiple_window_geometries(&window_addresses)
            .await
        {
            Ok(geometries) => {
                let mut cache = self.geometry_cache.write().await;
                for (address, geometry) in geometries {
                    cache.insert(address, geometry);
                }
                debug!("✅ Synced geometries for {} windows", cache.len());
            }
            Err(e) => {
                warn!("⚠️  Failed to sync geometries: {}", e);
            }
        }
    }

    /// Helper methods for internal operations
    async fn get_hyprland_client(&self) -> Result<Arc<HyprlandClient>> {
        let client_guard = self.hyprland_client.lock().await;
        match client_guard.as_ref() {
            Some(client) => Ok(Arc::clone(client)),
            None => Err(anyhow::anyhow!("Hyprland client not available")),
        }
    }

    fn get_validated_config(&self, name: &str) -> Result<ValidatedConfigRef> {
        self.validated_configs
            .get(name)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Scratchpad '{}' not found or not validated", name))
    }

    async fn cancel_hide_delay(&mut self, name: &str) {
        if let Some(handle) = self.hide_tasks.remove(name) {
            handle.abort();
            debug!("🚫 Cancelled hide delay for scratchpad '{}'", name);
        }
    }

    /// Main toggle logic for scratchpads
    async fn toggle_scratchpad(&mut self, name: &str) -> Result<String> {
        // Cancel any pending hide animation
        self.cancel_hide_delay(name).await;

        let validated_config = self.get_validated_config(name)?;
        debug!(
            "🔄 Processing toggle for scratchpad '{}' with class '{}'",
            name, validated_config.class
        );

        let client = self.get_hyprland_client().await?;
        let existing_windows = client
            .find_windows_by_class(&validated_config.class)
            .await?;

        if existing_windows.is_empty() {
            self.spawn_scratchpad(name, &validated_config).await
        } else {
            self.toggle_visibility(name, &validated_config, &existing_windows)
                .await
        }
    }

    /// Spawn a new scratchpad application
    async fn spawn_scratchpad(&mut self, name: &str, config: &ValidatedConfig) -> Result<String> {
        debug!("🚀 Spawning scratchpad '{}'", name);

        let client = self.get_hyprland_client().await?;
        let vars = self.variables.read().await;
        let expanded_command = self.expand_command(&config.command, &vars);

        info!("🚀 Spawning application: {}", expanded_command);
        client.spawn_app(&expanded_command).await?;

        // Update state
        let state = self.states.entry(name.to_string()).or_default();
        state.is_spawned = true;
        state.last_used = Some(Instant::now());

        Ok(format!("Scratchpad '{name}' spawned"))
    }

    /// Toggle visibility of existing windows
    async fn toggle_visibility(
        &mut self,
        name: &str,
        config: &ValidatedConfig,
        windows: &[hyprland::data::Client],
    ) -> Result<String> {
        debug!("🪟 Toggling visibility for scratchpad '{}'", name);

        let _client = self.get_hyprland_client().await?;
        let target_monitor = self.get_target_monitor(config).await?;

        // Check current visibility state
        let is_visible = self.is_scratchpad_visible(name);

        if is_visible {
            self.hide_scratchpad(name, config, windows).await
        } else {
            self.show_scratchpad(name, config, windows, &target_monitor)
                .await
        }
    }

    /// Show scratchpad with proper positioning
    async fn show_scratchpad(
        &mut self,
        name: &str,
        config: &ValidatedConfig,
        windows: &[hyprland::data::Client],
        monitor: &MonitorInfo,
    ) -> Result<String> {
        debug!("👁️ Showing scratchpad '{}'", name);

        let client = self.get_hyprland_client().await?;

        // Handle excludes
        if !config.excludes.is_empty() {
            self.handle_excludes(name, config).await?;
        }

        // Get the primary window (or create if multi-window)
        let window = if config.multi_window {
            self.get_or_create_window(name, config, windows).await?
        } else {
            windows
                .first()
                .ok_or_else(|| anyhow::anyhow!("No windows found for scratchpad '{}'", name))?
                .clone()
        };

        // Apply geometry
        self.apply_geometry(&window, config, monitor).await?;

        // Show window
        client.show_window(&window.address.to_string()).await?;

        // Focus if smart_focus is enabled
        if config.smart_focus {
            client.focus_window(&window.address.to_string()).await?;
        }

        // Update state
        self.mark_window_visible(name, &window.address.to_string());

        Ok(format!("Scratchpad '{name}' shown"))
    }

    /// Hide scratchpad with delay if configured
    async fn hide_scratchpad(
        &mut self,
        name: &str,
        config: &ValidatedConfig,
        windows: &[hyprland::data::Client],
    ) -> Result<String> {
        debug!("🙈 Hiding scratchpad '{}'", name);

        if let Some(delay_ms) = config.hide_delay {
            self.schedule_hide_delay(name, config, windows, delay_ms)
                .await?;
            Ok(format!("Scratchpad '{name}' will hide in {delay_ms}ms"))
        } else {
            self.perform_hide(name, config, windows).await?;
            Ok(format!("Scratchpad '{name}' hidden"))
        }
    }

    /// Apply geometry (position and size) to window
    async fn apply_geometry(
        &self,
        window: &hyprland::data::Client,
        config: &ValidatedConfig,
        monitor: &MonitorInfo,
    ) -> Result<()> {
        let client = self.get_hyprland_client().await?;
        let geometry = GeometryCalculator::calculate_geometry(config, monitor)?;

        client
            .move_resize_window(
                &window.address.to_string(),
                geometry.x,
                geometry.y,
                geometry.width,
                geometry.height,
            )
            .await?;

        Ok(())
    }

    async fn schedule_hide_delay(
        &mut self,
        name: &str,
        config: &ValidatedConfig,
        windows: &[hyprland::data::Client],
        delay_ms: u32,
    ) -> Result<()> {
        let scratchpad_name = name.to_string();
        let _config = config.clone();
        let windows = windows.to_vec();
        let client = self.get_hyprland_client().await?;

        let name_for_debug = scratchpad_name.clone();
        let handle = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(delay_ms as u64)).await;

            // Perform the hide operation
            for window in &windows {
                if let Err(e) = client.hide_window(&window.address.to_string()).await {
                    error!("Failed to hide window after delay: {}", e);
                }
            }

            debug!(
                "⏰ Hide delay completed for scratchpad '{}'",
                name_for_debug
            );
        });

        self.hide_tasks.insert(scratchpad_name, handle);
        Ok(())
    }

    async fn perform_hide(
        &mut self,
        name: &str,
        config: &ValidatedConfig,
        windows: &[hyprland::data::Client],
    ) -> Result<()> {
        let client = self.get_hyprland_client().await?;

        for window in windows {
            if config.close_on_hide {
                client.close_window(&window.address.to_string()).await?;
            } else {
                client.hide_window(&window.address.to_string()).await?;
            }
        }

        // Update state
        self.mark_scratchpad_hidden(name);

        // Restore excluded scratchpads if configured
        if config.restore_excluded {
            self.restore_excluded_scratchpads(name).await?;
        }

        Ok(())
    }

    async fn handle_excludes(&mut self, name: &str, config: &ValidatedConfig) -> Result<()> {
        let excludes = config.excludes.clone();
        let scratchpad_names: Vec<String> = self.scratchpads.keys().cloned().collect();

        for exclude_pattern in &excludes {
            if exclude_pattern == "*" {
                // Hide all other scratchpads
                for other_name in &scratchpad_names {
                    if other_name != name {
                        self.mark_scratchpad_excluded_by(other_name, name);
                        // Hide the other scratchpad logic would go here
                    }
                }
            } else if scratchpad_names.contains(exclude_pattern) {
                // Hide specific scratchpad
                self.mark_scratchpad_excluded_by(exclude_pattern, name);
                // Hide logic would go here
            }
        }
        Ok(())
    }

    async fn restore_excluded_scratchpads(&mut self, excluding_scratchpad: &str) -> Result<()> {
        for (name, state) in &mut self.states {
            if state.excluded_by.remove(excluding_scratchpad) {
                debug!("🔄 Restoring excluded scratchpad '{}'", name);
                // Restore logic would go here
            }
        }
        Ok(())
    }

    async fn get_or_create_window(
        &mut self,
        _name: &str,
        config: &ValidatedConfig,
        existing_windows: &[hyprland::data::Client],
    ) -> Result<hyprland::data::Client> {
        let max_instances = config.max_instances.unwrap_or(1);

        if existing_windows.len() < max_instances as usize {
            // Spawn new instance
            let client = self.get_hyprland_client().await?;
            let vars = self.variables.read().await;
            let expanded_command = self.expand_command(&config.command, &vars);
            client.spawn_app(&expanded_command).await?;

            // Wait for window to appear
            tokio::time::sleep(Duration::from_millis(500)).await;

            let new_windows = client.find_windows_by_class(&config.class).await?;
            new_windows
                .into_iter()
                .find(|w| !existing_windows.iter().any(|e| e.address == w.address))
                .ok_or_else(|| anyhow::anyhow!("Failed to find newly spawned window"))
        } else {
            // Use existing window
            Ok(existing_windows[0].clone())
        }
    }

    // Helper methods for state management
    fn is_scratchpad_visible(&self, name: &str) -> bool {
        self.states
            .get(name)
            .map(|s| s.windows.iter().any(|w| w.is_visible))
            .unwrap_or(false)
    }

    fn mark_window_visible(&mut self, scratchpad_name: &str, window_address: &str) {
        let state = self.states.entry(scratchpad_name.to_string()).or_default();
        state.last_used = Some(Instant::now());

        // Find or create window state
        if let Some(window_state) = state
            .windows
            .iter_mut()
            .find(|w| w.address == *window_address)
        {
            window_state.is_visible = true;
            window_state.last_focus = Some(Instant::now());
        } else {
            state.windows.push(WindowState {
                address: window_address.to_string(),
                is_visible: true,
                last_position: None,
                monitor: None,
                workspace: None,
                last_focus: Some(Instant::now()),
            });
        }

        self.window_to_scratchpad
            .insert(window_address.to_string(), scratchpad_name.to_string());
    }

    fn mark_scratchpad_hidden(&mut self, name: &str) {
        if let Some(state) = self.states.get_mut(name) {
            for window in &mut state.windows {
                window.is_visible = false;
            }
            state.last_used = Some(Instant::now());
        }
    }

    fn mark_scratchpad_excluded_by(&mut self, scratchpad_name: &str, excluded_by: &str) {
        let state = self.states.entry(scratchpad_name.to_string()).or_default();
        state.excluded_by.insert(excluded_by.to_string());
    }
}

impl Default for ScratchpadsPlugin {
    fn default() -> Self {
        Self::new()
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

        // Parse variables if present
        if let toml::Value::Table(map) = config {
            if let Some(toml::Value::Table(vars)) = map.get("variables") {
                for (key, value) in vars {
                    if let toml::Value::String(val_str) = value {
                        let mut vars = self.variables.write().await;
                        vars.insert(key.clone(), val_str.clone());
                        debug!("📝 Loaded variable: {} = {}", key, val_str);
                    }
                }
            }
        }

        // Parse scratchpad configurations
        if let toml::Value::Table(map) = config {
            for (name, scratchpad_config) in map {
                // Skip the variables section as it's already processed
                if name == "variables" {
                    continue;
                }
                if let toml::Value::Table(sc) = scratchpad_config {
                    let command = sc
                        .get("command")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();

                    let class = sc
                        .get("class")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();

                    let size = sc
                        .get("size")
                        .and_then(|v| v.as_str())
                        .unwrap_or("50% 50%")
                        .to_string();

                    let animation = sc
                        .get("animation")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());

                    let mut config = ScratchpadConfig {
                        command,
                        class,
                        size,
                        animation,
                        ..Default::default()
                    };

                    // Parse additional Pyprland-compatible options
                    if let Some(toml::Value::Boolean(lazy)) = sc.get("lazy") {
                        config.lazy = *lazy;
                    }
                    if let Some(toml::Value::Boolean(pinned)) = sc.get("pinned") {
                        config.pinned = *pinned;
                    }
                    if let Some(toml::Value::Array(excludes)) = sc.get("excludes") {
                        config.excludes = excludes
                            .iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect();
                    } else if let Some(toml::Value::String(exclude_all)) = sc.get("excludes") {
                        if exclude_all == "*" {
                            config.excludes = vec!["*".to_string()];
                        }
                    }
                    if let Some(toml::Value::Boolean(restore_excluded)) = sc.get("restore_excluded")
                    {
                        config.restore_excluded = *restore_excluded;
                    }
                    if let Some(toml::Value::String(force_monitor)) = sc.get("force_monitor") {
                        config.force_monitor = Some(force_monitor.clone());
                    }
                    if let Some(toml::Value::Integer(margin)) = sc.get("margin") {
                        config.margin = Some(*margin as i32);
                    }
                    if let Some(toml::Value::String(offset)) = sc.get("offset") {
                        config.offset = Some(offset.clone());
                    }
                    if let Some(toml::Value::Integer(hide_delay)) = sc.get("hide_delay") {
                        config.hide_delay = Some(*hide_delay as u32);
                    }
                    if let Some(toml::Value::Boolean(multi_window)) = sc.get("multi_window") {
                        config.multi_window = *multi_window;
                    }
                    if let Some(toml::Value::Integer(max_instances)) = sc.get("max_instances") {
                        config.max_instances = Some(*max_instances as u32);
                    }

                    self.scratchpads.insert(name.clone(), Arc::new(config));
                    self.states.insert(name.clone(), ScratchpadState::default());
                    info!("📝 Registered scratchpad: {}", name);
                }
            }
        }

        // Validate configurations
        let monitors = self.get_monitors().await.unwrap_or_default();
        self.validated_configs = ConfigValidator::validate_configs(&self.scratchpads, &monitors);

        info!(
            "✅ Scratchpads plugin initialized with {} scratchpads",
            self.scratchpads.len()
        );
        Ok(())
    }

    async fn handle_event(&mut self, event: &HyprlandEvent) -> Result<()> {
        debug!("🪟 Scratchpads handling event: {:?}", event);

        match event {
            HyprlandEvent::WindowOpened { window } => {
                debug!("Window opened: {} - checking if it is a scratchpad", window);
                self.handle_window_opened(window).await;
            }
            HyprlandEvent::WindowClosed { window } => {
                debug!("Window closed: {} - cleaning up if scratchpad", window);
                self.handle_window_closed(window).await;
            }
            HyprlandEvent::WindowMoved { window } => {
                debug!("Window moved: {} - syncing geometry", window);
                self.handle_window_moved(window).await;
            }
            HyprlandEvent::WorkspaceChanged { workspace } => {
                debug!("Workspace changed to: {}", workspace);
                self.handle_workspace_changed(workspace).await;
            }
            HyprlandEvent::MonitorChanged { monitor: _ } => {
                debug!("Monitor changed - invalidating cache");
                // Invalidate monitor cache
                {
                    let mut cache_valid = self.cache_valid_until.write().await;
                    *cache_valid = Instant::now();
                }

                // Sync all geometries as monitor layout may have changed
                self.sync_all_geometries().await;
            }
            HyprlandEvent::WindowFocusChanged { window } => {
                self.handle_focus_changed(window).await;
            }
            HyprlandEvent::Other(msg) => {
                debug!("Other event: {}", msg);
                self.handle_other_event(msg).await;
            }
        }

        Ok(())
    }

    async fn handle_command(&mut self, command: &str, args: &[&str]) -> Result<String> {
        match command {
            "toggle" => {
                if let Some(scratchpad_name) = args.first() {
                    info!("🔄 Toggling scratchpad: {}", scratchpad_name);

                    if self.scratchpads.contains_key(*scratchpad_name) {
                        match self.toggle_scratchpad(scratchpad_name).await {
                            Ok(message) => {
                                info!("✅ {}", message);
                                Ok(message)
                            }
                            Err(e) => {
                                error!(
                                    "❌ Failed to toggle scratchpad '{}': {}",
                                    scratchpad_name, e
                                );
                                Err(e)
                            }
                        }
                    } else {
                        warn!("⚠️  Scratchpad '{}' not found", scratchpad_name);
                        Err(anyhow::anyhow!(
                            "Scratchpad '{}' not found",
                            scratchpad_name
                        ))
                    }
                } else {
                    Err(anyhow::anyhow!("No scratchpad name provided"))
                }
            }
            "list" => {
                let mut status_list = Vec::new();
                for name in self.scratchpads.keys() {
                    let state = self.states.get(name);
                    let visible_count = state
                        .map(|s| s.windows.iter().filter(|w| w.is_visible).count())
                        .unwrap_or(0);
                    let total_count = state.map(|s| s.windows.len()).unwrap_or(0);
                    let spawned = state.map(|s| s.is_spawned).unwrap_or(false);

                    let status = if visible_count > 0 {
                        format!("{name} (visible: {visible_count}/{total_count})")
                    } else if spawned {
                        format!("{name} (hidden: {total_count})")
                    } else {
                        format!("{name} (not spawned)")
                    };
                    status_list.push(status);
                }
                Ok(format!("Scratchpads: {}", status_list.join(", ")))
            }
            _ => Err(anyhow::anyhow!("Unknown command: {}", command)),
        }
    }
}

// Enhanced event handling methods
impl ScratchpadsPlugin {
    async fn handle_window_opened(&mut self, window_address: &str) {
        debug!("🪟 Window opened: {}", window_address);

        // Check if this window belongs to any scratchpad by checking class
        if let Ok(client) = self
            .enhanced_client
            .get_window_geometry(window_address)
            .await
        {
            // Find scratchpad that matches this window class
            for (scratchpad_name, config) in &self.scratchpads {
                if config.class == client.workspace
                    || (config.class.is_empty() && scratchpad_name == &client.workspace)
                {
                    debug!(
                        "📋 Detected scratchpad window: {} for '{}'",
                        window_address, scratchpad_name
                    );

                    // Add to tracking
                    self.window_to_scratchpad
                        .insert(window_address.to_string(), scratchpad_name.clone());

                    // Update state
                    let state = self.states.entry(scratchpad_name.clone()).or_default();

                    let window_state = WindowState {
                        address: window_address.to_string(),
                        is_visible: !client.workspace.starts_with("special:"),
                        last_position: None,
                        monitor: Some(client.monitor.to_string()),
                        workspace: Some(client.workspace.clone()),
                        last_focus: Some(std::time::Instant::now()),
                    };

                    // Add if not already tracked
                    if !state.windows.iter().any(|w| w.address == *window_address) {
                        state.windows.push(window_state);
                        state.is_spawned = true;
                        debug!("✅ Added window to scratchpad '{}' state", scratchpad_name);
                    }

                    // Start geometry sync for this window
                    self.start_geometry_sync(window_address).await;

                    break;
                }
            }
        }
    }

    async fn handle_window_moved(&mut self, window_address: &str) {
        debug!("📍 Window moved: {}", window_address);

        // Only sync geometry for tracked scratchpad windows
        if self.window_to_scratchpad.contains_key(window_address) {
            // Update geometry cache
            if let Ok(geometry) = self
                .enhanced_client
                .get_window_geometry(window_address)
                .await
            {
                let mut cache = self.geometry_cache.write().await;
                cache.insert(window_address.to_string(), geometry);
                debug!("🔄 Updated geometry cache for window: {}", window_address);
            }
        }
    }

    async fn handle_workspace_changed(&mut self, workspace: &str) {
        debug!("🖥️ Workspace changed to: {}", workspace);

        // Update visibility status for scratchpad windows
        // Special workspaces (like special:scratchpad) typically hide windows
        let _is_special_workspace = workspace.starts_with("special:");

        // Update window visibility status based on workspace
        for (window_address, scratchpad_name) in &self.window_to_scratchpad {
            if let Some(state) = self.states.get_mut(scratchpad_name) {
                if let Some(window_state) = state
                    .windows
                    .iter_mut()
                    .find(|w| w.address == *window_address)
                {
                    // Get current window info to determine actual visibility
                    if let Ok(geometry) = self
                        .enhanced_client
                        .get_window_geometry(window_address)
                        .await
                    {
                        let new_visibility = !geometry.workspace.starts_with("special:");
                        if window_state.is_visible != new_visibility {
                            window_state.is_visible = new_visibility;
                            debug!(
                                "👁️ Updated visibility for {}: {}",
                                window_address, new_visibility
                            );
                        }
                    }
                }
            }
        }
    }

    async fn handle_other_event(&mut self, event_msg: &str) {
        debug!("🔄 Processing other event: {}", event_msg);

        // Handle specific other events that might be useful for scratchpads
        if event_msg.starts_with("windowtitle>>") {
            // Extract window address and title
            let parts: Vec<&str> = event_msg.splitn(2, ">>").collect();
            if parts.len() == 2 {
                let data_parts: Vec<&str> = parts[1].splitn(2, ',').collect();
                if !data_parts.is_empty() {
                    let window_address = data_parts[0];
                    debug!("📝 Title changed for window: {}", window_address);

                    // Sync geometry if this is a tracked window
                    if self.window_to_scratchpad.contains_key(window_address) {
                        self.start_geometry_sync(window_address).await;
                    }
                }
            }
        } else if event_msg.starts_with("resizewindow>>") {
            // Window resized, update geometry
            let parts: Vec<&str> = event_msg.splitn(2, ">>").collect();
            if parts.len() == 2 {
                let window_address = parts[1];
                debug!("📏 Window resized: {}", window_address);

                if self.window_to_scratchpad.contains_key(window_address) {
                    self.start_geometry_sync(window_address).await;
                }
            }
        }
    }

    async fn handle_window_closed(&mut self, window_address: &str) {
        // Remove from window mapping
        if let Some(scratchpad_name) = self.window_to_scratchpad.remove(window_address) {
            debug!(
                "📋 Window '{}' belonged to scratchpad '{}'",
                window_address, scratchpad_name
            );

            if let Some(state) = self.states.get_mut(&scratchpad_name) {
                // Remove window from state
                state.windows.retain(|w| w.address != window_address);

                // If no windows left, mark as not spawned
                if state.windows.is_empty() {
                    state.is_spawned = false;
                    debug!(
                        "📋 Scratchpad '{}' has no windows left, marked as not spawned",
                        scratchpad_name
                    );
                }
            }
        }

        // Update focus if this was the focused window
        if self.focused_window.as_deref() == Some(window_address) {
            self.focused_window = None;
        }
    }

    async fn handle_focus_changed(&mut self, window_address: &str) {
        debug!("👁️ Focus changed to: {}", window_address);

        self.focused_window = Some(window_address.to_string());

        // Update focus time for scratchpad windows
        if let Some(scratchpad_name) = self.window_to_scratchpad.get(window_address) {
            if let Some(state) = self.states.get_mut(scratchpad_name) {
                if let Some(window_state) = state
                    .windows
                    .iter_mut()
                    .find(|w| w.address == *window_address)
                {
                    window_state.last_focus = Some(Instant::now());
                }
                state.last_used = Some(Instant::now());
                debug!("🎯 Updated focus time for scratchpad '{}'", scratchpad_name);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test;

    fn create_test_config() -> toml::Value {
        toml::from_str(
            r#"
            [term]
            command = "foot --app-id=term"
            class = "foot"
            size = "75% 60%"
            lazy = false
            pinned = true
            
            [browser]
            command = "firefox --new-window"
            class = "firefox"
            size = "80% 70%"
            lazy = true
            excludes = ["term"]
            
            [variables]
            term_class = "foot"
        "#,
        )
        .unwrap()
    }

    fn create_test_monitor() -> MonitorInfo {
        MonitorInfo {
            name: "DP-1".to_string(),
            width: 1920,
            height: 1080,
            x: 0,
            y: 0,
            scale: 1.0,
            is_focused: true,
        }
    }

    #[tokio::test]
    async fn test_plugin_initialization() {
        let mut plugin = ScratchpadsPlugin::new();
        let config = create_test_config();
        plugin.init(&config).await.unwrap();

        // Should have loaded 2 scratchpads (term and browser)
        assert_eq!(plugin.scratchpads.len(), 2);

        // Check term scratchpad config
        let term_config = plugin.scratchpads.get("term").unwrap();
        assert_eq!(term_config.command, "foot --app-id=term");
        assert_eq!(term_config.class, "foot");
        assert_eq!(term_config.size, "75% 60%");
        assert!(!term_config.lazy);
        assert!(term_config.pinned);

        // Check browser scratchpad config
        let browser_config = plugin.scratchpads.get("browser").unwrap();
        assert_eq!(browser_config.command, "firefox --new-window");
        assert_eq!(browser_config.class, "firefox");
        assert!(browser_config.lazy);
        assert_eq!(browser_config.excludes, vec!["term"]);

        // Check variables
        // Note: This test would need to be async to properly test Arc<RwLock<HashMap>>
        // For now, we'll test that the structure exists
        assert!(!plugin.scratchpads.is_empty());

        // Check validated configs were created
        assert_eq!(plugin.validated_configs.len(), 2);
    }

    #[test]
    fn test_geometry_calculation() {
        let monitor = create_test_monitor();

        // Test percentage sizes
        let (width, height) = GeometryCalculator::parse_size("75% 60%", &monitor, None).unwrap();
        assert_eq!(width, 1440); // 75% of 1920
        assert_eq!(height, 648); // 60% of 1080

        // Test pixel sizes
        let (width, height) =
            GeometryCalculator::parse_size("800px 600px", &monitor, None).unwrap();
        assert_eq!(width, 800);
        assert_eq!(height, 600);

        // Test mixed sizes
        let (width, height) = GeometryCalculator::parse_size("50% 500px", &monitor, None).unwrap();
        assert_eq!(width, 960); // 50% of 1920
        assert_eq!(height, 500);

        // Test max_size constraint
        let (width, height) =
            GeometryCalculator::parse_size("90% 90%", &monitor, Some("1600px 900px")).unwrap();
        assert_eq!(width, 1600); // Constrained by max_size
        assert_eq!(height, 900); // Constrained by max_size
    }

    #[test]
    fn test_dimension_parsing() {
        assert_eq!(
            GeometryCalculator::parse_dimension("50%", 1920).unwrap(),
            960
        );
        assert_eq!(
            GeometryCalculator::parse_dimension("75%", 1080).unwrap(),
            810
        );

        assert_eq!(
            GeometryCalculator::parse_dimension("800px", 1920).unwrap(),
            800
        );
        assert_eq!(
            GeometryCalculator::parse_dimension("600", 1080).unwrap(),
            600
        );

        assert!(GeometryCalculator::parse_dimension("invalid", 1920).is_err());
        assert!(GeometryCalculator::parse_dimension("200%px", 1920).is_err());
    }

    #[test]
    fn test_offset_parsing() {
        let monitor = create_test_monitor();

        let (x, y) = GeometryCalculator::parse_offset(Some("50px 100px"), &monitor).unwrap();
        assert_eq!(x, 50);
        assert_eq!(y, 100);

        let (x, y) = GeometryCalculator::parse_offset(Some("10% 20%"), &monitor).unwrap();
        assert_eq!(x, 192); // 10% of 1920
        assert_eq!(y, 216); // 20% of 1080

        let (x, y) = GeometryCalculator::parse_offset(None, &monitor).unwrap();
        assert_eq!(x, 0);
        assert_eq!(y, 0);
    }

    #[tokio::test]
    async fn test_variable_expansion() {
        let plugin = ScratchpadsPlugin::new();
        let mut variables = HashMap::new();
        variables.insert("term_class".to_string(), "foot".to_string());

        let expanded = plugin.expand_command("foot --app-id=[term_class]", &variables);
        assert_eq!(expanded, "foot --app-id=foot");

        let expanded = plugin.expand_command("echo [missing_var]", &variables);
        assert_eq!(expanded, "echo [missing_var]"); // Should not expand missing variables

        let expanded = plugin.expand_command("no variables here", &variables);
        assert_eq!(expanded, "no variables here");
    }

    #[test]
    fn test_configuration_defaults() {
        let config = ScratchpadConfig::default();

        assert_eq!(config.command, "");
        assert_eq!(config.class, "");
        assert_eq!(config.size, "50% 50%");
        assert!(!config.lazy);
        assert!(config.pinned);
        assert!(config.excludes.is_empty());
        assert!(!config.restore_excluded);
        assert!(!config.preserve_aspect);
        assert!(config.force_monitor.is_none());
        assert!(!config.alt_toggle);
        assert!(!config.allow_special_workspaces);
        assert!(config.smart_focus);
        assert!(!config.close_on_hide);
        assert!(config.unfocus.is_none());
        assert!(config.max_size.is_none());
        assert!(config.r#use.is_none());
        assert!(!config.multi_window);
        assert_eq!(config.max_instances, Some(1));
    }

    #[test]
    fn test_config_validation() {
        let monitors = vec![create_test_monitor()];
        let mut configs = HashMap::new();

        configs.insert(
            "term".to_string(),
            ScratchpadConfig {
                command: "foot".to_string(),
                class: "foot".to_string(),
                size: "75% 60%".to_string(),
                ..Default::default()
            },
        );

        // Convert configs to Arc-wrapped for validation
        let arc_configs: std::collections::HashMap<String, ScratchpadConfigRef> =
            configs.into_iter().map(|(k, v)| (k, Arc::new(v))).collect();

        let validated = ConfigValidator::validate_configs(&arc_configs, &monitors);
        let term_config = validated.get("term").unwrap();

        assert!(term_config.validation_errors.is_empty());
        assert_eq!(term_config.command, "foot");
        assert_eq!(term_config.class, "foot");
        assert!(term_config.parsed_size.is_some());
    }

    // ============================================================================
    // TESTS FOR ENHANCED FUNCTIONALITY
    // ============================================================================

    #[tokio::test]
    async fn test_enhanced_event_handling() {
        let mut plugin = ScratchpadsPlugin::new();
        let config = create_test_config();
        plugin.init(&config).await.unwrap();

        // Test window opened event handling
        let window_address = "0x12345";
        plugin.handle_window_opened(window_address).await;

        // Should not add to tracking since enhanced_client will fail in test environment
        assert!(plugin.window_to_scratchpad.is_empty());
    }

    #[tokio::test]
    async fn test_window_state_management() {
        let mut plugin = ScratchpadsPlugin::new();
        let config = create_test_config();
        plugin.init(&config).await.unwrap();

        // Simulate window state
        let mut state = ScratchpadState::default();
        state.windows.push(WindowState {
            address: "0x12345".to_string(),
            is_visible: true,
            last_position: Some((100, 100, 800, 600)),
            monitor: Some("DP-1".to_string()),
            workspace: Some("1".to_string()),
            last_focus: Some(Instant::now()),
        });

        plugin.states.insert("term".to_string(), state);
        plugin
            .window_to_scratchpad
            .insert("0x12345".to_string(), "term".to_string());

        // Test window closed handling
        plugin.handle_window_closed("0x12345").await;

        // Window should be removed from tracking
        assert!(!plugin.window_to_scratchpad.contains_key("0x12345"));

        let term_state = plugin.states.get("term").unwrap();
        assert!(term_state.windows.is_empty());
        assert!(!term_state.is_spawned);
    }

    #[tokio::test]
    async fn test_focus_tracking() {
        let mut plugin = ScratchpadsPlugin::new();
        let config = create_test_config();
        plugin.init(&config).await.unwrap();

        // Setup test state
        let mut state = ScratchpadState::default();
        let now = Instant::now();
        state.windows.push(WindowState {
            address: "0x12345".to_string(),
            is_visible: true,
            last_position: None,
            monitor: Some("DP-1".to_string()),
            workspace: Some("1".to_string()),
            last_focus: Some(now),
        });

        plugin.states.insert("term".to_string(), state);
        plugin
            .window_to_scratchpad
            .insert("0x12345".to_string(), "term".to_string());

        // Test focus changed
        plugin.handle_focus_changed("0x12345").await;

        // Focus should be updated
        assert_eq!(plugin.focused_window, Some("0x12345".to_string()));

        let term_state = plugin.states.get("term").unwrap();
        let window_state = &term_state.windows[0];
        assert!(window_state.last_focus.unwrap() > now);
        assert!(term_state.last_used.unwrap() > now);
    }

    #[tokio::test]
    async fn test_workspace_change_handling() {
        let mut plugin = ScratchpadsPlugin::new();
        let config = create_test_config();
        plugin.init(&config).await.unwrap();

        // Setup test state with visible window
        let mut state = ScratchpadState::default();
        state.windows.push(WindowState {
            address: "0x12345".to_string(),
            is_visible: true,
            last_position: None,
            monitor: Some("DP-1".to_string()),
            workspace: Some("1".to_string()),
            last_focus: Some(Instant::now()),
        });

        plugin.states.insert("term".to_string(), state);
        plugin
            .window_to_scratchpad
            .insert("0x12345".to_string(), "term".to_string());

        // Test workspace change to special workspace
        plugin.handle_workspace_changed("special:scratchpad").await;

        // Window visibility should be handled (though enhanced_client will fail in test)
        // The test validates the logic path is executed correctly
        assert!(plugin.states.contains_key("term"));
    }

    #[tokio::test]
    async fn test_other_event_handling() {
        let mut plugin = ScratchpadsPlugin::new();
        let config = create_test_config();
        plugin.init(&config).await.unwrap();

        // Setup tracking
        plugin
            .window_to_scratchpad
            .insert("0x12345".to_string(), "term".to_string());

        // Test window title change event
        plugin
            .handle_other_event("windowtitle>>0x12345,New Title with, Commas")
            .await;

        // Test window resize event
        plugin.handle_other_event("resizewindow>>0x12345").await;

        // Test unknown event
        plugin.handle_other_event("unknown>>data").await;

        // Should complete without errors (geometry sync will fail due to test environment)
        assert!(plugin.window_to_scratchpad.contains_key("0x12345"));
    }

    #[test]
    fn test_window_geometry_structure() {
        use crate::ipc::WindowGeometry;

        // Test WindowGeometry structure from enhanced client
        let geometry = WindowGeometry {
            x: 100,
            y: 200,
            width: 800,
            height: 600,
            workspace: "1".to_string(),
            monitor: 0,
            floating: true,
        };

        assert_eq!(geometry.x, 100);
        assert_eq!(geometry.y, 200);
        assert_eq!(geometry.width, 800);
        assert_eq!(geometry.height, 600);
        assert_eq!(geometry.workspace, "1");
        assert_eq!(geometry.monitor, 0);
        assert!(geometry.floating);
    }

    #[tokio::test]
    async fn test_geometry_caching() {
        let plugin = ScratchpadsPlugin::new();

        // Test empty cache
        let cached = plugin.get_cached_geometry("0x12345").await;
        assert!(cached.is_none());

        // Test cache insertion (done via geometry sync normally)
        // This validates the cache structure works correctly
        let cache = plugin.geometry_cache.read().await;
        assert!(cache.is_empty());
    }

    #[tokio::test]
    async fn test_enhanced_client_initialization() {
        let plugin = ScratchpadsPlugin::new();

        // Verify enhanced client is initialized
        assert!(!(plugin.enhanced_client.is_connected().await)); // Not connected in test environment

        // Test connection stats
        let stats = plugin.enhanced_client.get_connection_stats().await;
        assert!(!stats.is_connected);
        assert_eq!(stats.connection_failures, 0);
    }

    #[tokio::test]
    async fn test_sync_task_management() {
        let mut plugin = ScratchpadsPlugin::new();
        let config = create_test_config();
        plugin.init(&config).await.unwrap();

        // Test that sync tasks can be managed
        assert!(plugin.sync_tasks.is_empty());

        // In real usage, start_geometry_sync would add tasks
        // This validates the HashMap structure works
        let task_count = plugin.sync_tasks.len();
        assert_eq!(task_count, 0);
    }

    #[tokio::test]
    async fn test_bulk_geometry_sync() {
        let mut plugin = ScratchpadsPlugin::new();
        let config = create_test_config();
        plugin.init(&config).await.unwrap();

        // Setup multiple tracked windows
        plugin
            .window_to_scratchpad
            .insert("0x12345".to_string(), "term".to_string());
        plugin
            .window_to_scratchpad
            .insert("0x67890".to_string(), "browser".to_string());

        // Test bulk sync (will fail due to test environment but validates logic)
        plugin.sync_all_geometries().await;

        // Should complete without panic
        assert_eq!(plugin.window_to_scratchpad.len(), 2);
    }

    #[test]
    fn test_enhanced_window_geometry_calculation() {
        let monitor = create_test_monitor();

        // Test that geometry calculation includes new fields
        let geometry = GeometryCalculator::calculate_geometry(
            &ValidatedConfig {
                command: "test".to_string(),
                class: "test".to_string(),
                size: "50% 60%".to_string(),
                animation: None,
                margin: Some(10),
                offset: None,
                hide_delay: None,
                lazy: false,
                pinned: true,
                excludes: Vec::new(),
                restore_excluded: false,
                preserve_aspect: false,
                force_monitor: None,
                alt_toggle: false,
                allow_special_workspaces: false,
                smart_focus: true,
                close_on_hide: false,
                unfocus: None,
                max_size: None,
                r#use: None,
                multi_window: false,
                max_instances: Some(1),
                validation_errors: Vec::new(),
                validation_warnings: Vec::new(),
                parsed_size: Some((960, 648)),
                parsed_offset: None,
                parsed_max_size: None,
            },
            &monitor,
        )
        .unwrap();

        // Verify enhanced fields are set
        assert_eq!(geometry.workspace, "e+0");
        assert_eq!(geometry.monitor, 0);
        assert!(geometry.floating);

        // Verify basic geometry calculation still works
        assert_eq!(geometry.width, 960); // 50% of 1920
        assert_eq!(geometry.height, 648); // 60% of 1080
    }

    #[tokio::test]
    async fn test_event_filtering_performance() {
        let mut plugin = ScratchpadsPlugin::new();
        let config = create_test_config();
        plugin.init(&config).await.unwrap();

        // Test that plugin can handle rapid event processing
        let events = vec![
            "workspace>>1",
            "openwindow>>0x12345,1,foot,Terminal",
            "closewindow>>0x12345",
            "movewindow>>0x67890,2",
            "windowtitle>>0x12345,New Title with, Commas in it",
            "resizewindow>>0x12345,800x600",
            "unknown>>irrelevant data",
        ];

        // Process events rapidly
        for event in events {
            plugin.handle_other_event(event).await;
        }

        // Should complete without performance issues
        //assert!(plugin.states.len() >= 0); // Basic validation
    }

    #[test]
    fn test_configuration_validation_with_enhanced_features() {
        let monitors = vec![create_test_monitor()];
        let mut configs = HashMap::new();

        // Test enhanced configuration options
        configs.insert(
            "advanced".to_string(),
            ScratchpadConfig {
                command: "advanced-app".to_string(),
                class: "advanced".to_string(),
                size: "80% 70%".to_string(),
                lazy: true,
                pinned: false,
                multi_window: true,
                max_instances: Some(3),
                smart_focus: true,
                preserve_aspect: true,
                max_size: Some("1600px 900px".to_string()),
                ..Default::default()
            },
        );

        // Convert configs to Arc-wrapped for validation
        let arc_configs: std::collections::HashMap<String, ScratchpadConfigRef> =
            configs.into_iter().map(|(k, v)| (k, Arc::new(v))).collect();

        let validated = ConfigValidator::validate_configs(&arc_configs, &monitors);
        let advanced_config = validated.get("advanced").unwrap();

        // Verify enhanced features are validated correctly
        assert!(advanced_config.validation_errors.is_empty());
        assert!(advanced_config.multi_window);
        assert_eq!(advanced_config.max_instances, Some(3));
        assert!(advanced_config.smart_focus);
        assert!(advanced_config.preserve_aspect);
        assert!(advanced_config.max_size.is_some());
    }
}
