use rustrland::animation::{
    AnimationConfig, window_animator::WindowAnimator
};
use rustrland::ipc::HyprlandClient;
use std::time::Duration;
use tokio::time::sleep;
use tracing::info;

/// Foot Floating Test - Simple demo_basic_directional with foot terminal
/// Run with: cargo run --example foot_floating_test

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🎬 Foot Terminal OVERSHOOT Animation Test");
    println!("==========================================");
    println!("Testing demo_basic_directional with spectacular OVERSHOOT effect!");
    println!("Using ease-out-back that goes BEYOND 1.0 - true overshoot animation!\n");

    if std::env::var("HYPRLAND_INSTANCE_SIGNATURE").is_err() {
        println!("❌ HYPRLAND_INSTANCE_SIGNATURE not set");
        return Ok(());
    }
    
    // Demo 1: Basic directional animation (like Pyprland but smoother)
    demo_manual_directional().await?;
    sleep(Duration::from_secs(2)).await;

    println!("");
    println!("");

    // Demo 2: Basic directional animation (like Pyprland but smoother)
    demo_animator_directional().await?;
    sleep(Duration::from_secs(2)).await;

    Ok(())
}

async fn demo_manual_directional() -> anyhow::Result<()> {
    println!("Manual Directional Animation (Enhanced fromTop)");

    // Create Hyprland client and animation engine
    let client = HyprlandClient::new().await?;
    let mut animator = WindowAnimator::new();
    // Provide Hyprland client to animator
    animator.set_hyprland_client(std::sync::Arc::new(client.clone())).await;

    info!("✅ Connected to Hyprland - ready for real animations!");

    // Spawn foot terminal off-screen 
    println!("   🚀 Spawning foot terminal for demo...");
    animator.spawn_window_offscreen("foot", 560, -600, 800, 600).await?;
    //spawn_window_offscreen("foot", 560, -400).await?;
    
    // Wait for window to appear
    let foot_window = animator.wait_for_window_by_class("foot", 5000).await?;
    println!("🔍 Waiting for foot window to spawn...");

    if let Some(window) = foot_window {
        println!("✅ Found foot terminal: {}", window.class);
        println!("📍 Current position: ({}, {}) (should be off-screen)", window.at.0, window.at.1);
        println!("🔍 Floating: {}", window.floating);
        
        let address = window.address.to_string();
        
        // Calculate target position (center of screen for scratchpad effect)
        let target_x = 560; // Center X for 1920px screen
        let target_y = 540; // Center Y for 1080px screen
        
        // Current position should already be off-screen due to window rules
        let _current_x = window.at.0 as i32;
        let current_y = window.at.1 as i32;

        let start_y = current_y; // Use actual off-screen position

        // Step 3: Scratchpad-style animation (slide from top)
        println!("\n🎬 Starting scratchpad slide-in animation...");
        println!("   Animating from Y={} to Y={} (should appear from top of screen)", start_y, target_y);

        // Spectacular overshoot animation - slide with back easing that overshoots
        println!("🎯 Overshoot animation (800ms with ease-out-back that goes BEYOND target)...");
        
        use rustrland::animation::easing::EasingFunction;
        let easing = EasingFunction::from_name("ease-out-back");
        
        let total_frames = 48; // 800ms at 60fps = 48 frames  
        let distance = target_y - start_y; // Should be ~940px (from -600 to 340)
        
        for frame in 0..total_frames {
            let progress = frame as f32 / (total_frames - 1) as f32;
            let eased_progress = easing.apply(progress);
            let current_y = start_y + (distance as f32 * eased_progress) as i32;
            
            // Move window to current position
            client.move_window_pixel(&address, target_x, current_y).await.ok();
            
            if frame % 8 == 0 { // Print every 8th frame (~every 130ms)
                println!("Frame {:2}: progress={:.2}, eased={:.3}, Y={}px (overshoot!)", 
                        frame, progress, eased_progress, current_y);
            }
            
            sleep(Duration::from_millis(16)).await; // 60fps
        }

        // Ensure final position
        client.move_window_pixel(&address, target_x, target_y).await?;
        
        // Wait longer to appreciate the bounce effect result
        sleep(Duration::from_secs(2)).await;

        // Close window
        animator.close_window(&address).await?;
        
    } else {
        println!("❌ Could not find foot terminal window");
    }

    Ok(())
}

async fn demo_animator_directional() -> anyhow::Result<()> {
    println!("Animator Directional Animation (Enhanced fromTop)");

    // Create Hyprland client and animation engine
    let client = HyprlandClient::new().await?;
    let mut animator = WindowAnimator::new();
    // Provide Hyprland client to animator
    animator.set_hyprland_client(std::sync::Arc::new(client.clone())).await;

    info!("✅ Connected to Hyprland - ready for real animations!");

    let config = AnimationConfig {
        animation_type: "fromTop".to_string(),
        duration: 800, // 2 seconds to see all the different easings clearly
        easing: "ease-out-back".to_string(), // Will be overridden per property
        ..Default::default()
    };
    println!("Config > animation: {} - duration: {} - easing: {}", config.animation_type, config.duration, config.easing);

    //let windows = animator.show_animated_window("foot", (560, 540), (800, 600), config).await?;
    let windows = animator.show_window("foot", (560, 540), (800, 600), config).await?;
    println!("Animation started");
    if let Some(client) = windows {

        println!("Waiting 2 sec before close");
        sleep(Duration::from_secs(2)).await;

        animator.close_window(&client.address.to_string()).await?;

    }

    Ok(())
}