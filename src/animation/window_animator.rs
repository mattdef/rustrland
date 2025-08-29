use anyhow::{Error, Result};
use hyprland::data::Monitor;
use tracing_subscriber::fmt::format;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};
use tracing::{debug, info, warn};

use super::{properties::PropertyValue, AnimationConfig, AnimationEngine};
use crate::animation::easing::EasingFunction;
use crate::ipc::{self, HyprlandClient, MonitorInfo};

/// Manages animations for Hyprland windows
pub struct WindowAnimator {
    animation_engine: Arc<Mutex<AnimationEngine>>,
    hyprland_client: Arc<Mutex<Option<Arc<HyprlandClient>>>>,
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
            active_window_animations: HashMap::new(),
        }
    }

    /// Set the Hyprland client for window manipulation
    pub async fn set_hyprland_client(&self, client: Arc<HyprlandClient>) {
        let mut client_guard = self.hyprland_client.lock().await;
        *client_guard = Some(client);
    }

    /// Animate a window showing with specified animation
    pub async fn show_window_with_animation(
        &mut self,
        client: &HyprlandClient,
        monitor: &MonitorInfo,
        app: &str,
        target_position: (i32, i32), // Coordinates relative to current monitor
        target_size: (i32, i32),
        config: AnimationConfig,
    ) -> Result<Option<hyprland::data::Client>> {
        // Calculate starting position based on animation type
        let start_position = self
            .calculate_start_position(&monitor, target_position, target_size, &config)
            .await?;
        println!(
            "Start position from x:{} and y:{}",
            start_position.0, start_position.1
        );

        let app_class = format!("toggle_{app}");
        let app_command = format!("{} --app-id {}", app, app_class);

        self.spawn_window_offscreen(
            &client,
            &app_command,
            (start_position.0, start_position.1),
            (target_size.0, target_size.1),
        )
        .await?;

        let window = self.wait_for_window_by_class(&client, &app_class, 5000).await?;

        if let Some(window) = window {
            let address = window.address.to_string();

            // Add window rule to prevent automatic workspace switching during animation
            self.prevent_workspace_switching(&address).await?;

            // Set window to starting position instantly
            let start_offset_position = self.calculate_offscreen_position(start_position, &monitor);
            let target_offset_position = self.calculate_offscreen_position(target_position, &monitor);
            println!("🎬 Setting window to start position: ({}, {})", start_offset_position.0, start_offset_position.1);
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
                end_properties.insert(
                    "opacity".to_string(),
                    PropertyValue::Float(1.0),
                );
            }

            // Add scale for scale animations
            if config.animation_type.contains("scale") {
                end_properties
                    .insert("scale".to_string(), PropertyValue::Float(1.0));
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
            engine
                .start_animation(animation_id.clone(), config, initial_properties, end_properties)
                .await?;

            // Start window update loop
            self.start_window_animation_loop(client, address.to_string(), animation_id, animation_type, monitor.refresh_rate)
                .await?;

            return Ok(Some(window));
        }
        else {
            println!("Window not found");
        }

        Ok(None)
    }

    // Close a window
    pub async fn close_window(&mut self, client: &HyprlandClient, window_adress: &str) -> Result<()> {
        // Close window
        client.close_window(window_adress).await?;

        Ok(())
    }

    /// Animate a window hiding with specified animation
    pub async fn hide_window(
        &mut self,
        client: HyprlandClient,
        monitor: &MonitorInfo,
        window_address: &str,
        current_position: (i32, i32),
        current_size: (i32, i32),
        config: AnimationConfig,
    ) -> Result<()> {
        info!(
            "🎬 Starting hide animation for window {} with type '{}'",
            window_address, config.animation_type
        );

        // Calculate ending position based on animation type
        let end_position = self
            .calculate_end_position(&monitor, current_position, current_size, &config)
            .await?;

        // Create animation state
        let animation_id = format!("hide_{window_address}");
        let state = WindowAnimationState {
            window_address: window_address.to_string(),
            original_position: current_position,
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
        initial_properties.insert("x".to_string(), PropertyValue::Pixels(current_position.0));
        initial_properties.insert("y".to_string(), PropertyValue::Pixels(current_position.1));
        initial_properties.insert("width".to_string(), PropertyValue::Pixels(current_size.0));
        initial_properties.insert("height".to_string(), PropertyValue::Pixels(current_size.1));
        initial_properties.insert("opacity".to_string(), PropertyValue::Float(1.0));
        initial_properties.insert("scale".to_string(), PropertyValue::Float(1.0));
        let mut target_properties = HashMap::new();
        target_properties.insert("x".to_string(), PropertyValue::Pixels(current_position.0));
        target_properties.insert("y".to_string(), PropertyValue::Pixels(current_position.1));
        target_properties.insert("width".to_string(), PropertyValue::Pixels(current_size.0));
        target_properties.insert("height".to_string(), PropertyValue::Pixels(current_size.1));
        target_properties.insert("opacity".to_string(), PropertyValue::Float(0.0));
        target_properties.insert("scale".to_string(), PropertyValue::Float(0.0));

        // Store animation type before moving config
        let animation_type = config.animation_type.clone();

        // Start the animation
        let mut engine = self.animation_engine.lock().await;
        engine
            .start_animation(animation_id.clone(), config, initial_properties, target_properties)
            .await?;

        // Start window update loop
        self.start_window_animation_loop(&client, window_address.to_string(), animation_id, animation_type, monitor.refresh_rate)
            .await?;

        Ok(())
    }

    /// Calculate starting position for show animation
    async fn calculate_start_position(
        &self,
        monitor: &MonitorInfo,
        target_position: (i32, i32),
        target_size: (i32, i32),
        config: &AnimationConfig,
    ) -> Result<(i32, i32)> {
        let offset_pixels = self.parse_offset(&config.offset, (monitor.width, monitor.height))?;

        match config.animation_type.as_str() {
            "fromTop" => Ok((target_position.0 + monitor.x, -target_size.1 - offset_pixels)),
            "fromBottom" => Ok((target_position.0 + monitor.x, monitor.y - target_size.1 - offset_pixels)),
            "fromLeft" => {
                println!("Calcul FromLeft.Width : -{} - {}", target_size.0, offset_pixels);
                println!("Calcul FromLeft.Height : {} + {}", target_position.1, monitor.y);
                // Ok((monitor.x - target_size.0 - offset_pixels, target_position.1 + monitor.y))
                Ok((-target_size.0 - offset_pixels, target_position.1 + monitor.y))
            },
            "fromRight" => Ok((monitor.width as i32 + monitor.x + target_size.0 + offset_pixels, target_position.1 + monitor.y)),
            "fromTopLeft" => Ok((
                -target_size.0 - offset_pixels,
                -target_size.1 - offset_pixels,
            )),
            "fromTopRight" => Ok((
                monitor.width as i32 + offset_pixels,
                -target_size.1 - offset_pixels,
            )),
            "fromBottomLeft" => Ok((
                -target_size.0 - offset_pixels,
                monitor.height as i32 + offset_pixels,
            )),
            "fromBottomRight" => Ok((target_size.0 + offset_pixels, target_size.1 + offset_pixels)),
            "bounce" => Ok((target_position.0 + monitor.x, -target_size.1 - offset_pixels)), // Start completely above screen
            "fade" => Ok((target_position.0 + monitor.x, target_position.1 + monitor.y)), // No position change for fade
            "scale" => Ok((target_position.0 + monitor.x, target_position.1 + monitor.y)), // No position change for scale
            _ => Ok((target_position.0 + monitor.x, target_position.1 + monitor.y)),      // Default to target position
        }
    }

    /// Calculate ending position for hide animation
    async fn calculate_end_position(
        &self,
        monitor: &MonitorInfo,
        current_position: (i32, i32),
        current_size: (i32, i32),
        config: &AnimationConfig,
    ) -> Result<(i32, i32)> {
        let screen_size = (monitor.width, monitor.width);
        let offset_pixels = self.parse_offset(&config.offset, screen_size)?;

        match config.animation_type.as_str() {
            "toTop" | "fromTop" => Ok((current_position.0, -current_size.1 - offset_pixels)),
            "toBottom" | "fromBottom" => Ok((current_position.0, screen_size.1 as i32 + offset_pixels)),
            "toLeft" | "fromLeft" => Ok((-current_size.0 - offset_pixels, current_position.1)),
            "toRight" | "fromRight" => Ok((screen_size.0 as i32 + offset_pixels, current_position.1)),
            "fade" => Ok(current_position), // No position change for fade
            "scale" => Ok(current_position), // No position change for scale
            _ => Ok(current_position),      // Default to current position
        }
    }

    /// Calculate center position for a given monitor
    /// For now, returns screen center - in production this should query actual monitor geometry
    pub async fn calculate_monitor_center_position(
        &self,
        monitor: &MonitorInfo,
        window_size: (i32, i32),
    ) -> Result<(i32, i32)> {
        
        let screen_size = (monitor.width as i32, monitor.width as i32);

        // Calculate center position
        let center_x = (screen_size.0 - window_size.0) / 2;
        let center_y = (screen_size.1 - window_size.1) / 2;

        Ok((center_x, center_y))
    }

    /// Window animation update loop
    async fn start_window_animation_loop(
        &self,
        client: &HyprlandClient,
        window_address: String,
        animation_id: String,
        animation_type: String,
        refresh_rate: f32,
    ) -> Result<()> {
        let engine = Arc::clone(&self.animation_engine);
        let client = client.clone();
        let window_address_for_unpin = window_address.clone();

        tokio::spawn(async move {
            println!("🎯 Animation loop started for window {}", window_address);
            let mut frame_count = 0;
            let refresh_ms = (1000.0 / refresh_rate).round() as u64;

            loop {
                tokio::time::sleep(tokio::time::Duration::from_millis(refresh_ms)).await; // 60fps
                frame_count += 1;

                let properties = {
                    let mut engine_guard = engine.lock().await;
                    match engine_guard.get_current_properties(&animation_id) {
                        Some(props) => {
                            // Animation running smoothly
                            props
                        }
                        None => {
                            println!(
                                "✅ Animation {} completed naturally, ending loop",
                                animation_id
                            );
                            break; // Animation completed
                        }
                    }
                };

                // Apply properties to window with animation type context
                if let Err(e) = Self::apply_properties_to_window(
                    &client,
                    &window_address,
                    &properties,
                    &animation_type,
                )
                .await
                {
                    println!("Failed to apply animation properties: {}", e);
                }
            }

            println!(
                "✅ Animation loop completed for window {} after {} frames",
                window_address, frame_count
            );

            // Unpin window after animation completes
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
            println!(
                "📌 Unpinned window {} after animation",
                window_address_for_unpin
            );
        });

        Ok(())
    }

    /// Apply animation properties to window via Hyprland commands
    async fn apply_properties_to_window(
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

        debug!("Window moved to x:Pixels({}) and y:Pixels({})", x, y);

        // Move window using precise pixel positioning (required for animations)
        client.move_window_pixel(window_address, x, y).await?;

        // Resize window for scale animations
        client.resize_window(window_address, width, height).await?;

        // Handle opacity changes ONLY for fade/scale animations to prevent transparency issues
        if animation_type.contains("fade") || animation_type.contains("scale") {
            if let Some(PropertyValue::Float(opacity)) = properties.get("opacity") {
                client.set_window_opacity(window_address, *opacity).await?;
            }
        } else {
            // For non-fade animations, explicitly ensure opacity stays at 1.0 to prevent hover transparency
            client.set_window_opacity(window_address, 1.0).await.ok();
        }

        Ok(())
    }

    /// Set window properties directly
    async fn set_window_properties(
        &self,
        client: &HyprlandClient,
        window_address: &str,
        position: (i32, i32),
        size: (i32, i32),
        opacity: f32,
    ) -> Result<()> {
        client
            .move_window_pixel(window_address, position.0, position.1)
            .await?;
        client.resize_window(window_address, size.0, size.1).await?;

        if opacity < 1.0 {
            client.set_window_opacity(window_address, opacity).await?;
        }

        println!("   🎬 Window {} moved to ({}, {}) and resized to {}x{} with opacity {}", window_address, position.0, position.1, size.0, size.1, opacity);

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
        client: &HyprlandClient,
        app: &str,
        spawn: (i32, i32),
        window_size: (i32, i32),
    ) -> Result<()> {
        println!(
            "🚀 Spawning {} off-screen at ({}, {}) with size {}x{}",
            app, spawn.0, spawn.1, window_size.0, window_size.1
        );

        let exec_command = format!(
            "[float; move {} {}; size {} {}] {}",
            spawn.0, spawn.1, window_size.0, window_size.1, app
        );

        match client.spawn_app(exec_command.as_str()).await {
            Ok(_) => {
                println!("✅ Simple spawn worked");
                println!("Exec command: {}", exec_command);
            }
            Err(e) => {
                println!("❌ Even simple spawn failed: {}", e);
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
        client: &HyprlandClient,
        class: &str,
        timeout_ms: u64,
    ) -> Result<Option<hyprland::data::Client>> {
        println!(
            "🔍 Waiting for {} window (timeout: {}ms)",
            class, timeout_ms
        );

        let max_attempts = timeout_ms / 100;

        for attempt in 1..=max_attempts {
            let windows = client.get_windows().await?;

            if let Some(window) = windows
                .iter()
                .find(|w| w.class.to_lowercase().contains(&class.to_lowercase()))
                .cloned()
            {
                println!(
                    "✅ Found {} window after {}ms: {}",
                    class,
                    attempt * 100,
                    window.address
                );
                return Ok(Some(window));
            }

            // Progress logging every 500ms to avoid spam
            if attempt % 5 == 0 {
                println!(
                    "Still waiting for {} window... attempt {}/{}",
                    class, attempt, max_attempts
                );
            }

            sleep(Duration::from_millis(100)).await;
        }

        println!(
            "⏰ Timeout waiting for {} window after {}ms",
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
    pub fn calculate_offscreen_position(
        &self,
        position: (i32, i32),
        monitor: &MonitorInfo,
    ) -> (i32, i32) {

        (monitor.x + position.0, monitor.y + position.1)
    }

    /// Prevent automatic workspace switching during animation
    async fn prevent_workspace_switching(&self, window_address: &str) -> Result<()> {
        // Pin window to current workspace to prevent automatic movement
        let pin_cmd = format!(
            "hyprctl keyword windowrulev2 'pin,address:{}'",
            window_address
        );
        tokio::process::Command::new("sh")
            .arg("-c")
            .arg(&pin_cmd)
            .output()
            .await?;

        println!(
            "📌 Pinned window {} to prevent workspace switching",
            window_address
        );
        Ok(())
    }

    /// Remove workspace switching prevention after animation
    async fn allow_workspace_switching(&self, window_address: &str) -> Result<()> {
        let unpin_cmd = format!(
            "hyprctl keyword windowrulev2 unset pin,address:{}",
            window_address
        );
        tokio::process::Command::new("sh")
            .arg("-c")
            .arg(&unpin_cmd)
            .output()
            .await?;

        debug!("📌 Unpinned window {} after animation", window_address);
        Ok(())
    }
}
