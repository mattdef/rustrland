# System Notifier Plugin

**Status**: ✅ Production Ready | **Tests**: 19/19 Passing | **Mode**: Simple (Hyprland-native)

The system_notifier plugin monitors system logs and command outputs to generate desktop notifications using Hyprland's native notification system. It provides enhanced color and icon support while maintaining full compatibility with Pyprland configurations.

## Features

- **Pyprland Compatible**: Full compatibility with existing Pyprland system_notifier configurations
- **Hyprland Native**: Uses `hyprctl notify` for reliable, native notifications
- **Enhanced Colors**: Support for rgb(), rgba(), hex, and Hyprland color formats
- **Icon Support**: Complete icon name to Hyprland icon number conversion
- **Log Stream Monitoring**: Monitor journalctl, logs, or any command output
- **Pattern Matching**: Use regex patterns to detect interesting log lines
- **Text Filtering**: Transform notification text using regex filters (s/pattern/replacement/ format)
- **Multiple Sources**: Monitor multiple log sources with different parsers simultaneously
- **Sound Support**: Play notification sounds when events occur

## Architecture

The system_notifier plugin operates in **simple mode only**, using Hyprland's built-in notification system. This provides:

- **Automatic monitor placement**: Notifications appear on the active monitor
- **No external dependencies**: Works out-of-the-box with Hyprland
- **High reliability**: Direct integration with the window manager
- **Consistent appearance**: Matches Hyprland's notification style

## Configuration

### Basic Configuration

```toml
# Simple mode configuration (default)
[system_notifier]
color = "#0088ff"           # Default notification color
timeout = 5000              # Default timeout in milliseconds
icon = "info"               # Default icon
sound = "/path/to/sound.wav" # Optional default sound

[system_notifier.sources]
systemd = { command = "sudo journalctl -fx", parser = "journal" }
custom_logs = { command = "tail -f /var/log/myapp.log", parser = "generic" }

[system_notifier.parsers.journal]
pattern = "([a-z0-9]+): Link UP$"
filter = "s/.*\\[\\d+\\]: ([a-z0-9]+): Link.*/🌐 \\1 is now active/"
color = "rgb(0,170,0)"      # Green for network success
icon = "none"               # No text icon, emoji in message
urgency = "normal"
timeout = 4000

[system_notifier.parsers.generic]
pattern = "ERROR: (.*)"
filter = "s/ERROR: (.*)/❌ Application error: \\1/"
color = "rgba(255,68,68,0.9)" # Red with transparency
urgency = "critical"
icon = "none"               # No text icon, emoji in message
sound = "/usr/share/sounds/error.wav"
```

### Enhanced Configuration with Emojis

```toml
# Enhanced configuration with emoji integration
[system_notifier]
color = "#0088ff"
timeout = 5000
icon = "info"

[system_notifier.sources]
network = { command = "journalctl -fx -u NetworkManager", parser = "network_events" }
errors = { command = "journalctl -fx -p err", parser = "error_events" }
packages = { command = "tail -f /var/log/pacman.log", parser = "package_events" }

[system_notifier.parsers.network_events]
pattern = "(\\w+): connected"
filter = "s/.*(\\w+): connected/🌐 Network \\1 connected/"
color = "rgb(0,255,0)"
icon = "none"               # Use emoji instead of text icon
urgency = "normal"
timeout = 3000

[system_notifier.parsers.error_events]
pattern = "(.+): (.+)"
filter = "s/.*: (.*)/❌ System Error: \\1/"
color = "rgba(255,68,68,0.9)"
urgency = "critical"
icon = "none"
timeout = 8000
sound = "/usr/share/sounds/error.wav"

[system_notifier.parsers.package_events]
pattern = "\\[ALPM\\] upgraded (\\S+)"
filter = "s/\\[ALPM\\] upgraded (\\S+).*/📦 Package Updated: \\1/"
color = "#0088ff"
urgency = "low"
icon = "none"
timeout = 2000
```

## Commands

### Manual Notifications

```bash
# Basic manual notifications
rustr notify "Hello World"                    # Basic notification
rustr notify "Important message" critical 10000 # Critical with 10s timeout
rustr notify "Info message" normal 3000       # Normal priority with 3s timeout
```

### Plugin Management

```bash
# Plugin status and management
rustr notify status                          # Show plugin status
rustr notify list-sources                    # List configured log sources
rustr notify list-parsers                    # List configured parsers

# Testing
rustr notify test-notification "Test message" # Send test notification
```

## Configuration Options

### Main Configuration

- **color**: Default notification color (supports rgb(), rgba(), hex, 0x formats)
- **timeout**: Default timeout in milliseconds (optional)
- **urgency**: Default urgency level: "low", "normal", or "critical" (optional)
- **icon**: Default icon name (optional)
- **sound**: Default sound file path (optional)

### Parser Configuration

#### Basic Parser Options
- **pattern**: Regex pattern to match log lines (required)
- **filter**: Transform text using s/pattern/replacement/ format (optional)
- **color**: Notification color hint (optional, inherits from main config)
- **timeout**: Timeout in milliseconds (optional, inherits from main config)
- **urgency**: "low", "normal", or "critical" (optional, default: "normal")
- **icon**: Icon name (optional, inherits from main config)
- **sound**: Sound file path (optional, inherits from main config)

#### Icon Names

Hyprland supports these text-based icon values:
- **"warning"** → Triangle warning icon (0)
- **"info"** → Information icon (1) 
- **"hint"** → Hint/tip icon (2)
- **"error"** → Error/critical icon (3)
- **"confused"** → Question mark icon (4)
- **"ok"** → Checkmark/success icon (5)
- **"none"** → No icon displayed (-1)

**Recommended**: Use `icon = "none"` and include emojis in your filter text for better visual appeal.

#### Color Formats

The plugin supports multiple color formats:
- **RGB**: `rgb(255,68,68)`
- **RGBA**: `rgba(255,68,68,0.9)` 
- **Hex**: `#ff4444`
- **Hex with alpha**: `#ff4444aa`
- **Hyprland native**: `0xff4444ff` (ARGB format)
- **Default**: `"0"` (uses Hyprland default color)

### Source Configuration

Each source defines a command to monitor and a parser to process its output:

```toml
[system_notifier.sources]
source_name = { 
    command = "shell_command_to_monitor", 
    parser = "parser_name"
}
```

## Use Cases and Examples

### System Monitoring

```toml
[system_notifier.sources]
disk_space = { command = "df -h | awk 'NR>1 && $5+0 > 90 {print $0}'", parser = "disk_alerts" }
memory_usage = { command = "free -m | awk 'NR==2 && $3/$2*100 > 80 {print \"Memory usage: \" $3/$2*100 \"%\"}'", parser = "memory_alerts" }

[system_notifier.parsers.disk_alerts]
pattern = "(/dev/\\S+).*(\\d+)%"
filter = "s|(/dev/\\S+).*(\\d+)%.*|💾 Disk \\1 is \\2% full|"
urgency = "critical"
color = "rgb(255,0,0)"
icon = "none"
sound = "/usr/share/sounds/freedesktop/stereo/dialog-warning.oga"

[system_notifier.parsers.memory_alerts]
pattern = "Memory usage: (\\d+)%"
filter = "s/Memory usage: (\\d+)%/🧠 High Memory Usage: \\1%/"
urgency = "critical"
color = "rgb(255,136,0)"
icon = "none"
```

### Network Monitoring

```toml
[system_notifier.sources]
wifi_events = { command = "sudo journalctl -fx -u wpa_supplicant", parser = "wifi" }
ethernet_events = { command = "sudo journalctl -fx -u systemd-networkd", parser = "ethernet" }

[system_notifier.parsers.wifi]
pattern = "CTRL-EVENT-CONNECTED"
filter = "s/.*/📶 WiFi Connected/"
color = "rgb(0,170,0)"
icon = "none"
sound = "/usr/share/sounds/freedesktop/stereo/network-connectivity-established.oga"

[system_notifier.parsers.ethernet]
pattern = "(eth\\d+): Link is Up"
filter = "s/(eth\\d+): Link is Up/🌐 Ethernet Connected: \\1/"
color = "rgb(0,170,0)"
icon = "none"
```

### Security Monitoring

```toml
[system_notifier.sources]
ssh_logins = { command = "sudo journalctl -fx -u ssh", parser = "ssh_events" }
sudo_usage = { command = "sudo journalctl -fx | grep sudo", parser = "sudo_events" }

[system_notifier.parsers.ssh_events]
pattern = "Accepted publickey for (\\w+) from ([0-9.]+)"
filter = "s/Accepted publickey for (\\w+) from ([0-9.]+).*/🔐 SSH Login: \\1 from \\2/"
color = "rgb(0,170,0)"
icon = "none"
urgency = "normal"

[system_notifier.parsers.sudo_events]
pattern = "sudo.*: (\\w+) : TTY=.* ; PWD=.* ; USER=root ; COMMAND=(.*)"
filter = "s/sudo.*: (\\w+) : TTY=.* ; PWD=.* ; USER=root ; COMMAND=(.*)/🔑 Sudo: \\1 executed \\2/"
color = "rgb(255,170,0)"
icon = "none"
urgency = "low"
timeout = 3000
```

### Package Management

```toml
[system_notifier.sources]
package_updates = { command = "tail -f /var/log/pacman.log", parser = "package_updates" }

[system_notifier.parsers.package_updates]
pattern = "\\[ALPM\\] upgraded (\\S+)"
filter = "s/\\[ALPM\\] upgraded (\\S+).*/📦 Package Updated: \\1/"
urgency = "low"
color = "#0088ff"
icon = "none"
timeout = 2000
```

## Keybindings

Add to your `~/.config/hypr/hyprland.conf`:

```bash
# Manual notification shortcuts
bind = SUPER_SHIFT, N, exec, rustr notify "Quick notification"
bind = SUPER_CTRL, N, exec, rustr notify "Critical alert" critical 0
bind = SUPER_ALT, N, exec, rustr notify "System info: $(date)" normal 3000

# System notifications
bind = SUPER_SHIFT, M, exec, rustr notify "$(free -h | head -2 | tail -1)" normal 4000
bind = SUPER_SHIFT, D, exec, rustr notify "$(df -h / | tail -1)" normal 4000

# Plugin management shortcuts
bind = SUPER_SHIFT, F1, exec, rustr notify status
bind = SUPER_SHIFT, F2, exec, rustr notify test-notification "Keybinding test"
```

## Integration with Hyprland

The plugin is designed specifically for Hyprland and uses:

- **`hyprctl notify`**: Native Hyprland notification command
- **Automatic positioning**: Notifications appear on the active monitor
- **System integration**: Works with Hyprland's notification settings
- **No daemon required**: Direct integration with the window manager

### Hyprland Configuration

The plugin works with Hyprland's notification settings in `hyprland.conf`:

```bash
# Hyprland notification settings (optional)
misc {
    # These settings affect rustrland notifications
    disable_hyprland_logo = true
    disable_splash_rendering = true
}
```

## Troubleshooting

### Common Issues

**Notifications not appearing:**
```bash
# Check plugin status
rustr notify status

# Test manual notification
rustr notify "Test notification"

# Check if hyprctl works
hyprctl notify 1 3000 0 "Manual test"
```

**Parser not matching log lines:**
```bash
# Test notification system
rustr notify test-notification "Sample message"

# Check if source command works
tail -f /var/log/yourlog.log
```

**Colors not displaying correctly:**
```bash
# Test different color formats
rustr notify "RGB test" normal 3000
# Check the color in the notification
```

### Debug Commands

```bash
# Debug notification functionality
rustr notify status            # Plugin status and configuration
rustr notify list-sources      # Active log sources
rustr notify list-parsers      # Configured parsers
```

## Migration from Pyprland

Existing Pyprland system_notifier configurations work without modification.

### Migration Steps

1. **Keep Existing Configuration**: All `[system_notifier.sources]` and `[system_notifier.parsers.*]` sections work unchanged
2. **Update Commands**: Use `rustr notify` instead of `pypr notify` for manual notifications
3. **Add Emoji Support**: Consider using `icon = "none"` and emojis in filter text for better visuals
4. **Test Configuration**: Verify existing parsers work with `rustr notify test-notification`

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
# Add icon = "none" for cleaner appearance (optional)
```

## Best Practices

### Configuration Best Practices

1. **Start Simple**: Begin with basic patterns and add complexity gradually
2. **Use Emojis**: Set `icon = "none"` and use emojis in filter text for better visuals
3. **Test Patterns**: Use `rustr notify test-notification` to verify functionality
4. **Color Consistency**: Use consistent color schemes (green for success, red for errors, etc.)
5. **Appropriate Timeouts**: Use longer timeouts for critical messages, shorter for informational ones

### Performance Best Practices

1. **Optimize Patterns**: Use efficient regex patterns
2. **Limit Sources**: Don't monitor too many sources simultaneously
3. **Monitor Resources**: Check that log monitoring commands don't consume too much CPU
4. **Sound Usage**: Use sounds sparingly to avoid audio spam

### Security Best Practices

1. **Sudo Commands**: Be careful with sudo commands in monitoring sources
2. **Log Access**: Ensure proper permissions for log file access
3. **Sensitive Data**: Don't include sensitive information in notification text
4. **Command Injection**: Validate any dynamic content in commands

## Performance

The simple mode design provides excellent performance:

- **Lightweight**: No external dependencies or complex overlay management
- **Reliable**: Direct integration with Hyprland ensures notifications always work
- **Fast**: `hyprctl notify` provides immediate notification delivery
- **Resource Efficient**: Minimal memory and CPU usage
- **Stable**: Simple architecture reduces potential failure points

## Summary

The system_notifier plugin provides a robust, Pyprland-compatible notification system that leverages Hyprland's native capabilities. By focusing on simplicity and reliability, it delivers consistent performance while offering enhanced features like improved color support and emoji integration.