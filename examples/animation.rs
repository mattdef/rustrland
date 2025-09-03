use hyprland::data::Monitor;
use rustrland::animation::WindowAnimator;
use rustrland::ipc::{HyprlandClient, MonitorInfo};
use rustrland::{AnimationConfig, EasingFunction};
use std::io::{self, Write};
use std::time::Duration;
use tokio::time::sleep;
use tracing::debug;
use tracing_subscriber;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing subscriber to see debug logs
    tracing_subscriber::fmt::init();

    println!("🎬 Simple window Animation Test");
    println!("=================================");
    println!("Visual representation of a window animation\n");

    // Create Hyprland client and animation engine
    let client = HyprlandClient::new().await?;
    let monitors = client.get_monitors().await?;

    // Find the active monitor
    let active_monitor = monitors.iter().find(|monitor| monitor.focused);
    match active_monitor {
        Some(monitor) => {
            println!("   🖥️ Found active monitor:");
            println!("      - Name: {}", monitor.name);
            println!("      - Resolution: {}x{}", monitor.width, monitor.height);
            println!("      - Position: ({}, {})", monitor.x, monitor.y);
            println!("      - Scale: {}", monitor.scale);

            // Start the demo
            demo(&client, &monitor).await?;
        }
        None => {
            println!("   ⚠️  Active monitor not found, available monitors:");
            for monitor in &monitors {
                println!("      - {}", monitor.name);
            }
        }
    }

    Ok(())
}

async fn demo(client: &HyprlandClient, monitor: &Monitor) -> anyhow::Result<()> {
    println!("   Choose an option to visualize");

    let from_functions = vec![
        ("fromLeft", "From Left"),
        ("fromRight", "From Right"),
        ("fromTop", "From Top"),
        ("fromBottom", "From Bottom"),
        ("fromTopLeft", "From Top Left"),
        ("fromTopRight", "From Top Right"),
        ("fromBottomLeft", "From Bottom Left"),
        ("fromBottomRight", "From Bottom Right"),
        ("fade", "Fade"),
        ("scale", "Scale"),
        ("spring", "Spring"),
    ];

    let monitor_info = MonitorInfo {
        active_workspace_id: monitor.active_workspace.id,
        height: monitor.height,
        width: monitor.width,
        id: monitor.id,
        name: monitor.name.to_string(),
        x: monitor.x,
        y: monitor.y,
        scale: monitor.scale,
        is_focused: monitor.focused,
        refresh_rate: monitor.refresh_rate,
    };

    loop {
        println!("   Available option functions:");
        println!("   ─────────────────────────────────────");
        for (i, (_, display_name)) in from_functions.iter().enumerate() {
            println!("   {:2}. {}", i + 1, display_name);
        }
        println!("   {:2}. Quit", from_functions.len() + 1);
        println!("   ─────────────────────────────────────");

        print!("   Enter your choice (1-{}): ", from_functions.len() + 1);
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        let choice = match input.trim().parse::<usize>() {
            Ok(n) => n,
            Err(_) => {
                println!("   ❌ Please enter a valid number\n");
                continue;
            }
        };

        if choice == 0 || choice > from_functions.len() + 1 {
            println!(
                "   ❌ Invalid choice. Please enter a number between 1 and {}\n",
                from_functions.len() + 1
            );
            continue;
        }

        if choice == from_functions.len() + 1 {
            println!("   👋 Goodbye!");
            break;
        }

        let (option_name, display_name) = &from_functions[choice - 1];
        debug!("\n   🎨 Testing {} Option:", display_name);

        let mut animator = WindowAnimator::new();
        animator
            .set_hyprland_client(std::sync::Arc::new(client.clone()))
            .await;

        // Initialize the animator with the correct monitor info
        animator.set_active_monitor(&monitor_info).await;

        let config = AnimationConfig {
            animation_type: option_name.to_string(),
            duration: 800, // Very slow animation to see the bounce effect clearly
            easing: EasingFunction::EaseOutBack,
            offset: "100px".to_string(), // Larger offset for more dramatic effect
            opacity_from: if option_name == &"fade" { 0.1 } else { 1.0 }, // Start fade from 0.1 for visibility
            ..Default::default()
        };

        let size = (800, 600);
        let app = "foot"; // Try different apps: foot, firefox, thunar, dolphin, kate

        // show_animated_window("foot", "DP-1", (800, 600), config, &mut animator).await?;
        debug!(
            "🚀 Launching {} on {} with {} animation",
            app, monitor.name, config.animation_type
        );
        debug!("   📐 Size: {}x{}", size.0, size.1);
        debug!("   ⏱️  Duration: {}ms", config.duration);
        debug!("   📈 Easing: {:?}", config.easing);

        // Get monitor info and calculate center position
        // let target_position =  animator.calculate_monitor_center_position(&monitor_info, size).await?;
        let target_position = (
            (monitor.width as i32 - size.0) / 2,
            (monitor.height as i32 - size.1) / 2,
        );
        debug!(
            "   📍 Target position: ({}, {})",
            target_position.0, target_position.1
        );

        // Launch the animated window
        let window = animator
            .show_window_with_animation(app, target_position, size, config)
            .await?;

        if let Some(ref window) = window {
            debug!("✅ Window launched successfully: {}", window.address);
            debug!("   🔍 Class: {}, Title: {}", window.class, window.title);
            debug!("   📍 Position: ({}, {})", window.at.0, window.at.1);
            debug!("   📐 Size: {}x{}", window.size.0, window.size.1);
            debug!("   🎯 Floating: {}", window.floating);

            // Wait longer to see animation, then close window
            debug!("⏳ Waiting 5 seconds to observe the animation...");
            sleep(Duration::from_secs(5)).await;

            let hide_config = AnimationConfig {
                animation_type: "toTop".to_string(),
                duration: 800,
                easing: EasingFunction::EaseIn,
                offset: "100px".to_string(),
                ..Default::default()
            };
            animator
                .hide_window(
                    &window.address.to_string(),
                    target_position,
                    size,
                    hide_config,
                )
                .await?;

            sleep(Duration::from_secs(1)).await;
            animator.close_window(&window.address.to_string()).await?;
            println!("🔴 Window closed");
        } else {
            println!("❌ Failed to launch window");
        }

        // Small pause before showing menu again
        sleep(Duration::from_millis(500)).await;
    }

    Ok(())
}
