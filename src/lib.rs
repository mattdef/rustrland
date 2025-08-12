#![allow(dead_code, unused_imports)]

//! Rustrland - A Rust implementation of Pyprland for Hyprland
//! 
//! This crate provides a fast, reliable plugin system for Hyprland
//! with drop-in compatibility for Pyprland configurations.

pub mod config;
pub mod core;
pub mod ipc;
pub mod plugins;
pub mod animation;

// Re-export commonly used types
pub use config::Config;
pub use core::daemon::Daemon;