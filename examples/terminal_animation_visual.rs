use rustrland::animation::
    easing::EasingFunction
;
use std::time::Duration;
use std::io::{self, Write};
use tokio::time::sleep;

/// Terminal Visual Animation - Shows animations in terminal with visual bars
/// Run with: cargo run --example terminal_animation_visual
/// This provides immediate visual feedback without needing Hyprland

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🎬 Terminal Visual Animation Test");
    println!("=================================");
    println!("Visual representation of demo_basic_directional animation\n");

    // Interactive easing function selector
    demo_easing_interactive().await?;

    Ok(())
}

async fn demo_easing_interactive() -> anyhow::Result<()> {
    println!("2️⃣  Interactive Easing Function Demo");
    println!("   Choose an easing function to visualize\n");

    let easing_functions = vec![
        ("linear", "Linear"),
        ("ease", "Ease"),
        ("ease-in", "Ease In"),
        ("ease-out", "Ease Out"),
        ("ease-in-out", "Ease In Out"),
        ("ease-in-sine", "Ease In Sine"),
        ("ease-out-sine", "Ease Out Sine"),
        ("ease-in-out-sine", "Ease In Out Sine"),
        ("ease-in-quad", "Ease In Quad"),
        ("ease-out-quad", "Ease Out Quad"),
        ("ease-in-out-quad", "Ease In Out Quad"),
        ("ease-in-cubic", "Ease In Cubic"),
        ("ease-out-cubic", "Ease Out Cubic"),
        ("ease-in-out-cubic", "Ease In Out Cubic"),
        ("ease-in-quart", "Ease In Quart"),
        ("ease-out-quart", "Ease Out Quart"),
        ("ease-in-out-quart", "Ease In Out Quart"),
        ("ease-in-quint", "Ease In Quint"),
        ("ease-out-quint", "Ease Out Quint"),
        ("ease-in-out-quint", "Ease In Out Quint"),
        ("ease-in-expo", "Ease In Expo"),
        ("ease-out-expo", "Ease Out Expo"),
        ("ease-in-out-expo", "Ease In Out Expo"),
        ("ease-in-circ", "Ease In Circ"),
        ("ease-out-circ", "Ease Out Circ"),
        ("ease-in-out-circ", "Ease In Out Circ"),
        ("ease-in-back", "Ease In Back"),
        ("ease-out-back", "Ease Out Back"),
        ("ease-in-out-back", "Ease In Out Back"),
        ("ease-in-elastic", "Ease In Elastic"),
        ("ease-out-elastic", "Ease Out Elastic"),
        ("ease-in-out-elastic", "Ease In Out Elastic"),
        ("ease-in-bounce", "Ease In Bounce"),
        ("ease-out-bounce", "Ease Out Bounce"),
        ("ease-in-out-bounce", "Ease In Out Bounce"),
        ("spring", "Spring"),
    ];

    loop {
        println!("   Available easing functions:");
        println!("   ─────────────────────────────────────");
        for (i, (_, display_name)) in easing_functions.iter().enumerate() {
            println!("   {:2}. {}", i + 1, display_name);
        }
        println!("   {:2}. Quit", easing_functions.len() + 1);
        println!("   ─────────────────────────────────────");
        
        print!("   Enter your choice (1-{}): ", easing_functions.len() + 1);
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
        
        if choice == 0 || choice > easing_functions.len() + 1 {
            println!("   ❌ Invalid choice. Please enter a number between 1 and {}\n", easing_functions.len() + 1);
            continue;
        }

        if choice == easing_functions.len() + 1 {
            println!("   👋 Goodbye!");
            break;
        }

        let (easing_name, display_name) = &easing_functions[choice - 1];
        
        println!("\n   🎨 Testing {} Easing:", display_name);
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
            
            sleep(Duration::from_millis(100)).await;
        }
        
        println!("   ──────────────────────────────────────────────────");
        println!("   ✅ {} animation complete!\n", display_name);
        
        // Small pause before showing menu again
        sleep(Duration::from_millis(500)).await;
    }

    Ok(())
}