# Wallpapers Plugin

**Status**: ✅ Production Ready | **Tests**: 8/8 Passing

Advanced wallpaper management with multi-monitor support and automatic rotation. Provides comprehensive wallpaper management for modern desktop environments.

## Features

- **Multi-Monitor Support**: Per-monitor wallpaper management with unique wallpapers
- **Multiple Backends**: Support for swaybg, swww, wpaperd, and custom commands
- **Auto-Rotation**: Automatic wallpaper changing with configurable intervals
- **Format Support**: PNG, JPG, JPEG, WebP, BMP, TIFF, and more
- **Smart Preloading**: Cache wallpapers for better performance

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

# Performance settings
preload_count = 3                    # Number of wallpapers to preload

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
rustr wallpapers set ~/Pictures/wallpaper.jpg    # Set specific file

# Wallpaper information
rustr wallpapers list           # List available wallpapers
rustr wallpapers status         # Show current wallpaper status
```

### Management Commands

```bash
# Directory management
rustr wallpapers scan           # Rescan wallpaper directories
rustr wallpapers clear          # Clear all wallpapers

# Rotation control
rustr wallpapers start          # Start automatic rotation
rustr wallpapers stop           # Stop automatic rotation
```

### Monitor-Specific Commands

```bash
# Monitor-specific commands (when unique = true)
rustr wallpapers next DP-1      # Next wallpaper on specific monitor
rustr wallpapers set ~/pic.jpg DP-2  # Set wallpaper on specific monitor
```

## Keybindings

Add to your `~/.config/hypr/hyprland.conf`:

### Wallpaper Controls

```bash
# Main wallpaper controls
bind = SUPER, W, exec, rustr wallpapers next      # Next wallpaper
bind = SUPER_SHIFT, W, exec, rustr wallpapers set ~/path/to/wallpaper.jpg # Set specific wallpaper

# Rotation controls
bind = SUPER_ALT, W, exec, rustr wallpapers stop  # Stop auto-rotation
bind = SUPER_ALT, R, exec, rustr wallpapers start # Start rotation

# Monitor-specific controls (when unique = true)
bind = SUPER_CTRL, 1, exec, rustr wallpapers next DP-1
bind = SUPER_CTRL, 2, exec, rustr wallpapers next DP-2

# Quick management
bind = SUPER_SHIFT, R, exec, rustr wallpapers scan    # Rescan wallpapers
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

**Performance issues:**
```toml
[wallpapers]
preload_count = 1              # Reduce preloading for better performance
debug_logging = true           # Enable debug output
```

### Debug Information

```bash
# Debug wallpaper functionality
rustr wallpapers status        # Current status and configuration
rustr wallpapers list          # List available wallpapers

# Test specific functionality
rustr wallpapers scan          # Test directory scanning
rustr wallpapers next          # Test wallpaper switching
```

## Migration and Compatibility

### From Other Wallpaper Tools

- **nitrogen**: Direct path migration, enhanced features
- **feh**: Compatible command-line interface
- **variety**: Similar rotation features
- **Desktop environments**: Replace built-in wallpaper managers

### Configuration Migration

Simply copy your wallpaper directories to the rustrland configuration:

```toml
[wallpapers]
path = ["~/Pictures/wallpapers", "~/.config/variety/wallpapers"]
```

## Best Practices

### Organization Tips

1. **Organize by Resolution**: Separate wallpapers by monitor resolution
2. **Group by Theme**: Organize wallpapers in themed directories
3. **Optimize Images**: Use appropriate formats (JPEG for photos, PNG for graphics)
4. **Monitor Specific**: Use per-monitor paths for multi-monitor setups

### Performance Tips

1. **Limit Collection Size**: Keep collections reasonably sized for faster scanning
2. **Use Appropriate Formats**: Choose efficient image formats
3. **Adjust Preloading**: Configure `preload_count` based on available memory
4. **Monitor Resources**: Use `rustr wallpapers status` to check performance