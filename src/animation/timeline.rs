use std::time::{Duration, Instant};

/// Precise animation timeline with keyframe support
#[derive(Debug, Clone)]
pub struct Timeline {
    duration: Duration,
    keyframes: Vec<Keyframe>,
    loop_count: Option<u32>,
    current_loop: u32,
    direction: AnimationDirection,
}

#[derive(Debug, Clone)]
pub struct Keyframe {
    pub time: f32,     // 0.0 to 1.0
    pub value: f32,    // Animation value at this time
    pub easing: Option<String>, // Optional easing for this segment
}

#[derive(Debug, Clone)]
pub enum AnimationDirection {
    Normal,
    Reverse,
    Alternate,
    AlternateReverse,
}

impl Timeline {
    /// Create a new timeline with duration
    pub fn new(duration: Duration) -> Self {
        Self {
            duration,
            keyframes: vec![
                Keyframe { time: 0.0, value: 0.0, easing: None },
                Keyframe { time: 1.0, value: 1.0, easing: None },
            ],
            loop_count: Some(1),
            current_loop: 0,
            direction: AnimationDirection::Normal,
        }
    }
    
    /// Create timeline with custom keyframes
    pub fn with_keyframes(duration: Duration, keyframes: Vec<Keyframe>) -> Self {
        let mut timeline = Self::new(duration);
        timeline.keyframes = keyframes;
        timeline.keyframes.sort_by(|a, b| {
            a.time.partial_cmp(&b.time).unwrap_or(std::cmp::Ordering::Equal)
        });
        timeline
    }
    
    /// Set loop count (None = infinite)
    pub fn set_loop_count(&mut self, count: Option<u32>) {
        self.loop_count = count;
    }
    
    /// Set animation direction
    pub fn set_direction(&mut self, direction: AnimationDirection) {
        self.direction = direction;
    }
    
    /// Get progress (0.0 to 1.0) at given elapsed time
    pub fn get_progress(&mut self, elapsed: Duration) -> f32 {
        if self.duration.as_millis() == 0 {
            return 1.0;
        }
        
        let total_progress = elapsed.as_millis() as f32 / self.duration.as_millis() as f32;
        
        // Handle looping
        if let Some(loop_count) = self.loop_count {
            if total_progress >= loop_count as f32 {
                return 1.0; // Animation complete
            }
        }
        
        // Calculate current loop progress
        let loop_progress = total_progress.fract();
        self.current_loop = total_progress.floor() as u32;
        
        // Apply direction
        let directed_progress = match self.direction {
            AnimationDirection::Normal => loop_progress,
            AnimationDirection::Reverse => 1.0 - loop_progress,
            AnimationDirection::Alternate => {
                if self.current_loop % 2 == 0 {
                    loop_progress
                } else {
                    1.0 - loop_progress
                }
            }
            AnimationDirection::AlternateReverse => {
                if self.current_loop % 2 == 0 {
                    1.0 - loop_progress
                } else {
                    loop_progress
                }
            }
        };
        
        directed_progress.clamp(0.0, 1.0)
    }
    
    /// Get interpolated value at specific progress using keyframes
    pub fn get_value_at_progress(&self, progress: f32) -> f32 {
        if self.keyframes.is_empty() {
            return progress;
        }
        
        if progress <= self.keyframes[0].time {
            return self.keyframes[0].value;
        }
        
        if let Some(last_keyframe) = self.keyframes.last() {
            if progress >= last_keyframe.time {
                return last_keyframe.value;
            }
        }
        
        // Find the two keyframes to interpolate between
        for i in 0..self.keyframes.len() - 1 {
            let current = &self.keyframes[i];
            let next = &self.keyframes[i + 1];
            
            if progress >= current.time && progress <= next.time {
                // Calculate interpolation factor within this segment
                let segment_progress = (progress - current.time) / (next.time - current.time);
                
                // Apply easing to this segment if specified
                let eased_progress = if let Some(ref easing_name) = next.easing {
                    use crate::animation::easing::EasingFunction;
                    EasingFunction::from_name(easing_name).apply(segment_progress)
                } else {
                    segment_progress
                };
                
                // Linear interpolation between keyframe values
                return current.value + (next.value - current.value) * eased_progress;
            }
        }
        
        progress // Fallback
    }
    
    /// Add a keyframe to the timeline
    pub fn add_keyframe(&mut self, time: f32, value: f32, easing: Option<String>) {
        let keyframe = Keyframe {
            time: time.clamp(0.0, 1.0),
            value,
            easing,
        };
        
        self.keyframes.push(keyframe);
        self.keyframes.sort_by(|a, b| {
            a.time.partial_cmp(&b.time).unwrap_or(std::cmp::Ordering::Equal)
        });
    }
    
    /// Remove keyframe at specific time
    pub fn remove_keyframe_at(&mut self, time: f32) {
        self.keyframes.retain(|k| (k.time - time).abs() > f32::EPSILON);
    }
    
    /// Get duration
    pub fn duration(&self) -> Duration {
        self.duration
    }
    
    /// Set duration
    pub fn set_duration(&mut self, duration: Duration) {
        self.duration = duration;
    }
    
    /// Check if animation is complete
    pub fn is_complete(&self, elapsed: Duration) -> bool {
        if let Some(loop_count) = self.loop_count {
            elapsed >= self.duration * loop_count
        } else {
            false // Infinite loop never completes
        }
    }
    
    /// Get current loop number
    pub fn current_loop(&self) -> u32 {
        self.current_loop
    }
    
    /// Reset timeline to beginning
    pub fn reset(&mut self) {
        self.current_loop = 0;
    }
    
    /// Create a timeline for fade animation
    pub fn fade_timeline(duration: Duration, from_opacity: f32, to_opacity: f32) -> Self {
        let keyframes = vec![
            Keyframe { time: 0.0, value: from_opacity, easing: None },
            Keyframe { time: 1.0, value: to_opacity, easing: Some("ease-out".to_string()) },
        ];
        Self::with_keyframes(duration, keyframes)
    }
    
    /// Create a timeline for scale animation with overshoot
    pub fn scale_timeline(duration: Duration, from_scale: f32, to_scale: f32) -> Self {
        let overshoot = to_scale * 1.1; // 10% overshoot for bounce effect
        let keyframes = vec![
            Keyframe { time: 0.0, value: from_scale, easing: None },
            Keyframe { time: 0.7, value: overshoot, easing: Some("ease-out".to_string()) },
            Keyframe { time: 1.0, value: to_scale, easing: Some("ease-in".to_string()) },
        ];
        Self::with_keyframes(duration, keyframes)
    }
    
    /// Create a timeline for slide animation with ease-in-out
    pub fn slide_timeline(duration: Duration, from_pos: f32, to_pos: f32) -> Self {
        let keyframes = vec![
            Keyframe { time: 0.0, value: from_pos, easing: None },
            Keyframe { time: 0.2, value: from_pos + (to_pos - from_pos) * 0.1, easing: Some("ease-in".to_string()) },
            Keyframe { time: 0.8, value: from_pos + (to_pos - from_pos) * 0.9, easing: Some("ease-out".to_string()) },
            Keyframe { time: 1.0, value: to_pos, easing: Some("ease-out".to_string()) },
        ];
        Self::with_keyframes(duration, keyframes)
    }
    
    /// Create a complex bounce timeline with multiple bounces
    pub fn bounce_timeline(duration: Duration) -> Self {
        let keyframes = vec![
            Keyframe { time: 0.0, value: 0.0, easing: None },
            Keyframe { time: 0.2, value: 1.0, easing: Some("ease-out".to_string()) },
            Keyframe { time: 0.4, value: 0.7, easing: Some("ease-in".to_string()) },
            Keyframe { time: 0.6, value: 0.9, easing: Some("ease-out".to_string()) },
            Keyframe { time: 0.8, value: 0.8, easing: Some("ease-in".to_string()) },
            Keyframe { time: 1.0, value: 1.0, easing: Some("ease-out".to_string()) },
        ];
        Self::with_keyframes(duration, keyframes)
    }
    
    /// Create elastic timeline with rubber band effect
    pub fn elastic_timeline(duration: Duration) -> Self {
        let keyframes = vec![
            Keyframe { time: 0.0, value: 0.0, easing: None },
            Keyframe { time: 0.6, value: 1.3, easing: Some("ease-out".to_string()) },
            Keyframe { time: 0.75, value: 0.9, easing: Some("ease-in".to_string()) },
            Keyframe { time: 0.85, value: 1.1, easing: Some("ease-out".to_string()) },
            Keyframe { time: 0.95, value: 0.95, easing: Some("ease-in".to_string()) },
            Keyframe { time: 1.0, value: 1.0, easing: Some("ease-out".to_string()) },
        ];
        Self::with_keyframes(duration, keyframes)
    }
}

/// Timeline builder for fluent API
pub struct TimelineBuilder {
    timeline: Timeline,
}

impl TimelineBuilder {
    pub fn new(duration: Duration) -> Self {
        Self {
            timeline: Timeline::new(duration),
        }
    }
    
    pub fn keyframe(mut self, time: f32, value: f32, easing: Option<&str>) -> Self {
        self.timeline.add_keyframe(time, value, easing.map(|s| s.to_string()));
        self
    }
    
    pub fn loop_count(mut self, count: Option<u32>) -> Self {
        self.timeline.set_loop_count(count);
        self
    }
    
    pub fn direction(mut self, direction: AnimationDirection) -> Self {
        self.timeline.set_direction(direction);
        self
    }
    
    pub fn build(self) -> Timeline {
        self.timeline
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_basic_timeline() {
        let mut timeline = Timeline::new(Duration::from_millis(1000));
        
        assert_eq!(timeline.get_progress(Duration::from_millis(0)), 0.0);
        assert_eq!(timeline.get_progress(Duration::from_millis(500)), 0.5);
        assert_eq!(timeline.get_progress(Duration::from_millis(1000)), 1.0);
    }
    
    #[test]
    fn test_keyframe_interpolation() {
        let keyframes = vec![
            Keyframe { time: 0.0, value: 0.0, easing: None },
            Keyframe { time: 0.5, value: 1.0, easing: None },
            Keyframe { time: 1.0, value: 0.5, easing: None },
        ];
        
        let timeline = Timeline::with_keyframes(Duration::from_millis(1000), keyframes);
        
        assert_eq!(timeline.get_value_at_progress(0.0), 0.0);
        assert_eq!(timeline.get_value_at_progress(0.25), 0.5);
        assert_eq!(timeline.get_value_at_progress(0.5), 1.0);
        assert_eq!(timeline.get_value_at_progress(0.75), 0.75);
        assert_eq!(timeline.get_value_at_progress(1.0), 0.5);
    }
    
    #[test]
    fn test_loop_animation() {
        let mut timeline = Timeline::new(Duration::from_millis(1000));
        timeline.set_loop_count(Some(2));
        
        assert_eq!(timeline.get_progress(Duration::from_millis(500)), 0.5);
        assert_eq!(timeline.get_progress(Duration::from_millis(1000)), 0.0); // Start of second loop
        assert_eq!(timeline.get_progress(Duration::from_millis(1500)), 0.5);
        assert_eq!(timeline.get_progress(Duration::from_millis(2000)), 1.0);
    }
    
    #[test]
    fn test_alternate_direction() {
        let mut timeline = Timeline::new(Duration::from_millis(1000));
        timeline.set_loop_count(Some(2));
        timeline.set_direction(AnimationDirection::Alternate);
        
        assert_eq!(timeline.get_progress(Duration::from_millis(500)), 0.5);  // Forward
        assert_eq!(timeline.get_progress(Duration::from_millis(1500)), 0.5); // Reverse
    }
    
    #[test]
    fn test_timeline_builder() {
        let timeline = TimelineBuilder::new(Duration::from_millis(1000))
            .keyframe(0.0, 0.0, None)
            .keyframe(0.5, 1.0, Some("ease-out"))
            .keyframe(1.0, 0.5, Some("ease-in"))
            .loop_count(Some(3))
            .direction(AnimationDirection::Alternate)
            .build();
        
        assert_eq!(timeline.keyframes.len(), 5); // 2 default + 3 added = 5 keyframes
        assert_eq!(timeline.loop_count, Some(3));
    }
}