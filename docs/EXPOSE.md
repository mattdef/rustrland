# Expose Plugin

**Status**: ✅ Production Ready | **Tests**: Integrated

The Expose plugin provides a Mission Control-style window overview for Hyprland, allowing you to see all open windows at once in a special workspace and quickly switch between them.

## Overview

The Expose plugin implements a Pyprland-compatible approach using Hyprland's special workspace system. When activated, it moves all visible windows to a special workspace called `special:exposed`, where Hyprland automatically arranges them in a grid layout.

## Architecture

### Implementation Approach

The plugin uses **Hyprland's native special workspace system** rather than complex manual window positioning:

- **Special Workspace**: Uses `special:exposed` workspace for window overview
- **Native Layout**: Hyprland automatically handles window arrangement and scaling
- **Simple Commands**: Direct `hyprctl` commands for reliable window management
- **State Preservation**: Tracks original window positions for restoration

### Key Features

1. **Automatic Cleanup**: Detects and restores orphaned windows from previous sessions
2. **Event Handling**: Automatically exits expose mode on workspace changes or window closures
3. **Debug Logging**: Comprehensive logging for troubleshooting
4. **State Management**: Tracks window states and original workspaces
5. **Pyprland Compatibility**: Compatible with existing Pyprland configurations

## Configuration

Add the expose plugin to your `rustrland.toml` configuration:

```toml
[rustrland]
plugins = [
    # ... other plugins ...
    "expose",
]

[expose]
# Enable debug logging (default: false)
debug_logging = true

# Include windows from special workspaces (default: false)
include_special = false

# Target monitor for expose (empty = current focused monitor)
target_monitor = ""
```

### Configuration Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `debug_logging` | boolean | `false` | Enable detailed debug output for troubleshooting |
| `include_special` | boolean | `false` | Include windows from special workspaces in expose view |
| `target_monitor` | string | `""` | Target monitor name (empty uses current focused monitor) |

## Usage

### Commands

The expose plugin supports the following commands via the `rustr` client:

```bash
# Toggle expose mode (enter/exit)
rustr expose
rustr expose toggle

# Enter expose mode explicitly
rustr expose show
rustr expose enter

# Exit expose mode explicitly
rustr expose hide
rustr expose exit

# Check current status
rustr expose status
```

### Keyboard Integration

Add these keybindings to your `~/.config/hypr/hyprland.conf`:

```bash
# Expose keybinding (Mission Control style)
bind = SUPER, TAB, exec, rustr expose

# Alternative keybindings
bind = SUPER, E, exec, rustr expose        # Super + E
bind = ALT, TAB, exec, rustr expose        # Alt + Tab style
```

## Workflow

### Enter Expose Mode

1. **Window Detection**: Scans all open windows and filters out invalid ones
2. **State Storage**: Records original workspace and position of each window
3. **Workspace Activation**: Creates and shows `special:exposed` workspace
4. **Window Movement**: Moves all valid windows to the special workspace
5. **Grid Display**: Hyprland automatically arranges windows in a grid

### Exit Expose Mode

1. **Workspace Hiding**: Hides the `special:exposed` workspace
2. **Window Restoration**: Moves each window back to its original workspace
3. **Focus Restoration**: Returns to the original workspace
4. **State Cleanup**: Clears internal state tracking

### Automatic Cleanup

When the daemon starts, it automatically:

1. **Orphan Detection**: Checks if `special:exposed` contains abandoned windows
2. **Window Restoration**: Moves orphaned windows back to workspace 1
3. **Workspace Cleanup**: Hides the orphaned special workspace
4. **Log Reporting**: Reports cleanup actions in the logs

## Window Filtering

The plugin applies intelligent filtering to ensure only useful windows appear in expose mode:

### Included Windows
- **Mapped Windows**: Only visible, mapped windows
- **Normal Size**: Windows with reasonable dimensions (e 50x30 pixels)
- **Regular Workspaces**: Windows from normal workspaces (1, 2, 3, etc.)
- **Special Workspaces**: Only if `include_special = true`

### Excluded Windows
- **Invalid Geometry**: Windows with zero or negative dimensions
- **Tiny Windows**: Windows smaller than 50x30 pixels (likely system windows)
- **Unmapped Windows**: Hidden or minimized windows
- **Special Workspaces**: Excluded by default (configurable)

## Event Handling

The plugin automatically responds to Hyprland events:

### Auto-Exit Events
- **Window Closed**: Exits expose mode if any window is closed
- **Workspace Changed**: Exits expose mode if user switches workspaces manually

This ensures expose mode doesn't get "stuck" and maintains a clean user experience.

## Technical Implementation

### Core Methods

```rust
// Enter expose mode
async fn enter_expose(&mut self) -> Result<String>

// Exit expose mode and restore windows  
async fn exit_expose(&mut self) -> Result<String>

// Cleanup orphaned windows from previous sessions
async fn cleanup_orphaned_exposed_workspace(&mut self) -> Result<()>

// Get windows eligible for expose
async fn get_expose_windows(&self) -> Result<Vec<Client>>
```

### State Management

```rust
pub struct ExposeState {
    pub is_active: bool,                    // Whether expose mode is active
    pub original_workspace: i32,            // Workspace to return to
    pub original_windows: Vec<WindowState>, // Original window positions
    pub target_monitor: Option<String>,     // Target monitor name
}

pub struct WindowState {
    pub address: String,         // Hyprland window address
    pub original_workspace: i32, // Original workspace ID
    pub title: String,          // Window title for logging
}
```

### Command Execution

The plugin uses direct `hyprctl` commands for maximum reliability:

```rust
// Show special workspace
"hyprctl dispatch togglespecialworkspace exposed"

// Move window to exposed workspace
"hyprctl dispatch movetoworkspace special:exposed,address:{window.address}"

// Restore window to original workspace
"hyprctl dispatch movetoworkspacesilent {original_workspace},address:{window.address}"
```

## Troubleshooting

### Enable Debug Logging

Set `debug_logging = true` in your configuration to see detailed operation logs:

```toml
[expose]
debug_logging = true
```

### Common Issues

#### No Windows to Expose
- **Cause**: All windows are in special workspaces or too small
- **Solution**: Switch to a workspace with regular windows, or enable `include_special = true`

#### Windows Disappear
- **Cause**: Daemon restart while expose mode was active
- **Solution**: Automatic cleanup on daemon restart now handles this

#### Expose Won't Exit
- **Cause**: State corruption or command failure
- **Solution**: Restart daemon or manually run `hyprctl dispatch togglespecialworkspace exposed`

### Logs Analysis

Look for these log patterns:

```bash
# Successful expose activation
INFO <� Entering expose mode (Pyprland-compatible)
INFO  Cleaned up orphaned special:exposed workspace

# Window filtering
DEBUG Including window: Firefox [firefox] (1920x1080)
DEBUG Skipping tiny window: notification (10x10)

# Restoration process
DEBUG Restored window 'Terminal' to workspace 1
INFO =� Exiting expose mode
```

## Performance

### Optimizations

- **Direct Commands**: Uses `hyprctl` directly instead of API layers
- **Minimal State**: Only tracks essential window information
- **Event Filtering**: Handles only relevant Hyprland events
- **Async Operations**: Non-blocking window operations

### Memory Usage

- **Low Overhead**: Minimal memory footprint when inactive
- **State Cleanup**: Automatic cleanup prevents memory leaks
- **Efficient Storage**: Uses references where possible

## Compatibility

### Pyprland Migration

The plugin is designed for seamless migration from Pyprland:

- **Same Commands**: Uses identical command syntax (`expose`, `toggle`, etc.)
- **Compatible Configuration**: Accepts Pyprland-style config options
- **Similar Behavior**: Provides the same user experience

### Hyprland Versions

- **Minimum Version**: Requires Hyprland with special workspace support
- **Tested On**: Hyprland 0.45+ (current development versions)
- **Dependencies**: Uses stable Hyprland IPC commands

## Future Enhancements

### Planned Features

- **Animation Support**: Smooth transitions using the animation system
- **Custom Layouts**: Alternative arrangements beyond grid layout
- **Window Previews**: Enhanced window thumbnails
- **Monitor Awareness**: Better multi-monitor support

### Extensibility

The plugin architecture allows for:

- **Custom Filters**: Pluggable window filtering logic
- **Layout Engines**: Alternative arrangement algorithms  
- **Event Handlers**: Custom response to Hyprland events
- **Animation Integration**: Smooth transitions between states

## Example Session

```bash
# Start daemon
rustrland --debug --foreground

# Enter expose mode
rustr expose
#  Expose mode activated with 3 windows

# Check status
rustr expose status
#  Expose: Active | Windows: 3 | Original Workspace: 2

# Exit expose mode
rustr expose exit
#  Expose mode deactivated
```

The plugin provides a reliable, Pyprland-compatible expose functionality with robust error handling and automatic cleanup capabilities.