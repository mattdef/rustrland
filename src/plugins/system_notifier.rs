use anyhow::{Context, Result};
use async_trait::async_trait;
use notify_rust::Notification;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};

use crate::animation::{
    AnimationConfig, AnimationEngine, EasingFunction, PropertyValue, WindowAnimator,
};
use crate::ipc::{HyprlandClient, HyprlandEvent};
use crate::plugins::Plugin;
use std::sync::Arc;
use std::time::Instant;

// Backward compatibility alias for the advanced animation system
pub type SimpleAnimationConfig = AnimationConfig;

/// Main plugin configuration for system_notifier
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemNotifierConfig {
    /// Default timeout for notifications (ms)
    pub timeout: Option<i32>,
    /// Default urgency level
    pub urgency: Option<String>,
    /// Default color for notifications
    pub color: Option<String>,
    /// Default icon for notifications
    pub icon: Option<String>,
    /// Default sound for notifications
    pub sound: Option<String>,
}

impl Default for SystemNotifierConfig {
    fn default() -> Self {
        Self {
            timeout: Some(5000),
            urgency: Some("normal".to_string()),
            color: Some("#0088ff".to_string()),
            icon: Some("info".to_string()),
            sound: None,
        }
    }
}

/// Configuration for a log source (command to monitor)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceConfig {
    /// Command to execute for log monitoring
    pub command: String,
    /// Parser to use for this source
    pub parser: String,
}

/// Configuration for a log parser with pattern matching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParserConfig {
    /// Regex pattern to match log lines
    pub pattern: String,
    /// Optional regex filter to transform notification text
    pub filter: Option<String>,
    /// Optional color for notifications
    pub color: Option<String>,
    /// Optional notification timeout in milliseconds
    pub timeout: Option<i32>,
    /// Optional urgency level (low, normal, critical)
    pub urgency: Option<String>,
    /// Optional icon for notifications
    pub icon: Option<String>,
    /// Optional sound for notifications
    pub sound: Option<String>,
}

/// Enhanced notification configuration with animation support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationConfig {
    /// Basic Pyprland-compatible configuration
    #[serde(flatten)]
    pub basic: ParserConfig,
    /// Enhanced animation configuration (Rustrland extension) - temporarily disabled
    pub animation: Option<NotificationAnimation>,
}

/// Animation configuration for notifications (Rustrland enhancement)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationAnimation {
    /// Appearance animation
    pub appear: Option<AnimationConfig>,
    /// Disappearance animation  
    pub disappear: Option<AnimationConfig>,
    /// Duration to show notification before disappearing (ms)
    pub display_duration: Option<u32>,
    /// Enable smooth fade transitions
    pub smooth_transitions: Option<bool>,
}

/// Internal parser with compiled regex
#[derive(Clone)]
struct CompiledParser {
    pattern: Regex,
    filter: Option<Regex>,
    filter_replacement: Option<String>,
    color: Option<String>,
    timeout: Option<i32>,
    urgency: notify_rust::Urgency,
    icon: Option<String>,
    sound: Option<String>,
    animation: Option<NotificationAnimation>,
}

/// System Notifier plugin for monitoring logs and sending animated notifications
pub struct SystemNotifier {
    // Main plugin configuration
    config: SystemNotifierConfig,
    sources: HashMap<String, SourceConfig>,
    parsers: HashMap<String, CompiledParser>,
    handles: Vec<JoinHandle<()>>,
    shutdown_tx: Option<mpsc::Sender<()>>,
    notification_counter: u32,
    // Startup time to avoid showing old notifications
    startup_time: Instant,
}

impl SystemNotifier {
    pub fn new() -> Self {
        Self {
            config: SystemNotifierConfig::default(),
            sources: HashMap::new(),
            parsers: HashMap::new(),
            handles: Vec::new(),
            shutdown_tx: None,
            notification_counter: 0,
            startup_time: Instant::now(),
        }
    }

    /// Parse configuration and compile regex patterns
    fn parse_config(&mut self, config: &toml::Value) -> Result<()> {
        // Parse main plugin configuration (new structure)
        if let Ok(main_config) = config.clone().try_into::<SystemNotifierConfig>() {
            // Merge with defaults to ensure all fields have values
            let mut merged_config = SystemNotifierConfig::default();
            merged_config.timeout = main_config.timeout.or(merged_config.timeout);
            merged_config.urgency = main_config.urgency.or(merged_config.urgency);
            merged_config.color = main_config.color.or(merged_config.color);
            merged_config.icon = main_config.icon.or(merged_config.icon);
            merged_config.sound = main_config.sound.or(merged_config.sound);

            self.config = merged_config;
            info!("📋 Loaded main system_notifier configuration");
        } else {
            info!("📋 Using default system_notifier configuration");
            self.config = SystemNotifierConfig::default();
        }

        // Parse sources from [system_notifier.sources] section
        if let Some(sources) = config.get("sources").and_then(|s| s.as_table()) {
            for (name, source_config) in sources {
                let source: SourceConfig = source_config
                    .clone()
                    .try_into()
                    .with_context(|| format!("Failed to parse source config for '{name}'"))?;
                self.sources.insert(name.clone(), source);
                debug!("Loaded source '{}': {}", name, self.sources[name].command);
            }
        }

        // Parse parsers from [system_notifier.parsers.*] sections with enhanced animation support
        if let Some(parsers) = config.get("parsers").and_then(|p| p.as_table()) {
            for (name, parser_config) in parsers {
                // Try enhanced format first (with animation), fallback to basic format
                let notification_config: NotificationConfig = parser_config
                    .clone()
                    .try_into()
                    .or_else(|_| {
                        // Fallback to basic format
                        let basic: ParserConfig = parser_config.clone().try_into()?;
                        Ok::<NotificationConfig, anyhow::Error>(NotificationConfig {
                            basic,
                            animation: None,
                        })
                    })
                    .with_context(|| format!("Failed to parse parser config for '{name}'"))?;

                let compiled = self
                    .compile_parser(&notification_config)
                    .with_context(|| format!("Failed to compile parser '{name}'"))?;

                self.parsers.insert(name.clone(), compiled);
                debug!("Loaded parser '{}'", name);
            }
        }

        Ok(())
    }

    /// Compile a parser configuration into runtime structures
    fn compile_parser(&self, config: &NotificationConfig) -> Result<CompiledParser> {
        let pattern = Regex::new(&config.basic.pattern)
            .with_context(|| format!("Invalid regex pattern: {}", config.basic.pattern))?;

        let (filter, filter_replacement) = if let Some(filter_str) = &config.basic.filter {
            // Parse s/pattern/replacement/ format (Pyprland compatible)
            if filter_str.starts_with("s/") && filter_str.len() > 2 {
                let parts: Vec<&str> = filter_str[2..].splitn(3, '/').collect();
                if parts.len() >= 2 {
                    let filter_pattern = Regex::new(parts[0])
                        .with_context(|| format!("Invalid filter regex: {}", parts[0]))?;
                    let replacement = parts.get(1).unwrap_or(&"").to_string();
                    (Some(filter_pattern), Some(replacement))
                } else {
                    return Err(anyhow::anyhow!("Invalid filter format: {}", filter_str));
                }
            } else {
                return Err(anyhow::anyhow!(
                    "Filter must be in s/pattern/replacement/ format"
                ));
            }
        } else {
            (None, None)
        };

        // Use parser-specific urgency or fall back to main config default
        let urgency_str = config
            .basic
            .urgency
            .as_deref()
            .or(self.config.urgency.as_deref())
            .unwrap_or("normal");
        let urgency = match urgency_str {
            "low" => notify_rust::Urgency::Low,
            "critical" => notify_rust::Urgency::Critical,
            _ => notify_rust::Urgency::Normal,
        };

        Ok(CompiledParser {
            pattern,
            filter,
            filter_replacement,
            // Use parser-specific values or fall back to main config defaults
            color: config
                .basic
                .color
                .clone()
                .or_else(|| self.config.color.clone()),
            timeout: config.basic.timeout.or(self.config.timeout),
            urgency,
            icon: config
                .basic
                .icon
                .clone()
                .or_else(|| self.config.icon.clone()),
            sound: config
                .basic
                .sound
                .clone()
                .or_else(|| self.config.sound.clone()),
            animation: config.animation.clone(),
        })
    }

    /// Start monitoring all configured sources
    async fn start_monitoring(&mut self) -> Result<()> {
        // Create broadcast channel for shutdown signals
        let (shutdown_tx, _shutdown_rx) = tokio::sync::broadcast::channel(10);

        // Store the sender in mpsc format for compatibility
        let (mpsc_tx, mut mpsc_rx) = mpsc::channel(1);
        self.shutdown_tx = Some(mpsc_tx);

        // Collect the sources and parsers to avoid borrowing issues
        let mut monitor_tasks = Vec::new();

        for (source_name, source_config) in &self.sources {
            let parser_name = &source_config.parser;

            if let Some(parser) = self.parsers.get(parser_name) {
                monitor_tasks.push((source_name.clone(), source_config.clone(), parser.clone()));
            } else {
                warn!(
                    "Parser '{}' not found for source '{}'",
                    parser_name, source_name
                );
            }
        }

        // Now spawn all the monitoring tasks with shutdown channels
        for (source_name, source_config, parser) in monitor_tasks {
            let task_shutdown_rx = shutdown_tx.subscribe();
            let handle = Self::spawn_source_monitor_with_shutdown(
                source_name,
                source_config,
                parser,
                task_shutdown_rx,
                self.startup_time,
            )
            .await?;
            self.handles.push(handle);
        }

        // Forward mpsc shutdown signal to broadcast
        let shutdown_tx_clone = shutdown_tx;
        tokio::spawn(async move {
            if mpsc_rx.recv().await.is_some() {
                let _ = shutdown_tx_clone.send(());
            }
        });

        info!("Started monitoring {} sources", self.handles.len());
        Ok(())
    }

    /// Spawn a monitor task for a single source with shutdown support
    async fn spawn_source_monitor_with_shutdown(
        source_name: String,
        source_config: SourceConfig,
        parser: CompiledParser,
        mut shutdown_rx: tokio::sync::broadcast::Receiver<()>,
        startup_time: Instant,
    ) -> Result<JoinHandle<()>> {
        let handle = tokio::spawn(async move {
            debug!("Starting monitor for source '{}'", source_name);
            loop {
                tokio::select! {
                    // Check for shutdown signal
                    _ = shutdown_rx.recv() => {
                        debug!("Received shutdown signal for source '{}'", source_name);
                        break;
                    }
                    // Monitor command
                    result = Self::monitor_command(&source_config.command, &parser, startup_time) => {
                        match result {
                            Ok(_) => {
                                debug!("Command completed for source '{}'", source_name);
                            }
                            Err(e) => {
                                error!("Error monitoring source '{}': {}", source_name, e);
                            }
                        }

                        // Restart after a delay, but also check for shutdown during delay
                        tokio::select! {
                            _ = shutdown_rx.recv() => {
                                debug!("Received shutdown signal during delay for source '{}'", source_name);
                                break;
                            }
                            _ = tokio::time::sleep(tokio::time::Duration::from_secs(5)) => {
                                // Continue loop
                            }
                        }
                    }
                }
            }
            debug!("Monitor for source '{}' shutdown complete", source_name);
        });
        Ok(handle)
    }

    /// Spawn a monitor task for a single source (static version)
    async fn spawn_source_monitor_static(
        source_name: String,
        source_config: SourceConfig,
        parser: CompiledParser,
        startup_time: Instant,
    ) -> Result<JoinHandle<()>> {
        let handle = tokio::spawn(async move {
            debug!("Starting monitor for source '{}'", source_name);

            loop {
                match Self::monitor_command(&source_config.command, &parser, startup_time).await {
                    Ok(_) => {
                        debug!("Command completed for source '{}'", source_name);
                    }
                    Err(e) => {
                        error!("Error monitoring source '{}': {}", source_name, e);
                    }
                }
                // Restart after a delay
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            }
        });

        Ok(handle)
    }

    /// Monitor a command output and send notifications for matches
    async fn monitor_command(
        command: &str,
        parser: &CompiledParser,
        startup_time: Instant,
    ) -> Result<()> {
        // Modify command to filter out old log entries for common log monitoring commands
        let filtered_command = if command.contains("journalctl") {
            // For journalctl, add --since option to only show entries from after startup
            // Use --since 'now' to avoid showing historical entries when plugin starts
            if command.contains("--since") {
                // Command already has --since, don't modify it
                command.to_string()
            } else if command.contains("|") {
                // Handle piped commands - add --since to the journalctl part before the pipe
                let parts: Vec<&str> = command.splitn(2, '|').collect();
                if parts.len() == 2 {
                    format!("{} --since 'now' |{}", parts[0].trim(), parts[1])
                } else {
                    command.to_string()
                }
            } else {
                // Add --since 'now' to prevent old notifications at startup
                format!("{} --since 'now'", command)
            }
        } else if command.contains("tail -f") {
            // For tail -f, we'll add a startup delay to avoid showing recent entries that existed before startup
            // This is particularly important for pacman.log which might have recent entries
            command.to_string()
        } else {
            // For other commands, use as-is
            command.to_string()
        };

        debug!("Executing filtered command: {}", filtered_command);

        let mut cmd = Command::new("sh")
            .arg("-c")
            .arg(&filtered_command)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .with_context(|| format!("Failed to spawn command: {command}"))?;

        if let Some(stdout) = cmd.stdout.take() {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();

            while let Some(line) = lines.next_line().await? {
                // For tail -f commands, ignore lines during the first few seconds to avoid old entries
                if command.contains("tail -f") && startup_time.elapsed().as_secs() < 10 {
                    debug!(
                        "🚫 Ignoring line during startup grace period ({}s elapsed): {}",
                        startup_time.elapsed().as_secs(),
                        line
                    );
                    continue;
                }

                if let Some(captures) = parser.pattern.captures(&line) {
                    let notification_text = if let (Some(filter), Some(replacement)) =
                        (&parser.filter, &parser.filter_replacement)
                    {
                        // Convert sed-style \1, \2 to Rust regex $1, $2 for Pyprland compatibility
                        let rust_replacement = replacement
                            .replace("\\1", "$1")
                            .replace("\\2", "$2")
                            .replace("\\3", "$3")
                            .replace("\\4", "$4")
                            .replace("\\5", "$5")
                            .replace("\\6", "$6")
                            .replace("\\7", "$7")
                            .replace("\\8", "$8")
                            .replace("\\9", "$9");
                        filter.replace(&line, rust_replacement.as_str()).to_string()
                    } else {
                        // Use the first capture group if available, otherwise the full match
                        captures
                            .get(1)
                            .or_else(|| captures.get(0))
                            .map(|m| m.as_str())
                            .unwrap_or(&line)
                            .to_string()
                    };

                    if let Err(e) =
                        Self::send_animated_notification_static(&notification_text, parser).await
                    {
                        error!("Failed to send notification: {}", e);
                    }
                }
            }
        }

        let status = cmd.wait().await?;
        if !status.success() {
            return Err(anyhow::anyhow!("Command failed with status: {}", status));
        }

        Ok(())
    }

    /// Send a desktop notification with optional animations (static version for monitoring)
    async fn send_animated_notification_static(text: &str, parser: &CompiledParser) -> Result<()> {
        // For monitoring, use Hyprland native notifications with color/icon support
        let temp_notifier = SystemNotifier::new();
        temp_notifier
            .send_hyprland_native_notification_static(text, parser)
            .await?;
        debug!("Sent monitoring notification: {}", text);
        Ok(())
    }

    /// Send a desktop notification with optional animations (instance version for manual notifications)
    async fn send_animated_notification(&self, text: &str, parser: &CompiledParser) -> Result<()> {
        // Handle animation if configured (Rustrland enhancement)
        if let Some(animation_config) = &parser.animation {
            debug!("🎬 Applying advanced notification animations");
            self.send_notification_with_animation(text, parser, animation_config)
                .await?;
        } else {
            // Standard notification without animation
            self.send_standard_notification_static(text, parser).await?;
        }

        debug!("Sent notification: {}", text);
        Ok(())
    }

    /// Send notification with advanced animation system
    async fn send_notification_with_animation(
        &self,
        text: &str,
        parser: &CompiledParser,
        animation_config: &NotificationAnimation,
    ) -> Result<()> {
        // For appear animation, create animated notification sequence
        if let Some(appear_config) = &animation_config.appear {
            debug!(
                "🎬 Starting appear animation: {} ({}ms)",
                appear_config.animation_type, appear_config.duration
            );

            // Show animated notification based on type
            match appear_config.animation_type.as_str() {
                "fade" => {
                    Self::show_fade_notification(text, parser, appear_config).await?;
                }
                "scale" => {
                    Self::show_scale_notification(text, parser, appear_config).await?;
                }
                "slide" => {
                    Self::show_slide_notification(text, parser, appear_config).await?;
                }
                _ => {
                    // Default to standard notification
                    self.send_standard_notification_static(text, parser).await?;
                }
            }
        } else {
            // Show main notification immediately
            self.send_standard_notification_static(text, parser).await?;
        }

        // Handle display duration and disappearance
        if let Some(display_duration) = animation_config.display_duration {
            tokio::time::sleep(tokio::time::Duration::from_millis(display_duration as u64)).await;

            if let Some(disappear_config) = &animation_config.disappear {
                debug!(
                    "🎬 Starting disappear animation: {} ({}ms)",
                    disappear_config.animation_type, disappear_config.duration
                );
                // For desktop notifications, we can't really "disappear" them after showing
                // But we can log the animation intent
                info!("🎬 Notification disappear animation completed");
            }
        }

        Ok(())
    }

    /// Show fade animation notification
    async fn show_fade_notification(
        text: &str,
        parser: &CompiledParser,
        config: &AnimationConfig,
    ) -> Result<()> {
        let steps = 3;
        let step_duration = config.duration / steps;

        for i in 0..steps {
            let progress = (i + 1) as f32 / steps as f32;
            let opacity = config.opacity_from + (1.0 - config.opacity_from) * progress;

            let mut notification = Notification::new();

            let summary = if i < steps - 1 {
                format!("🎬 System Notification (fade: {:.0}%)", opacity * 100.0)
            } else {
                "System Notification".to_string()
            };

            notification
                .summary(&summary)
                .body(text)
                .urgency(parser.urgency)
                .timeout(step_duration as i32);

            Self::apply_parser_config_to_notification(&mut notification, parser);

            notification.hint(notify_rust::Hint::Custom(
                "rustrland-animation".to_string(),
                format!("fade-{}-{}", i + 1, steps),
            ));

            notification
                .show()
                .with_context(|| "Failed to show fade animation step")?;

            if i < steps - 1 {
                tokio::time::sleep(tokio::time::Duration::from_millis(step_duration as u64)).await;
            }
        }

        Ok(())
    }

    /// Show scale animation notification
    async fn show_scale_notification(
        text: &str,
        parser: &CompiledParser,
        config: &AnimationConfig,
    ) -> Result<()> {
        let steps = 3;
        let step_duration = config.duration / steps;

        for i in 0..steps {
            let progress = (i + 1) as f32 / steps as f32;
            let scale = config.scale_from + (1.0 - config.scale_from) * progress;

            let mut notification = Notification::new();

            let summary = if i < steps - 1 {
                format!("📏 System Notification (scale: {:.0}%)", scale * 100.0)
            } else {
                "System Notification".to_string()
            };

            notification
                .summary(&summary)
                .body(text)
                .urgency(parser.urgency)
                .timeout(step_duration as i32);

            Self::apply_parser_config_to_notification(&mut notification, parser);

            notification.hint(notify_rust::Hint::Custom(
                "rustrland-animation".to_string(),
                format!("scale-{}-{}", i + 1, steps),
            ));

            notification
                .show()
                .with_context(|| "Failed to show scale animation step")?;

            if i < steps - 1 {
                tokio::time::sleep(tokio::time::Duration::from_millis(step_duration as u64)).await;
            }
        }

        Ok(())
    }

    /// Show slide animation notification
    async fn show_slide_notification(
        text: &str,
        parser: &CompiledParser,
        config: &AnimationConfig,
    ) -> Result<()> {
        let steps = 3;
        let step_duration = config.duration / steps;

        for i in 0..steps {
            let progress = (i + 1) as f32 / steps as f32;

            let mut notification = Notification::new();

            let summary = if i < steps - 1 {
                format!("➡️ System Notification (slide: {:.0}%)", progress * 100.0)
            } else {
                "System Notification".to_string()
            };

            notification
                .summary(&summary)
                .body(text)
                .urgency(parser.urgency)
                .timeout(step_duration as i32);

            Self::apply_parser_config_to_notification(&mut notification, parser);

            notification.hint(notify_rust::Hint::Custom(
                "rustrland-animation".to_string(),
                format!("slide-{}-{}", i + 1, steps),
            ));

            notification
                .show()
                .with_context(|| "Failed to show slide animation step")?;

            if i < steps - 1 {
                tokio::time::sleep(tokio::time::Duration::from_millis(step_duration as u64)).await;
            }
        }

        Ok(())
    }

    /// Send standard notification without animation
    async fn send_standard_notification_static(
        &self,
        text: &str,
        parser: &CompiledParser,
    ) -> Result<()> {
        debug!("🎯 send_standard_notification_static called:");
        debug!("   - text: '{}'", text);
        debug!("   - parser.color: {:?}", parser.color);

        // Always use Hyprland native notifications
        debug!("🎬 Using Hyprland native notify");
        self.send_hyprland_native_notification(text, parser).await?;

        // Play sound if configured
        if let Some(sound) = &parser.sound {
            if let Err(e) = Self::play_notification_sound(sound).await {
                warn!("Failed to play notification sound '{}': {}", sound, e);
            }
        }

        Ok(())
    }

    /// Send notification using Hyprland native notify (static version for monitoring)
    async fn send_hyprland_native_notification_static(
        &self,
        text: &str,
        parser: &CompiledParser,
    ) -> Result<()> {
        debug!("🎯 Using Hyprland native notify (Static monitoring version)");

        // Use hyprctl notify command for monitoring notifications
        let timeout = parser.timeout.unwrap_or(5000);

        // Get icon using the icon conversion method
        let icon = self.get_hyprland_icon(parser);

        // Use parser-specific color or fall back to default with conversion
        let raw_color = parser.color.as_deref().unwrap_or("0");
        let color = self.convert_color_to_hyprland_format(raw_color);

        let notify_command = format!(
            "hyprctl notify {} {} {} '{}'",
            icon,
            timeout,
            color,
            text.replace('"', "\\\"")
        );

        debug!("🔔 Executing monitoring hyprctl notify: {}", notify_command);

        match tokio::process::Command::new("sh")
            .arg("-c")
            .arg(&notify_command)
            .output()
            .await
        {
            Ok(output) => {
                if output.status.success() {
                    debug!("✅ Hyprland monitoring notification sent: {}", text);
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    warn!("⚠️ hyprctl notify failed for monitoring: {}", stderr);
                    // Fallback: try again with hyprctl
                    self.send_hyprland_native_notification(text, parser).await?;
                }
            }
            Err(e) => {
                warn!("⚠️ Failed to execute hyprctl notify for monitoring: {}", e);
                // Fallback: try basic hyprctl command
                self.send_hyprland_native_notification(text, parser).await?;
            }
        }

        Ok(())
    }

    /// Send notification using Hyprland native notify (Standard mode)
    async fn send_hyprland_native_notification(
        &self,
        text: &str,
        parser: &CompiledParser,
    ) -> Result<()> {
        debug!("🎯 Using Hyprland native notify (Standard mode)");

        // Use hyprctl notify command for Standard mode (Pyprland compatibility)
        let timeout = parser.timeout.unwrap_or(5000);

        // Build hyprctl notify command with correct syntax: hyprctl notify <icon> <time_ms> <color> <message>
        let icon = self.get_hyprland_icon(parser);

        // Use parser-specific color or fall back to main config color or default
        let raw_color = parser
            .color
            .as_deref()
            .or(self.config.color.as_deref())
            .unwrap_or("0"); // Default color for Hyprland compatibility
        let color = self.convert_color_to_hyprland_format(raw_color);

        let notify_command = format!(
            "hyprctl notify {} {} {} '{}'",
            icon,
            timeout,
            color,
            text.replace('"', "\\\"")
        );

        debug!("🔔 Executing hyprctl notify: {}", notify_command);

        match tokio::process::Command::new("sh")
            .arg("-c")
            .arg(&notify_command)
            .output()
            .await
        {
            Ok(output) => {
                if output.status.success() {
                    info!("✅ Hyprland native notification sent: {}", text);
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    warn!("⚠️ hyprctl notify failed: {}", stderr);
                    // Note: already tried hyprctl, no further fallback needed
                    warn!("Unable to send notification");
                }
            }
            Err(e) => {
                warn!("⚠️ Failed to execute hyprctl notify: {}", e);
                // Note: already tried hyprctl, no further fallback needed
                warn!("Unable to send notification");
            }
        }

        Ok(())
    }

    /// Convert icon name to Hyprland notify icon number
    fn get_hyprland_icon(&self, parser: &CompiledParser) -> &str {
        if let Some(icon_name) = &parser.icon {
            match icon_name.as_str() {
                "warning" => "0",
                "info" => "1",
                "hint" => "2",
                "error" => "3",
                "confused" => "4",
                "ok" => "5",
                "none" => "-1",
                _ => {
                    debug!(
                        "Unknown icon '{}', falling back to urgency-based icon",
                        icon_name
                    );
                    self.get_icon_from_urgency(parser.urgency)
                }
            }
        } else {
            self.get_icon_from_urgency(parser.urgency)
        }
    }

    /// Get icon number from urgency level
    fn get_icon_from_urgency(&self, urgency: notify_rust::Urgency) -> &str {
        match urgency {
            notify_rust::Urgency::Low => "2",      // Hint
            notify_rust::Urgency::Critical => "3", // Error
            _ => "1",                              // Info
        }
    }

    /// Convert color format to Hyprland-compatible format
    fn convert_color_to_hyprland_format(&self, color: &str) -> String {
        // Handle different color formats and convert to Hyprland format
        if color == "0" {
            return "0".to_string(); // Default color
        }

        // Handle rgb(r,g,b) format
        if color.starts_with("rgb(") && color.ends_with(")") {
            let inner = &color[4..color.len() - 1];
            let parts: Vec<&str> = inner.split(',').collect();
            if parts.len() == 3 {
                if let (Ok(r), Ok(g), Ok(b)) = (
                    parts[0].trim().parse::<u8>(),
                    parts[1].trim().parse::<u8>(),
                    parts[2].trim().parse::<u8>(),
                ) {
                    return format!("0xff{:02x}{:02x}{:02x}", r, g, b);
                }
            }
        }

        // Handle rgba(r,g,b,a) format
        if color.starts_with("rgba(") && color.ends_with(")") {
            let inner = &color[5..color.len() - 1];
            let parts: Vec<&str> = inner.split(',').collect();
            if parts.len() == 4 {
                if let (Ok(r), Ok(g), Ok(b), Ok(a)) = (
                    parts[0].trim().parse::<u8>(),
                    parts[1].trim().parse::<u8>(),
                    parts[2].trim().parse::<u8>(),
                    parts[3].trim().parse::<f32>(),
                ) {
                    let alpha = (a * 255.0) as u8;
                    return format!("0x{:02x}{:02x}{:02x}{:02x}", alpha, r, g, b);
                }
            }
        }

        // Handle hex formats (#RRGGBB, #RRGGBBAA)
        if let Some(hex) = color.strip_prefix("#") {
            if hex.len() == 6 {
                // #RRGGBB -> 0xffRRGGBB (full opacity)
                return format!("0xff{}", hex);
            } else if hex.len() == 8 {
                // #RRGGBBAA -> 0xAARRGGBB
                let rgba = &hex;
                let rr = &rgba[0..2];
                let gg = &rgba[2..4];
                let bb = &rgba[4..6];
                let aa = &rgba[6..8];
                return format!("0x{}{}{}{}", aa, rr, gg, bb);
            }
        }

        // Handle 0x format (already Hyprland compatible)
        if color.starts_with("0x") {
            return color.to_string();
        }

        // Fallback to default if parsing failed
        debug!("Could not parse color '{}', falling back to default", color);
        "0".to_string()
    }

    /// Apply parser configuration to notification
    fn apply_parser_config_to_notification(
        notification: &mut Notification,
        parser: &CompiledParser,
    ) {
        if let Some(timeout) = parser.timeout {
            notification.timeout(timeout);
        }

        if let Some(icon) = &parser.icon {
            notification.icon(icon);
        }
    }

    /// Play notification sound
    async fn play_notification_sound(sound_path: &str) -> Result<()> {
        // Use system sound player (paplay, aplay, etc.)
        let sound_players = ["paplay", "aplay", "ossplay"];

        for player in &sound_players {
            if let Ok(mut cmd) = Command::new(player)
                .arg(sound_path)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
            {
                let _ = cmd.wait().await;
                return Ok(());
            }
        }

        Err(anyhow::anyhow!("No compatible sound player found"))
    }

    /// Send manual notification with animation support
    async fn send_manual_notification(
        &mut self,
        message: &str,
        urgency: notify_rust::Urgency,
        timeout: i32,
        with_animation: bool,
    ) -> Result<()> {
        debug!("🔧 Manual notification called: message='{}', urgency={:?}, timeout={}, with_animation={}", 
               message, urgency, timeout, with_animation);

        // For manual notifications in simple mode, no monitor selection needed
        debug!("🔧 Sending manual notification in simple mode (Hyprland-native)");

        self.send_manual_notification_with_options(
            message,
            urgency,
            timeout,
            with_animation,
            None, // No monitor parameter needed for simple mode
        )
        .await
    }

    /// Send manual notification with full options
    async fn send_manual_notification_with_options(
        &mut self,
        message: &str,
        urgency: notify_rust::Urgency,
        timeout: i32,
        with_animation: bool,
        monitor: Option<String>,
    ) -> Result<()> {
        self.notification_counter += 1;

        info!("🚀 MANUAL NOTIFICATION DEBUG:");
        info!("   - message: '{}'", message);
        info!("   - urgency: {:?}", urgency);
        info!("   - timeout: {}ms", timeout);
        info!("   - with_animation: {}", with_animation);
        info!("   - monitor: {:?}", monitor);
        info!("🔧 MAIN CONFIG STATE:");
        info!("   - config.color: {:?}", self.config.color);
        info!("   - config.timeout: {:?}", self.config.timeout);
        info!("   - config.icon: {:?}", self.config.icon);

        // Create a temporary parser for manual notifications using main config defaults
        let manual_parser = CompiledParser {
            pattern: Regex::new(".*").unwrap(), // Match anything (not used)
            filter: None,
            filter_replacement: None,
            color: self
                .config
                .color
                .clone()
                .or_else(|| Some("#0088ff".to_string())),
            timeout: Some(timeout),
            urgency,
            icon: Some("rustrland".to_string()),
            sound: self.config.sound.clone(),
            animation: if with_animation {
                Some(NotificationAnimation {
                    appear: Some(AnimationConfig {
                        animation_type: "fade".to_string(),
                        duration: 300,
                        easing: EasingFunction::EaseOut,
                        opacity_from: 0.0,
                        target_position: None,
                        ..Default::default()
                    }),
                    disappear: None,
                    display_duration: Some(timeout as u32),
                    smooth_transitions: Some(true),
                })
            } else {
                None
            },
        };

        info!("📋 CREATED MANUAL PARSER:");
        info!("   - color: {:?}", manual_parser.color);
        info!("   - animation: {}", manual_parser.animation.is_some());

        // Use the existing notification system
        if with_animation && manual_parser.animation.is_some() {
            debug!("🎭 Sending animated notification");
            self.send_notification_with_animation(
                message,
                &manual_parser,
                manual_parser.animation.as_ref().unwrap(),
            )
            .await?;
        } else {
            debug!("📝 Sending standard notification");
            self.send_standard_notification_static(message, &manual_parser)
                .await?;
        }

        Ok(())
    }

    /// Apply animation hints to notification for supported notification daemons
    fn apply_notification_animation_hints(
        animation_id: &str,
        notification: &mut Notification,
    ) -> Result<()> {
        // Add custom hints that some notification daemons might support
        notification.hint(notify_rust::Hint::Category("system".to_owned()));
        notification.hint(notify_rust::Hint::Custom(
            "rustrland-animation".to_string(),
            animation_id.into(),
        ));
        notification.hint(notify_rust::Hint::Custom(
            "animation-type".to_string(),
            "fade-in".into(),
        ));

        debug!("🎬 Applied animation hints for {}", animation_id);
        Ok(())
    }

    /// Stop all monitoring tasks
    async fn stop_monitoring(&mut self) {
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(()).await;
        }

        // Wait for all tasks to complete
        while let Some(handle) = self.handles.pop() {
            let _ = handle.await;
        }

        info!("Stopped all monitoring tasks");
    }
}

impl Default for SystemNotifier {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Plugin for SystemNotifier {
    fn name(&self) -> &str {
        "system_notifier"
    }

    async fn init(&mut self, config: &toml::Value) -> Result<()> {
        info!("🔔 Initializing system_notifier plugin with animation support");

        self.parse_config(config)
            .with_context(|| "Failed to parse system_notifier configuration")?;

        if !self.sources.is_empty() {
            self.start_monitoring()
                .await
                .with_context(|| "Failed to start log monitoring")?;
        } else {
            warn!("No sources configured for system_notifier");
        }

        info!(
            "✅ system_notifier plugin initialized with {} sources, {} parsers (animation support: enabled)",
            self.sources.len(),
            self.parsers.len()
        );
        Ok(())
    }

    async fn handle_event(&mut self, _event: &HyprlandEvent) -> Result<()> {
        // System notifier doesn't need to handle Hyprland events directly
        Ok(())
    }

    async fn handle_command(&mut self, command: &str, args: &[&str]) -> Result<String> {
        match command {
            "notify" => {
                if args.is_empty() {
                    return Err(anyhow::anyhow!("Usage: notify <message> [urgency] [timeout] [--animated]"));
                }

                let message = args[0];
                let urgency = args.get(1).unwrap_or(&"normal");
                let timeout = args.get(2)
                    .and_then(|s| s.parse::<i32>().ok())
                    .unwrap_or(5000);
                let with_animation = args.contains(&"--animated");

                let urgency_level = match *urgency {
                    "low" => notify_rust::Urgency::Low,
                    "critical" => notify_rust::Urgency::Critical,
                    _ => notify_rust::Urgency::Normal,
                };

                self.send_manual_notification(message, urgency_level, timeout, with_animation)
                    .await
                    .with_context(|| "Failed to send notification")?;

                let animation_note = if with_animation { " (animated)" } else { "" };
                Ok(format!("Sent notification: {message}{animation_note}"))
            }
            "status" => {
                Ok(format!(
                    "System Notifier Status:\n- Simple Mode (Hyprland-native notifications)\n- Sources: {}\n- Parsers: {}\n- Active monitors: {}",
                    self.sources.len(),
                    self.parsers.len(),
                    self.handles.len()
                ))
            }
            "list-sources" => {
                let sources: Vec<String> = self.sources.keys().cloned().collect();
                Ok(format!("Configured sources: {}", sources.join(", ")))
            }
            "list-parsers" => {
                let parsers: Vec<String> = self.parsers.keys().cloned().collect();
                Ok(format!("Configured parsers: {}", parsers.join(", ")))
            }
            "test-notification" => {
                let test_message = args.first().unwrap_or(&"Test notification (hyprctl notify)");
                let temp_parser = CompiledParser {
                    pattern: regex::Regex::new(".*").unwrap(),
                    filter: None,
                    filter_replacement: None,
                    color: Some("#00ff00".to_string()),
                    timeout: Some(3000),
                    urgency: notify_rust::Urgency::Normal,
                    icon: Some("info".to_string()),
                    sound: None,
                    animation: None,
                };
                // Test notification
                self.send_hyprland_native_notification(test_message, &temp_parser).await?;
                Ok(format!("Sent test notification: {test_message}"))
            }
            _ => Err(anyhow::anyhow!("Unknown command: {}", command)),
        }
    }

    async fn cleanup(&mut self) -> Result<()> {
        info!("🧹 Cleaning up system_notifier plugin");
        self.stop_monitoring().await;
        Ok(())
    }
}

impl Drop for SystemNotifier {
    fn drop(&mut self) {
        // Note: This is a synchronous drop, so we can't await
        // In a real implementation, you might want to handle shutdown more gracefully
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.try_send(());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test;

    #[tokio::test]
    async fn test_plugin_initialization() {
        let mut plugin = SystemNotifier::new();
        let config = toml::from_str("").unwrap();
        assert!(plugin.init(&config).await.is_ok());
    }

    #[tokio::test]
    async fn test_parser_compilation() {
        let plugin = SystemNotifier::new();
        let notification_config = NotificationConfig {
            basic: ParserConfig {
                pattern: r"(\w+): Link UP$".to_string(),
                filter: Some("s/.*: (\\w+): Link.*/\\1 is active/".to_string()),
                color: Some("#00aa00".to_string()),
                timeout: Some(5000),
                urgency: Some("normal".to_string()),
                icon: Some("network-wired".to_string()),
                sound: None,
            },
            animation: Some(NotificationAnimation {
                appear: Some(AnimationConfig {
                    animation_type: "fade".to_string(),
                    duration: 300,
                    easing: EasingFunction::EaseOut,
                    opacity_from: 0.0,
                    target_position: None,
                    ..Default::default()
                }),
                disappear: Some(AnimationConfig {
                    animation_type: "fade".to_string(),
                    duration: 200,
                    easing: EasingFunction::EaseIn,
                    opacity_from: 1.0,
                    target_position: None,
                    ..Default::default()
                }),
                display_duration: Some(3000),
                smooth_transitions: Some(true),
            }),
        };

        let compiled = plugin.compile_parser(&notification_config);
        assert!(compiled.is_ok());

        let compiled = compiled.unwrap();
        assert!(compiled.animation.is_some());
    }

    #[tokio::test]
    async fn test_config_structure() {
        let mut plugin = SystemNotifier::new();
        let config_str = r##"
# Main plugin config 
color = "#ff6600"
timeout = 3000
icon = "info"

[sources]
test = { command = "echo 'test'", parser = "test" }

[parsers.test]
pattern = "test"
filter = "s/test/success/"
color = "#00aa00"
urgency = "normal"
timeout = 5000
        "##;

        let config: toml::Value = toml::from_str(config_str).unwrap();
        assert!(plugin.parse_config(&config).is_ok());
        assert_eq!(plugin.sources.len(), 1);
        assert_eq!(plugin.parsers.len(), 1);

        // Check that main config was loaded (simple mode fields only)
        assert_eq!(plugin.config.color, Some("#ff6600".to_string()));
        assert_eq!(plugin.config.timeout, Some(3000));
        assert_eq!(plugin.config.icon, Some("info".to_string()));

        // Check that parser was created successfully
        let parser = plugin.parsers.get("test").unwrap();
        assert!(parser.pattern.is_match("test"));
    }

    #[tokio::test]
    async fn test_manual_notification() {
        // Skip test in CI environments where notifications aren't available
        if std::env::var("CI").is_ok() || std::env::var("GITHUB_ACTIONS").is_ok() {
            return;
        }

        let mut plugin = SystemNotifier::new();
        let config = toml::from_str("").unwrap();
        plugin.init(&config).await.unwrap();

        let result = plugin
            .handle_command("notify", &["Test message", "normal", "1000"])
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_regex_pattern_matching() {
        let plugin = SystemNotifier::new();
        let notification_config = NotificationConfig {
            basic: ParserConfig {
                pattern: r"ERROR: (.+)".to_string(),
                filter: Some("s/ERROR: (.*)/Application error: \\1/".to_string()),
                color: Some("#ff0000".to_string()),
                timeout: None,
                urgency: Some("critical".to_string()),
                icon: Some("dialog-error".to_string()),
                sound: Some("/usr/share/sounds/error.wav".to_string()),
            },
            animation: None,
        };

        let compiled = plugin.compile_parser(&notification_config).unwrap();
        let test_line = "ERROR: Database connection failed";

        assert!(compiled.pattern.is_match(test_line));
        if let Some(captures) = compiled.pattern.captures(test_line) {
            assert_eq!(
                captures.get(1).unwrap().as_str(),
                "Database connection failed"
            );
        }
    }

    #[tokio::test]
    async fn test_simple_config() {
        let mut plugin = SystemNotifier::new();
        let config_str = r##"
# Simple configuration
color = "#ff6600"
timeout = 3000
icon = "info"

[sources]
test_notification = { command = "echo 'notification test'", parser = "test_notification" }

[parsers.test_notification]
pattern = "notification test"
urgency = "normal"
        "##;

        let config: toml::Value = toml::from_str(config_str).unwrap();
        assert!(plugin.parse_config(&config).is_ok());

        // Check main config
        assert_eq!(plugin.config.color, Some("#ff6600".to_string()));
        assert_eq!(plugin.config.timeout, Some(3000));
        assert_eq!(plugin.config.icon, Some("info".to_string()));

        // Check parser configuration
        let parser = plugin.parsers.get("test_notification").unwrap();
        assert_eq!(parser.color, Some("#ff6600".to_string()));
        assert_eq!(parser.timeout, Some(3000));
    }

    #[tokio::test]
    async fn test_parser_color_overrides() {
        let mut plugin = SystemNotifier::new();
        let config_str = r##"
# Main config with default color
color = "#0088ff"

[sources]
default_source = { command = "echo 'default'", parser = "default_parser" }
custom_source = { command = "echo 'custom'", parser = "custom_parser" }

[parsers.default_parser]
pattern = "default"
# Inherits color from main config

[parsers.custom_parser]
pattern = "custom"
color = "#ff0000"    # Override color
        "##;

        let config: toml::Value = toml::from_str(config_str).unwrap();
        assert!(plugin.parse_config(&config).is_ok());

        // Check default parser inherits from main config
        let default_parser = plugin.parsers.get("default_parser").unwrap();
        assert_eq!(default_parser.color, Some("#0088ff".to_string()));

        // Check custom parser overrides main config
        let custom_parser = plugin.parsers.get("custom_parser").unwrap();
        assert_eq!(custom_parser.color, Some("#ff0000".to_string()));
    }

    #[tokio::test]
    async fn test_manual_notification_uses_main_config() {
        // Skip test in CI environments where notifications aren't available
        if std::env::var("CI").is_ok() || std::env::var("GITHUB_ACTIONS").is_ok() {
            return;
        }

        let mut plugin = SystemNotifier::new();
        let config_str = r##"
color = "#ff6600"
timeout = 2000
icon = "info"
        "##;

        let config: toml::Value = toml::from_str(config_str).unwrap();
        plugin.init(&config).await.unwrap();

        // Manual notification should use main config defaults
        let result = plugin
            .handle_command("notify", &["Test with main config defaults"])
            .await;
        assert!(result.is_ok());
        assert!(result.unwrap().contains("Test with main config defaults"));
    }

    #[tokio::test]
    async fn test_fallback_to_defaults_when_no_main_config() {
        let mut plugin = SystemNotifier::new();
        let config_str = r##"
# No main config, only parsers
[sources]
test = { command = "echo 'test'", parser = "test" }

[parsers.test]
pattern = "test"
        "##;

        let config: toml::Value = toml::from_str(config_str).unwrap();
        assert!(plugin.parse_config(&config).is_ok());

        // Should use default values for simple mode
        assert_eq!(plugin.config.timeout, Some(5000)); // Default timeout set
        assert_eq!(plugin.config.color, Some("#0088ff".to_string())); // Default color set
        assert_eq!(plugin.config.icon, Some("info".to_string())); // Default icon set

        // Parser should be created successfully
        let parser = plugin.parsers.get("test").unwrap();
        assert!(parser.pattern.is_match("test"));
    }

    #[tokio::test]
    async fn test_color_and_icon_configuration() {
        let mut plugin = SystemNotifier::new();
        let config_str = r##"
mode = "simple"
color = "rgb(255,136,0)"
icon = "warning"
timeout = 3000

[sources]
test = { command = "echo 'test'", parser = "test" }

[parsers.test]
pattern = "test"
color = "rgba(255,68,68,0.9)"
icon = "error"
        "##;

        let config: toml::Value = toml::from_str(config_str).unwrap();
        assert!(plugin.parse_config(&config).is_ok());

        // Check main config
        assert_eq!(plugin.config.color, Some("rgb(255,136,0)".to_string()));
        assert_eq!(plugin.config.icon, Some("warning".to_string()));

        // Check parser inherits and overrides
        let parser = plugin.parsers.get("test").unwrap();
        assert_eq!(parser.color, Some("rgba(255,68,68,0.9)".to_string()));
        assert_eq!(parser.icon, Some("error".to_string()));
    }

    #[tokio::test]
    async fn test_icon_name_conversion() {
        let plugin = SystemNotifier::new();
        let parser = CompiledParser {
            pattern: regex::Regex::new(".*").unwrap(),
            filter: None,
            filter_replacement: None,
            color: None,
            timeout: None,
            urgency: notify_rust::Urgency::Normal,
            icon: Some("error".to_string()),
            sound: None,
            animation: None,
        };

        assert_eq!(plugin.get_hyprland_icon(&parser), "3");
    }

    #[tokio::test]
    async fn test_icon_fallback_to_urgency() {
        let plugin = SystemNotifier::new();
        let parser = CompiledParser {
            pattern: regex::Regex::new(".*").unwrap(),
            filter: None,
            filter_replacement: None,
            color: None,
            timeout: None,
            urgency: notify_rust::Urgency::Critical,
            icon: None,
            sound: None,
            animation: None,
        };

        assert_eq!(plugin.get_hyprland_icon(&parser), "3"); // Critical = Error icon
    }

    #[tokio::test]
    async fn test_color_format_conversion() {
        let plugin = SystemNotifier::new();

        // Test RGB format
        assert_eq!(
            plugin.convert_color_to_hyprland_format("rgb(255,68,68)"),
            "0xffff4444"
        );

        // Test RGBA format
        assert_eq!(
            plugin.convert_color_to_hyprland_format("rgba(255,68,68,0.9)"),
            "0xe5ff4444"
        );

        // Test hex format
        assert_eq!(
            plugin.convert_color_to_hyprland_format("#ff4444"),
            "0xffff4444"
        );
        assert_eq!(
            plugin.convert_color_to_hyprland_format("#ff4444aa"),
            "0xaaff4444"
        );

        // Test 0x format (already compatible)
        assert_eq!(
            plugin.convert_color_to_hyprland_format("0xff4444ff"),
            "0xff4444ff"
        );

        // Test default
        assert_eq!(plugin.convert_color_to_hyprland_format("0"), "0");

        // Test invalid format
        assert_eq!(plugin.convert_color_to_hyprland_format("invalid"), "0");
    }

    #[tokio::test]
    async fn test_color_inheritance_hierarchy() {
        let mut plugin = SystemNotifier::new();
        let config_str = r##"
mode = "simple"
color = "rgb(0,136,255)"

[sources]
inherit_test = { command = "echo 'inherit'", parser = "inherit_parser" }
override_test = { command = "echo 'override'", parser = "override_parser" }

[parsers.inherit_parser]
pattern = "inherit"
# Should inherit main config color

[parsers.override_parser]
pattern = "override"
color = "rgb(255,68,68)"
# Should use its own color
        "##;

        let config: toml::Value = toml::from_str(config_str).unwrap();
        assert!(plugin.parse_config(&config).is_ok());

        let inherit_parser = plugin.parsers.get("inherit_parser").unwrap();
        assert_eq!(inherit_parser.color, Some("rgb(0,136,255)".to_string()));

        let override_parser = plugin.parsers.get("override_parser").unwrap();
        assert_eq!(override_parser.color, Some("rgb(255,68,68)".to_string()));
    }
}
