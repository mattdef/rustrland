use anyhow::Result;
use async_trait::async_trait;
use tracing::{info, debug, warn, error};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
use std::time::{Instant, Duration};

use crate::plugins::Plugin;
use crate::ipc::{HyprlandEvent, HyprlandClient, MonitorInfo};

#[derive(Debug, Clone, Serialize, Deserialize)]
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
}

impl Default for ScratchpadState {
    fn default() -> Self {
        Self {
            windows: Vec::new(),
            is_spawned: false,
            last_used: None,
            excluded_by: HashSet::new(),
            cached_position: None,
        }
    }
}

pub struct ScratchpadsPlugin {
    pub scratchpads: HashMap<String, ScratchpadConfig>,
    pub states: HashMap<String, ScratchpadState>,
    pub hyprland_client: Arc<Mutex<Option<Arc<HyprlandClient>>>>,
    pub variables: HashMap<String, String>,
    
    // Performance optimizations
    pub monitors_cache: Arc<RwLock<Vec<MonitorInfo>>>,
    pub cache_valid_until: Arc<RwLock<Instant>>,
    pub cache_duration: Duration,
    
    // Multi-window tracking
    pub window_to_scratchpad: HashMap<String, String>, // window_address -> scratchpad_name
    pub focused_window: Option<String>,
    
    // Template inheritance cache
    pub resolved_configs: HashMap<String, ScratchpadConfig>,
}

impl ScratchpadsPlugin {
    pub fn new() -> Self {
        Self {
            scratchpads: HashMap::new(),
            states: HashMap::new(),
            hyprland_client: Arc::new(Mutex::new(None)),
            variables: HashMap::new(),
            monitors_cache: Arc::new(RwLock::new(Vec::new())),
            cache_valid_until: Arc::new(RwLock::new(Instant::now())),
            cache_duration: Duration::from_secs(2), // Cache monitors for 2 seconds
            window_to_scratchpad: HashMap::new(),
            focused_window: None,
            resolved_configs: HashMap::new(),
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
        let monitor_infos: Vec<MonitorInfo> = monitors.iter().map(|m| {
            MonitorInfo {
                name: m.name.clone(),
                width: m.width as i32,
                height: m.height as i32,
                x: m.x as i32,
                y: m.y as i32,
                scale: m.scale,
                is_focused: m.focused,
            }
        }).collect();
        
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
    pub async fn get_target_monitor(&self, config: &ScratchpadConfig) -> Result<MonitorInfo> {
        let monitors = self.get_monitors().await?;
        
        // Force specific monitor if configured
        if let Some(forced_monitor) = &config.force_monitor {
            if let Some(monitor) = monitors.iter().find(|m| m.name == *forced_monitor) {
                return Ok(monitor.clone());
            }
            warn!("Forced monitor '{}' not found, using focused monitor", forced_monitor);
        }
        
        // Use focused monitor
        monitors.iter()
            .find(|m| m.is_focused)
            .cloned()
            .or_else(|| monitors.first().cloned())
            .ok_or_else(|| anyhow::anyhow!("No monitors available"))
    }
    
    /// Parse size string with monitor-aware dimensions
    pub async fn parse_size(&self, size_str: &str, monitor: &MonitorInfo, max_size: Option<&str>) -> Result<(i32, i32)> {
        let parts: Vec<&str> = size_str.split_whitespace().collect();
        if parts.len() != 2 {
            warn!("Invalid size format '{}', using default 50% 50%", size_str);
            return Ok((monitor.width / 2, monitor.height / 2));
        }
        
        let width = self.parse_dimension(parts[0], monitor.width)?;
        let height = self.parse_dimension(parts[1], monitor.height)?;
        
        // Apply max_size constraints if specified
        if let Some(max_size_str) = max_size {
            let max_parts: Vec<&str> = max_size_str.split_whitespace().collect();
            if max_parts.len() == 2 {
                let max_width = self.parse_dimension(max_parts[0], monitor.width)?;
                let max_height = self.parse_dimension(max_parts[1], monitor.height)?;
                return Ok((width.min(max_width), height.min(max_height)));
            }
        }
        
        Ok((width, height))
    }
    
    /// Parse individual dimension (supports %, px, or raw numbers)
    pub fn parse_dimension(&self, dim_str: &str, monitor_size: i32) -> Result<i32> {
        if dim_str.ends_with('%') {
            let percent = dim_str.trim_end_matches('%').parse::<f32>()
                .map_err(|_| anyhow::anyhow!("Invalid percentage: {}", dim_str))?;
            Ok((monitor_size as f32 * percent / 100.0) as i32)
        } else if dim_str.ends_with("px") {
            let pixels = dim_str.trim_end_matches("px").parse::<i32>()
                .map_err(|_| anyhow::anyhow!("Invalid pixel value: {}", dim_str))?;
            Ok(pixels)
        } else {
            // Raw number, assume pixels
            dim_str.parse::<i32>()
                .map_err(|_| anyhow::anyhow!("Invalid dimension: {}", dim_str))
        }
    }
    
    /// Process variable substitution in commands
    pub fn expand_command(&self, command: &str, variables: &HashMap<String, String>) -> String {
        let mut result = command.to_string();
        
        // Replace variables in [variable] format
        for (key, value) in variables {
            let pattern = format!("[{}]", key);
            result = result.replace(&pattern, value);
        }
        
        debug!("🔄 Expanded command '{}' to '{}'", command, result);
        result
    }
    
    /// Main toggle logic for scratchpads
    async fn toggle_scratchpad(&mut self, name: &str) -> Result<String> {
        let config = if let Some(config) = self.scratchpads.get(name).cloned() {
            config
        } else {
            return Err(anyhow::anyhow!("Scratchpad '{}' not found", name));
        };
        
        debug!("🔄 Processing toggle for scratchpad '{}' with class '{}'", name, config.class);
        
        let client_guard = self.hyprland_client.lock().await;
        let client = match client_guard.as_ref() {
            Some(client) => Arc::clone(client),
            None => return Err(anyhow::anyhow!("Hyprland client not available")),
        };
        drop(client_guard);
        
        // Check if window exists
        let existing_window = client.find_window_by_class(&config.class).await?;
        
        match existing_window {
            Some(window) => {
                debug!("🪟 Window exists, toggling visibility");
                client.toggle_window_visibility(&window.address.to_string()).await?;
                
                // Update state
                let state = self.states.entry(name.to_string()).or_default();
                state.last_used = Some(Instant::now());
                
                Ok(format!("Scratchpad '{}' toggled", name))
            }
            None => {
                debug!("🚀 Window does not exist, spawning new application");
                let expanded_command = self.expand_command(&config.command, &self.variables);
                info!("🚀 Spawning application: {}", expanded_command);
                client.spawn_app(&expanded_command).await?;
                
                // Update state
                let state = self.states.entry(name.to_string()).or_default();
                state.is_spawned = true;
                state.last_used = Some(Instant::now());
                
                Ok(format!("Scratchpad '{}' spawned", name))
            }
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
        
        // Parse variables if present
        if let toml::Value::Table(map) = config {
            if let Some(toml::Value::Table(vars)) = map.get("variables") {
                for (key, value) in vars {
                    if let toml::Value::String(val_str) = value {
                        self.variables.insert(key.clone(), val_str.clone());
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
                        config.excludes = excludes.iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect();
                    } else if let Some(toml::Value::String(exclude_all)) = sc.get("excludes") {
                        if exclude_all == "*" {
                            config.excludes = vec!["*".to_string()];
                        }
                    }
                    if let Some(toml::Value::Boolean(restore_excluded)) = sc.get("restore_excluded") {
                        config.restore_excluded = *restore_excluded;
                    }
                    if let Some(toml::Value::String(force_monitor)) = sc.get("force_monitor") {
                        config.force_monitor = Some(force_monitor.clone());
                    }
                    if let Some(toml::Value::Integer(margin)) = sc.get("margin") {
                        config.margin = Some(*margin as i32);
                    }
                    
                    self.scratchpads.insert(name.clone(), config);
                    self.states.insert(name.clone(), ScratchpadState::default());
                    info!("📝 Registered scratchpad: {}", name);
                }
            }
        }
        
        info!("✅ Scratchpads plugin initialized with {} scratchpads", self.scratchpads.len());
        Ok(())
    }
    
    async fn handle_event(&mut self, event: &HyprlandEvent) -> Result<()> {
        debug!("🪟 Scratchpads handling event: {:?}", event);
        
        match event {
            HyprlandEvent::WindowOpened { window } => {
                debug!("Window opened: {} - checking if it is a scratchpad", window);
            }
            HyprlandEvent::WindowClosed { window } => {
                debug!("Window closed: {} - cleaning up if scratchpad", window);
            }
            HyprlandEvent::WorkspaceChanged { workspace } => {
                debug!("Workspace changed to: {}", workspace);
            }
            HyprlandEvent::MonitorChanged { monitor: _ } => {
                debug!("Monitor changed - invalidating cache");
                // Invalidate monitor cache
                let mut cache_valid = self.cache_valid_until.write().await;
                *cache_valid = Instant::now();
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
                    
                    if self.scratchpads.contains_key(*scratchpad_name) {
                        match self.toggle_scratchpad(scratchpad_name).await {
                            Ok(message) => {
                                info!("✅ {}", message);
                                Ok(message)
                            }
                            Err(e) => {
                                error!("❌ Failed to toggle scratchpad '{}': {}", scratchpad_name, e);
                                Err(e)
                            }
                        }
                    } else {
                        warn!("⚠️  Scratchpad '{}' not found", scratchpad_name);
                        Err(anyhow::anyhow!("Scratchpad '{}' not found", scratchpad_name))
                    }
                } else {
                    Err(anyhow::anyhow!("No scratchpad name provided"))
                }
            }
            "list" => {
                let mut status_list = Vec::new();
                for (name, _config) in &self.scratchpads {
                    let state = self.states.get(name);
                    let visible_count = state.map(|s| s.windows.iter().filter(|w| w.is_visible).count()).unwrap_or(0);
                    let total_count = state.map(|s| s.windows.len()).unwrap_or(0);
                    let spawned = state.map(|s| s.is_spawned).unwrap_or(false);
                    
                    let status = if visible_count > 0 {
                        format!("{} (visible: {}/{})", name, visible_count, total_count)
                    } else if spawned {
                        format!("{} (hidden: {})", name, total_count)
                    } else {
                        format!("{} (not spawned)", name)
                    };
                    status_list.push(status);
                }
                Ok(format!("Scratchpads: {}", status_list.join(", ")))
            }
            _ => {
                Err(anyhow::anyhow!("Unknown command: {}", command))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test;
    
    fn create_test_config() -> toml::Value {
        toml::from_str(r#"
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
        "#).unwrap()
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
        assert_eq!(plugin.variables.get("term_class"), Some(&"foot".to_string()));
    }
    
    #[tokio::test]
    async fn test_size_parsing() {
        let plugin = ScratchpadsPlugin::new();
        let monitor = MonitorInfo {
            name: "DP-1".to_string(),
            width: 1920,
            height: 1080,
            x: 0,
            y: 0,
            scale: 1.0,
            is_focused: true,
        };
        
        // Test percentage sizes
        let (width, height) = plugin.parse_size("75% 60%", &monitor, None).await.unwrap();
        assert_eq!(width, 1440); // 75% of 1920
        assert_eq!(height, 648);  // 60% of 1080
        
        // Test pixel sizes
        let (width, height) = plugin.parse_size("800px 600px", &monitor, None).await.unwrap();
        assert_eq!(width, 800);
        assert_eq!(height, 600);
        
        // Test mixed sizes
        let (width, height) = plugin.parse_size("50% 500px", &monitor, None).await.unwrap();
        assert_eq!(width, 960);  // 50% of 1920
        assert_eq!(height, 500);
        
        // Test max_size constraint
        let (width, height) = plugin.parse_size("90% 90%", &monitor, Some("1600px 900px")).await.unwrap();
        assert_eq!(width, 1600); // Constrained by max_size
        assert_eq!(height, 900);  // Constrained by max_size
    }
    
    #[tokio::test]
    async fn test_dimension_parsing() {
        let plugin = ScratchpadsPlugin::new();
        
        // Test percentage
        assert_eq!(plugin.parse_dimension("50%", 1920).unwrap(), 960);
        assert_eq!(plugin.parse_dimension("75%", 1080).unwrap(), 810);
        
        // Test pixels
        assert_eq!(plugin.parse_dimension("800px", 1920).unwrap(), 800);
        assert_eq!(plugin.parse_dimension("600", 1080).unwrap(), 600);
        
        // Test invalid input
        assert!(plugin.parse_dimension("invalid", 1920).is_err());
        assert!(plugin.parse_dimension("200%px", 1920).is_err());
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
    
    #[tokio::test]
    async fn test_configuration_defaults() {
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
}