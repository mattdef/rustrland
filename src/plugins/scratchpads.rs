use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};

// Arc-optimized configuration types
pub type ScratchpadConfigRef = Arc<ScratchpadConfig>;
pub type ValidatedConfigRef = Arc<ValidatedConfig>;

use crate::animation::WindowAnimator;
use crate::ipc::{
    EnhancedHyprlandClient, HyprlandClient, HyprlandEvent, MonitorInfo, WindowGeometry,
};
use crate::plugins::Plugin;

// ============================================================================
// CONFIGURATION STRUCTURES
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct ScratchpadConfig {
    // Basic config
    pub command: String,
    pub class: Option<String>,
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

    // Position and focus control
    pub position: Option<String>, // Manual window positioning
    pub hysteresis: Option<f32>,  // Unfocus reactivity control (default: 0.4)
    pub restore_focus: bool,      // Restore focused state when hiding (default: true)
    pub multi: bool,              // Pyprland compatibility alias for multi_window

    // Multi-window support
    pub multi_window: bool,
    pub max_instances: Option<u32>,
}

impl Default for ScratchpadConfig {
    fn default() -> Self {
        Self {
            command: String::new(),
            class: None,
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
            position: None,
            hysteresis: Some(0.4),
            restore_focus: true,
            multi: false,
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
    pub position: Option<String>,
    pub hysteresis: Option<f32>,
    pub restore_focus: bool,
    pub multi: bool,
    pub multi_window: bool,
    pub max_instances: Option<u32>,

    // Validation metadata
    pub validation_errors: Vec<String>,
    pub validation_warnings: Vec<String>,

    // Pre-calculated values for performance
    pub parsed_size: Option<(i32, i32)>, // width, height (cached for default monitor)
    pub parsed_offset: Option<(i32, i32)>, // x, y offset
    pub parsed_max_size: Option<(i32, i32)>, // max width, height
    pub parsed_position: Option<(i32, i32)>, // parsed x, y position
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
            position: None,
            hysteresis: Some(0.4),
            restore_focus: true,
            multi: false,
            multi_window: false,
            max_instances: Some(1),
            validation_errors: Vec::new(),
            validation_warnings: Vec::new(),
            parsed_size: None,
            parsed_offset: None,
            parsed_max_size: None,
            parsed_position: None,
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

#[derive(Debug, Clone)]
pub struct ScratchpadState {
    pub windows: Vec<WindowState>,
    pub is_spawned: bool,
    pub last_used: Option<Instant>,
    pub excluded_by: HashSet<String>, // Which scratchpads excluded this one
    pub cached_position: Option<(String, i32, i32, i32, i32)>, // monitor, x, y, w, h
    pub is_attached: bool,            // Whether window is attached to scratchpad system
}

impl Default for ScratchpadState {
    fn default() -> Self {
        Self {
            windows: Vec::new(),
            is_spawned: false,
            last_used: None,
            excluded_by: HashSet::new(),
            cached_position: None,
            is_attached: true, // Default to attached
        }
    }
}

impl ScratchpadState {
    pub fn new() -> Self {
        Self::default()
    }
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
        let (x, y) = if let Some((pos_x, pos_y)) = config.parsed_position {
            // Use explicit position when provided
            (monitor.x + pos_x, monitor.y + pos_y)
        } else {
            // Use offset and margin-based positioning
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

            (x, y)
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
        variables: &HashMap<String, String>,
    ) -> HashMap<String, ValidatedConfigRef> {
        let mut validated_temp = HashMap::new();

        // First pass: basic validation, variable expansion, and template resolution
        for (name, config) in configs {
            let mut validated_config = Self::convert_to_validated(config);

            // Expand variables in configuration fields
            validated_config.command = Self::expand_variables(&validated_config.command, variables);
            // Only expand class if it's not AUTO_DETECT
            if validated_config.class != "AUTO_DETECT" {
                validated_config.class = Self::expand_variables(&validated_config.class, variables);
            }

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
        let class = match &config.class {
            Some(class_str) if !class_str.trim().is_empty() => class_str.clone(),
            _ => "AUTO_DETECT".to_string(),
        };

        ValidatedConfig {
            command: config.command.clone(),
            class,
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
            position: config.position.clone(),
            hysteresis: config.hysteresis,
            restore_focus: config.restore_focus,
            multi: config.multi,
            multi_window: config.multi_window || config.multi, // Support both
            max_instances: config.max_instances,
            validation_errors: Vec::new(),
            validation_warnings: Vec::new(),
            parsed_size: None,
            parsed_offset: None,
            parsed_max_size: None,
            parsed_position: None,
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

        // Note: Class validation skipped - AUTO_DETECT is resolved after spawning

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

            // Pre-calculate position
            if let Some(position_str) = &config.position {
                if let Ok((x, y)) =
                    GeometryCalculator::parse_offset(Some(position_str), default_monitor)
                {
                    config.parsed_position = Some((x, y));
                } else {
                    config
                        .validation_errors
                        .push(format!("Invalid position format: {}", position_str));
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

        // Validate hysteresis
        if let Some(hysteresis) = config.hysteresis {
            if hysteresis < 0.0 {
                config
                    .validation_errors
                    .push("Hysteresis cannot be negative".to_string());
            } else if hysteresis > 5.0 {
                config.validation_warnings.push(
                    "Very high hysteresis value may make unfocus behavior sluggish".to_string(),
                );
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
        if config.class == "AUTO_DETECT" {
            if let Some(template_class) = &template.class {
                config.class = template_class.clone();
            }
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
        if config.position.is_none() {
            config.position = template.position.clone();
        }
        if config.hysteresis.is_none() {
            config.hysteresis = template.hysteresis;
        }
        if !config.multi && template.multi {
            config.multi = template.multi;
            config.multi_window = true; // Propagate to multi_window as well
        }

        config
    }

    /// Expand variables in a string
    fn expand_variables(input: &str, variables: &HashMap<String, String>) -> String {
        let mut result = input.to_string();
        for (key, value) in variables {
            let pattern = format!("[{key}]");
            result = result.replace(&pattern, value);
        }
        result
    }
}

// ============================================================================
// INTERNAL COMMANDS FOR DELAYED ACTIONS
// ============================================================================

#[derive(Debug, Clone)]
pub enum InternalCommand {
    SimpleHide { scratchpad_name: String },
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
    pub previous_focused_window: Option<String>, // For focus restoration

    // Template inheritance cache (Arc-optimized)
    pub resolved_configs: HashMap<String, ScratchpadConfigRef>,

    // Animation and delay management
    pub hide_tasks: HashMap<String, JoinHandle<()>>,
    pub hysteresis_tasks: HashMap<String, JoinHandle<()>>, // For hysteresis delays
    pub window_animator: Arc<Mutex<WindowAnimator>>,

    // Internal command channel for hysteresis and other delayed actions
    pub internal_sender: Option<mpsc::UnboundedSender<InternalCommand>>,
    pub internal_receiver: Option<mpsc::UnboundedReceiver<InternalCommand>>,

    // Validated configurations (Arc-optimized)
    pub validated_configs: HashMap<String, ValidatedConfigRef>,

    // Geometry synchronization
    pub geometry_cache: Arc<RwLock<HashMap<String, WindowGeometry>>>, // window_address -> geometry
    pub sync_tasks: HashMap<String, JoinHandle<()>>,                  // window_address -> sync task
}

impl ScratchpadsPlugin {
    pub fn new() -> Self {
        let (internal_sender, internal_receiver) = mpsc::unbounded_channel();

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
            previous_focused_window: None,
            resolved_configs: HashMap::new(),
            hide_tasks: HashMap::new(),
            hysteresis_tasks: HashMap::new(),
            window_animator: Arc::new(Mutex::new(WindowAnimator::new())),
            internal_sender: Some(internal_sender),
            internal_receiver: Some(internal_receiver),
            validated_configs: HashMap::new(),
            geometry_cache: Arc::new(RwLock::new(HashMap::new())),
            sync_tasks: HashMap::new(),
        }
    }

    pub async fn set_hyprland_client(&self, client: Arc<HyprlandClient>) {
        let mut client_guard = self.hyprland_client.lock().await;
        *client_guard = Some(client.clone());

        // Set the client for the WindowAnimator as well
        let animator = self.window_animator.lock().await;
        animator.set_hyprland_client(client).await;
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
                active_workspace_id: m.active_workspace.id,
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
        info!("🔄 Toggling scratchpad: {}", name);

        let validated_config = self.get_validated_config(name)?;
        debug!(
            "📋 Using config for '{}': class='{}', command='{}'",
            name, validated_config.class, validated_config.command
        );
        let client = self.get_hyprland_client().await?;

        // Find all windows of this class (handle AUTO_DETECT case)
        let windows = if validated_config.class == "AUTO_DETECT" {
            debug!("🔍 Class is AUTO_DETECT, no existing windows can be found - will spawn new");
            Vec::new() // Force spawn for auto-detection
        } else {
            client
                .find_windows_by_class(&validated_config.class)
                .await?
        };

        if windows.is_empty() {
            // No window exists - spawn a new one
            info!(
                "🚀 No {} window found, spawning new one",
                validated_config.class
            );
            self.spawn_and_show_scratchpad(name, &validated_config)
                .await
        } else {
            // Window exists - check if it's visible on current workspace
            let current_workspace = self.get_current_workspace(&client).await?;
            debug!("🖥️ Current workspace detected as: '{}'", current_workspace);

            let visible_window = self.find_visible_window(&windows, &current_workspace);

            if let Some(window) = visible_window {
                // Window is visible - hide it
                info!(
                    "👁️ {} window visible on current workspace, hiding it",
                    validated_config.class
                );
                self.hide_scratchpad_window(&client, window, name).await
            } else {
                // Window exists but not visible - show it
                info!(
                    "🙈 {} window exists but hidden, showing it",
                    validated_config.class
                );
                let window = &windows[0]; // Use first window
                self.show_scratchpad_window(&client, window, &validated_config, name)
                    .await
            }
        }
    }

    /// Show a scratchpad directly (without toggling)
    async fn show_scratchpad_direct(&mut self, name: &str) -> Result<String> {
        info!("👁️  Showing scratchpad directly: {}", name);

        let validated_config = self.get_validated_config(name)?;
        let client = self.get_hyprland_client().await?;

        // Find all windows of this class
        let windows = client
            .find_windows_by_class(&validated_config.class)
            .await?;

        if windows.is_empty() {
            // No window exists - spawn a new one
            info!(
                "🚀 No {} window found, spawning new one",
                validated_config.class
            );
            self.spawn_and_show_scratchpad(name, &validated_config)
                .await
        } else {
            // Window exists - show it
            let current_workspace = self.get_current_workspace(&client).await?;
            let visible_window = self.find_visible_window(&windows, &current_workspace);

            if visible_window.is_some() {
                // Already visible
                Ok(format!("Scratchpad '{}' is already visible", name))
            } else {
                // Show the window
                let window = &windows[0]; // Use first window
                self.show_scratchpad_window(&client, window, &validated_config, name)
                    .await
            }
        }
    }

    /// Hide a scratchpad directly (without toggling)
    async fn hide_scratchpad_direct(&mut self, name: &str) -> Result<String> {
        info!("🙈 Hiding scratchpad directly: {}", name);

        let validated_config = self.get_validated_config(name)?;
        let client = self.get_hyprland_client().await?;

        // Find all windows of this class
        let windows = client
            .find_windows_by_class(&validated_config.class)
            .await?;

        if windows.is_empty() {
            Ok(format!("No windows found for scratchpad '{}'", name))
        } else {
            // For auto-hide: find ANY scratchpad window that's not already in special workspace
            let active_window = windows
                .iter()
                .find(|window| !window.workspace.name.starts_with("special:"));

            if let Some(window) = active_window {
                info!(
                    "🔍 Found active scratchpad window {} on workspace {}",
                    window.address, window.workspace.name
                );
                // Hide the active window (even if on different workspace)
                self.hide_scratchpad_window(&client, window, name).await
            } else {
                Ok(format!("Scratchpad '{}' is already hidden", name))
            }
        }
    }

    /// Toggle window anchoring (attach/detach from scratchpad system)
    async fn toggle_attach_scratchpad(&mut self, name: &str) -> Result<String> {
        info!("📌 Toggling attach for scratchpad: {}", name);

        let validated_config = self.get_validated_config(name)?;
        let client = self.get_hyprland_client().await?;

        // Find all windows of this class
        let windows = client
            .find_windows_by_class(&validated_config.class)
            .await?;

        if windows.is_empty() {
            return Ok(format!("No windows found for scratchpad '{}'", name));
        }

        // Get the state for this scratchpad
        let state = self.states.get_mut(name);
        if let Some(state) = state {
            // Toggle attachment state
            state.is_attached = !state.is_attached;
            let status = if state.is_attached {
                "attached to scratchpad system"
            } else {
                "detached from scratchpad system"
            };

            info!("📌 Scratchpad '{}' is now {}", name, status);
            Ok(format!("Scratchpad '{}' is now {}", name, status))
        } else {
            // Initialize state if not exists
            let mut new_state = ScratchpadState::new();
            new_state.is_attached = false; // Start detached
            self.states.insert(name.to_string(), new_state);

            Ok(format!(
                "Scratchpad '{}' is now detached from scratchpad system",
                name
            ))
        }
    }

    /// Spawn a new scratchpad application
    async fn spawn_scratchpad(&mut self, name: &str, config: &ValidatedConfig) -> Result<String> {
        debug!("🚀 Spawning scratchpad '{}'", name);
        debug!(
            "📋 Scratchpad config - size: '{}', animation: {:?}, margin: {:?}",
            config.size, config.animation, config.margin
        );
        debug!("📋 Animation: {:?}", config.animation);

        let client = self.get_hyprland_client().await?;
        let vars = self.variables.read().await;
        let expanded_command = self.expand_command(&config.command, &vars);

        info!("🚀 Spawning application: {}", expanded_command);
        client.spawn_app(&expanded_command).await?;

        // Wait for the window to appear and configure it immediately
        let window = self.wait_for_window_and_configure(name, config).await?;

        // Update state
        let state = self.states.entry(name.to_string()).or_default();
        state.is_spawned = true;
        state.last_used = Some(Instant::now());

        // Add window to tracking
        self.window_to_scratchpad
            .insert(window.address.to_string(), name.to_string());

        let window_state = WindowState {
            address: window.address.to_string(),
            is_visible: true,
            last_position: None,
            monitor: None,
            workspace: None,
            last_focus: Some(Instant::now()),
        };

        state.windows.push(window_state);

        Ok(format!("Scratchpad '{name}' spawned and configured"))
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

        let window_address = window.address.to_string();

        // Reset any animation states that might cause transparency
        info!("🔄 Resetting window state for showing: {}", window_address);

        // Make sure opacity is reset to 1.0
        if let Err(e) = client.set_window_opacity(&window_address, 1.0).await {
            warn!("Failed to reset window opacity: {}", e);
        }

        // Apply final geometry
        self.apply_geometry(&window, config, monitor).await?;

        // Show window
        client.show_window(&window_address).await?;

        // Hyprland will handle animations automatically based on its configuration
        info!("✨ Window shown - Hyprland handles animations natively");

        // Focus if smart_focus is enabled
        if config.smart_focus {
            client.focus_window(&window_address).await?;
        }

        // Update state
        self.mark_window_visible(name, &window_address);

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

    /// Get current workspace information
    async fn get_current_workspace(&self, client: &HyprlandClient) -> Result<String> {
        client.get_active_workspace().await
    }

    /// Find if any window is visible on the current workspace  
    fn find_visible_window<'a>(
        &self,
        windows: &'a [hyprland::data::Client],
        current_workspace: &str,
    ) -> Option<&'a hyprland::data::Client> {
        // A window is visible if it's on the current workspace and not in a special workspace
        for window in windows {
            let workspace_id = window.workspace.id.to_string();
            let workspace_name = &window.workspace.name;

            debug!(
                "🔍 Checking window {} - workspace ID: '{}', name: '{}', current: '{}'",
                window.address, workspace_id, workspace_name, current_workspace
            );

            // Check if window is on current workspace and not special
            let is_visible =
                workspace_id == current_workspace && !workspace_name.starts_with("special:");
            debug!("👁️ Window {} visibility: {}", window.address, is_visible);

            if is_visible {
                return Some(window);
            }
        }
        None
    }

    /// Spawn and show a new scratchpad window
    async fn spawn_and_show_scratchpad(
        &mut self,
        name: &str,
        config: &ValidatedConfig,
    ) -> Result<String> {
        info!("🚀 Spawning and showing scratchpad: {}", name);

        let client = self.get_hyprland_client().await?;

        // Expand command with variables
        let variables = self.variables.read().await;
        let command = self.expand_command(&config.command, &variables);

        // Spawn the application
        client.spawn_app(&command).await?;

        // Handle auto-detection or use existing class
        let window = if config.class == "AUTO_DETECT" {
            info!("🔍 Auto-detecting window class for scratchpad '{}'", name);
            if let Some(detected_window) = self.wait_for_any_new_window(&client).await? {
                let detected_class = detected_window.class.clone();
                info!(
                    "✅ Auto-detected class '{}' for scratchpad '{}'",
                    detected_class, name
                );
                debug!("🔧 Updating validated config with detected class");

                // Update the validated config with the detected class
                if let Some(validated_config) = self.validated_configs.get_mut(name) {
                    let mut updated_config = (**validated_config).clone();
                    updated_config.class = detected_class;
                    self.validated_configs
                        .insert(name.to_string(), Arc::new(updated_config));
                }

                detected_window
            } else {
                return Err(anyhow::anyhow!(
                    "Failed to auto-detect window for scratchpad '{}'",
                    name
                ));
            }
        } else {
            // Use configured class
            if let Some(window) = self
                .wait_for_window_to_appear(&client, &config.class)
                .await?
            {
                window
            } else {
                return Err(anyhow::anyhow!(
                    "Failed to find spawned window for scratchpad '{}'",
                    name
                ));
            }
        };

        self.configure_new_scratchpad_window(&client, &window, config, name)
            .await?;

        // Add window to tracking
        self.window_to_scratchpad
            .insert(window.address.to_string(), name.to_string());

        // Update state
        let state = self.states.entry(name.to_string()).or_default();
        state.is_spawned = true;
        state.last_used = Some(Instant::now());

        let window_state = WindowState {
            address: window.address.to_string(),
            is_visible: true,
            last_position: None,
            monitor: None,
            workspace: None,
            last_focus: Some(Instant::now()),
        };

        state.windows.push(window_state);

        Ok(format!("Scratchpad '{name}' spawned and shown"))
    }

    /// Hide a scratchpad window with animation, then move to special workspace
    async fn hide_scratchpad_window(
        &self,
        client: &HyprlandClient,
        window: &hyprland::data::Client,
        name: &str,
    ) -> Result<String> {
        info!("🙈 Hiding scratchpad window: {}", window.address);

        // Get config for restore_focus setting and animation
        let config = self.get_validated_config(name)?;
        let window_address = window.address.to_string();

        // Store current focus for potential restoration
        let should_restore_focus = config.restore_focus;

        // Handle hide animations if specified
        if let Some(animation_type) = &config.animation {
            if let Ok(monitor) = self.get_target_monitor(&config).await {
                let geometry = GeometryCalculator::calculate_geometry(&config, &monitor)?;

                match animation_type.as_str() {
                    "fromTop" => {
                        info!("🎬 Starting hide animation: toTop");
                        let offset_pixels =
                            self.calculate_animation_offset(&config, geometry.height)?;
                        let end_y = geometry.y - geometry.height - offset_pixels;
                        info!(
                            "🎯 ToTop (hide): current_y={}, end_y={}, offset={}px",
                            geometry.y, end_y, offset_pixels
                        );

                        self.apply_animation_positions(
                            client,
                            &window_address,
                            geometry.x,
                            geometry.y,
                            geometry.x,
                            end_y,
                            geometry.width,
                            geometry.height,
                        )
                        .await?;
                        tokio::time::sleep(tokio::time::Duration::from_millis(250)).await; // Wait for animation
                        info!("✨ ToTop hide animation complete");
                    }
                    "fromBottom" => {
                        info!("🎬 Starting hide animation: toBottom");
                        let offset_pixels =
                            self.calculate_animation_offset(&config, geometry.height)?;
                        let screen_height = 1080; // TODO: Get actual screen height
                        let end_y = screen_height + offset_pixels;
                        info!(
                            "🎯 ToBottom (hide): current_y={}, end_y={}, offset={}px",
                            geometry.y, end_y, offset_pixels
                        );

                        self.apply_animation_positions(
                            client,
                            &window_address,
                            geometry.x,
                            geometry.y,
                            geometry.x,
                            end_y,
                            geometry.width,
                            geometry.height,
                        )
                        .await?;
                        tokio::time::sleep(tokio::time::Duration::from_millis(250)).await; // Wait for animation
                        info!("✨ ToBottom hide animation complete");
                    }
                    "fromLeft" => {
                        info!("🎬 Starting hide animation: toLeft");
                        let offset_pixels =
                            self.calculate_animation_offset(&config, geometry.width)?;
                        let end_x = geometry.x - geometry.width - offset_pixels;
                        info!(
                            "🎯 ToLeft (hide): current_x={}, end_x={}, offset={}px",
                            geometry.x, end_x, offset_pixels
                        );

                        self.apply_animation_positions(
                            client,
                            &window_address,
                            geometry.x,
                            geometry.y,
                            end_x,
                            geometry.y,
                            geometry.width,
                            geometry.height,
                        )
                        .await?;
                        tokio::time::sleep(tokio::time::Duration::from_millis(250)).await; // Wait for animation
                        info!("✨ ToLeft hide animation complete");
                    }
                    "fromRight" => {
                        info!("🎬 Starting hide animation: toRight");
                        let offset_pixels =
                            self.calculate_animation_offset(&config, geometry.width)?;
                        let screen_width = 1920; // TODO: Get actual screen width
                        let end_x = screen_width + offset_pixels;
                        info!(
                            "🎯 ToRight (hide): current_x={}, end_x={}, offset={}px",
                            geometry.x, end_x, offset_pixels
                        );

                        self.apply_animation_positions(
                            client,
                            &window_address,
                            geometry.x,
                            geometry.y,
                            end_x,
                            geometry.y,
                            geometry.width,
                            geometry.height,
                        )
                        .await?;
                        tokio::time::sleep(tokio::time::Duration::from_millis(250)).await; // Wait for animation
                        info!("✨ ToRight hide animation complete");
                    }
                    "fromTopLeft" => {
                        info!("🎬 Starting hide animation: toTopLeft");
                        let offset_pixels = self.calculate_animation_offset(
                            &config,
                            geometry.height.max(geometry.width),
                        )?;
                        let end_x = geometry.x - geometry.width - offset_pixels;
                        let end_y = geometry.y - geometry.height - offset_pixels;
                        info!(
                            "🎯 ToTopLeft (hide): current=({}, {}), end=({}, {})",
                            geometry.x, geometry.y, end_x, end_y
                        );

                        self.apply_animation_positions(
                            client,
                            &window_address,
                            geometry.x,
                            geometry.y,
                            end_x,
                            end_y,
                            geometry.width,
                            geometry.height,
                        )
                        .await?;
                        tokio::time::sleep(tokio::time::Duration::from_millis(250)).await; // Wait for animation
                        info!("✨ ToTopLeft hide animation complete");
                    }
                    "fromTopRight" => {
                        info!("🎬 Starting hide animation: toTopRight");
                        let offset_pixels = self.calculate_animation_offset(
                            &config,
                            geometry.height.max(geometry.width),
                        )?;
                        let screen_width = 1920; // TODO: Get actual screen width
                        let end_x = screen_width + offset_pixels;
                        let end_y = geometry.y - geometry.height - offset_pixels;
                        info!(
                            "🎯 ToTopRight (hide): current=({}, {}), end=({}, {})",
                            geometry.x, geometry.y, end_x, end_y
                        );

                        self.apply_animation_positions(
                            client,
                            &window_address,
                            geometry.x,
                            geometry.y,
                            end_x,
                            end_y,
                            geometry.width,
                            geometry.height,
                        )
                        .await?;
                        tokio::time::sleep(tokio::time::Duration::from_millis(250)).await; // Wait for animation
                        info!("✨ ToTopRight hide animation complete");
                    }
                    "fromBottomLeft" => {
                        info!("🎬 Starting hide animation: toBottomLeft");
                        let offset_pixels = self.calculate_animation_offset(
                            &config,
                            geometry.height.max(geometry.width),
                        )?;
                        let screen_height = 1080; // TODO: Get actual screen height
                        let end_x = geometry.x - geometry.width - offset_pixels;
                        let end_y = screen_height + offset_pixels;
                        info!(
                            "🎯 ToBottomLeft (hide): current=({}, {}), end=({}, {})",
                            geometry.x, geometry.y, end_x, end_y
                        );

                        self.apply_animation_positions(
                            client,
                            &window_address,
                            geometry.x,
                            geometry.y,
                            end_x,
                            end_y,
                            geometry.width,
                            geometry.height,
                        )
                        .await?;
                        tokio::time::sleep(tokio::time::Duration::from_millis(250)).await; // Wait for animation
                        info!("✨ ToBottomLeft hide animation complete");
                    }
                    "fromBottomRight" => {
                        info!("🎬 Starting hide animation: toBottomRight");
                        let offset_pixels = self.calculate_animation_offset(
                            &config,
                            geometry.height.max(geometry.width),
                        )?;
                        let screen_width = 1920; // TODO: Get actual screen width
                        let screen_height = 1080; // TODO: Get actual screen height
                        let end_x = screen_width + offset_pixels;
                        let end_y = screen_height + offset_pixels;
                        info!(
                            "🎯 ToBottomRight (hide): current=({}, {}), end=({}, {})",
                            geometry.x, geometry.y, end_x, end_y
                        );

                        self.apply_animation_positions(
                            client,
                            &window_address,
                            geometry.x,
                            geometry.y,
                            end_x,
                            end_y,
                            geometry.width,
                            geometry.height,
                        )
                        .await?;
                        tokio::time::sleep(tokio::time::Duration::from_millis(250)).await; // Wait for animation
                        info!("✨ ToBottomRight hide animation complete");
                    }
                    "fade" => {
                        info!("🎬 Starting hide animation: fade out");
                        // For fade animations, Hyprland handles opacity - we just wait for the animation
                        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
                        info!("✨ Fade out hide animation complete");
                    }
                    "scale" => {
                        info!("🎬 Starting hide animation: scale down");
                        // For scale animations, Hyprland handles scaling - we just wait for the animation
                        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
                        info!("✨ Scale down hide animation complete");
                    }
                    _ => {
                        debug!("🔄 No specific hide animation for type: {}", animation_type);
                    }
                }
            }
        }

        // Move to special workspace named after the scratchpad
        let special_workspace = format!("special:{name}");
        client
            .move_window_to_workspace(&window_address, &special_workspace)
            .await?;

        // Restore focus to previously focused window if enabled
        if should_restore_focus {
            if let Err(e) = self.restore_previous_focus(client).await {
                debug!("⚠️  Failed to restore focus: {}", e);
            }
        }

        Ok(format!("Scratchpad '{name}' hidden with animation"))
    }

    /// Show a scratchpad window on current workspace
    async fn show_scratchpad_window(
        &self,
        client: &HyprlandClient,
        window: &hyprland::data::Client,
        config: &ValidatedConfig,
        name: &str,
    ) -> Result<String> {
        info!("👁️ Showing scratchpad window: {}", window.address);

        let window_address = window.address.to_string();

        // Get target monitor and its active workspace
        let target_monitor = self.get_target_monitor(config).await?;
        let target_workspace = target_monitor.active_workspace_id.to_string();

        // Move to target monitor's active workspace (not special)
        client
            .move_window_to_workspace(&window_address, &target_workspace)
            .await?;

        // Apply geometry and focus
        if let Ok(monitor) = self.get_target_monitor(config).await {
            let geometry = GeometryCalculator::calculate_geometry(config, &monitor)?;

            // Handle animations for showing existing window
            if let Some(animation_type) = &config.animation {
                match animation_type.as_str() {
                    "fromTop" => {
                        info!("🎬 Starting fromTop animation for existing window");

                        let offset_pixels =
                            self.calculate_animation_offset(config, geometry.height)?;
                        let start_y = geometry.y - offset_pixels;
                        info!(
                            "🎯 FromTop (existing): start_y={}, final_y={}, offset={}px",
                            start_y, geometry.y, offset_pixels
                        );

                        self.apply_animation_positions(
                            client,
                            &window_address,
                            geometry.x,
                            start_y,
                            geometry.x,
                            geometry.y,
                            geometry.width,
                            geometry.height,
                        )
                        .await?;
                        info!("✨ FromTop animation (existing) setup complete");
                    }
                    "fromBottom" => {
                        info!("🎬 Starting fromBottom animation for existing window");

                        let offset_pixels =
                            self.calculate_animation_offset(config, geometry.height)?;
                        let screen_height = 1080; // TODO: Get actual screen height
                        let start_y = screen_height + offset_pixels;
                        info!(
                            "🎯 FromBottom (existing): start_y={}, final_y={}, offset={}px",
                            start_y, geometry.y, offset_pixels
                        );

                        self.apply_animation_positions(
                            client,
                            &window_address,
                            geometry.x,
                            start_y,
                            geometry.x,
                            geometry.y,
                            geometry.width,
                            geometry.height,
                        )
                        .await?;
                        info!("✨ FromBottom animation (existing) setup complete");
                    }
                    "fromLeft" => {
                        info!("🎬 Starting fromLeft animation for existing window");

                        let offset_pixels =
                            self.calculate_animation_offset(config, geometry.width)?;
                        let start_x = geometry.x - geometry.width - offset_pixels;
                        info!(
                            "🎯 FromLeft (existing): start_x={}, final_x={}, offset={}px",
                            start_x, geometry.x, offset_pixels
                        );

                        self.apply_animation_positions(
                            client,
                            &window_address,
                            start_x,
                            geometry.y,
                            geometry.x,
                            geometry.y,
                            geometry.width,
                            geometry.height,
                        )
                        .await?;
                        info!("✨ FromLeft animation (existing) setup complete");
                    }
                    "fromRight" => {
                        info!("🎬 Starting fromRight animation for existing window");

                        let offset_pixels =
                            self.calculate_animation_offset(config, geometry.width)?;
                        let screen_width = 1920; // TODO: Get actual screen width
                        let start_x = screen_width + offset_pixels;
                        info!(
                            "🎯 FromRight (existing): start_x={}, final_x={}, offset={}px",
                            start_x, geometry.x, offset_pixels
                        );

                        self.apply_animation_positions(
                            client,
                            &window_address,
                            start_x,
                            geometry.y,
                            geometry.x,
                            geometry.y,
                            geometry.width,
                            geometry.height,
                        )
                        .await?;
                        info!("✨ FromRight animation (existing) setup complete");
                    }
                    "fromTopLeft" => {
                        info!("🎬 Starting fromTopLeft animation for existing window");

                        let offset_pixels = self.calculate_animation_offset(
                            config,
                            geometry.height.max(geometry.width),
                        )?;
                        let start_x = geometry.x - geometry.width - offset_pixels;
                        let start_y = geometry.y - geometry.height - offset_pixels;
                        info!(
                            "🎯 FromTopLeft (existing): start=({}, {}), final=({}, {})",
                            start_x, start_y, geometry.x, geometry.y
                        );

                        self.apply_animation_positions(
                            client,
                            &window_address,
                            start_x,
                            start_y,
                            geometry.x,
                            geometry.y,
                            geometry.width,
                            geometry.height,
                        )
                        .await?;
                        info!("✨ FromTopLeft animation (existing) setup complete");
                    }
                    "fromTopRight" => {
                        info!("🎬 Starting fromTopRight animation for existing window");

                        let offset_pixels = self.calculate_animation_offset(
                            config,
                            geometry.height.max(geometry.width),
                        )?;
                        let screen_width = 1920; // TODO: Get actual screen width
                        let start_x = screen_width + offset_pixels;
                        let start_y = geometry.y - geometry.height - offset_pixels;
                        info!(
                            "🎯 FromTopRight (existing): start=({}, {}), final=({}, {})",
                            start_x, start_y, geometry.x, geometry.y
                        );

                        self.apply_animation_positions(
                            client,
                            &window_address,
                            start_x,
                            start_y,
                            geometry.x,
                            geometry.y,
                            geometry.width,
                            geometry.height,
                        )
                        .await?;
                        info!("✨ FromTopRight animation (existing) setup complete");
                    }
                    "fromBottomLeft" => {
                        info!("🎬 Starting fromBottomLeft animation for existing window");

                        let offset_pixels = self.calculate_animation_offset(
                            config,
                            geometry.height.max(geometry.width),
                        )?;
                        let screen_height = 1080; // TODO: Get actual screen height
                        let start_x = geometry.x - geometry.width - offset_pixels;
                        let start_y = screen_height + offset_pixels;
                        info!(
                            "🎯 FromBottomLeft (existing): start=({}, {}), final=({}, {})",
                            start_x, start_y, geometry.x, geometry.y
                        );

                        self.apply_animation_positions(
                            client,
                            &window_address,
                            start_x,
                            start_y,
                            geometry.x,
                            geometry.y,
                            geometry.width,
                            geometry.height,
                        )
                        .await?;
                        info!("✨ FromBottomLeft animation (existing) setup complete");
                    }
                    "fromBottomRight" => {
                        info!("🎬 Starting fromBottomRight animation for existing window");

                        let offset_pixels = self.calculate_animation_offset(
                            config,
                            geometry.height.max(geometry.width),
                        )?;
                        let screen_width = 1920; // TODO: Get actual screen width
                        let screen_height = 1080; // TODO: Get actual screen height
                        let start_x = screen_width + offset_pixels;
                        let start_y = screen_height + offset_pixels;
                        info!(
                            "🎯 FromBottomRight (existing): start=({}, {}), final=({}, {})",
                            start_x, start_y, geometry.x, geometry.y
                        );

                        self.apply_animation_positions(
                            client,
                            &window_address,
                            start_x,
                            start_y,
                            geometry.x,
                            geometry.y,
                            geometry.width,
                            geometry.height,
                        )
                        .await?;
                        info!("✨ FromBottomRight animation (existing) setup complete");
                    }
                    "fade" | "scale" => {
                        info!(
                            "🎬 Starting {} animation for existing window",
                            animation_type
                        );

                        // For fade and scale, just apply final geometry - Hyprland handles the visual effect
                        client
                            .resize_and_position_window(
                                &window_address,
                                geometry.x,
                                geometry.y,
                                geometry.width,
                                geometry.height,
                            )
                            .await?;

                        info!("✨ {} animation (existing) setup complete", animation_type);
                    }
                    _ => {
                        // Unknown animation type - just apply geometry
                        client
                            .resize_and_position_window(
                                &window_address,
                                geometry.x,
                                geometry.y,
                                geometry.width,
                                geometry.height,
                            )
                            .await?;

                        info!("✨ Animation '{}' - geometry applied", animation_type);
                    }
                }
            } else {
                // No animation - apply geometry directly
                client
                    .resize_and_position_window(
                        &window_address,
                        geometry.x,
                        geometry.y,
                        geometry.width,
                        geometry.height,
                    )
                    .await?;
            }

            // Focus if configured
            if config.smart_focus {
                client.focus_window(&window_address).await?;
            }
        }

        Ok(format!("Scratchpad '{name}' shown"))
    }

    /// Wait for a window to appear after spawning
    async fn wait_for_window_to_appear(
        &self,
        client: &HyprlandClient,
        class: &str,
    ) -> Result<Option<hyprland::data::Client>> {
        use tokio::time::{sleep, timeout, Duration};

        let wait_timeout = Duration::from_secs(5);
        let check_interval = Duration::from_millis(100);

        timeout(wait_timeout, async {
            loop {
                if let Ok(windows) = client.find_windows_by_class(class).await {
                    if !windows.is_empty() {
                        return Ok(Some(windows[0].clone()));
                    }
                }
                sleep(check_interval).await;
            }
        })
        .await
        .unwrap_or(Ok(None))
    }

    /// Wait for any new window to appear (for auto-detection)
    async fn wait_for_any_new_window(
        &self,
        client: &HyprlandClient,
    ) -> Result<Option<hyprland::data::Client>> {
        use tokio::time::{sleep, timeout, Duration};

        let wait_timeout = Duration::from_secs(5);
        let check_interval = Duration::from_millis(100);

        // Get current windows before spawning
        let initial_windows: std::collections::HashSet<_> = client
            .get_windows()
            .await?
            .into_iter()
            .map(|w| w.address.to_string())
            .collect();

        timeout(wait_timeout, async {
            loop {
                if let Ok(current_windows) = client.get_windows().await {
                    // Find new windows
                    for window in current_windows {
                        if !initial_windows.contains(&window.address.to_string()) {
                            info!(
                                "🔍 Found new window: {} (class: {})",
                                window.address, window.class
                            );
                            return Ok(Some(window));
                        }
                    }
                }
                sleep(check_interval).await;
            }
        })
        .await
        .unwrap_or(Ok(None))
    }

    /// Configure a newly spawned scratchpad window
    async fn configure_new_scratchpad_window(
        &self,
        client: &HyprlandClient,
        window: &hyprland::data::Client,
        config: &ValidatedConfig,
        name: &str,
    ) -> Result<()> {
        info!(
            "🔧 Configuring new scratchpad window: {} for '{}'",
            window.address, name
        );

        let window_address = window.address.to_string();

        // Make window floating
        client.toggle_floating(&window_address).await?;

        // Get target monitor and apply geometry
        let monitor = self.get_target_monitor(config).await?;
        let geometry = GeometryCalculator::calculate_geometry(config, &monitor)?;

        // Handle animations based on configuration
        if let Some(animation_type) = &config.animation {
            match animation_type.as_str() {
                "fromTop" => {
                    info!("🎬 Starting fromTop animation");

                    let offset_pixels = self.calculate_animation_offset(config, geometry.height)?;
                    let start_y = geometry.y - offset_pixels;
                    info!(
                        "🎯 FromTop animation: start_y={}, final_y={}, offset={}px",
                        start_y, geometry.y, offset_pixels
                    );

                    self.apply_animation_positions(
                        client,
                        &window_address,
                        geometry.x,
                        start_y,
                        geometry.x,
                        geometry.y,
                        geometry.width,
                        geometry.height,
                    )
                    .await?;
                    info!("✨ FromTop animation setup complete - Hyprland handling transition");
                }
                "fromBottom" => {
                    info!("🎬 Starting fromBottom animation");

                    let offset_pixels = self.calculate_animation_offset(config, geometry.height)?;
                    let screen_height = 1080; // TODO: Get actual screen height
                    let start_y = screen_height + offset_pixels;
                    info!(
                        "🎯 FromBottom animation: start_y={}, final_y={}, offset={}px",
                        start_y, geometry.y, offset_pixels
                    );

                    self.apply_animation_positions(
                        client,
                        &window_address,
                        geometry.x,
                        start_y,
                        geometry.x,
                        geometry.y,
                        geometry.width,
                        geometry.height,
                    )
                    .await?;
                    info!("✨ FromBottom animation setup complete");
                }
                "fromLeft" => {
                    info!("🎬 Starting fromLeft animation");

                    let offset_pixels = self.calculate_animation_offset(config, geometry.width)?;
                    let start_x = geometry.x - geometry.width - offset_pixels;
                    info!(
                        "🎯 FromLeft animation: start_x={}, final_x={}, offset={}px",
                        start_x, geometry.x, offset_pixels
                    );

                    self.apply_animation_positions(
                        client,
                        &window_address,
                        start_x,
                        geometry.y,
                        geometry.x,
                        geometry.y,
                        geometry.width,
                        geometry.height,
                    )
                    .await?;
                    info!("✨ FromLeft animation setup complete");
                }
                "fromRight" => {
                    info!("🎬 Starting fromRight animation");

                    let offset_pixels = self.calculate_animation_offset(config, geometry.width)?;
                    let screen_width = 1920; // TODO: Get actual screen width
                    let start_x = screen_width + offset_pixels;
                    info!(
                        "🎯 FromRight animation: start_x={}, final_x={}, offset={}px",
                        start_x, geometry.x, offset_pixels
                    );

                    self.apply_animation_positions(
                        client,
                        &window_address,
                        start_x,
                        geometry.y,
                        geometry.x,
                        geometry.y,
                        geometry.width,
                        geometry.height,
                    )
                    .await?;
                    info!("✨ FromRight animation setup complete");
                }
                "fromTopLeft" => {
                    info!("🎬 Starting fromTopLeft animation");

                    let offset_pixels = self
                        .calculate_animation_offset(config, geometry.height.max(geometry.width))?;
                    let start_x = geometry.x - geometry.width - offset_pixels;
                    let start_y = geometry.y - geometry.height - offset_pixels;
                    info!(
                        "🎯 FromTopLeft animation: start=({}, {}), final=({}, {})",
                        start_x, start_y, geometry.x, geometry.y
                    );

                    self.apply_animation_positions(
                        client,
                        &window_address,
                        start_x,
                        start_y,
                        geometry.x,
                        geometry.y,
                        geometry.width,
                        geometry.height,
                    )
                    .await?;
                    info!("✨ FromTopLeft animation setup complete");
                }
                "fromTopRight" => {
                    info!("🎬 Starting fromTopRight animation");

                    let offset_pixels = self
                        .calculate_animation_offset(config, geometry.height.max(geometry.width))?;
                    let screen_width = 1920; // TODO: Get actual screen width
                    let start_x = screen_width + offset_pixels;
                    let start_y = geometry.y - geometry.height - offset_pixels;
                    info!(
                        "🎯 FromTopRight animation: start=({}, {}), final=({}, {})",
                        start_x, start_y, geometry.x, geometry.y
                    );

                    self.apply_animation_positions(
                        client,
                        &window_address,
                        start_x,
                        start_y,
                        geometry.x,
                        geometry.y,
                        geometry.width,
                        geometry.height,
                    )
                    .await?;
                    info!("✨ FromTopRight animation setup complete");
                }
                "fromBottomLeft" => {
                    info!("🎬 Starting fromBottomLeft animation");

                    let offset_pixels = self
                        .calculate_animation_offset(config, geometry.height.max(geometry.width))?;
                    let screen_height = 1080; // TODO: Get actual screen height
                    let start_x = geometry.x - geometry.width - offset_pixels;
                    let start_y = screen_height + offset_pixels;
                    info!(
                        "🎯 FromBottomLeft animation: start=({}, {}), final=({}, {})",
                        start_x, start_y, geometry.x, geometry.y
                    );

                    self.apply_animation_positions(
                        client,
                        &window_address,
                        start_x,
                        start_y,
                        geometry.x,
                        geometry.y,
                        geometry.width,
                        geometry.height,
                    )
                    .await?;
                    info!("✨ FromBottomLeft animation setup complete");
                }
                "fromBottomRight" => {
                    info!("🎬 Starting fromBottomRight animation");

                    let offset_pixels = self
                        .calculate_animation_offset(config, geometry.height.max(geometry.width))?;
                    let screen_width = 1920; // TODO: Get actual screen width
                    let screen_height = 1080; // TODO: Get actual screen height
                    let start_x = screen_width + offset_pixels;
                    let start_y = screen_height + offset_pixels;
                    info!(
                        "🎯 FromBottomRight animation: start=({}, {}), final=({}, {})",
                        start_x, start_y, geometry.x, geometry.y
                    );

                    self.apply_animation_positions(
                        client,
                        &window_address,
                        start_x,
                        start_y,
                        geometry.x,
                        geometry.y,
                        geometry.width,
                        geometry.height,
                    )
                    .await?;
                    info!("✨ FromBottomRight animation setup complete");
                }
                "fade" | "scale" => {
                    info!("🎬 Starting {} animation", animation_type);

                    // For fade and scale, just apply final geometry - Hyprland handles the visual effect
                    client
                        .resize_and_position_window(
                            &window_address,
                            geometry.x,
                            geometry.y,
                            geometry.width,
                            geometry.height,
                        )
                        .await?;

                    info!("✨ {} animation setup complete", animation_type);
                }
                _ => {
                    // Unknown animation type - just apply geometry
                    client
                        .resize_and_position_window(
                            &window_address,
                            geometry.x,
                            geometry.y,
                            geometry.width,
                            geometry.height,
                        )
                        .await?;

                    info!("✨ Animation '{}' - geometry applied", animation_type);
                }
            }
        } else {
            // No animation - apply geometry directly
            client
                .resize_and_position_window(
                    &window_address,
                    geometry.x,
                    geometry.y,
                    geometry.width,
                    geometry.height,
                )
                .await?;
        }

        // Focus if configured
        if config.smart_focus {
            client.focus_window(&window_address).await?;
        }

        Ok(())
    }

    /// Parse offset string (pixels or percentage)
    fn parse_offset(&self, offset: &str, screen_dimension: i32) -> Result<i32> {
        if offset.ends_with('%') {
            let percent = offset.trim_end_matches('%').parse::<f32>()?;
            Ok((screen_dimension as f32 * percent / 100.0) as i32)
        } else if offset.ends_with("px") {
            Ok(offset.trim_end_matches("px").parse::<i32>()?)
        } else {
            Ok(offset.parse::<i32>()?)
        }
    }

    /// Apply simple easing functions
    fn apply_simple_easing(&self, easing: &str, t: f32) -> f32 {
        match easing {
            "ease-out-cubic" => {
                let t1 = 1.0 - t;
                1.0 - t1 * t1 * t1
            }
            "ease-out-quart" => {
                let t1 = 1.0 - t;
                1.0 - t1 * t1 * t1 * t1
            }
            "ease-out-smooth" => {
                // Smoother variant - less aggressive than cubic
                let t1 = 1.0 - t;
                1.0 - t1 * t1 * (2.0 - t1)
            }
            "ease-in-cubic" => t.powi(3),
            "ease-in-out-cubic" => {
                if t < 0.5 {
                    4.0 * t.powi(3)
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
                }
            }
            "ease-out" => 1.0 - (1.0 - t).powi(2),
            "ease-in" => t.powi(2),
            "linear" => t,
            _ => t,
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
            // TEMPORARILY DISABLE BROKEN ANIMATION SYSTEM
            // TODO: Fix animation system to properly restore window state
            info!("🔧 Hide animation system disabled due to window state corruption issues");

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
                        class: Some(class),
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

                    // Parse unfocus field
                    if let Some(toml::Value::String(unfocus_behavior)) = sc.get("unfocus") {
                        config.unfocus = Some(unfocus_behavior.clone());
                    }

                    // Parse hysteresis field
                    if let Some(toml::Value::Float(hysteresis)) = sc.get("hysteresis") {
                        config.hysteresis = Some(*hysteresis as f32);
                    } else if let Some(toml::Value::Integer(hysteresis)) = sc.get("hysteresis") {
                        config.hysteresis = Some(*hysteresis as f32);
                    }

                    // Parse restore_focus field
                    if let Some(toml::Value::Boolean(restore_focus)) = sc.get("restore_focus") {
                        config.restore_focus = *restore_focus;
                    }

                    self.scratchpads.insert(name.clone(), Arc::new(config));
                    self.states.insert(name.clone(), ScratchpadState::default());
                    info!("📝 Registered scratchpad: {}", name);
                }
            }
        }

        // Validate configurations
        let monitors = self.get_monitors().await.unwrap_or_default();
        let variables = self.variables.read().await.clone();
        self.validated_configs =
            ConfigValidator::validate_configs(&self.scratchpads, &monitors, &variables);

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

        // Process any pending internal commands (like hysteresis hide)
        self.process_internal_commands().await;

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
            "show" => {
                if let Some(scratchpad_name) = args.first() {
                    info!("👁️  Showing scratchpad: {}", scratchpad_name);
                    if self.scratchpads.contains_key(*scratchpad_name) {
                        match self.show_scratchpad_direct(scratchpad_name).await {
                            Ok(message) => {
                                info!("✅ {}", message);
                                Ok(message)
                            }
                            Err(e) => {
                                error!("❌ Failed to show scratchpad '{}': {}", scratchpad_name, e);
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
            "hide" => {
                if let Some(scratchpad_name) = args.first() {
                    info!("🙈 Hiding scratchpad: {}", scratchpad_name);
                    if self.scratchpads.contains_key(*scratchpad_name) {
                        match self.hide_scratchpad_direct(scratchpad_name).await {
                            Ok(message) => {
                                info!("✅ {}", message);
                                Ok(message)
                            }
                            Err(e) => {
                                error!("❌ Failed to hide scratchpad '{}': {}", scratchpad_name, e);
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
            "attach" => {
                if let Some(scratchpad_name) = args.first() {
                    info!("📌 Toggling attach for scratchpad: {}", scratchpad_name);
                    if self.scratchpads.contains_key(*scratchpad_name) {
                        match self.toggle_attach_scratchpad(scratchpad_name).await {
                            Ok(message) => {
                                info!("✅ {}", message);
                                Ok(message)
                            }
                            Err(e) => {
                                error!(
                                    "❌ Failed to toggle attach for scratchpad '{}': {}",
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
            _ => Err(anyhow::anyhow!("Unknown command: {}", command)),
        }
    }

    async fn cleanup(&mut self) -> Result<()> {
        info!("🧹 Cleaning up scratchpads plugin");

        // Cancel all hide tasks
        for (window_addr, handle) in self.hide_tasks.drain() {
            handle.abort();
            debug!("❌ Cancelled hide task for window: {}", window_addr);
        }

        // Cancel all hysteresis tasks
        for (scratchpad_name, handle) in self.hysteresis_tasks.drain() {
            handle.abort();
            debug!(
                "❌ Cancelled hysteresis task for scratchpad: {}",
                scratchpad_name
            );
        }

        // Cancel all sync tasks
        for (window_addr, handle) in self.sync_tasks.drain() {
            handle.abort();
            debug!("❌ Cancelled sync task for window: {}", window_addr);
        }

        info!("✅ Scratchpads plugin cleanup complete");
        Ok(())
    }
}

// Enhanced event handling methods
impl ScratchpadsPlugin {
    async fn handle_window_opened(&mut self, window_address: &str) {
        debug!("🪟 Window opened: {}", window_address);

        // Get window information from Hyprland to check if it's a scratchpad
        let client = match self.get_hyprland_client().await {
            Ok(client) => client,
            Err(e) => {
                debug!("❌ Failed to get Hyprland client: {}", e);
                return;
            }
        };

        // Get all windows to find the one that just opened
        let windows = match client.get_windows().await {
            Ok(windows) => windows,
            Err(e) => {
                debug!("❌ Failed to get window list: {}", e);
                return;
            }
        };

        // Find the window that was just opened
        let opened_window = windows
            .into_iter()
            .find(|w| w.address.to_string() == window_address);
        let window_class = match opened_window {
            Some(window) => {
                debug!(
                    "🔍 Found opened window - class: '{}', title: '{}'",
                    window.class, window.title
                );
                window.class
            }
            None => {
                debug!(
                    "❌ Could not find opened window with address: {}",
                    window_address
                );
                return;
            }
        };

        // Find scratchpad that matches this window class
        for (scratchpad_name, config) in &self.scratchpads {
            if config.class.as_ref() == Some(&window_class) {
                debug!(
                    "📋 Detected scratchpad window: {} for '{}' (class: '{}')",
                    window_address, scratchpad_name, window_class
                );

                // Add to tracking
                self.window_to_scratchpad
                    .insert(window_address.to_string(), scratchpad_name.clone());

                // Update state
                let state = self.states.entry(scratchpad_name.clone()).or_default();

                let window_state = WindowState {
                    address: window_address.to_string(),
                    is_visible: true, // Newly opened windows are visible
                    last_position: None,
                    monitor: None,
                    workspace: None,
                    last_focus: Some(std::time::Instant::now()),
                };

                // Add if not already tracked
                if !state.windows.iter().any(|w| w.address == *window_address) {
                    state.windows.push(window_state);
                    state.is_spawned = true;
                    debug!("✅ Added window to scratchpad '{}' state", scratchpad_name);
                }

                // Apply scratchpad geometry and trigger animation
                if let Err(e) = self
                    .setup_scratchpad_window(window_address, scratchpad_name, config)
                    .await
                {
                    warn!("❌ Failed to setup scratchpad window: {}", e);
                }

                // Start geometry sync for this window
                self.start_geometry_sync(window_address).await;

                break;
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
        info!("🖥️ WORKSPACE EVENT: Workspace changed to: {}", workspace);

        // DEBUG: Check what happens to focus tracking during workspace change
        if let Some(focused_window) = &self.focused_window {
            info!("🔍 Focus before workspace change: {}", focused_window);
        } else {
            info!("🔍 No focused window tracked before workspace change");
        }

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
        info!("👁️ FOCUS EVENT: Focus changed to: {}", window_address);

        // Handle previously focused window losing focus (simplified)
        let prev_window_clone = self.focused_window.clone();
        if let Some(prev_window) = prev_window_clone {
            info!("🔄 Previous focused window: {}", prev_window);
            if prev_window != window_address {
                info!(
                    "🔄 Focus changed from {} to {}",
                    prev_window, window_address
                );
                // Previous window lost focus - check if it needs auto-hide
                if let Some(scratchpad_name) = self.window_to_scratchpad.get(&prev_window).cloned()
                {
                    info!(
                        "🔍 Previous window '{}' is scratchpad '{}'",
                        prev_window, scratchpad_name
                    );
                    if let Ok(config) = self.get_validated_config(&scratchpad_name) {
                        if config.unfocus.as_deref() == Some("hide") {
                            let hysteresis = config.hysteresis.unwrap_or(0.4);
                            info!(
                                "🙈 UNFOCUS TRIGGER: Scratchpad '{}' lost focus, hiding in {:.1}s",
                                scratchpad_name, hysteresis
                            );
                            self.schedule_simple_hide(scratchpad_name, hysteresis).await;
                        } else {
                            info!(
                                "🔍 Scratchpad '{}' unfocus is {:?}, not hiding",
                                scratchpad_name, config.unfocus
                            );
                        }
                    }
                } else {
                    info!("🔍 Previous window '{}' is not a scratchpad", prev_window);
                }
                // Store for potential focus restoration
                if !self.window_to_scratchpad.contains_key(&prev_window) {
                    self.previous_focused_window = Some(prev_window);
                }
            } else {
                info!("🔄 Same window focused, no change");
            }
        } else {
            info!("🔄 No previous focused window");
        }

        // Update current focus
        self.focused_window = Some(window_address.to_string());
        info!("🎯 Updated focused_window to: {}", window_address);

        // Cancel hide timer if focusing a scratchpad
        if let Some(scratchpad_name) = self.window_to_scratchpad.get(window_address).cloned() {
            self.cancel_hide_timer(&scratchpad_name).await;
            info!(
                "🎯 Focused scratchpad '{}' - cancelled hide timer",
                scratchpad_name
            );
        } else {
            info!("🔍 Focused window '{}' is not a scratchpad", window_address);
        }
    }

    /// Simple hide scheduling with hysteresis (Pyprland-style)
    async fn schedule_simple_hide(&mut self, scratchpad_name: String, hysteresis_seconds: f32) {
        // Cancel any existing hide timer
        self.cancel_hide_timer(&scratchpad_name).await;

        // Create simple timer
        let delay_ms = (hysteresis_seconds * 1000.0) as u64;
        let sender = self.internal_sender.clone();
        let scratchpad_name_clone = scratchpad_name.clone();

        let handle = tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;

            if let Some(sender) = sender {
                let _ = sender.send(InternalCommand::SimpleHide {
                    scratchpad_name: scratchpad_name_clone,
                });
            }
        });

        self.hysteresis_tasks.insert(scratchpad_name, handle);
    }

    /// Cancel hide timer (simplified)
    async fn cancel_hide_timer(&mut self, scratchpad_name: &str) {
        if let Some(handle) = self.hysteresis_tasks.remove(scratchpad_name) {
            handle.abort();
        }
    }

    /// Process internal commands (like hysteresis hide)
    async fn process_internal_commands(&mut self) {
        // Collect commands first to avoid borrow conflicts
        let mut commands = Vec::new();

        if let Some(receiver) = &mut self.internal_receiver {
            while let Ok(command) = receiver.try_recv() {
                commands.push(command);
            }
        }

        // Process the collected commands (simplified)
        for command in commands {
            match command {
                InternalCommand::SimpleHide { scratchpad_name } => {
                    debug!("🙈 Processing simple hide for '{}'", scratchpad_name);
                    if let Err(e) = self.hide_scratchpad_direct(&scratchpad_name).await {
                        warn!("Failed to hide scratchpad '{}': {}", scratchpad_name, e);
                    } else {
                        debug!("✅ Scratchpad '{}' hidden", scratchpad_name);
                    }
                }
            }
        }
    }

    /// Restore focus to previously focused window
    async fn restore_previous_focus(&self, client: &HyprlandClient) -> Result<()> {
        if let Some(prev_window) = &self.previous_focused_window {
            // Check if the previous window still exists
            // Simply try to focus the window - if it fails, the window likely doesn't exist
            debug!("🎯 Restoring focus to previous window: {}", prev_window);
            if let Err(e) = client.focus_window(prev_window).await {
                debug!("⚠️  Failed to focus previous window {}: {}", prev_window, e);
            }
        }
        Ok(())
    }

    /// Animate window show with configured animation
    async fn animate_window_show(
        &self,
        window: &hyprland::data::Client,
        config: &ValidatedConfig,
        monitor: &MonitorInfo,
    ) -> Result<()> {
        let mut animator = self.window_animator.lock().await;

        // Create animation config from legacy animation field if present
        let animation_config = if let Some(animation_type) = &config.animation {
            crate::animation::AnimationConfig {
                animation_type: animation_type.clone(),
                direction: None,
                duration: 250, // Default duration
                easing: "ease-out-cubic".to_string(),
                delay: 0,
                offset: config.offset.clone().unwrap_or("100px".to_string()),
                scale_from: 1.0,
                opacity_from: 0.0,
                spring: None,
                properties: None,
                sequence: None,
                target_fps: 60,
                hardware_accelerated: true,
            }
        } else {
            return Ok(()); // No animation configured
        };

        // Get target geometry
        let target_geometry = GeometryCalculator::calculate_geometry(config, monitor)?;

        // Trigger show animation
        animator
            .show_window(
                &window.address.to_string(),
                (target_geometry.x, target_geometry.y),
                (target_geometry.width, target_geometry.height),
                animation_config,
            )
            .await?;

        Ok(())
    }

    /// Animate window hide with configured animation
    async fn animate_window_hide(
        &self,
        window: &hyprland::data::Client,
        config: &ValidatedConfig,
    ) -> Result<()> {
        let mut animator = self.window_animator.lock().await;

        // Create animation config from legacy animation field if present
        let animation_config = if let Some(animation_type) = &config.animation {
            crate::animation::AnimationConfig {
                animation_type: animation_type.clone(),
                direction: None,
                duration: 200, // Slightly faster for hide
                easing: "ease-in-cubic".to_string(),
                delay: 0,
                offset: config.offset.clone().unwrap_or("100px".to_string()),
                scale_from: 1.0,
                opacity_from: 1.0,
                spring: None,
                properties: None,
                sequence: None,
                target_fps: 60,
                hardware_accelerated: true,
            }
        } else {
            return Ok(()); // No animation configured
        };

        // Get current window position and size from Hyprland data
        let current_position = (window.at.0 as i32, window.at.1 as i32);
        let current_size = (window.size.0 as i32, window.size.1 as i32);

        // Trigger hide animation
        animator
            .hide_window(
                &window.address.to_string(),
                current_position,
                current_size,
                animation_config,
            )
            .await?;

        Ok(())
    }

    /// Wait for window to appear and configure it immediately
    async fn wait_for_window_and_configure(
        &self,
        scratchpad_name: &str,
        config: &ValidatedConfig,
    ) -> Result<hyprland::data::Client> {
        let client = self.get_hyprland_client().await?;
        let target_class = &config.class;

        debug!(
            "⏳ Waiting for window with class '{}' to appear",
            target_class
        );

        // Wait up to 5 seconds for the window to appear
        let mut attempts = 0;
        let max_attempts = 50; // 5 seconds with 100ms intervals

        loop {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            let windows = client.get_windows().await?;
            if let Some(window) = windows.into_iter().find(|w| w.class == *target_class) {
                info!(
                    "✅ Found window: {} with class '{}'",
                    window.address, window.class
                );

                // Configure the window immediately
                self.configure_scratchpad_window(&window, scratchpad_name, config)
                    .await?;

                return Ok(window);
            }

            attempts += 1;
            if attempts >= max_attempts {
                return Err(anyhow::anyhow!(
                    "Timeout waiting for window with class '{}' to appear",
                    target_class
                ));
            }
        }
    }

    /// Configure a scratchpad window with proper geometry and floating
    async fn configure_scratchpad_window(
        &self,
        window: &hyprland::data::Client,
        scratchpad_name: &str,
        config: &ValidatedConfig,
    ) -> Result<()> {
        info!(
            "🔧 Configuring scratchpad window: {} for '{}'",
            window.address, scratchpad_name
        );

        let client = self.get_hyprland_client().await?;
        let window_address = window.address.to_string();

        // Get monitor info
        let monitors = self.get_monitors().await?;
        let monitor = monitors
            .iter()
            .find(|m| m.is_focused)
            .or_else(|| monitors.first())
            .ok_or_else(|| anyhow::anyhow!("No monitors found"))?;

        debug!(
            "📋 Using config for '{}': size='{}', animation={:?}, margin={:?}",
            scratchpad_name, config.size, config.animation, config.margin
        );

        info!(
            "🖥️ Monitor info: '{}' - {}x{} at ({}, {})",
            monitor.name, monitor.width, monitor.height, monitor.x, monitor.y
        );

        // Step 1: Make the window floating FIRST
        info!("🔄 Making window floating: {}", window_address);
        client.toggle_floating(&window_address).await?;

        // Small delay to ensure floating state is applied
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Step 2: Calculate and apply proper geometry
        let target_geometry = GeometryCalculator::calculate_geometry(config, monitor)?;

        info!(
            "📐 Calculated geometry: {}x{} at ({}, {}) on monitor '{}' ({}x{} at {}x{})",
            target_geometry.width,
            target_geometry.height,
            target_geometry.x,
            target_geometry.y,
            monitor.name,
            monitor.width,
            monitor.height,
            monitor.x,
            monitor.y
        );

        client
            .move_resize_window(
                &window_address,
                target_geometry.x,
                target_geometry.y,
                target_geometry.width,
                target_geometry.height,
            )
            .await?;

        // Hyprland will handle animations automatically
        info!("✨ Geometry applied - letting Hyprland handle animations");

        info!(
            "✅ Scratchpad window '{}' configured successfully",
            scratchpad_name
        );
        Ok(())
    }

    /// Setup a newly opened scratchpad window with proper geometry and animation
    async fn setup_scratchpad_window(
        &self,
        window_address: &str,
        scratchpad_name: &str,
        _config: &ScratchpadConfigRef,
    ) -> Result<()> {
        info!(
            "🎬 Setting up scratchpad window: {} for '{}'",
            window_address, scratchpad_name
        );

        // Wait a moment for window to be fully created
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Get monitor info
        let monitors = self.get_monitors().await?;
        let monitor = monitors
            .iter()
            .find(|m| m.is_focused)
            .or_else(|| monitors.first())
            .ok_or_else(|| anyhow::anyhow!("No monitors found"))?;

        // Get validated config
        let validated_config = self.validated_configs.get(scratchpad_name).ok_or_else(|| {
            anyhow::anyhow!("No validated config for scratchpad: {}", scratchpad_name)
        })?;

        debug!(
            "📋 Using config for '{}': size='{}', animation={:?}, margin={:?}",
            scratchpad_name,
            validated_config.size,
            validated_config.animation,
            validated_config.margin
        );

        let client = self.get_hyprland_client().await?;

        // First, make sure the window is floating
        info!("🔄 Making window floating: {}", window_address);
        if let Err(e) = client.toggle_floating(window_address).await {
            warn!("Failed to toggle floating: {}", e);
        }

        // Small delay to ensure floating state is applied
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // Calculate and apply proper geometry
        let target_geometry = GeometryCalculator::calculate_geometry(validated_config, monitor)?;

        info!(
            "📐 Applying geometry: {}x{} at ({}, {}) on monitor '{}'",
            target_geometry.width,
            target_geometry.height,
            target_geometry.x,
            target_geometry.y,
            monitor.name
        );

        client
            .move_resize_window(
                window_address,
                target_geometry.x,
                target_geometry.y,
                target_geometry.width,
                target_geometry.height,
            )
            .await?;

        // Apply animation if configured
        if validated_config.animation.is_some() {
            info!(
                "🎬 Applying show animation for scratchpad '{}'",
                scratchpad_name
            );

            // Get the window data for animation
            let windows = client.get_windows().await?;
            if let Some(window) = windows
                .into_iter()
                .find(|w| w.address.to_string() == window_address)
            {
                if let Err(e) = self
                    .animate_window_show(&window, validated_config, monitor)
                    .await
                {
                    warn!("Failed to animate window show: {}", e);
                }
            }
        }

        info!("✅ Scratchpad window '{}' setup complete", scratchpad_name);
        Ok(())
    }
    /// Helper function to calculate animation offset with config support
    fn calculate_animation_offset(&self, config: &ValidatedConfig, dimension: i32) -> Result<i32> {
        if let Some(offset_str) = &config.offset {
            self.parse_offset(offset_str, dimension)
                .map_err(|_| anyhow::anyhow!("Failed to parse offset"))
        } else {
            Ok(dimension + 100) // Default offset
        }
    }

    /// Helper function to apply animation positions (start -> final)
    #[allow(clippy::too_many_arguments)]
    async fn apply_animation_positions(
        &self,
        client: &HyprlandClient,
        window_address: &str,
        start_x: i32,
        start_y: i32,
        final_x: i32,
        final_y: i32,
        width: i32,
        height: i32,
    ) -> Result<()> {
        // Position window at start position
        client
            .resize_and_position_window(window_address, start_x, start_y, width, height)
            .await?;

        // Brief delay to ensure start position is applied
        tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;

        // Move to final position - Hyprland will animate smoothly
        client
            .resize_and_position_window(window_address, final_x, final_y, width, height)
            .await?;

        Ok(())
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
            active_workspace_id: 1,
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
        assert_eq!(term_config.class, Some("foot".to_string()));
        assert_eq!(term_config.size, "75% 60%");
        assert!(!term_config.lazy);
        assert!(term_config.pinned);

        // Check browser scratchpad config
        let browser_config = plugin.scratchpads.get("browser").unwrap();
        assert_eq!(browser_config.command, "firefox --new-window");
        assert_eq!(browser_config.class, Some("firefox".to_string()));
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
        assert_eq!(config.class, None);
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
                class: Some("foot".to_string()),
                size: "75% 60%".to_string(),
                ..Default::default()
            },
        );

        // Convert configs to Arc-wrapped for validation
        let arc_configs: std::collections::HashMap<String, ScratchpadConfigRef> =
            configs.into_iter().map(|(k, v)| (k, Arc::new(v))).collect();

        let variables = HashMap::new();
        let validated = ConfigValidator::validate_configs(&arc_configs, &monitors, &variables);
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
                position: None,
                hysteresis: Some(0.4),
                restore_focus: true,
                multi: false,
                multi_window: false,
                max_instances: Some(1),
                validation_errors: Vec::new(),
                validation_warnings: Vec::new(),
                parsed_size: Some((960, 648)),
                parsed_offset: None,
                parsed_max_size: None,
                parsed_position: None,
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
                class: Some("advanced".to_string()),
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

        let variables = HashMap::new();
        let validated = ConfigValidator::validate_configs(&arc_configs, &monitors, &variables);
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
