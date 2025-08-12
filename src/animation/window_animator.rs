use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

use super::{properties::PropertyValue, AnimationConfig, AnimationEngine};
use crate::ipc::HyprlandClient;

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
    pub async fn show_window(
        &mut self,
        window_address: &str,
        target_position: (i32, i32),
        target_size: (i32, i32),
        config: AnimationConfig,
    ) -> Result<()> {
        info!(
            "🎬 Starting show animation for window {} with type '{}'",
            window_address, config.animation_type
        );

        // Calculate starting position based on animation type
        let start_position = self
            .calculate_start_position(target_position, target_size, &config)
            .await?;

        // Set window to starting position instantly
        self.set_window_properties(window_address, start_position, target_size, 0.0)
            .await?;

        // Create animation state
        let animation_id = format!("show_{}", window_address);
        let state = WindowAnimationState {
            window_address: window_address.to_string(),
            original_position: start_position,
            original_size: target_size,
            target_position,
            target_size,
            animation_id: animation_id.clone(),
            is_showing: true,
        };

        self.active_window_animations
            .insert(window_address.to_string(), state);

        // Prepare animation properties
        let mut initial_properties = HashMap::new();
        initial_properties.insert("x".to_string(), PropertyValue::Pixels(start_position.0));
        initial_properties.insert("y".to_string(), PropertyValue::Pixels(start_position.1));
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
            initial_properties.insert("scale".to_string(), PropertyValue::Float(config.scale_from));
        }

        // Start the animation
        let mut engine = self.animation_engine.lock().await;
        engine
            .start_animation(animation_id.clone(), config, initial_properties)
            .await?;

        // Start window update loop
        self.start_window_animation_loop(window_address.to_string(), animation_id)
            .await?;

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
        info!(
            "🎬 Starting hide animation for window {} with type '{}'",
            window_address, config.animation_type
        );

        // Calculate ending position based on animation type
        let end_position = self
            .calculate_end_position(current_position, current_size, &config)
            .await?;

        // Create animation state
        let animation_id = format!("hide_{}", window_address);
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

        // Start the animation
        let mut engine = self.animation_engine.lock().await;
        engine
            .start_animation(animation_id.clone(), config, initial_properties)
            .await?;

        // Start window update loop
        self.start_window_animation_loop(window_address.to_string(), animation_id)
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
        let screen_size = self.get_screen_size().await?;
        let offset_pixels = self.parse_offset(&config.offset, screen_size)?;

        match config.animation_type.as_str() {
            "fromTop" => Ok((target_position.0, -target_size.1 - offset_pixels)),
            "fromBottom" => Ok((target_position.0, screen_size.1 + offset_pixels)),
            "fromLeft" => Ok((-target_size.0 - offset_pixels, target_position.1)),
            "fromRight" => Ok((screen_size.0 + offset_pixels, target_position.1)),
            "fromTopLeft" => Ok((
                -target_size.0 - offset_pixels,
                -target_size.1 - offset_pixels,
            )),
            "fromTopRight" => Ok((
                screen_size.0 + offset_pixels,
                -target_size.1 - offset_pixels,
            )),
            "fromBottomLeft" => Ok((
                -target_size.0 - offset_pixels,
                screen_size.1 + offset_pixels,
            )),
            "fromBottomRight" => Ok((screen_size.0 + offset_pixels, screen_size.1 + offset_pixels)),
            "fade" => Ok(target_position), // No position change for fade
            "scale" => Ok(target_position), // No position change for scale
            _ => Ok(target_position),      // Default to target position
        }
    }

    /// Calculate ending position for hide animation
    async fn calculate_end_position(
        &self,
        current_position: (i32, i32),
        current_size: (i32, i32),
        config: &AnimationConfig,
    ) -> Result<(i32, i32)> {
        let screen_size = self.get_screen_size().await?;
        let offset_pixels = self.parse_offset(&config.offset, screen_size)?;

        match config.animation_type.as_str() {
            "toTop" | "fromTop" => Ok((current_position.0, -current_size.1 - offset_pixels)),
            "toBottom" | "fromBottom" => Ok((current_position.0, screen_size.1 + offset_pixels)),
            "toLeft" | "fromLeft" => Ok((-current_size.0 - offset_pixels, current_position.1)),
            "toRight" | "fromRight" => Ok((screen_size.0 + offset_pixels, current_position.1)),
            "fade" => Ok(current_position), // No position change for fade
            "scale" => Ok(current_position), // No position change for scale
            _ => Ok(current_position),      // Default to current position
        }
    }

    /// Window animation update loop
    async fn start_window_animation_loop(
        &self,
        window_address: String,
        animation_id: String,
    ) -> Result<()> {
        let engine = Arc::clone(&self.animation_engine);
        let hyprland_client = Arc::clone(&self.hyprland_client);

        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_millis(16)).await; // 60fps

                let properties = {
                    let engine_guard = engine.lock().await;
                    match engine_guard.get_current_properties(&animation_id) {
                        Some(props) => props.clone(),
                        None => break, // Animation completed
                    }
                };

                // Apply properties to window
                if let Err(e) =
                    Self::apply_properties_to_window(&hyprland_client, &window_address, &properties)
                        .await
                {
                    warn!("Failed to apply animation properties: {}", e);
                }
            }

            debug!("Animation loop completed for window {}", window_address);
        });

        Ok(())
    }

    /// Apply animation properties to window via Hyprland commands
    async fn apply_properties_to_window(
        hyprland_client: &Arc<Mutex<Option<Arc<HyprlandClient>>>>,
        window_address: &str,
        properties: &HashMap<String, PropertyValue>,
    ) -> Result<()> {
        let client_guard = hyprland_client.lock().await;
        let client = match client_guard.as_ref() {
            Some(client) => client,
            None => return Ok(()), // No client available
        };

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

        // Move and resize window
        client.move_window(window_address, x, y).await?;
        client.resize_window(window_address, width, height).await?;

        // Handle opacity changes
        if let Some(PropertyValue::Float(opacity)) = properties.get("opacity") {
            client.set_window_opacity(window_address, *opacity).await?;
        }

        Ok(())
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
            .move_window(window_address, position.0, position.1)
            .await?;
        client.resize_window(window_address, size.0, size.1).await?;

        if opacity < 1.0 {
            client.set_window_opacity(window_address, opacity).await?;
        }

        Ok(())
    }

    /// Get screen dimensions
    async fn get_screen_size(&self) -> Result<(i32, i32)> {
        // TODO: Get actual screen size from Hyprland
        // For now, return common resolution
        Ok((1920, 1080))
    }

    /// Parse offset string to pixels
    fn parse_offset(&self, offset: &str, screen_size: (i32, i32)) -> Result<i32> {
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
}
