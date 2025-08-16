# Workspaces Follow Focus Plugin

**Status**: ✅ Still in development | **Tests**: Integrated

Multi-monitor workspace management with cross-monitor switching, intelligent focus following, and advanced workspace manipulation. Provides seamless workspace navigation across multiple monitors.

## Features

- **Cross-Monitor Navigation**: Switch workspaces across different monitors
- **Focus Following**: Automatically follow focus between workspaces  
- **Workspace Management**: Create, switch, and manage workspaces dynamically
- **Monitor Awareness**: Intelligent handling of monitor-specific workspaces
- **Relative Navigation**: Navigate with +1/-1 relative movements
- **Workspace Persistence**: Remember workspace state across sessions
- **Smart Switching**: Intelligent workspace switching based on content

## Configuration

### Basic Configuration

```toml
[workspaces_follow_focus]
# Enable the plugin
enabled = true

# Optional: Max workspaces per monitor
max_workspaces = 10

# Optional: Auto-create workspaces when switching
auto_create = true

# Optional: Start with workspace 1 on all monitors
start_workspace = 1

# Optional: Workspace naming scheme
workspace_naming = "numeric"     # "numeric", "named", "hybrid"
```

### Advanced Configuration

```toml
[workspaces_follow_focus]
# Monitor behavior
follow_focus_delay = 100         # Delay in ms before following focus
cross_monitor_switching = true   # Enable cross-monitor workspace switching
preserve_monitor_workspaces = true # Keep workspace-monitor associations

# Workspace management
empty_workspace_timeout = 30     # Auto-remove empty workspaces after 30s
remember_last_workspace = true   # Remember last workspace per monitor
workspace_wrap_around = true     # Wrap to workspace 1 after max

# Focus behavior
focus_follows_workspace = true   # Focus follows when switching workspaces
workspace_follows_focus = false  # Workspace switches when focus changes
smart_focus_switching = true     # Intelligent focus-based switching

# Advanced options
[workspaces_follow_focus.naming]
# Custom workspace names
workspace_names = {
    1 = "Main",
    2 = "Dev", 
    3 = "Web",
    4 = "Chat",
    5 = "Media"
}

# Monitor-specific workspace ranges
monitor_workspace_ranges = {
    "DP-1" = [1, 5],      # Monitor DP-1 uses workspaces 1-5
    "DP-2" = [6, 10],     # Monitor DP-2 uses workspaces 6-10
    "HDMI-1" = [11, 15]   # Monitor HDMI-1 uses workspaces 11-15
}
```

## Commands

### Basic Workspace Commands

```bash
# Direct workspace switching
rustr workspace switch 1        # Switch to workspace 1
rustr workspace switch 2        # Switch to workspace 2
rustr workspace switch 3        # Switch to workspace 3

# Relative workspace navigation
rustr workspace change +1       # Next workspace
rustr workspace change -1       # Previous workspace (use -- for negative)
rustr workspace change +2       # Skip ahead 2 workspaces
rustr workspace change -- -2    # Go back 2 workspaces

# Workspace information
rustr workspace list            # List all workspaces and monitors
rustr workspace current         # Show current workspace info
rustr workspace status          # Show detailed workspace status
```

### Advanced Workspace Commands

```bash
# Workspace creation and management
rustr workspace create 5        # Create workspace 5 if it doesn't exist
rustr workspace create "Dev"    # Create named workspace
rustr workspace remove 5       # Remove empty workspace 5
rustr workspace rename 5 "Code" # Rename workspace 5 to "Code"

# Cross-monitor operations
rustr workspace move-to-monitor "DP-2"  # Move current workspace to monitor
rustr workspace switch-monitor 3 "DP-1" # Switch workspace 3 on specific monitor

# Window management
rustr workspace move-window 4   # Move current window to workspace 4
rustr workspace follow-window 4 # Move window and follow to workspace 4
```

### Monitoring and Status

```bash
# Status and debugging
rustr workspace status          # Detailed workspace and monitor status
rustr workspace list-monitors   # List all connected monitors
rustr workspace list-windows    # List windows per workspace
rustr workspace history         # Show workspace switch history

# Configuration management
rustr workspace reload          # Reload workspace configuration
rustr workspace reset           # Reset all workspaces to default state
```

## Keybindings

Add to your `~/.config/hypr/hyprland.conf`:

### Basic Workspace Navigation

```bash
# Direct workspace switching
bind = SUPER, 1, exec, rustr workspace switch 1
bind = SUPER, 2, exec, rustr workspace switch 2
bind = SUPER, 3, exec, rustr workspace switch 3
bind = SUPER, 4, exec, rustr workspace switch 4
bind = SUPER, 5, exec, rustr workspace switch 5

# Relative navigation
bind = SUPER, Right, exec, rustr workspace change +1
bind = SUPER, Left, exec, rustr workspace change -- -1
bind = SUPER, Page_Up, exec, rustr workspace change +5
bind = SUPER, Page_Down, exec, rustr workspace change -- -5

# Quick access
bind = SUPER, Home, exec, rustr workspace switch 1    # Go to first workspace
bind = SUPER, End, exec, rustr workspace list         # Show workspace list
```

### Advanced Keybindings

```bash
# Window management with workspaces
bind = SUPER_SHIFT, 1, exec, rustr workspace move-window 1
bind = SUPER_SHIFT, 2, exec, rustr workspace move-window 2
bind = SUPER_SHIFT, 3, exec, rustr workspace move-window 3

# Follow window to workspace
bind = SUPER_CTRL, 1, exec, rustr workspace follow-window 1
bind = SUPER_CTRL, 2, exec, rustr workspace follow-window 2
bind = SUPER_CTRL, 3, exec, rustr workspace follow-window 3

# Monitor operations
bind = SUPER_ALT, Right, exec, rustr workspace move-to-monitor next
bind = SUPER_ALT, Left, exec, rustr workspace move-to-monitor prev

# Special operations
bind = SUPER, grave, exec, rustr workspace switch-back  # Switch to last workspace
bind = SUPER_SHIFT, grave, exec, rustr workspace create-next # Create next workspace
```

## Multi-Monitor Workspace Management

### Monitor-Specific Workspaces

The plugin supports sophisticated multi-monitor workspace management:

```toml
[workspaces_follow_focus]
# Per-monitor workspace configuration
monitor_workspace_strategy = "dedicated"  # "dedicated", "shared", "hybrid"

# Dedicated: Each monitor has its own workspace range
[workspaces_follow_focus.monitors."DP-1"]
workspace_range = [1, 5]         # Workspaces 1-5 for primary monitor
default_workspace = 1
wrap_around = true

[workspaces_follow_focus.monitors."DP-2"]  
workspace_range = [6, 10]        # Workspaces 6-10 for secondary monitor
default_workspace = 6
wrap_around = true

[workspaces_follow_focus.monitors."HDMI-1"]
workspace_range = [11, 15]       # Workspaces 11-15 for tertiary monitor
default_workspace = 11
wrap_around = false
```

### Cross-Monitor Navigation

```bash
# Navigate workspaces across monitors
rustr workspace switch 1         # Switch to workspace 1 (monitor DP-1)
rustr workspace switch 6         # Switch to workspace 6 (monitor DP-2)
rustr workspace switch 11        # Switch to workspace 11 (monitor HDMI-1)

# Cross-monitor relative navigation
rustr workspace change +1        # Next workspace (may switch monitors)
rustr workspace change-monitor +1 # Next workspace on same monitor only
```

## Focus Following Behavior

### Focus-Driven Workspace Switching

```toml
[workspaces_follow_focus]
# Focus following options
workspace_follows_focus = true   # Switch workspace when focus changes
focus_follow_threshold = 500     # Minimum time in ms before switching
focus_follow_exclusions = ["Rofi", "wofi"] # Don't follow focus for these apps

# Smart following behavior
smart_focus_switching = true     # Use intelligent focus following
focus_memory_duration = 5000     # Remember focus for 5 seconds
prevent_focus_loops = true       # Prevent infinite focus switching loops
```

### Use Cases for Focus Following

1. **Multi-Monitor Development**: Follow focus between code editor and terminal across monitors
2. **Content Creation**: Switch between design tools and reference materials
3. **Presentations**: Follow focus between presentation and notes
4. **Gaming**: Switch between game and chat applications

## Workspace Persistence

### Session Management

```toml
[workspaces_follow_focus.persistence]
# Save workspace state
save_workspace_state = true     # Save state between sessions
state_file = "~/.cache/rustrland/workspaces.json"
save_interval = 30              # Save every 30 seconds

# Restore behavior
restore_on_startup = true       # Restore workspaces on startup
restore_window_positions = true # Restore window positions
restore_focus_state = true      # Restore last focused window
```

### Workspace History

```bash
# Access workspace history
rustr workspace history         # Show recent workspace switches
rustr workspace back           # Go to previous workspace in history
rustr workspace forward        # Go to next workspace in history

# History navigation with keybindings
bind = SUPER, bracketleft, exec, rustr workspace back
bind = SUPER, bracketright, exec, rustr workspace forward
```

## Integration with Other Plugins

### Scratchpads Integration

```toml
# Scratchpads respect workspace following
[scratchpads.term]
follow_workspace = true         # Scratchpad follows workspace changes
workspace_specific = false      # Show on all workspaces vs. workspace-specific
```

### Expose Integration

```bash
# Show expose for current workspace only
rustr expose show-current       # Uses current workspace from follow focus

# Navigate between workspaces in expose mode
bind = SUPER, TAB, exec, rustr expose && rustr workspace-nav
```

### Wallpapers Integration

```toml
# Different wallpapers per workspace
[wallpapers]
workspace_specific = true       # Different wallpaper per workspace
workspace_wallpapers = {
    1 = "~/Pictures/desktop.jpg",
    2 = "~/Pictures/coding.jpg", 
    3 = "~/Pictures/web.jpg"
}
```

## Advanced Features

### Smart Workspace Creation

```toml
[workspaces_follow_focus.smart_creation]
# Automatically create workspaces based on patterns
auto_create_on_switch = true    # Create workspace when switching to non-existent
template_workspaces = true      # Use templates for new workspaces
inherit_layout = true           # Inherit layout from similar workspaces

# Workspace templates
[workspaces_follow_focus.templates]
"Dev" = {
    layout = "tiled",
    default_apps = ["code", "terminal", "browser"],
    monitor_preference = "DP-1"
}

"Media" = {
    layout = "floating", 
    default_apps = ["vlc", "spotify"],
    monitor_preference = "DP-2"
}
```

### Workspace Rules

```toml
[workspaces_follow_focus.rules]
# Application-specific workspace rules
app_workspace_rules = [
    { app = "firefox", workspace = 3, monitor = "DP-1" },
    { app = "code", workspace = 2, monitor = "DP-1" },
    { app = "spotify", workspace = 5, monitor = "DP-2" },
    { app = "discord", workspace = 4, monitor = "DP-2" }
]

# Window class rules
window_class_rules = [
    { class = "org.gnome.Nautilus", workspace = 6 },
    { class = "Gimp", workspace = 7, monitor = "DP-2" }
]
```

## Performance Optimization

### Efficient Workspace Switching

```toml
[workspaces_follow_focus.performance]
# Optimize workspace switching
cache_workspace_info = true     # Cache workspace information
lazy_workspace_loading = true   # Load workspace content on demand
batch_workspace_operations = true # Batch multiple operations

# Memory management
max_workspace_history = 50      # Limit history size
cleanup_empty_workspaces = true # Auto-remove empty workspaces
workspace_cleanup_interval = 300 # Cleanup every 5 minutes
```

### Multi-Monitor Performance

- **Intelligent Caching**: Workspace and monitor information cached
- **Event Batching**: Multiple workspace changes batched together
- **Lazy Loading**: Workspace content loaded only when needed
- **Memory Efficiency**: Efficient data structures for workspace tracking

## Troubleshooting

### Common Issues

**Workspace not switching:**
```bash
# Check current workspace status
rustr workspace status

# Verify Hyprland connection
rustr workspace current
```

**Focus following not working:**
```toml
[workspaces_follow_focus]
workspace_follows_focus = true  # Ensure this is enabled
focus_follow_threshold = 200    # Reduce threshold if too slow
```

**Cross-monitor switching issues:**
```toml
[workspaces_follow_focus]
cross_monitor_switching = true  # Enable cross-monitor switching
monitor_workspace_strategy = "shared" # Try shared strategy
```

### Debug Commands

```bash
# Debug workspace state
rustr workspace status          # Detailed status information
rustr workspace list-monitors   # Check monitor detection
rustr workspace history         # Check switching history

# Test workspace operations
rustr workspace switch 1        # Test basic switching
rustr workspace change +1       # Test relative navigation
```

## Migration from Other Tools

### From i3/Sway Workspaces
- Similar numeric workspace concepts
- Enhanced multi-monitor support
- Better focus following behavior

### From GNOME Activities
- More precise workspace control
- Better keyboard navigation
- Improved multi-monitor handling

### From Pyprland
- Full compatibility with existing configurations
- Enhanced focus following features
- Better performance and reliability