use rustrland::animation::easing::EasingFunction;

fn main() {
    println!("🧪 Testing EaseOutBack easing function");
    println!("=====================================");

    let easing = EasingFunction::EaseOutBack;

    // Test progression from 0.0 to 1.0
    println!("Progress -> Eased Value:");
    for i in 0..=20 {
        let progress = i as f32 / 20.0;
        let eased = easing.apply(progress);
        println!("  {:.2} -> {:.6}", progress, eased);
    }

    // Test specific values that should show overshoot
    println!("\nKey points:");
    println!("  0.0 -> {:.6} (should be 0.0)", easing.apply(0.0));
    println!(
        "  0.5 -> {:.6} (should be around 0.6-0.7)",
        easing.apply(0.5)
    );
    println!("  0.8 -> {:.6} (should overshoot > 1.0)", easing.apply(0.8));
    println!("  0.9 -> {:.6} (should overshoot > 1.0)", easing.apply(0.9));
    println!("  1.0 -> {:.6} (should be exactly 1.0)", easing.apply(1.0));

    // Calculate the maximum overshoot point
    let mut max_value = 0.0;
    let mut max_progress = 0.0;
    for i in 0..=100 {
        let progress = i as f32 / 100.0;
        let eased = easing.apply(progress);
        if eased > max_value {
            max_value = eased;
            max_progress = progress;
        }
    }

    println!("\n📈 Maximum overshoot:");
    println!(
        "  Progress: {:.2}, Value: {:.6} (overshoot: {:.3})",
        max_progress,
        max_value,
        max_value - 1.0
    );

    // Test window position calculation simulation
    println!("\n🪟 Simulating window position during animation:");
    let start_x = -800; // Window starts off-screen
    let target_x = 400; // Target position on screen

    for i in 0..=10 {
        let progress = i as f32 / 10.0;
        let eased = easing.apply(progress);
        let current_x = start_x + ((target_x - start_x) as f32 * eased) as i32;
        println!(
            "  Progress {:.1}: eased={:.3}, window_x={}",
            progress, eased, current_x
        );
    }
}
