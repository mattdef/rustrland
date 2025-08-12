/// Animation Showcase - Demonstrates Rustrland's advanced animation system
/// Run with: cargo run --example animation_showcase

use std::time::Duration;
use tokio::time::sleep;
use rustrland::animation::{
    AnimationConfig, AnimationEngine, SpringConfig, AnimationPropertyConfig,
    properties::PropertyValue,
    timeline::{TimelineBuilder, AnimationDirection},
    easing::EasingFunction
};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    
    println!("🎬 Rustrland Animation System Showcase");
    println!("=====================================");
    println!("Demonstrating advanced animations that surpass Pyprland\n");
    
    // Create animation engine
    let mut engine = AnimationEngine::new();
    
    // Demo 1: Basic directional animation (like Pyprland but smoother)
    demo_basic_directional(&mut engine).await?;
    
    // Demo 2: Physics-based spring animation
    demo_spring_physics(&mut engine).await?;
    
    // Demo 3: Advanced multi-property animation
    demo_multi_property(&mut engine).await?;
    
    // Demo 4: Custom easing functions
    demo_custom_easing(&mut engine).await?;
    
    // Demo 5: Animation sequences and chaining
    demo_animation_sequence(&mut engine).await?;
    
    // Demo 6: Performance monitoring
    demo_performance_monitoring(&mut engine).await?;
    
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

async fn demo_basic_directional(engine: &mut AnimationEngine) -> anyhow::Result<()> {
    println!("1️⃣  Basic Directional Animation (Enhanced fromTop)");
    println!("   Like Pyprland but with smooth easing and 60fps");
    
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
    
    engine.start_animation("demo1".to_string(), config, properties).await?;
    
    // Simulate animation running
    for i in 0..10 {
        if let Some(props) = engine.get_current_properties("demo1") {
            if let Some(PropertyValue::Pixels(y)) = props.get("y") {
                println!("   Frame {}: Y position = {}px", i, y);
            }
        }
        sleep(Duration::from_millis(30)).await;
    }
    
    engine.stop_animation("demo1")?;
    println!("   ✅ Smooth 60fps animation with cubic easing\n");
    
    Ok(())
}

async fn demo_spring_physics(engine: &mut AnimationEngine) -> anyhow::Result<()> {
    println!("2️⃣  Physics-Based Spring Animation");
    println!("   Real spring dynamics (NOT available in Pyprland)");
    
    let config = AnimationConfig {
        animation_type: "spring".to_string(),
        duration: 500,
        easing: "spring".to_string(),
        spring: Some(SpringConfig {
            stiffness: 300.0,
            damping: 25.0,
            initial_velocity: 0.0,
            mass: 1.0,
        }),
        ..Default::default()
    };
    
    let mut properties = HashMap::new();
    properties.insert("scale".to_string(), PropertyValue::Float(0.5));
    
    engine.start_animation("spring_demo".to_string(), config, properties).await?;
    
    println!("   🔬 Physics simulation with spring dynamics:");
    for i in 0..15 {
        if let Some(props) = engine.get_current_properties("spring_demo") {
            if let Some(PropertyValue::Float(scale)) = props.get("scale") {
                println!("   Frame {}: Scale = {:.3} (realistic bounce)", i, scale);
            }
        }
        sleep(Duration::from_millis(30)).await;
    }
    
    engine.stop_animation("spring_demo")?;
    println!("   ✅ Physically accurate spring animation\n");
    
    Ok(())
}

async fn demo_multi_property(engine: &mut AnimationEngine) -> anyhow::Result<()> {
    println!("3️⃣  Multi-Property Animation");
    println!("   Animate position, scale, and opacity simultaneously");
    
    let config = AnimationConfig {
        animation_type: "complex".to_string(),
        duration: 400,
        easing: "ease-in-out".to_string(),
        properties: Some(vec![
            AnimationPropertyConfig {
                property: "x".to_string(),
                from: PropertyValue::Pixels(-100),
                to: PropertyValue::Pixels(200),
                easing: Some("ease-out".to_string()),
            },
            AnimationPropertyConfig {
                property: "opacity".to_string(),
                from: PropertyValue::Float(0.0),
                to: PropertyValue::Float(1.0),
                easing: Some("ease-in".to_string()),
            },
            AnimationPropertyConfig {
                property: "scale".to_string(),
                from: PropertyValue::Float(0.8),
                to: PropertyValue::Float(1.0),
                easing: Some("ease-out-back".to_string()),
            },
        ]),
        ..Default::default()
    };
    
    let mut properties = HashMap::new();
    properties.insert("x".to_string(), PropertyValue::Pixels(-100));
    properties.insert("opacity".to_string(), PropertyValue::Float(0.0));
    properties.insert("scale".to_string(), PropertyValue::Float(0.8));
    
    engine.start_animation("multi_prop".to_string(), config, properties).await?;
    
    println!("   🎯 Multiple properties with different easing:");
    for i in 0..12 {
        if let Some(props) = engine.get_current_properties("multi_prop") {
            let x = props.get("x").unwrap().as_pixels();
            let opacity = props.get("opacity").unwrap().as_float();
            let scale = props.get("scale").unwrap().as_float();
            println!("   Frame {}: X={}px, Opacity={:.2}, Scale={:.2}", i, x, opacity, scale);
        }
        sleep(Duration::from_millis(30)).await;
    }
    
    engine.stop_animation("multi_prop")?;
    println!("   ✅ Complex multi-property animation\n");
    
    Ok(())
}

async fn demo_custom_easing(engine: &mut AnimationEngine) -> anyhow::Result<()> {
    println!("4️⃣  Custom Easing Functions");
    println!("   Demonstrating various easing types");
    
    let easing_types = vec![
        ("linear", "Linear"),
        ("ease-in-cubic", "Cubic Ease In"),
        ("ease-out-bounce", "Bounce"),
        ("ease-out-elastic", "Elastic"),
        ("cubic-bezier(0.68, -0.55, 0.265, 1.55)", "Custom Bezier"),
    ];
    
    for (easing, name) in easing_types {
        println!("   🎨 Testing {} easing:", name);
        
        let easing_fn = EasingFunction::from_name(easing);
        print!("      Progress: ");
        for i in 0..11 {
            let progress = i as f32 / 10.0;
            let eased = easing_fn.apply(progress);
            print!("{:.2} ", eased);
        }
        println!();
    }
    
    println!("   ✅ Rich easing function library\n");
    
    Ok(())
}

async fn demo_animation_sequence(engine: &mut AnimationEngine) -> anyhow::Result<()> {
    println!("5️⃣  Animation Sequences");
    println!("   Chain multiple animations together");
    
    // Create timeline with keyframes
    let timeline = TimelineBuilder::new(Duration::from_millis(600))
        .keyframe(0.0, 0.0, None)
        .keyframe(0.3, 1.0, Some("ease-out"))
        .keyframe(0.7, 0.8, Some("ease-in"))
        .keyframe(1.0, 1.0, Some("ease-out"))
        .direction(AnimationDirection::Normal)
        .build();
    
    println!("   📈 Complex keyframe timeline:");
    for i in 0..21 {
        let progress = i as f32 / 20.0;
        let value = timeline.get_value_at_progress(progress);
        if i % 4 == 0 { // Print every 4th frame
            println!("   Progress {:.2}: Value {:.3}", progress, value);
        }
    }
    
    println!("   ✅ Advanced timeline with keyframes\n");
    
    Ok(())
}

async fn demo_performance_monitoring(engine: &mut AnimationEngine) -> anyhow::Result<()> {
    println!("6️⃣  Performance Monitoring");
    println!("   Real-time performance statistics");
    
    // Simulate some animations
    for i in 0..3 {
        let config = AnimationConfig {
            animation_type: "fromTop".to_string(),
            duration: 200,
            target_fps: 60,
            ..Default::default()
        };
        
        let mut properties = HashMap::new();
        properties.insert("x".to_string(), PropertyValue::Pixels(i * 100));
        
        engine.start_animation(format!("perf_test_{}", i), config, properties).await?;
    }
    
    sleep(Duration::from_millis(100)).await;
    
    let stats = engine.get_performance_stats();
    println!("   📊 Performance Statistics:");
    println!("      • Current FPS: {:.1}", stats.current_fps);
    println!("      • Target FPS: {:.1}", stats.target_fps);
    println!("      • Active Animations: {}", stats.active_animations);
    println!("      • Average Frame Time: {:.2}ms", stats.average_frame_time.as_millis());
    
    println!("   ✅ Real-time performance monitoring\n");
    
    Ok(())
}

// Helper functions for the showcase
fn new_spring_config(stiffness: f32, damping: f32) -> AnimationConfig {
    AnimationConfig {
        animation_type: "spring".to_string(),
        duration: 500,
        easing: "spring".to_string(),
        spring: Some(SpringConfig {
            stiffness,
            damping,
            initial_velocity: 0.0,
            mass: 1.0,
        }),
        ..Default::default()
    }
}

fn new_bezier_config(x1: f32, y1: f32, x2: f32, y2: f32) -> AnimationConfig {
    AnimationConfig {
        animation_type: "custom".to_string(),
        duration: 300,
        easing: format!("cubic-bezier({}, {}, {}, {})", x1, y1, x2, y2),
        ..Default::default()
    }
}