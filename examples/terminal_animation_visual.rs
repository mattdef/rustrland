use rustrland::animation::{
    AnimationConfig, AnimationEngine,
    properties::PropertyValue,
    easing::EasingFunction,
};
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::sleep;

/// Terminal Visual Animation - Shows animations in terminal with visual bars
/// Run with: cargo run --example terminal_animation_visual
/// This provides immediate visual feedback without needing Hyprland

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🎬 Terminal Visual Animation Test");
    println!("=================================");
    println!("Visual representation of demo_basic_directional animation\n");

    // Test the actual demo_basic_directional logic
    demo_basic_directional_visual().await?;

    // Also test easing functions visually
    demo_easing_visual().await?;

    Ok(())
}

async fn demo_basic_directional_visual() -> anyhow::Result<()> {
    println!("1️⃣  Visual demo_basic_directional Animation");
    println!("   300ms duration, ease-out-cubic easing, fromTop animation\n");

    let mut engine = AnimationEngine::new();

    // Exact same config as demo_basic_directional
    let config = AnimationConfig {
        animation_type: "fromTop".to_string(),
        duration: 300,
        easing: "ease-out-cubic".to_string(),
        offset: "100px".to_string(),
        ..Default::default()
    };

    let mut properties = HashMap::new();
    properties.insert("x".to_string(), PropertyValue::Pixels(100));
    properties.insert("y".to_string(), PropertyValue::Pixels(-200)); // Start off-screen

    println!("   🎯 Target: Y moves from -200px to -100px (100px offset)");
    println!("   ⏱️  Duration: 300ms with ease-out-cubic easing\n");

    engine.start_animation("demo1".to_string(), config, properties).await?;

    // Visual representation
    println!("   Animation Progress:");
    println!("   Y Position  │ Progress Bar                    │ Frame");
    println!("   ──────────────────────────────────────────────────────");

    for frame in 0..25 {
        if let Some(props) = engine.get_current_properties("demo1") {
            if let Some(PropertyValue::Pixels(y)) = props.get("y") {
                let progress = ((y + 200) as f32 / 100.0).clamp(0.0, 1.0);
                let bar_length = (progress * 30.0) as usize;
                let bar = "█".repeat(bar_length) + &"░".repeat(30 - bar_length);
                
                println!("   {:4}px      │ {} │ {:2}", 
                        y, bar, frame);
            }
        }
        sleep(Duration::from_millis(12)).await; // ~80fps for smooth terminal animation
    }

    engine.stop_animation("demo1")?;
    println!("   ──────────────────────────────────────────────────────");
    println!("   ✅ Animation completed visually!\n");

    Ok(())
}

async fn demo_easing_visual() -> anyhow::Result<()> {
    println!("2️⃣  Visual Easing Function Comparison");
    println!("   Comparing different easing functions over time\n");

    let easing_functions = vec![
        ("linear", "Linear"),
        ("ease-out-cubic", "Ease Out Cubic"),
        ("ease-in-out", "Ease In Out"),
        ("ease-out-bounce", "Bounce"),
        ("ease-out-back", "Back (Overshoot)"),
    ];

    for (easing_name, display_name) in easing_functions {
        println!("   🎨 {} Easing:", display_name);
        println!("   Time    │ Progress Bar                    │ Value");
        println!("   ──────────────────────────────────────────────────");
        
        let easing = EasingFunction::from_name(easing_name);
        
        for i in 0..=20 {
            let time = i as f32 / 20.0; // 0.0 to 1.0
            let eased_value = easing.apply(time);
            
            // Handle overshoot visualization (values > 1.0)
            let bar_length = if eased_value <= 1.0 {
                (eased_value * 30.0) as usize
            } else {
                30 + ((eased_value - 1.0) * 10.0) as usize // Extra chars for overshoot
            };
            
            let bar = if eased_value <= 1.0 {
                "█".repeat(bar_length.min(30)) + &"░".repeat(30 - bar_length.min(30))
            } else {
                "█".repeat(30) + &"▓".repeat((bar_length - 30).min(10)) // Different char for overshoot
            };
            
            let overshoot_indicator = if eased_value > 1.0 { " ← OVERSHOOT!" } else { "" };
            
            println!("   {:.2}     │ {} │ {:.3}{}", 
                    time, bar, eased_value, overshoot_indicator);
            
            sleep(Duration::from_millis(50)).await;
        }
        
        println!("   ──────────────────────────────────────────────────");
        println!("   ✅ {} complete\n", display_name);
        sleep(Duration::from_millis(200)).await;
    }

    Ok(())
}