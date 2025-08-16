# System Notifier Plugin

**Status**: ✅ Still in development | **Tests**: 10/10 Passing | **Animation Support**: ✨ Enhanced with Rustrland Animations

The system_notifier plugin monitors system logs and command outputs to generate desktop notifications with support for animated appearance and disappearance effects.

## Features

- **Pyprland Compatible**: Full compatibility with existing Pyprland system_notifier configurations
- **Enhanced Animations**: Appearance/disappearance animations using Rustrland's animation engine (enhancement over Pyprland)
- **Log Stream Monitoring**: Monitor journalctl, logs, or any command output
- **Pattern Matching**: Use regex patterns to detect interesting log lines
- **Text Filtering**: Transform notification text using regex filters (s/pattern/replacement/ format)
- **Desktop Notifications**: Native freedesktop.org notifications with urgency levels
- **Custom Icons and Sounds**: Support for custom notification appearance and audio feedback
- **Multiple Sources**: Monitor multiple log sources with different parsers simultaneously

## Configuration

### Basic Pyprland Compatible Configuration

```toml
# Basic Pyprland compatible configuration
[system_notifier.sources]
systemd = { command = "sudo journalctl -fx", parser = "journal" }
custom_logs = { command = "tail -f /var/log/myapp.log", parser = "generic" }

[system_notifier.parsers.journal]
pattern = "([a-z0-9]+): Link UP$"
filter = "s/.*\\[\\d+\\]: ([a-z0-9]+): Link.*/\\1 is now active/"
color = "#00aa00"
timeout = 5000
urgency = "normal"

[system_notifier.parsers.generic]
pattern = "ERROR: (.*)"
filter = "s/ERROR: (.*)/Application error: \\1/"
color = "#ff0000"
urgency = "critical"
icon = "dialog-error"
```

### Enhanced Configuration with Animations (Rustrland Extension)

```toml
# Enhanced configuration with animation support
[system_notifier.sources]
network = { command = "sudo journalctl -fx -u NetworkManager", parser = "network_events" }
errors = { command = "journalctl -fx -p err", parser = "error_events" }
security = { command = "sudo journalctl -fx -u ssh", parser = "security_events" }

[system_notifier.parsers.network_events]
pattern = "(\\w+): connected"
filter = "s/.*(\\w+): connected/Network \\1 connected/"
color = "#00ff00"
icon = "network-wireless"
sound = "/usr/share/sounds/freedesktop/stereo/network-connectivity-established.oga"

# Rustrland animation enhancement
[system_notifier.parsers.network_events.animation]
display_duration = 4000
smooth_transitions = true

[system_notifier.parsers.network_events.animation.appear]
animation_type = "fade"
duration = 300
easing = "easeOut"
opacity_from = 0.0
scale_from = 1.0

[system_notifier.parsers.network_events.animation.disappear]
animation_type = "scale"
duration = 200
easing = "easeIn"
opacity_from = 1.0
scale_from = 0.8

[system_notifier.parsers.error_events]
pattern = "(.+): (.+)"
filter = "s/.*: (.*)/System Error: \\1/"
color = "#ff4444"
urgency = "critical"
icon = "dialog-error"
timeout = 8000

[system_notifier.parsers.error_events.animation]
display_duration = 6000
smooth_transitions = true

[system_notifier.parsers.error_events.animation.appear]
animation_type = "fromTop"
duration = 400
easing = "bounce"
opacity_from = 0.0
scale_from = 0.5

[system_notifier.parsers.security_events]
pattern = "Failed password for .* from (.*) port"
filter = "s/Failed password for .* from (.*) port.*/Security Alert: Failed login from \\1/"
color = "#ff8800"
urgency = "critical"
icon = "security-low"
sound = "/usr/share/sounds/freedesktop/stereo/dialog-warning.oga"

[system_notifier.parsers.security_events.animation]
display_duration = 10000  # Security alerts stay longer
smooth_transitions = true

[system_notifier.parsers.security_events.animation.appear]
animation_type = "fromLeft"
duration = 500
easing = "elastic"
opacity_from = 0.0
scale_from = 0.3
```

## Commands

### Manual Notifications

```bash
# Basic manual notifications
rustr notify "Hello World"                    # Basic notification
rustr notify "Important message" critical 10000 # Critical with 10s timeout
rustr notify "Info message" normal 3000       # Normal priority with 3s timeout

# Enhanced notifications with animations (Rustrland extension)
rustr notify "Animated message" normal 5000 --animated   # With animation
rustr notify "Test notification" low 2000 --bounce      # With bounce animation
rustr notify "Error message" critical 0 --shake         # Critical with shake animation
```

### Plugin Management

```bash
# Plugin status and management
rustr notify status                          # Show plugin status and performance
rustr notify list-sources                    # List configured log sources
rustr notify list-parsers                    # List configured parsers
rustr notify reload                          # Reload configuration

# Source management
rustr notify start-source network            # Start specific source monitoring
rustr notify stop-source network             # Stop specific source monitoring
rustr notify restart-source errors           # Restart source monitoring
```

### Testing and Development

```bash
# Testing notifications
rustr notify test-animation "Test message"    # Send test notification with animations
rustr notify test-parser journal "test log line" # Test parser with sample input
rustr notify test-urgency critical "Critical test" # Test specific urgency level

# Performance and debugging
rustr notify performance                     # Show performance metrics
rustr notify debug-parser journal           # Debug specific parser
rustr notify clear-cache                    # Clear notification cache
```

## Configuration Options

### Parser Configuration

#### Basic Parser Options
- **pattern**: Regex pattern to match log lines (required)
- **filter**: Transform text using s/pattern/replacement/ format (optional)
- **color**: Notification color hint (optional)
- **timeout**: Timeout in milliseconds (optional, default: 5000, 0 = no timeout)
- **urgency**: "low", "normal", or "critical" (optional, default: "normal")
- **icon**: Icon name or path (optional)
- **sound**: Sound file path (optional)

#### Animation Configuration (Rustrland Enhancement)

**Appearance Animation (`animation.appear`):**
- **animation_type**: "fade", "scale", "fromTop", "fromBottom", "fromLeft", "fromRight", "bounce", "shake"
- **duration**: Animation duration in milliseconds
- **easing**: "linear", "easeIn", "easeOut", "easeInOut", "bounce", "elastic"
- **opacity_from**: Starting opacity (0.0-1.0)
- **scale_from**: Starting scale factor (e.g., 0.5 for half size)

**Disappearance Animation (`animation.disappear`):**
- Same properties as appearance animation
- Controls how notification disappears

**General Animation Settings:**
- **animation.display_duration**: How long to show notification before disappearing (ms)
- **animation.smooth_transitions**: Enable smooth transitions between animations

### Source Configuration

Each source defines a command to monitor and a parser to process its output:

```toml
[system_notifier.sources]
source_name = { 
    command = "shell_command_to_monitor", 
    parser = "parser_name",
    enabled = true,                    # Optional: enable/disable source
    restart_on_exit = true,            # Optional: restart if command exits
    buffer_size = 1024,                # Optional: buffer size for command output
    timeout = 30                       # Optional: restart timeout in seconds
}
```

## Use Cases and Examples

### System Monitoring

```toml
[system_notifier.sources]
disk_space = { command = "df -h | awk 'NR>1 && $5+0 > 90 {print $0}'", parser = "disk_alerts" }
memory_usage = { command = "free -m | awk 'NR==2 && $3/$2*100 > 80 {print \"Memory usage: \" $3/$2*100 \"%\"}'", parser = "memory_alerts" }
cpu_temperature = { command = "sensors | grep 'Package id 0:' | awk '{print $4}' | grep -o '[0-9.]*' | awk '$1 > 80 {print \"CPU temp: \" $1 \"°C\"}'", parser = "temp_alerts" }

[system_notifier.parsers.disk_alerts]
pattern = "(/dev/\\S+).*(\\d+)%"
filter = "s|(/dev/\\S+).*(\\d+)%.*|Disk \\1 is \\2% full|"
urgency = "critical"
color = "#ff0000"
icon = "drive-harddisk"
sound = "/usr/share/sounds/freedesktop/stereo/dialog-warning.oga"

[system_notifier.parsers.disk_alerts.animation]
display_duration = 8000
smooth_transitions = true

[system_notifier.parsers.disk_alerts.animation.appear]
animation_type = "shake"
duration = 600
easing = "bounce"
opacity_from = 0.0

[system_notifier.parsers.memory_alerts]
pattern = "Memory usage: (\\d+)%"
filter = "s/Memory usage: (\\d+)%/High Memory Usage: \\1%/"
urgency = "critical"
color = "#ff8800"
icon = "dialog-warning"

[system_notifier.parsers.temp_alerts]
pattern = "CPU temp: ([0-9.]+)°C"
filter = "s/CPU temp: ([0-9.]+)°C/High CPU Temperature: \\1°C/"
urgency = "critical"
color = "#ff0000"
icon = "weather-clear"
```

### Network Monitoring

```toml
[system_notifier.sources]
wifi_events = { command = "sudo journalctl -fx -u wpa_supplicant", parser = "wifi" }
ethernet_events = { command = "sudo journalctl -fx -u systemd-networkd", parser = "ethernet" }
vpn_events = { command = "sudo journalctl -fx -u openvpn", parser = "vpn" }

[system_notifier.parsers.wifi]
pattern = "CTRL-EVENT-CONNECTED"
filter = "s/.*/WiFi Connected/"
color = "#00aa00"
icon = "network-wireless"
sound = "/usr/share/sounds/freedesktop/stereo/network-connectivity-established.oga"

[system_notifier.parsers.wifi.animation]
display_duration = 3000
smooth_transitions = true

[system_notifier.parsers.wifi.animation.appear]
animation_type = "fade"
duration = 500
easing = "easeOut"

[system_notifier.parsers.ethernet]
pattern = "(eth\\d+): Link is Up"
filter = "s/(eth\\d+): Link is Up/Ethernet Connected: \\1/"
color = "#00aa00"
icon = "network-wired"

[system_notifier.parsers.vpn]
pattern = "Initialization Sequence Completed"
filter = "s/.*/VPN Connection Established/"
color = "#0088ff"
icon = "network-vpn"
urgency = "normal"
timeout = 4000
```

### Application Monitoring

```toml
[system_notifier.sources]
app_crashes = { command = "journalctl -fx -p crit", parser = "crashes" }
service_failures = { command = "journalctl -fx | grep 'Failed to start'", parser = "service_failures" }
package_updates = { command = "tail -f /var/log/pacman.log", parser = "package_updates" }

[system_notifier.parsers.crashes]
pattern = "segfault.*\\[(.+?)\\]"
filter = "s/.*segfault.*\\[(.+?)\\].*/Application \\1 crashed/"
urgency = "critical"
sound = "/usr/share/sounds/freedesktop/stereo/dialog-error.oga"
icon = "dialog-error"

[system_notifier.parsers.crashes.animation]
display_duration = 10000
smooth_transitions = true

[system_notifier.parsers.crashes.animation.appear]
animation_type = "bounce"
duration = 800
easing = "bounce"
opacity_from = 0.0
scale_from = 0.3

[system_notifier.parsers.service_failures]
pattern = "Failed to start (.+)\\.service"
filter = "s/Failed to start (.+)\\.service/Service Failed: \\1/"
urgency = "critical"
color = "#ff4444"
icon = "dialog-error"

[system_notifier.parsers.package_updates]
pattern = "\\[ALPM\\] upgraded (\\S+)"
filter = "s/\\[ALPM\\] upgraded (\\S+).*/Package Updated: \\1/"
urgency = "low"
color = "#0088ff"
icon = "system-software-update"
timeout = 2000
```

### Security Monitoring

```toml
[system_notifier.sources]
ssh_logins = { command = "sudo journalctl -fx -u ssh", parser = "ssh_events" }
sudo_usage = { command = "sudo journalctl -fx | grep sudo", parser = "sudo_events" }
firewall_blocks = { command = "sudo journalctl -fx -k | grep 'UFW BLOCK'", parser = "firewall_events" }

[system_notifier.parsers.ssh_events]
pattern = "Accepted publickey for (\\w+) from ([0-9.]+)"
filter = "s/Accepted publickey for (\\w+) from ([0-9.]+).*/SSH Login: \\1 from \\2/"
color = "#00aa00"
icon = "network-server"
urgency = "normal"

[system_notifier.parsers.ssh_events.animation]
display_duration = 5000
smooth_transitions = true

[system_notifier.parsers.ssh_events.animation.appear]
animation_type = "fromLeft"
duration = 400
easing = "easeOut"

[system_notifier.parsers.sudo_events]
pattern = "sudo.*: (\\w+) : TTY=.* ; PWD=.* ; USER=root ; COMMAND=(.*)"
filter = "s/sudo.*: (\\w+) : TTY=.* ; PWD=.* ; USER=root ; COMMAND=(.*)/Sudo: \\1 executed \\2/"
color = "#ffaa00"
icon = "dialog-password"
urgency = "low"
timeout = 3000

[system_notifier.parsers.firewall_events]
pattern = "UFW BLOCK.*SRC=([0-9.]+).*DPT=(\\d+)"
filter = "s/UFW BLOCK.*SRC=([0-9.]+).*DPT=(\\d+).*/Firewall Block: \\1 port \\2/"
color = "#ff4444"
icon = "security-high"
urgency = "normal"
timeout = 4000
```

## Keybindings

Add to your `~/.config/hypr/hyprland.conf`:

### Manual Notification Shortcuts

```bash
# Manual notification shortcuts
bind = SUPER_SHIFT, N, exec, rustr notify "Quick notification"
bind = SUPER_CTRL, N, exec, rustr notify "Critical alert" critical 0
bind = SUPER_ALT, N, exec, rustr notify "Animated message" normal 3000 --animated

# System notifications
bind = SUPER_SHIFT, I, exec, rustr notify "$(date)" normal 2000
bind = SUPER_SHIFT, M, exec, rustr notify "$(free -h | head -2 | tail -1)" normal 4000
bind = SUPER_SHIFT, D, exec, rustr notify "$(df -h / | tail -1)" normal 4000
```

### Plugin Management

```bash
# Plugin management shortcuts
bind = SUPER_SHIFT, F1, exec, rustr notify status
bind = SUPER_SHIFT, F2, exec, rustr notify test-animation "Keybinding test"
bind = SUPER_SHIFT, F3, exec, rustr notify performance
bind = SUPER_SHIFT, F4, exec, rustr notify reload
```

## Performance Considerations

### Optimization Settings

```toml
[system_notifier.performance]
# Resource management
max_concurrent_sources = 10     # Maximum concurrent source processes
buffer_size = 4096              # Buffer size for command output
notification_queue_size = 50    # Maximum queued notifications
cache_parsed_patterns = true    # Cache compiled regex patterns

# Rate limiting
max_notifications_per_minute = 30     # Prevent notification spam
duplicate_suppression = true          # Suppress duplicate notifications
duplicate_timeout = 300               # Duplicate suppression timeout (seconds)

# Memory management
cleanup_interval = 600               # Cleanup interval in seconds
max_memory_usage = 64                # Maximum memory usage in MB
gc_frequency = 1800                  # Garbage collection frequency (seconds)
```

### Performance Features

- **Efficient Parsing**: Regex patterns are compiled once at startup
- **Background Processing**: Log monitoring runs in separate async tasks
- **Animation Optimization**: Hardware-accelerated animations when available
- **Resource Management**: Automatic cleanup of completed monitoring tasks
- **Rate Limiting**: Built-in protection against notification spam

## Integration with Desktop Environment

The plugin uses freedesktop.org notification specifications and works with:

- **GNOME**: Native notification support via gnome-shell
- **KDE Plasma**: Native notification support via plasma-workspace
- **XFCE**: Via xfce4-notifyd
- **i3/Sway**: Via mako, dunst, or other notification daemons
- **Hyprland**: Native Wayland notification support

### Desktop-Specific Configuration

```toml
[system_notifier.desktop]
# Desktop environment detection and optimization
auto_detect_de = true
prefer_native_notifications = true

# Desktop-specific settings
[system_notifier.desktop.gnome]
use_gnome_notifications = true
integrate_with_gnome_shell = true

[system_notifier.desktop.kde]
use_kde_notifications = true
integrate_with_plasma = true

[system_notifier.desktop.sway]
notification_daemon = "mako"    # or "dunst", "swaync"
daemon_config_path = "~/.config/mako/config"
```

## Troubleshooting

### Common Issues

**Notifications not appearing:**
```bash
# Check plugin status
rustr notify status

# Test manual notification
rustr notify "Test notification"

# Check desktop notification daemon
ps aux | grep -E "(mako|dunst|notify)"
```

**Parser not matching log lines:**
```bash
# Test parser with sample input
rustr notify test-parser journal "sample log line"

# Debug parser regex
rustr notify debug-parser journal
```

**Performance issues:**
```toml
[system_notifier.performance]
max_concurrent_sources = 5     # Reduce concurrent sources
buffer_size = 2048             # Reduce buffer size
cleanup_interval = 300         # More frequent cleanup
```

### Debug Commands

```bash
# Debug notification functionality
rustr notify status            # Plugin status and configuration
rustr notify list-sources      # Active log sources
rustr notify performance       # Performance metrics

# Test specific components
rustr notify test-animation "Test" # Test animation system
rustr notify test-urgency critical "Test" # Test urgency levels
rustr notify test-parser journal "test input" # Test specific parser
```

## Migration from Pyprland

Existing Pyprland system_notifier configurations work without modification. To add Rustrland animation enhancements:

### Migration Steps

1. **Keep Existing Configuration**: All `[system_notifier.sources]` and `[system_notifier.parsers.*]` sections work unchanged
2. **Add Animation Sections**: Add `[system_notifier.parsers.*.animation]` sections for enhanced features
3. **Update Commands**: Use `rustr notify` instead of `pypr notify` for manual notifications
4. **Test Configuration**: Verify existing parsers work with `rustr notify test-parser`

### Example Migration

```toml
# Original Pyprland configuration (works unchanged)
[system_notifier.sources]
systemd = { command = "sudo journalctl -fx", parser = "system_events" }

[system_notifier.parsers.system_events]
pattern = "ERROR: (.*)"
filter = "s/ERROR: (.*)/System Error: \\1/"
urgency = "critical"
color = "#ff0000"

# Add Rustrland enhancements
[system_notifier.parsers.system_events.animation]
display_duration = 5000
smooth_transitions = true

[system_notifier.parsers.system_events.animation.appear]
animation_type = "shake"
duration = 400
easing = "bounce"
```

## Best Practices

### Configuration Best Practices

1. **Start Simple**: Begin with basic patterns and add complexity gradually
2. **Test Patterns**: Use `rustr notify test-parser` to verify regex patterns
3. **Rate Limiting**: Configure appropriate rate limiting to prevent spam
4. **Resource Management**: Monitor resource usage with performance commands
5. **Security**: Be careful with sudo commands in monitoring sources

### Performance Best Practices

1. **Optimize Patterns**: Use efficient regex patterns
2. **Limit Sources**: Don't monitor too many sources simultaneously
3. **Cache Patterns**: Enable pattern caching for better performance
4. **Cleanup Regularly**: Configure appropriate cleanup intervals
5. **Monitor Resources**: Regular check performance metrics