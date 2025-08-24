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
            "🎬 Starting animation '{}' with type '{}', duration: {}ms",
            id, config.animation_type, config.duration
        );

        // Initialize properties correctly for multi-property animations
        let (final_initial_properties, target_properties) = if config.properties.is_some() {
            // For multi-property animations, use 'from' values as initial properties
            let mut multi_initial = HashMap::new();
            let mut multi_targets = HashMap::new();
            
            if let Some(properties) = &config.properties {
                for prop_config in properties {
                    multi_initial.insert(prop_config.property.clone(), prop_config.from.clone());
                    multi_targets.insert(prop_config.property.clone(), prop_config.to.clone());
                }
            }
            
            info!("Multi-property animation with {} properties", multi_initial.len());
            (multi_initial, multi_targets)
        } else {
            // Traditional single-property animation - LOGIC CORRECTED
            // initial_properties = where window should START (off-screen)
            // targets = where window should END (final position)
            let start_properties = self.calculate_start_properties(&config, &initial_properties)?;
            (start_properties, initial_properties) // SWAPPED: start->final
        };

        let state = AnimationState {
            config: config.clone(),
            start_time: Instant::now() + Duration::from_millis(config.delay as u64),
            current_progress: 0.0,
            is_running: true,
            is_paused: false,
            timeline: Timeline::new(Duration::from_millis(config.duration as u64)),
            properties: final_initial_properties,
            target_properties,
        };

        self.active_animations.insert(id.clone(), state);

        // Don't start animation loop here - let start_window_animation_loop handle it
        info!("✅ Animation '{}' initialized and ready", id);

        Ok(())
    }

    /// Calculate start properties based on animation type and direction
    fn calculate_start_properties(
        &self,
        config: &AnimationConfig,
        final_properties: &HashMap<String, PropertyValue>,
    ) -> Result<HashMap<String, PropertyValue>> {
        let mut start_props = final_properties.clone();

        match config.animation_type.as_str() {
            "fromTop" => {
                let offset_pixels = self.parse_offset(&config.offset, "height")?;
                if let Some(y) = start_props.get("y") {
                    start_props.insert(
                        "y".to_string(),
                        PropertyValue::Pixels(y.as_pixels() - offset_pixels as i32),
                    );
                }
            }
            "fromBottom" => {
                let offset_pixels = self.parse_offset(&config.offset, "height")?;
                if let Some(y) = start_props.get("y") {
                    start_props.insert(
                        "y".to_string(),
                        PropertyValue::Pixels(y.as_pixels() + offset_pixels as i32),
                    );
                }
            }
            "fromLeft" => {
                let offset_pixels = self.parse_offset(&config.offset, "width")?;
                if let Some(x) = start_props.get("x") {
                    start_props.insert(
                        "x".to_string(),
                        PropertyValue::Pixels(x.as_pixels() - offset_pixels as i32),
                    );
                }
            }
            "fromRight" => {
                let offset_pixels = self.parse_offset(&config.offset, "width")?;
                if let Some(x) = start_props.get("x") {
                    start_props.insert(
                        "x".to_string(),
                        PropertyValue::Pixels(x.as_pixels() + offset_pixels as i32),
                    );
                }
            }
            "fade" => {
                start_props.insert(
                    "opacity".to_string(),
                    PropertyValue::Float(config.opacity_from),
                );
            }
            "scale" => {
                start_props.insert("scale".to_string(), PropertyValue::Float(config.scale_from));
            }
            "bounce" | "elastic" | "spring" => {
                // Physics-based animations will be handled by spring dynamics
                if let Some(spring) = &config.spring {
                    self.setup_spring_animation(&mut start_props, spring)?;
                }
            }
            _ => {
                warn!("Unknown animation type: {}", config.animation_type);
            }
        }

        // Handle custom property animations - targets remain the same
        if let Some(properties) = &config.properties {
            for prop_config in properties {
                start_props.insert(prop_config.property.clone(), prop_config.to.clone());
            }
        }

        Ok(start_props)
    }

    /// Optimized 60fps animation loop with precise frame timing
    async fn run_animation_loop(&mut self, animation_id: String) -> Result<()> {
        info!("🎬 Starting 60fps animation loop for '{}'", animation_id);
        
        // Get animation duration to calculate total frames
        let (duration_ms, easing_name) = {
            let animation = match self.active_animations.get(&animation_id) {
                Some(anim) => anim,
                None => return Ok(()),
            };
            (animation.config.duration, animation.config.easing.clone())
        };
        
        let total_frames = ((duration_ms as f32 / 16.67).round() as u32).max(1); // 60fps = 16.67ms per frame
        // Note: easing is now handled per-property in multi-property animations
        
        // Precise 60fps loop with frame-perfect timing
        for frame in 0..total_frames {
            let frame_start = Instant::now();
            
            // Calculate progress (0.0 to 1.0)
            let progress = if total_frames == 1 {
                1.0 // Handle single frame case
            } else {
                frame as f32 / (total_frames - 1) as f32
            };
            
            // Update animation properties with per-property easing support
            Self::interpolate_properties_with_individual_easing(
                &mut self.active_animations,
                &animation_id,
                progress, // Raw progress, not eased yet
                &easing_name,
            )?;
            
            // Check if animation was stopped
            if !self.active_animations.get(&animation_id)
                .map(|anim| anim.is_running && !anim.is_paused)
                .unwrap_or(false)
            {
                debug!("Animation '{}' was stopped during loop", animation_id);
                break;
            }
            
            // Performance monitoring
            let frame_time = frame_start.elapsed();
            self.performance_monitor.frame_times.push(frame_time);
            if self.performance_monitor.frame_times.len() > 60 {
                self.performance_monitor.frame_times.remove(0);
            }
            
            // Frame timing debug (every 10th frame to avoid spam)
            if frame % 10 == 0 {
                debug!("Animation '{}' frame {}/{}: progress={:.3}, frame_time={:.1}ms", 
                       animation_id, frame + 1, total_frames, progress, frame_time.as_millis());
            }
            
            // Maintain 60fps (16.67ms per frame)
            let target_frame_time = Duration::from_millis(16);
            if frame_time < target_frame_time {
                sleep(target_frame_time - frame_time).await;
            }
        }
        
        // Complete the animation
        self.complete_animation(&animation_id).await?;
        info!("✅ Animation '{}' completed after {} frames", animation_id, total_frames);
        
        Ok(())
    }

    /// Advanced interpolation with per-property easing support
    fn interpolate_properties_with_individual_easing(
        animations: &mut HashMap<String, AnimationState>,
        animation_id: &str,
        raw_progress: f32,
        default_easing: &str,
    ) -> Result<()> {
        if let Some(animation) = animations.get_mut(animation_id) {
            // Check if we have custom property configurations
            if let Some(properties_config) = &animation.config.properties {
                // Multi-property animation with individual easing
                for prop_config in properties_config {
                    // Use property-specific easing or default
                    let easing_name = prop_config.easing.as_deref().unwrap_or(default_easing);
                    let easing = EasingFunction::from_name(easing_name);
                    let eased_progress = easing.apply(raw_progress);
                    
                    // Interpolate from configured 'from' to configured 'to' value
                    let interpolated = prop_config.from.interpolate(&prop_config.to, eased_progress);
                    
                    debug!("Property '{}': easing={}, progress={:.3}, eased={:.3}, value={:?}",
                           prop_config.property, easing_name, raw_progress, eased_progress, interpolated);
                    
                    animation.properties.insert(prop_config.property.clone(), interpolated);
                }
            } else {
                // Single-property animation (legacy behavior)
                let easing = EasingFunction::from_name(default_easing);
                let eased_progress = easing.apply(raw_progress);
                
                for (property_name, target_value) in &animation.target_properties.clone() {
                    if let Some(current_value) = animation.properties.get(property_name) {
                        let interpolated = current_value.interpolate(target_value, eased_progress);
                        animation.properties.insert(property_name.clone(), interpolated);
                    }
                }
            }
        }
        Ok(())
    }

    /// Legacy interpolation method (kept for compatibility)
    fn interpolate_properties_for_animation(
        animations: &mut HashMap<String, AnimationState>,
        animation_id: &str,
        progress: f32,
    ) -> Result<()> {
        Self::interpolate_properties_with_individual_easing(animations, animation_id, progress, "linear")
    }

    /// Apply sophisticated easing functions with validation
    fn apply_easing(&self, easing_name: &str, progress: f32) -> f32 {
        // Validate easing function exists, fallback to linear if not
        let validated_easing = self.validate_easing_function(easing_name);
        EasingFunction::from_name(&validated_easing).apply(progress)
    }

    /// Validate easing function exists, return valid name or fallback
    fn validate_easing_function(&self, easing_name: &str) -> String {
        // List of all supported easing functions
        let valid_easings = [
            "linear",
            "ease", "ease-in", "ease-out", "ease-in-out",
            "ease-in-sine", "ease-out-sine", "ease-in-out-sine",
            "ease-in-quad", "ease-out-quad", "ease-in-out-quad",
            "ease-in-cubic", "ease-out-cubic", "ease-in-out-cubic",
            "ease-in-quart", "ease-out-quart", "ease-in-out-quart",
            "ease-in-quint", "ease-out-quint", "ease-in-out-quint",
            "ease-in-expo", "ease-out-expo", "ease-in-out-expo",
            "ease-in-circ", "ease-out-circ", "ease-in-out-circ",
            "ease-in-back", "ease-out-back", "ease-in-out-back",
            "ease-in-elastic", "ease-out-elastic", "ease-in-out-elastic",
            "ease-in-bounce", "ease-out-bounce", "ease-in-out-bounce",
            "spring",
        ];

        // Check if the requested easing function is valid
        if valid_easings.contains(&easing_name) {
            easing_name.to_string()
        } else {
            // Check for custom cubic-bezier format
            if easing_name.starts_with("cubic-bezier(") && easing_name.ends_with(')') {
                easing_name.to_string() // Assume custom bezier is valid
            } else {
                warn!("⚠️  Unknown easing function '{}', falling back to 'linear'", easing_name);
                "linear".to_string()
            }
        }
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
        &mut self,
        animation_id: &str,
    ) -> Option<HashMap<String, PropertyValue>> {
        let (raw_progress, duration_completed, easing_name) = {
            if let Some(animation) = self.active_animations.get(animation_id) {
                // Calculate elapsed time
                let elapsed = animation.start_time.elapsed();
                let duration = Duration::from_millis(animation.config.duration as u64);
                
                // Calculate progress (0.0 to 1.0)
                let raw_progress = if duration.is_zero() {
                    1.0
                } else {
                    (elapsed.as_millis() as f32 / duration.as_millis() as f32).min(1.0)
                };
                
                (raw_progress, raw_progress >= 1.0, animation.config.easing.clone())
            } else {
                return None;
            }
        };
        
        if duration_completed {
            // Animation completed
            if let Some(animation) = self.active_animations.get_mut(animation_id) {
                animation.current_progress = 1.0;
                animation.is_running = false;
            }
            return None; // Signal completion
        }
        
        // Update animation properties in real-time
        Self::interpolate_properties_with_individual_easing(
            &mut self.active_animations,
            animation_id,
            raw_progress,
            &easing_name,
        ).ok()?;
        
        // Return cloned properties
        self.active_animations
            .get(animation_id)
            .map(|anim| anim.properties.clone())
    }

    /// Validate if an easing function is supported
    pub fn is_easing_supported(&self, easing_name: &str) -> bool {
        let validated = self.validate_easing_function(easing_name);
        validated == easing_name || easing_name.starts_with("cubic-bezier(")
    }

    /// Get list of all supported easing functions
    pub fn get_supported_easings(&self) -> Vec<&'static str> {
        vec![
            "linear",
            "ease", "ease-in", "ease-out", "ease-in-out",
            "ease-in-sine", "ease-out-sine", "ease-in-out-sine",
            "ease-in-quad", "ease-out-quad", "ease-in-out-quad",
            "ease-in-cubic", "ease-out-cubic", "ease-in-out-cubic",
            "ease-in-quart", "ease-out-quart", "ease-in-out-quart",
            "ease-in-quint", "ease-out-quint", "ease-in-out-quint",
            "ease-in-expo", "ease-out-expo", "ease-in-out-expo",
            "ease-in-circ", "ease-out-circ", "ease-in-out-circ",
            "ease-in-back", "ease-out-back", "ease-in-out-back",
            "ease-in-elastic", "ease-out-elastic", "ease-in-out-elastic",
            "ease-in-bounce", "ease-out-bounce", "ease-in-out-bounce",
            "spring",
        ]
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
    "ease-out-cubic".to_string() // Better default for scratchpads
}
fn default_offset() -> String {
    "200px".to_string() // Plus réaliste pour les scratchpads
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
