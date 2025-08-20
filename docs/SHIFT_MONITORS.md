# Shift Monitors Plugin

**Status**: ✅ Still in development | **Tests**: Integrated

Shift workspaces between monitors with configurable direction and intelligent workspace management. Provides seamless workspace migration across multiple monitors with smooth transitions.

## Features

- **Bi-directional Shifting**: Move workspaces forward or backward between monitors
- **Workspace Preservation**: Maintains workspace content during shifts
- **Monitor Detection**: Automatically detects available monitors
- **Smooth Transitions**: Hardware-accelerated workspace transitions
- **Direction Control**: Configure shift direction (clockwise/counterclockwise)
- **Workspace Mapping**: Intelligent workspace-to-monitor mapping
- **Focus Following**: Maintain focus during workspace shifts

## Configuration

### Basic Configuration

```toml
[shift_monitors]
# Enable shift monitors plugin
enabled = true

# Default shift direction ("+1" or "-1")
default_direction = "+1"

# Animation duration in milliseconds
animation_duration = 250
```

## Commands

### Basic Shift Commands

```bash
# Basic workspace shifting
rustr shift-monitors +1         # Shift workspaces forward (right/down)
rustr shift-monitors -1         # Shift workspaces backward (left/up)
rustr shift-monitors            # Use default direction

# Alternative syntax
rustr shift-monitors forward    # Same as +1
rustr shift-monitors backward   # Same as -1
rustr shift-monitors next       # Same as +1
rustr shift-monitors prev       # Same as -1
```

### Advanced Commands

```bash
# Direction control
rustr shift-monitors set-direction +1    # Set default direction to forward
rustr shift-monitors set-direction -1    # Set default direction to backward
rustr shift-monitors toggle-direction    # Toggle default direction

# Specific monitor targeting
rustr shift-monitors to-monitor DP-2     # Shift current workspace to specific monitor
rustr shift-monitors from-monitor DP-1   # Shift workspace from specific monitor

# Workspace-specific operations
rustr shift-monitors workspace 3 +1      # Shift specific workspace forward
rustr shift-monitors workspace current +1 # Shift current workspace forward
rustr shift-monitors all-workspaces +1   # Shift all workspaces forward
```

### Monitor Management

```bash
# Monitor information
rustr shift-monitors list-monitors       # List available monitors in order
rustr shift-monitors monitor-order       # Show current monitor order
rustr shift-monitors set-order "DP-1,DP-2,HDMI-1"  # Set custom monitor order

# Status and configuration
rustr shift-monitors status              # Show plugin status
rustr shift-monitors config              # Show current configuration
rustr shift-monitors reload              # Reload configuration
```

## Keybindings

Add to your `~/.config/hypr/hyprland.conf`:

### Basic Shift Controls

```bash
# Main shift controls
bind = SUPER_SHIFT, Right, exec, rustr shift-monitors +1   # Shift right/forward
bind = SUPER_SHIFT, Left, exec, rustr shift-monitors -1    # Shift left/backward

# Alternative controls
bind = SUPER_ALT, bracketright, exec, rustr shift-monitors +1  # ] key
bind = SUPER_ALT, bracketleft, exec, rustr shift-monitors -1   # [ key

# Quick access
bind = SUPER_CTRL_SHIFT, M, exec, rustr shift-monitors        # Use default direction
```

### Advanced Keybindings

```bash
# Direction control
bind = SUPER_SHIFT, D, exec, rustr shift-monitors toggle-direction

# Monitor-specific shifts
bind = SUPER_SHIFT, 1, exec, rustr shift-monitors to-monitor DP-1
bind = SUPER_SHIFT, 2, exec, rustr shift-monitors to-monitor DP-2
bind = SUPER_SHIFT, 3, exec, rustr shift-monitors to-monitor HDMI-1

# Workspace-specific shifts
bind = SUPER_CTRL, Right, exec, rustr shift-monitors workspace current +1
bind = SUPER_CTRL, Left, exec, rustr shift-monitors workspace current -1

# Emergency controls
bind = SUPER_CTRL_ALT, R, exec, rustr shift-monitors all-workspaces +1  # Reset all
```

## Shift Modes

### Circular Mode (Default)

Workspaces shift in a circular pattern, wrapping around from last to first monitor:

```toml
[shift_monitors]
shift_mode = "circular"
wrap_around = true

# Monitor order: DP-1 → DP-2 → HDMI-1 → DP-1 (wraps around)
```

**Example**: With monitors DP-1, DP-2, HDMI-1:
- Shift +1: DP-1 → DP-2 → HDMI-1 → DP-1
- Shift -1: DP-1 → HDMI-1 → DP-2 → DP-1

### Linear Mode

Workspaces shift linearly without wrapping:

```toml
[shift_monitors]
shift_mode = "linear"
wrap_around = false

# Monitor order: DP-1 → DP-2 → HDMI-1 (stops at ends)
```

**Example**: With monitors DP-1, DP-2, HDMI-1:
- Shift +1: DP-1 → DP-2 → HDMI-1 (stops)
- Shift -1: HDMI-1 → DP-2 → DP-1 (stops)

### Ping-Pong Mode

Workspaces shift back and forth, reversing direction at ends:

```toml
[shift_monitors]
shift_mode = "ping_pong"
auto_reverse = true

# Direction reverses automatically at ends
```

**Example**: With monitors DP-1, DP-2, HDMI-1:
- Forward: DP-1 → DP-2 → HDMI-1 → DP-2 → DP-1 → ...
- Direction automatically reverses at ends

## Workspace Handling

### Workspace Preservation

```toml
[shift_monitors.workspace_handling]
# Preserve workspace numbers during shift
preserve_workspace_numbers = true

# Workspace content handling
preserve_window_layout = true      # Maintain window positions
preserve_window_focus = true       # Maintain focused window
preserve_workspace_state = true    # Maintain workspace state (visible/hidden)

# Window management during shift
move_floating_windows = true       # Move floating windows with workspace
move_tiled_windows = true          # Move tiled windows with workspace
preserve_window_properties = true  # Maintain window properties
```

### Empty Workspace Behavior

```toml
[shift_monitors]
# How to handle empty workspaces during shift
empty_workspace_behavior = "skip"

# Options:
# "skip" - Skip empty workspaces during shift
# "include" - Include empty workspaces in shift
# "create" - Create empty workspaces as needed
# "remove" - Remove empty workspaces after shift
```

### Workspace Creation

```toml
[shift_monitors.workspace_creation]
# Automatic workspace creation
auto_create_workspaces = true
create_on_demand = true            # Create workspaces when needed
default_workspace_layout = "tiled" # Default layout for new workspaces

# Workspace templates
use_templates = true
template_source = "similar"        # Use similar workspace as template
```

## Monitor Order and Detection

### Monitor Order Configuration

```toml
[shift_monitors.monitors]
# Explicit monitor order (left to right, top to bottom)
order = ["DP-1", "DP-2", "HDMI-1"]

# Automatic detection options
auto_detect_order = true           # Auto-detect monitor physical arrangement
prefer_horizontal = true           # Prefer horizontal arrangement
fallback_order = "alphabetical"    # Fallback ordering method

# Monitor grouping
monitor_groups = [
    ["DP-1", "DP-2"],              # Primary group
    ["HDMI-1"]                     # Secondary group
]
group_shift_mode = "within_group"  # "within_group", "between_groups", "global"
```

### Dynamic Monitor Detection

```toml
[shift_monitors.detection]
# Automatic monitor detection
auto_detect_changes = true         # Detect monitor connects/disconnects
update_order_on_change = true      # Update order when monitors change
rebalance_on_change = true         # Rebalance workspaces when monitors change

# Detection settings
detection_delay = 1000             # Delay after monitor change (ms)
retry_attempts = 3                 # Retry attempts for monitor detection
retry_delay = 500                  # Delay between retry attempts (ms)
```

## Visual Feedback and Animations

### Transition Animations

```toml
[shift_monitors.animations.transitions]
# Slide transition
slide_transition = {
    type = "slide",
    direction = "horizontal",       # "horizontal", "vertical", "auto"
    duration = 300,
    easing = "easeOut"
}

# Fade transition
fade_transition = {
    type = "fade",
    duration = 250,
    overlap = 100                   # Overlap duration for smooth transition
}

# Zoom transition
zoom_transition = {
    type = "zoom",
    scale_factor = 0.8,
    duration = 400,
    easing = "easeInOut"
}
```

### Visual Indicators

```toml
[shift_monitors.indicators]
# Shift direction indicator
show_direction_indicator = true
direction_indicator = {
    position = "center",
    size = 64,                      # Icon size in pixels
    color = "#007acc",
    duration = 1000,                # Display duration in ms
    animation = "pulse"             # "pulse", "fade", "bounce"
}

# Monitor labels
show_monitor_labels = true
monitor_labels = {
    position = "top_right",
    font_size = 16,
    color = "#ffffff",
    background = "#00000080",
    duration = 2000
}

# Progress indicator
show_progress = true
progress_indicator = {
    type = "bar",                   # "bar", "dots", "circle"
    position = "bottom",
    color = "#007acc",
    background = "#ffffff40"
}
```

## Integration with Other Plugins

### Workspaces Follow Focus Integration

```toml
# Coordinate with workspaces follow focus
[shift_monitors.integration.workspaces]
respect_workspace_focus = true     # Respect workspace focus settings
maintain_focus_following = true    # Maintain focus following during shift
sync_workspace_state = true        # Sync workspace state between plugins
```

### Scratchpads Integration

```toml
# Handle scratchpads during monitor shift
[shift_monitors.integration.scratchpads]
move_scratchpads = true            # Move scratchpads with workspaces
preserve_scratchpad_state = true   # Maintain scratchpad visibility state
reposition_scratchpads = true      # Reposition scratchpads for new monitor
```

### Wallpapers Integration

```toml
# Coordinate wallpaper changes during shift
[shift_monitors.integration.wallpapers]
sync_wallpapers = true             # Sync wallpapers during shift
maintain_per_monitor_wallpapers = true # Keep per-monitor wallpaper assignments
```

## Use Cases and Scenarios

### Development Workflow

```bash
# Shift development workspace to larger monitor
rustr shift-monitors workspace dev to-monitor DP-1

# Rotate workspaces for different tasks
rustr shift-monitors +1  # Move communication to secondary monitor
```

### Presentation Setup

```bash
# Shift main workspace to presentation monitor
rustr shift-monitors to-monitor HDMI-1

# Return to normal setup after presentation
rustr shift-monitors to-monitor DP-1
```

### Multi-Monitor Gaming

```bash
# Shift all workspaces away from gaming monitor
rustr shift-monitors from-monitor DP-2 +1

# Return workspaces after gaming
rustr shift-monitors from-monitor DP-1 -1
```

### Monitor Reconfiguration

```bash
# Rebalance workspaces after adding/removing monitor
rustr shift-monitors rebalance

# Set new monitor order after hardware change
rustr shift-monitors set-order "DP-1,HDMI-1,DP-2"
```

## Troubleshooting

### Common Issues

**Workspaces not shifting:**
```bash
# Check monitor detection
rustr shift-monitors list-monitors

# Verify monitor order
rustr shift-monitors monitor-order

# Test shift operation
rustr shift-monitors +1
```

**Incorrect shift direction:**
```bash
# Check current direction
rustr shift-monitors status

# Set correct direction
rustr shift-monitors set-direction +1

# Test with explicit direction
rustr shift-monitors forward
```

**Animation issues:**
```toml
[shift_monitors.animations]
enable_animations = false         # Disable if causing problems
transition_duration = 100         # Reduce duration
```

### Debug Commands

```bash
# Debug shift operations
rustr shift-monitors status       # Current plugin status
rustr shift-monitors config       # Current configuration
rustr shift-monitors list-monitors # Available monitors

# Test operations
rustr shift-monitors test +1      # Test shift without executing
rustr shift-monitors dry-run +1   # Dry run with full logging
```

## Performance Optimization

### Performance Settings

```toml
[shift_monitors.performance]
# Operation optimization
batch_operations = true           # Batch multiple workspace operations
async_transitions = true          # Asynchronous workspace transitions
parallel_processing = true        # Process operations in parallel

# Caching
cache_workspace_info = true       # Cache workspace information
cache_monitor_info = true         # Cache monitor information
cache_duration = 300              # Cache duration in seconds

# Resource management
max_concurrent_operations = 4     # Maximum concurrent shift operations
operation_timeout = 5000          # Operation timeout in milliseconds
cleanup_interval = 600            # Cleanup interval in seconds
```

### Memory Management

- **Efficient State Tracking**: Minimal memory usage for workspace state
- **Smart Caching**: Cache frequently accessed monitor and workspace information
- **Batch Processing**: Group multiple operations together for efficiency
- **Async Operations**: Non-blocking workspace transitions
- **Resource Cleanup**: Automatic cleanup of completed operations

## Advanced Configuration Examples

### Custom Monitor Arrangement

```toml
[shift_monitors]
# Complex monitor setup with custom order
monitor_order = ["DP-1", "DP-2", "DP-3", "HDMI-1"]
shift_mode = "circular"

# Monitor-specific settings
[shift_monitors.monitors.DP-1]
primary = true
shift_preference = "horizontal"

[shift_monitors.monitors.DP-2]
shift_preference = "horizontal"
group = "main"

[shift_monitors.monitors.HDMI-1]
shift_preference = "vertical"
group = "secondary"
```

### Workspace Templates

```toml
[shift_monitors.templates]
# Workspace templates for different monitor types
development = {
    layout = "tiled",
    windows = ["code", "terminal", "browser"],
    monitor_types = ["large", "primary"]
}

media = {
    layout = "floating",
    windows = ["vlc", "spotify"],
    monitor_types = ["secondary", "tv"]
}

communication = {
    layout = "stacked",
    windows = ["discord", "slack", "email"],
    monitor_types = ["portrait", "secondary"]
}
```

## Best Practices

### Setup Recommendations

1. **Configure Monitor Order**: Set explicit monitor order for predictable shifts
2. **Test Directions**: Verify shift directions match your physical setup
3. **Enable Animations**: Use animations for better visual feedback
4. **Preserve Focus**: Enable focus preservation for better user experience
5. **Use Keybindings**: Set up convenient keybindings for frequent shifts

### Usage Tips

1. **Start Simple**: Begin with basic +1/-1 shifts before complex operations
2. **Monitor Labels**: Enable monitor labels to understand current arrangement
3. **Group Related Workspaces**: Keep related workspaces on the same monitor
4. **Use Workspace Numbers**: Maintain consistent workspace numbering across monitors
5. **Regular Rebalancing**: Periodically rebalance workspaces across monitors

### Performance Tips

1. **Enable Caching**: Use caching for better performance with many workspaces
2. **Batch Operations**: Allow plugin to batch multiple shifts together
3. **Async Transitions**: Enable async transitions for smoother experience
4. **Optimize Animations**: Adjust animation duration for your hardware
5. **Monitor Resource Usage**: Check performance with status commands