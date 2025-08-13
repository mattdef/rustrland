use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use tracing::{debug, info, warn};

pub mod easing;
pub mod properties;
pub mod timeline;
pub mod window_animator;

// Re-export commonly used types
pub use easing::EasingFunction;
pub use properties::{AnimationProperty, Color, PropertyValue, Transform};
pub use timeline::{AnimationDirection, Keyframe, Timeline, TimelineBuilder};

// Types are available via pub use statements above
pub use window_animator::WindowAnimator;

/// Advanced animation configuration with physics support
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AnimationConfig {
    /// Animation type (slide, fade, scale, bounce, elastic, physics)
    pub animation_type: String,

    /// Direction for directional animations (top, bottom, left, right, topLeft, etc.)
    pub direction: Option<String>,

    /// Duration in milliseconds
    #[serde(default = "default_duration")]
    pub duration: u32,

    /// Easing function (linear, ease, easeIn, easeOut, easeInOut, bounce, elastic, spring)
    #[serde(default = "default_easing")]
    pub easing: String,

    /// Delay before animation starts (ms)
    #[serde(default)]
    pub delay: u32,

    /// Offset distance for slide animations (pixels or percentage)
    #[serde(default = "default_offset")]
    pub offset: String,

    /// Scale factor for scale animations
    #[serde(default = "default_scale")]
    pub scale_from: f32,

    /// Opacity for fade animations
    #[serde(default)]
    pub opacity_from: f32,

    /// Spring physics parameters
    pub spring: Option<SpringConfig>,

    /// Multiple animation properties to animate simultaneously
    pub properties: Option<Vec<AnimationPropertyConfig>>,

    /// Animation sequence/chain
    pub sequence: Option<Vec<AnimationConfig>>,

    /// Performance settings
    #[serde(default)]
    pub target_fps: u32,

    /// Whether to use hardware acceleration hints
    #[serde(default = "default_true")]
    pub hardware_accelerated: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SpringConfig {
    /// Spring stiffness (higher = snappier)
    #[serde(default = "default_spring_stiffness")]
    pub stiffness: f32,

    /// Spring damping (higher = less bouncy)
    #[serde(default = "default_spring_damping")]
    pub damping: f32,

    /// Initial velocity
    #[serde(default)]
    pub initial_velocity: f32,

    /// Mass of the animated object
    #[serde(default = "default_spring_mass")]
    pub mass: f32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AnimationPropertyConfig {
    pub property: String, // x, y, width, height, opacity, scale, rotation
    pub from: PropertyValue,
    pub to: PropertyValue,
    pub easing: Option<String>,
}

/// Runtime animation state
#[derive(Debug)]
pub struct AnimationState {
    pub config: AnimationConfig,
    pub start_time: Instant,
    pub current_progress: f32,
    pub is_running: bool,
    pub is_paused: bool,
    pub timeline: Timeline,
    pub properties: HashMap<String, PropertyValue>,
    pub target_properties: HashMap<String, PropertyValue>,
}

/// Advanced animation engine
pub struct AnimationEngine {
    active_animations: HashMap<String, AnimationState>,
    performance_monitor: PerformanceMonitor,
}

#[derive(Debug)]
struct PerformanceMonitor {
    frame_times: Vec<Duration>,
    target_frame_time: Duration,
    adaptive_quality: bool,
}

impl Default for AnimationEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl AnimationEngine {
    pub fn new() -> Self {
        Self {
            active_animations: HashMap::new(),
            performance_monitor: PerformanceMonitor {
                frame_times: Vec::with_capacity(60),
                target_frame_time: Duration::from_millis(16), // 60fps
                adaptive_quality: true,
            },
        }
    }

    /// Start a new animation
    pub async fn start_animation(
        &mut self,
        id: String,
        config: AnimationConfig,
        initial_properties: HashMap<String, PropertyValue>,
    ) -> Result<()> {
        info!(
            "🎬 Starting animation '{}' with type '{}'",
            id, config.animation_type
        );

        let target_properties = self.calculate_target_properties(&config, &initial_properties)?;

        let state = AnimationState {
            config: config.clone(),
            start_time: Instant::now() + Duration::from_millis(config.delay as u64),
            current_progress: 0.0,
            is_running: true,
            is_paused: false,
            timeline: Timeline::new(Duration::from_millis(config.duration as u64)),
            properties: initial_properties,
            target_properties,
        };

        self.active_animations.insert(id.clone(), state);

        // Start animation loop for this animation
        self.run_animation_loop(id).await?;

        Ok(())
    }

    /// Calculate target properties based on animation type and direction
    fn calculate_target_properties(
        &self,
        config: &AnimationConfig,
        initial: &HashMap<String, PropertyValue>,
    ) -> Result<HashMap<String, PropertyValue>> {
        let mut targets = initial.clone();

        match config.animation_type.as_str() {
            "fromTop" => {
                let offset_pixels = self.parse_offset(&config.offset, "height")?;
                if let Some(y) = targets.get("y") {
                    targets.insert(
                        "y".to_string(),
                        PropertyValue::Pixels(y.as_pixels() - offset_pixels as i32),
                    );
                }
            }
            "fromBottom" => {
                let offset_pixels = self.parse_offset(&config.offset, "height")?;
                if let Some(y) = targets.get("y") {
                    targets.insert(
                        "y".to_string(),
                        PropertyValue::Pixels(y.as_pixels() + offset_pixels as i32),
                    );
                }
            }
            "fromLeft" => {
                let offset_pixels = self.parse_offset(&config.offset, "width")?;
                if let Some(x) = targets.get("x") {
                    targets.insert(
                        "x".to_string(),
                        PropertyValue::Pixels(x.as_pixels() - offset_pixels as i32),
                    );
                }
            }
            "fromRight" => {
                let offset_pixels = self.parse_offset(&config.offset, "width")?;
                if let Some(x) = targets.get("x") {
                    targets.insert(
                        "x".to_string(),
                        PropertyValue::Pixels(x.as_pixels() + offset_pixels as i32),
                    );
                }
            }
            "fade" => {
                targets.insert(
                    "opacity".to_string(),
                    PropertyValue::Float(config.opacity_from),
                );
            }
            "scale" => {
                targets.insert("scale".to_string(), PropertyValue::Float(config.scale_from));
            }
            "bounce" | "elastic" | "spring" => {
                // Physics-based animations will be handled by spring dynamics
                if let Some(spring) = &config.spring {
                    self.setup_spring_animation(&mut targets, spring)?;
                }
            }
            _ => {
                warn!("Unknown animation type: {}", config.animation_type);
            }
        }

        // Handle custom property animations
        if let Some(properties) = &config.properties {
            for prop_config in properties {
                targets.insert(prop_config.property.clone(), prop_config.to.clone());
            }
        }

        Ok(targets)
    }

    /// Main animation loop with 60fps target and performance monitoring
    async fn run_animation_loop(&mut self, animation_id: String) -> Result<()> {
        let target_frame_duration = Duration::from_millis(16); // 60fps

        loop {
            let frame_start = Instant::now();

            let should_continue = {
                let (easing_name, progress, should_continue) = {
                    let animation = match self.active_animations.get_mut(&animation_id) {
                        Some(anim) => anim,
                        None => break, // Animation was stopped
                    };

                    if animation.is_paused || !animation.is_running {
                        sleep(Duration::from_millis(16)).await;
                        continue;
                    }

                    // Check if animation should start (handle delay)
                    if Instant::now() < animation.start_time {
                        sleep(Duration::from_millis(1)).await;
                        continue;
                    }

                    // Update timeline progress
                    let elapsed = animation.start_time.elapsed();
                    let progress = animation.timeline.get_progress(elapsed);
                    animation.current_progress = progress;

                    (animation.config.easing.clone(), progress, progress < 1.0)
                };

                // Apply easing function
                let eased_progress = self.apply_easing(&easing_name, progress);

                // Calculate current property values
                Self::interpolate_properties_for_animation(
                    &mut self.active_animations,
                    &animation_id,
                    eased_progress,
                )?;

                should_continue
            };

            if !should_continue {
                self.complete_animation(&animation_id).await?;
                break;
            }

            // Performance monitoring and adaptive frame rate
            let frame_time = frame_start.elapsed();
            self.performance_monitor.frame_times.push(frame_time);

            if self.performance_monitor.frame_times.len() > 60 {
                self.performance_monitor.frame_times.remove(0);
            }

            // Adaptive sleep to maintain target FPS
            if frame_time < target_frame_duration {
                sleep(target_frame_duration - frame_time).await;
            }
        }

        Ok(())
    }

    /// Interpolate between current and target properties
    fn interpolate_properties_for_animation(
        animations: &mut HashMap<String, AnimationState>,
        animation_id: &str,
        progress: f32,
    ) -> Result<()> {
        if let Some(animation) = animations.get_mut(animation_id) {
            for (property_name, target_value) in &animation.target_properties.clone() {
                if let Some(current_value) = animation.properties.get(property_name) {
                    let interpolated = current_value.interpolate(target_value, progress);
                    animation
                        .properties
                        .insert(property_name.clone(), interpolated);
                }
            }
        }
        Ok(())
    }

    /// Apply sophisticated easing functions
    fn apply_easing(&self, easing_name: &str, progress: f32) -> f32 {
        EasingFunction::from_name(easing_name).apply(progress)
    }

    /// Complete an animation and trigger callbacks
    async fn complete_animation(&mut self, animation_id: &str) -> Result<()> {
        if let Some(mut animation) = self.active_animations.remove(animation_id) {
            animation.is_running = false;
            animation.current_progress = 1.0;

            info!("✅ Animation '{}' completed", animation_id);

            // Handle animation sequences
            if let Some(sequence) = &animation.config.sequence {
                if !sequence.is_empty() {
                    debug!("🔄 Starting next animation in sequence");
                    // Start next animation in sequence
                    // Implementation would continue the sequence here
                }
            }
        }

        Ok(())
    }

    /// Parse offset string (pixels or percentage)
    fn parse_offset(&self, offset: &str, dimension: &str) -> Result<f32> {
        if offset.ends_with('%') {
            let percent = offset.trim_end_matches('%').parse::<f32>()?;
            // Get screen dimension (simplified - would use actual screen resolution)
            let screen_size = match dimension {
                "width" => 1920.0,
                "height" => 1080.0,
                _ => 1920.0,
            };
            Ok(screen_size * percent / 100.0)
        } else if offset.ends_with("px") {
            Ok(offset.trim_end_matches("px").parse::<f32>()?)
        } else {
            Ok(offset.parse::<f32>()?)
        }
    }

    /// Setup spring physics for bounce/elastic animations
    fn setup_spring_animation(
        &self,
        _targets: &mut HashMap<String, PropertyValue>,
        _spring: &SpringConfig,
    ) -> Result<()> {
        // Spring physics implementation would go here
        // This would calculate spring dynamics for realistic physics-based animation
        Ok(())
    }

    /// Stop an animation
    pub fn stop_animation(&mut self, animation_id: &str) -> Result<()> {
        if let Some(animation) = self.active_animations.get_mut(animation_id) {
            animation.is_running = false;
            info!("⏹️  Stopped animation '{}'", animation_id);
        }
        Ok(())
    }

    /// Pause/resume animation
    pub fn pause_animation(&mut self, animation_id: &str, paused: bool) -> Result<()> {
        if let Some(animation) = self.active_animations.get_mut(animation_id) {
            animation.is_paused = paused;
            let action = if paused {
                "⏸️  Paused"
            } else {
                "▶️  Resumed"
            };
            info!("{} animation '{}'", action, animation_id);
        }
        Ok(())
    }

    /// Get current animation properties for applying to windows
    pub fn get_current_properties(
        &self,
        animation_id: &str,
    ) -> Option<&HashMap<String, PropertyValue>> {
        self.active_animations
            .get(animation_id)
            .map(|anim| &anim.properties)
    }

    /// Get performance statistics
    pub fn get_performance_stats(&self) -> PerformanceStats {
        let avg_frame_time = if !self.performance_monitor.frame_times.is_empty() {
            self.performance_monitor
                .frame_times
                .iter()
                .sum::<Duration>()
                / self.performance_monitor.frame_times.len() as u32
        } else {
            Duration::from_millis(16)
        };

        PerformanceStats {
            average_frame_time: avg_frame_time,
            current_fps: 1000.0 / avg_frame_time.as_millis() as f32,
            active_animations: self.active_animations.len(),
            target_fps: 60.0,
        }
    }
}

#[derive(Debug)]
pub struct PerformanceStats {
    pub average_frame_time: Duration,
    pub current_fps: f32,
    pub active_animations: usize,
    pub target_fps: f32,
}

// Default values for configuration
fn default_duration() -> u32 {
    300
}
fn default_easing() -> String {
    "easeInOut".to_string()
}
fn default_offset() -> String {
    "100%".to_string()
}
fn default_scale() -> f32 {
    0.0
}
fn default_true() -> bool {
    true
}
fn default_spring_stiffness() -> f32 {
    300.0
}
fn default_spring_damping() -> f32 {
    30.0
}
fn default_spring_mass() -> f32 {
    1.0
}

impl Default for AnimationConfig {
    fn default() -> Self {
        Self {
            animation_type: "fromTop".to_string(),
            direction: None,
            duration: default_duration(),
            easing: default_easing(),
            delay: 0,
            offset: default_offset(),
            scale_from: default_scale(),
            opacity_from: 0.0,
            spring: None,
            properties: None,
            sequence: None,
            target_fps: 60,
            hardware_accelerated: default_true(),
        }
    }
}
