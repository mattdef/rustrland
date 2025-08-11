<div align="center">

# 🦀 Rustrland

<img src="docs/logo/rustrland_logo.png" alt="Rustrland Logo" width="200">

**A fast, reliable Rust implementation of Pyprland for Hyprland**

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org)
[![Hyprland](https://img.shields.io/badge/hyprland-compatible-blue.svg)](https://hyprland.org)

</div>

## Features

- **⚡ Fast**: Written in Rust for maximum performance
- **🔒 Reliable**: Memory-safe with comprehensive error handling  
- **🧩 Plugin-based**: Modular architecture with hot-reload support
- **🔄 Compatible**: Drop-in replacement for Pyprland configurations
- **📦 Easy deployment**: Single binary, no Python dependencies

---

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
plugins = ["scratchpads", "expose", "workspaces_follow_focus", "magnify"]

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

[magnify]
factor = 2.0
duration = 300
smooth_animation = true
min_zoom = 1.0
max_zoom = 5.0
increment = 0.5
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
rustr magnify toggle     # Toggle zoom (1.0x ↔ 2.0x)
rustr magnify in         # Zoom in by increment
rustr magnify set 3.0    # Set absolute zoom level
rustr magnify reset      # Reset zoom to 1.0x
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
- ✅ **magnify**: Viewport zooming and magnification with smooth animations

## License

MIT License - see [LICENSE](LICENSE) for details.

## Contributing

Contributions welcome! See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

---

<div align="center">
<img src="docs/logo/rustrland_logo.png" alt="Rustrland" width="100">
<br>
<strong>🦀 Rustrland - Rust-powered window management for Hyprland</strong>
</div>
