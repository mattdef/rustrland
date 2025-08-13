use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

use crate::ipc::{HyprlandClient, HyprlandEvent};
use crate::plugins::Plugin;

use hyprland::data::{Client, Clients, Workspaces};
use hyprland::dispatch::{Dispatch, DispatchType, WorkspaceIdentifierWithSpecial};
use hyprland::shared::{HyprData, HyprDataVec};

#[derive(Debug, Deserialize, Serialize)]
pub struct ToggleSpecialConfig {
    /// Default special workspace name if none provided (default: "special")
    #[serde(default = "default_special_name")]
    pub default_special_name: String,

    /// Animation duration for window transitions in milliseconds (default: 250)
    #[serde(default = "default_animation_duration")]
    pub animation_duration: u64,

    /// Delay between operations to prevent rapid toggling (default: 100)
    #[serde(default = "default_operation_delay")]
    pub operation_delay: u64,

    /// Log toggle operations for debugging (default: false)
    #[serde(default)]
    pub debug_logging: bool,

    /// Enable smooth transitions during toggles (default: true)
    #[serde(default = "default_true")]
    pub enable_animations: bool,

    /// Auto-close special workspace when last window is moved out (default: true)
    #[serde(default = "default_true")]
    pub auto_close_empty: bool,

    /// Remember window position when moving to/from special workspace (default: true)
    #[serde(default = "default_true")]
    pub remember_position: bool,
}

fn default_special_name() -> String {
    "special".to_string()
}

fn default_animation_duration() -> u64 {
    250
}

fn default_operation_delay() -> u64 {
    100
}

fn default_true() -> bool {
    true
}

impl Default for ToggleSpecialConfig {
    fn default() -> Self {
        Self {
            default_special_name: "special".to_string(),
            animation_duration: 250,
            operation_delay: 100,
            debug_logging: false,
            enable_animations: true,
            auto_close_empty: true,
            remember_position: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct WindowInfo {
    pub address: String,
    pub title: String,
    pub class: String,
    pub workspace_id: i32,
    pub workspace_name: String,
    pub focused: bool,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, Clone)]
pub struct SpecialWorkspaceState {
    pub name: String,
    pub windows: Vec<String>, // Window addresses
    pub visible: bool,
    pub last_focused_window: Option<String>,
}

pub struct ToggleSpecialPlugin {
    config: ToggleSpecialConfig,
    current_windows: HashMap<String, WindowInfo>, // address -> WindowInfo
    special_workspaces: HashMap<String, SpecialWorkspaceState>, // workspace_name -> state
    window_positions: HashMap<String, (i32, i32, i32, i32)>, // address -> (x, y, w, h)
    hyprland_client: Arc<Mutex<Option<Arc<HyprlandClient>>>>,
    last_operation_time: Option<Instant>,
}

impl ToggleSpecialPlugin {
    pub fn new() -> Self {
        Self {
            config: ToggleSpecialConfig::default(),
            current_windows: HashMap::new(),
            special_workspaces: HashMap::new(),
            window_positions: HashMap::new(),
            hyprland_client: Arc::new(Mutex::new(None)),
            last_operation_time: None,
        }
    }

    /// Update window information from Hyprland
    async fn update_windows(&mut self) -> Result<()> {
        let clients = tokio::task::spawn_blocking(|| Clients::get()).await??;
        let client_vec = clients.to_vec();

        self.current_windows.clear();

        for client in client_vec {
            let window_info = WindowInfo {
                address: client.address.to_string(),
                title: client.title.clone(),
                class: client.class.clone(),
                workspace_id: client.workspace.id,
                workspace_name: client.workspace.name.clone(),
                focused: client.focus_history_id == 0, // Most recent focus
                x: client.at.0.into(),
                y: client.at.1.into(),
                width: client.size.0.into(),
                height: client.size.1.into(),
            };

            // Store position if this window isn't in a special workspace and position tracking is enabled
            if self.config.remember_position && !client.workspace.name.starts_with("special") {
                self.window_positions.insert(
                    client.address.to_string(),
                    (
                        client.at.0.into(),
                        client.at.1.into(),
                        client.size.0.into(),
                        client.size.1.into(),
                    ),
                );
            }

            self.current_windows
                .insert(client.address.to_string(), window_info);
        }

        // Update special workspace states
        self.update_special_workspace_states().await?;

        if self.config.debug_logging {
            debug!("🪟 Updated {} windows", self.current_windows.len());
        }

        Ok(())
    }

    /// Update special workspace states based on current windows
    async fn update_special_workspace_states(&mut self) -> Result<()> {
        // Get all workspaces to determine which special ones are visible
        let workspaces = tokio::task::spawn_blocking(|| Workspaces::get()).await??;
        let workspace_vec = workspaces.to_vec();

        // Track visible special workspaces
        let mut visible_specials = std::collections::HashSet::new();
        for workspace in workspace_vec {
            if workspace.name.starts_with("special") {
                visible_specials.insert(workspace.name.clone());
            }
        }

        // Update special workspace states
        for (_, window) in &self.current_windows {
            if window.workspace_name.starts_with("special") {
                let workspace_name = window.workspace_name.clone();

                let state = self
                    .special_workspaces
                    .entry(workspace_name.clone())
                    .or_insert_with(|| SpecialWorkspaceState {
                        name: workspace_name.clone(),
                        windows: Vec::new(),
                        visible: visible_specials.contains(&workspace_name),
                        last_focused_window: None,
                    });

                if !state.windows.contains(&window.address) {
                    state.windows.push(window.address.clone());
                }

                if window.focused {
                    state.last_focused_window = Some(window.address.clone());
                }

                state.visible = visible_specials.contains(&workspace_name);
            }
        }

        // Clean up empty special workspaces
        self.special_workspaces.retain(|_, state| {
            // Remove windows that no longer exist
            state
                .windows
                .retain(|addr| self.current_windows.contains_key(addr));

            // Keep workspace if it has windows or if it's visible
            !state.windows.is_empty() || state.visible
        });

        if self.config.debug_logging {
            debug!(
                "🎯 Tracking {} special workspaces",
                self.special_workspaces.len()
            );
        }

        Ok(())
    }

    /// Check if enough time has passed since last operation (debouncing)
    fn can_perform_operation(&self) -> bool {
        if let Some(last_time) = self.last_operation_time {
            let elapsed = last_time.elapsed();
            elapsed.as_millis() >= self.config.operation_delay as u128
        } else {
            true
        }
    }

    /// Get the currently focused window
    fn get_focused_window(&self) -> Option<&WindowInfo> {
        self.current_windows.values().find(|w| w.focused)
    }

    /// Check if a window is in a special workspace
    fn is_window_in_special(&self, window_address: &str) -> bool {
        if let Some(window) = self.current_windows.get(window_address) {
            window.workspace_name.starts_with("special")
        } else {
            false
        }
    }

    /// Get the special workspace name for a window
    fn get_window_special_workspace(&self, window_address: &str) -> Option<String> {
        if let Some(window) = self.current_windows.get(window_address) {
            if window.workspace_name.starts_with("special") {
                Some(window.workspace_name.clone())
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Add transition animation delay if enabled
    async fn animate_transition(&self) -> Result<()> {
        if !self.config.enable_animations {
            return Ok(());
        }

        if self.config.debug_logging {
            debug!(
                "🎬 Starting transition animation ({}ms)",
                self.config.animation_duration
            );
        }

        sleep(Duration::from_millis(self.config.animation_duration)).await;

        Ok(())
    }

    /// Move focused window to special workspace
    async fn move_to_special(&mut self, special_name: &str) -> Result<String> {
        // Check debouncing
        if !self.can_perform_operation() {
            if self.config.debug_logging {
                debug!("🚫 Operation debounced (too soon since last operation)");
            }
            return Ok("Operation debounced".to_string());
        }

        // Update window state
        self.update_windows().await?;

        let focused_window = match self.get_focused_window() {
            Some(window) => window.clone(),
            None => return Err(anyhow::anyhow!("No focused window found")),
        };

        if self.config.debug_logging {
            debug!(
                "📦 Moving window '{}' ({}) to special workspace '{}'",
                focused_window.title, focused_window.address, special_name
            );
        }

        // Store current position if enabled
        if self.config.remember_position {
            self.window_positions.insert(
                focused_window.address.clone(),
                (
                    focused_window.x,
                    focused_window.y,
                    focused_window.width,
                    focused_window.height,
                ),
            );
        }

        // Move window to special workspace
        let special_workspace = format!("special:{}", special_name);

        tokio::task::spawn_blocking(move || {
            let workspace_identifier = WorkspaceIdentifierWithSpecial::Name(&special_workspace);
            Dispatch::call(DispatchType::MoveToWorkspace(workspace_identifier, None))
        })
        .await??;

        // Add animation delay
        self.animate_transition().await?;

        // Update last operation time
        self.last_operation_time = Some(Instant::now());

        info!(
            "📦 Moved window '{}' to special workspace '{}'",
            focused_window.title, special_name
        );

        Ok(format!(
            "Moved window '{}' to special workspace '{}'",
            focused_window.title, special_name
        ))
    }

    /// Move window from special workspace back to regular workspace
    async fn move_from_special(&mut self, window_address: &str) -> Result<String> {
        // Check debouncing
        if !self.can_perform_operation() {
            if self.config.debug_logging {
                debug!("🚫 Operation debounced (too soon since last operation)");
            }
            return Ok("Operation debounced".to_string());
        }

        // Update window state
        self.update_windows().await?;

        let window = match self.current_windows.get(window_address) {
            Some(window) => window.clone(),
            None => return Err(anyhow::anyhow!("Window not found: {}", window_address)),
        };

        if self.config.debug_logging {
            debug!(
                "📤 Moving window '{}' ({}) from special workspace back to regular workspace",
                window.title, window.address
            );
        }

        // Move window to workspace 1 (or current workspace)
        let workspace_identifier = WorkspaceIdentifierWithSpecial::Id(1);

        tokio::task::spawn_blocking(move || {
            Dispatch::call(DispatchType::MoveToWorkspace(workspace_identifier, None))
        })
        .await??;

        // Add animation delay
        self.animate_transition().await?;

        // Update last operation time
        self.last_operation_time = Some(Instant::now());

        info!(
            "📤 Moved window '{}' from special workspace back to regular workspace",
            window.title
        );

        Ok(format!(
            "Moved window '{}' from special workspace back to regular workspace",
            window.title
        ))
    }

    /// Toggle special workspace visibility
    async fn toggle_special_visibility(&mut self, special_name: &str) -> Result<String> {
        if self.config.debug_logging {
            debug!(
                "👁️ Toggling visibility of special workspace '{}'",
                special_name
            );
        }

        // Use Hyprland's togglespecialworkspace command
        let special_workspace = if special_name == "special" {
            "special".to_string()
        } else {
            format!("special:{}", special_name)
        };

        tokio::task::spawn_blocking(move || {
            Dispatch::call(DispatchType::ToggleSpecialWorkspace(Some(
                special_workspace,
            )))
        })
        .await??;

        // Add animation delay
        self.animate_transition().await?;

        // Update last operation time
        self.last_operation_time = Some(Instant::now());

        Ok(format!(
            "Toggled visibility of special workspace '{}'",
            special_name
        ))
    }

    /// Main toggle function - intelligently decides what to do
    async fn toggle_special(&mut self, special_name: Option<&str>) -> Result<String> {
        let special_name = special_name
            .unwrap_or(&self.config.default_special_name)
            .to_string();

        // Update window state
        self.update_windows().await?;

        let focused_window = self.get_focused_window().cloned();

        match focused_window {
            Some(window) => {
                if self.is_window_in_special(&window.address) {
                    // Window is in special workspace - move it back to regular workspace
                    self.move_from_special(&window.address).await
                } else {
                    // Window is in regular workspace - move it to special workspace
                    self.move_to_special(&special_name).await
                }
            }
            None => {
                // No focused window - just toggle special workspace visibility
                self.toggle_special_visibility(&special_name).await
            }
        }
    }

    /// List all special workspaces and their windows
    async fn list_special_workspaces(&mut self) -> Result<String> {
        self.update_windows().await?;

        let mut output = String::from("🎯 Special Workspaces:\n");

        if self.special_workspaces.is_empty() {
            output.push_str("  No special workspaces currently active\n");
        } else {
            for (name, state) in &self.special_workspaces {
                let visibility = if state.visible {
                    "👁️  visible"
                } else {
                    "🙈 hidden"
                };
                output.push_str(&format!(
                    "  {} ({}) - {} windows {}\n",
                    name,
                    visibility,
                    state.windows.len(),
                    if state.windows.len() > 0 { ":" } else { "" }
                ));

                for window_addr in &state.windows {
                    if let Some(window) = self.current_windows.get(window_addr) {
                        let focused_marker = if window.focused { "🎯" } else { "  " };
                        output.push_str(&format!(
                            "    {} {} ({})\n",
                            focused_marker, window.title, window.class
                        ));
                    }
                }
            }
        }

        output.push_str(&format!(
            "\nConfig: default='{}', animations={}, auto-close={}\n",
            self.config.default_special_name,
            self.config.enable_animations,
            self.config.auto_close_empty
        ));

        Ok(output)
    }

    /// Get status of toggle_special plugin
    async fn get_status(&mut self) -> Result<String> {
        self.update_windows().await?;

        let total_windows = self.current_windows.len();
        let special_workspaces_count = self.special_workspaces.len();
        let special_windows_count: usize = self
            .special_workspaces
            .values()
            .map(|state| state.windows.len())
            .sum();

        let focused_window = self.get_focused_window();
        let focused_status = match focused_window {
            Some(window) => {
                if self.is_window_in_special(&window.address) {
                    format!("'{}' (in special)", window.title)
                } else {
                    format!("'{}' (regular)", window.title)
                }
            }
            None => "None".to_string(),
        };

        let mut status = format!(
            "ToggleSpecial: {} total windows, {} special workspaces, {} windows in special\n",
            total_windows, special_workspaces_count, special_windows_count
        );

        status.push_str(&format!("Focused window: {}\n", focused_status));

        status.push_str(&format!(
            "Config:\n  - Default special: '{}'\n  - Animations: {} ({}ms)\n  - Operation delay: {}ms\n",
            self.config.default_special_name,
            self.config.enable_animations,
            self.config.animation_duration,
            self.config.operation_delay
        ));

        status.push_str(&format!(
            "  - Auto-close empty: {}\n  - Remember position: {}\n  - Debug logging: {}\n",
            self.config.auto_close_empty, self.config.remember_position, self.config.debug_logging
        ));

        Ok(status)
    }
}

#[async_trait]
impl Plugin for ToggleSpecialPlugin {
    fn name(&self) -> &str {
        "toggle_special"
    }

    async fn init(&mut self, config: &toml::Value) -> Result<()> {
        info!("🎯 Initializing toggle_special plugin");

        if let Some(plugin_config) = config.get("toggle_special") {
            match plugin_config.clone().try_into() {
                Ok(config) => self.config = config,
                Err(e) => {
                    return Err(anyhow::anyhow!(
                        "Invalid toggle_special configuration: {}",
                        e
                    ))
                }
            }
        }

        debug!("ToggleSpecial config: {:?}", self.config);

        // Initialize window state
        self.update_windows().await?;

        info!(
            "✅ ToggleSpecial plugin initialized. Default special workspace: '{}'",
            self.config.default_special_name
        );

        Ok(())
    }

    async fn handle_event(&mut self, event: &HyprlandEvent) -> Result<()> {
        match event {
            HyprlandEvent::WorkspaceChanged { workspace: _ } => {
                // Update window state when workspace changes
                self.update_windows().await?;
            }

            HyprlandEvent::WindowOpened { window: _ } => {
                // Update window state when new windows are opened
                self.update_windows().await?;
            }

            HyprlandEvent::WindowClosed { window: _ } => {
                // Update window state when windows are closed
                self.update_windows().await?;

                // Clean up window positions for closed windows
                if self.config.remember_position {
                    self.window_positions
                        .retain(|addr, _| self.current_windows.contains_key(addr));
                }
            }

            HyprlandEvent::WindowMoved { window: _ } => {
                // Update window state when windows are moved
                self.update_windows().await?;
            }

            _ => {}
        }

        Ok(())
    }

    async fn handle_command(&mut self, command: &str, args: &[&str]) -> Result<String> {
        debug!("🎯 ToggleSpecial command: {} {:?}", command, args);

        match command {
            "" | "toggle" => {
                // Main toggle command - use first arg as special workspace name
                let special_name = args.first().copied();
                self.toggle_special(special_name).await
            }

            "show" => {
                // Show special workspace
                let default_name = self.config.default_special_name.clone();
                let special_name = args.first().map_or(default_name.as_str(), |s| s);
                self.toggle_special_visibility(special_name).await
            }

            "move" => {
                // Move focused window to special workspace
                let default_name = self.config.default_special_name.clone();
                let special_name = args.first().map_or(default_name.as_str(), |s| s);
                self.move_to_special(special_name).await
            }

            "list" => self.list_special_workspaces().await,
            "status" => self.get_status().await,

            _ => Ok(format!("Unknown toggle_special command: {}", command)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_plugin() -> ToggleSpecialPlugin {
        ToggleSpecialPlugin::new()
    }

    fn create_test_config() -> ToggleSpecialConfig {
        let mut config = ToggleSpecialConfig::default();
        config.operation_delay = 50;
        config.animation_duration = 100;
        config.debug_logging = true;
        config.default_special_name = "test".to_string();
        config
    }

    #[test]
    fn test_plugin_creation() {
        let plugin = create_test_plugin();
        assert_eq!(plugin.name(), "toggle_special");
        assert_eq!(plugin.current_windows.len(), 0);
        assert_eq!(plugin.special_workspaces.len(), 0);
        assert!(plugin.last_operation_time.is_none());
    }

    #[test]
    fn test_config_defaults() {
        let config = ToggleSpecialConfig::default();
        assert_eq!(config.default_special_name, "special");
        assert_eq!(config.animation_duration, 250);
        assert_eq!(config.operation_delay, 100);
        assert!(!config.debug_logging);
        assert!(config.enable_animations);
        assert!(config.auto_close_empty);
        assert!(config.remember_position);
    }

    #[test]
    fn test_operation_debounce() {
        let mut plugin = create_test_plugin();
        plugin.config = create_test_config();

        // Initially should allow operations
        assert!(plugin.can_perform_operation());

        // After setting last operation time, should debounce
        plugin.last_operation_time = Some(Instant::now());
        assert!(!plugin.can_perform_operation());

        // After enough time, should allow again
        plugin.last_operation_time = Some(Instant::now() - Duration::from_millis(100));
        assert!(plugin.can_perform_operation());
    }

    #[test]
    fn test_window_info_structure() {
        let window = WindowInfo {
            address: "0x12345".to_string(),
            title: "Test Window".to_string(),
            class: "test-app".to_string(),
            workspace_id: 1,
            workspace_name: "1".to_string(),
            focused: true,
            x: 100,
            y: 200,
            width: 800,
            height: 600,
        };

        assert_eq!(window.address, "0x12345");
        assert_eq!(window.title, "Test Window");
        assert_eq!(window.class, "test-app");
        assert_eq!(window.workspace_id, 1);
        assert!(window.focused);
        assert_eq!(window.x, 100);
        assert_eq!(window.y, 200);
        assert_eq!(window.width, 800);
        assert_eq!(window.height, 600);
    }

    #[test]
    fn test_special_workspace_state() {
        let state = SpecialWorkspaceState {
            name: "special:test".to_string(),
            windows: vec!["0x12345".to_string(), "0x67890".to_string()],
            visible: true,
            last_focused_window: Some("0x12345".to_string()),
        };

        assert_eq!(state.name, "special:test");
        assert_eq!(state.windows.len(), 2);
        assert!(state.visible);
        assert_eq!(state.last_focused_window, Some("0x12345".to_string()));
    }

    #[test]
    fn test_window_position_tracking() {
        let mut plugin = create_test_plugin();
        plugin.config.remember_position = true;

        // Add a window position
        plugin
            .window_positions
            .insert("0x12345".to_string(), (100, 200, 800, 600));

        assert_eq!(plugin.window_positions.len(), 1);
        assert_eq!(
            plugin.window_positions.get("0x12345"),
            Some(&(100, 200, 800, 600))
        );
    }

    #[test]
    fn test_is_window_in_special() {
        let mut plugin = create_test_plugin();

        // Add a regular window
        plugin.current_windows.insert(
            "0x12345".to_string(),
            WindowInfo {
                address: "0x12345".to_string(),
                title: "Regular Window".to_string(),
                class: "test-app".to_string(),
                workspace_id: 1,
                workspace_name: "1".to_string(),
                focused: false,
                x: 0,
                y: 0,
                width: 800,
                height: 600,
            },
        );

        // Add a special window
        plugin.current_windows.insert(
            "0x67890".to_string(),
            WindowInfo {
                address: "0x67890".to_string(),
                title: "Special Window".to_string(),
                class: "test-app".to_string(),
                workspace_id: -99,
                workspace_name: "special:test".to_string(),
                focused: false,
                x: 0,
                y: 0,
                width: 800,
                height: 600,
            },
        );

        assert!(!plugin.is_window_in_special("0x12345"));
        assert!(plugin.is_window_in_special("0x67890"));
        assert!(!plugin.is_window_in_special("0xnonexistent"));
    }

    #[test]
    fn test_get_window_special_workspace() {
        let mut plugin = create_test_plugin();

        // Add a special window
        plugin.current_windows.insert(
            "0x67890".to_string(),
            WindowInfo {
                address: "0x67890".to_string(),
                title: "Special Window".to_string(),
                class: "test-app".to_string(),
                workspace_id: -99,
                workspace_name: "special:minimized".to_string(),
                focused: false,
                x: 0,
                y: 0,
                width: 800,
                height: 600,
            },
        );

        assert_eq!(
            plugin.get_window_special_workspace("0x67890"),
            Some("special:minimized".to_string())
        );
        assert_eq!(plugin.get_window_special_workspace("0xnonexistent"), None);
    }

    #[test]
    fn test_config_serialization() {
        let config = create_test_config();

        // Test that config can be serialized to TOML
        let toml_str = toml::to_string(&config).expect("Failed to serialize config");
        assert!(toml_str.contains("default_special_name"));
        assert!(toml_str.contains("animation_duration"));
        assert!(toml_str.contains("operation_delay"));
        assert!(toml_str.contains("debug_logging"));
        assert!(toml_str.contains("enable_animations"));
        assert!(toml_str.contains("auto_close_empty"));
        assert!(toml_str.contains("remember_position"));

        // Test that it can be deserialized back
        let _deserialized: ToggleSpecialConfig =
            toml::from_str(&toml_str).expect("Failed to deserialize config");
    }

    #[test]
    fn test_default_functions() {
        assert_eq!(default_special_name(), "special");
        assert_eq!(default_animation_duration(), 250);
        assert_eq!(default_operation_delay(), 100);
        assert_eq!(default_true(), true);
    }

    #[test]
    fn test_focused_window_detection() {
        let mut plugin = create_test_plugin();

        // Add windows, one focused
        plugin.current_windows.insert(
            "0x12345".to_string(),
            WindowInfo {
                address: "0x12345".to_string(),
                title: "Unfocused Window".to_string(),
                class: "test-app".to_string(),
                workspace_id: 1,
                workspace_name: "1".to_string(),
                focused: false,
                x: 0,
                y: 0,
                width: 800,
                height: 600,
            },
        );

        plugin.current_windows.insert(
            "0x67890".to_string(),
            WindowInfo {
                address: "0x67890".to_string(),
                title: "Focused Window".to_string(),
                class: "test-app".to_string(),
                workspace_id: 1,
                workspace_name: "1".to_string(),
                focused: true,
                x: 0,
                y: 0,
                width: 800,
                height: 600,
            },
        );

        let focused = plugin.get_focused_window();
        assert!(focused.is_some());
        assert_eq!(focused.unwrap().title, "Focused Window");
        assert_eq!(focused.unwrap().address, "0x67890");
    }
}
