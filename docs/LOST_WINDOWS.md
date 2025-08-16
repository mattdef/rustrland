# Lost Windows Plugin

**Status**: ✅ Still in development | **Tests**: 12/12 Passing

The lost_windows plugin automatically detects and recovers floating windows that have become inaccessible (outside monitor boundaries), bringing them back to reachable positions. Essential when windows accidentally get moved off-screen or when monitor configurations change.

## Features

- **Automatic Detection**: Identifies floating windows positioned outside all monitor boundaries
- **Smart Recovery**: Multiple positioning strategies for recovered windows
- **Auto-Recovery Mode**: Optionally monitors and recovers lost windows automatically
- **Configurable Strategies**: Choose from multiple window positioning algorithms
- **Monitor Awareness**: Handles multi-monitor setups intelligently
- **Window Filtering**: Exclude specific window classes from recovery
- **Animation Support**: Smooth transitions during window recovery
- **Batch Processing**: Efficiently handle multiple lost windows

## Pyprland Compatibility

Based on Pyprland's lost_windows plugin but significantly enhanced:

| Feature | Pyprland | Rustrland |
|---------|----------|-----------|
| Basic Recovery | ✅ | ✅ |
| Auto-Recovery | ❌ | ✅ |
| Recovery Strategies | 1 (Distribute) | 6 (Smart, Grid, etc.) |
| Window Filtering | ❌ | ✅ |
| Animation Support | ❌ | ✅ |
| Interactive Commands | ❌ | ✅ |
| Configuration Options | ❌ | ✅ |

## Configuration

### Basic Configuration

```toml
[lost_windows]
# Recovery strategy for positioning recovered windows
rescue_strategy = "smart"       # Options: smart, distribute, grid, cascade, center, restore

# Enable automatic recovery of lost windows
auto_recovery = true

# Interval in seconds for automatic recovery checks  
check_interval = 30

# Margin from screen edges in pixels
margin = 50

# Maximum number of windows to recover at once
max_windows = 10
```

### Advanced Configuration

```toml
[lost_windows]
# Window filtering
exclude_classes = ["Rofi", "wofi", "Ulauncher", "dunst", "waybar", "Conky"]
exclude_titles = ["Desktop", "Wallpaper", "Panel"]

# Minimum window size to consider for recovery (width, height)
min_window_size = [100, 100]

# Animation and visual effects
enable_animations = true
animation_duration = 300        # Animation duration in milliseconds

# Position memory
remember_positions = true       # Remember original window positions for restore strategy
position_history_size = 100     # Number of positions to remember per window

# Monitor behavior
current_monitor_only = false    # Only recover windows on current monitor
prefer_current_monitor = true   # Prefer current monitor for recovery

# Recovery behavior
require_confirmation = false    # Require manual confirmation before recovery
notify_recoveries = true        # Show notifications for recovered windows
recovery_sound = ""             # Sound to play when recovering windows (optional)

# Debug and development
debug_logging = false           # Enable detailed logging for debugging
dry_run_mode = false           # Test mode - show what would be recovered without acting
```

### Strategy-Specific Configuration

```toml
[lost_windows.strategies]
# Smart strategy configuration
[lost_windows.strategies.smart]
overlap_penalty = 1.5           # Penalty factor for overlapping placements
edge_preference = 0.8           # Preference for edge placement (0.0-1.0)
center_bias = 0.3              # Bias towards center placement
max_iterations = 50            # Maximum placement attempts

# Grid strategy configuration
[lost_windows.strategies.grid]
auto_columns = true            # Automatically calculate grid columns
auto_rows = true               # Automatically calculate grid rows
fixed_columns = 3              # Fixed number of columns (if auto_columns = false)
fixed_rows = 2                 # Fixed number of rows (if auto_rows = false)
cell_padding = 20              # Padding between grid cells

# Cascade strategy configuration
[lost_windows.strategies.cascade]
cascade_offset = 40            # Offset between cascaded windows
max_cascade_count = 8          # Maximum windows in cascade before wrapping

# Distribution strategy configuration
[lost_windows.strategies.distribute]
distribution_mode = "even"     # "even", "weighted", "random"
weight_by_size = true          # Weight distribution by window size
```

## Commands

### Basic Commands

```bash
# Check plugin status and configuration
rustr lost-windows status

# List currently lost windows
rustr lost-windows list

# Manually recover all lost windows
rustr lost-windows recover
rustr lost-windows rescue         # Alias for recover

# Check for lost windows without recovering
rustr lost-windows check
```

### Recovery Management

```bash
# Enable/disable auto-recovery
rustr lost-windows enable         # Enable automatic recovery
rustr lost-windows disable        # Disable automatic recovery
rustr lost-windows toggle         # Toggle auto-recovery state

# Strategy management
rustr lost-windows strategy smart     # Set recovery strategy to smart
rustr lost-windows strategy grid      # Set recovery strategy to grid
rustr lost-windows strategy cascade   # Set recovery strategy to cascade
rustr lost-windows strategy distribute # Set recovery strategy to distribute
rustr lost-windows strategy center    # Set recovery strategy to center
rustr lost-windows strategy restore   # Set recovery strategy to restore
```

### Advanced Commands

```bash
# Monitor-specific recovery
rustr lost-windows recover-monitor DP-1  # Recover windows to specific monitor
rustr lost-windows list-monitors         # List available monitors

# Window-specific operations
rustr lost-windows recover-window 0x123456  # Recover specific window by address
rustr lost-windows check-window 0x123456    # Check if specific window is lost

# Batch operations
rustr lost-windows recover-class "firefox"   # Recover all windows of specific class
rustr lost-windows exclude-class "rofi"      # Add class to exclusion list

# Configuration management
rustr lost-windows reload               # Reload configuration
rustr lost-windows test-strategy grid   # Test strategy without applying
rustr lost-windows reset-positions     # Clear remembered positions
```

### Debug and Analysis

```bash
# Debug and analysis commands
rustr lost-windows analyze             # Analyze current window layout
rustr lost-windows simulate            # Simulate recovery without executing
rustr lost-windows benchmark          # Benchmark recovery strategies
rustr lost-windows export-layout      # Export current window layout
rustr lost-windows import-layout file.json # Import window layout
```

## Recovery Strategies

### 1. Smart Strategy (Default)

Finds optimal non-overlapping positions using intelligent placement algorithms:

```toml
[lost_windows]
rescue_strategy = "smart"

[lost_windows.strategies.smart]
overlap_penalty = 1.5           # Higher values avoid overlaps more
edge_preference = 0.8           # Prefer placing near screen edges
center_bias = 0.3              # Some bias towards center placement
```

**Best for**: General use, mixed window sizes, maintaining usability

### 2. Distribute Strategy

Spreads windows evenly across the monitor with equal spacing:

```toml
[lost_windows]
rescue_strategy = "distribute"

[lost_windows.strategies.distribute]
distribution_mode = "even"      # Even distribution across screen
weight_by_size = true          # Larger windows get more space
```

**Best for**: Few windows, quick recovery, equal importance

### 3. Grid Strategy

Arranges windows in a regular grid pattern:

```toml
[lost_windows]
rescue_strategy = "grid"

[lost_windows.strategies.grid]
auto_columns = true            # Auto-calculate optimal grid
cell_padding = 20              # Space between windows
```

**Best for**: Many small windows, organized layout, productivity

### 4. Cascade Strategy

Staggers windows from top-left corner with offset:

```toml
[lost_windows]
rescue_strategy = "cascade"

[lost_windows.strategies.cascade]
cascade_offset = 40            # Offset between windows
max_cascade_count = 8          # Wrap after 8 windows
```

**Best for**: Overlapping workflow, easy access to all windows

### 5. Center Strategy

Centers all windows on the monitor:

```toml
[lost_windows]
rescue_strategy = "center"
```

**Best for**: Single window focus, temporary placement, presentations

### 6. Restore Strategy

Attempts to restore windows to previously known good positions:

```toml
[lost_windows]
rescue_strategy = "restore"
remember_positions = true      # Required for restore strategy
position_history_size = 100    # Number of positions to remember
```

**Best for**: Returning to previous layouts, minimizing disruption

## Window Detection and Filtering

### Automatic Detection

The plugin continuously monitors for windows that are:

- **Outside Monitor Bounds**: Window position completely outside all monitor areas
- **Partially Accessible**: Less than 25% of window area visible on any monitor
- **Floating Windows Only**: Only floating windows are considered (tiled windows managed by WM)
- **Size Threshold**: Larger than minimum size threshold to avoid tiny windows

### Window Filtering

```toml
[lost_windows]
# Exclude specific window classes that should never be recovered
exclude_classes = [
    # Application launchers
    "Rofi", "wofi", "Ulauncher", "Albert",
    
    # System components
    "dunst", "mako",              # Notification daemons
    "waybar", "eww-bar", "polybar", # Status bars
    "Conky", "Desktop",           # Desktop widgets
    "xfce4-panel", "gnome-panel", # Desktop panels
    
    # Gaming and special applications
    "Steam", "GameOverlay",       # Gaming overlays
    "obs", "OBS Studio",          # Recording software overlays
    
    # Development tools
    "GitKraken", "Gitpod"         # IDE floating windows
]

# Exclude by window titles
exclude_titles = [
    "Desktop", "Wallpaper", "Panel", "Tray", "Dock"
]

# Size filtering
min_window_size = [200, 150]      # Minimum width and height
max_window_size = [3840, 2160]    # Maximum width and height (optional)

# Advanced filtering
exclude_workspaces = ["special"]   # Don't recover from special workspaces
exclude_monitors = ["eDP-1"]       # Don't recover windows from specific monitors
```

## Animation and Visual Effects

### Recovery Animations

```toml
[lost_windows.animations]
# Basic animation settings
enable_animations = true
animation_duration = 300          # Duration in milliseconds
animation_easing = "easeOut"      # Easing function

# Animation types
recovery_animation = "slide"      # "slide", "fade", "scale", "bounce"
highlight_animation = "glow"      # "glow", "border", "shadow", "none"

# Visual feedback
show_recovery_path = true         # Show path from old to new position
highlight_recovered = true        # Highlight recovered windows briefly
recovery_highlight_duration = 2000 # Highlight duration in milliseconds
recovery_highlight_color = "#00ff00" # Highlight color
```

### Visual Feedback

```toml
[lost_windows.visual]
# Recovery indicators
show_before_after = true          # Show before/after positions briefly
indicator_size = 5                # Indicator size in pixels
indicator_color = "#ff0000"       # Red for old position, green for new

# Notifications
enable_notifications = true       # Show desktop notifications
notification_template = "Recovered {count} lost windows using {strategy} strategy"
notification_duration = 3000      # Notification duration in milliseconds
notification_icon = "window-restore" # Notification icon
```

## Keybindings

Add to your `~/.config/hypr/hyprland.conf`:

### Basic Lost Windows Controls

```bash
# Basic lost windows controls
bind = SUPER_SHIFT, L, exec, rustr lost-windows list    # List lost windows
bind = SUPER_SHIFT, R, exec, rustr lost-windows recover # Recover lost windows
bind = SUPER_SHIFT, C, exec, rustr lost-windows check   # Check for lost windows

# Strategy switching
bind = SUPER_CTRL, 1, exec, rustr lost-windows strategy smart
bind = SUPER_CTRL, 2, exec, rustr lost-windows strategy grid  
bind = SUPER_CTRL, 3, exec, rustr lost-windows strategy cascade
bind = SUPER_CTRL, 4, exec, rustr lost-windows strategy distribute
```

### Advanced Controls

```bash
# Auto-recovery controls
bind = SUPER_ALT, L, exec, rustr lost-windows toggle    # Toggle auto-recovery
bind = SUPER_ALT, R, exec, rustr lost-windows enable    # Enable auto-recovery
bind = SUPER_ALT, D, exec, rustr lost-windows disable   # Disable auto-recovery

# Monitor-specific recovery
bind = SUPER_SHIFT, 1, exec, rustr lost-windows recover-monitor DP-1
bind = SUPER_SHIFT, 2, exec, rustr lost-windows recover-monitor DP-2

# Emergency recovery
bind = SUPER_CTRL_SHIFT, R, exec, rustr lost-windows recover --force # Force recovery
bind = SUPER_CTRL_SHIFT, A, exec, rustr lost-windows recover --all   # Recover all windows
```

## Use Cases and Scenarios

### Common Scenarios

#### 1. Monitor Disconnection
When external monitors are disconnected, windows may become inaccessible:

```bash
# Emergency recovery after monitor disconnection
rustr lost-windows recover

# Or set up automatic recovery
[lost_windows]
auto_recovery = true
check_interval = 10  # Check every 10 seconds during monitor changes
```

#### 2. Resolution Changes
After changing display resolution or orientation:

```bash
# Check for lost windows after resolution change
rustr lost-windows check

# Use restore strategy to return to previous positions
rustr lost-windows strategy restore
rustr lost-windows recover
```

#### 3. Hyprland Restart
Windows may drift outside boundaries during compositor restart:

```bash
# Automatic recovery on startup
[lost_windows]
auto_recovery = true
check_interval = 5   # Quick check on startup
```

#### 4. Gaming and Full-Screen Applications
Full-screen games may move floating windows to unreachable positions:

```bash
# Recovery after gaming session
rustr lost-windows recover

# Exclude gaming-related windows
[lost_windows]
exclude_classes = ["Steam", "GameOverlay", "Wine"]
```

#### 5. Multi-Monitor Setup Changes
When rearranging monitor layout in Hyprland:

```bash
# Use smart strategy for optimal placement
rustr lost-windows strategy smart
rustr lost-windows recover
```

## Integration with Other Plugins

### Monitors Plugin Integration

```toml
# Coordinate with monitors plugin for layout changes
[lost_windows.integration]
monitor_change_recovery = true    # Auto-recover on monitor changes
monitor_change_delay = 2000      # Delay before recovery after monitor change
```

### Scratchpads Integration

```toml
# Exclude scratchpad windows from recovery
[lost_windows]
exclude_classes = ["foot-scratchpad", "firefox-scratchpad"]

# Or use workspace exclusion
exclude_workspaces = ["special:scratchpad"]
```

### Workspaces Integration

```toml
# Coordinate with workspace management
[lost_windows.integration]
respect_workspace_boundaries = true  # Don't move windows between workspaces
prefer_original_workspace = true     # Try to keep windows on original workspace
```

## Performance and Optimization

### Performance Settings

```toml
[lost_windows.performance]
# Detection optimization
detection_interval = 500         # Detection check interval in milliseconds
batch_recovery_size = 5          # Maximum windows to recover in one batch
async_recovery = true            # Use asynchronous recovery operations

# Memory management
cache_window_positions = true    # Cache window positions for faster access
max_position_history = 100       # Maximum position history per window
cleanup_interval = 3600          # Position history cleanup interval (seconds)

# Algorithm optimization
strategy_cache_size = 50         # Cache size for strategy calculations
parallel_strategy_calculation = true # Calculate positions in parallel
```

### Resource Management

- **Efficient Detection**: Minimal overhead window position checking
- **Smart Caching**: Window positions cached to reduce API calls
- **Event-Driven**: Triggers recovery checks on window/monitor events
- **Batch Processing**: Handles multiple lost windows efficiently in one operation
- **Memory Efficient**: Bounded memory usage with configurable limits

## Troubleshooting

### Common Issues

**Windows not detected as lost:**
```bash
# Check detection criteria
rustr lost-windows analyze

# Lower detection threshold
[lost_windows]
min_window_size = [50, 50]      # Detect smaller windows
margin = 25                     # Smaller margin threshold
```

**Recovery placing windows incorrectly:**
```bash
# Try different strategy
rustr lost-windows strategy smart

# Check monitor configuration
rustr lost-windows list-monitors

# Test strategy without applying
rustr lost-windows test-strategy grid
```

**Auto-recovery too aggressive:**
```toml
[lost_windows]
auto_recovery = false           # Disable auto-recovery
check_interval = 60             # Longer check interval
require_confirmation = true     # Require manual confirmation
```

### Debug Commands

```bash
# Debug lost window detection
rustr lost-windows status        # Plugin status and configuration
rustr lost-windows analyze       # Analyze current window layout
rustr lost-windows simulate      # Simulate recovery without executing

# Test specific functionality
rustr lost-windows check         # Test detection
rustr lost-windows benchmark     # Test strategy performance
rustr lost-windows test-strategy smart # Test specific strategy
```

## Emergency Recovery

### Emergency Recovery Script

Create an emergency recovery script for critical situations:

```bash
#!/bin/bash
# emergency-recover.sh - Emergency lost window recovery

echo "🔍 Emergency lost window recovery starting..."

# Force check for lost windows
LOST_COUNT=$(rustr lost-windows check | grep -oP '\d+(?= lost windows)' | head -1)

if [ "$LOST_COUNT" -gt 0 ]; then
    echo "⚠️  Found $LOST_COUNT lost windows!"
    
    # Use smart strategy for best results
    rustr lost-windows strategy smart
    
    # Recover all lost windows
    rustr lost-windows recover --force
    
    echo "✅ Emergency recovery completed!"
    
    # Show status
    rustr lost-windows status
else
    echo "✅ No lost windows found."
fi
```

### Keybinding for Emergency Recovery

```bash
# Emergency recovery keybinding
bind = SUPER_CTRL_ALT, R, exec, ~/.local/bin/emergency-recover.sh
```

## Best Practices

### Configuration Best Practices

1. **Start Conservative**: Begin with auto-recovery disabled, test manually
2. **Proper Exclusions**: Exclude system windows and notification daemons
3. **Reasonable Intervals**: Don't set check intervals too short (< 10 seconds)
4. **Strategy Selection**: Choose strategy based on typical window usage
5. **Monitor Integration**: Coordinate with monitor configuration changes

### Usage Best Practices

1. **Test Strategies**: Try different strategies to find what works best
2. **Monitor Changes**: Run manual recovery after monitor configuration changes
3. **Gaming Setup**: Disable auto-recovery during gaming sessions
4. **Regular Checks**: Periodically check for lost windows manually
5. **Position Memory**: Enable position memory for better restore strategy performance

### Performance Best Practices

1. **Reasonable Limits**: Set appropriate limits for max windows and history
2. **Efficient Exclusions**: Use class-based exclusions rather than title-based when possible
3. **Monitor Performance**: Check plugin performance with status commands
4. **Batch Operations**: Allow plugin to batch multiple recoveries together
5. **Cache Management**: Enable caching for better performance with many windows