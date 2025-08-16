# Monitors Plugin

**Status**: ✅ Still in development | **Tests**: 15/15 Passing

Advanced monitor management with relative positioning, hotplug support, hardware acceleration, and automatic monitor detection. Provides comprehensive multi-monitor setup management.

## Features

- **Relative Monitor Placement**: Rule-based monitor positioning (left-of, right-of, above, below)
- **Hotplug Event Handling**: Automatic monitor detection and configuration
- **Hardware Acceleration**: GPU-accelerated monitor operations when available
- **Multiple Configuration Formats**: Support for both Pyprland and native formats
- **Real-time Updates**: Dynamic monitor configuration without restart
- **Profile Management**: Save and switch between monitor configurations
- **Resolution Management**: Automatic resolution and refresh rate optimization

## Configuration

### Basic Configuration

```toml
[monitors]
# Enable monitors plugin
enabled = true

# Monitor placement rules
placement_rules = [
    { monitor = "DP-1", position = "primary" },
    { monitor = "DP-2", position = { right_of = "DP-1" } },
    { monitor = "HDMI-1", position = { above = "DP-1" } }
]

# Hardware acceleration settings
hardware_acceleration = true
use_gpu_scaling = true

# Hotplug detection
hotplug_enabled = true
hotplug_delay = 1000  # milliseconds
```

### Advanced Configuration

```toml
[monitors]
# Monitor-specific settings
[[monitors.monitor_settings]]
name = "DP-1"
resolution = "3840x2160"
refresh_rate = 144
scale = 1.5
position = { x = 0, y = 0 }
rotation = 0                    # 0, 90, 180, 270 degrees
primary = true
enabled = true

[[monitors.monitor_settings]]
name = "DP-2"  
resolution = "2560x1440"
refresh_rate = 165
scale = 1.0
position = { x = 2560, y = 0 }  # Calculated automatically if using relative positioning
rotation = 0
enabled = true

[[monitors.monitor_settings]]
name = "HDMI-1"
resolution = "1920x1080"
refresh_rate = 60
scale = 1.0
position = { x = 0, y = -1080 } # Above DP-1
rotation = 0
enabled = false                 # Disabled by default

# Global monitor settings
[monitors.global]
auto_detect_resolution = true   # Automatically detect best resolution
auto_detect_refresh_rate = true # Automatically detect best refresh rate
preserve_aspect_ratio = true    # Maintain aspect ratios
enable_adaptive_sync = true     # Enable VRR/FreeSync/G-Sync if available
```

## Placement Rules

### Basic Positioning

```toml
[monitors]
placement_rules = [
    # Primary monitor (always positioned first)
    { monitor = "DP-1", position = "primary" },
    
    # Relative positioning
    { monitor = "DP-2", position = { right_of = "DP-1" } },
    { monitor = "DP-3", position = { left_of = "DP-1" } },
    { monitor = "HDMI-1", position = { above = "DP-1" } },
    { monitor = "HDMI-2", position = { below = "DP-1" } },
]
```

### Conditional Placement

```toml
[monitors]
placement_rules = [
    # Primary monitor
    { monitor = "DP-1", position = "primary", conditions = ["always"] },
    
    # Conditional placement based on monitor presence
    { 
        monitor = "DP-2", 
        position = { right_of = "DP-1" },
        conditions = ["if_present:DP-1"]
    },
    
    # Fallback positioning
    {
        monitor = "HDMI-1",
        position = { left_of = "DP-1" },
        conditions = ["if_absent:DP-2"]
    },

    # Complex conditions
    {
        monitor = "eDP-1",  # Laptop screen
        position = { below = "DP-1" },
        conditions = ["if_present:DP-1", "if_laptop"]
    }
]
```

### Advanced Positioning

```toml
[monitors]
# Exact positioning with coordinates
placement_rules = [
    { monitor = "DP-1", position = { x = 0, y = 0 } },
    { monitor = "DP-2", position = { x = 3840, y = 0 } },
    { monitor = "DP-3", position = { x = 1920, y = -1080 } },
]

# Calculated positioning with offsets
placement_rules = [
    { monitor = "DP-1", position = "primary" },
    { 
        monitor = "DP-2", 
        position = { right_of = "DP-1", offset = { x = 50, y = -100 } }
    },
]
```

## Commands

### Basic Monitor Commands

```bash
# Monitor layout management
rustr monitors relayout        # Apply monitor layout rules
rustr monitors detect          # Detect connected monitors
rustr monitors list            # List all connected monitors
rustr monitors status          # Show monitor status and configuration

# Configuration management
rustr monitors reload          # Reload monitor configuration
rustr monitors test            # Test monitor configuration
rustr monitors reset           # Reset to default configuration
```

### Advanced Commands

```bash
# Individual monitor control
rustr monitors enable DP-2     # Enable specific monitor
rustr monitors disable HDMI-1  # Disable specific monitor
rustr monitors primary DP-1    # Set primary monitor

# Resolution and refresh rate
rustr monitors resolution DP-1 3840x2160   # Set monitor resolution
rustr monitors refresh DP-1 144            # Set refresh rate
rustr monitors scale DP-1 1.5              # Set monitor scaling
rustr monitors rotate DP-1 90              # Rotate monitor (0/90/180/270)

# Profile management
rustr monitors save-profile "home"         # Save current setup as profile
rustr monitors load-profile "office"       # Load saved profile
rustr monitors list-profiles               # List saved profiles
rustr monitors delete-profile "old"        # Delete saved profile

# Hotplug management
rustr monitors hotplug enable              # Enable hotplug detection
rustr monitors hotplug disable             # Disable hotplug detection
rustr monitors hotplug status              # Show hotplug status
```

## Monitor Profiles

### Profile Management

Monitor profiles allow you to save and quickly switch between different monitor configurations:

```toml
[monitors.profiles]
# Home setup: Single 4K monitor
home = {
    monitors = [
        { name = "DP-1", resolution = "3840x2160", refresh_rate = 60, scale = 1.5, primary = true }
    ]
}

# Office setup: Dual monitor
office = {
    monitors = [
        { name = "DP-1", resolution = "3840x2160", refresh_rate = 144, scale = 1.5, primary = true },
        { name = "DP-2", resolution = "2560x1440", refresh_rate = 165, scale = 1.0, position = { right_of = "DP-1" } }
    ]
}

# Presentation setup: Mirror to projector
presentation = {
    monitors = [
        { name = "DP-1", resolution = "1920x1080", refresh_rate = 60, scale = 1.0, primary = true },
        { name = "HDMI-1", resolution = "1920x1080", refresh_rate = 60, scale = 1.0, mirror = "DP-1" }
    ]
}

# Gaming setup: Single high refresh rate monitor
gaming = {
    monitors = [
        { name = "DP-1", resolution = "2560x1440", refresh_rate = 240, scale = 1.0, primary = true, adaptive_sync = true }
    ]
}
```

### Profile Commands

```bash
# Profile management
rustr monitors save-profile "current-setup"    # Save current configuration
rustr monitors load-profile "gaming"           # Switch to gaming profile
rustr monitors auto-profile                    # Auto-detect and switch profile
rustr monitors profile-info "office"           # Show profile details
```

## Hotplug Support

### Automatic Detection

```toml
[monitors.hotplug]
# Hotplug configuration
enabled = true
detection_delay = 1000          # Wait 1 second after hotplug event
auto_configure = true           # Automatically configure new monitors
preserve_layout = true          # Maintain existing layout when possible
notify_changes = true           # Show notifications for monitor changes

# Hotplug rules
rules = [
    # When DP-2 is connected, apply office profile
    { event = "connected", monitor = "DP-2", action = "load_profile:office" },
    
    # When HDMI-1 is connected, mirror primary
    { event = "connected", monitor = "HDMI-1", action = "mirror_primary" },
    
    # When laptop lid is closed, disable eDP-1
    { event = "lid_closed", monitor = "eDP-1", action = "disable" },
]
```

### Hotplug Events

The monitors plugin responds to various hotplug events:

- **Monitor Connected**: New monitor plugged in
- **Monitor Disconnected**: Monitor unplugged
- **Resolution Changed**: Monitor resolution changed externally
- **Lid Events**: Laptop lid opened/closed (when applicable)
- **Dock Events**: Docking station connected/disconnected

## Hardware Acceleration

### GPU Acceleration

```toml
[monitors.acceleration]
# Hardware acceleration settings
enabled = true
use_vulkan = true               # Prefer Vulkan for modern GPUs
use_opengl = true               # Fallback to OpenGL
software_fallback = true        # Software rendering as last resort

# GPU-specific optimizations
nvidia_optimizations = true     # NVIDIA-specific optimizations
amd_optimizations = true        # AMD-specific optimizations
intel_optimizations = true      # Intel integrated graphics optimizations

# Performance settings
async_operations = true         # Asynchronous monitor operations
batch_changes = true            # Batch multiple monitor changes
gpu_memory_limit = 512          # GPU memory limit in MB
```

### Performance Optimization

```toml
[monitors.performance]
# Caching and optimization
cache_monitor_info = true       # Cache monitor capabilities
lazy_resolution_detection = true # Detect resolutions on demand
parallel_configuration = true   # Configure monitors in parallel
reduce_polling = true           # Reduce monitor polling frequency

# Memory management
max_cache_size = 64             # Maximum cache size in MB
cache_timeout = 300             # Cache timeout in seconds
cleanup_interval = 600          # Cleanup interval in seconds
```

## Integration with Other Plugins

### Wallpapers Integration

```toml
# Different wallpapers per monitor
[wallpapers]
monitor_specific = true
monitor_wallpapers = {
    "DP-1" = "~/Pictures/main-wallpaper.jpg",
    "DP-2" = "~/Pictures/secondary-wallpaper.jpg"
}
```

### Workspaces Integration

```toml
# Monitor-specific workspaces
[workspaces_follow_focus.monitors]
"DP-1" = { workspace_range = [1, 5] }
"DP-2" = { workspace_range = [6, 10] }
```

### Scratchpads Integration

```toml
# Force scratchpads to specific monitors
[scratchpads.term]
force_monitor = "DP-1"

[scratchpads.music]
force_monitor = "DP-2"
```

## Advanced Features

### Resolution Management

```toml
[monitors.resolution]
# Automatic resolution selection
auto_select_best = true         # Automatically select best resolution
prefer_native = true            # Prefer native resolution when available
fallback_resolutions = [        # Fallback resolutions in order of preference
    "2560x1440",
    "1920x1080", 
    "1680x1050",
    "1366x768"
]

# Custom resolutions
custom_resolutions = [
    { monitor = "DP-1", resolution = "3840x1600", refresh_rate = 144 },
    { monitor = "DP-2", resolution = "2560x1080", refresh_rate = 75 }
]
```

### Color Management

```toml
[monitors.color]
# Color profile management
enable_color_management = true
color_profiles = {
    "DP-1" = "~/color-profiles/monitor1.icc",
    "DP-2" = "~/color-profiles/monitor2.icc"
}

# Automatic color temperature
auto_color_temperature = true
day_temperature = 6500          # Kelvin
night_temperature = 3400        # Kelvin
transition_duration = 1800      # 30 minutes
```

### Monitor-Specific Settings

```toml
# Per-monitor advanced settings
[[monitors.advanced_settings]]
monitor = "DP-1"
brightness = 0.8                # Monitor brightness (0.0-1.0)
contrast = 1.1                  # Contrast adjustment
gamma = 1.0                     # Gamma correction
color_temperature = 6500        # Color temperature in Kelvin
hdr_enabled = true              # Enable HDR if supported
adaptive_sync = true            # Enable VRR/FreeSync/G-Sync

[[monitors.advanced_settings]]
monitor = "DP-2"
brightness = 0.9
contrast = 1.0
gamma = 0.9
color_temperature = 6200
```

## Troubleshooting

### Common Issues

**Monitor not detected:**
```bash
# Check monitor detection
rustr monitors detect
rustr monitors list

# Force rescan
rustr monitors rescan
```

**Incorrect positioning:**
```bash
# Test monitor layout
rustr monitors test
rustr monitors relayout

# Check placement rules
rustr monitors status
```

**Performance issues:**
```toml
[monitors.performance]
hardware_acceleration = false  # Disable if causing issues
async_operations = false       # Synchronous operations
reduce_polling = true          # Reduce polling frequency
```

### Debug Commands

```bash
# Debug monitor configuration
rustr monitors status          # Detailed monitor status
rustr monitors capabilities    # Show monitor capabilities
rustr monitors test-config     # Test current configuration

# Hardware information
rustr monitors gpu-info        # GPU and acceleration info
rustr monitors driver-info     # Display driver information
```

### Monitor Detection Issues

```bash
# Force monitor detection
rustr monitors force-detect

# Check hardware support
rustr monitors hardware-test

# Verify connections
rustr monitors connection-test DP-1
```

## Migration and Compatibility

### From Other Monitor Tools

- **xrandr**: Direct migration of xrandr configurations
- **kanshi**: Compatible with kanshi-style configurations
- **autorandr**: Similar profile-based management
- **System Settings**: GUI equivalent functionality

### Configuration Migration

```bash
# Import from xrandr
rustr monitors import-xrandr ~/.config/xrandr/setup.sh

# Import from kanshi
rustr monitors import-kanshi ~/.config/kanshi/config

# Export current configuration
rustr monitors export-config ~/.config/rustrland/monitors.toml
```

## Best Practices

### Setup Recommendations

1. **Start with Primary**: Always configure primary monitor first
2. **Use Relative Positioning**: Prefer relative positioning over absolute coordinates
3. **Enable Hotplug**: Enable hotplug for dynamic environments
4. **Save Profiles**: Create profiles for different scenarios
5. **Test Configurations**: Always test before applying permanently

### Performance Tips

1. **Enable Hardware Acceleration**: Use GPU acceleration when available
2. **Cache Monitor Info**: Enable caching for better performance
3. **Batch Changes**: Apply multiple monitor changes together
4. **Optimize Polling**: Reduce polling frequency when not needed

### Multi-Monitor Best Practices

1. **Consistent Scaling**: Use consistent DPI scaling across monitors
2. **Proper Alignment**: Align monitors properly for smooth cursor movement
3. **Workspace Assignment**: Assign specific workspaces to monitors
4. **Application Placement**: Configure applications for specific monitors