use anyhow::Result;
use async_trait::async_trait;
use tracing::{info, debug, warn, error};
use std::sync::Arc;
use tokio::sync::Mutex;
use serde::{Deserialize, Serialize};
use std::process::Command;

use crate::plugins::Plugin;
use crate::ipc::{HyprlandEvent, HyprlandClient};

// Arc-optimized configuration type
pub type MagnifyConfigRef = Arc<MagnifyConfig>;

#[derive(Debug, Deserialize, Serialize)]
pub struct MagnifyConfig {
    /// Default zoom factor when toggling (default: 2.0)
    #[serde(default = "default_factor")]
    pub factor: f32,
    
    /// Animation duration in milliseconds (default: 300)
    #[serde(default = "default_duration")]
    pub duration: u32,
    
    /// Number of animation steps (default: 30)
    #[serde(default = "default_steps")]
    pub steps: u32,
    
    /// Enable smooth animations (default: true)
    #[serde(default = "default_true")]
    pub smooth_animation: bool,
    
    /// Use external hypr-zoom tool if available (default: true)
    #[serde(default = "default_true")]
    pub use_external_tool: bool,
    
    /// Minimum zoom level (default: 1.0)
    #[serde(default = "default_min_zoom")]
    pub min_zoom: f32,
    
    /// Maximum zoom level (default: 5.0) 
    #[serde(default = "default_max_zoom")]
    pub max_zoom: f32,
    
    /// Zoom increment for relative changes (default: 0.5)
    #[serde(default = "default_increment")]
    pub increment: f32,
    
    /// Easing function for animations (default: "ease-in-out")
    #[serde(default = "default_easing")]
    pub easing: String,
}

fn default_factor() -> f32 { 2.0 }
fn default_duration() -> u32 { 300 }
fn default_steps() -> u32 { 30 }
fn default_true() -> bool { true }
fn default_min_zoom() -> f32 { 1.0 }
fn default_max_zoom() -> f32 { 5.0 }
fn default_increment() -> f32 { 0.5 }
fn default_easing() -> String { "ease-in-out".to_string() }

impl Default for MagnifyConfig {
    fn default() -> Self {
        Self {
            factor: default_factor(),
            duration: default_duration(),
            steps: default_steps(),
            smooth_animation: default_true(),
            use_external_tool: default_true(),
            min_zoom: default_min_zoom(),
            max_zoom: default_max_zoom(),
            increment: default_increment(),
            easing: default_easing(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MagnifyState {
    pub current_zoom: f32,
    pub is_zoomed: bool,
    pub target_zoom: f32,
    pub animating: bool,
}

impl Default for MagnifyState {
    fn default() -> Self {
        Self {
            current_zoom: 1.0,
            is_zoomed: false,
            target_zoom: 1.0,
            animating: false,
        }
    }
}

pub struct MagnifyPlugin {
    config: MagnifyConfig,
    state: MagnifyState,
    hyprland_client: Arc<Mutex<Option<Arc<HyprlandClient>>>>,
    external_tool_available: bool,
}

impl MagnifyPlugin {
    pub fn new() -> Self {
        Self {
            config: MagnifyConfig::default(),
            state: MagnifyState::default(),
            hyprland_client: Arc::new(Mutex::new(None)),
            external_tool_available: false,
        }
    }
    
    /// Check if external zoom tools are available
    async fn check_external_tools(&mut self) -> bool {
        debug!("🔍 Checking for external zoom tools...");
        
        // Check for hypr-zoom
        let hypr_zoom_check = tokio::task::spawn_blocking(|| {
            Command::new("hypr-zoom")
                .arg("--help")
                .output()
                .map(|output| output.status.success())
                .unwrap_or(false)
        }).await.unwrap_or(false);
        
        if hypr_zoom_check {
            info!("✅ Found hypr-zoom tool");
            self.external_tool_available = true;
            return true;
        }
        
        // Check for direct hyprctl zoom support
        let hyprctl_check = tokio::task::spawn_blocking(|| {
            Command::new("hyprctl")
                .args(["getoption", "misc:cursor_zoom_factor"])
                .output()
                .map(|output| !output.stdout.is_empty() && !String::from_utf8_lossy(&output.stdout).contains("no such option"))
                .unwrap_or(false)
        }).await.unwrap_or(false);
        
        if hyprctl_check {
            info!("✅ Found hyprctl zoom support");
            self.external_tool_available = true;
            return true;
        }
        
        warn!("⚠️  No external zoom tools or hyprctl zoom support found");
        false
    }
    
    /// Set zoom level using available method
    async fn set_zoom_level(&mut self, target_zoom: f32) -> Result<()> {
        let clamped_zoom = target_zoom.clamp(self.config.min_zoom, self.config.max_zoom);
        
        if self.config.use_external_tool && self.external_tool_available {
            self.set_zoom_external(clamped_zoom).await
        } else {
            self.set_zoom_hyprctl(clamped_zoom).await
        }
    }
    
    /// Set zoom using external hypr-zoom tool
    async fn set_zoom_external(&mut self, target_zoom: f32) -> Result<()> {
        debug!("🔍 Setting zoom to {} using hypr-zoom", target_zoom);
        
        let duration = if self.config.smooth_animation {
            self.config.duration
        } else {
            1 // Near-instant
        };
        
        let steps = if self.config.smooth_animation { 
            self.config.steps 
        } else { 
            1 
        };
        
        let result = tokio::task::spawn_blocking(move || {
            Command::new("hypr-zoom")
                .arg(format!("-target={}", target_zoom))
                .arg(format!("-duration={}", duration))
                .arg(format!("-steps={}", steps))
                .output()
        }).await??;
        
        if result.status.success() {
            self.state.current_zoom = target_zoom;
            self.state.target_zoom = target_zoom;
            self.state.is_zoomed = target_zoom > 1.0;
            Ok(())
        } else {
            let error_msg = String::from_utf8_lossy(&result.stderr);
            Err(anyhow::anyhow!("hypr-zoom failed: {}", error_msg))
        }
    }
    
    /// Set zoom using hyprctl directly
    async fn set_zoom_hyprctl(&mut self, target_zoom: f32) -> Result<()> {
        debug!("🔍 Setting zoom to {} using hyprctl", target_zoom);
        
        if self.config.smooth_animation && (target_zoom - self.state.current_zoom).abs() > 0.1 {
            // Smooth animation by stepping through intermediate values
            let start_zoom = self.state.current_zoom;
            let steps = self.config.steps as f32;
            let step_duration = self.config.duration / self.config.steps;
            
            self.state.animating = true;
            
            for i in 1..=self.config.steps {
                let progress = i as f32 / steps;
                let eased_progress = self.apply_easing(progress);
                let current_zoom = start_zoom + (target_zoom - start_zoom) * eased_progress;
                
                let result = tokio::task::spawn_blocking(move || {
                    Command::new("hyprctl")
                        .args(["keyword", "misc:cursor_zoom_factor", &current_zoom.to_string()])
                        .output()
                }).await??;
                
                if !result.status.success() {
                    // Fallback: try without the misc: prefix
                    tokio::task::spawn_blocking(move || {
                        Command::new("hyprctl")
                            .args(["keyword", "cursor_zoom_factor", &current_zoom.to_string()])
                            .output()
                    }).await??;
                }
                
                self.state.current_zoom = current_zoom;
                tokio::time::sleep(tokio::time::Duration::from_millis(step_duration as u64)).await;
            }
            
            self.state.animating = false;
        } else {
            // Direct zoom change
            let result = tokio::task::spawn_blocking(move || {
                Command::new("hyprctl")
                    .args(["keyword", "misc:cursor_zoom_factor", &target_zoom.to_string()])
                    .output()
            }).await??;
            
            if !result.status.success() {
                // Fallback: try without the misc: prefix
                tokio::task::spawn_blocking(move || {
                    Command::new("hyprctl")
                        .args(["keyword", "cursor_zoom_factor", &target_zoom.to_string()])
                        .output()
                }).await??;
            }
            
            self.state.current_zoom = target_zoom;
        }
        
        self.state.target_zoom = target_zoom;
        self.state.is_zoomed = target_zoom > 1.0;
        Ok(())
    }
    
    /// Apply easing function to animation progress
    fn apply_easing(&self, progress: f32) -> f32 {
        match self.config.easing.as_str() {
            "linear" => progress,
            "ease-in" => progress * progress,
            "ease-out" => 1.0 - (1.0 - progress).powi(2),
            "ease-in-out" => {
                if progress < 0.5 {
                    2.0 * progress * progress
                } else {
                    1.0 - 2.0 * (1.0 - progress).powi(2)
                }
            }
            _ => progress, // Default to linear
        }
    }
    
    /// Toggle zoom (zoom in if not zoomed, zoom out if zoomed)
    async fn toggle_zoom(&mut self) -> Result<String> {
        let target_zoom = if self.state.is_zoomed {
            1.0 // Zoom out
        } else {
            self.config.factor // Zoom in
        };
        
        info!("🔍 Toggling zoom from {:.1}x to {:.1}x", self.state.current_zoom, target_zoom);
        
        self.set_zoom_level(target_zoom).await?;
        
        let action = if target_zoom > 1.0 { "in" } else { "out" };
        Ok(format!("Zoomed {} to {:.1}x", action, target_zoom))
    }
    
    /// Set absolute zoom level
    async fn set_zoom(&mut self, zoom: f32) -> Result<String> {
        info!("🔍 Setting absolute zoom to {:.1}x", zoom);
        
        if zoom < self.config.min_zoom || zoom > self.config.max_zoom {
            return Err(anyhow::anyhow!(
                "Zoom level {:.1}x out of range ({:.1}x - {:.1}x)",
                zoom, self.config.min_zoom, self.config.max_zoom
            ));
        }
        
        self.set_zoom_level(zoom).await?;
        
        Ok(format!("Zoom set to {:.1}x", zoom))
    }
    
    /// Change zoom relatively (+ or -)
    async fn change_zoom(&mut self, delta: f32) -> Result<String> {
        let target_zoom = self.state.current_zoom + delta;
        
        info!("🔍 Changing zoom by {:.1}x (from {:.1}x to {:.1}x)", 
            delta, self.state.current_zoom, target_zoom);
        
        if target_zoom < self.config.min_zoom {
            return Err(anyhow::anyhow!(
                "Zoom level would be too low: {:.1}x (minimum: {:.1}x)",
                target_zoom, self.config.min_zoom
            ));
        }
        
        if target_zoom > self.config.max_zoom {
            return Err(anyhow::anyhow!(
                "Zoom level would be too high: {:.1}x (maximum: {:.1}x)",
                target_zoom, self.config.max_zoom
            ));
        }
        
        self.set_zoom_level(target_zoom).await?;
        
        let direction = if delta > 0.0 { "in" } else { "out" };
        Ok(format!("Zoomed {} by {:.1}x to {:.1}x", direction, delta.abs(), target_zoom))
    }
    
    /// Zoom in by increment
    async fn zoom_in(&mut self) -> Result<String> {
        self.change_zoom(self.config.increment).await
    }
    
    /// Zoom out by increment  
    async fn zoom_out(&mut self) -> Result<String> {
        self.change_zoom(-self.config.increment).await
    }
    
    /// Reset zoom to 1.0x
    async fn reset_zoom(&mut self) -> Result<String> {
        info!("🔍 Resetting zoom to 1.0x");
        
        self.set_zoom_level(1.0).await?;
        
        Ok("Zoom reset to 1.0x".to_string())
    }
    
    /// Get current zoom status
    async fn get_status(&self) -> Result<String> {
        let status = if self.state.is_zoomed { "Active" } else { "Inactive" };
        let tool_status = if self.external_tool_available { "External tool" } else { "hyprctl" };
        let animation_status = if self.state.animating { " (animating)" } else { "" };
        
        Ok(format!(
            "Magnify: {} | Current: {:.1}x | Method: {}{}\nRange: {:.1}x - {:.1}x | Increment: {:.1}x",
            status,
            self.state.current_zoom,
            tool_status,
            animation_status,
            self.config.min_zoom,
            self.config.max_zoom,
            self.config.increment
        ))
    }
}

#[async_trait]
impl Plugin for MagnifyPlugin {
    fn name(&self) -> &str {
        "magnify"
    }
    
    async fn init(&mut self, config: &toml::Value) -> Result<()> {
        info!("🔍 Initializing magnify plugin");
        
        if let Some(magnify_config) = config.get("magnify") {
            match magnify_config.clone().try_into() {
                Ok(config) => self.config = config,
                Err(e) => return Err(anyhow::anyhow!("Invalid magnify configuration: {}", e)),
            }
        }
        
        debug!("Magnify config: {:?}", self.config);
        
        // Check for available zoom tools
        self.external_tool_available = self.check_external_tools().await;
        
        if !self.external_tool_available && self.config.use_external_tool {
            warn!("⚠️  External zoom tools not available, will attempt hyprctl direct commands");
        }
        
        info!("✅ Magnify plugin initialized (factor: {:.1}x, external_tool: {})", 
            self.config.factor, self.external_tool_available);
        
        Ok(())
    }
    
    async fn handle_event(&mut self, event: &HyprlandEvent) -> Result<()> {
        // Handle events that might affect zoom state
        match event {
            HyprlandEvent::WorkspaceChanged { .. } => {
                // Could reset zoom on workspace change if configured
                debug!("Workspace changed during magnify");
            }
            _ => {}
        }
        
        Ok(())
    }
    
    async fn handle_command(&mut self, command: &str, args: &[&str]) -> Result<String> {
        debug!("🔍 Magnify command: {} {:?}", command, args);
        
        match command {
            "toggle" => self.toggle_zoom().await,
            "set" => {
                if let Some(zoom_str) = args.first() {
                    let zoom: f32 = zoom_str.parse()
                        .map_err(|_| anyhow::anyhow!("Invalid zoom level: {}", zoom_str))?;
                    self.set_zoom(zoom).await
                } else {
                    Err(anyhow::anyhow!("Set command requires zoom level"))
                }
            }
            "change" => {
                if let Some(delta_str) = args.first() {
                    let delta: f32 = delta_str.parse()
                        .map_err(|_| anyhow::anyhow!("Invalid zoom delta: {}", delta_str))?;
                    self.change_zoom(delta).await
                } else {
                    Err(anyhow::anyhow!("Change command requires delta value"))
                }
            }
            "in" => self.zoom_in().await,
            "out" => self.zoom_out().await,
            "reset" => self.reset_zoom().await,
            "status" => self.get_status().await,
            _ => Ok(format!("Unknown magnify command: {}", command)),
        }
    }
}