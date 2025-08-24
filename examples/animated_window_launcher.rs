use rustrland::animation::{
    AnimationConfig, window_animator::WindowAnimator, SpringConfig
};
use rustrland::ipc::HyprlandClient;
use std::time::Duration;
use tokio::time::sleep;
use tracing::info;
use anyhow::Result;

/// Advanced Animated Window Launcher - Production Ready Example
/// Run with: cargo run --example animated_window_launcher

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 Advanced Animated Window Launcher");
    println!("=====================================");
    println!("Testing configurable window animations with monitor support!\n");

    if std::env::var("HYPRLAND_INSTANCE_SIGNATURE").is_err() {
        println!("❌ HYPRLAND_INSTANCE_SIGNATURE not set");
        return Ok(());
    }

    // Create Hyprland client and animation engine
    let client = HyprlandClient::new().await?;
    let mut animator = WindowAnimator::new();
    animator.set_hyprland_client(std::sync::Arc::new(client)).await;
    
    info!("✅ Connected to Hyprland - ready for animated window launching!");

    println!("🎬 Demo 1: Foot terminal with slide animation from top");
    let slide_config = AnimationConfig {
        animation_type: "fromTop".to_string(),
        duration: 800,
        easing: "ease-out-back".to_string(),
        offset: "100px".to_string(),
        ..Default::default()
    };
    
    show_animated_window("foot", "DP-1", (800, 600), slide_config, &mut animator).await?;
    sleep(Duration::from_secs(2)).await;
    println!("");

    println!("🎬 Demo 2: Firefox with fade + scale animation");
    let fade_scale_config = AnimationConfig {
        animation_type: "fade".to_string(),
        duration: 800,
        easing: "ease-out-cubic".to_string(),
        scale_from: 0.5,
        opacity_from: 0.0,
        ..Default::default()
    };
    
    show_animated_window("firefox", "DP-1", (1200, 800), fade_scale_config, &mut animator).await?;
    sleep(Duration::from_secs(2)).await;
    println!("");

    println!("🎬 Demo 3: Spring physics animation with bounce");
    let spring_config = AnimationConfig {
        animation_type: "spring".to_string(),
        duration: 1200,
        easing: "spring".to_string(),
        spring: Some(SpringConfig {
            stiffness: 200.0,
            damping: 15.0,
            initial_velocity: 0.0,
            mass: 1.0,
        }),
        ..Default::default()
    };
    
    show_animated_window("thunar", "DP-1", (900, 650), spring_config, &mut animator).await?;
    sleep(Duration::from_secs(3)).await;

    Ok(())
}

/// Show an animated floating window with configurable animation
/// 
/// # Parameters
/// - `app`: Application command (e.g. "foot", "firefox", "thunar")
/// - `monitor`: Target monitor (e.g. "DP-1", "HDMI-A-1") 
/// - `size`: Window size (width, height)
/// - `config`: Animation configuration with type, duration, easing, etc.
/// - `animator`: WindowAnimator instance with Hyprland client
/// 
/// # Animation Types Supported
/// - `fromTop`, `fromBottom`, `fromLeft`, `fromRight`: Directional slides
/// - `fromTopLeft`, `fromTopRight`, `fromBottomLeft`, `fromBottomRight`: Diagonal slides  
/// - `fade`: Opacity-based animation
/// - `scale`: Size-based animation
/// - `spring`: Physics-based spring animation
/// 
/// # Configuration Examples
/// 
/// ## Basic slide from top:
/// ```rust
/// AnimationConfig {
///     animation_type: "fromTop".to_string(),
///     duration: 400,
///     easing: "ease-out".to_string(),
///     offset: "100px".to_string(),
///     ..Default::default()
/// }
/// ```
/// 
/// ## Fade with scale:
/// ```rust
/// AnimationConfig {
///     animation_type: "fade".to_string(),
///     duration: 600,
///     easing: "ease-out-cubic".to_string(), 
///     scale_from: 0.8,
///     opacity_from: 0.0,
///     ..Default::default()
/// }
/// ```
/// 
/// ## Spring physics:
/// ```rust
/// AnimationConfig {
///     animation_type: "spring".to_string(),
///     duration: 1000,
///     easing: "spring".to_string(),
///     spring: Some(SpringConfig {
///         stiffness: 300.0,
///         damping: 25.0,
///         initial_velocity: 0.0,
///         mass: 1.0,
///     }),
///     ..Default::default()
/// }
/// ```
pub async fn show_animated_window(
    app: &str,
    monitor: &str, 
    size: (i32, i32),
    config: AnimationConfig,
    animator: &mut WindowAnimator,
) -> Result<Option<hyprland::data::Client>> {
    println!("🚀 Launching {} on {} with {} animation", app, monitor, config.animation_type);
    println!("   📐 Size: {}x{}", size.0, size.1);
    println!("   ⏱️  Duration: {}ms", config.duration);
    println!("   📈 Easing: {}", config.easing);
    
    // Get monitor info and calculate center position
    let target_position = calculate_monitor_center_position(monitor, size).await?;
    println!("   📍 Target position: ({}, {})", target_position.0, target_position.1);
    
    // Launch the animated window
    let window = animator.show_window(app, target_position, size, config).await?;
    
    if let Some(ref window) = window {
        println!("✅ Window launched successfully: {}", window.address);
        println!("   🔍 Class: {}, Title: {}", window.class, window.title);
        println!("   📍 Position: ({}, {})", window.at.0, window.at.1);
        println!("   📐 Size: {}x{}", window.size.0, window.size.1);
        println!("   🎯 Floating: {}", window.floating);
        
        // Wait longer to see animation, then close window
        println!("⏳ Waiting 10 seconds to observe the animation...");
        sleep(Duration::from_secs(10)).await;
        animator.close_window(&window.address.to_string()).await?;
        println!("🔴 Window closed");
    } else {
        println!("❌ Failed to launch window");
    }
    
    Ok(window)
}

/// Calculate center position for a given monitor
/// For now, returns screen center - in production this should query actual monitor geometry
async fn calculate_monitor_center_position(
    monitor: &str, 
    window_size: (i32, i32)
) -> Result<(i32, i32)> {
    // TODO: Query actual monitor geometry from Hyprland
    // For now, assume standard monitor setup
    let (screen_width, screen_height) = match monitor {
        "DP-3" | "HDMI-A-1" | "eDP-1" => (1920, 1080),
        "DP-1" | "HDMI-A-2" => (2560, 1440), // Assume higher res secondary monitor
        _ => (1920, 1080), // Default fallback
    };
    
    // Calculate center position
    let center_x = (screen_width - window_size.0) / 2;
    let center_y = (screen_height - window_size.1) / 2;
    
    Ok((center_x, center_y))
}

/// Create common animation configurations for easy reuse
pub mod animation_presets {
    use super::{AnimationConfig, SpringConfig};
    
    /// Quick slide from top with smooth easing
    pub fn slide_from_top() -> AnimationConfig {
        AnimationConfig {
            animation_type: "fromTop".to_string(),
            duration: 400,
            easing: "ease-out-cubic".to_string(),
            offset: "100px".to_string(),
            ..Default::default()
        }
    }
    
    /// Smooth fade in with subtle scale
    pub fn fade_in_scale() -> AnimationConfig {
        AnimationConfig {
            animation_type: "fade".to_string(),
            duration: 500,
            easing: "ease-out".to_string(),
            scale_from: 0.9,
            opacity_from: 0.0,
            ..Default::default()
        }
    }
    
    /// Bouncy spring animation
    pub fn spring_bounce() -> AnimationConfig {
        AnimationConfig {
            animation_type: "spring".to_string(),
            duration: 800,
            easing: "spring".to_string(),
            spring: Some(SpringConfig {
                stiffness: 250.0,
                damping: 20.0,
                initial_velocity: 0.0,
                mass: 1.0,
            }),
            ..Default::default()
        }
    }
    
    /// Elegant slide from left
    pub fn slide_from_left() -> AnimationConfig {
        AnimationConfig {
            animation_type: "fromLeft".to_string(),
            duration: 450,
            easing: "ease-out-back".to_string(),
            offset: "200px".to_string(),
            ..Default::default()
        }
    }
}