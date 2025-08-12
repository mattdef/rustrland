<div align="center">

<img src="docs/logo/rustrland_logo.png" alt="Rustrland Logo" width="200">

**A fast, reliable Rust implementation of Pyprland for Hyprland**

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org)
[![Hyprland](https://img.shields.io/badge/hyprland-compatible-blue.svg)](https://hyprland.org)

</div>

## Features

- **⚡ Fast**: Written in Rust for maximum performance and efficiency
- **🔒 Production-Ready**: Memory-safe with comprehensive error handling and 48+ tests
- **🧩 Plugin-based**: Modular architecture with hot-reload and animation systems
- **🔄 Compatible**: Drop-in replacement for Pyprland configurations
- **📦 Easy deployment**: Single binary, no Python dependencies
- **🎯 Multi-monitor**: Intelligent caching with 90% API call reduction
- **🔧 Enhanced IPC**: Robust reconnection logic and event filtering
- **🎨 Animation Support**: Complete animation framework with 16+ easing types

---

## Installation

### Prerequisites

- **Rust**: Version 1.70 or newer ([Install Rust](https://rustup.rs/))
- **Hyprland**: Compatible window manager ([Install Hyprland](https://hyprland.org/))
- **Environment**: `HYPRLAND_INSTANCE_SIGNATURE` must be set (usually automatic)

### Method 1: From Source (Recommended)

```bash
# Clone the repository
git clone https://github.com/mattdef/rustrland.git
cd rustrland

# Build and install
cargo install --path .
```

### Method 2: Development Build

```bash
# Clone and build in debug mode
git clone https://github.com/mattdef/rustrland.git
cd rustrland
cargo build

# Run directly from source
cargo run --bin rustrland -- --help
cargo run --bin rustr -- --help
```

### Method 3: Release Build

```bash
# Build optimized release version
cargo build --release

# Copy binaries to your PATH
sudo cp target/release/rustrland /usr/local/bin/
sudo cp target/release/rustr /usr/local/bin/
```

### Verify Installation

```bash
# Check daemon version and features
rustrland --version

# Check client connectivity  
rustr status

# Verify Hyprland connection
echo $HYPRLAND_INSTANCE_SIGNATURE  # Should show instance ID
```

## Configuration

### Overview

Rustrland supports **dual configuration formats** for maximum compatibility:

- **Pyprland Format**: `[pyprland]` - Full compatibility with existing Pyprland configs
- **Rustrland Format**: `[rustrland]` - Native format with enhanced features  
- **Dual Format**: Both sections in one file - Rustrland merges them intelligently

### Configuration Locations

Rustrland looks for configuration files in this order:

1. `--config` command line argument
2. `~/.config/hypr/rustrland.toml` (primary)
3. `~/.config/rustrland.toml` (alternative)
4. `./rustrland.toml` (current directory)

### Creating Your Configuration

Create your configuration file at `~/.config/hypr/rustrland.toml`:

#### Basic Pyprland-Compatible Configuration

```toml
[pyprland]
plugins = ["scratchpads"]

[pyprland.variables]
term_classed = "foot --app-id"
browser_class = "firefox"
filemanager_class = "thunar"

[scratchpads.term]
command = "[term_classed] dropterm"
class = "dropterm"
size = "75% 60%"
animation = "fromTop"

[scratchpads.browser]
command = "firefox --new-window --class=[browser_class]"
class = "[browser_class]"
size = "90% 85%"
position = "center"

[scratchpads.filemanager]  
command = "thunar --class=[filemanager_class]"
class = "[filemanager_class]"
size = "80% 70%"
position = "center"
```

#### Advanced Rustrland Native Configuration

```toml
[rustrland]
plugins = ["scratchpads", "expose", "workspaces_follow_focus", "magnify"]

[rustrland.variables]
term_classed = "foot --app-id"
browser_class = "firefox"
filemanager_class = "thunar"
music_class = "spotify"

# Enhanced scratchpad with all features
[scratchpads.term]
command = "[term_classed] dropterm"
class = "dropterm"
size = "75% 60%"
position = "center"
animation = "fromTop"
lazy = true
pinned = false
excludes = ["class:^org\\.gnome\\..*"]

# Multi-monitor aware scratchpad
[scratchpads.browser]
command = "firefox --new-window --class=[browser_class]"
class = "[browser_class]"
size = "90% 85%"
position = "center"
multi_monitor = true
preserve_aspect = true

# Mission Control-style expose
[expose]
padding = 20
scale = 0.2
show_titles = true
include_special = false
animation_duration = 300

# Multi-monitor workspace management  
[workspaces_follow_focus]
follow_window_focus = true
allow_cross_monitor_switch = true
workspace_switching_delay = 100

# Viewport magnification
[magnify]
factor = 2.0
duration = 300
smooth_animation = true
min_zoom = 1.0
max_zoom = 5.0
increment = 0.5
```

#### Dual Format Configuration

```toml
# Legacy Pyprland section (for compatibility)
[pyprland]
plugins = ["scratchpads"]

[pyprland.variables]
term_classed = "foot --app-id"

# Enhanced Rustrland section (overrides and extends)
[rustrland]
plugins = ["scratchpads", "expose", "workspaces_follow_focus"]

[rustrland.variables]
browser_class = "firefox"  # Additional variables

# Scratchpad definitions (shared between formats)
[scratchpads.term]
command = "[term_classed] dropterm"
class = "dropterm"
size = "75% 60%"
animation = "fromTop"

[scratchpads.browser]
command = "firefox --class=[browser_class]"
class = "[browser_class]"
size = "90% 85%"

[expose]
padding = 20
scale = 0.2
```

### Configuration Examples

The `examples/` directory contains ready-to-use configurations:

- `examples/pyprland-compatible.toml` - Drop-in Pyprland replacement
- `examples/rustrland-native.toml` - Full Rustrland features
- `examples/dual-config.toml` - Hybrid configuration approach
- `examples/advanced-animations.toml` - Animation system showcase

## Usage

### Starting Rustrland

#### As a Daemon (Recommended)

```bash
# Start with default config location
rustrland

# Start with specific config file
rustrland --config ~/.config/hypr/rustrland.toml

# Start in foreground with debug output
rustrland --debug --foreground

# Start as background service
rustrland --config ~/.config/hypr/rustrland.toml &
```

#### Auto-start with Hyprland

Add to your `~/.config/hypr/hyprland.conf`:

```bash
# Auto-start Rustrland daemon
exec-once = rustrland --config ~/.config/hypr/rustrland.toml
```

### Client Commands

#### Scratchpad Management

```bash
# Toggle scratchpads
rustr toggle term        # Toggle terminal scratchpad
rustr toggle browser     # Toggle browser scratchpad  
rustr toggle filemanager # Toggle file manager scratchpad
rustr toggle music       # Toggle music player scratchpad

# List and manage
rustr list              # List all available scratchpads
rustr status            # Check daemon status and uptime
```

#### Window Overview (Expose)

```bash
# Mission Control-style window overview
rustr expose             # Show all windows in grid
rustr expose next        # Navigate to next window
rustr expose prev        # Navigate to previous window
rustr expose select      # Select focused window
rustr expose exit        # Exit expose mode
```

#### Workspace Management

```bash
# Cross-monitor workspace switching
rustr workspace switch 1    # Switch to workspace 1 (on focused monitor)
rustr workspace switch 2    # Switch to workspace 2 (on focused monitor)
rustr workspace change +1   # Switch to next workspace
rustr workspace change -1   # Switch to previous workspace
rustr workspace list        # List all workspaces and monitors
```

#### Magnification/Zoom

```bash
# Viewport magnification
rustr magnify toggle        # Toggle zoom (1.0x ↔ 2.0x)
rustr magnify in           # Zoom in by increment (default 0.5)
rustr magnify out          # Zoom out by increment
rustr magnify set 3.0      # Set absolute zoom level
rustr magnify reset        # Reset zoom to 1.0x
rustr magnify status       # Show current zoom level
```

### Command Line Options

#### Daemon Options

```bash
rustrland --help
  -c, --config <FILE>     Configuration file path
  -d, --debug            Enable debug logging
  -f, --foreground       Run in foreground (don't daemonize)
  -v, --version          Show version information
```

#### Client Options

```bash
rustr --help
  -h, --help            Show help message
  -v, --version         Show version information
  --socket <PATH>       Custom socket path
```

## Keyboard Integration

### Basic Keybindings

Add these to your `~/.config/hypr/hyprland.conf` for essential functionality:

```bash
# Scratchpad keybindings
bind = SUPER, grave, exec, rustr toggle term        # Super + ` (backtick)
bind = SUPER, B, exec, rustr toggle browser         # Super + B
bind = SUPER, F, exec, rustr toggle filemanager     # Super + F
bind = SUPER, M, exec, rustr toggle music           # Super + M

# Window overview (Mission Control)
bind = SUPER, TAB, exec, rustr expose               # Super + Tab

# Workspace management
bind = SUPER, 1, exec, rustr workspace switch 1     # Super + 1
bind = SUPER, 2, exec, rustr workspace switch 2     # Super + 2
bind = SUPER, 3, exec, rustr workspace switch 3     # Super + 3
bind = SUPER, Right, exec, rustr workspace change +1 # Super + Right
bind = SUPER, Left, exec, rustr workspace change -1  # Super + Left

# Magnification
bind = SUPER, equal, exec, rustr magnify in          # Super + = (zoom in)
bind = SUPER, minus, exec, rustr magnify out         # Super + - (zoom out)
bind = SUPER, 0, exec, rustr magnify reset           # Super + 0 (reset)

# Utility
bind = SUPER, L, exec, rustr list                   # Super + L (list all)
bind = SUPER_SHIFT, S, exec, rustr status           # Super + Shift + S
```

### Advanced Keybindings

```bash
# Expose navigation
bind = SUPER, j, exec, rustr expose next             # Navigate expose
bind = SUPER, k, exec, rustr expose prev             # Navigate expose
bind = SUPER, Return, exec, rustr expose select      # Select in expose
bind = SUPER, Escape, exec, rustr expose exit        # Exit expose

# Advanced workspace management  
bind = SUPER_SHIFT, 1, exec, rustr workspace switch 1  # Force workspace 1
bind = SUPER_SHIFT, 2, exec, rustr workspace switch 2  # Force workspace 2
bind = SUPER_CTRL, Right, exec, rustr workspace list   # List workspaces

# Magnification control
bind = SUPER_SHIFT, equal, exec, rustr magnify set 2.0    # 2x zoom
bind = SUPER_SHIFT, 3, exec, rustr magnify set 3.0        # 3x zoom
bind = SUPER_SHIFT, 0, exec, rustr magnify toggle         # Toggle zoom
```

### Installation Script

For automatic keybinding setup:

```bash
# Download and run the keybinding setup script
curl -sSL https://raw.githubusercontent.com/mattdef/rustrland/master/scripts/install-keybindings.sh | bash
```

See [KEYBINDINGS.md](KEYBINDINGS.md) for complete setup guide and alternative key schemes.

## Troubleshooting

### Common Issues

#### Daemon won't start
```bash
# Check Hyprland environment
echo $HYPRLAND_INSTANCE_SIGNATURE  # Should show instance ID

# Verify Hyprland is running
hyprctl version

# Check configuration syntax
rustrland --config ~/.config/hypr/rustrland.toml --debug --foreground
```

#### Client can't connect
```bash
# Check if daemon is running
rustr status

# Check socket permissions
ls -la /tmp/rustrland-*.sock

# Restart daemon
pkill rustrland
rustrland &
```

#### Scratchpads not working
```bash
# Test application command manually
foot --app-id dropterm  # Should work outside Rustrland

# Check window class detection
hyprctl clients | grep -i dropterm

# Verify configuration
rustr list  # Should show your scratchpads
```

### Debug Mode

For detailed troubleshooting:

```bash
# Run daemon with debug output
rustrland --debug --foreground

# Check logs (if using systemd)
journalctl -f -u rustrland

# Verbose client output
rustr --debug toggle term
```

## Development

### Building from Source

```bash
# Clone and build
git clone https://github.com/mattdef/rustrland.git
cd rustrland
cargo build

# Run in development mode
cargo run --bin rustrland -- --debug --foreground

# Run client from source
cargo run --bin rustr -- --help
```

### Development Commands

```bash
# Run comprehensive tests (48+ tests)
cargo test

# Run specific test suites
cargo test --lib scratchpads       # 20 scratchpad tests
cargo test --lib animation         # 16 animation tests  
cargo test --lib enhanced_client   # Enhanced client tests

# Code quality
cargo fmt                          # Format code
cargo clippy -- -D warnings       # Lint (fails on warnings)

# Full CI pipeline
make ci                           # fmt + lint + test + build
```

### Project Structure

```
rustrland/
├── src/
│   ├── main.rs              # Daemon entry point
│   ├── client.rs            # Client entry point
│   ├── lib.rs               # Shared library
│   ├── config/              # Configuration system
│   ├── core/                # Core daemon and plugin management
│   │   ├── daemon.rs        # Main daemon loop
│   │   ├── plugin_manager.rs # Plugin loading and management
│   │   ├── event_handler.rs # Event processing
│   │   └── hot_reload.rs    # Hot reload system
│   ├── ipc/                 # IPC communication
│   │   ├── mod.rs           # Hyprland IPC client
│   │   ├── enhanced_client.rs # Production-ready client
│   │   ├── protocol.rs      # IPC message definitions
│   │   └── server.rs        # Unix socket server
│   ├── animation/           # Animation system
│   │   ├── timeline.rs      # Keyframe timelines
│   │   ├── easing.rs        # Easing functions
│   │   └── properties.rs    # Property interpolation
│   └── plugins/             # Plugin implementations
│       └── scratchpads.rs   # Production-ready scratchpads
├── examples/                # Configuration examples
├── tests/                   # Integration tests
└── docs/                    # Documentation
```

## Supported Plugins

- ✅ **scratchpads**: Production-ready dropdown terminals and applications with multi-monitor support
- ✅ **expose**: Mission Control-style window overview with grid layout and navigation
- ✅ **workspaces_follow_focus**: Multi-monitor workspace management with cross-monitor switching  
- ✅ **magnify**: Viewport zooming and magnification with smooth animations

### Plugin Status

| Plugin | Status | Tests | Features |
|--------|--------|-------|----------|
| scratchpads | ✅ Production | 20 tests | Multi-monitor, caching, reconnection |
| expose | ✅ Stable | Integrated | Grid layout, navigation, selection |
| workspaces_follow_focus | ✅ Stable | Integrated | Cross-monitor switching |
| magnify | ✅ Stable | Integrated | Smooth animations, external tools |
| hot_reload | 🔧 Available | Framework | File watching, state preservation |
| animation | 🔧 Available | 16 tests | Timelines, easing, interpolation |

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
