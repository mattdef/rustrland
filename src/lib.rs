#![allow(dead_code, unused_imports)]

//! Rustrland - A Rust implementation of Pyprland for Hyprland
//!
//! This crate provides a fast, reliable plugin system for Hyprland
//! with drop-in compatibility for Pyprland configurations.

pub mod animation;
pub mod config;
pub mod core;
pub mod ipc;
pub mod plugins;

// Re-export commonly used types
pub use config::Config;
pub use core::daemon::Daemon;

// Re-export animation types for plugin usage
pub use animation::{AnimationConfig, AnimationEngine, AnimationPropertyConfig, SpringConfig};
pub use animation::{EasingFunction, PropertyValue, Color, Transform};
pub use animation::{Timeline, TimelineBuilder, AnimationDirection, Keyframe};
