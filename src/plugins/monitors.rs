use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Command;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

use crate::ipc::{HyprlandClient, HyprlandEvent};
use crate::plugins::Plugin;

use hyprland::data::Monitors;
use hyprland::shared::{HyprData, HyprDataVec};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct MonitorSettings {
    /// Monitor resolution (e.g., "1920x1080")
    pub resolution: Option<String>,
    /// Refresh rate in Hz (e.g., 60, 144, 240)
    pub rate: Option<u32>,
    /// Scale factor (e.g., 1.0, 1.5, 2.0)
    pub scale: Option<f64>,
    /// Transform/rotation (0-7: 0=normal, 1=90°, 2=180°, 3=270°, 4-7=flipped versions)
    pub transform: Option<u32>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PlacementRule {
    /// Target monitor to place relative to
    pub target: String,
    /// Placement direction: left_of, right_of, top_of, bottom_of
    pub direction: PlacementDirection,
    /// Alignment modifier: start, center/middle, end
    pub alignment: Option<PlacementAlignment>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum PlacementDirection {
    Left,
    Right,
    Top,
    Bottom,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum PlacementAlignment {
    Start,
    Center,
    Middle, // Alias for center
    End,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MonitorsConfig {
    /// Whether to apply layout on startup (default: true)
    #[serde(default = "default_true")]
    pub startup_relayout: bool,

    /// Delay in milliseconds before applying layout after monitor change (default: 1000)
    #[serde(default = "default_monitor_delay")]
    pub new_monitor_delay: u64,

    /// Command to run when any monitor is plugged
    pub hotplug_command: Option<String>,

    /// Commands to run when specific monitors are plugged (monitor_name -> command)
    #[serde(default)]
    pub hotplug_commands: HashMap<String, String>,

    /// Monitor placement rules (monitor_name -> placement_rule)
    #[serde(default)]
    pub placement: HashMap<String, PlacementRuleConfig>,

    /// Monitor-specific settings (monitor_name -> settings)
    #[serde(default)]
    pub settings: HashMap<String, MonitorSettings>,

    /// Enable debug logging (default: false)
    #[serde(default)]
    pub debug_logging: bool,

    /// Case insensitive monitor name matching (default: true)
    #[serde(default = "default_true")]
    pub case_insensitive: bool,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum PlacementRuleConfig {
    /// Simple string format: "target" (assumes left_of)
    Simple(String),
    /// Complex placement rule with direction and alignment
    Complex {
        /// Direction relative to target
        #[serde(flatten)]
        direction_target: Box<DirectionTarget>,
        /// Optional alignment
        alignment: Option<PlacementAlignment>,
    },
    /// Pyprland-compatible format (leftOf, rightOf, etc.)
    PyprlandStyle(HashMap<String, Vec<String>>),
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectionTarget {
    pub left_of: Option<Vec<String>>,
    pub right_of: Option<Vec<String>>,
    pub top_of: Option<Vec<String>>,
    pub bottom_of: Option<Vec<String>>,
    pub left_center_of: Option<Vec<String>>,
    pub right_center_of: Option<Vec<String>>,
    pub top_center_of: Option<Vec<String>>,
    pub bottom_center_of: Option<Vec<String>>,
    pub left_end_of: Option<Vec<String>>,
    pub right_end_of: Option<Vec<String>>,
    pub top_end_of: Option<Vec<String>>,
    pub bottom_end_of: Option<Vec<String>>,
}

fn default_true() -> bool {
    true
}

fn default_monitor_delay() -> u64 {
    1000
}

impl Default for MonitorsConfig {
    fn default() -> Self {
        Self {
            startup_relayout: true,
            new_monitor_delay: 1000,
            hotplug_command: None,
            hotplug_commands: HashMap::new(),
            placement: HashMap::new(),
            settings: HashMap::new(),
            debug_logging: false,
            case_insensitive: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MonitorInfo {
    pub id: i128,
    pub name: String,
    pub description: String,
    pub make: String,
    pub model: String,
    pub serial: String,
    pub active_workspace_id: i32,
    pub active_workspace_name: String,
    pub focused: bool,
    pub width: u16,
    pub height: u16,
    pub refresh_rate: f32,
    pub x: i32,
    pub y: i32,
    pub scale: f64,
    pub transform: u32,
    pub disabled: bool,
}

#[derive(Debug)]
pub struct MonitorLayout {
    pub monitors: HashMap<String, MonitorInfo>,
    pub placement_rules: Vec<ResolvedPlacementRule>,
    pub last_update: Instant,
}

#[derive(Debug, Clone)]
pub struct ResolvedPlacementRule {
    pub source_monitor: String,
    pub target_monitor: String,
    pub direction: PlacementDirection,
    pub alignment: Option<PlacementAlignment>,
}

pub struct MonitorsPlugin {
    config: MonitorsConfig,
    current_layout: Option<MonitorLayout>,
    hyprland_client: Arc<Mutex<Option<Arc<HyprlandClient>>>>,
    last_layout_time: Option<Instant>,
    pending_layout_apply: bool,
}

impl MonitorsPlugin {
    pub fn new() -> Self {
        Self {
            config: MonitorsConfig::default(),
            current_layout: None,
            hyprland_client: Arc::new(Mutex::new(None)),
            last_layout_time: None,
            pending_layout_apply: false,
        }
    }

    /// Update monitor information from Hyprland
    async fn update_monitors(&mut self) -> Result<()> {
        let monitors = tokio::task::spawn_blocking(Monitors::get).await??;
        let monitor_vec = monitors.to_vec();

        let mut monitor_map = HashMap::new();

        for monitor in monitor_vec {
            let monitor_info = MonitorInfo {
                id: monitor.id,
                name: monitor.name.clone(),
                description: monitor.description.clone(),
                make: String::new(),   // Not available in hyprland crate
                model: String::new(),  // Not available in hyprland crate
                serial: String::new(), // Not available in hyprland crate
                active_workspace_id: monitor.active_workspace.id,
                active_workspace_name: monitor.active_workspace.name.clone(),
                focused: monitor.focused,
                width: monitor.width,
                height: monitor.height,
                refresh_rate: monitor.refresh_rate,
                x: monitor.x,
                y: monitor.y,
                scale: monitor.scale as f64,
                transform: monitor.transform as u32,
                disabled: monitor.disabled,
            };

            monitor_map.insert(monitor.name, monitor_info);
        }

        let placement_rules = self.resolve_placement_rules(&monitor_map)?;

        self.current_layout = Some(MonitorLayout {
            monitors: monitor_map,
            placement_rules,
            last_update: Instant::now(),
        });

        if self.config.debug_logging {
            debug!(
                "🖥️  Updated monitor layout with {} monitors",
                self.current_layout.as_ref().unwrap().monitors.len()
            );
        }

        Ok(())
    }

    /// Resolve placement rules from configuration
    fn resolve_placement_rules(
        &self,
        monitors: &HashMap<String, MonitorInfo>,
    ) -> Result<Vec<ResolvedPlacementRule>> {
        let mut rules = Vec::new();

        for (monitor_name, rule_config) in &self.config.placement {
            let resolved_rules = self.parse_placement_rule(monitor_name, rule_config, monitors)?;
            rules.extend(resolved_rules);
        }

        Ok(rules)
    }

    /// Parse a placement rule configuration into resolved rules
    fn parse_placement_rule(
        &self,
        source_monitor: &str,
        rule_config: &PlacementRuleConfig,
        monitors: &HashMap<String, MonitorInfo>,
    ) -> Result<Vec<ResolvedPlacementRule>> {
        let mut rules = Vec::new();

        match rule_config {
            PlacementRuleConfig::Simple(target) => {
                if self.monitor_exists(target, monitors) {
                    rules.push(ResolvedPlacementRule {
                        source_monitor: source_monitor.to_string(),
                        target_monitor: target.clone(),
                        direction: PlacementDirection::Left,
                        alignment: None,
                    });
                }
            }

            PlacementRuleConfig::PyprlandStyle(style_map) => {
                for (direction_key, targets) in style_map {
                    let (direction, alignment) = self.parse_direction_key(direction_key);

                    for target in targets {
                        if self.monitor_exists(target, monitors) {
                            rules.push(ResolvedPlacementRule {
                                source_monitor: source_monitor.to_string(),
                                target_monitor: target.clone(),
                                direction: direction.clone(),
                                alignment: alignment.clone(),
                            });
                        }
                    }
                }
            }

            PlacementRuleConfig::Complex {
                direction_target,
                alignment,
            } => {
                // Handle complex direction-target combinations
                let direction_mappings = [
                    (&direction_target.left_of, PlacementDirection::Left),
                    (&direction_target.right_of, PlacementDirection::Right),
                    (&direction_target.top_of, PlacementDirection::Top),
                    (&direction_target.bottom_of, PlacementDirection::Bottom),
                    (&direction_target.left_center_of, PlacementDirection::Left),
                    (&direction_target.right_center_of, PlacementDirection::Right),
                    (&direction_target.top_center_of, PlacementDirection::Top),
                    (
                        &direction_target.bottom_center_of,
                        PlacementDirection::Bottom,
                    ),
                    (&direction_target.left_end_of, PlacementDirection::Left),
                    (&direction_target.right_end_of, PlacementDirection::Right),
                    (&direction_target.top_end_of, PlacementDirection::Top),
                    (&direction_target.bottom_end_of, PlacementDirection::Bottom),
                ];

                for (targets_option, direction) in direction_mappings {
                    if let Some(targets) = targets_option {
                        let rule_alignment =
                            self.extract_alignment_from_direction_key(&format!("{direction:?}"));

                        for target in targets {
                            if self.monitor_exists(target, monitors) {
                                rules.push(ResolvedPlacementRule {
                                    source_monitor: source_monitor.to_string(),
                                    target_monitor: target.clone(),
                                    direction: direction.clone(),
                                    alignment: rule_alignment.clone().or_else(|| alignment.clone()),
                                });
                            }
                        }
                    }
                }
            }
        }

        Ok(rules)
    }

    /// Parse direction key (e.g., "leftOf", "topCenterOf") into direction and alignment
    fn parse_direction_key(&self, key: &str) -> (PlacementDirection, Option<PlacementAlignment>) {
        let key_lower = key.to_lowercase();

        let alignment = if key_lower.contains("center") || key_lower.contains("middle") {
            Some(PlacementAlignment::Center)
        } else if key_lower.contains("end") {
            Some(PlacementAlignment::End)
        } else {
            None
        };

        let direction = if key_lower.contains("left") {
            PlacementDirection::Left
        } else if key_lower.contains("right") {
            PlacementDirection::Right
        } else if key_lower.contains("top") {
            PlacementDirection::Top
        } else if key_lower.contains("bottom") {
            PlacementDirection::Bottom
        } else {
            PlacementDirection::Left // Default
        };

        (direction, alignment)
    }

    /// Extract alignment information from direction key
    fn extract_alignment_from_direction_key(&self, key: &str) -> Option<PlacementAlignment> {
        let key_lower = key.to_lowercase();

        if key_lower.contains("center") {
            Some(PlacementAlignment::Center)
        } else if key_lower.contains("middle") {
            Some(PlacementAlignment::Middle)
        } else if key_lower.contains("end") {
            Some(PlacementAlignment::End)
        } else {
            None
        }
    }

    /// Check if monitor exists (with case-insensitive matching if enabled)
    fn monitor_exists(&self, name: &str, monitors: &HashMap<String, MonitorInfo>) -> bool {
        if self.config.case_insensitive {
            monitors
                .keys()
                .any(|k| k.to_lowercase() == name.to_lowercase())
                || monitors.values().any(|m| {
                    m.description.to_lowercase().contains(&name.to_lowercase())
                        || m.model.to_lowercase() == name.to_lowercase()
                        || m.make.to_lowercase() == name.to_lowercase()
                })
        } else {
            monitors.contains_key(name)
                || monitors
                    .values()
                    .any(|m| m.description.contains(name) || m.model == name || m.make == name)
        }
    }

    /// Find monitor by name or description
    fn find_monitor<'a>(
        &self,
        name: &str,
        monitors: &'a HashMap<String, MonitorInfo>,
    ) -> Option<&'a MonitorInfo> {
        // First try exact name match
        if let Some(monitor) = monitors.get(name) {
            return Some(monitor);
        }

        // Then try case-insensitive name match if enabled
        if self.config.case_insensitive {
            if let Some((_, monitor)) = monitors
                .iter()
                .find(|(k, _)| k.to_lowercase() == name.to_lowercase())
            {
                return Some(monitor);
            }
        }

        // Try description/model/make matching
        monitors.values().find(|m| {
            if self.config.case_insensitive {
                m.description.to_lowercase().contains(&name.to_lowercase())
                    || m.model.to_lowercase() == name.to_lowercase()
                    || m.make.to_lowercase() == name.to_lowercase()
            } else {
                m.description.contains(name) || m.model == name || m.make == name
            }
        })
    }

    /// Apply monitor layout using hyprctl
    async fn apply_monitor_layout(&mut self) -> Result<String> {
        if self.pending_layout_apply {
            return Ok("Layout application already in progress".to_string());
        }

        self.pending_layout_apply = true;

        // Add delay to prevent rapid re-applications
        sleep(Duration::from_millis(self.config.new_monitor_delay)).await;

        let result = self.apply_layout_internal().await;

        self.pending_layout_apply = false;
        self.last_layout_time = Some(Instant::now());

        result
    }

    /// Internal layout application logic
    async fn apply_layout_internal(&mut self) -> Result<String> {
        self.update_monitors().await?;

        let layout = match &self.current_layout {
            Some(layout) => layout,
            None => return Err(anyhow::anyhow!("No monitor layout available")),
        };

        if self.config.debug_logging {
            debug!(
                "🖥️  Applying monitor layout with {} monitors and {} rules",
                layout.monitors.len(),
                layout.placement_rules.len()
            );
        }

        let mut commands_applied = 0;
        let mut errors = Vec::new();

        // Apply monitor settings first
        for (monitor_name, monitor_info) in &layout.monitors {
            if let Some(settings) = self.config.settings.get(monitor_name) {
                match self.apply_monitor_settings(monitor_info, settings).await {
                    Ok(_) => {
                        commands_applied += 1;
                        if self.config.debug_logging {
                            debug!("✅ Applied settings for monitor {}", monitor_name);
                        }
                    }
                    Err(e) => {
                        let error_msg = format!("Failed to apply settings for {monitor_name}: {e}");
                        errors.push(error_msg.clone());
                        warn!("{}", error_msg);
                    }
                }
            }
        }

        // Apply placement rules
        for rule in &layout.placement_rules {
            match self.apply_placement_rule(rule, &layout.monitors).await {
                Ok(_) => {
                    commands_applied += 1;
                    if self.config.debug_logging {
                        debug!(
                            "✅ Applied placement rule: {} {:?} {}",
                            rule.source_monitor, rule.direction, rule.target_monitor
                        );
                    }
                }
                Err(e) => {
                    let error_msg = format!(
                        "Failed to apply rule {} {:?} {}: {}",
                        rule.source_monitor, rule.direction, rule.target_monitor, e
                    );
                    errors.push(error_msg.clone());
                    warn!("{}", error_msg);
                }
            }
        }

        let mut result = format!("Applied {commands_applied} monitor layout commands");

        if !errors.is_empty() {
            result.push_str(&format!(
                " with {} errors:\n{}",
                errors.len(),
                errors.join("\n")
            ));
        }

        info!("🖥️  Monitor layout applied: {}", result);

        Ok(result)
    }

    /// Apply settings for a specific monitor
    async fn apply_monitor_settings(
        &self,
        monitor: &MonitorInfo,
        settings: &MonitorSettings,
    ) -> Result<()> {
        let mut monitor_spec = monitor.name.clone();

        // Build monitor specification
        if let Some(resolution) = &settings.resolution {
            monitor_spec.push('@');
            monitor_spec.push_str(resolution);
        }

        if let Some(rate) = settings.rate {
            monitor_spec.push('@');
            monitor_spec.push_str(&rate.to_string());
        }

        if let Some(scale) = settings.scale {
            monitor_spec.push(',');
            monitor_spec.push_str(&scale.to_string());
        }

        if let Some(transform) = settings.transform {
            monitor_spec.push(',');
            monitor_spec.push_str("transform,");
            monitor_spec.push_str(&transform.to_string());
        }

        // Execute hyprctl command
        let output = tokio::task::spawn_blocking(move || {
            Command::new("hyprctl")
                .args(["keyword", "monitor", &monitor_spec])
                .output()
        })
        .await??;

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!(
                "hyprctl monitor command failed: {}",
                error_msg
            ));
        }

        Ok(())
    }

    /// Apply a placement rule
    async fn apply_placement_rule(
        &self,
        rule: &ResolvedPlacementRule,
        monitors: &HashMap<String, MonitorInfo>,
    ) -> Result<()> {
        let source_monitor = self
            .find_monitor(&rule.source_monitor, monitors)
            .ok_or_else(|| anyhow::anyhow!("Source monitor '{}' not found", rule.source_monitor))?;

        let target_monitor = self
            .find_monitor(&rule.target_monitor, monitors)
            .ok_or_else(|| anyhow::anyhow!("Target monitor '{}' not found", rule.target_monitor))?;

        // Calculate new position based on rule
        let (new_x, new_y) = self.calculate_position(source_monitor, target_monitor, rule)?;

        // Build monitor positioning command
        let position_spec = format!(
            "{}@{}x{},{}x{}",
            source_monitor.name, source_monitor.width, source_monitor.height, new_x, new_y
        );

        // Execute hyprctl command
        let output = tokio::task::spawn_blocking(move || {
            Command::new("hyprctl")
                .args(["keyword", "monitor", &position_spec])
                .output()
        })
        .await??;

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!(
                "hyprctl position command failed: {}",
                error_msg
            ));
        }

        Ok(())
    }

    /// Calculate new position for monitor based on placement rule
    fn calculate_position(
        &self,
        source: &MonitorInfo,
        target: &MonitorInfo,
        rule: &ResolvedPlacementRule,
    ) -> Result<(i32, i32)> {
        let (mut new_x, mut new_y) = match rule.direction {
            PlacementDirection::Left => (target.x - source.width as i32, target.y),
            PlacementDirection::Right => (target.x + target.width as i32, target.y),
            PlacementDirection::Top => (target.x, target.y - source.height as i32),
            PlacementDirection::Bottom => (target.x, target.y + target.height as i32),
        };

        // Apply alignment
        if let Some(alignment) = &rule.alignment {
            match rule.direction {
                PlacementDirection::Left | PlacementDirection::Right => {
                    // Vertical alignment for horizontal placement
                    match alignment {
                        PlacementAlignment::Center | PlacementAlignment::Middle => {
                            new_y = target.y + (target.height as i32 - source.height as i32) / 2;
                        }
                        PlacementAlignment::End => {
                            new_y = target.y + target.height as i32 - source.height as i32;
                        }
                        PlacementAlignment::Start => {
                            // Already at start (target.y)
                        }
                    }
                }
                PlacementDirection::Top | PlacementDirection::Bottom => {
                    // Horizontal alignment for vertical placement
                    match alignment {
                        PlacementAlignment::Center | PlacementAlignment::Middle => {
                            new_x = target.x + (target.width as i32 - source.width as i32) / 2;
                        }
                        PlacementAlignment::End => {
                            new_x = target.x + target.width as i32 - source.width as i32;
                        }
                        PlacementAlignment::Start => {
                            // Already at start (target.x)
                        }
                    }
                }
            }
        }

        Ok((new_x, new_y))
    }

    /// Execute hotplug command for monitor
    async fn execute_hotplug_command(&self, monitor_name: &str) -> Result<()> {
        // Check for monitor-specific command first
        if let Some(command) = self.config.hotplug_commands.get(monitor_name) {
            if self.config.debug_logging {
                debug!(
                    "🔌 Executing hotplug command for {}: {}",
                    monitor_name, command
                );
            }

            let output = tokio::task::spawn_blocking({
                let cmd = command.clone();
                move || Command::new("sh").args(["-c", &cmd]).output()
            })
            .await??;

            if !output.status.success() {
                let error_msg = String::from_utf8_lossy(&output.stderr);
                warn!("Hotplug command failed for {}: {}", monitor_name, error_msg);
            } else {
                info!("✅ Hotplug command executed for {}", monitor_name);
            }
        }

        // Execute general hotplug command if configured
        if let Some(command) = &self.config.hotplug_command {
            if self.config.debug_logging {
                debug!("🔌 Executing general hotplug command: {}", command);
            }

            let output = tokio::task::spawn_blocking({
                let cmd = command.clone();
                move || Command::new("sh").args(["-c", &cmd]).output()
            })
            .await??;

            if !output.status.success() {
                let error_msg = String::from_utf8_lossy(&output.stderr);
                warn!("General hotplug command failed: {}", error_msg);
            } else {
                info!("✅ General hotplug command executed");
            }
        }

        Ok(())
    }

    /// List all monitors with their configurations
    async fn list_monitors(&mut self) -> Result<String> {
        self.update_monitors().await?;

        let layout = match &self.current_layout {
            Some(layout) => layout,
            None => return Ok("No monitor layout available".to_string()),
        };

        let mut output = String::from("🖥️  Monitors:\n");

        for (name, monitor) in &layout.monitors {
            let status = if monitor.disabled {
                "❌ disabled"
            } else {
                "✅ active"
            };
            let focused = if monitor.focused { " 🎯" } else { "" };

            output.push_str(&format!(
                "  {} {}{}\n    Resolution: {}x{}@{:.1}Hz, Scale: {:.1}x, Transform: {}\n    Position: ({}, {}), Workspace: {}\n",
                name, status, focused,
                monitor.width, monitor.height, monitor.refresh_rate, monitor.scale, monitor.transform,
                monitor.x, monitor.y, monitor.active_workspace_name
            ));

            // Show description/model if different from name
            if &monitor.description != name && !monitor.description.is_empty() {
                output.push_str(&format!("    Description: {}\n", monitor.description));
            }

            // Show configured settings
            if let Some(settings) = self.config.settings.get(name) {
                output.push_str("    Configured settings:");
                if let Some(res) = &settings.resolution {
                    output.push_str(&format!(" res={res}"));
                }
                if let Some(rate) = settings.rate {
                    output.push_str(&format!(" rate={rate}Hz"));
                }
                if let Some(scale) = settings.scale {
                    output.push_str(&format!(" scale={scale:.1}x"));
                }
                if let Some(transform) = settings.transform {
                    output.push_str(&format!(" transform={transform}"));
                }
                output.push('\n');
            }
        }

        // Show placement rules
        if !layout.placement_rules.is_empty() {
            output.push_str("\n📐 Placement Rules:\n");
            for rule in &layout.placement_rules {
                let alignment_str = match &rule.alignment {
                    Some(align) => format!(" ({})", format!("{align:?}").to_lowercase()),
                    None => String::new(),
                };
                output.push_str(&format!(
                    "  {} → {:?} {}{}\n",
                    rule.source_monitor, rule.direction, rule.target_monitor, alignment_str
                ));
            }
        }

        Ok(output)
    }

    /// Test monitor layout without applying
    async fn test_layout(&mut self) -> Result<String> {
        self.update_monitors().await?;

        let layout = match &self.current_layout {
            Some(layout) => layout,
            None => return Ok("No monitor layout available to test".to_string()),
        };

        let mut output = String::from("🧪 Testing Monitor Layout:\n\n");
        let mut errors = Vec::new();

        // Test monitor settings
        for (monitor_name, monitor_info) in &layout.monitors {
            if let Some(settings) = self.config.settings.get(monitor_name) {
                output.push_str(&format!("📝 Settings for {monitor_name}:\n"));

                if let Some(resolution) = &settings.resolution {
                    output.push_str(&format!(
                        "  Resolution: {} (current: {}x{})\n",
                        resolution, monitor_info.width, monitor_info.height
                    ));
                }
                if let Some(rate) = settings.rate {
                    output.push_str(&format!(
                        "  Rate: {}Hz (current: {:.1}Hz)\n",
                        rate, monitor_info.refresh_rate
                    ));
                }
                if let Some(scale) = settings.scale {
                    output.push_str(&format!(
                        "  Scale: {:.1}x (current: {:.1}x)\n",
                        scale, monitor_info.scale
                    ));
                }
                if let Some(transform) = settings.transform {
                    output.push_str(&format!(
                        "  Transform: {} (current: {})\n",
                        transform, monitor_info.transform
                    ));
                }
            }
        }

        // Test placement rules
        output.push_str("\n📐 Placement Rules:\n");
        for rule in &layout.placement_rules {
            let source_monitor = self.find_monitor(&rule.source_monitor, &layout.monitors);
            let target_monitor = self.find_monitor(&rule.target_monitor, &layout.monitors);

            match (source_monitor, target_monitor) {
                (Some(source), Some(target)) => {
                    match self.calculate_position(source, target, rule) {
                        Ok((new_x, new_y)) => {
                            output.push_str(&format!(
                                "  ✅ {} → {:?} {}: would move to ({}, {}) (current: ({}, {}))\n",
                                rule.source_monitor,
                                rule.direction,
                                rule.target_monitor,
                                new_x,
                                new_y,
                                source.x,
                                source.y
                            ));
                        }
                        Err(e) => {
                            let error_msg = format!(
                                "Failed to calculate position for {}: {}",
                                rule.source_monitor, e
                            );
                            errors.push(error_msg.clone());
                            output.push_str(&format!("  ❌ {error_msg}\n"));
                        }
                    }
                }
                (None, _) => {
                    let error_msg = format!("Source monitor '{}' not found", rule.source_monitor);
                    errors.push(error_msg.clone());
                    output.push_str(&format!("  ❌ {error_msg}\n"));
                }
                (_, None) => {
                    let error_msg = format!("Target monitor '{}' not found", rule.target_monitor);
                    errors.push(error_msg.clone());
                    output.push_str(&format!("  ❌ {error_msg}\n"));
                }
            }
        }

        if errors.is_empty() {
            output.push_str("\n✅ Layout test passed - no errors detected\n");
        } else {
            output.push_str(&format!("\n❌ Layout test found {} errors\n", errors.len()));
        }

        Ok(output)
    }

    /// Get status of monitors plugin
    async fn get_status(&mut self) -> Result<String> {
        self.update_monitors().await?;

        let (monitor_count, rule_count, last_update) = match &self.current_layout {
            Some(layout) => (
                layout.monitors.len(),
                layout.placement_rules.len(),
                Some(layout.last_update),
            ),
            None => (0, 0, None),
        };

        let mut status = format!(
            "Monitors Plugin Status:\n  {monitor_count} monitors detected, {rule_count} placement rules configured\n"
        );

        if let Some(last_time) = self.last_layout_time {
            let elapsed = last_time.elapsed();
            status.push_str(&format!(
                "  Last layout applied: {:.1}s ago\n",
                elapsed.as_secs_f64()
            ));
        } else {
            status.push_str("  No layout applied yet\n");
        }

        if let Some(update_time) = last_update {
            let elapsed = update_time.elapsed();
            status.push_str(&format!(
                "  Monitor data updated: {:.1}s ago\n",
                elapsed.as_secs_f64()
            ));
        }

        status.push_str(&format!(
            "\nConfiguration:\n  - Startup relayout: {}\n  - Monitor delay: {}ms\n  - Case insensitive: {}\n  - Debug logging: {}\n",
            self.config.startup_relayout,
            self.config.new_monitor_delay,
            self.config.case_insensitive,
            self.config.debug_logging
        ));

        let hotplug_commands = self.config.hotplug_commands.len()
            + if self.config.hotplug_command.is_some() {
                1
            } else {
                0
            };
        if hotplug_commands > 0 {
            status.push_str(&format!(
                "  - Hotplug commands: {hotplug_commands} configured\n"
            ));
        }

        let settings_count = self.config.settings.len();
        if settings_count > 0 {
            status.push_str(&format!(
                "  - Monitor settings: {settings_count} configured\n"
            ));
        }

        Ok(status)
    }
}

impl Default for MonitorsPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Plugin for MonitorsPlugin {
    fn name(&self) -> &str {
        "monitors"
    }

    async fn init(&mut self, config: &toml::Value) -> Result<()> {
        info!("🖥️  Initializing monitors plugin");

        if let Some(plugin_config) = config.get("monitors") {
            match plugin_config.clone().try_into() {
                Ok(config) => self.config = config,
                Err(e) => return Err(anyhow::anyhow!("Invalid monitors configuration: {}", e)),
            }
        }

        debug!("Monitors config: {:?}", self.config);

        // Initialize monitor state
        self.update_monitors().await?;

        // Apply initial layout if configured
        if self.config.startup_relayout {
            if let Err(e) = self.apply_monitor_layout().await {
                warn!("Failed to apply initial monitor layout: {}", e);
            }
        }

        info!(
            "✅ Monitors plugin initialized with {} monitors",
            self.current_layout
                .as_ref()
                .map(|l| l.monitors.len())
                .unwrap_or(0)
        );

        Ok(())
    }

    async fn handle_event(&mut self, event: &HyprlandEvent) -> Result<()> {
        match event {
            HyprlandEvent::Other(event_data) => {
                // Handle monitor connection events
                if event_data.starts_with("monitoradded>>") {
                    let monitor_name = event_data
                        .strip_prefix("monitoradded>>")
                        .unwrap_or("")
                        .trim();

                    if self.config.debug_logging {
                        debug!("🔌 Monitor connected: {}", monitor_name);
                    }

                    // Execute hotplug commands
                    if let Err(e) = self.execute_hotplug_command(monitor_name).await {
                        warn!(
                            "Failed to execute hotplug command for {}: {}",
                            monitor_name, e
                        );
                    }

                    // Apply layout after delay
                    if let Err(e) = self.apply_monitor_layout().await {
                        warn!("Failed to apply layout after monitor connection: {}", e);
                    }
                } else if event_data.starts_with("monitorremoved>>") {
                    let monitor_name = event_data
                        .strip_prefix("monitorremoved>>")
                        .unwrap_or("")
                        .trim();

                    if self.config.debug_logging {
                        debug!("🔌 Monitor disconnected: {}", monitor_name);
                    }

                    // Update monitor state
                    self.update_monitors().await?;
                }
            }

            _ => {
                // Update monitor state on workspace or window changes
                // This helps keep monitor information current
                if matches!(
                    event,
                    HyprlandEvent::WorkspaceChanged { .. } | HyprlandEvent::WindowMoved { .. }
                ) {
                    self.update_monitors().await?;
                }
            }
        }

        Ok(())
    }

    async fn handle_command(&mut self, command: &str, args: &[&str]) -> Result<String> {
        debug!("🖥️  Monitors command: {} {:?}", command, args);

        match command {
            "" | "relayout" => {
                // Apply monitor layout
                self.apply_monitor_layout().await
            }

            "list" => self.list_monitors().await,
            "status" => self.get_status().await,
            "test" => self.test_layout().await,

            "reload" => {
                // Force reload of monitor configuration
                self.update_monitors().await?;
                Ok("Monitor configuration reloaded".to_string())
            }

            _ => Ok(format!(
                "Unknown monitors command: {command}. Available: relayout, list, status, test, reload"  
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_plugin() -> MonitorsPlugin {
        MonitorsPlugin::new()
    }

    fn create_test_config() -> MonitorsConfig {
        let mut config = MonitorsConfig::default();
        config.new_monitor_delay = 100;
        config.debug_logging = true;
        config.case_insensitive = true;
        config.hotplug_command = Some("echo 'monitor connected'".to_string());
        config
    }

    fn create_test_monitor(name: &str, x: i32, y: i32, width: u16, height: u16) -> MonitorInfo {
        MonitorInfo {
            id: 0,
            name: name.to_string(),
            description: format!("{name} Description"),
            make: "TestMake".to_string(),
            model: "TestModel".to_string(),
            serial: "12345".to_string(),
            active_workspace_id: 1,
            active_workspace_name: "1".to_string(),
            focused: false,
            width,
            height,
            refresh_rate: 60.0,
            x,
            y,
            scale: 1.0,
            transform: 0,
            disabled: false,
        }
    }

    #[test]
    fn test_plugin_creation() {
        let plugin = create_test_plugin();
        assert_eq!(plugin.name(), "monitors");
        assert!(plugin.current_layout.is_none());
        assert!(plugin.last_layout_time.is_none());
        assert!(!plugin.pending_layout_apply);
    }

    #[test]
    fn test_config_defaults() {
        let config = MonitorsConfig::default();
        assert!(config.startup_relayout);
        assert_eq!(config.new_monitor_delay, 1000);
        assert!(config.hotplug_command.is_none());
        assert!(config.hotplug_commands.is_empty());
        assert!(config.placement.is_empty());
        assert!(config.settings.is_empty());
        assert!(!config.debug_logging);
        assert!(config.case_insensitive);
    }

    #[test]
    fn test_monitor_info_structure() {
        let monitor = create_test_monitor("DP-1", 0, 0, 1920, 1080);

        assert_eq!(monitor.name, "DP-1");
        assert_eq!(monitor.width, 1920);
        assert_eq!(monitor.height, 1080);
        assert_eq!(monitor.x, 0);
        assert_eq!(monitor.y, 0);
        assert_eq!(monitor.scale, 1.0);
        assert!(!monitor.disabled);
    }

    #[test]
    fn test_monitor_settings() {
        let settings = MonitorSettings {
            resolution: Some("2560x1440".to_string()),
            rate: Some(144),
            scale: Some(1.5),
            transform: Some(1),
        };

        assert_eq!(settings.resolution, Some("2560x1440".to_string()));
        assert_eq!(settings.rate, Some(144));
        assert_eq!(settings.scale, Some(1.5));
        assert_eq!(settings.transform, Some(1));
    }

    #[test]
    fn test_placement_direction() {
        let directions = vec![
            PlacementDirection::Left,
            PlacementDirection::Right,
            PlacementDirection::Top,
            PlacementDirection::Bottom,
        ];

        // Test serialization/deserialization
        for direction in directions {
            let serialized = serde_json::to_string(&direction).unwrap();
            let _deserialized: PlacementDirection = serde_json::from_str(&serialized).unwrap();
        }
    }

    #[test]
    fn test_placement_alignment() {
        let alignments = vec![
            PlacementAlignment::Start,
            PlacementAlignment::Center,
            PlacementAlignment::Middle,
            PlacementAlignment::End,
        ];

        for alignment in alignments {
            let serialized = serde_json::to_string(&alignment).unwrap();
            let _deserialized: PlacementAlignment = serde_json::from_str(&serialized).unwrap();
        }
    }

    #[test]
    fn test_direction_key_parsing() {
        let plugin = create_test_plugin();

        let (direction, alignment) = plugin.parse_direction_key("leftOf");
        assert!(matches!(direction, PlacementDirection::Left));
        assert!(alignment.is_none());

        let (direction, alignment) = plugin.parse_direction_key("topCenterOf");
        assert!(matches!(direction, PlacementDirection::Top));
        assert!(matches!(alignment, Some(PlacementAlignment::Center)));

        let (direction, alignment) = plugin.parse_direction_key("rightEndOf");
        assert!(matches!(direction, PlacementDirection::Right));
        assert!(matches!(alignment, Some(PlacementAlignment::End)));
    }

    #[test]
    fn test_position_calculation() {
        let plugin = create_test_plugin();

        let source = create_test_monitor("DP-1", 0, 0, 1920, 1080);
        let target = create_test_monitor("DP-2", 1920, 0, 1920, 1080);

        let rule = ResolvedPlacementRule {
            source_monitor: "DP-1".to_string(),
            target_monitor: "DP-2".to_string(),
            direction: PlacementDirection::Left,
            alignment: None,
        };

        let (new_x, new_y) = plugin.calculate_position(&source, &target, &rule).unwrap();
        assert_eq!(new_x, 0); // 1920 - 1920 = 0
        assert_eq!(new_y, 0);

        let rule_right = ResolvedPlacementRule {
            source_monitor: "DP-1".to_string(),
            target_monitor: "DP-2".to_string(),
            direction: PlacementDirection::Right,
            alignment: None,
        };

        let (new_x, new_y) = plugin
            .calculate_position(&source, &target, &rule_right)
            .unwrap();
        assert_eq!(new_x, 3840); // 1920 + 1920 = 3840
        assert_eq!(new_y, 0);
    }

    #[test]
    fn test_monitor_exists() {
        let mut plugin = create_test_plugin();
        plugin.config.case_insensitive = true;

        let mut monitors = HashMap::new();
        monitors.insert(
            "DP-1".to_string(),
            create_test_monitor("DP-1", 0, 0, 1920, 1080),
        );

        assert!(plugin.monitor_exists("DP-1", &monitors));
        assert!(plugin.monitor_exists("dp-1", &monitors)); // case insensitive
        assert!(!plugin.monitor_exists("HDMI-1", &monitors));

        plugin.config.case_insensitive = false;
        assert!(plugin.monitor_exists("DP-1", &monitors));
        assert!(!plugin.monitor_exists("dp-1", &monitors)); // case sensitive
    }

    #[test]
    fn test_find_monitor() {
        let mut plugin = create_test_plugin();
        plugin.config.case_insensitive = true;

        let mut monitors = HashMap::new();
        let monitor = create_test_monitor("DP-1", 0, 0, 1920, 1080);
        monitors.insert("DP-1".to_string(), monitor);

        let found = plugin.find_monitor("DP-1", &monitors);
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "DP-1");

        let found_case_insensitive = plugin.find_monitor("dp-1", &monitors);
        assert!(found_case_insensitive.is_some());

        let not_found = plugin.find_monitor("HDMI-1", &monitors);
        assert!(not_found.is_none());
    }

    #[test]
    fn test_config_serialization() {
        let config = create_test_config();

        let toml_str = toml::to_string(&config).expect("Failed to serialize config");
        assert!(toml_str.contains("startup_relayout"));
        assert!(toml_str.contains("new_monitor_delay"));
        assert!(toml_str.contains("debug_logging"));
        assert!(toml_str.contains("case_insensitive"));

        let _deserialized: MonitorsConfig =
            toml::from_str(&toml_str).expect("Failed to deserialize config");
    }

    #[test]
    fn test_resolved_placement_rule() {
        let rule = ResolvedPlacementRule {
            source_monitor: "DP-1".to_string(),
            target_monitor: "DP-2".to_string(),
            direction: PlacementDirection::Top,
            alignment: Some(PlacementAlignment::Center),
        };

        assert_eq!(rule.source_monitor, "DP-1");
        assert_eq!(rule.target_monitor, "DP-2");
        assert!(matches!(rule.direction, PlacementDirection::Top));
        assert!(matches!(rule.alignment, Some(PlacementAlignment::Center)));
    }

    #[test]
    fn test_monitor_layout_structure() {
        let mut monitors = HashMap::new();
        monitors.insert(
            "DP-1".to_string(),
            create_test_monitor("DP-1", 0, 0, 1920, 1080),
        );

        let placement_rules = vec![ResolvedPlacementRule {
            source_monitor: "DP-2".to_string(),
            target_monitor: "DP-1".to_string(),
            direction: PlacementDirection::Right,
            alignment: None,
        }];

        let layout = MonitorLayout {
            monitors,
            placement_rules,
            last_update: Instant::now(),
        };

        assert_eq!(layout.monitors.len(), 1);
        assert_eq!(layout.placement_rules.len(), 1);
    }

    #[test]
    fn test_default_functions() {
        assert!(default_true());
        assert_eq!(default_monitor_delay(), 1000);
    }

    #[test]
    fn test_alignment_center_middle_equivalence() {
        // Test that Center and Middle are treated equivalently
        let center = PlacementAlignment::Center;
        let middle = PlacementAlignment::Middle;

        // Both should serialize to their respective values
        let center_json = serde_json::to_string(&center).unwrap();
        let middle_json = serde_json::to_string(&middle).unwrap();

        assert_ne!(center_json, middle_json); // They serialize differently

        // But in logic they should be treated the same
        assert!(matches!(center, PlacementAlignment::Center));
        assert!(matches!(middle, PlacementAlignment::Middle));
    }
}
