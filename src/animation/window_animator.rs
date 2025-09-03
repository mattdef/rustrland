use anyhow::{Error, Result};
use hyprland::data::Monitor;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};
use tracing::{debug, info, warn};
use tracing_subscriber;
use tracing_subscriber::fmt::format;

use super::{properties::PropertyValue, AnimationConfig, AnimationEngine};
use crate::animation::easing::EasingFunction;
use crate::ipc::{self, HyprlandClient, MonitorInfo};
use crate::plugins::monitors;
use hyprland::ctl::Color;
use hyprland::keyword::{Keyword, OptionValue};

#[derive(Debug, Clone)]
pub struct HyprlandStyle {
    pub border_size: i32,
    pub active_border_color: String,
    pub inactive_border_color: String,
    pub drop_shadow: bool,
    pub shadow_range: i32,
    pub shadow_render_power: i32,
    pub shadow_color: String,
}

impl Default for HyprlandStyle {
    fn default() -> Self {
        Self {
            border_size: 1,
            active_border_color: "rgba(777777AA)".to_string(),
            inactive_border_color: "rgba(595959AA)".to_string(),
            drop_shadow: true,
            shadow_range: 4,
            shadow_render_power: 3,
            shadow_color: "rgba(1a1a1aee)".to_string(),
        }
    }
}

/// Manages animations for Hyprland windows
pub struct WindowAnimator {
    pub animation_engine: Arc<Mutex<AnimationEngine>>,
    pub hyprland_client: Arc<Mutex<Option<Arc<HyprlandClient>>>>,
    pub active_monitor: Arc<Mutex<MonitorInfo>>,
    active_window_animations: HashMap<String, WindowAnimationState>,
}

#[derive(Debug)]
struct WindowAnimationState {
    window_address: String,
    original_position: (i32, i32),
    original_size: (i32, i32),
    target_position: (i32, i32),
    target_size: (i32, i32),
    animation_id: String,
    is_showing: bool,
}

impl Default for WindowAnimator {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowAnimator {
    pub fn new() -> Self {
        Self {
            animation_engine: Arc::new(Mutex::new(AnimationEngine::new())),
            hyprland_client: Arc::new(Mutex::new(None)),
            active_monitor: Arc::new(Mutex::new(MonitorInfo::new())),
            active_window_animations: HashMap::new(),
        }
    }

    /// Set the Hyprland client for window manipulation
    pub async fn set_hyprland_client(&self, client: Arc<HyprlandClient>) {
        let mut client_guard = self.hyprland_client.lock().await;
        *client_guard = Some(client);
    }

    /// Set the active monitor information
    pub async fn set_active_monitor(&self, monitor_info: &MonitorInfo) {
        let mut monitor_guard = self.active_monitor.lock().await;
        *monitor_guard = monitor_info.clone();
    }

    /// Animate a window showing with specified animation
    pub async fn show_window_with_animation(
        &mut self,
        app: &str,
        target_position: (i32, i32), // Coordinates relative to current monitor
        target_size: (i32, i32),
        config: AnimationConfig,
    ) -> Result<Option<hyprland::data::Client>> {
        // Calculate starting position based on animation type
        let start_position = self
            .calculate_start_position(target_position, target_size, &config)
            .await?;
        debug!(
            "Start position from x:{} and y:{}",
            start_position.0, start_position.1
        );

        // Get existing windows before launching to identify the new one
        let client_guard = self.hyprland_client.lock().await;
        let client = match client_guard.as_ref() {
            Some(client) => client,
            None => return Ok(None),
        };
        
        let existing_windows = client.get_windows().await?;
        let existing_addresses: std::collections::HashSet<String> = existing_windows
            .iter()
            .map(|w| w.address.to_string())
            .collect();
        drop(client_guard);

        // Determine app command and class based on app type
        let (app_command, app_class) = if app == "foot" {
            // foot supports --app-id for custom class
            let class = format!("{app}_toggle");
            (format!("{} --app-id {}", app, class), class)
        } else {
            // Other apps: use normal class detection
            (app.to_string(), app.to_string())
        };

        // Get Hyprland style for consistent appearance
        let style = self.get_hyprland_style().await;

        // Create windowrulev2 rules with explicit priorities to override defaults
        let popup_float_rule = format!(
            "hyprctl keyword windowrulev2 'float, class:^{}$'",
            app_class
        );

        // Use completely disabled decorations initially
        let popup_nodeco_rule = format!(
            "hyprctl keyword windowrulev2 'nodecoration, class:^{}$'",
            app_class
        );

        let popup_pin_rule = format!("hyprctl keyword windowrulev2 'pin, class:^{}$'", app_class);

        // Try removing all existing window rules first
        let clear_rules_cmd = format!("hyprctl keyword windowrulev2 unset,class:^{}$", app_class);

        // Clear existing rules first
        tokio::process::Command::new("sh")
            .arg("-c")
            .arg(&clear_rules_cmd)
            .output()
            .await
            .ok();

        // Apply all rules with error handling
        let rules = vec![&popup_float_rule, &popup_nodeco_rule, &popup_pin_rule];

        for rule in rules {
            if let Err(e) = tokio::process::Command::new("sh")
                .arg("-c")
                .arg(rule)
                .output()
                .await
            {
                warn!("Failed to apply windowrule: {}", e);
            }
        }

        if !style.drop_shadow {
            let popup_shadow_rule = format!(
                "hyprctl keyword windowrulev2 'noshadow, class:^{}$'",
                app_class
            );

            if let Err(e) = tokio::process::Command::new("sh")
                .arg("-c")
                .arg(&popup_shadow_rule)
                .output()
                .await
            {
                warn!("Failed to apply shadow rule: {}", e);
            }
        }

        // Small delay to ensure rules are processed
        sleep(Duration::from_millis(50)).await;

        debug!("🎨 Applied global popup window rule for all *_popup windows");

        self.spawn_window_offscreen(
            &app_command,
            (start_position.0, start_position.1),
            (target_size.0, target_size.1),
        )
        .await?;

        // Wait for the new window using the improved detection
        let window = if app == "foot" {
            // For foot, we can use the custom class
            self.wait_for_window_by_class(&app_class, 5000).await?
        } else {
            // For other apps, detect new window by comparing addresses
            self.wait_for_new_window_by_class(&app_class, &existing_addresses, 5000).await?
        };

        if let Some(window) = window {
            let address = window.address.to_string();

            debug!(
                "🎨 Window {} created with popup rule automatically applied",
                address
            );

            // Apply decorations directly to the window to override defaults
            self.apply_popup_decorations(&address, &style).await;

            // Set window to starting position instantly
            let start_offset_position = self.get_offscreen_position(start_position).await;
            let target_offset_position = self.get_offscreen_position(target_position).await;
            debug!(
                "🎬 Setting window to start position: ({}, {})",
                start_offset_position.0, start_offset_position.1
            );
            //self.set_window_properties(&client, &address, start_offset_position, target_size, 0.0)
            //    .await?;

            // Create animation state
            let animation_id = format!("show_{address}");
            let state = WindowAnimationState {
                window_address: address.to_string(),
                original_position: start_offset_position,
                original_size: target_size,
                target_position: target_offset_position,
                target_size,
                animation_id: animation_id.clone(),
                is_showing: true,
            };

            self.active_window_animations
                .insert(address.to_string(), state);

            // Prepare animation properties - CORRECTED: Pass final position as initial_properties
            // AnimationEngine expects final position and calculates start position internally
            let mut end_properties = HashMap::new();
            let mut initial_properties = HashMap::new();
            end_properties.insert(
                "x".to_string(),
                PropertyValue::Pixels(target_offset_position.0),
            );
            end_properties.insert(
                "y".to_string(),
                PropertyValue::Pixels(target_offset_position.1),
            );
            end_properties.insert("width".to_string(), PropertyValue::Pixels(target_size.0));
            end_properties.insert("height".to_string(), PropertyValue::Pixels(target_size.1));

            // Add opacity for fade animations
            if config.animation_type.contains("fade") {
                end_properties.insert("opacity".to_string(), PropertyValue::Float(1.0));
            }

            // Add scale for scale animations
            if config.animation_type.contains("scale") {
                end_properties.insert("scale".to_string(), PropertyValue::Float(1.0));
            }

            initial_properties.insert(
                "x".to_string(),
                PropertyValue::Pixels(start_offset_position.0),
            );
            initial_properties.insert(
                "y".to_string(),
                PropertyValue::Pixels(start_offset_position.1),
            );
            initial_properties.insert("width".to_string(), PropertyValue::Pixels(target_size.0));
            initial_properties.insert("height".to_string(), PropertyValue::Pixels(target_size.1));

            // Add opacity for fade animations
            if config.animation_type.contains("fade") {
                debug!(
                    "🎭 Setting up fade animation: opacity_from={} -> 1.0",
                    config.opacity_from
                );
                initial_properties.insert(
                    "opacity".to_string(),
                    PropertyValue::Float(config.opacity_from),
                );
            }

            // Add scale for scale animations
            if config.animation_type.contains("scale") {
                initial_properties
                    .insert("scale".to_string(), PropertyValue::Float(config.scale_from));
            }

            // Store animation type before moving config
            let animation_type = config.animation_type.clone();

            // Start the animation
            let mut engine = self.animation_engine.lock().await;
            debug!(
                "🎬 Starting animation '{}' with {} initial properties",
                animation_id,
                initial_properties.len()
            );
            engine
                .start_animation(
                    animation_id.clone(),
                    config,
                    initial_properties,
                    end_properties,
                )
                .await?;
            debug!("✅ Animation '{}' started successfully", animation_id);

            let monitor = &self.active_monitor.lock().await;
            // Start window update loop
            self.start_window_animation_loop(
                address.to_string(),
                animation_id,
                animation_type,
                monitor.refresh_rate,
            )
            .await?;

            return Ok(Some(window));
        } else {
            println!("Window not found");
        }

        Ok(None)
    }

    // Close a window
    pub async fn close_window(&mut self, window_adress: &str) -> Result<()> {
        let client_guard = self.hyprland_client.lock().await;
        let client = match client_guard.as_ref() {
            Some(client) => client,
            None => return Ok(()),
        };

        // Close window
        client.close_window(window_adress).await?;

        Ok(())
    }

    /// Animate a window hiding with specified animation
    pub async fn hide_window(
        &mut self,
        window_address: &str,
        current_position: (i32, i32),
        current_size: (i32, i32),
        config: AnimationConfig,
    ) -> Result<()> {
        debug!(
            "🎬 Starting hide animation for window {} with type '{}'",
            window_address, config.animation_type
        );

        let absolute_current_position = {
            let monitor = self.active_monitor.lock().await;
            (
                monitor.x + current_position.0,
                monitor.y + current_position.1,
            )
        };

        // Calculate ending position based on animation type
        let end_position = self
            .calculate_end_position(absolute_current_position, current_size, &config)
            .await?;

        println!(
            "End position: x:{} and y:{}",
            end_position.0, end_position.1
        );

        // Create animation state
        let animation_id = format!("hide_{window_address}");
        let state = WindowAnimationState {
            window_address: window_address.to_string(),
            original_position: absolute_current_position,
            original_size: current_size,
            target_position: end_position,
            target_size: current_size,
            animation_id: animation_id.clone(),
            is_showing: false,
        };

        self.active_window_animations
            .insert(window_address.to_string(), state);

        // Prepare animation properties
        let mut initial_properties = HashMap::new();
        initial_properties.insert(
            "x".to_string(),
            PropertyValue::Pixels(absolute_current_position.0),
        );
        initial_properties.insert(
            "y".to_string(),
            PropertyValue::Pixels(absolute_current_position.1),
        );
        initial_properties.insert("width".to_string(), PropertyValue::Pixels(current_size.0));
        initial_properties.insert("height".to_string(), PropertyValue::Pixels(current_size.1));
        initial_properties.insert("opacity".to_string(), PropertyValue::Float(1.0));
        initial_properties.insert("scale".to_string(), PropertyValue::Float(1.0));
        let mut target_properties = HashMap::new();
        target_properties.insert("x".to_string(), PropertyValue::Pixels(end_position.0));
        target_properties.insert("y".to_string(), PropertyValue::Pixels(end_position.1));
        target_properties.insert("width".to_string(), PropertyValue::Pixels(current_size.0));
        target_properties.insert("height".to_string(), PropertyValue::Pixels(current_size.1));
        target_properties.insert("opacity".to_string(), PropertyValue::Float(0.0));
        target_properties.insert("scale".to_string(), PropertyValue::Float(0.0));

        // Store animation type before moving config
        let animation_type = config.animation_type.clone();

        // Start the animation
        let mut engine = self.animation_engine.lock().await;
        engine
            .start_animation(
                animation_id.clone(),
                config,
                initial_properties,
                target_properties,
            )
            .await?;

        let refresh_rate = {
            let monitor = self.active_monitor.lock().await;
            monitor.refresh_rate
        };

        // Start window update loop
        self.start_window_animation_loop(
            window_address.to_string(),
            animation_id,
            animation_type,
            refresh_rate,
        )
        .await?;

        Ok(())
    }

    /// Calculate starting position for show animation
    async fn calculate_start_position(
        &self,
        target_position: (i32, i32),
        target_size: (i32, i32),
        config: &AnimationConfig,
    ) -> Result<(i32, i32)> {
        let monitor = &self.active_monitor.lock().await;
        let offset_pixels = self.parse_offset(&config.offset, (monitor.width, monitor.height))?;

        match config.animation_type.as_str() {
            "fromTop" => Ok((target_position.0, -target_size.1 - offset_pixels)),
            "fromBottom" => Ok((
                target_position.0,
                target_size.1 + offset_pixels + monitor.height as i32,
            )),
            "fromLeft" => Ok((-target_size.0 - offset_pixels, target_position.1)),
            "fromRight" => Ok((
                monitor.width as i32 + target_size.0 + offset_pixels,
                target_position.1,
            )),
            "fromTopLeft" => Ok((
                -target_size.0 - offset_pixels,
                -target_size.1 - offset_pixels,
            )),
            "fromTopRight" => Ok((
                monitor.width as i32 + target_size.0 + offset_pixels,
                -target_size.1 - offset_pixels,
            )),
            "fromBottomLeft" => Ok((
                -target_size.0 - offset_pixels,
                target_size.1 + offset_pixels + monitor.height as i32,
            )),
            "fromBottomRight" => Ok((
                monitor.width as i32 + target_size.0 + offset_pixels,
                target_size.1 + offset_pixels + monitor.height as i32,
            )),
            "bounce" => Ok((target_position.0, -target_size.1 - offset_pixels)), // Start completely above screen
            "fade" => Ok((target_position.0, target_position.1)), // No position change for fade
            "scale" => Ok((target_position.0, target_position.1)), // No position change for scale
            _ => Ok((target_position.0, target_position.1)),      // Default to target position
        }
    }

    /// Calculate ending position for hide animation
    async fn calculate_end_position(
        &self,
        current_position: (i32, i32),
        current_size: (i32, i32),
        config: &AnimationConfig,
    ) -> Result<(i32, i32)> {
        let monitor = &self.active_monitor.lock().await;
        let screen_size = (monitor.width, monitor.height);
        let offset_pixels = self.parse_offset(&config.offset, screen_size)?;

        match config.animation_type.as_str() {
            "toTop" | "fromTop" => Ok((current_position.0, -current_size.1 - offset_pixels)),
            "toBottom" | "fromBottom" => {
                Ok((current_position.0, screen_size.1 as i32 + offset_pixels))
            }
            "toLeft" | "fromLeft" => Ok((-current_size.0 - offset_pixels, current_position.1)),
            "toRight" | "fromRight" => {
                Ok((screen_size.0 as i32 + offset_pixels, current_position.1))
            }
            "fade" => Ok(current_position), // No position change for fade
            "scale" => Ok(current_position), // No position change for scale
            _ => Ok(current_position),      // Default to current position
        }
    }

    /// Calculate center position for a given monitor
    /// For now, returns screen center - in production this should query actual monitor geometry
    pub async fn calculate_monitor_center_position(
        &self,
        window_size: (i32, i32),
    ) -> Result<(i32, i32)> {
        let monitor = &self.active_monitor.lock().await;
        let screen_size = (monitor.width as i32, monitor.height as i32);

        // Calculate center position
        let center_x = (screen_size.0 - window_size.0) / 2;
        let center_y = (screen_size.1 - window_size.1) / 2;

        Ok((center_x, center_y))
    }

    /// Window animation update loop
    async fn start_window_animation_loop(
        &self,
        window_address: String,
        animation_id: String,
        animation_type: String,
        refresh_rate: f32,
    ) -> Result<()> {
        let client_guard = self.hyprland_client.lock().await;
        let client = match client_guard.as_ref() {
            Some(client) => client.clone(),
            None => return Ok(()),
        };
        drop(client_guard); // Release the lock before spawning

        let engine = Arc::clone(&self.animation_engine);
        let window_address_for_unpin = window_address.clone();

        tokio::spawn(async move {
            debug!("🎯 Animation loop started for window {}", window_address);
            let mut frame_count = 0;
            let refresh_ms = (1000.0 / refresh_rate).round() as u64;

            loop {
                tokio::time::sleep(tokio::time::Duration::from_millis(refresh_ms)).await; // Adapt to refresh rate of the monitor
                frame_count += 1;

                let properties = {
                    let mut engine_guard = engine.lock().await;
                    match engine_guard.get_current_properties(&animation_id) {
                        Some(props) => {
                            // Animation running smoothly
                            debug!("📊 Frame {}: Got properties from engine", frame_count);
                            props
                        }
                        None => {
                            debug!(
                                "✅ Animation {} completed naturally after {} frames, ending loop",
                                animation_id, frame_count
                            );
                            break; // Animation completed
                        }
                    }
                };

                // Apply properties to window with animation type context
                if let Err(e) = Self::apply_properties_to_window_static(
                    &client,
                    &window_address,
                    &properties,
                    &animation_type,
                )
                .await
                {
                    debug!("Failed to apply animation properties: {}", e);
                }
            }

            debug!(
                "✅ Animation loop completed for window {} after {} frames",
                window_address, frame_count
            );

            // Clean up window rules after animation completes
            let unpin_cmd = format!(
                "hyprctl keyword windowrulev2 unset pin,address:{}",
                window_address_for_unpin
            );

            tokio::process::Command::new("sh")
                .arg("-c")
                .arg(&unpin_cmd)
                .output()
                .await
                .ok();

            debug!(
                "📌 Unpinned window {} after animation",
                window_address_for_unpin
            );
        });

        Ok(())
    }

    /// Apply animation properties to window via Hyprland commands (static version)
    async fn apply_properties_to_window_static(
        client: &HyprlandClient,
        window_address: &str,
        properties: &HashMap<String, PropertyValue>,
        animation_type: &str,
    ) -> Result<()> {
        // Extract position
        let x = properties.get("x").map(|p| p.as_pixels()).unwrap_or(0);
        let y = properties.get("y").map(|p| p.as_pixels()).unwrap_or(0);

        // Extract size
        let width = properties
            .get("width")
            .map(|p| p.as_pixels())
            .unwrap_or(800);
        let height = properties
            .get("height")
            .map(|p| p.as_pixels())
            .unwrap_or(600);

        debug!(
            "🪟 WINDOW POSITION: x={} y={} (animation_type: {})",
            x, y, animation_type
        );

        // Move window using precise pixel positioning (required for animations)
        client.move_window_pixel(window_address, x, y).await?;

        // Resize window for scale animations
        if animation_type.contains("scale") {
            client.resize_window(window_address, width, height).await?;
        }

        // Handle opacity changes ONLY for fade animations to prevent visual artifacts
        if animation_type.contains("fade") {
            if let Some(PropertyValue::Float(opacity)) = properties.get("opacity") {
                client.set_window_opacity(window_address, *opacity).await?;
            }
        }

        Ok(())
    }

    /// Apply animation properties to window via Hyprland commands
    async fn apply_properties_to_window(
        &self,
        window_address: &str,
        properties: &HashMap<String, PropertyValue>,
        animation_type: &str,
    ) -> Result<()> {
        let client_guard = self.hyprland_client.lock().await;
        let client = match client_guard.as_ref() {
            Some(client) => client,
            None => return Ok(()),
        };

        Self::apply_properties_to_window_static(client, window_address, properties, animation_type)
            .await
    }

    /// Set window properties directly
    async fn set_window_properties(
        &self,
        window_address: &str,
        position: (i32, i32),
        size: (i32, i32),
        opacity: f32,
    ) -> Result<()> {
        let client_guard = self.hyprland_client.lock().await;
        let client = match client_guard.as_ref() {
            Some(client) => client,
            None => return Ok(()),
        };

        client
            .move_window_pixel(window_address, position.0, position.1)
            .await?;
        client.resize_window(window_address, size.0, size.1).await?;

        if opacity < 1.0 {
            client.set_window_opacity(window_address, opacity).await?;
        }

        debug!(
            "   🎬 Window {} moved to ({}, {}) and resized to {}x{} with opacity {}",
            window_address, position.0, position.1, size.0, size.1, opacity
        );

        Ok(())
    }

    /// Parse offset string to pixels
    fn parse_offset(&self, offset: &str, screen_size: (u16, u16)) -> Result<i32> {
        if offset.ends_with('%') {
            let percent = offset.trim_end_matches('%').parse::<f32>()?;
            Ok(((screen_size.0.max(screen_size.1) as f32) * percent / 100.0) as i32)
        } else if offset.ends_with("px") {
            Ok(offset.trim_end_matches("px").parse::<i32>()?)
        } else {
            Ok(offset.parse::<i32>()?)
        }
    }

    /// Stop animation for a window
    pub async fn stop_animation(&mut self, window_address: &str) -> Result<()> {
        if let Some(state) = self.active_window_animations.remove(window_address) {
            let mut engine = self.animation_engine.lock().await;
            engine.stop_animation(&state.animation_id)?;
            info!("⏹️  Stopped animation for window {}", window_address);
        }
        Ok(())
    }

    /// Check if window is currently animating
    pub fn is_animating(&self, window_address: &str) -> bool {
        self.active_window_animations.contains_key(window_address)
    }

    /// Get performance statistics
    pub async fn get_performance_stats(&self) -> super::PerformanceStats {
        let engine = self.animation_engine.lock().await;
        engine.get_performance_stats()
    }

    /// Spawn window off-screen using Hyprland API (best practice for animations)
    pub async fn spawn_window_offscreen(
        &self,
        app: &str,
        spawn: (i32, i32),
        window_size: (i32, i32),
    ) -> Result<()> {
        let client_guard = self.hyprland_client.lock().await;
        let client = match client_guard.as_ref() {
            Some(client) => client,
            None => return Ok(()),
        };

        debug!(
            "🚀 Spawning {} off-screen at ({}, {}) with size {}x{}",
            app, spawn.0, spawn.1, window_size.0, window_size.1
        );

        // Add border colors to prevent style flash
        let exec_command = format!(
            "[move {} {};size {} {}] {}",
            spawn.0, spawn.1, window_size.0, window_size.1, app
        );

        match client.spawn_app(exec_command.as_str()).await {
            Ok(_) => {
                debug!("✅ Simple spawn worked");
                debug!("Exec command: {}", exec_command);
            }
            Err(e) => {
                debug!("❌ Even simple spawn failed: {}", e);
                return Err(e.into());
            }
        }

        info!(
            "✅ Spawned {} with position rules at ({}, {}) size {}x{}",
            app, spawn.0, spawn.1, window_size.0, window_size.1
        );
        Ok(())
    }

    /// Wait for window with specific class to appear (intelligent detection)
    pub async fn wait_for_window_by_class(
        &self,
        class: &str,
        timeout_ms: u64,
    ) -> Result<Option<hyprland::data::Client>> {
        debug!(
            "🔍 Waiting for {} window (timeout: {}ms)",
            class, timeout_ms
        );

        let client_guard = self.hyprland_client.lock().await;
        let client = match client_guard.as_ref() {
            Some(client) => client,
            None => return Ok(None),
        };

        let max_attempts = timeout_ms / 100;

        for attempt in 1..=max_attempts {
            let windows = client.get_windows().await?;

            if let Some(window) = windows
                .iter()
                .find(|w| w.class.to_lowercase().contains(&class.to_lowercase()))
                .cloned()
            {
                debug!(
                    "✅ Found {} window after {}ms: {}",
                    class,
                    attempt * 100,
                    window.address
                );
                return Ok(Some(window));
            }

            // Progress logging every 500ms to avoid spam
            if attempt % 5 == 0 {
                debug!(
                    "Still waiting for {} window... attempt {}/{}",
                    class, attempt, max_attempts
                );
            }

            sleep(Duration::from_millis(100)).await;
        }

        debug!(
            "⏰ Timeout waiting for {} window after {}ms",
            class, timeout_ms
        );
        Ok(None)
    }

    /// Wait for new window with specific class by comparing before/after window lists
    pub async fn wait_for_new_window_by_class(
        &self,
        class: &str,
        existing_addresses: &std::collections::HashSet<String>,
        timeout_ms: u64,
    ) -> Result<Option<hyprland::data::Client>> {
        debug!(
            "🔍 Waiting for NEW {} window (timeout: {}ms)",
            class, timeout_ms
        );

        let client_guard = self.hyprland_client.lock().await;
        let client = match client_guard.as_ref() {
            Some(client) => client,
            None => return Ok(None),
        };

        let max_attempts = timeout_ms / 100;

        for attempt in 1..=max_attempts {
            let windows = client.get_windows().await?;

            // Find new windows of the specified class
            if let Some(window) = windows
                .iter()
                .find(|w| {
                    w.class.to_lowercase().contains(&class.to_lowercase()) 
                    && !existing_addresses.contains(&w.address.to_string())
                })
                .cloned()
            {
                debug!(
                    "✅ Found NEW {} window after {}ms: {}",
                    class,
                    attempt * 100,
                    window.address
                );
                return Ok(Some(window));
            }

            // Progress logging every 500ms to avoid spam
            if attempt % 5 == 0 {
                debug!(
                    "Still waiting for NEW {} window... attempt {}/{}",
                    class, attempt, max_attempts
                );
            }

            sleep(Duration::from_millis(100)).await;
        }

        debug!(
            "⏰ Timeout waiting for NEW {} window after {}ms",
            class, timeout_ms
        );
        Ok(None)
    }

    /// Wait for window to reach specific position (validation)
    pub async fn wait_for_window_positioning(
        &self,
        address: &str,
        expected_condition: impl Fn(i16, i16) -> bool,
        timeout_ms: u64,
    ) -> Result<bool> {
        debug!(
            "🎯 Waiting for window {} to reach expected position",
            address
        );

        let client_guard = self.hyprland_client.lock().await;
        let client = match client_guard.as_ref() {
            Some(client) => client,
            None => return Ok(false),
        };

        let max_attempts = timeout_ms / 50;

        for attempt in 1..=max_attempts {
            let windows = client.get_windows().await?;

            if let Some(window) = windows.iter().find(|w| w.address.to_string() == address) {
                if expected_condition(window.at.0, window.at.1) {
                    info!(
                        "✅ Window positioned correctly at ({}, {}) after {}ms",
                        window.at.0,
                        window.at.1,
                        attempt * 50
                    );
                    return Ok(true);
                }
            }

            sleep(Duration::from_millis(50)).await;
        }

        warn!("⏰ Window positioning timeout after {}ms", timeout_ms);
        Ok(false)
    }

    /// Calculate optimal off-screen position for animation type
    pub async fn get_offscreen_position(&self, position: (i32, i32)) -> (i32, i32) {
        let monitor = &self.active_monitor.lock().await;

        (monitor.x + position.0, monitor.y + position.1)
    }

    /// Get current Hyprland style configuration
    async fn get_hyprland_style(&self) -> HyprlandStyle {
        let mut style = HyprlandStyle::default();

        // Get border size
        if let Ok(border_size) = Keyword::get("general:border_size") {
            if let OptionValue::Int(size) = border_size.value {
                style.border_size = size as i32;
            }
        }

        // Try to get border colors via hyprctl command as fallback
        if let Ok(output) = tokio::process::Command::new("hyprctl")
            .arg("getoption")
            .arg("general:col.active_border")
            .output()
            .await
        {
            let output_str = String::from_utf8_lossy(&output.stdout);
            if let Some(hex_part) = self.extract_hex_from_hyprctl_output(&output_str) {
                debug!("🎨 Active border from hyprctl: {}", hex_part);
                style.active_border_color = self.hex_to_rgba(&hex_part);
            }
        }

        if let Ok(output) = tokio::process::Command::new("hyprctl")
            .arg("getoption")
            .arg("general:col.inactive_border")
            .output()
            .await
        {
            let output_str = String::from_utf8_lossy(&output.stdout);
            if let Some(hex_part) = self.extract_hex_from_hyprctl_output(&output_str) {
                debug!("🎨 Inactive border from hyprctl: {}", hex_part);
                style.inactive_border_color = self.hex_to_rgba(&hex_part);
            }
        }

        // Get shadow settings
        if let Ok(drop_shadow) = Keyword::get("decoration:shadow:enabled") {
            if let OptionValue::Int(shadow) = drop_shadow.value {
                style.drop_shadow = shadow != 0;
            }
        }

        if let Ok(shadow_range) = Keyword::get("decoration:shadow:range") {
            if let OptionValue::Int(range) = shadow_range.value {
                style.shadow_range = range as i32;
            }
        }

        if let Ok(shadow_power) = Keyword::get("decoration:shadow:render_power") {
            if let OptionValue::Int(power) = shadow_power.value {
                style.shadow_render_power = power as i32;
            }
        }

        if let Ok(shadow_color) = Keyword::get("decoration:shadow:color") {
            if let OptionValue::Int(color_int) = shadow_color.value {
                // Convert integer color to rgba format
                style.shadow_color = self.int_color_to_rgba(color_int);
            }
        }

        debug!("🎨 Retrieved Hyprland style: {:?}", style);
        style
    }

    /// Parse Hyprland color string format
    fn parse_color_string(&self, color_str: &str) -> String {
        // Handle various Hyprland color formats
        if color_str.starts_with("rgba(") {
            color_str.to_string()
        } else if color_str.starts_with("rgb(") {
            color_str.to_string()
        } else {
            // Default fallback
            format!("rgba({})", color_str)
        }
    }

    /// Convert Hyprland integer color to RGBA format
    fn int_color_to_rgba(&self, color_int: i64) -> String {
        let color = color_int as u32;
        let r = (color >> 24) & 0xFF;
        let g = (color >> 16) & 0xFF;
        let b = (color >> 8) & 0xFF;
        let a = color & 0xFF;
        format!("rgba({}, {}, {}, {})", r, g, b, a)
    }

    /// Convert hex color (like "aa7c7674") to RGBA format
    fn hex_to_rgba(&self, hex: &str) -> String {
        if hex.len() == 8 {
            // Format: AARRGGBB (alpha, red, green, blue)
            if let Ok(color) = u32::from_str_radix(hex, 16) {
                let a = (color >> 24) & 0xFF;
                let r = (color >> 16) & 0xFF;
                let g = (color >> 8) & 0xFF;
                let b = color & 0xFF;
                return format!("rgba({}, {}, {}, {})", r, g, b, a);
            }
        }
        // Fallback to default
        format!("rgba({})", hex)
    }

    /// Extract hex color from hyprctl output like "custom type: aa7c7674 0deg"
    fn extract_hex_from_hyprctl_output(&self, output: &str) -> Option<String> {
        // Look for hex pattern after "custom type:" or in the output
        for line in output.lines() {
            if line.contains("custom type:") {
                // Extract hex from "custom type: aa7c7674 0deg"
                let parts: Vec<&str> = line.split_whitespace().collect();
                for part in parts {
                    if part.len() == 8 && part.chars().all(|c| c.is_ascii_hexdigit()) {
                        return Some(part.to_string());
                    }
                }
            }
        }
        None
    }

    /// Remove workspace switching prevention after animation
    async fn allow_workspace_switching(&self, window_address: &str) -> Result<()> {
        // Remove pin rule and restore normal border
        let unpin_cmd = format!(
            "hyprctl keyword windowrulev2 unset pin,address:{}",
            window_address
        );
        let restore_border_cmd = format!(
            "hyprctl keyword windowrulev2 'bordersize 1,address:{}'",
            window_address
        );

        tokio::process::Command::new("sh")
            .arg("-c")
            .arg(&unpin_cmd)
            .output()
            .await?;

        tokio::process::Command::new("sh")
            .arg("-c")
            .arg(&restore_border_cmd)
            .output()
            .await?;

        debug!(
            "📌 Unpinned window {} and restored borders after animation",
            window_address
        );
        Ok(())
    }

    /// Apply popup decorations directly to a window using hyprctl commands
    async fn apply_popup_decorations(&self, window_address: &str, style: &HyprlandStyle) {
        // First, try to remove ALL existing decoration rules for this window
        let remove_decorations_cmd = format!(
            "hyprctl keyword windowrulev2 unset,address:{}",
            window_address
        );

        tokio::process::Command::new("sh")
            .arg("-c")
            .arg(&remove_decorations_cmd)
            .output()
            .await
            .ok();

        debug!(
            "🗑️ Removed existing decoration rules for window {}",
            window_address
        );

        // Small delay to let the removal take effect
        sleep(Duration::from_millis(100)).await;

        // Now try direct property setting with more specific commands
        let decoration_commands = vec![
            // Now set the desired border size if > 0
            format!(
                "hyprctl setprop address:{} bordersize {}",
                window_address, style.border_size
            ),
            // Set border colors
            format!(
                "hyprctl setprop address:{} activebordercolor {}",
                window_address, style.active_border_color
            ),
            format!(
                "hyprctl setprop address:{} inactivebordercolor {}",
                window_address, style.inactive_border_color
            ),
        ];

        for (i, cmd) in decoration_commands.iter().enumerate() {
            if let Err(e) = tokio::process::Command::new("sh")
                .arg("-c")
                .arg(cmd)
                .output()
                .await
            {
                warn!("Failed to apply decoration command '{}': {}", cmd, e);
            } else {
                debug!("✅ Applied decoration {}: {}", i + 1, cmd);
            }

            // Longer delay between critical commands
            sleep(Duration::from_millis(50)).await;
        }

        // As a last resort, try using dispatch to focus and apply rules
        let focus_cmd = format!("hyprctl dispatch focuswindow address:{}", window_address);
        tokio::process::Command::new("sh")
            .arg("-c")
            .arg(&focus_cmd)
            .output()
            .await
            .ok();

        sleep(Duration::from_millis(50)).await;

        // One final attempt to set border size
        let final_border_cmd = format!(
            "hyprctl setprop address:{} bordersize {}",
            window_address, style.border_size
        );

        if let Err(e) = tokio::process::Command::new("sh")
            .arg("-c")
            .arg(&final_border_cmd)
            .output()
            .await
        {
            warn!("Final border setting failed: {}", e);
        } else {
            debug!("✅ Final border size set to {}", style.border_size);
        }

        debug!(
            "🎨 Completed all popup decoration attempts for window {}",
            window_address
        );
    }
}
