# 🦀 Rustrland

A fast, reliable Rust implementation of Pyprland for Hyprland.

## Features

- **⚡ Fast**: Written in Rust for maximum performance
- **🔒 Reliable**: Memory-safe with comprehensive error handling  
- **🧩 Plugin-based**: Modular architecture with hot-reload support
- **🔄 Compatible**: Drop-in replacement for Pyprland configurations
- **📦 Easy deployment**: Single binary, no Python dependencies

## Quick Start

### Installation

```bash
cargo install --path .
```

### Configuration

Rustrland supports both legacy Pyprland format and native Rustrland format.

Create `~/.config/hypr/rustrland.toml` using either format:

**Pyprland-Compatible:**
```toml
[pyprland]
plugins = ["scratchpads"]

[pyprland.variables]
term_classed = "foot --app-id"

[scratchpads.term]
command = "[term_classed] dropterm"
class = "dropterm"
size = "75% 60%"
animation = "fromTop"
```

**Native Rustrland:**
```toml
[rustrland]
plugins = ["scratchpads", "expose", "workspaces_follow_focus"]

[rustrland.variables]
term_classed = "foot --app-id"

[scratchpads.term]
command = "[term_classed] dropterm"
class = "dropterm"
size = "75% 60%"
animation = "fromTop"

[expose]
padding = 20
scale = 0.2
show_titles = true

[workspaces_follow_focus]
follow_window_focus = true
allow_cross_monitor_switch = true
```

See `examples/` for more configuration options.

### Usage

```bash
# Start daemon
rustrland --config ~/.config/hypr/rustrland.toml

# Use client commands
rustr toggle term        # Toggle terminal scratchpad
rustr toggle browser     # Toggle browser scratchpad
rustr expose             # Show window overview (Mission Control style)
rustr expose next        # Navigate to next window in expose
rustr expose exit        # Exit expose mode
rustr workspace switch 2 # Switch to workspace 2 (moves to focused monitor)
rustr workspace change +1 # Switch to next workspace
rustr workspace list     # List all workspaces and monitors
rustr list              # List available scratchpads
rustr status            # Check daemon status
```

### Keyboard Integration

Add to your `~/.config/hypr/hyprland.conf` for keyboard access:

```bash
bind = SUPER, grave, exec, rustr toggle term     # Super + ` 
bind = SUPER, B, exec, rustr toggle browser      # Super + B
bind = SUPER, F, exec, rustr toggle filemanager  # Super + F
bind = SUPER, TAB, exec, rustr expose             # Super + Tab (Mission Control)
```

See [KEYBINDINGS.md](KEYBINDINGS.md) for complete setup guide.

## Development

```bash
# Run in development
cargo run -- --debug --foreground

# Run tests
cargo test

# Format code
cargo fmt

# Lint
cargo clippy
```

## Supported Plugins

- ✅ **scratchpads**: Dropdown terminals and applications
- ✅ **expose**: Window overview and navigation (macOS Mission Control style)
- ✅ **workspaces_follow_focus**: Multi-monitor workspace management and switching
- 🚧 **magnify**: Viewport zooming (planned)

## License

MIT License - see [LICENSE](LICENSE) for details.

## Contributing

Contributions welcome! See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.
