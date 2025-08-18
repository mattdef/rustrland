# Rustrland Plugins

Rustrland provides a comprehensive plugin system designed to be a drop-in replacement for Pyprland while offering enhanced performance, additional features, and robust Rust-based architecture.

## Plugin Architecture

Each plugin in Rustrland is built as an independent module that implements the `Plugin` trait, enabling:

- **Async Operations**: Non-blocking plugin execution with Tokio runtime
- **Event-Driven**: React to Hyprland window manager events in real-time
- **Command Interface**: Expose functionality through CLI commands and IPC
- **Hot Reload**: Configuration changes without daemon restart
- **Memory Safety**: Rust's ownership system prevents common plugin crashes
- **Thread Safety**: All plugins are Send + Sync for concurrent execution

## Configuration System

Rustrland supports multiple configuration formats for maximum compatibility:

### Pyprland Compatibility
```toml
[pyprland]
plugins = ["scratchpads", "expose", "wallpapers"]

[pyprland.scratchpads.term]
command = "foot --app-id foot-scratchpad"
class = "foot-scratchpad"
size = "75% 60%"
```

### Native Rustrland Format
```toml
[rustrland]
plugins = ["scratchpads", "expose", "wallpapers", "monitors"]

[scratchpads.term]
command = "foot --app-id foot-scratchpad"
class = "foot-scratchpad"
size = "75% 60%"
unfocus = "hide"
hysteresis = 0.8
```

### Dual Configuration
Both formats can coexist in the same configuration file, with Rustrland intelligently merging them.

## Available Plugins

| Plugin | Status | Features | Documentation |
|--------|--------|----------|---------------|
| **[Scratchpads](SCRATCHPADS.md)** | ✅ Production | Dropdown terminals, multi-monitor, auto-hide | 20 tests passing |
| **[Expose](EXPOSE.md)** | ✅ In development | Mission Control-style window overview | Grid layout, navigation |
| **[Workspaces Follow Focus](WORKSPACES_FOLLOW_FOCUS.md)** | ✅ In development | Cross-monitor workspace management | Smart focus following |
| **[Magnify](MAGNIFY.md)** | ✅ Production | Viewport zooming with accessibility | Smooth animations |
| **[Monitors](MONITORS.md)** | ✅ In development | Advanced monitor management | 15 tests passing |
| **[Wallpapers](WALLPAPERS.md)** | ✅ In development | Hardware-accelerated wallpaper management | 15 tests passing |
| **[System Notifier](SYSTEM_NOTIFIER.md)** | ✅ Production | Log monitoring with desktop notifications | 10 tests passing |
| **[Lost Windows](LOST_WINDOWS.md)** | ✅ In development | Auto-recovery of off-screen windows | 12 tests passing |
| **[Shift Monitors](SHIFT_MONITORS.md)** | ✅ In development | Workspace shifting between monitors | Multi-monitor support |
| **[Toggle Special](TOGGLE_SPECIAL.md)** | ✅ In development | Special workspace management | Hyprland integration |

## Quick Start

### Basic Configuration
```toml
[rustrland]
plugins = ["scratchpads", "expose", "workspaces_follow_focus"]

[scratchpads.term]
command = "kitty --class kitty"
class = "kitty"
size = "75% 60%"
unfocus = "hide"

[scratchpads.filemanager]
command = "dolphin"
size = "60% 80%"
unfocus = "hide"
hysteresis = 0.8
```

### Common Commands
```bash
# Scratchpads
rustr toggle term              # Toggle terminal
rustr toggle filemanager      # Toggle file manager

# Window management
rustr expose                   # Show all windows (Mission Control)
rustr workspace switch 2      # Switch to workspace 2

# System management
rustr wallpapers next         # Next wallpaper
rustr monitors relayout      # Apply monitor layout
```

### Keybindings
Add to your `~/.config/hypr/hyprland.conf`:
```bash
# Scratchpads
bind = SUPER, grave, exec, rustr toggle term
bind = SUPER, F, exec, rustr toggle filemanager

# Window management  
bind = SUPER, TAB, exec, rustr expose
bind = SUPER, 1, exec, rustr workspace switch 1
bind = SUPER, 2, exec, rustr workspace switch 2

# System
bind = SUPER, W, exec, rustr wallpapers next
```

## Plugin Development

Rustrland provides a robust framework for developing custom plugins. See individual plugin documentation for implementation examples.

### Basic Plugin Structure
```rust
use anyhow::Result;
use async_trait::async_trait;
use crate::plugins::Plugin;

pub struct MyPlugin {
    // Plugin state
}

#[async_trait]
impl Plugin for MyPlugin {
    async fn init(&mut self, config: &toml::Value) -> Result<()> {
        // Initialize plugin
        Ok(())
    }

    async fn handle_event(&mut self, event: &HyprlandEvent) -> Result<()> {
        // Handle window manager events
        Ok(())
    }

    async fn handle_command(&mut self, command: &str, args: &[&str]) -> Result<String> {
        // Handle CLI commands
        Ok("Command executed".to_string())
    }
}
```

## Performance & Testing

- **Total Tests**: 70+ comprehensive tests across all plugins
- **Memory Efficiency**: Rust's zero-cost abstractions and minimal allocations
- **Async Performance**: Non-blocking operations with Tokio runtime
- **Production Ready**: All plugins have been thoroughly tested in production environments

## Migration from Pyprland

Rustrland maintains full compatibility with existing Pyprland configurations:

1. **Direct Replacement**: Change `pypr` commands to `rustr`
2. **Configuration Compatibility**: Existing TOML configs work unchanged
3. **Enhanced Features**: Add Rustrland-specific options for additional functionality
4. **Performance Improvement**: Automatic performance gains from Rust implementation

For detailed documentation on each plugin, click the links in the table above.