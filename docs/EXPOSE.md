# Expose Plugin

**Status**: ✅ Still in development | **Tests**: Integrated

Mission Control-style window overview with grid layout, navigation, and selection capabilities. Provides a visual overview of all open windows with keyboard and mouse navigation.

## Features

- **Grid Layout**: Automatically arranges windows in an optimal grid
- **Keyboard Navigation**: Arrow key navigation through windows
- **Visual Feedback**: Clear indication of selected window
- **Multi-Monitor**: Works across multiple monitors
- **Smooth Animations**: Hardware-accelerated transitions
- **Window Filtering**: Option to filter windows by workspace or monitor
- **Zoom Preview**: Hover to preview window content
- **Quick Selection**: Click or press Enter to select window

## Configuration

### Basic Configuration

```toml
[expose]
# Enable expose plugin
enabled = true

# Optional: Custom grid spacing in pixels
spacing = 20

# Optional: Animation duration in milliseconds  
animation_duration = 200

# Optional: Grid layout options
grid_cols = 0                    # Auto-calculate columns (0 = auto)
grid_rows = 0                    # Auto-calculate rows (0 = auto)

# Optional: Window filtering
show_minimized = false           # Show minimized windows
show_special = false             # Show special workspace windows
current_monitor_only = false     # Only show windows on current monitor
current_workspace_only = false   # Only show windows on current workspace
```

### Advanced Configuration

```toml
[expose]
# Visual appearance
background_color = "#1e1e1e80"   # Semi-transparent background
selected_color = "#007acc"       # Selection highlight color
window_padding = 10              # Padding around each window preview
border_width = 2                 # Border width for selected window

# Performance settings
max_windows = 50                 # Maximum windows to show
preview_quality = "medium"       # Preview quality: "low", "medium", "high"
hardware_acceleration = true     # Use GPU acceleration when available

# Navigation behavior
wrap_navigation = true           # Wrap around when navigating edges
auto_select_single = true        # Auto-select if only one window
exit_on_click_outside = true     # Exit expose when clicking outside

# Animation settings
[expose.animations]
fade_in_duration = 150           # Fade in animation duration
fade_out_duration = 100          # Fade out animation duration
zoom_duration = 200              # Window zoom animation duration
slide_duration = 250             # Window slide animation duration
easing = "easeOut"              # Animation easing function
```

## Commands

### Basic Commands

```bash
# Toggle expose mode
rustr expose                     # Toggle expose on/off
rustr expose toggle              # Same as above

# Navigation commands
rustr expose next                # Navigate to next window
rustr expose prev                # Navigate to previous window
rustr expose up                  # Navigate up in grid
rustr expose down                # Navigate down in grid
rustr expose left                # Navigate left in grid
rustr expose right               # Navigate right in grid

# Selection and control
rustr expose select              # Select current window and exit
rustr expose exit                # Exit expose mode without selection

# Status and management
rustr expose status              # Show expose status
rustr expose reload              # Reload expose configuration
```

### Advanced Commands

```bash
# Filtering commands
rustr expose show-all            # Show all windows (ignore filters)
rustr expose show-current        # Show only current workspace windows
rustr expose show-monitor        # Show only current monitor windows

# Layout commands
rustr expose grid 3 2            # Set specific grid layout (3 cols, 2 rows)
rustr expose grid auto           # Reset to auto grid layout

# Preview commands
rustr expose preview on          # Enable window previews
rustr expose preview off         # Disable window previews
```

## Keybindings

Add to your `~/.config/hypr/hyprland.conf`:

### Basic Navigation
```bash
# Main expose controls
bind = SUPER, TAB, exec, rustr expose        # Show all windows
bind = SUPER_SHIFT, TAB, exec, rustr expose toggle # Same as above

# Navigation while in expose mode
bind = , Right, exec, rustr expose next      # Next window
bind = , Left, exec, rustr expose prev       # Previous window  
bind = , Up, exec, rustr expose up           # Navigate up
bind = , Down, exec, rustr expose down       # Navigate down

# Selection and exit
bind = , Return, exec, rustr expose select   # Select current window
bind = , Escape, exec, rustr expose exit     # Exit without selection
bind = , Space, exec, rustr expose select    # Alternative select key
```

### Advanced Navigation
```bash
# Grid navigation
bind = SUPER, TAB, exec, rustr expose
bind = SUPER, Right, exec, rustr expose right
bind = SUPER, Left, exec, rustr expose left
bind = SUPER, Up, exec, rustr expose up
bind = SUPER, Down, exec, rustr expose down

# Quick access
bind = SUPER, grave, exec, rustr expose      # Alternative trigger
bind = SUPER, A, exec, rustr expose show-all # Show all windows
bind = SUPER_CTRL, TAB, exec, rustr expose show-current # Current workspace only
```

## Usage Workflow

### Basic Workflow
1. **Activate**: Press `Super + Tab` to enter expose mode
2. **Navigate**: Use arrow keys to navigate between windows
3. **Select**: Press `Enter` to focus the selected window
4. **Exit**: Press `Escape` to exit without selecting

### Mouse Workflow
1. **Activate**: Press `Super + Tab` to enter expose mode
2. **Hover**: Move mouse over windows to preview them
3. **Select**: Click on a window to focus it
4. **Exit**: Click outside the grid to exit without selecting

### Keyboard Shortcuts in Expose Mode
- **Arrow Keys**: Navigate between windows
- **Enter/Space**: Select current window
- **Escape**: Exit without selecting
- **Tab**: Cycle through windows in order
- **Home**: Go to first window
- **End**: Go to last window

## Multi-Monitor Support

Expose intelligently handles multi-monitor setups:

```toml
[expose]
# Monitor behavior options
current_monitor_only = false     # Show windows from all monitors
monitor_aware_layout = true      # Arrange by monitor position
preserve_monitor_groups = true   # Group windows by monitor

# Per-monitor settings
[expose.monitors]
"DP-1" = { grid_cols = 4, grid_rows = 3 }
"DP-2" = { grid_cols = 3, grid_rows = 2 }
"HDMI-1" = { spacing = 15 }
```

### Features
- **Cross-Monitor Navigation**: Navigate between windows across monitors
- **Monitor Grouping**: Group windows by their monitor
- **Adaptive Layout**: Adjust grid based on monitor count and resolution
- **Focus Following**: Expose follows your focus between monitors

## Window Filtering

Control which windows appear in expose mode:

```toml
[expose]
# Basic filtering
show_minimized = false           # Exclude minimized windows
show_special = false             # Exclude special workspace windows
show_floating_only = false       # Only show floating windows
show_tiled_only = false          # Only show tiled windows

# Advanced filtering
exclude_classes = ["Rofi", "wofi", "waybar"]  # Exclude specific window classes
exclude_titles = ["Desktop", "Wallpaper"]     # Exclude by window title
min_window_size = [100, 100]                  # Minimum window size to show

# Workspace filtering
current_workspace_only = false   # Only current workspace
include_workspaces = [1, 2, 3]  # Specific workspaces to include
exclude_workspaces = [9, 10]    # Workspaces to exclude
```

## Animations and Visual Effects

Expose supports various animation and visual effects:

### Animation Types
- **Grid Formation**: Windows animate into grid positions
- **Focus Transition**: Smooth transition when selecting window
- **Fade Effects**: Fade in/out when entering/exiting expose
- **Zoom Preview**: Zoom effect when hovering over windows

### Visual Customization
```toml
[expose.visual]
# Background and overlay
background_opacity = 0.8        # Background dimming opacity
background_blur = true          # Blur background windows
overlay_color = "#00000080"     # Overlay color

# Window appearance  
window_border_color = "#ffffff40"    # Window border color
selected_border_color = "#007acc"    # Selected window border color
window_corner_radius = 8             # Window corner rounding
window_shadow = true                 # Drop shadow for windows

# Text and labels
show_window_titles = true            # Show window titles
title_font_size = 12                 # Title font size
title_color = "#ffffff"              # Title text color
title_background = "#00000080"       # Title background color
```

## Performance Optimization

Expose is optimized for smooth performance even with many windows:

### Performance Settings
```toml
[expose.performance]
# Rendering optimization
max_preview_size = 256           # Maximum preview texture size
use_gpu_rendering = true         # Use GPU for window previews
async_preview_loading = true     # Load previews asynchronously
preview_cache_size = 50          # Number of previews to cache

# Layout optimization
fast_layout_mode = false         # Simplified layout for better performance
reduce_animations = false        # Reduce animations on slower systems
skip_preview_updates = false     # Don't update previews while navigating
```

### Resource Management
- **Memory Efficient**: Window previews are cached and reused
- **GPU Acceleration**: Hardware acceleration for smooth animations
- **Async Loading**: Window previews load in background
- **Smart Updates**: Only update visible window previews

## Integration with Other Plugins

Expose integrates seamlessly with other Rustrland plugins:

### Workspaces Follow Focus
```bash
# Show expose for specific workspace
rustr workspace switch 2 && rustr expose show-current
```

### Scratchpads
```bash
# Exclude scratchpads from expose
[expose]
exclude_classes = ["foot-scratchpad", "firefox-scratchpad"]
```

### Wallpapers
```bash
# Expose respects wallpaper as background
[expose.visual]
background_blur = true           # Blur wallpaper for better visibility
```

## Troubleshooting

### Common Issues

**Expose not showing all windows:**
```toml
[expose]
show_minimized = true           # Include minimized windows
show_special = true             # Include special workspace windows
current_workspace_only = false  # Show all workspaces
```

**Performance issues with many windows:**
```toml
[expose.performance]
max_windows = 30                # Limit number of windows shown
fast_layout_mode = true         # Use simplified layout
reduce_animations = true        # Reduce animation complexity
```

**Navigation not working in expose mode:**
```bash
# Check if keybindings conflict with other applications
# Make sure expose mode is actually active
rustr expose status
```

### Debug Information
```bash
# Check expose status
rustr expose status

# Test expose functionality
rustr expose                    # Enter expose mode
rustr expose next              # Test navigation
rustr expose exit              # Exit cleanly
```

## Advanced Use Cases

### Workspace Overview
```bash
# Create workspace-specific expose
bind = SUPER, 1, exec, rustr workspace switch 1 && rustr expose show-current
bind = SUPER, 2, exec, rustr workspace switch 2 && rustr expose show-current
```

### Application Launcher
```bash
# Use expose as application launcher
bind = SUPER, Space, exec, rustr expose show-all
```

### Window Management
```bash
# Combine with window management
bind = SUPER_SHIFT, Q, exec, rustr expose && rustr close-selected
bind = SUPER_SHIFT, M, exec, rustr expose && rustr minimize-selected
```

## Migration from Other Tools

### From macOS Mission Control
- Similar grid layout and navigation
- Enhanced keyboard navigation
- Better multi-monitor support

### From GNOME Activities
- More responsive performance
- Customizable appearance
- Better integration with tiling WMs

### From KDE Present Windows
- Equivalent functionality with better performance
- More configuration options
- Seamless Hyprland integration