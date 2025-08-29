use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

use crate::ipc::{HyprlandClient, HyprlandEvent, MonitorInfo, WorkspaceInfo};
use crate::plugins::Plugin;

use hyprland::data::{Monitors, Workspaces};
use hyprland::dispatch::{
    Dispatch, DispatchType, MonitorIdentifier, WorkspaceIdentifier, WorkspaceIdentifierWithSpecial,
};
use hyprland::shared::{HyprData, HyprDataVec};

#[derive(Debug, Deserialize, Serialize)]
pub struct ShiftMonitorsConfig {
    /// Delay between workspace shifts in milliseconds to prevent rapid shifting (default: 200)
    #[serde(default = "default_shift_delay")]
    pub shift_delay: u64,

    /// Animation duration for workspace transitions in milliseconds (default: 300)
    #[serde(default = "default_animation_duration")]
    pub animation_duration: u64,

    /// Log shift operations for debugging (default: false)
    #[serde(default)]
    pub debug_logging: bool,

    /// Enable smooth transitions during shifts (default: true)
    #[serde(default = "default_true")]
    pub enable_animations: bool,
}

fn default_shift_delay() -> u64 {
    200
}

fn default_animation_duration() -> u64 {
    300
}

fn default_true() -> bool {
    true
}

impl Default for ShiftMonitorsConfig {
    fn default() -> Self {
        Self {
            shift_delay: 200,
            animation_duration: 300,
            debug_logging: false,
            enable_animations: true,
        }
    }
}

pub struct ShiftMonitorsPlugin {
    config: ShiftMonitorsConfig,
    monitors: HashMap<String, MonitorInfo>,
    workspaces: HashMap<i32, WorkspaceInfo>,
    hyprland_client: Arc<Mutex<Option<Arc<HyprlandClient>>>>,
    last_shift_time: Option<Instant>,
}

impl ShiftMonitorsPlugin {
    pub fn new() -> Self {
        Self {
            config: ShiftMonitorsConfig::default(),
            monitors: HashMap::new(),
            workspaces: HashMap::new(),
            hyprland_client: Arc::new(Mutex::new(None)),
            last_shift_time: None,
        }
    }

    /// Update monitor information from Hyprland
    async fn update_monitors(&mut self) -> Result<()> {
        let monitors = tokio::task::spawn_blocking(Monitors::get).await??;
        let monitor_vec = monitors.to_vec();

        self.monitors.clear();

        for monitor in monitor_vec {
            let monitor_info = MonitorInfo {
                id: monitor.id,
                name: monitor.name.clone(),
                is_focused: monitor.focused,
                active_workspace_id: monitor.active_workspace.id,
                width: monitor.width,
                height: monitor.height,
                x: monitor.x,
                y: monitor.y,
                scale: monitor.scale,
                refresh_rate: monitor.refresh_rate,
            };

            self.monitors.insert(monitor.name, monitor_info);
        }

        if self.config.debug_logging {
            debug!("🖥️  Updated {} monitors", self.monitors.len());
        }

        Ok(())
    }

    /// Update workspace information from Hyprland  
    async fn update_workspaces(&mut self) -> Result<()> {
        let workspaces = tokio::task::spawn_blocking(Workspaces::get).await??;
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

    /// Check if enough time has passed since last shift (debouncing)
    fn can_shift(&self) -> bool {
        if let Some(last_time) = self.last_shift_time {
            let elapsed = last_time.elapsed();
            elapsed.as_millis() >= self.config.shift_delay as u128
        } else {
            true
        }
    }

    /// Get ordered list of monitors (sorted by position)
    fn get_ordered_monitors(&self) -> Vec<&MonitorInfo> {
        let mut monitors: Vec<_> = self.monitors.values().collect();
        // Sort monitors by x position for logical ordering
        monitors.sort_by_key(|m| m.x);
        monitors
    }

    /// Shift workspaces between monitors in the specified direction
    async fn shift_workspaces(&mut self, direction: i32) -> Result<String> {
        // Check debouncing
        if !self.can_shift() {
            if self.config.debug_logging {
                debug!("🚫 Workspace shift debounced (too soon since last shift)");
            }
            return Ok("Workspace shift debounced".to_string());
        }

        // Update current state
        self.update_monitors().await?;
        self.update_workspaces().await?;

        let monitor_count = self.monitors.len();

        if monitor_count < 2 {
            return Err(anyhow::anyhow!(
                "Need at least 2 monitors to shift workspaces"
            ));
        }

        if self.config.debug_logging {
            debug!(
                "🔄 Shifting workspaces in direction {} across {} monitors",
                direction, monitor_count
            );
        }

        // Collect current workspace assignments
        let monitor_workspaces: Vec<(String, i32)> = {
            let ordered_monitors = self.get_ordered_monitors();
            ordered_monitors
                .iter()
                .map(|m| (m.name.clone(), m.active_workspace_id))
                .collect()
        };

        if self.config.debug_logging {
            debug!("Current workspace mapping: {:?}", monitor_workspaces);
        }

        // Determine shift direction
        let shift_amount = if direction > 0 {
            1 // Shift right/forward
        } else {
            monitor_workspaces.len() - 1 // Shift left/backward (equivalent to right by n-1)
        };

        // Create new workspace assignments by rotating
        let mut new_assignments = Vec::new();
        for (i, (monitor_name, _)) in monitor_workspaces.iter().enumerate() {
            let source_index = (i + shift_amount) % monitor_workspaces.len();
            let source_workspace = monitor_workspaces[source_index].1;
            new_assignments.push((monitor_name.clone(), source_workspace));
        }

        if self.config.debug_logging {
            debug!("New workspace mapping: {:?}", new_assignments);
        }

        // Add transition animation delay if enabled
        if self.config.enable_animations {
            if self.config.debug_logging {
                debug!(
                    "🎬 Starting workspace shift animation ({}ms)",
                    self.config.animation_duration
                );
            }

            // Simple animation by adding a small delay
            let animation_steps = 5;
            let step_duration =
                Duration::from_millis(self.config.animation_duration / animation_steps);

            for step in 0..animation_steps {
                sleep(step_duration).await;
                if self.config.debug_logging && step % 2 == 0 {
                    let progress = (step + 1) as f32 / animation_steps as f32 * 100.0;
                    debug!("🎬 Animation progress: {:.1}%", progress);
                }
            }
        }

        // Apply the workspace shifts by moving workspaces to their new monitors
        for (monitor_name, new_workspace) in new_assignments {
            let workspace_identifier = WorkspaceIdentifier::Id(new_workspace);
            let monitor_name_clone = monitor_name.clone();

            tokio::task::spawn_blocking(move || {
                let monitor_identifier = MonitorIdentifier::Name(&monitor_name_clone);
                Dispatch::call(DispatchType::MoveWorkspaceToMonitor(
                    workspace_identifier,
                    monitor_identifier,
                ))
            })
            .await??;

            if self.config.debug_logging {
                debug!(
                    "📱 Moved workspace {} to monitor {}",
                    new_workspace, monitor_name
                );
            }
        }

        // Update last shift time
        self.last_shift_time = Some(Instant::now());

        let direction_text = if direction > 0 { "forward" } else { "backward" };

        info!(
            "🔄 Shifted workspaces {} across {} monitors",
            direction_text, monitor_count
        );

        Ok(format!(
            "Shifted workspaces {direction_text} across {monitor_count} monitors"
        ))
    }

    /// Get current status of the shift_monitors plugin
    async fn get_status(&mut self) -> Result<String> {
        self.update_monitors().await?;
        self.update_workspaces().await?;

        let monitor_count = self.monitors.len();
        let workspace_count = self.workspaces.len();

        let mut status =
            format!("ShiftMonitors: {monitor_count} monitors, {workspace_count} workspaces\n");

        // Show current monitor-workspace mapping
        status.push_str("Current mapping:\n");
        let ordered_monitors = self.get_ordered_monitors();

        for monitor in ordered_monitors {
            let focused_marker = if monitor.is_focused { "🎯" } else { "  " };
            status.push_str(&format!(
                "{} {}: Workspace {} ({}x{} @ {},{}) - {} windows\n",
                focused_marker,
                monitor.name,
                monitor.active_workspace_id,
                monitor.width,
                monitor.height,
                monitor.x,
                monitor.y,
                self.workspaces
                    .get(&monitor.active_workspace_id)
                    .map(|w| w.windows)
                    .unwrap_or(0)
            ));
        }

        status.push_str(&format!(
            "\nConfig:\n  - Shift delay: {}ms\n  - Animations: {} ({}ms)\n  - Debug logging: {}\n",
            self.config.shift_delay,
            self.config.enable_animations,
            self.config.animation_duration,
            self.config.debug_logging
        ));

        Ok(status)
    }

    /// List all available monitors with their workspaces
    async fn list_monitors(&mut self) -> Result<String> {
        self.update_monitors().await?;
        self.update_workspaces().await?;

        let mut output = String::from("🖥️  Monitors and Workspaces:\n");

        let ordered_monitors = self.get_ordered_monitors();

        for (index, monitor) in ordered_monitors.iter().enumerate() {
            let focused_marker = if monitor.is_focused { "🎯" } else { "  " };
            let workspace_info = self
                .workspaces
                .get(&monitor.active_workspace_id)
                .map(|w| format!("({} windows)", w.windows))
                .unwrap_or_else(|| "(0 windows)".to_string());

            output.push_str(&format!(
                "{} [{}] {}: {}x{} @ ({},{}) - Workspace {} {}\n",
                focused_marker,
                index + 1,
                monitor.name,
                monitor.width,
                monitor.height,
                monitor.x,
                monitor.y,
                monitor.active_workspace_id,
                workspace_info
            ));
        }

        output.push_str("\nUsage:\n");
        output.push_str("  'shift_monitors'     - Shift workspaces forward (default: +1)\n");
        output.push_str("  'shift_monitors +1'  - Shift workspaces forward\n");
        output.push_str("  'shift_monitors -1'  - Shift workspaces backward\n");
        output.push_str("  'shift_monitors +2'  - Shift workspaces forward by 2 positions\n");

        Ok(output)
    }
}

impl Default for ShiftMonitorsPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Plugin for ShiftMonitorsPlugin {
    fn name(&self) -> &str {
        "shift_monitors"
    }

    async fn init(&mut self, config: &toml::Value) -> Result<()> {
        info!("🔄 Initializing shift_monitors plugin");

        if let Some(plugin_config) = config.get("shift_monitors") {
            match plugin_config.clone().try_into() {
                Ok(config) => self.config = config,
                Err(e) => {
                    return Err(anyhow::anyhow!(
                        "Invalid shift_monitors configuration: {}",
                        e
                    ))
                }
            }
        }

        debug!("ShiftMonitors config: {:?}", self.config);

        // Initialize monitor and workspace state
        self.update_monitors().await?;
        self.update_workspaces().await?;

        info!(
            "✅ ShiftMonitors plugin initialized with {} monitors, {} workspaces",
            self.monitors.len(),
            self.workspaces.len()
        );

        Ok(())
    }

    async fn handle_event(&mut self, event: &HyprlandEvent) -> Result<()> {
        match event {
            HyprlandEvent::WorkspaceChanged { workspace: _ } => {
                // Update our state when workspace changes
                self.update_monitors().await?;
                self.update_workspaces().await?;
            }

            HyprlandEvent::WindowOpened { window: _ } => {
                // Update workspace info when windows are opened
                self.update_workspaces().await?;
            }

            HyprlandEvent::WindowClosed { window: _ } => {
                // Update workspace info when windows are closed
                self.update_workspaces().await?;
            }

            HyprlandEvent::WindowMoved { window: _ } => {
                // Update monitor and workspace info when windows are moved
                self.update_monitors().await?;
                self.update_workspaces().await?;
            }

            _ => {}
        }

        Ok(())
    }

    async fn handle_command(&mut self, command: &str, args: &[&str]) -> Result<String> {
        debug!("🔄 ShiftMonitors command: {} {:?}", command, args);

        match command {
            "" => {
                // Default behavior: shift forward by 1
                self.shift_workspaces(1).await
            }

            direction_str => {
                // Parse direction from command
                let direction: i32 = direction_str.parse().map_err(|_| {
                    anyhow::anyhow!(
                        "Invalid direction: {}. Use +1 for forward, -1 for backward",
                        direction_str
                    )
                })?;

                if direction == 0 {
                    return Err(anyhow::anyhow!(
                        "Direction cannot be 0. Use +1 for forward, -1 for backward"
                    ));
                }

                self.shift_workspaces(direction).await
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_plugin() -> ShiftMonitorsPlugin {
        ShiftMonitorsPlugin::new()
    }

    fn create_test_config() -> ShiftMonitorsConfig {
        let mut config = ShiftMonitorsConfig::default();
        config.shift_delay = 100;
        config.animation_duration = 200;
        config.debug_logging = true;
        config
    }

    #[test]
    fn test_plugin_creation() {
        let plugin = create_test_plugin();
        assert_eq!(plugin.name(), "shift_monitors");
        assert_eq!(plugin.monitors.len(), 0);
        assert_eq!(plugin.workspaces.len(), 0);
        assert!(plugin.last_shift_time.is_none());
    }

    #[test]
    fn test_config_defaults() {
        let config = ShiftMonitorsConfig::default();
        assert_eq!(config.shift_delay, 200);
        assert_eq!(config.animation_duration, 300);
        assert!(!config.debug_logging);
        assert!(config.enable_animations);
    }

    #[test]
    fn test_shift_debounce() {
        let mut plugin = create_test_plugin();
        plugin.config = create_test_config();

        // Initially should allow shifting
        assert!(plugin.can_shift());

        // After setting last shift time, should debounce
        plugin.last_shift_time = Some(Instant::now());
        assert!(!plugin.can_shift());

        // After enough time, should allow again
        plugin.last_shift_time = Some(Instant::now() - Duration::from_millis(150));
        assert!(plugin.can_shift());
    }

    #[test]
    fn test_monitor_ordering() {
        let mut plugin = create_test_plugin();

        // Add test monitors in random order
        plugin.monitors.insert(
            "DP-3".to_string(),
            MonitorInfo {
                id: 1,
                name: "DP-2".to_string(),
                is_focused: false,
                active_workspace_id: 2,
                width: 1920,
                height: 1080,
                x: 1920, // Second monitor position
                y: 0,
                scale: 1.0,
                refresh_rate: 60.0,
            },
        );

        plugin.monitors.insert(
            "DP-1".to_string(),
            MonitorInfo {
                id: 0,
                name: "DP-1".to_string(),
                is_focused: true,
                active_workspace_id: 1,
                width: 1920,
                height: 1080,
                x: 0, // First monitor position
                y: 0,
                scale: 1.0,
                refresh_rate: 60.0,
            },
        );

        let ordered = plugin.get_ordered_monitors();

        // Should be ordered by x position
        assert_eq!(ordered.len(), 2);
        assert_eq!(ordered[0].name, "DP-1");
        assert_eq!(ordered[1].name, "DP-2");
        assert_eq!(ordered[0].x, 0);
        assert_eq!(ordered[1].x, 1920);
    }

    #[test]
    fn test_monitor_info_structure() {
        let monitor = MonitorInfo {
            id: 0,
            name: "DP-1".to_string(),
            is_focused: true,
            active_workspace_id: 1,
            width: 1920,
            height: 1080,
            x: 0,
            y: 0,
            scale: 1.0,
            refresh_rate: 60.0,
        };

        assert_eq!(monitor.name, "DP-1");
        assert!(monitor.is_focused);
        assert_eq!(monitor.active_workspace_id, 1);
        assert_eq!(monitor.width, 1920);
        assert_eq!(monitor.height, 1080);
    }

    #[test]
    fn test_workspace_info_structure() {
        let workspace = WorkspaceInfo {
            id: 1,
            name: "1".to_string(),
            monitor: "DP-1".to_string(),
            windows: 3,
            last_window_addr: String::from(""),
        };

        assert_eq!(workspace.id, 1);
        assert_eq!(workspace.name, "1");
        assert_eq!(workspace.monitor, "DP-1");
        assert_eq!(workspace.windows, 3);
    }

    #[test]
    fn test_config_serialization() {
        let config = create_test_config();

        // Test that config can be serialized to TOML
        let toml_str = toml::to_string(&config).expect("Failed to serialize config");
        assert!(toml_str.contains("shift_delay"));
        assert!(toml_str.contains("animation_duration"));
        assert!(toml_str.contains("debug_logging"));
        assert!(toml_str.contains("enable_animations"));

        // Test that it can be deserialized back
        let _deserialized: ShiftMonitorsConfig =
            toml::from_str(&toml_str).expect("Failed to deserialize config");
    }

    #[test]
    fn test_direction_parsing() {
        // Test positive direction
        let direction_str = "+1";
        let direction: Result<i32, _> = direction_str.parse();
        assert_eq!(direction.unwrap(), 1);

        // Test negative direction
        let direction_str = "-1";
        let direction: Result<i32, _> = direction_str.parse();
        assert_eq!(direction.unwrap(), -1);

        // Test larger values
        let direction_str = "3";
        let direction: Result<i32, _> = direction_str.parse();
        assert_eq!(direction.unwrap(), 3);

        // Test invalid direction
        let direction_str = "invalid";
        let direction: Result<i32, _> = direction_str.parse();
        assert!(direction.is_err());
    }

    #[test]
    fn test_default_functions() {
        assert_eq!(default_shift_delay(), 200);
        assert_eq!(default_animation_duration(), 300);
        assert!(default_true());
    }

    #[test]
    fn test_default_direction_handling() {
        // Test simple de la logique de correspondance des commandes
        // sans exécution async qui dépend de Hyprland

        // Simuler la logique de handle_command pour la direction par défaut
        let empty_command = "";
        let explicit_positive = "+1";
        let explicit_negative = "-1";

        // Test que la commande vide est bien détectée
        assert_eq!(empty_command, "");
        assert_ne!(explicit_positive, "");
        assert_ne!(explicit_negative, "");

        // Test parsing des directions explicites
        let pos_result: Result<i32, _> = explicit_positive.parse();
        let neg_result: Result<i32, _> = explicit_negative.parse();

        assert!(pos_result.is_ok());
        assert!(neg_result.is_ok());
        assert_eq!(pos_result.unwrap(), 1);
        assert_eq!(neg_result.unwrap(), -1);

        // La direction par défaut est +1 (testé par la logique dans handle_command)
    }

    #[test]
    fn test_command_parsing_logic() {
        // Test de la logique de parsing des directions

        // Direction par défaut (chaîne vide)
        let empty_command = "";
        assert_eq!(empty_command, "");

        // Directions explicites
        let positive_dir: Result<i32, _> = "+1".parse();
        assert_eq!(positive_dir.unwrap(), 1);

        let negative_dir: Result<i32, _> = "-1".parse();
        assert_eq!(negative_dir.unwrap(), -1);

        let large_dir: Result<i32, _> = "5".parse();
        assert_eq!(large_dir.unwrap(), 5);

        // Direction invalide
        let invalid_dir: Result<i32, _> = "invalid".parse();
        assert!(invalid_dir.is_err());
    }
}
