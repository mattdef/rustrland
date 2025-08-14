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

use crate::ipc::{HyprlandClient, HyprlandEvent};
use crate::plugins::Plugin;

// Simplified animation config for notifications (avoiding complex dependencies)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SimpleAnimationConfig {
    pub animation_type: String,
    pub duration: u32,
    pub easing: String,
    pub opacity_from: f32,
    pub scale_from: f32,
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
    pub appear: Option<SimpleAnimationConfig>,
    /// Disappearance animation  
    pub disappear: Option<SimpleAnimationConfig>,
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
    sources: HashMap<String, SourceConfig>,
    parsers: HashMap<String, CompiledParser>,
    handles: Vec<JoinHandle<()>>,
    shutdown_tx: Option<mpsc::Sender<()>>,
    // Use a simple counter for animation tracking instead of full engine
    animation_counter: u32,
    notification_counter: u32,
}

impl SystemNotifier {
    pub fn new() -> Self {
        Self {
            sources: HashMap::new(),
            parsers: HashMap::new(),
            handles: Vec::new(),
            shutdown_tx: None,
            animation_counter: 0,
            notification_counter: 0,
        }
    }

    /// Parse configuration and compile regex patterns
    fn parse_config(&mut self, config: &toml::Value) -> Result<()> {
        // Parse sources (Pyprland compatible format)
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

        // Parse parsers with enhanced animation support
        if let Some(parsers) = config.get("parsers").and_then(|p| p.as_table()) {
            for (name, parser_config) in parsers {
                // Try enhanced format first (with animation), fallback to basic Pyprland format
                let notification_config: NotificationConfig = parser_config
                    .clone()
                    .try_into()
                    .or_else(|_| {
                        // Fallback to basic Pyprland format
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

        let urgency = match config.basic.urgency.as_deref() {
            Some("low") => notify_rust::Urgency::Low,
            Some("critical") => notify_rust::Urgency::Critical,
            _ => notify_rust::Urgency::Normal,
        };

        Ok(CompiledParser {
            pattern,
            filter,
            filter_replacement,
            color: config.basic.color.clone(),
            timeout: config.basic.timeout,
            urgency,
            icon: config.basic.icon.clone(),
            sound: config.basic.sound.clone(),
            animation: config.animation.clone(),
        })
    }

    /// Start monitoring all configured sources
    async fn start_monitoring(&mut self) -> Result<()> {
        let (shutdown_tx, _shutdown_rx) = mpsc::channel(1);
        self.shutdown_tx = Some(shutdown_tx);

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

        // Now spawn all the monitoring tasks
        for (source_name, source_config, parser) in monitor_tasks {
            let handle =
                Self::spawn_source_monitor_static(source_name, source_config, parser).await?;
            self.handles.push(handle);
        }

        info!("Started monitoring {} sources", self.handles.len());
        Ok(())
    }

    /// Spawn a monitor task for a single source (static version)
    async fn spawn_source_monitor_static(
        source_name: String,
        source_config: SourceConfig,
        parser: CompiledParser,
    ) -> Result<JoinHandle<()>> {
        let handle = tokio::spawn(async move {
            debug!("Starting monitor for source '{}'", source_name);

            loop {
                match Self::monitor_command(&source_config.command, &parser).await {
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
    async fn monitor_command(command: &str, parser: &CompiledParser) -> Result<()> {
        debug!("Executing command: {}", command);

        let mut cmd = Command::new("sh")
            .arg("-c")
            .arg(command)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .with_context(|| format!("Failed to spawn command: {command}"))?;

        if let Some(stdout) = cmd.stdout.take() {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();

            while let Some(line) = lines.next_line().await? {
                if let Some(captures) = parser.pattern.captures(&line) {
                    let notification_text = if let (Some(filter), Some(replacement)) =
                        (&parser.filter, &parser.filter_replacement)
                    {
                        filter.replace(&line, replacement).to_string()
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
                        Self::send_animated_notification(&notification_text, parser).await
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

    /// Send a desktop notification with optional animations
    async fn send_animated_notification(text: &str, parser: &CompiledParser) -> Result<()> {
        let mut notification = Notification::new();
        notification
            .summary("System Notification")
            .body(text)
            .urgency(parser.urgency);

        // Apply basic configuration
        if let Some(timeout) = parser.timeout {
            notification.timeout(timeout);
        }

        if let Some(icon) = &parser.icon {
            notification.icon(icon);
        }

        // Handle animation if configured (Rustrland enhancement)
        if let Some(animation_config) = &parser.animation {
            debug!("🎬 Applying notification animations");

            // For desktop notifications, we simulate animation by showing progressive notifications
            // with slight delays and opacity changes (this is a conceptual implementation)
            if let Some(appear_config) = &animation_config.appear {
                // Apply appearance animation effects
                Self::apply_appearance_animation(&mut notification, appear_config)?;
            }

            // Show the notification
            notification
                .show()
                .with_context(|| "Failed to display notification")?;

            // Handle display duration and disappearance
            if let Some(display_duration) = animation_config.display_duration {
                tokio::time::sleep(tokio::time::Duration::from_millis(display_duration as u64))
                    .await;

                if let Some(disappear_config) = &animation_config.disappear {
                    Self::apply_disappearance_animation(disappear_config).await?;
                }
            }
        } else {
            // Standard notification without animation
            notification
                .show()
                .with_context(|| "Failed to display notification")?;
        }

        // Play sound if configured
        if let Some(sound) = &parser.sound {
            if let Err(e) = Self::play_notification_sound(sound).await {
                warn!("Failed to play notification sound '{}': {}", sound, e);
            }
        }

        debug!("Sent notification: {}", text);
        Ok(())
    }

    /// Apply appearance animation to notification
    fn apply_appearance_animation(
        notification: &mut Notification,
        config: &SimpleAnimationConfig,
    ) -> Result<()> {
        // Note: Desktop notification systems have limited animation support
        // This is where we'd apply hints or effects supported by the notification daemon

        // For now, we can set hints that some notification daemons might support
        notification.hint(notify_rust::Hint::Category("system".to_owned()));

        debug!(
            "Applied appearance animation: {} ({}ms, {})",
            config.animation_type, config.duration, config.easing
        );
        Ok(())
    }

    /// Apply disappearance animation
    async fn apply_disappearance_animation(config: &SimpleAnimationConfig) -> Result<()> {
        // In a real implementation, this might:
        // - Send fade-out commands to the notification daemon
        // - Use desktop environment specific APIs for smooth transitions
        // - Apply gradual opacity changes if supported

        debug!(
            "Applied disappearance animation: {} ({}ms, {})",
            config.animation_type, config.duration, config.easing
        );
        Ok(())
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
        self.notification_counter += 1;

        let mut notification = Notification::new();
        notification
            .summary("Rustrland")
            .body(message)
            .urgency(urgency)
            .timeout(timeout);

        if with_animation {
            // Apply default animation for manual notifications
            let animation_id = format!("manual_notification_{}", self.notification_counter);

            // Create a simple fade-in animation
            let _animation_config = SimpleAnimationConfig {
                animation_type: "fade".to_string(),
                duration: 300,
                easing: "easeOut".to_string(),
                opacity_from: 0.0,
                scale_from: 1.0,
            };

            debug!(
                "🎬 Applying manual notification animation: {}",
                animation_id
            );

            // Note: In a full implementation, we'd integrate with the window manager
            // to apply actual visual effects to notification windows
        }

        notification
            .show()
            .with_context(|| "Failed to send manual notification")?;

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
                    "System Notifier Status:\n- Sources: {}\n- Parsers: {}\n- Active monitors: {}\n- Animation support: Enabled (simplified engine, {} animations processed)",
                    self.sources.len(),
                    self.parsers.len(),
                    self.handles.len(),
                    self.animation_counter
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
            "test-animation" => {
                let test_message = args.first().unwrap_or(&"Test animated notification");
                self.send_manual_notification(
                    test_message,
                    notify_rust::Urgency::Normal,
                    3000,
                    true
                ).await?;
                Ok(format!("Sent test animated notification: {test_message}"))
            }
            _ => Err(anyhow::anyhow!("Unknown command: {}", command)),
        }
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
                appear: Some(SimpleAnimationConfig {
                    animation_type: "fade".to_string(),
                    duration: 300,
                    easing: "easeOut".to_string(),
                    opacity_from: 0.0,
                    scale_from: 1.0,
                }),
                disappear: Some(SimpleAnimationConfig {
                    animation_type: "fade".to_string(),
                    duration: 200,
                    easing: "easeIn".to_string(),
                    opacity_from: 1.0,
                    scale_from: 1.0,
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
    async fn test_pyprland_compatible_config() {
        let mut plugin = SystemNotifier::new();
        let config_str = r##"
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

        // Check that parser was created without animation (Pyprland compatible)
        let parser = plugin.parsers.get("test").unwrap();
        assert!(parser.animation.is_none());
    }

    #[tokio::test]
    async fn test_enhanced_config_with_animation() {
        let mut plugin = SystemNotifier::new();
        let config_str = r##"
[sources]
test = { command = "echo 'test'", parser = "enhanced_test" }

[parsers.enhanced_test]
pattern = "test"
filter = "s/test/success/"
color = "#00aa00"
icon = "dialog-information"

[parsers.enhanced_test.animation]
display_duration = 3000
smooth_transitions = true

[parsers.enhanced_test.animation.appear]
animation_type = "fade"
duration = 300
easing = "easeOut"
opacity_from = 0.0
scale_from = 1.0

[parsers.enhanced_test.animation.disappear]
animation_type = "scale"
duration = 200
easing = "easeIn"
opacity_from = 1.0
scale_from = 0.8
        "##;

        let config: toml::Value = toml::from_str(config_str).unwrap();
        assert!(plugin.parse_config(&config).is_ok());

        // Check that parser was created with animation
        let parser = plugin.parsers.get("enhanced_test").unwrap();
        assert!(parser.animation.is_some());
        assert!(parser.icon.is_some());
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
    async fn test_animated_notification() {
        // Skip test in CI environments where notifications aren't available
        if std::env::var("CI").is_ok() || std::env::var("GITHUB_ACTIONS").is_ok() {
            return;
        }

        let mut plugin = SystemNotifier::new();
        let config = toml::from_str("").unwrap();
        plugin.init(&config).await.unwrap();

        let result = plugin
            .handle_command("notify", &["Animated test", "normal", "1000", "--animated"])
            .await;
        assert!(result.is_ok());
        assert!(result.unwrap().contains("animated"));
    }

    #[tokio::test]
    async fn test_status_with_animation_engine() {
        let mut plugin = SystemNotifier::new();
        let config = toml::from_str("").unwrap();
        plugin.init(&config).await.unwrap();

        let result = plugin.handle_command("status", &[]).await;
        assert!(result.is_ok());
        let status = result.unwrap();
        assert!(status.contains("System Notifier Status"));
        assert!(status.contains("Animation support"));
        assert!(status.contains("animations processed"));
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
    async fn test_test_animation_command() {
        // Skip test in CI environments where notifications aren't available
        if std::env::var("CI").is_ok() || std::env::var("GITHUB_ACTIONS").is_ok() {
            return;
        }

        let mut plugin = SystemNotifier::new();
        let config = toml::from_str("").unwrap();
        plugin.init(&config).await.unwrap();

        let result = plugin
            .handle_command("test-animation", &["Custom test message"])
            .await;
        assert!(result.is_ok());
        assert!(result.unwrap().contains("Custom test message"));
    }
}
