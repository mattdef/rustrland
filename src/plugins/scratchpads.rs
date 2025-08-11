use anyhow::Result;
use async_trait::async_trait;
use tracing::{info, debug, warn, error};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::plugins::Plugin;
use crate::ipc::{HyprlandEvent, HyprlandClient};

#[derive(Debug, Clone)]
pub struct ScratchpadConfig {
    pub command: String,
    pub class: String,
    pub size: String,
    pub animation: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ScratchpadState {
    pub window_address: Option<String>,
    pub is_visible: bool,
    pub last_seen: Option<std::time::Instant>,
}

impl Default for ScratchpadState {
    fn default() -> Self {
        Self {
            window_address: None,
            is_visible: false,
            last_seen: None,
        }
    }
}

pub struct ScratchpadsPlugin {
    scratchpads: HashMap<String, ScratchpadConfig>,
    states: HashMap<String, ScratchpadState>,
    hyprland_client: Arc<Mutex<Option<Arc<HyprlandClient>>>>,
    variables: HashMap<String, String>,
}

impl ScratchpadsPlugin {
    pub fn new() -> Self {
        Self {
            scratchpads: HashMap::new(),
            states: HashMap::new(),
            hyprland_client: Arc::new(Mutex::new(None)),
            variables: HashMap::new(),
        }
    }
    
    pub async fn set_hyprland_client(&self, client: Arc<HyprlandClient>) {
        let mut client_guard = self.hyprland_client.lock().await;
        *client_guard = Some(client);
    }
    
    /// Parse size string like "75% 60%" into pixel dimensions
    fn parse_size(&self, size_str: &str) -> (i32, i32) {
        let parts: Vec<&str> = size_str.split_whitespace().collect();
        if parts.len() != 2 {
            warn!("Invalid size format '{}', using default 800x600", size_str);
            return (800, 600);
        }
        
        // For now, use fixed sizes. A real implementation would calculate based on screen resolution
        let width = if parts[0].ends_with('%') {
            let percent = parts[0].trim_end_matches('%').parse::<f32>().unwrap_or(75.0);
            (1920.0 * percent / 100.0) as i32
        } else {
            parts[0].trim_end_matches("px").parse::<i32>().unwrap_or(800)
        };
        
        let height = if parts[1].ends_with('%') {
            let percent = parts[1].trim_end_matches('%').parse::<f32>().unwrap_or(60.0);
            (1080.0 * percent / 100.0) as i32
        } else {
            parts[1].trim_end_matches("px").parse::<i32>().unwrap_or(600)
        };
        
        (width, height)
    }
    
    /// Process variable substitution in commands
    fn expand_command(&self, command: &str, variables: &HashMap<String, String>) -> String {
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
    async fn toggle_scratchpad(&mut self, name: &str, config: &ScratchpadConfig) -> Result<String> {
        debug!("🔄 Processing toggle for scratchpad '{}' with class '{}'", name, config.class);
        
        let client_guard = self.hyprland_client.lock().await;
        let client = match client_guard.as_ref() {
            Some(client) => Arc::clone(client),
            None => {
                return Err(anyhow::anyhow!("Hyprland client not available"));
            }
        };
        drop(client_guard);
        
        // Check if window exists
        let existing_window = client.find_window_by_class(&config.class).await?;
        
        match existing_window {
            Some(window) => {
                // Window exists, toggle its visibility
                debug!("🪟 Window exists, toggling visibility");
                client.toggle_window_visibility(&window.address.to_string()).await?;
                
                // Update state
                if let Some(state) = self.states.get_mut(name) {
                    state.window_address = Some(window.address.to_string());
                    state.is_visible = !state.is_visible;
                    state.last_seen = Some(std::time::Instant::now());
                }
                
                let action = if self.states.get(name).map(|s| s.is_visible).unwrap_or(false) {
                    "shown"
                } else {
                    "hidden"
                };
                
                Ok(format!("Scratchpad '{}' {}", name, action))
            }
            None => {
                // Window doesn't exist, spawn it
                debug!("🚀 Window doesn't exist, spawning new application");
                
                let expanded_command = self.expand_command(&config.command, &self.variables);
                info!("🚀 Spawning application: {}", expanded_command);
                client.spawn_app(&expanded_command).await?;
                
                // Wait a moment for the window to appear
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                
                // Try to find the newly spawned window
                if let Ok(Some(new_window)) = client.find_window_by_class(&config.class).await {
                    debug!("✅ Found newly spawned window");
                    
                    // Position and resize the window
                    let (width, height) = self.parse_size(&config.size);
                    let x = (1920 - width) / 2; // Center horizontally (assuming 1920px screen)
                    let y = (1080 - height) / 2; // Center vertically (assuming 1080px screen)
                    
                    if let Err(e) = client.move_resize_window(&new_window.address.to_string(), x, y, width, height).await {
                        warn!("⚠️  Failed to position window: {}", e);
                    }
                    
                    // Focus the window
                    if let Err(e) = client.focus_window(&new_window.address.to_string()).await {
                        warn!("⚠️  Failed to focus window: {}", e);
                    }
                    
                    // Update state
                    if let Some(state) = self.states.get_mut(name) {
                        state.window_address = Some(new_window.address.to_string());
                        state.is_visible = true;
                        state.last_seen = Some(std::time::Instant::now());
                    }
                    
                    Ok(format!("Scratchpad '{}' spawned and shown", name))
                } else {
                    warn!("⚠️  Failed to find window after spawning");
                    Ok(format!("Scratchpad '{}' spawned (window not immediately found)", name))
                }
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
                    
                    let config = ScratchpadConfig {
                        command,
                        class,
                        size,
                        animation,
                    };
                    
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
        
        // Handle relevant events (window open/close, workspace changes, etc.)
        match event {
            HyprlandEvent::WindowOpened { window } => {
                debug!("Window opened: {} - checking if it's a scratchpad", window);
            }
            HyprlandEvent::WindowClosed { window } => {
                debug!("Window closed: {} - checking if it was a scratchpad", window);
            }
            HyprlandEvent::WorkspaceChanged { workspace } => {
                debug!("Workspace changed to: {}", workspace);
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
                    
                    if let Some(config) = self.scratchpads.get(*scratchpad_name).cloned() {
                        match self.toggle_scratchpad(scratchpad_name, &config).await {
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
                let names: Vec<String> = self.scratchpads.keys().cloned().collect();
                Ok(format!("Available scratchpads: {}", names.join(", ")))
            }
            _ => {
                Err(anyhow::anyhow!("Unknown command: {}", command))
            }
        }
    }
}
