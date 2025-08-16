# Wallpapers Plugin

**Status**: ✅ Still in development | **Tests**: 15/15 Passing

Advanced wallpaper management with hardware acceleration, interactive carousel navigation, multi-monitor support, and automatic rotation. Provides comprehensive wallpaper management for modern desktop environments.

## Features

- **Hardware-Accelerated Processing**: ImageMagick with OpenCL acceleration for thumbnails and image processing
- **Interactive Carousel**: Horizontal/vertical navigation with mouse and keyboard controls
- **Multi-Monitor Support**: Per-monitor wallpaper management with unique wallpapers
- **Smart Caching**: Thumbnail caching with modification time checking
- **Multiple Backends**: Support for swaybg, swww, wpaperd, and custom commands
- **Auto-Rotation**: Automatic wallpaper changing with configurable intervals
- **Format Support**: PNG, JPG, JPEG, WebP, BMP, TIFF, and more

## Configuration

### Basic Configuration

```toml
[wallpapers]
# Wallpaper directories (single path or array)
path = "~/Pictures/wallpapers"
# Or multiple paths:
# path = ["~/Pictures/wallpapers", "~/Downloads/backgrounds", "/usr/share/pixmaps"]

# Rotation interval in seconds (default: 600 = 10 minutes)
interval = 600

# Supported file extensions
extensions = ["png", "jpg", "jpeg", "webp", "bmp", "tiff"]

# Recursively scan subdirectories
recurse = true

# Set different wallpaper for each monitor
unique = false

# Wallpaper command (supports multiple backends)
command = "swaybg -i \"[file]\" -m fill"
```

### Advanced Configuration

```toml
[wallpapers]
# Multiple backend options
# Swaybg (default)
command = "swaybg -i \"[file]\" -m fill"

# Swww (with transitions)
# command = "swww img \"[file]\" --transition-type fade --transition-duration 2"

# Wpaperd (per-monitor)
# command = "wpaperd -w \"[output]::[file]\""

# Custom command
# command = "feh --bg-fill \"[file]\""

# Clear wallpapers command (optional)
clear_command = "killall swaybg"

# Carousel settings
enable_carousel = true
carousel_orientation = "horizontal"  # or "vertical"
thumbnail_size = 200
carousel_columns = 5                 # Number of columns in carousel grid
carousel_rows = 3                    # Number of rows in carousel grid

# Hardware acceleration
hardware_acceleration = true
smooth_transitions = true
transition_duration = 300            # milliseconds

# Caching (optional, defaults to ~/.cache/rustrland/wallpapers)
cache_dir = "~/.cache/rustrland/wallpapers"
cache_thumbnails = true
thumbnail_quality = 85               # JPEG quality for thumbnails (1-100)

# Performance settings
preload_count = 3                    # Number of wallpapers to preload
async_loading = true                 # Load wallpapers asynchronously
parallel_processing = true           # Process multiple wallpapers in parallel

# Debug logging
debug_logging = false
```

### Multi-Monitor Configuration

```toml
[wallpapers]
# Enable per-monitor wallpapers
unique = true

# Global settings
interval = 300
extensions = ["png", "jpg", "jpeg", "webp"]

# Per-monitor settings
[wallpapers.monitors]
"DP-1" = { 
    interval = 300, 
    command = "swaybg -o DP-1 -i \"[file]\" -m fill",
    path = "~/Pictures/wallpapers/4k"
}
"DP-2" = { 
    interval = 600, 
    command = "swaybg -o DP-2 -i \"[file]\" -m stretch",
    path = "~/Pictures/wallpapers/1440p"
}
"HDMI-1" = {
    interval = 0,  # No auto-rotation
    command = "swaybg -o HDMI-1 -i \"[file]\" -m center",
    path = "~/Pictures/presentations"
}
```

## Commands

### Basic Wallpaper Commands

```bash
# Basic wallpaper controls
rustr wallpapers next           # Next wallpaper (global or per-monitor)
rustr wallpapers prev           # Previous wallpaper
rustr wallpapers random         # Set random wallpaper
rustr wallpapers current        # Show current wallpaper info

# Set specific wallpaper
rustr wallpapers set ~/Pictures/wallpaper.jpg    # Set specific file
rustr wallpapers set-index 5    # Set by index in current list

# Wallpaper information
rustr wallpapers list           # List available wallpapers
rustr wallpapers status         # Show current wallpaper status
rustr wallpapers count          # Show total wallpaper count
```

### Carousel Navigation

```bash
# Interactive carousel
rustr wallpapers carousel       # Show interactive carousel
rustr wallpapers carousel next  # Navigate carousel forward
rustr wallpapers carousel prev  # Navigate carousel backward
rustr wallpapers carousel select # Select current carousel item
rustr wallpapers carousel exit  # Exit carousel mode

# Carousel grid navigation
rustr wallpapers carousel up    # Navigate up in grid
rustr wallpapers carousel down  # Navigate down in grid
rustr wallpapers carousel left  # Navigate left in grid
rustr wallpapers carousel right # Navigate right in grid
```

### Management Commands

```bash
# Directory and cache management
rustr wallpapers scan           # Rescan wallpaper directories
rustr wallpapers rescan         # Force full rescan (ignore cache)
rustr wallpapers clear          # Clear all wallpapers
rustr wallpapers cache-clear    # Clear thumbnail cache
rustr wallpapers cache-rebuild  # Rebuild thumbnail cache

# Rotation control
rustr wallpapers start          # Start automatic rotation
rustr wallpapers stop           # Stop automatic rotation
rustr wallpapers pause          # Pause rotation (resume with start)
rustr wallpapers restart        # Restart rotation timer
```

### Advanced Commands

```bash
# Monitor-specific commands (when unique = true)
rustr wallpapers next DP-1      # Next wallpaper on specific monitor
rustr wallpapers set ~/pic.jpg DP-2  # Set wallpaper on specific monitor
rustr wallpapers status DP-1    # Show status for specific monitor

# Collection management
rustr wallpapers add-path ~/new/wallpapers   # Add new wallpaper directory
rustr wallpapers remove-path ~/old/path      # Remove wallpaper directory
rustr wallpapers list-paths     # List configured wallpaper paths

# Filter and search
rustr wallpapers filter "nature" # Filter wallpapers by filename
rustr wallpapers search "sunset" # Search wallpapers by metadata
rustr wallpapers tag list        # List available tags (if supported)
```

## Keybindings

Add to your `~/.config/hypr/hyprland.conf`:

### Basic Wallpaper Controls

```bash
# Main wallpaper controls
bind = SUPER, W, exec, rustr wallpapers next      # Next wallpaper
bind = SUPER_SHIFT, W, exec, rustr wallpapers prev # Previous wallpaper
bind = SUPER_CTRL, W, exec, rustr wallpapers random # Random wallpaper

# Carousel controls
bind = SUPER, C, exec, rustr wallpapers carousel  # Show carousel
bind = SUPER_SHIFT, C, exec, rustr wallpapers carousel exit # Exit carousel

# Rotation controls
bind = SUPER_ALT, W, exec, rustr wallpapers pause # Pause auto-rotation
bind = SUPER_ALT, R, exec, rustr wallpapers start # Resume rotation
```

### Carousel Navigation (when active)

```bash
# Carousel navigation keys (active when carousel is shown)
bind = , Right, exec, rustr wallpapers carousel right
bind = , Left, exec, rustr wallpapers carousel left
bind = , Up, exec, rustr wallpapers carousel up
bind = , Down, exec, rustr wallpapers carousel down
bind = , Return, exec, rustr wallpapers carousel select
bind = , Escape, exec, rustr wallpapers carousel exit

# Alternative carousel navigation
bind = , Tab, exec, rustr wallpapers carousel next
bind = SHIFT, Tab, exec, rustr wallpapers carousel prev
bind = , Space, exec, rustr wallpapers carousel select
```

### Advanced Keybindings

```bash
# Monitor-specific controls (when unique = true)
bind = SUPER_CTRL, 1, exec, rustr wallpapers next DP-1
bind = SUPER_CTRL, 2, exec, rustr wallpapers next DP-2

# Quick management
bind = SUPER_SHIFT, R, exec, rustr wallpapers scan    # Rescan wallpapers
bind = SUPER_CTRL, R, exec, rustr wallpapers restart  # Restart rotation
```

## Carousel Interface

### Interactive Navigation

The carousel provides a visual grid interface for wallpaper selection:

- **Grid Layout**: Thumbnails arranged in responsive grid (configurable rows/columns)
- **Keyboard Navigation**: Arrow keys to navigate, Enter to select, Escape to exit
- **Mouse Support**: Click thumbnails to preview, double-click to select
- **Hardware Acceleration**: GPU-accelerated rendering for smooth scrolling
- **Smart Preloading**: Adjacent wallpapers preloaded for instant navigation

### Carousel Configuration

```toml
[wallpapers.carousel]
# Grid layout
columns = 5                     # Number of columns
rows = 3                        # Number of rows  
thumbnail_size = 200            # Thumbnail size in pixels
spacing = 10                    # Spacing between thumbnails

# Visual appearance
background_color = "#1e1e1e80"  # Semi-transparent background
selected_color = "#007acc"      # Selection highlight color
border_width = 2                # Border width for selected thumbnail
corner_radius = 8               # Thumbnail corner radius

# Animation settings
fade_in_duration = 200          # Fade in animation duration
scroll_animation = true         # Smooth scrolling animation
preview_animation = true        # Preview zoom animation

# Performance
async_thumbnail_loading = true  # Load thumbnails asynchronously
thumbnail_cache_size = 100      # Number of thumbnails to keep in memory
preload_adjacent = true         # Preload adjacent thumbnails
```

## Hardware Acceleration

### GPU Acceleration Settings

```toml
[wallpapers.acceleration]
# Hardware acceleration options
enable_opencl = true            # Use OpenCL for image processing
enable_vulkan = true            # Use Vulkan for rendering
enable_opengl = true            # Use OpenGL as fallback

# GPU memory management
gpu_memory_limit = 512          # GPU memory limit in MB
texture_cache_size = 256        # Texture cache size in MB
async_gpu_operations = true     # Asynchronous GPU operations

# Image processing acceleration
parallel_image_processing = true # Process multiple images in parallel
hardware_image_scaling = true   # Use GPU for image scaling
hardware_format_conversion = true # Use GPU for format conversion
```

### Performance Optimization

```toml
[wallpapers.performance]
# CPU optimization
worker_threads = 4              # Number of worker threads
async_file_operations = true    # Asynchronous file operations
memory_pool_size = 128          # Memory pool size in MB

# I/O optimization
read_buffer_size = 64           # Read buffer size in KB
write_buffer_size = 64          # Write buffer size in KB
concurrent_operations = 8       # Max concurrent file operations

# Cache optimization
intelligent_caching = true     # Smart cache management
cache_compression = true        # Compress cached data
cache_cleanup_interval = 3600   # Cache cleanup interval in seconds
```

## Multiple Backend Support

### Swaybg Backend (Default)

```toml
[wallpapers]
command = "swaybg -i \"[file]\" -m fill"
clear_command = "killall swaybg"

# Swaybg scaling modes
# -m fill      # Scale to fill, maintaining aspect ratio
# -m fit       # Scale to fit, maintaining aspect ratio  
# -m stretch   # Stretch to fill screen
# -m center    # Center image on screen
# -m tile      # Tile image across screen
```

### Swww Backend (Animated Transitions)

```toml
[wallpapers]
command = "swww img \"[file]\" --transition-type fade --transition-duration 2"
clear_command = "swww clear"

# Swww transition types
# --transition-type fade
# --transition-type wipe  
# --transition-type slide
# --transition-type grow
# --transition-type outer
```

### Wpaperd Backend (Wayland Native)

```toml
[wallpapers]
command = "wpaperd -w \"[output]::[file]\""

# Per-monitor wpaperd configuration
[wallpapers.monitors]
"DP-1" = { command = "wpaperd -w \"DP-1::[file]\"" }
"DP-2" = { command = "wpaperd -w \"DP-2::[file]\"" }
```

### Custom Backend

```toml
[wallpapers]
# Custom command with variable substitution
command = "my-wallpaper-tool --file \"[file]\" --monitor \"[monitor]\" --mode fill"

# Available variables:
# [file]     - Full path to wallpaper file
# [monitor]  - Monitor name (when unique = true)
# [width]    - Monitor width
# [height]   - Monitor height
# [index]    - Wallpaper index in current list
```

## Advanced Configuration

### Multiple Wallpaper Sources

```toml
[wallpapers]
# Multiple directories with different settings
[[wallpapers.sources]]
path = "~/Pictures/nature"
recurse = true
extensions = ["jpg", "png"]
weight = 2                      # Higher chance of selection

[[wallpapers.sources]]
path = "~/Pictures/abstract"
recurse = false
extensions = ["png", "webp"]
weight = 1

[[wallpapers.sources]]
path = "/usr/share/backgrounds"
recurse = true
extensions = ["jpg", "png"]
weight = 1

# Source-specific rotation
[[wallpapers.sources]]
path = "~/Pictures/seasonal"
recurse = true
schedule = "seasonal"           # Special scheduling
conditions = ["month:12,1,2"]   # Winter wallpapers
```

### Time-Based Wallpapers

```toml
[wallpapers.scheduling]
# Time-based wallpaper selection
enable_scheduling = true

# Time of day scheduling
[wallpapers.scheduling.time_of_day]
morning = { path = "~/Pictures/morning", time = "06:00-12:00" }
afternoon = { path = "~/Pictures/day", time = "12:00-18:00" }
evening = { path = "~/Pictures/evening", time = "18:00-22:00" }
night = { path = "~/Pictures/night", time = "22:00-06:00" }

# Seasonal scheduling
[wallpapers.scheduling.seasonal]
spring = { path = "~/Pictures/spring", months = [3, 4, 5] }
summer = { path = "~/Pictures/summer", months = [6, 7, 8] }
autumn = { path = "~/Pictures/autumn", months = [9, 10, 11] }
winter = { path = "~/Pictures/winter", months = [12, 1, 2] }
```

### Wallpaper Collections

```toml
[wallpapers.collections]
# Named collections for easy switching
nature = {
    paths = ["~/Pictures/nature", "~/Pictures/landscapes"],
    interval = 600,
    shuffle = true
}

abstract = {
    paths = ["~/Pictures/abstract", "~/Pictures/digital-art"],
    interval = 300,
    shuffle = false
}

work = {
    paths = ["~/Pictures/minimal", "~/Pictures/professional"],
    interval = 0,  # No auto-rotation for work
    shuffle = false
}

# Collection switching commands
# rustr wallpapers collection nature
# rustr wallpapers collection work
```

## Integration with Other Plugins

### Monitors Plugin Integration

```toml
# Automatic wallpaper adjustment for monitor changes
[wallpapers.monitor_integration]
auto_adjust_on_change = true    # Adjust wallpapers when monitors change
preserve_per_monitor = true     # Maintain per-monitor wallpaper assignments
scale_for_resolution = true     # Scale wallpapers for different resolutions
```

### Time/Weather Integration

```toml
[wallpapers.dynamic]
# Weather-based wallpapers (requires weather data)
weather_based = true
weather_api_key = "your-api-key"
location = "City, Country"

weather_collections = {
    sunny = "~/Pictures/sunny",
    cloudy = "~/Pictures/cloudy", 
    rainy = "~/Pictures/rainy",
    snowy = "~/Pictures/snowy"
}
```

## Troubleshooting

### Common Issues

**Wallpapers not changing:**
```bash
# Check wallpaper status
rustr wallpapers status

# Test manual change
rustr wallpapers next

# Check rotation status
rustr wallpapers start
```

**Thumbnail generation issues:**
```bash
# Clear and rebuild cache
rustr wallpapers cache-clear
rustr wallpapers cache-rebuild

# Check hardware acceleration
rustr wallpapers status | grep -i acceleration
```

**Performance issues:**
```toml
[wallpapers.performance]
hardware_acceleration = false  # Disable if causing issues
parallel_processing = false    # Reduce CPU usage
thumbnail_size = 128           # Smaller thumbnails
```

### Debug Information

```bash
# Debug wallpaper functionality
rustr wallpapers status        # Current status and configuration
rustr wallpapers list-paths    # Configured wallpaper directories
rustr wallpapers cache-info    # Cache usage and statistics

# Test specific functionality
rustr wallpapers scan          # Test directory scanning
rustr wallpapers carousel      # Test carousel interface
```

## Migration and Compatibility

### From Other Wallpaper Tools

- **nitrogen**: Direct path migration, enhanced features
- **feh**: Compatible command-line interface with GUI enhancements
- **variety**: Similar rotation and collection features
- **Desktop environments**: Replace built-in wallpaper managers

### Configuration Migration

```bash
# Import wallpaper collections
rustr wallpapers import-collection ~/.config/variety/wallpapers
rustr wallpapers import-nitrogen ~/.config/nitrogen/bg-saved.cfg

# Export current configuration
rustr wallpapers export-config ~/.config/rustrland/wallpapers-backup.toml
```

## Best Practices

### Organization Tips

1. **Organize by Resolution**: Separate wallpapers by monitor resolution
2. **Use Collections**: Group wallpapers by theme or mood
3. **Optimize Images**: Use appropriate formats (JPEG for photos, PNG for graphics)
4. **Cache Management**: Regularly clean thumbnail cache
5. **Monitor Specific**: Use per-monitor paths for multi-monitor setups

### Performance Tips

1. **Enable Hardware Acceleration**: Use GPU acceleration for better performance
2. **Limit Collection Size**: Keep collections reasonably sized for faster scanning
3. **Use Appropriate Formats**: Choose efficient image formats
4. **Cache Thumbnails**: Enable thumbnail caching for faster carousel
5. **Async Loading**: Enable asynchronous loading for smoother experience