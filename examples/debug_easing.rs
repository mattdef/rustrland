use anyhow::Result;
use rustrland::animation::{easing::EasingFunction, properties::PropertyValue};
use std::collections::HashMap;

fn main() -> Result<()> {
    println!("🔍 Debugging EaseOutBack Animation Interpolation");
    println!("===============================================");

    let easing = EasingFunction::EaseOutBack;
    
    // Simulate window animation from left side (like fromLeft)
    let start_x = PropertyValue::Pixels(-800); // Off-screen left
    let target_x = PropertyValue::Pixels(400); // Center of screen
    
    println!("Animation simulation: fromLeft with EaseOutBack");
    println!("Start position: {:?}", start_x);
    println!("Target position: {:?}", target_x);
    println!();
    
    println!("Frame | Progress | Eased    | X Position | Expected Overshoot");
    println!("------|----------|----------|------------|------------------");
    
    for frame in 0..=20 {
        let progress = frame as f32 / 20.0;
        let eased = easing.apply(progress);
        let interpolated = start_x.interpolate(&target_x, eased);
        
        let x_pos = if let PropertyValue::Pixels(x) = interpolated {
            x
        } else {
            0
        };
        
        let overshoot = if x_pos > 400 { 
            format!("+{}", x_pos - 400) 
        } else { 
            "".to_string() 
        };
        
        println!(
            "{:5} | {:.3}    | {:.6} | {:10} | {}",
            frame, progress, eased, x_pos, overshoot
        );
    }
    
    println!();
    println!("Expected behavior:");
    println!("- Window should overshoot target position (400px)");
    println!("- Maximum overshoot around frame 12 (progress ~0.6)"); 
    println!("- Then settle back to exactly 400px at frame 20");
    
    Ok(())
}