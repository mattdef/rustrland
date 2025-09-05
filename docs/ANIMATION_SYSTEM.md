# 🎬 Rustrland Advanced Animation System

**Production-ready window animation framework with comprehensive easing functions, physics simulation, and smooth 60fps interpolation. A complete rewrite offering far superior performance and capabilities compared to Pyprland.**

---

## 🚀 **Getting Started**

### **Run Live Demonstrations**
```bash
# Interactive animation showcase with real windows
cargo run --example animation

# Terminal-based easing visualization (no Hyprland required)
cargo run --example terminal_animation_visual

# Individual window tests
cargo run --example foot_floating_test
```

---

## 🏗️ **Architecture Overview**

The animation system is built on a modular architecture with clear separation of concerns:

### **Core Components**
- **`AnimationEngine`**: Core orchestration and 60fps animation loops
- **`WindowAnimator`**: High-level window animation management with Hyprland integration  
- **`EasingFunction`**: Comprehensive mathematical easing implementations (40+ functions)
- **`PropertyValue`**: Type-safe animation values with interpolation support
- **`Timeline`**: Keyframe-based animation sequences with loop support

---

## 🎯 **Animation Types**

### **1. Directional Slide Animations**
```toml
[scratchpads.terminal]
animation = "fromTop"
animation_config = { duration = 300, easing = "EaseOutCubic", offset = "100px" }
```

**Available directions:**
- `fromTop` / `fromBottom` - Vertical slide animations
- `fromLeft` / `fromRight` - Horizontal slide animations  
- `fromTopLeft` / `fromTopRight` / `fromBottomLeft` / `fromBottomRight` - Diagonal animations

### **2. Property-Based Animations**
```toml
[scratchpads.calculator]
animation = "fade"
animation_config = { duration = 400, easing = "EaseOut", opacity_from = 0.0 }
```

**Core animation types:**
- `fade` - Opacity transitions (0.0 to 1.0)
- `scale` - Size scaling with overshoot support
- `bounce` - Physics-based bounce animations

### **3. Multi-Property Animations**
Complex animations targeting multiple properties simultaneously:

```rust
let config = AnimationConfig {
    animation_type: "complex".to_string(),
    duration: 500,
    easing: EasingFunction::EaseOutBack,
    properties: Some(vec![
        AnimationPropertyConfig {
            property: "x".to_string(),
            from: PropertyValue::Pixels(0),
            to: PropertyValue::Pixels(400),
            easing: Some(EasingFunction::EaseOutCubic),
        },
        AnimationPropertyConfig {
            property: "opacity".to_string(),
            from: PropertyValue::Float(0.0),
            to: PropertyValue::Float(1.0),
            easing: Some(EasingFunction::EaseOut),
        },
    ]),
    ..Default::default()
};
```

---

## 🎨 **Comprehensive Easing Functions Library**

### **Standard CSS Easing**
- `Linear` - Constant speed
- `Ease` / `EaseIn` / `EaseOut` / `EaseInOut` - Standard CSS curves

### **Mathematical Curves**  
- **Sine**: `EaseInSine`, `EaseOutSine`, `EaseInOutSine`
- **Quadratic**: `EaseInQuad`, `EaseOutQuad`, `EaseInOutQuad`
- **Cubic**: `EaseInCubic`, `EaseOutCubic`, `EaseInOutCubic`
- **Quartic**: `EaseInQuart`, `EaseOutQuart`, `EaseInOutQuart`
- **Quintic**: `EaseInQuint`, `EaseOutQuint`, `EaseInOutQuint`
- **Exponential**: `EaseInExpo`, `EaseOutExpo`, `EaseInOutExpo`
- **Circular**: `EaseInCirc`, `EaseOutCirc`, `EaseInOutCirc`

### **Advanced Effects**
- **Back (Overshoot)**: `EaseInBack`, `EaseOutBack`, `EaseInOutBack` - True overshoot beyond target
- **Bounce**: `EaseInBounce`, `EaseOutBounce`, `EaseInOutBounce` - Ball-dropping physics
- **Elastic**: `EaseInElastic`, `EaseOutElastic`, `EaseInOutElastic` - Rubber band effect

### **Physics-Based**
- **Spring**: `Spring { stiffness, damping }` - Real damped oscillation physics
- **Custom Bezier**: `CubicBezier { x1, y1, x2, y2 }` - Custom cubic-bezier curves

### **Recommended Combinations**
- **Scratchpad entrance**: `EaseOutCubic` - Smooth deceleration
- **Attention-grabbing**: `EaseOutBack` - Overshoot effect
- **Playful interactions**: `EaseOutBounce` - Professional bounce
- **Physics simulation**: `Spring` - Realistic motion

---

## 🔧 **Usage Examples**

### **Basic WindowAnimator Setup**
```rust
use rustrland::animation::{WindowAnimator, AnimationConfig, EasingFunction};
use std::sync::Arc;

// Create animator
let mut animator = WindowAnimator::new();
animator.set_hyprland_client(Arc::new(client)).await;
animator.set_active_monitor(&monitor_info).await;

// Configure animation
let config = AnimationConfig {
    animation_type: "fromTop".to_string(),
    duration: 300,
    easing: EasingFunction::EaseOutCubic,
    offset: "150px".to_string(),
    ..Default::default()
};

// Show window with animation
let window = animator.show_window_with_animation(
    "foot",
    (400, 300),  // target position
    (800, 600),  // window size
    config
).await?;
```

### **Complex Multi-Property Animation**
```rust
let config = AnimationConfig {
    animation_type: "multi_property".to_string(),
    duration: 600,
    easing: EasingFunction::EaseOutBack,
    properties: Some(vec![
        // Slide in from right with overshoot
        AnimationPropertyConfig {
            property: "x".to_string(),
            from: PropertyValue::Pixels(1920),
            to: PropertyValue::Pixels(560),
            easing: Some(EasingFunction::EaseOutBack),
        },
        // Fade in smoothly
        AnimationPropertyConfig {
            property: "opacity".to_string(),
            from: PropertyValue::Float(0.0),
            to: PropertyValue::Float(1.0),
            easing: Some(EasingFunction::EaseOut),
        },
        // Scale with bounce
        AnimationPropertyConfig {
            property: "scale".to_string(),
            from: PropertyValue::Float(0.8),
            to: PropertyValue::Float(1.0),
            easing: Some(EasingFunction::EaseOutBounce),
        },
    ]),
    ..Default::default()
};
```

### **Spring Physics Animation**
```rust
let spring_config = AnimationConfig {
    animation_type: "fromLeft".to_string(),
    duration: 800, // Longer duration for spring settle
    easing: EasingFunction::Spring {
        stiffness: 300.0,  // Oscillation frequency
        damping: 30.0,     // Damping factor (higher = less bounce)
    },
    offset: "200px".to_string(),
    ..Default::default()
};
```

---

## ⚡ **Performance & Technical Features**

### **60fps Animation Engine**
- **Frame-perfect timing**: 16.67ms precision with adaptive frame rates
- **Concurrent animations**: Multiple windows animated simultaneously
- **Performance monitoring**: Real-time FPS tracking and adaptive quality
- **Memory efficient**: Zero-allocation hot paths in interpolation

### **Production-Ready Window Management**
- **Off-screen spawning**: Windows spawn directly off-screen to prevent visual artifacts
- **Intelligent positioning**: Automatic monitor-aware coordinate calculation
- **Style preservation**: Maintains Hyprland window decorations and styling
- **Error tolerance**: Robust handling of network hiccups and edge cases

### **Animation Property System**
```rust
pub enum PropertyValue {
    Pixels(i32),           // Window coordinates and dimensions
    Percentage(f32),       // Screen-relative positioning
    Float(f32),           // Opacity, scale factors, rotation
    Color(Color),         // RGBA color animations
    Transform(Transform), // Complex 2D transformations
    Vector2D/3D { x, y, z }, // Multi-dimensional values
}
```

### **Timeline System**
```rust
// Create complex keyframe animations
let timeline = TimelineBuilder::new(Duration::from_millis(1000))
    .keyframe(0.0, 0.0, None)
    .keyframe(0.3, 0.8, Some("ease-out"))
    .keyframe(0.7, 1.2, Some("ease-in-out"))
    .keyframe(1.0, 1.0, Some("ease-in"))
    .loop_count(Some(2))
    .direction(AnimationDirection::Alternate)
    .build();
```

---

## 📊 **Performance Guidelines**

### **Recommended Durations**
- **Scratchpad toggle**: 200-400ms (300ms optimal)
- **Window focus**: 100-200ms (immediate feedback) 
- **Workspace transitions**: 150-300ms (smooth context switching)
- **Spring animations**: 400-1000ms (allow settling time)
- **Complex multi-property**: 500-800ms (balanced smoothness)

### **Performance Targets**
- **Target FPS**: 60fps (16.67ms frame time)
- **Maximum concurrent animations**: 10+ windows simultaneously
- **Memory usage**: <2MB for full animation engine
- **CPU overhead**: <5% during active animations
- **Animation latency**: <50ms from trigger to first frame

### **Optimization Features**
- **Adaptive quality**: Automatic frame rate adjustment under load
- **Smart interpolation**: Optimized mathematical functions
- **Property caching**: Minimal allocations during animation loops
- **Async architecture**: Non-blocking animation execution

---

## 🎬 **Live Examples & Testing**

### **Interactive Animation Showcase**
```bash
cargo run --example animation
```

**Features:**
- ✅ Real-time menu-driven animation testing
- ✅ 10+ different animation types with live windows
- ✅ All easing functions demonstrated
- ✅ Performance monitoring and FPS display
- ✅ Window cleanup and proper resource management

### **Production Scratchpad Integration**
The animation system integrates seamlessly with Rustrland's scratchpad system:

```toml
[scratchpads.terminal]
class = "foot_toggle"
command = "foot --app-id foot_toggle"
animation = "fromTop"
animation_config = { 
    duration = 300, 
    easing = "EaseOutCubic", 
    offset = "100px" 
}

[scratchpads.calculator]
class = "gnome-calculator"
command = "gnome-calculator"
animation = "scale"
animation_config = { 
    duration = 400, 
    easing = "EaseOutBack", 
    scale_from = 0.8 
}
```

---

## 🔬 **Advanced Features**

### **Color Animations**
```rust
// Animate window border colors
let color_config = AnimationPropertyConfig {
    property: "border_color".to_string(),
    from: PropertyValue::Color(Color::new(1.0, 0.0, 0.0, 1.0)), // Red
    to: PropertyValue::Color(Color::new(0.0, 1.0, 0.0, 1.0)),   // Green
    easing: Some(EasingFunction::EaseInOut),
};
```

### **Transform Animations**
```rust
// Complex 2D transformations
let transform = Transform {
    translate_x: 100.0,
    translate_y: 50.0,
    scale_x: 1.2,
    scale_y: 1.2,
    rotation: 15.0,
    skew_x: 0.0,
    skew_y: 0.0,
};
```

### **Custom Easing Functions**
```rust
// Custom cubic-bezier curves
let custom_easing = EasingFunction::CubicBezier {
    x1: 0.68, y1: -0.55,
    x2: 0.265, y2: 1.55
};

// Spring physics with custom parameters
let spring_easing = EasingFunction::Spring {
    stiffness: 400.0,   // Higher = faster oscillation
    damping: 25.0,      // Lower = more bounce
};
```

---

## 🏆 **Rustrland vs Pyprland Animation Comparison**

| Feature | Pyprland | Rustrland |
|---------|----------|-----------|
| **Easing Functions** | 3-4 basic | **40+ advanced mathematical** |
| **Animation Types** | Linear/basic | **Physics, springs, overshoot, multi-property** |
| **Performance** | Python bottlenecks | **60fps Rust with adaptive quality** |
| **Window Spawning** | Visible artifacts | **Intelligent off-screen spawning** |
| **Concurrent Animations** | Limited/sequential | **10+ simultaneous animations** |
| **Overshoot Effects** | None | **True mathematical overshoot** |
| **Memory Safety** | Runtime errors possible | **Memory-safe Rust guarantees** |
| **Performance Monitoring** | None | **Real-time FPS tracking & adaptive quality** |
| **Property System** | Basic positioning | **Colors, transforms, multi-dimensional values** |
| **Timeline Support** | None | **Keyframe-based complex sequences** |
| **Physics Simulation** | None | **Real damped spring oscillation** |

---

## 🔮 **Future Roadmap**

### **Planned Enhancements**
- **GPU acceleration**: Hardware-accelerated animations via Vulkan/OpenGL
- **3D transforms**: Full 3D transformation support with perspective
- **Particle effects**: Window trails and particle systems
- **Audio sync**: Beat-synchronized animations
- **Gesture integration**: Touch/mouse gesture-driven animations

### **Advanced Animation Types**
- **Path animations**: Bezier curve following
- **Morphing**: Shape transformation animations
- **Shader effects**: Custom GPU shader animations
- **Physics constraints**: Collision detection and response

---

**🦀 Rustrland's animation system represents the cutting edge of window management UX. With mathematically precise easing functions, real physics simulation, and production-ready 60fps performance, it provides smooth, professional animations that make traditional window managers feel primitive.**

**Every animation runs with frame-perfect timing, real-time performance monitoring, and adaptive quality control. This is the future of modern desktop environments.**