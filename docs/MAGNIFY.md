# Magnify Plugin

**Status**: ✅ Production Ready | **Tests**: Integrated

Viewport zooming with smooth animations and external tool support for enhanced accessibility. Provides screen magnification capabilities with multiple zoom levels and seamless integration.

## Features

- **Smooth Zooming**: Hardware-accelerated zoom animations
- **Multiple Zoom Levels**: Configurable zoom presets (1.0x to 5.0x)
- **External Tool Support**: Integration with system magnification tools
- **Keyboard Shortcuts**: Quick zoom in/out/reset commands
- **Mouse Integration**: Zoom follows mouse cursor
- **Accessibility**: Full screen reader and accessibility tool compatibility
- **Performance Optimized**: GPU-accelerated rendering when available

## Configuration

### Basic Configuration

```toml
[magnify]
# Enable magnify plugin
enabled = true

# Zoom levels (default: [1.0, 1.5, 2.0, 3.0])
zoom_levels = [1.0, 1.25, 1.5, 2.0, 2.5, 3.0]

# Animation duration in milliseconds
animation_duration = 300

# Default zoom level index (0-based)
default_zoom_level = 0

# Maximum zoom level (safety limit)
max_zoom = 5.0

# Minimum zoom level
min_zoom = 1.0
```

### Advanced Configuration

```toml
[magnify]
# External magnification tool integration
external_tool = "magnus"        # Options: "magnus", "kmag", "xzoom", "none"
external_tool_args = ["--follow-mouse", "--smooth"]

# Mouse behavior
follow_mouse = true             # Zoom follows mouse cursor
mouse_zoom_speed = 0.2          # Mouse wheel zoom speed
invert_mouse_zoom = false       # Invert mouse wheel direction

# Animation settings
[magnify.animations]
zoom_easing = "easeOut"         # Animation easing function
smooth_transitions = true      # Enable smooth zoom transitions
frame_rate = 60                # Target animation frame rate

# Performance settings
[magnify.performance]
hardware_acceleration = true   # Use GPU acceleration
use_bilinear_filtering = true  # Smooth scaling algorithm
cache_zoom_levels = true       # Cache rendered zoom levels
async_rendering = true         # Render zoom asynchronously

# Accessibility integration
[magnify.accessibility]
screen_reader_compatible = true # Compatible with screen readers
high_contrast_mode = false     # Enable high contrast during zoom
invert_colors = false          # Invert colors when zooming
focus_tracking = true          # Track keyboard focus during zoom
```

## Commands

### Basic Zoom Commands

```bash
# Basic zoom controls
rustr magnify toggle            # Toggle magnification on/off
rustr magnify in                # Zoom in to next level
rustr magnify out               # Zoom out to previous level
rustr magnify reset             # Reset to normal zoom (1.0x)

# Specific zoom levels
rustr magnify set 2.0           # Set specific zoom level
rustr magnify set 150%          # Set zoom as percentage
rustr magnify level 3           # Set to 3rd zoom level (index-based)

# Status and information
rustr magnify status            # Show current zoom status
rustr magnify levels            # List available zoom levels
```

### Advanced Commands

```bash
# External tool integration
rustr magnify external on       # Enable external magnification tool
rustr magnify external off      # Disable external tool
rustr magnify external toggle   # Toggle external tool

# Mouse and cursor control
rustr magnify follow-mouse on   # Enable mouse following
rustr magnify follow-mouse off  # Disable mouse following
rustr magnify center-cursor     # Center zoom on cursor position

# Zoom area control
rustr magnify zoom-region 100 100 800 600  # Zoom specific region (x y width height)
rustr magnify zoom-window       # Zoom current window
rustr magnify zoom-screen       # Zoom entire screen

# Preset management
rustr magnify save-preset "reading"  # Save current zoom as preset
rustr magnify load-preset "reading"  # Load saved preset
rustr magnify list-presets      # List saved presets
```

## Keybindings

Add to your `~/.config/hypr/hyprland.conf`:

### Basic Zoom Controls

```bash
# Main zoom controls
bind = SUPER, PLUS, exec, rustr magnify in      # Zoom in
bind = SUPER, MINUS, exec, rustr magnify out    # Zoom out
bind = SUPER, 0, exec, rustr magnify reset      # Reset zoom
bind = SUPER, M, exec, rustr magnify toggle     # Toggle magnify

# Alternative controls
bind = SUPER, equal, exec, rustr magnify in     # Alternative zoom in
bind = SUPER, underscore, exec, rustr magnify out # Alternative zoom out
```

### Advanced Keybindings

```bash
# Specific zoom levels
bind = SUPER, 1, exec, rustr magnify level 1    # 1.25x zoom
bind = SUPER, 2, exec, rustr magnify level 2    # 1.5x zoom
bind = SUPER, 3, exec, rustr magnify level 3    # 2.0x zoom
bind = SUPER, 4, exec, rustr magnify level 4    # 2.5x zoom
bind = SUPER, 5, exec, rustr magnify level 5    # 3.0x zoom

# Mouse control
bind = SUPER_CTRL, M, exec, rustr magnify follow-mouse toggle
bind = SUPER_SHIFT, M, exec, rustr magnify center-cursor

# External tool integration
bind = SUPER_ALT, M, exec, rustr magnify external toggle

# Quick presets
bind = SUPER, F1, exec, rustr magnify load-preset "reading"
bind = SUPER, F2, exec, rustr magnify load-preset "coding"
bind = SUPER, F3, exec, rustr magnify load-preset "presentation"
```

### Mouse Controls

```bash
# Mouse wheel zooming (when magnify is active)
# These are handled automatically by the plugin:
# - Ctrl + Mouse Wheel: Zoom in/out
# - Shift + Mouse Wheel: Faster zoom in/out
# - Mouse movement: Pan around when zoomed in
```

## External Tool Integration

### Supported External Tools

#### Magnus (Recommended)
```toml
[magnify]
external_tool = "magnus"
external_tool_args = [
    "--follow-mouse",
    "--smooth", 
    "--hide-cursor",
    "--zoom-level", "2.0"
]
```

#### KMag (KDE)
```toml
[magnify]
external_tool = "kmag"
external_tool_args = ["--followmouse", "--hideframe"]
```

#### XZoom (X11)
```toml
[magnify]
external_tool = "xzoom"
external_tool_args = ["-mag", "2", "-continuous"]
```

#### Custom External Tool
```toml
[magnify]
external_tool = "custom"
external_tool_command = "my-magnifier --zoom {zoom_level} --x {cursor_x} --y {cursor_y}"
```

### Integration Features

- **Automatic Launching**: External tools launched when magnify is enabled
- **Zoom Level Sync**: Rustrland zoom levels synchronized with external tools
- **Mouse Coordination**: Mouse following coordinated between tools
- **Process Management**: External tools automatically managed (start/stop/restart)

## Accessibility Features

### Screen Reader Compatibility

```toml
[magnify.accessibility]
# Screen reader integration
screen_reader_compatible = true
announce_zoom_changes = true    # Announce zoom level changes
provide_zoom_context = true     # Provide context about zoom area

# NVDA/JAWS specific settings
nvda_integration = true
jaws_integration = true
orca_integration = true         # For Linux screen readers
```

### Visual Accessibility

```toml
[magnify.accessibility]
# Visual enhancements
high_contrast_mode = false      # High contrast during zoom
invert_colors = false           # Color inversion
enhance_contrast = 1.2          # Contrast enhancement factor
adjust_brightness = 1.0         # Brightness adjustment
color_filter = "none"           # Color filters: "none", "deuteranopia", "protanopia", "tritanopia"

# Focus indicators
show_focus_indicator = true     # Show keyboard focus indicator
focus_indicator_color = "#ff0000" # Focus indicator color
focus_indicator_thickness = 3   # Focus indicator border thickness
```

### Keyboard Navigation

```toml
[magnify.accessibility]
# Keyboard navigation
focus_tracking = true           # Track keyboard focus
smooth_focus_following = true   # Smooth transition to focused elements
focus_margin = 50              # Margin around focused element

# Navigation assistance
show_navigation_hints = true    # Show navigation hints
keyboard_zoom_step = 0.25      # Keyboard zoom increment
```

## Performance Optimization

### Hardware Acceleration

```toml
[magnify.performance]
# GPU acceleration
hardware_acceleration = true   # Enable GPU acceleration
use_vulkan = true              # Use Vulkan API if available
use_opengl = true              # Fallback to OpenGL
software_fallback = true       # Software rendering fallback

# Rendering optimization
multisampling = true           # Anti-aliasing for smooth edges
texture_filtering = "bilinear" # Texture filtering method
vsync = true                   # Vertical sync for smooth animation
```

### Memory Management

```toml
[magnify.performance]
# Memory optimization
cache_zoom_levels = true       # Cache rendered zoom levels
max_cache_size = 256           # Maximum cache size in MB
async_rendering = true         # Asynchronous rendering
lazy_loading = true            # Load content on demand

# Performance tuning
frame_rate_limit = 60          # Limit animation frame rate
reduce_quality_on_zoom = false # Reduce quality during zoom animation
skip_frames_when_busy = true   # Skip frames under heavy load
```

## Use Cases and Scenarios

### Reading and Text Work

```toml
# Preset for reading documents
[magnify.presets.reading]
zoom_level = 1.5
follow_mouse = false
high_contrast_mode = true
enhance_contrast = 1.3
focus_tracking = true
```

```bash
# Quick reading setup
rustr magnify load-preset "reading"
```

### Coding and Development

```toml
# Preset for code editing
[magnify.presets.coding]
zoom_level = 1.25
follow_mouse = true
focus_tracking = true
show_focus_indicator = true
```

```bash
# Development zoom setup
rustr magnify load-preset "coding"
```

### Presentations and Demos

```toml
# Preset for presentations
[magnify.presets.presentation]
zoom_level = 2.0
follow_mouse = true
smooth_transitions = true
external_tool = "magnus"
```

```bash
# Presentation mode
rustr magnify load-preset "presentation"
```

### Accessibility Assistance

```toml
# High-accessibility preset
[magnify.presets.accessibility]
zoom_level = 3.0
high_contrast_mode = true
invert_colors = false
show_focus_indicator = true
focus_indicator_thickness = 4
announce_zoom_changes = true
```

```bash
# High accessibility mode
rustr magnify load-preset "accessibility"
```

## Integration with Other Plugins

### Expose Plugin Integration

```bash
# Magnify works with expose mode
rustr expose                    # Enter expose mode
rustr magnify in               # Zoom in on expose grid
rustr magnify follow-mouse on  # Follow mouse in expose
```

### Scratchpads Integration

```toml
# Magnify scratchpad windows
[scratchpads.term]
auto_magnify = true            # Auto-magnify when shown
magnify_level = 1.25           # Default magnify level
```

### Wallpapers Integration

```bash
# Magnify with wallpaper awareness
rustr magnify in               # Zoom preserves wallpaper quality
rustr wallpapers next && rustr magnify reset # Reset zoom with new wallpaper
```

## Troubleshooting

### Common Issues

**Magnify not working:**
```bash
# Check magnify status
rustr magnify status

# Test basic zoom
rustr magnify set 1.5
rustr magnify reset
```

**Performance issues:**
```toml
[magnify.performance]
hardware_acceleration = false  # Disable if causing issues
reduce_quality_on_zoom = true  # Reduce quality for performance
frame_rate_limit = 30          # Lower frame rate limit
```

**External tool not launching:**
```bash
# Check if external tool is installed
which magnus
which kmag

# Test manual launch
magnus --help
```

### Debug Information

```bash
# Debug magnify functionality
rustr magnify status           # Current zoom status
rustr magnify levels           # Available zoom levels
rustr magnify external status  # External tool status

# Performance debugging
rustr magnify performance      # Performance metrics
rustr magnify cache-info       # Cache usage information
```

## Advanced Configuration Examples

### Custom Zoom Levels

```toml
[magnify]
# Fine-grained zoom levels for precise work
zoom_levels = [1.0, 1.1, 1.25, 1.33, 1.5, 1.67, 2.0, 2.5, 3.0, 4.0, 5.0]

# Quick zoom steps for presentations
zoom_levels = [1.0, 2.0, 3.0, 4.0]
```

### Multi-Monitor Setup

```toml
[magnify.monitors]
# Per-monitor magnify settings
"DP-1" = { 
    max_zoom = 3.0,
    default_tool = "magnus",
    follow_mouse = true
}

"DP-2" = {
    max_zoom = 2.0, 
    default_tool = "kmag",
    follow_mouse = false
}
```

### Accessibility Profiles

```toml
# Low vision profile
[magnify.profiles.low_vision]
zoom_levels = [2.0, 3.0, 4.0, 5.0]
high_contrast_mode = true
invert_colors = true
focus_tracking = true
announce_zoom_changes = true

# Dyslexia-friendly profile  
[magnify.profiles.dyslexia]
zoom_levels = [1.25, 1.5, 1.75, 2.0]
color_filter = "none"
enhance_contrast = 1.1
focus_tracking = true
```

## Migration and Compatibility

### From System Zoom Tools

Most system zoom tools can be replaced or integrated:

- **macOS Zoom**: Similar functionality with better customization
- **Windows Magnifier**: Enhanced features with Linux compatibility
- **GNOME Magnifier**: Better performance and more options

### From Other Linux Tools

- **Magnus**: Direct integration as external tool
- **KMag**: Full compatibility and integration
- **XZoom**: Legacy X11 support with Wayland enhancements