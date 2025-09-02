# Hyprland Window Styling Guide

This document provides comprehensive information about styling windows in Hyprland, both through native configuration and the hyprland-rs Rust library.

## Table of Contents

- [Overview](#overview)
- [Native Hyprland Window Rules](#native-hyprland-window-rules)
- [Window Rule Syntax](#window-rule-syntax)
- [Styling Rules](#styling-rules)
- [Hyprland-rs API](#hyprland-rs-api)
- [Common Issues and Solutions](#common-issues-and-solutions)
- [Best Practices](#best-practices)

## Overview

Hyprland provides multiple ways to style windows:

1. **Global styling** via `decoration` section in configuration
2. **Window-specific styling** via `windowrule` and `windowrulev2`
3. **Dynamic styling** via `hyprctl` commands
4. **Programmatic styling** via hyprland-rs Rust library

## Native Hyprland Window Rules

### Window Rule Types

Hyprland supports two types of window rules:

- **`windowrule`** (legacy): Simple syntax with limited matching
- **`windowrulev2`** (recommended): Advanced syntax with powerful matching capabilities

### Basic Syntax

```conf
# windowrule = RULE, WINDOW_IDENTIFIER
windowrule = opacity 0.8,^(kitty)$

# windowrulev2 = RULE, PARAMETERS
windowrulev2 = opacity 0.8 0.6,class:^(kitty)$
```

⚠️ **Important Notes:**
- Window rules are **case sensitive** (`firefox` ≠ `Firefox`)
- Rules are evaluated **top to bottom** (order matters)
- RegEx patterns must **fully match** window values (as of v0.46.0)
- Hyprland uses Google's RE2 for RegEx parsing

## Window Rule Syntax

### Window Matching Fields

`windowrulev2` supports these matching parameters:

| Field | Description | Type | Example |
|-------|-------------|------|---------|
| `class` | Window class | RegEx | `class:^(firefox)$` |
| `title` | Window title | RegEx | `title:^(.*YouTube.*)$` |
| `initialClass` | Initial class at launch | RegEx | `initialClass:^(code)$` |
| `initialTitle` | Initial title at launch | RegEx | `initialTitle:^(New Tab)$` |
| `xwayland` | XWayland status | 0/1 | `xwayland:1` |
| `floating` | Floating status | 0/1 | `floating:1` |
| `fullscreen` | Fullscreen status | 0/1 | `fullscreen:1` |
| `pinned` | Pinned status | 0/1 | `pinned:1` |
| `focus` | Focus status | 0/1 | `focus:1` |
| `workspace` | Workspace ID/name | ID/name | `workspace:2` |
| `onworkspace` | Window count on workspace | int | `onworkspace:>5` |

### Rule Types: Static vs Dynamic

- **Static rules**: Evaluated once at window open
- **Dynamic rules**: Re-evaluated when matching property changes

Common dynamic rules: `opacity`, `bordercolor`, `bordersize`

## Styling Rules

### Border Styling

#### Border Size
```conf
# Set border size to 2px for all floating windows
windowrulev2 = bordersize 2,floating:1

# Remove borders from unfocused windows
windowrulev2 = bordersize 0,focus:0

# Set border size based on window class
windowrulev2 = bordersize 3,class:^(firefox)$
```

#### Border Color
```conf
# Single color (applies to both active/inactive)
windowrulev2 = bordercolor rgb(FF0000),class:^(firefox)$

# Active and inactive colors
windowrulev2 = bordercolor rgb(00FF00) rgb(FF0000),focus:1

# RGBA with transparency
windowrulev2 = bordercolor rgba(255,0,0,0.8) rgba(100,100,100,0.5),title:^(.*Terminal.*)$

# Hex format
windowrulev2 = bordercolor 0xFF00FF00 0x80FF0000,class:^(kitty)$
```

#### Dynamic Border Colors
```conf
# Red border when fullscreen
windowrulev2 = bordercolor rgb(FF0000),fullscreen:1

# Green border for focused windows
windowrulev2 = bordercolor rgb(00FF00),focus:1

# Different colors per workspace
windowrulev2 = bordercolor rgb(0000FF),workspace:1
windowrulev2 = bordercolor rgb(00FF00),workspace:2
```

### Shadow Styling

#### Shadow Rules
```conf
# Disable shadows for specific windows
windowrulev2 = noshadow,class:^(firefox)$

# Enable shadows only for floating windows
windowrulev2 = shadow,floating:1
```

#### Global Shadow Configuration
```conf
decoration {
    drop_shadow = true
    shadow_range = 30
    shadow_render_power = 4
    shadow_offset = 0 5
    col.shadow = rgba(00000099)
    shadow_ignore_window = true
}
```

### Opacity Styling

#### Basic Opacity
```conf
# Single opacity value
windowrulev2 = opacity 0.8,class:^(kitty)$

# Active and inactive opacity
windowrulev2 = opacity 1.0 0.8,class:^(code)$

# Active, inactive, and fullscreen opacity
windowrulev2 = opacity 1.0 0.8 0.9,class:^(firefox)$
```

⚠️ **Opacity Important Notes:**
- Opacity values are **multiplicative** (0.5 × 0.5 = 0.25 total)
- Values > 1.0 can cause graphical glitches
- Use `override` to ignore global opacity settings:
  ```conf
  windowrulev2 = opacity 0.8 override,class:^(kitty)$
  ```

### Other Styling Rules

#### Rounding
```conf
windowrulev2 = rounding 10,class:^(kitty)$
windowrulev2 = rounding 0,fullscreen:1
```

#### Blur
```conf
windowrulev2 = noblur,class:^(firefox)$
windowrulev2 = blur,floating:1
```

#### Animation
```conf
windowrulev2 = animation popin,class:^(kitty)$
windowrulev2 = animation slide,workspace:special
```

## Hyprland-rs API

### Keyword API

The hyprland-rs library provides the `Keyword` API for getting/setting configuration values:

```rust
use hyprland::keyword::{Keyword, OptionValue};

// Get a configuration value
let border_size = Keyword::get("general:border_size")?;
match border_size.value {
    OptionValue::Int(size) => println!("Border size: {}", size),
    OptionValue::String(s) => println!("Border size: {}", s),
    OptionValue::Float(f) => println!("Border size: {}", f),
}

// Set a configuration value
Keyword::set("general:border_size", "2")?;
```

### OptionValue Types

The `OptionValue` enum supports three variants:

- `Int(i64)`: 64-bit integers
- `Float(f64)`: 64-bit floating-point numbers  
- `String(String)`: String values

### Color Handling

Color values in hyprland-rs are typically returned as:

1. **String format**: `"rgba(255,0,0,255)"` or `"rgb(255,0,0)"`
2. **Integer format**: Raw color values as `i64`
3. **Custom format**: Complex color definitions (gradients, etc.)

#### Color Conversion Example

```rust
fn parse_color_from_hyprctl(output: &str) -> Option<String> {
    // Parse "custom type: aa7c7674 0deg" format
    for line in output.lines() {
        if line.contains("custom type:") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            for part in parts {
                if part.len() == 8 && part.chars().all(|c| c.is_ascii_hexdigit()) {
                    return Some(hex_to_rgba(part));
                }
            }
        }
    }
    None
}

fn hex_to_rgba(hex: &str) -> String {
    if hex.len() == 8 {
        // Format: AARRGGBB
        if let Ok(color) = u32::from_str_radix(hex, 16) {
            let a = (color >> 24) & 0xFF;
            let r = (color >> 16) & 0xFF;
            let g = (color >> 8) & 0xFF;
            let b = color & 0xFF;
            return format!("rgba({}, {}, {}, {})", r, g, b, a);
        }
    }
    format!("rgba({})", hex)
}
```

### Configuration Retrieval Limitations

The hyprland-rs `Keyword::get()` has limitations:

- **Color values**: May not parse complex color formats correctly
- **Custom types**: Advanced configurations might not be accessible
- **Fallback needed**: Use `hyprctl` commands for reliable color retrieval

#### Reliable Color Retrieval

```rust
async fn get_border_color_reliable() -> Result<String> {
    let output = tokio::process::Command::new("hyprctl")
        .arg("getoption")
        .arg("general:col.active_border")
        .output()
        .await?;
    
    let output_str = String::from_utf8_lossy(&output.stdout);
    extract_color_from_output(&output_str)
}
```

## Common Issues and Solutions

### 1. Style "Flash" on Window Creation

**Problem**: Window appears with default style, then changes to correct style

**Cause**: Style applied after window creation via `windowrulev2` command

**Solution**: Apply style during spawn with launch rules

```rust
// Bad: Apply style after spawn
client.spawn_app("firefox").await?;
// Window appears with default style
apply_window_rules(window_address).await?;

// Good: Apply style during spawn
let spawn_rule = format!(
    "[float;bordersize 2;bordercolor {} {};pin] firefox",
    active_color, inactive_color
);
client.spawn_app(&spawn_rule).await?;
```

### 2. Inconsistent Color Retrieval

**Problem**: `Keyword::get()` fails to retrieve color values

**Solution**: Use `hyprctl` command as fallback

```rust
async fn get_style_safe() -> HyprlandStyle {
    let mut style = HyprlandStyle::default();
    
    // Try Keyword API first
    if let Ok(border) = Keyword::get("general:col.active_border") {
        // Handle successful retrieval
    } else {
        // Fallback to hyprctl
        let output = Command::new("hyprctl")
            .arg("getoption")
            .arg("general:col.active_border")
            .output()
            .await?;
        style.active_border = parse_hyprctl_color(&output.stdout)?;
    }
    
    style
}
```

### 3. Opacity Multiplication Issues

**Problem**: Opacity becomes too low due to multiplication

**Solution**: Use `override` flag or calculate carefully

```conf
# Problem: 0.5 (global) × 0.5 (rule) = 0.25 total
windowrulev2 = opacity 0.5,class:^(kitty)$

# Solution: Override global opacity
windowrulev2 = opacity 0.8 override,class:^(kitty)$
```

### 4. Shadow Configuration Conflicts

**Problem**: Shadows don't appear as expected with borders

**Cause**: Shadow rendering changed from inside borders to outside

**Solution**: Configure shadows and borders separately

```conf
decoration {
    drop_shadow = true
    shadow_ignore_window = true  # Ignore window-specific shadow rules
    shadow_range = 20
    col.shadow = rgba(00000099)
}

# Separate window-specific rules
windowrulev2 = bordersize 2,class:^(kitty)$
windowrulev2 = noshadow,class:^(no-shadow-app)$
```

### 5. RegEx Pattern Matching

**Problem**: Window rules not applying due to incorrect RegEx

**Solution**: Use precise patterns and test with `hyprctl clients`

```bash
# Check window properties
hyprctl clients | grep -A 5 "class:"

# Test patterns
windowrulev2 = bordercolor rgb(FF0000),class:^(exact-match)$
windowrulev2 = bordercolor rgb(00FF00),class:.*partial.*
windowrulev2 = bordercolor rgb(0000FF),negative:unwanted-class
```

## Best Practices

### 1. Rule Organization

```conf
# Group rules by purpose
# === OPACITY RULES ===
windowrulev2 = opacity 0.9 0.7,class:^(code)$
windowrulev2 = opacity 1.0 override,class:^(firefox)$

# === BORDER RULES ===
windowrulev2 = bordercolor rgb(00FF00),focus:1
windowrulev2 = bordercolor rgb(555555),focus:0

# === FLOATING RULES ===
windowrulev2 = float,class:^(calculator)$
windowrulev2 = size 400 300,class:^(calculator)$
```

### 2. Performance Considerations

- Use **static rules** when possible (evaluated once)
- Minimize **dynamic rules** (re-evaluated frequently)
- Group related rules for the same window

### 3. Debugging Window Rules

```bash
# Get window information
hyprctl clients

# Test rule syntax
hyprctl keyword windowrulev2 "bordercolor rgb(FF0000),class:^(test)$"

# Monitor rule evaluation
hyprctl monitors  # Check active windows
```

### 4. Color Format Consistency

```rust
// Standardize on one color format throughout your application
const ACTIVE_BORDER: &str = "rgba(124, 118, 116, 170)";
const INACTIVE_BORDER: &str = "rgba(204, 197, 195, 170)";

// Avoid mixing formats
// ❌ Don't mix: "rgb(255,0,0)" and "0xFF0000FF"
// ✅ Use consistently: "rgba(255, 0, 0, 255)"
```

### 5. Configuration Management

```rust
#[derive(Debug, Clone)]
pub struct WindowStyle {
    pub border_size: i32,
    pub active_border_color: String,
    pub inactive_border_color: String,
    pub shadow_enabled: bool,
    pub shadow_color: String,
    pub opacity: f32,
}

impl WindowStyle {
    pub fn to_spawn_rule(&self, position: (i32, i32), size: (i32, i32)) -> String {
        let shadow_rule = if self.shadow_enabled { "" } else { "noshadow;" };
        format!(
            "[float;bordersize {};bordercolor {} {};{}move {} {};size {} {}]",
            self.border_size,
            self.active_border_color,
            self.inactive_border_color,
            shadow_rule,
            position.0, position.1,
            size.0, size.1
        )
    }
}
```

## Conclusion

Hyprland provides powerful and flexible window styling capabilities through multiple interfaces. For dynamic applications like Rustrland, combining spawn-time rule application with hyprctl fallbacks provides the most reliable styling experience.

Key takeaways:
- Apply styles at window spawn to prevent visual artifacts
- Use hyprctl commands for reliable configuration retrieval
- Test RegEx patterns thoroughly
- Organize rules logically and document complex configurations
- Handle color format conversion consistently

For more information, refer to:
- [Hyprland Wiki - Window Rules](https://wiki.hypr.land/Configuring/Window-Rules/)
- [Hyprland Wiki - Variables](https://wiki.hypr.land/Configuring/Variables/)  
- [hyprland-rs Documentation](https://docs.rs/hyprland/)