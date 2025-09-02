use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

use crate::ipc::{HyprlandClient, HyprlandEvent, MonitorInfo};
use crate::plugins::Plugin;

// ============================================================================
// CONFIGURATION STRUCTURES
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LostWindowsConfig {
    /// Rescue strategy for positioning recovered windows
    #[serde(default = "default_rescue_strategy")]
    pub rescue_strategy: RescueStrategy,

    /// Enable automatic recovery of lost windows
    #[serde(default = "default_true")]
    pub auto_recovery: bool,

    /// Interval in seconds for automatic recovery checks
    #[serde(default = "default_check_interval")]
    pub check_interval: u64,

    /// Margin from screen edges in pixels
    #[serde(default = "default_margin")]
    pub margin: i32,

    /// Maximum number of windows to recover at once
    #[serde(default = "default_max_windows")]
    pub max_windows: usize,

    /// Window classes to exclude from recovery
    #[serde(default)]
    pub exclude_classes: Vec<String>,

    /// Minimum window size to consider for recovery
    #[serde(default = "default_min_size")]
    pub min_window_size: (i32, i32),

    /// Enable smooth animations for window recovery
    #[serde(default = "default_true")]
    pub enable_animations: bool,

    /// Animation duration in milliseconds
    #[serde(default = "default_animation_duration")]
    pub animation_duration: u64,

    /// Remember original window positions
    #[serde(default = "default_true")]
    pub remember_positions: bool,

    /// Only recover windows on current monitor
    #[serde(default)]
    pub current_monitor_only: bool,

    /// Debug logging for lost window detection
    #[serde(default)]
    pub debug_logging: bool,

    /// Recovery confirmation before moving windows
    #[serde(default)]
    pub require_confirmation: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RescueStrategy {
    /// Distribute windows evenly across monitor
    Distribute,
    /// Arrange windows in a grid pattern
    Grid,
    /// Cascade windows from top-left
    Cascade,
    /// Center all windows on monitor
    Center,
    /// Place at last known good position
    Restore,
    /// Smart placement avoiding overlaps
    Smart,
}

// Default functions
fn default_rescue_strategy() -> RescueStrategy {
    RescueStrategy::Smart
}

fn default_true() -> bool {
    true
}

fn default_check_interval() -> u64 {
    30 // Check every 30 seconds
}

fn default_margin() -> i32 {
    50
}

fn default_max_windows() -> usize {
    10
}

fn default_min_size() -> (i32, i32) {
    (100, 100)
}

fn default_animation_duration() -> u64 {
    300
}

impl Default for LostWindowsConfig {
    fn default() -> Self {
        Self {
            rescue_strategy: default_rescue_strategy(),
            auto_recovery: default_true(),
            check_interval: default_check_interval(),
            margin: default_margin(),
            max_windows: default_max_windows(),
            exclude_classes: Vec::new(),
            min_window_size: default_min_size(),
            enable_animations: default_true(),
            animation_duration: default_animation_duration(),
            remember_positions: default_true(),
            current_monitor_only: false,
            debug_logging: false,
            require_confirmation: false,
        }
    }
}

// ============================================================================
// WINDOW TRACKING STRUCTURES
// ============================================================================

#[derive(Debug, Clone)]
pub struct WindowInfo {
    pub address: String,
    pub pid: i32,
    pub class: String,
    pub title: String,
    pub position: (i32, i32),
    pub size: (i32, i32),
    pub workspace: String,
    pub monitor: Option<String>,
    pub is_floating: bool,
    pub is_lost: bool,
    pub last_seen: Instant,
}

#[derive(Debug, Clone)]
pub struct WindowHistory {
    pub address: String,
    pub good_positions: Vec<((i32, i32), Instant)>, // Position history with timestamps
    pub last_monitor: Option<String>,
    pub recovery_count: u32,
}

#[derive(Debug)]
pub struct RecoverySession {
    pub lost_windows: Vec<WindowInfo>,
    pub target_monitor: MonitorInfo,
    pub strategy: RescueStrategy,
    pub positions: Vec<(i32, i32)>,
    pub created_at: Instant,
}

// ============================================================================
// WINDOW POSITIONING ALGORITHMS
// ============================================================================

pub struct WindowPositioner;

impl WindowPositioner {
    /// Calculate positions using the specified rescue strategy
    pub fn calculate_positions(
        strategy: &RescueStrategy,
        windows: &[WindowInfo],
        monitor: &MonitorInfo,
        margin: i32,
    ) -> Vec<(i32, i32)> {
        match strategy {
            RescueStrategy::Distribute => Self::distribute_positions(windows, monitor, margin),
            RescueStrategy::Grid => Self::grid_positions(windows, monitor, margin),
            RescueStrategy::Cascade => Self::cascade_positions(windows, monitor, margin),
            RescueStrategy::Center => Self::center_positions(windows, monitor),
            RescueStrategy::Restore => Self::restore_positions(windows, monitor, margin),
            RescueStrategy::Smart => Self::smart_positions(windows, monitor, margin),
        }
    }

    fn distribute_positions(
        windows: &[WindowInfo],
        monitor: &MonitorInfo,
        margin: i32,
    ) -> Vec<(i32, i32)> {
        let count = windows.len() as i32;
        if count == 0 {
            return Vec::new();
        }

        let usable_width = monitor.width as i32 - (margin * 2);
        let usable_height = monitor.height as i32 - (margin * 2);

        let interval_x = usable_width / (count + 1);
        let interval_y = usable_height / (count + 1);

        (0..count)
            .map(|i| {
                let x = monitor.x + margin + interval_x * (i + 1);
                let y = monitor.y + margin + interval_y * (i + 1);
                (x, y)
            })
            .collect()
    }

    fn grid_positions(
        windows: &[WindowInfo],
        monitor: &MonitorInfo,
        margin: i32,
    ) -> Vec<(i32, i32)> {
        let count = windows.len();
        if count == 0 {
            return Vec::new();
        }

        // Calculate optimal grid dimensions
        let cols = ((count as f32).sqrt().ceil() as i32).max(1);
        let rows = ((count as f32 / cols as f32).ceil() as i32).max(1);

        let usable_width = monitor.width as i32 - (margin * 2);
        let usable_height = monitor.height as i32 - (margin * 2);

        let cell_width = usable_width / cols;
        let cell_height = usable_height / rows;

        windows
            .iter()
            .enumerate()
            .map(|(i, _)| {
                let row = i as i32 / cols;
                let col = i as i32 % cols;

                let x = monitor.x + margin + (cell_width * col) + (cell_width / 4);
                let y = monitor.y + margin + (cell_height * row) + (cell_height / 4);
                (x, y)
            })
            .collect()
    }

    fn cascade_positions(
        windows: &[WindowInfo],
        monitor: &MonitorInfo,
        margin: i32,
    ) -> Vec<(i32, i32)> {
        let cascade_offset = 40;
        let mut positions = Vec::new();

        for (i, _) in windows.iter().enumerate() {
            let x = monitor.x + margin + (cascade_offset * i as i32);
            let y = monitor.y + margin + (cascade_offset * i as i32);
            positions.push((x, y));
        }

        positions
    }

    fn center_positions(windows: &[WindowInfo], monitor: &MonitorInfo) -> Vec<(i32, i32)> {
        let center_x = monitor.x + monitor.width as i32 / 2;
        let center_y = monitor.y + monitor.height as i32 / 2;

        windows.iter().map(|_| (center_x, center_y)).collect()
    }

    fn restore_positions(
        windows: &[WindowInfo],
        monitor: &MonitorInfo,
        margin: i32,
    ) -> Vec<(i32, i32)> {
        // For now, fall back to smart positioning
        // In a full implementation, this would use window history
        Self::smart_positions(windows, monitor, margin)
    }

    fn smart_positions(
        windows: &[WindowInfo],
        monitor: &MonitorInfo,
        margin: i32,
    ) -> Vec<(i32, i32)> {
        let mut positions = Vec::new();
        let mut used_positions: Vec<(i32, i32, i32, i32)> = Vec::new(); // x, y, width, height

        for window in windows {
            let (width, height) = window.size;
            let mut best_position = None;
            let mut min_overlap_area = i32::MAX;

            // Try different positions and find the one with minimal overlap
            let step = 50;
            for y in (monitor.y + margin..=monitor.y + monitor.height as i32 - height - margin)
                .step_by(step)
            {
                for x in (monitor.x + margin..=monitor.x + monitor.width as i32 - width - margin)
                    .step_by(step)
                {
                    let overlap_area =
                        Self::calculate_overlap_area(x, y, width, height, &used_positions);

                    if overlap_area < min_overlap_area {
                        min_overlap_area = overlap_area;
                        best_position = Some((x, y));

                        if overlap_area == 0 {
                            break; // Found non-overlapping position
                        }
                    }
                }
                if min_overlap_area == 0 {
                    break;
                }
            }

            let position = best_position.unwrap_or((monitor.x + margin, monitor.y + margin));

            positions.push(position);
            used_positions.push((position.0, position.1, width, height));
        }

        positions
    }

    fn calculate_overlap_area(
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        used_positions: &[(i32, i32, i32, i32)],
    ) -> i32 {
        let mut total_overlap = 0;

        for &(used_x, used_y, used_width, used_height) in used_positions {
            let overlap_width = (x + width).min(used_x + used_width) - x.max(used_x);
            let overlap_height = (y + height).min(used_y + used_height) - y.max(used_y);

            if overlap_width > 0 && overlap_height > 0 {
                total_overlap += overlap_width * overlap_height;
            }
        }

        total_overlap
    }
}

// ============================================================================
// MAIN PLUGIN IMPLEMENTATION
// ============================================================================

pub struct LostWindowsPlugin {
    config: LostWindowsConfig,
    hyprland_client: Arc<Mutex<Option<Arc<HyprlandClient>>>>,
    window_history: HashMap<String, WindowHistory>,
    last_check: Option<Instant>,
    recovery_sessions: Vec<RecoverySession>,
    auto_recovery_enabled: bool,
}

impl LostWindowsPlugin {
    pub fn new() -> Self {
        Self {
            config: LostWindowsConfig::default(),
            hyprland_client: Arc::new(Mutex::new(None)),
            window_history: HashMap::new(),
            last_check: None,
            recovery_sessions: Vec::new(),
            auto_recovery_enabled: false,
        }
    }

    pub async fn set_hyprland_client(&self, client: Arc<HyprlandClient>) {
        let mut client_guard = self.hyprland_client.lock().await;
        *client_guard = Some(client);
    }

    /// Get current monitors
    async fn get_monitors(&self) -> Result<Vec<MonitorInfo>> {
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
                id: m.id,
                name: m.name.clone(),
                width: m.width,
                height: m.height,
                x: m.x,
                y: m.y,
                scale: m.scale,
                is_focused: m.focused,
                active_workspace_id: m.active_workspace.id,
                refresh_rate: m.refresh_rate,
            })
            .collect();

        Ok(monitor_infos)
    }

    /// Get current windows
    async fn get_windows(&self) -> Result<Vec<WindowInfo>> {
        let client_guard = self.hyprland_client.lock().await;
        let client = match client_guard.as_ref() {
            Some(client) => Arc::clone(client),
            None => return Err(anyhow::anyhow!("Hyprland client not available")),
        };
        drop(client_guard);

        let windows = client.get_windows().await?;
        let mut window_infos = Vec::new();

        for window in windows {
            let window_info = WindowInfo {
                address: window.address.to_string(),
                pid: window.pid,
                class: window.class.clone(),
                title: window.title.clone(),
                position: (window.at.0.into(), window.at.1.into()),
                size: (window.size.0.into(), window.size.1.into()),
                workspace: window.workspace.name.clone(),
                monitor: Some(window.monitor.to_string()),
                is_floating: window.floating,
                is_lost: false, // Will be determined later
                last_seen: Instant::now(),
            };
            window_infos.push(window_info);
        }

        Ok(window_infos)
    }

    /// Check if a window is contained within any monitor
    fn is_window_contained(window: &WindowInfo, monitors: &[MonitorInfo]) -> bool {
        let (win_x, win_y) = window.position;
        let (win_width, win_height) = window.size;

        for monitor in monitors {
            // Check if window overlaps with monitor bounds
            let overlap_x =
                (win_x + win_width).min(monitor.x + monitor.width as i32) - win_x.max(monitor.x);
            let overlap_y =
                (win_y + win_height).min(monitor.y + monitor.height as i32) - win_y.max(monitor.y);

            // Consider window contained if there's significant overlap
            if overlap_x > win_width / 4 && overlap_y > win_height / 4 {
                return true;
            }
        }

        false
    }

    /// Find lost windows
    async fn find_lost_windows(&self) -> Result<Vec<WindowInfo>> {
        let monitors = self.get_monitors().await?;
        let mut windows = self.get_windows().await?;

        let mut lost_windows = Vec::new();

        for window in &mut windows {
            // Skip if not floating
            if !window.is_floating {
                continue;
            }

            // Skip if excluded class
            if self.config.exclude_classes.contains(&window.class) {
                continue;
            }

            // Skip if too small
            let (min_width, min_height) = self.config.min_window_size;
            if window.size.0 < min_width || window.size.1 < min_height {
                continue;
            }

            // Check if window is contained within any monitor
            if !Self::is_window_contained(window, &monitors) {
                window.is_lost = true;
                lost_windows.push(window.clone());

                if self.config.debug_logging {
                    debug!(
                        "🔍 Found lost window: {} ({}) at ({}, {})",
                        window.title, window.class, window.position.0, window.position.1
                    );
                }
            }
        }

        Ok(lost_windows)
    }

    /// Get the focused monitor
    async fn get_focused_monitor(&self) -> Result<MonitorInfo> {
        let monitors = self.get_monitors().await?;
        monitors
            .into_iter()
            .find(|m| m.is_focused)
            .ok_or_else(|| anyhow::anyhow!("No focused monitor found"))
    }

    /// Create recovery session
    async fn create_recovery_session(&mut self, lost_windows: Vec<WindowInfo>) -> Result<()> {
        if lost_windows.is_empty() {
            return Ok(());
        }

        let target_monitor = if self.config.current_monitor_only {
            self.get_focused_monitor().await?
        } else {
            // Use the monitor with most space or focused monitor
            self.get_focused_monitor().await?
        };

        let positions = WindowPositioner::calculate_positions(
            &self.config.rescue_strategy,
            &lost_windows,
            &target_monitor,
            self.config.margin,
        );

        let session = RecoverySession {
            lost_windows,
            target_monitor,
            strategy: self.config.rescue_strategy.clone(),
            positions,
            created_at: Instant::now(),
        };

        self.recovery_sessions.push(session);
        Ok(())
    }

    /// Execute window recovery
    async fn execute_recovery(&mut self) -> Result<String> {
        if self.recovery_sessions.is_empty() {
            return Ok("No recovery sessions available".to_string());
        }

        let session = self.recovery_sessions.remove(0);
        let recovered_count = session.lost_windows.len();

        if recovered_count == 0 {
            return Ok("No lost windows to recover".to_string());
        }

        let client_guard = self.hyprland_client.lock().await;
        let client = match client_guard.as_ref() {
            Some(client) => Arc::clone(client),
            None => return Err(anyhow::anyhow!("Hyprland client not available")),
        };
        drop(client_guard);

        info!(
            "🔧 Recovering {} lost windows using {:?} strategy",
            recovered_count, session.strategy
        );

        for (window, &(new_x, new_y)) in session.lost_windows.iter().zip(session.positions.iter()) {
            // Move window to current workspace
            if let Err(e) = client
                .move_window_to_workspace(&window.address, &session.target_monitor.name)
                .await
            {
                warn!("Failed to move window to workspace: {}", e);
            }

            // Set window position
            if let Err(e) = client.move_window(&window.address, new_x, new_y).await {
                warn!("Failed to move window to position: {}", e);
            } else {
                debug!(
                    "✅ Recovered window: {} to ({}, {})",
                    window.title, new_x, new_y
                );
            }

            // Add smooth animation if enabled
            if self.config.enable_animations {
                tokio::time::sleep(Duration::from_millis(
                    self.config.animation_duration / recovered_count as u64,
                ))
                .await;
            }
        }

        Ok(format!("Recovered {recovered_count} lost windows"))
    }

    /// Automatic recovery check
    async fn check_auto_recovery(&mut self) -> Result<()> {
        if !self.auto_recovery_enabled {
            return Ok(());
        }

        let now = Instant::now();
        #[allow(clippy::unnecessary_map_or)]
        let should_check = self.last_check.map_or(true, |last| {
            now.duration_since(last).as_secs() >= self.config.check_interval
        });

        if !should_check {
            return Ok(());
        }

        self.last_check = Some(now);

        let lost_windows = self.find_lost_windows().await?;
        if !lost_windows.is_empty() {
            info!("🔍 Auto-recovery found {} lost windows", lost_windows.len());
            self.create_recovery_session(lost_windows).await?;

            if !self.config.require_confirmation {
                self.execute_recovery().await?;
            }
        }

        Ok(())
    }

    /// List lost windows
    async fn list_lost_windows(&self) -> Result<String> {
        let lost_windows = self.find_lost_windows().await?;

        if lost_windows.is_empty() {
            return Ok("✅ No lost windows found".to_string());
        }

        let mut output = format!("🔍 Found {} lost windows:\n\n", lost_windows.len());

        for (i, window) in lost_windows.iter().enumerate() {
            output.push_str(&format!(
                "[{}] {} ({})\n    Class: {} | Position: ({}, {}) | Size: {}x{}\n",
                i + 1,
                window.title,
                window.address,
                window.class,
                window.position.0,
                window.position.1,
                window.size.0,
                window.size.1
            ));
        }

        output.push_str("\nUse 'lost_windows recover' to rescue these windows\n");

        Ok(output)
    }

    /// Get plugin status
    async fn get_status(&self) -> Result<String> {
        let monitors = self.get_monitors().await?;
        let windows = self.get_windows().await?;
        let lost_windows = self.find_lost_windows().await?;

        let floating_count = windows.iter().filter(|w| w.is_floating).count();

        let mut status = format!(
            "Lost Windows Plugin Status:\n  {} monitors, {} windows ({} floating, {} lost)\n",
            monitors.len(),
            windows.len(),
            floating_count,
            lost_windows.len()
        );

        status.push_str(&format!(
            "Configuration:\n  - Auto recovery: {}\n  - Strategy: {:?}\n  - Check interval: {}s\n  - Max windows: {}\n",
            self.auto_recovery_enabled,
            self.config.rescue_strategy,
            self.config.check_interval,
            self.config.max_windows
        ));

        if !self.config.exclude_classes.is_empty() {
            status.push_str(&format!(
                "  - Excluded classes: {}\n",
                self.config.exclude_classes.join(", ")
            ));
        }

        if let Some(last_check) = self.last_check {
            let elapsed = last_check.elapsed();
            status.push_str(&format!(
                "  - Last check: {:.1}s ago\n",
                elapsed.as_secs_f64()
            ));
        }

        if !self.recovery_sessions.is_empty() {
            status.push_str(&format!(
                "  - Pending recovery sessions: {}\n",
                self.recovery_sessions.len()
            ));
        }

        Ok(status)
    }
}

impl Default for LostWindowsPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Plugin for LostWindowsPlugin {
    fn name(&self) -> &str {
        "lost_windows"
    }

    async fn init(&mut self, config: &toml::Value) -> Result<()> {
        info!("🔍 Initializing lost_windows plugin");

        if let Some(plugin_config) = config.get("lost_windows") {
            match plugin_config.clone().try_into() {
                Ok(config) => self.config = config,
                Err(e) => return Err(anyhow::anyhow!("Invalid lost_windows configuration: {}", e)),
            }
        }

        debug!("Lost windows config: {:?}", self.config);

        // Enable auto-recovery if configured
        self.auto_recovery_enabled = self.config.auto_recovery;

        info!(
            "✅ Lost windows plugin initialized (strategy: {:?}, auto_recovery: {})",
            self.config.rescue_strategy, self.auto_recovery_enabled
        );

        Ok(())
    }

    async fn handle_event(&mut self, event: &HyprlandEvent) -> Result<()> {
        // Check for auto-recovery on various events
        match event {
            HyprlandEvent::WindowOpened { window: _ }
            | HyprlandEvent::WindowClosed { window: _ }
            | HyprlandEvent::WindowMoved { window: _ }
            | HyprlandEvent::MonitorChanged { monitor: _ } => {
                self.check_auto_recovery().await?;
            }
            _ => {}
        }

        Ok(())
    }

    async fn handle_command(&mut self, command: &str, args: &[&str]) -> Result<String> {
        match command {
            "list" => self.list_lost_windows().await,

            "recover" | "rescue" => {
                let lost_windows = self.find_lost_windows().await?;
                if lost_windows.is_empty() {
                    Ok("✅ No lost windows found".to_string())
                } else {
                    self.create_recovery_session(lost_windows).await?;
                    self.execute_recovery().await
                }
            }

            "status" => self.get_status().await,

            "enable" => {
                self.auto_recovery_enabled = true;
                Ok("✅ Auto-recovery enabled".to_string())
            }

            "disable" => {
                self.auto_recovery_enabled = false;
                Ok("✅ Auto-recovery disabled".to_string())
            }

            "strategy" => {
                if let Some(strategy_str) = args.first() {
                    match strategy_str.to_lowercase().as_str() {
                        "distribute" => self.config.rescue_strategy = RescueStrategy::Distribute,
                        "grid" => self.config.rescue_strategy = RescueStrategy::Grid,
                        "cascade" => self.config.rescue_strategy = RescueStrategy::Cascade,
                        "center" => self.config.rescue_strategy = RescueStrategy::Center,
                        "restore" => self.config.rescue_strategy = RescueStrategy::Restore,
                        "smart" => self.config.rescue_strategy = RescueStrategy::Smart,
                        _ => return Err(anyhow::anyhow!("Unknown strategy: {}", strategy_str)),
                    }
                    Ok(format!("✅ Rescue strategy set to: {:?}", self.config.rescue_strategy))
                } else {
                    Ok(format!("Current strategy: {:?}", self.config.rescue_strategy))
                }
            }

            "check" => {
                let lost_windows = self.find_lost_windows().await?;
                Ok(format!("🔍 Found {} lost windows", lost_windows.len()))
            }

            _ => Err(anyhow::anyhow!(
                "Unknown lost_windows command: {}. Available: list, recover, status, enable, disable, strategy, check",
                command
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let config = LostWindowsConfig::default();
        assert!(matches!(config.rescue_strategy, RescueStrategy::Smart));
        assert!(config.auto_recovery);
        assert_eq!(config.check_interval, 30);
        assert_eq!(config.margin, 50);
        assert_eq!(config.max_windows, 10);
    }

    #[test]
    fn test_rescue_strategies() {
        let strategies = [
            RescueStrategy::Distribute,
            RescueStrategy::Grid,
            RescueStrategy::Cascade,
            RescueStrategy::Center,
            RescueStrategy::Restore,
            RescueStrategy::Smart,
        ];

        for strategy in strategies {
            // Test serialization/deserialization
            let json = serde_json::to_string(&strategy).unwrap();
            let _deserialized: RescueStrategy = serde_json::from_str(&json).unwrap();
        }
    }

    #[test]
    fn test_window_positioning_grid() {
        let monitor = MonitorInfo {
            id: 0,
            name: "DP-1".to_string(),
            width: 1920,
            height: 1080,
            x: 0,
            y: 0,
            scale: 1.0,
            is_focused: true,
            active_workspace_id: 1,
            refresh_rate: 60.0,
        };

        let windows = vec![
            WindowInfo {
                address: "0x1".to_string(),
                pid: 1,
                class: "test".to_string(),
                title: "Test".to_string(),
                position: (0, 0),
                size: (100, 100),
                workspace: "1".to_string(),
                monitor: None,
                is_floating: true,
                is_lost: true,
                last_seen: Instant::now(),
            };
            4
        ];

        let positions =
            WindowPositioner::calculate_positions(&RescueStrategy::Grid, &windows, &monitor, 50);

        assert_eq!(positions.len(), 4);
        // Positions should be distributed in a 2x2 grid
        assert!(positions.iter().all(|(x, y)| *x >= 50 && *y >= 50));
    }

    #[test]
    fn test_overlap_calculation() {
        let overlap = WindowPositioner::calculate_overlap_area(
            100,
            100,
            200,
            200,                     // New window
            &[(150, 150, 200, 200)], // Existing window
        );

        // Should have 150x150 = 22500 overlap
        assert_eq!(overlap, 22500);
    }

    #[test]
    fn test_window_containment() {
        let window = WindowInfo {
            address: "0x1".to_string(),
            pid: 1,
            class: "test".to_string(),
            title: "Test".to_string(),
            position: (-100, -100), // Outside monitor
            size: (200, 200),
            workspace: "1".to_string(),
            monitor: None,
            is_floating: true,
            is_lost: false,
            last_seen: Instant::now(),
        };

        let monitor = MonitorInfo {
            id: 0,
            name: "DP-1".to_string(),
            width: 1920,
            height: 1080,
            x: 0,
            y: 0,
            scale: 1.0,
            is_focused: true,
            active_workspace_id: 1,
            refresh_rate: 60.0,
        };

        let monitors = vec![monitor];

        // Window partially outside should still be considered contained if enough overlap
        assert!(LostWindowsPlugin::is_window_contained(&window, &monitors));

        // Completely outside window
        let lost_window = WindowInfo {
            position: (-300, -300),
            ..window
        };

        assert!(!LostWindowsPlugin::is_window_contained(
            &lost_window,
            &monitors
        ));
    }
}
