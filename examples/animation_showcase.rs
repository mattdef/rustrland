#![allow(warnings)]
use rustrland::animation::{
    easing::EasingFunction,
    properties::PropertyValue,
    timeline::{AnimationDirection, TimelineBuilder},
    window_animator::WindowAnimator,
    AnimationConfig, AnimationEngine, AnimationPropertyConfig,
};
use rustrland::ipc::HyprlandClient;
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, warn};

/// Animation Showcase - Demonstrates Rustrland's advanced animation system with REAL windows
/// Run with: cargo run --example animation_showcase

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    println!("🎬 Rustrland Animation System REAL Showcase");
    println!("============================================");
    println!("Demonstrating advanced animations with REAL WINDOWS that move!");
    println!("This surpasses Pyprland with actual visual demonstrations\n");

    // Check Hyprland environment
    if std::env::var("HYPRLAND_INSTANCE_SIGNATURE").is_err() {
        println!("❌ HYPRLAND_INSTANCE_SIGNATURE not set");
        println!("   Make sure you're running this from within a Hyprland session");
        return Ok(());
    }

    // Create Hyprland client and animation engine
    let client = HyprlandClient::new().await?;
    let mut engine = AnimationEngine::new();
    let mut animator = WindowAnimator::new();

    // Provide Hyprland client to animator
    animator
        .set_hyprland_client(std::sync::Arc::new(client.clone()))
        .await;

    info!("✅ Connected to Hyprland - ready for real animations!");

    // Demo 1: Basic directional animation (like Pyprland but smoother)
    demo_basic_directional(&mut engine, &client, &mut animator).await?;
    sleep(Duration::from_secs(2)).await;

    // Demo 2: Physics-based spring animation
    demo_spring_physics(&mut engine, &client, &mut animator).await?;
    sleep(Duration::from_secs(2)).await;

    // Demo 3: Advanced multi-property animation
    demo_multi_property(&mut engine, &client, &mut animator).await?;
    sleep(Duration::from_secs(2)).await;

    // Demo 4: Custom easing functions
    demo_custom_easing(&mut engine, &client, &mut animator).await?;
    sleep(Duration::from_secs(2)).await;

    // Demo 5: Animation sequences and chaining
    demo_animation_sequence(&mut engine, &client, &mut animator).await?;
    sleep(Duration::from_secs(2)).await;

    // Demo 6: Performance monitoring
    demo_performance_monitoring(&mut engine, &client, &mut animator).await?;

    println!("\n✅ Animation showcase completed!");
    println!("🚀 Rustrland's animation system provides:");
    println!("   • 25+ easing functions (vs Pyprland's basic linear/ease)");
    println!("   • Physics-based spring dynamics");
    println!("   • Multi-property animations");
    println!("   • 60fps smooth interpolation");
    println!("   • Custom cubic-bezier curves");
    println!("   • Animation chaining and sequences");
    println!("   • Real-time performance monitoring");
    println!("   • Adaptive quality based on system performance");

    Ok(())
}

async fn demo_basic_directional(
    engine: &mut AnimationEngine,
    client: &HyprlandClient,
    animator: &mut WindowAnimator,
) -> anyhow::Result<()> {
    println!("1️⃣  Basic Directional Animation (Enhanced fromTop)");
    println!("   Like Pyprland but with smooth easing and 60fps - REAL WINDOW!");

    // Spawn foot terminal off-screen
    println!("   🚀 Spawning foot terminal for demo...");
    animator
        .spawn_window_offscreen("foot", (200, -600), (800, 600))
        .await?;
    //spawn_window_offscreen("foot", 560, -400).await?;

    // Wait for window to appear
    let foot_window = animator.wait_for_window_by_class("foot", 3000).await?;
    if let Some(window) = foot_window {
        println!(
            "   ✅ Found foot window at ({}, {})",
            window.at.0, window.at.1
        );
        let address = window.address.to_string();

        // Target position (center screen)
        let target_x = 560;
        let target_y = 300;
        let start_y = window.at.1 as i32;

        println!(
            "   🎬 Animating fromTop: {} → {} with ease-out-cubic",
            start_y, target_y
        );

        // Manual animation (since engine integration needs more work)
        let easing = EasingFunction::from_name("ease-out-cubic");
        let total_frames = 18; // 300ms at 60fps
        let distance = target_y - start_y;

        for frame in 0..total_frames {
            let progress = frame as f32 / (total_frames - 1) as f32;
            let eased_progress = easing.apply(progress);
            let current_y = start_y + (distance as f32 * eased_progress) as i32;

            client
                .move_window_pixel(&address, target_x, current_y)
                .await
                .ok();

            if frame % 6 == 0 {
                println!(
                    "   Frame {}: Y = {}px (progress: {:.2})",
                    frame, current_y, progress
                );
            }

            sleep(Duration::from_millis(16)).await; // 60fps
        }

        // Final position
        client
            .move_window_pixel(&address, target_x, target_y)
            .await?;
        println!("   ✅ Smooth fromTop animation complete - window visible!");

        sleep(Duration::from_secs(2)).await; // Let user see result

        // Close window
        animator.close_window(&address).await?;

        println!("   🧹 Cleaned up demo window\n");
    } else {
        println!("   ❌ Could not spawn foot window for demo\n");
    }

    Ok(())
}

async fn demo_spring_physics(
    engine: &mut AnimationEngine,
    client: &HyprlandClient,
    animator: &mut WindowAnimator,
) -> anyhow::Result<()> {
    println!("2️⃣  Physics-Based Spring Animation");
    println!("   Real spring dynamics (NOT available in Pyprland) - REAL WINDOW!");

    // Spawn kitty terminal off-screen for variety
    println!("   🚀 Spawning foot terminal for spring demo...");
    animator
        .spawn_window_offscreen("foot", (200, -600), (800, 600))
        .await?;
    //spawn_window_offscreen("foot", 400, -500).await?;

    let foot_window = animator.wait_for_window_by_class("foot", 3000).await?;
    if let Some(window) = foot_window {
        println!(
            "   ✅ Found foot window at ({}, {})",
            window.at.0, window.at.1
        );
        let address = window.address.to_string();

        let target_x = 400;
        let target_y = 200;
        let start_y = window.at.1 as i32;

        println!(
            "   🔬 Spring physics animation: {} → {} with bounce",
            start_y, target_y
        );

        // Use spring easing for realistic physics
        let easing = EasingFunction::from_name("spring");
        let total_frames = 60; // 1000ms for spring to settle
        let distance = target_y - start_y;

        for frame in 0..total_frames {
            let progress = frame as f32 / (total_frames - 1) as f32;
            let eased_progress = easing.apply(progress);
            let current_y = start_y + (distance as f32 * eased_progress) as i32;

            client
                .move_window_pixel(&address, target_x, current_y)
                .await
                .ok();

            if frame % 12 == 0 {
                println!(
                    "   Frame {}: Y = {}px (spring physics: {:.3})",
                    frame, current_y, eased_progress
                );
            }

            sleep(Duration::from_millis(16)).await; // 60fps
        }

        client
            .move_window_pixel(&address, target_x, target_y)
            .await?;
        println!("   ✅ Physically accurate spring animation complete!");

        sleep(Duration::from_secs(2)).await;

        // Close window
        animator.close_window(&address).await?;

        println!("   🧹 Cleaned up spring demo window\n");
    } else {
        println!("   ❌ Could not spawn kitty window for demo\n");
    }

    Ok(())
}

async fn demo_multi_property(
    engine: &mut AnimationEngine,
    client: &HyprlandClient,
    animator: &mut WindowAnimator,
) -> anyhow::Result<()> {
    println!("3️⃣  Multi-Property Animation");
    println!("   Animate position AND overshoot effect simultaneously - REAL WINDOW!");

    // Spawn thunar file manager for this demo
    println!("   🚀 Spawning thunar file manager for multi-property demo...");
    animator
        .spawn_window_offscreen("thunar", (200, -600), (800, 600))
        .await?;
    //spawn_window_offscreen("thunar", -200, 150).await?;

    let thunar_window = animator.wait_for_window_by_class("thunar", 3000).await?;
    if let Some(window) = thunar_window {
        println!(
            "   ✅ Found thunar window at ({}, {})",
            window.at.0, window.at.1
        );
        let address = window.address.to_string();

        let start_x = window.at.0 as i32;
        let start_y = window.at.1 as i32;
        let target_x = 600; // Move right across screen
        let target_y = 250; // And down a bit

        println!(
            "   🎯 Multi-property animation: position ({},{}) → ({},{}) with ease-out-back",
            start_x, start_y, target_x, target_y
        );

        // Use ease-out-back for overshoot effect
        let easing = EasingFunction::from_name("ease-out-back");
        let total_frames = 30; // 500ms animation
        let distance_x = target_x - start_x;
        let distance_y = target_y - start_y;

        for frame in 0..total_frames {
            let progress = frame as f32 / (total_frames - 1) as f32;
            let eased_progress = easing.apply(progress);

            let current_x = start_x + (distance_x as f32 * eased_progress) as i32;
            let current_y = start_y + (distance_y as f32 * eased_progress) as i32;

            client
                .move_window_pixel(&address, current_x, current_y)
                .await
                .ok();

            if frame % 6 == 0 {
                let overshoot_info = if eased_progress > 1.0 {
                    " ← OVERSHOOT!"
                } else {
                    ""
                };
                println!(
                    "   Frame {}: ({}, {}) eased={:.3}{}",
                    frame, current_x, current_y, eased_progress, overshoot_info
                );
            }

            sleep(Duration::from_millis(16)).await; // 60fps
        }

        client
            .move_window_pixel(&address, target_x, target_y)
            .await?;
        println!("   ✅ Multi-property animation with overshoot complete!");

        sleep(Duration::from_secs(2)).await;

        // Close window
        animator.close_window(&address).await?;

        println!("   🧹 Cleaned up multi-property demo window\n");
    } else {
        println!("   ❌ Could not spawn thunar window for demo\n");
    }

    Ok(())
}

async fn demo_custom_easing(
    engine: &mut AnimationEngine,
    client: &HyprlandClient,
    animator: &mut WindowAnimator,
) -> anyhow::Result<()> {
    println!("4️⃣  Custom Easing Functions Showcase");
    println!("   Demonstrating bounce effect with REAL WINDOW movement!");

    // Quick demonstration with one spectacular easing
    println!("   🚀 Spawning terminal for bounce showcase...");
    animator
        .spawn_window_offscreen("foot", (200, -600), (800, 600))
        .await?;
    //spawn_window_offscreen("foot", 300, -300).await?;

    let foot_window = animator.wait_for_window_by_class("foot", 3000).await?;
    if let Some(window) = foot_window {
        println!("   ✅ Found terminal at ({}, {})", window.at.0, window.at.1);
        let address = window.address.to_string();

        let start_y = window.at.1 as i32;
        let target_y = 400;

        println!(
            "   🎨 Demonstrating ease-out-bounce: {} → {}",
            start_y, target_y
        );

        let easing = EasingFunction::from_name("ease-out-bounce");
        let total_frames = 36; // 600ms for good bounce visibility
        let distance = target_y - start_y;

        for frame in 0..total_frames {
            let progress = frame as f32 / (total_frames - 1) as f32;
            let eased_progress = easing.apply(progress);
            let current_y = start_y + (distance as f32 * eased_progress) as i32;

            client
                .move_window_pixel(&address, 300, current_y)
                .await
                .ok();

            if frame % 8 == 0 {
                println!(
                    "   Frame {}: Y={}px bounce={:.3}",
                    frame, current_y, eased_progress
                );
            }

            sleep(Duration::from_millis(16)).await;
        }

        client.move_window_pixel(&address, 300, target_y).await?;
        println!("   ✅ Spectacular bounce animation complete!");

        sleep(Duration::from_secs(1)).await;

        // Close window
        animator.close_window(&address).await?;

        println!("   🧹 Cleaned up easing demo window");
    } else {
        println!("   ❌ Could not spawn terminal for easing demo");
    }

    println!("   🎯 Rustrland supports 25+ easing functions:");
    println!("      Linear, Cubic, Bounce, Elastic, Spring, Back, Custom Bezier, etc.\n");

    Ok(())
}

async fn demo_animation_sequence(
    engine: &mut AnimationEngine,
    client: &HyprlandClient,
    animator: &mut WindowAnimator,
) -> anyhow::Result<()> {
    println!("5️⃣  Animation Sequences with REAL WINDOW");
    println!("   Chain multiple animations together - sequence of movements!");

    // Spawn foot terminal for sequence demo
    println!("   🚀 Spawning terminal for sequence demo...");
    animator
        .spawn_window_offscreen("foot", (200, -600), (800, 600))
        .await?;
    //spawn_window_offscreen("foot", 100, -300).await?;

    let foot_window = animator.wait_for_window_by_class("foot", 3000).await?;
    if let Some(window) = foot_window {
        println!("   ✅ Found terminal at ({}, {})", window.at.0, window.at.1);
        let address = window.address.to_string();

        // Sequence: slide in → bounce → slide out
        println!("   🎬 Animation sequence: Slide In → Bounce → Slide Out");

        let start_x = 100;
        let start_y = window.at.1 as i32;

        // Phase 1: Slide in from top with ease-out
        println!("   Phase 1/3: Slide in from top (ease-out)");
        let easing1 = EasingFunction::from_name("ease-out");
        let target_y1 = 200;
        let distance1 = target_y1 - start_y;

        for frame in 0..18 {
            // 300ms
            let progress = frame as f32 / 17.0;
            let eased = easing1.apply(progress);
            let current_y = start_y + (distance1 as f32 * eased) as i32;
            client
                .move_window_pixel(&address, start_x, current_y)
                .await
                .ok();

            if frame % 6 == 0 {
                println!("   Phase 1 Frame {}: Y={}px", frame, current_y);
            }
            sleep(Duration::from_millis(16)).await;
        }

        // Phase 2: Bounce horizontally with ease-out-bounce
        println!("   Phase 2/3: Bounce horizontally (ease-out-bounce)");
        let easing2 = EasingFunction::from_name("ease-out-bounce");
        let target_x2 = 800;
        let distance2 = target_x2 - start_x;

        for frame in 0..30 {
            // 500ms
            let progress = frame as f32 / 29.0;
            let eased = easing2.apply(progress);
            let current_x = start_x + (distance2 as f32 * eased) as i32;
            client
                .move_window_pixel(&address, current_x, target_y1)
                .await
                .ok();

            if frame % 8 == 0 {
                println!(
                    "   Phase 2 Frame {}: X={}px (bounce={:.3})",
                    frame, current_x, eased
                );
            }
            sleep(Duration::from_millis(16)).await;
        }

        // Phase 3: Slide out to bottom with ease-in
        println!("   Phase 3/3: Slide out to bottom (ease-in)");
        let easing3 = EasingFunction::from_name("ease-in");
        let target_y3 = 800; // Off-screen bottom
        let distance3 = target_y3 - target_y1;

        for frame in 0..18 {
            // 300ms
            let progress = frame as f32 / 17.0;
            let eased = easing3.apply(progress);
            let current_y = target_y1 + (distance3 as f32 * eased) as i32;
            client
                .move_window_pixel(&address, target_x2, current_y)
                .await
                .ok();

            if frame % 6 == 0 {
                println!("   Phase 3 Frame {}: Y={}px", frame, current_y);
            }
            sleep(Duration::from_millis(16)).await;
        }

        println!("   ✅ Complex 3-phase animation sequence completed!");
        println!("      Slide In (ease-out) → Bounce (bounce) → Slide Out (ease-in)");

        // Close window
        tokio::process::Command::new("hyprctl")
            .arg("dispatch")
            .arg("closewindow")
            .arg(format!("address:{}", address))
            .output()
            .await
            .ok();

        println!("   🧹 Cleaned up sequence demo window\n");
    } else {
        println!("   ❌ Could not spawn terminal for sequence demo");
        println!("   📈 Fallback: Showing keyframe timeline structure:");

        // Create timeline with keyframes as fallback
        let timeline = TimelineBuilder::new(Duration::from_millis(600))
            .keyframe(0.0, 0.0, None)
            .keyframe(0.3, 1.0, Some("ease-out"))
            .keyframe(0.7, 0.8, Some("ease-in"))
            .keyframe(1.0, 1.0, Some("ease-out"))
            .direction(AnimationDirection::Normal)
            .build();

        for i in 0..21 {
            let progress = i as f32 / 20.0;
            let value = timeline.get_value_at_progress(progress);
            if i % 4 == 0 {
                println!("   Progress {:.2}: Value {:.3}", progress, value);
            }
        }
        println!("   ✅ Advanced timeline with keyframes\n");
    }

    Ok(())
}

async fn demo_performance_monitoring(
    engine: &mut AnimationEngine,
    client: &HyprlandClient,
    animator: &mut WindowAnimator,
) -> anyhow::Result<()> {
    println!("6️⃣  Performance Monitoring with REAL WINDOWS");
    println!("   Stress test with multiple simultaneous animations!");

    // Spawn 3 terminals simultaneously for performance test
    println!("   🚀 Spawning 3 terminals for performance stress test...");

    // Spawn multiple windows off-screen at different positions
    animator
        .spawn_window_offscreen("foot", (100, -600), (800, 600))
        .await?;
    animator
        .spawn_window_offscreen("foot", (200, -600), (800, 600))
        .await?;
    animator
        .spawn_window_offscreen("foot", (300, -600), (800, 600))
        .await?;
    //spawn_window_offscreen("foot", 200, -400).await?;
    //spawn_window_offscreen("foot", 400, -400).await?;
    //spawn_window_offscreen("foot", 600, -400).await?;

    sleep(Duration::from_millis(500)).await; // Give time for all to spawn

    let windows = client.get_windows().await?;
    let foot_windows: Vec<_> = windows
        .iter()
        .filter(|w| w.class.to_lowercase().contains("foot"))
        .cloned()
        .collect();

    if foot_windows.len() >= 3 {
        println!(
            "   ✅ Found {} foot windows for performance test",
            foot_windows.len()
        );

        // Animate all 3 simultaneously with different easing functions
        println!("   🎯 Starting simultaneous animations:");
        println!("      Window 1: ease-out-cubic");
        println!("      Window 2: ease-out-bounce");
        println!("      Window 3: ease-out-back");

        let start_time = std::time::Instant::now();
        let total_frames = 40; // 666ms total
        let target_y = 250;

        for frame in 0..total_frames {
            let frame_start = std::time::Instant::now();
            let progress = frame as f32 / (total_frames - 1) as f32;

            // Animate each window with different easing
            if let Some(window1) = foot_windows.get(0) {
                let easing = EasingFunction::from_name("ease-out-cubic");
                let eased = easing.apply(progress);
                let start_y = -400;
                let current_y = start_y + ((target_y - start_y) as f32 * eased) as i32;
                client
                    .move_window_pixel(&window1.address.to_string(), 200, current_y)
                    .await
                    .ok();
            }

            if let Some(window2) = foot_windows.get(1) {
                let easing = EasingFunction::from_name("ease-out-bounce");
                let eased = easing.apply(progress);
                let start_y = -400;
                let current_y = start_y + ((target_y - start_y) as f32 * eased) as i32;
                client
                    .move_window_pixel(&window2.address.to_string(), 400, current_y)
                    .await
                    .ok();
            }

            if let Some(window3) = foot_windows.get(2) {
                let easing = EasingFunction::from_name("ease-out-back");
                let eased = easing.apply(progress);
                let start_y = -400;
                let current_y = start_y + ((target_y - start_y) as f32 * eased) as i32;
                client
                    .move_window_pixel(&window3.address.to_string(), 600, current_y)
                    .await
                    .ok();
            }

            // Performance monitoring
            let frame_time = frame_start.elapsed();
            if frame % 10 == 0 {
                let elapsed = start_time.elapsed().as_millis();
                let current_fps = 1000.0 / frame_time.as_millis() as f32;
                println!(
                    "   Frame {}: {:.1}ms ({:.0} FPS) - {} windows animated",
                    frame,
                    frame_time.as_millis(),
                    current_fps,
                    foot_windows.len()
                );
            }

            sleep(Duration::from_millis(16)).await; // Target 60fps
        }

        let total_time = start_time.elapsed();
        let average_fps = (total_frames as f32 / total_time.as_secs_f32()).round();

        println!("   📊 Performance Statistics:");
        println!("      • Total Animation Time: {}ms", total_time.as_millis());
        println!("      • Target FPS: 60.0");
        println!("      • Achieved FPS: {:.1}", average_fps);
        println!("      • Simultaneous Windows: {}", foot_windows.len());
        println!(
            "      • Total Frames Rendered: {}",
            total_frames * foot_windows.len()
        );
        println!("      • Animation Complexity: Multi-window + different easing functions");

        sleep(Duration::from_secs(1)).await;

        // Clean up all windows
        for window in &foot_windows {
            animator
                .close_window(&format!("{}", window.address))
                .await?;
        }

        println!(
            "   🧹 Cleaned up {} performance test windows",
            foot_windows.len()
        );
        println!(
            "   ✅ Performance test completed - system handled {} simultaneous animations!",
            foot_windows.len()
        );
    } else {
        println!("   ❌ Could not spawn enough windows for performance test");
        println!("   📊 Fallback: Showing theoretical performance stats:");

        // Simulate performance stats as fallback
        for i in 0..3 {
            let config = AnimationConfig {
                animation_type: "fromTop".to_string(),
                duration: 200,
                target_fps: 60,
                ..Default::default()
            };

            let mut properties = HashMap::new();
            properties.insert("x".to_string(), PropertyValue::Pixels(i * 100));

            engine
                .start_animation(format!("perf_test_{}", i), config, properties.clone(), properties)
                .await?;
        }

        sleep(Duration::from_millis(100)).await;

        let stats = engine.get_performance_stats();
        println!("      • Current FPS: {:.1}", stats.current_fps);
        println!("      • Target FPS: {:.1}", stats.target_fps);
        println!("      • Active Animations: {}", stats.active_animations);
        println!(
            "      • Average Frame Time: {:.2}ms",
            stats.average_frame_time.as_millis()
        );

        println!("   ✅ Theoretical performance monitoring");
    }

    println!();
    Ok(())
}
