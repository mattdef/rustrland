//! GUI module for Rustrland
//! 
//! This module provides GUI components using egui framework that can be shared
//! across different plugins. It centralizes all GUI-related functionality.

pub mod wallpaper_carousel;
pub mod standalone_carousel;

// Re-export common GUI types for easy access
pub use wallpaper_carousel::{WallpaperCarouselGUI, CarouselConfig, CarouselSelection};
pub use standalone_carousel::StandaloneCarouselApp;

// Common GUI utilities and configurations that can be shared
use egui::{Color32, Vec2};

/// Common GUI theme configuration
#[derive(Debug, Clone)]
pub struct GuiTheme {
    pub background_color: Color32,
    pub primary_color: Color32,
    pub secondary_color: Color32,
    pub text_color: Color32,
    pub accent_color: Color32,
}

impl Default for GuiTheme {
    fn default() -> Self {
        Self {
            background_color: Color32::from_gray(20),
            primary_color: Color32::from_rgb(70, 130, 255),
            secondary_color: Color32::from_rgb(100, 100, 100),
            text_color: Color32::WHITE,
            accent_color: Color32::from_rgb(255, 165, 0),
        }
    }
}

/// Common GUI window configuration
#[derive(Debug, Clone)]
pub struct WindowConfig {
    pub title: String,
    pub size: Vec2,
    pub resizable: bool,
    pub theme: GuiTheme,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: "Rustrland".to_string(),
            size: Vec2::new(800.0, 600.0),
            resizable: true,
            theme: GuiTheme::default(),
        }
    }
}

/// Trait for GUI applications that can be launched by plugins
pub trait GuiApp: eframe::App + Send + 'static {
    fn window_config(&self) -> WindowConfig;
}