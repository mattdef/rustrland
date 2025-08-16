# Toggle Special Plugin

**Status**: ✅ Still in development | **Tests**: Integrated

Toggle special workspace with smooth animations and intelligent window management. Provides quick access to special workspace with enhanced functionality beyond standard Hyprland special workspace handling.

## Features

- **Special Workspace Toggle**: Quick toggle access to Hyprland's special workspace
- **Smooth Animations**: Hardware-accelerated animations for show/hide transitions
- **Window Management**: Intelligent handling of windows on special workspace
- **Multi-Monitor Support**: Special workspace management across multiple monitors
- **Focus Management**: Smart focus handling when toggling special workspace
- **Animation Customization**: Configurable animation types and timing

## Configuration

### Basic Configuration

```toml
[toggle_special]
# Enable toggle special plugin
enabled = true

# Animation settings
animation = "fromTop"            # Animation type: "fromTop", "fromBottom", "fromLeft", "fromRight", "fade", "scale"
animation_duration = 300         # Animation duration in milliseconds
animation_easing = "easeOut"     # Easing function: "linear", "easeIn", "easeOut", "easeInOut"

# Behavior settings
auto_hide_delay = 0              # Auto-hide after delay (0 = no auto-hide)
focus_on_toggle = true           # Focus special workspace when toggling
restore_focus_on_hide = true     # Restore previous focus when hiding
```

### Advanced Configuration

```toml
[toggle_special]
# Special workspace management
special_workspace_name = "special"    # Name of special workspace
create_if_missing = true              # Create special workspace if it doesn't exist
preserve_layout = true                # Preserve window layout on special workspace

# Multi-monitor behavior
monitor_behavior = "current"          # "current", "primary", "all", "follow_cursor"
per_monitor_special = false           # Separate special workspace per monitor
sync_across_monitors = true           # Sync special workspace across monitors

# Window management
move_new_windows = true               # Move new windows to special workspace when active
smart_window_placement = true         # Intelligent window placement on special workspace
window_grouping = true                # Group related windows together

# Advanced animation settings
[toggle_special.animations]
show_animation = {
    type = "fromTop",
    duration = 300,
    easing = "easeOut",
    opacity_start = 0.0,
    scale_start = 0.9
}

hide_animation = {
    type = "toTop", 
    duration = 250,
    easing = "easeIn",
    opacity_end = 0.0,
    scale_end = 0.9
}

# Performance and behavior
[toggle_special.performance]
hardware_acceleration = true         # Use GPU acceleration for animations
async_operations = true              # Asynchronous special workspace operations
cache_window_info = true             # Cache window information for performance
```

## Commands

### Basic Toggle Commands

```bash
# Basic special workspace toggle
rustr toggle-special               # Toggle special workspace visibility
rustr special toggle               # Alternative syntax
rustr special show                 # Show special workspace
rustr special hide                 # Hide special workspace

# Status and information
rustr special status               # Show special workspace status
rustr special list                 # List windows on special workspace
rustr special info                 # Detailed special workspace information
```

### Window Management

```bash
# Move windows to/from special workspace
rustr special move-here            # Move current window to special workspace
rustr special move-from            # Move current window from special workspace
rustr special move-window <id>     # Move specific window to special workspace

# Focus management
rustr special focus                # Focus special workspace without showing
rustr special focus-window <id>    # Focus specific window on special workspace
rustr special focus-last           # Focus last used window on special workspace
```

### Advanced Commands

```bash
# Special workspace management
rustr special clear                # Clear all windows from special workspace
rustr special organize            # Organize windows on special workspace
rustr special save-layout         # Save current special workspace layout
rustr special restore-layout      # Restore saved layout

# Animation control
rustr special set-animation fade   # Change animation type
rustr special test-animation       # Test current animation settings
rustr special no-animation         # Toggle without animation (one-time)

# Configuration management
rustr special reload               # Reload plugin configuration
rustr special reset                # Reset special workspace to defaults
```

## Keybindings

Add to your `~/.config/hypr/hyprland.conf`:

### Basic Toggle Controls

```bash
# Main toggle keybinding
bind = SUPER, S, exec, rustr toggle-special        # Super + S
bind = SUPER_SHIFT, S, exec, rustr special show    # Force show
bind = SUPER_CTRL, S, exec, rustr special hide     # Force hide

# Alternative access
bind = SUPER, grave, exec, rustr toggle-special    # Super + ` (backtick)
bind = SUPER, F12, exec, rustr toggle-special      # F12 for quick access
```

### Window Management

```bash
# Move windows to special workspace
bind = SUPER_SHIFT, grave, exec, rustr special move-here    # Move current window
bind = SUPER_ALT, S, exec, rustr special move-here         # Alternative binding

# Focus management
bind = SUPER_CTRL_SHIFT, S, exec, rustr special focus      # Focus without showing
bind = SUPER_ALT_SHIFT, S, exec, rustr special focus-last  # Focus last window
```

### Advanced Controls

```bash
# Special workspace management
bind = SUPER_CTRL, Delete, exec, rustr special clear       # Clear special workspace
bind = SUPER_SHIFT, F12, exec, rustr special organize      # Organize windows

# Quick access patterns
bind = SUPER, KP_Multiply, exec, rustr toggle-special      # Keypad * 
bind = SUPER, Menu, exec, rustr toggle-special             # Menu key
```

## Animation Types

### Slide Animations

```toml
[toggle_special]
# Slide from edges
animation = "fromTop"        # Slide from top edge
animation = "fromBottom"     # Slide from bottom edge  
animation = "fromLeft"       # Slide from left edge
animation = "fromRight"      # Slide from right edge

# Slide with customization
[toggle_special.animations.show_animation]
type = "fromTop"
duration = 400
easing = "easeOutBounce"     # Bouncy slide effect
distance = 100               # Distance to slide (pixels)
```

### Transform Animations

```toml
[toggle_special]
# Scale and fade effects
animation = "scale"          # Scale up/down
animation = "fade"           # Fade in/out
animation = "scaleAndFade"   # Combined scale and fade

# Advanced transform settings
[toggle_special.animations.show_animation]
type = "scale"
duration = 300
easing = "easeOutElastic"
scale_start = 0.3            # Start at 30% size
scale_end = 1.0              # End at 100% size
opacity_start = 0.0          # Start transparent
opacity_end = 1.0            # End opaque
```

### Custom Animations

```toml
# Complex custom animation
[toggle_special.animations.show_animation]
type = "custom"
duration = 500
easing = "easeInOutCubic"
keyframes = [
    { time = 0.0, opacity = 0.0, scale = 0.5, y_offset = -100 },
    { time = 0.6, opacity = 0.8, scale = 1.1, y_offset = 10 },
    { time = 1.0, opacity = 1.0, scale = 1.0, y_offset = 0 }
]
```

## Multi-Monitor Behavior

### Monitor Strategies

```toml
[toggle_special]
# Monitor targeting strategies
monitor_behavior = "current"      # Show on current monitor
monitor_behavior = "primary"      # Always show on primary monitor
monitor_behavior = "all"          # Show on all monitors
monitor_behavior = "follow_cursor" # Show on monitor with cursor

# Per-monitor configuration
per_monitor_special = true        # Separate special workspace per monitor
```

### Multi-Monitor Examples

```toml
# Different special workspaces per monitor
[toggle_special.monitors."DP-1"]
special_workspace_name = "special-main"
animation = "fromTop"
focus_on_toggle = true

[toggle_special.monitors."DP-2"] 
special_workspace_name = "special-secondary"
animation = "fromLeft"
focus_on_toggle = false

# Synchronized special workspace
[toggle_special]
sync_across_monitors = true       # Same special workspace on all monitors
global_toggle = true              # One toggle affects all monitors
```

## Integration with Other Plugins

### Scratchpads Integration

```toml
# Coordinate with scratchpads
[toggle_special.integration.scratchpads]
hide_scratchpads_on_show = true   # Hide scratchpads when showing special
restore_scratchpads_on_hide = true # Restore scratchpads when hiding special
```

### Expose Integration

```bash
# Show expose for special workspace
rustr expose special              # Show expose view of special workspace
bind = SUPER_SHIFT, TAB, exec, rustr expose special
```

### Workspaces Integration

```toml
# Special workspace and regular workspaces
[toggle_special.integration.workspaces]
remember_last_workspace = true    # Remember workspace before showing special
auto_return_to_workspace = true   # Return to previous workspace on hide
```

## Window Management Features

### Automatic Window Placement

```toml
[toggle_special.window_management]
# Automatic window placement rules
auto_placement_rules = [
    { app = "calculator", position = "top_right", size = "300x400" },
    { app = "notes", position = "center", size = "50% 60%" },
    { class = "floating", position = "bottom_left", size = "400x300" }
]

# Window organization
auto_organize = true              # Automatically organize windows
organization_style = "grid"      # "grid", "stack", "tiles", "cascade"
max_windows_per_row = 3          # Maximum windows per row in grid
```

### Window Grouping

```toml
[toggle_special.grouping]
# Group related windows
enable_grouping = true
group_by_app = true              # Group windows from same application
group_by_class = true            # Group windows with same class
group_by_workspace_origin = true # Group by original workspace

# Group behavior
group_animation_delay = 50       # Delay between animating windows in group
group_focus_behavior = "cycle"   # "cycle", "stack", "spread"
```

## Use Cases and Examples

### Quick Calculator

```toml
[toggle_special]
# Quick access calculator setup
auto_launch_apps = ["gnome-calculator"]
auto_launch_delay = 100          # Launch after showing special workspace

[toggle_special.window_management.auto_placement_rules]
calculator = { 
    app = "gnome-calculator", 
    position = "center", 
    size = "300x400",
    always_on_top = true
}
```

### Note-Taking Workspace

```toml
[toggle_special]
# Note-taking special workspace
special_workspace_name = "notes"
auto_launch_apps = ["obsidian", "xournalpp"]

# Organize note-taking tools
[toggle_special.window_management]
organization_style = "tiles"
tile_layout = "vertical_split"   # Split vertically for text + drawing
```

### System Monitoring

```bash
# Launch system monitors on special workspace
rustr special show
rustr special move-here &
htop &  # Process monitor
iotop & # I/O monitor
nethogs & # Network monitor
```

### Development Scratch Space

```toml
[toggle_special]
# Development scratch workspace
auto_launch_apps = ["code --new-window", "gnome-terminal"]

[toggle_special.window_management.auto_placement_rules]
code_scratch = {
    app = "code",
    position = "left",
    size = "70% 100%"
}
terminal_scratch = {
    app = "gnome-terminal", 
    position = "right",
    size = "30% 100%"
}
```

## Performance Optimization

### Animation Performance

```toml
[toggle_special.performance]
# Optimize animations for performance
hardware_acceleration = true     # Use GPU when available
animation_quality = "medium"     # "low", "medium", "high"
vsync_aware = true               # Sync with display refresh rate
reduce_animations_on_battery = true # Reduce effects on battery power

# Memory management
cache_animation_frames = true    # Cache pre-rendered animation frames
max_cache_size = 32              # Maximum cache size in MB
cleanup_cache_interval = 300    # Cache cleanup interval in seconds
```

### Window Management Performance

```toml
[toggle_special.performance.window_management]
# Efficient window operations
batch_window_operations = true  # Batch multiple window operations
async_window_placement = true   # Asynchronous window placement
lazy_window_loading = true      # Load window information on demand

# Resource limits
max_windows_on_special = 20     # Limit windows on special workspace
window_info_cache_size = 100    # Cache size for window information
```

## Troubleshooting

### Common Issues

**Special workspace not showing:**
```bash
# Check plugin status
rustr special status

# Test without animation
rustr special no-animation

# Check Hyprland special workspace
hyprctl dispatch togglespecialworkspace
```

**Animation issues:**
```toml
[toggle_special]
hardware_acceleration = false   # Disable if causing problems
animation_duration = 100        # Reduce duration
animation = "fade"              # Use simpler animation
```

**Focus problems:**
```toml
[toggle_special]
focus_on_toggle = false         # Disable automatic focus
restore_focus_on_hide = false   # Don't restore focus
```

### Debug Commands

```bash
# Debug special workspace functionality
rustr special status            # Show detailed status
rustr special info              # Show configuration info
rustr special test-animation    # Test animation without showing

# Hyprland integration debugging
rustr special hyprland-status   # Check Hyprland special workspace status
rustr special window-debug      # Debug window management
```

## Migration and Compatibility

### From Standard Hyprland Special Workspace

Rustrland's toggle special plugin enhances the standard Hyprland special workspace with:
- Smooth animations
- Better window management
- Multi-monitor support
- Configurable behavior

### Migration Steps

1. **Replace Keybindings**: Replace `hyprctl dispatch togglespecialworkspace` with `rustr toggle-special`
2. **Add Configuration**: Add `[toggle_special]` configuration section
3. **Test Functionality**: Verify enhanced features work as expected

## Best Practices

### Configuration Best Practices

1. **Start Simple**: Begin with basic toggle functionality before adding complex features
2. **Test Animations**: Test different animation types to find what works best
3. **Monitor Performance**: Check animation performance on your hardware
4. **Use Appropriate Keybindings**: Choose convenient and memorable keybindings

### Usage Tips

1. **Quick Access**: Use toggle special for frequently accessed utilities
2. **Temporary Workspaces**: Use for temporary work that doesn't belong in regular workspaces
3. **System Tools**: Great for system monitoring and administrative tools
4. **Scratch Work**: Perfect for quick notes, calculations, and temporary tasks

### Performance Tips

1. **Limit Windows**: Don't overload special workspace with too many windows
2. **Optimize Animations**: Choose appropriate animation settings for your hardware
3. **Cache Settings**: Enable caching for better performance with many windows
4. **Hardware Acceleration**: Use GPU acceleration when available