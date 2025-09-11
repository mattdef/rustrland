# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Rustrland is a Rust implementation of Pyprland for Hyprland - a fast, reliable plugin system that provides dropdown terminals, window management, and other window manager enhancements. It's designed as a drop-in replacement for Pyprland with better performance and memory safety.

## Architecture

The codebase follows a modular architecture with clear separation of concerns:

- **Daemon (`src/main.rs`)**: Main daemon process that runs continuously, manages Hyprland connections and coordinates plugins
- **Client (`src/client.rs`)**: Command-line client (`rustr`) for sending commands to the running daemon via IPC
- **Library (`src/lib.rs`)**: Shared library with common types and IPC protocol
- **Core System (`src/core/`)**: 
  - `daemon.rs`: Core daemon lifecycle and event loop management
  - `plugin_manager.rs`: Loads and manages plugins dynamically with Hyprland client injection and hot reload support
  - `event_handler.rs`: Processes Hyprland window manager events
  - `hot_reload.rs`: File watching and configuration hot-reloading system with plugin state preservation
- **Configuration (`src/config/`)**: TOML-based configuration system compatible with Pyprland syntax
- **IPC (`src/ipc/`)**: 
  - `mod.rs`: Hyprland IPC client with window management functions
  - `enhanced_client.rs`: Production-ready enhanced client with reconnection logic and performance optimizations
  - `protocol.rs`: Client-daemon IPC message definitions
  - `server.rs`: Unix socket server for client-daemon communication
- **Animation (`src/animation/`)**: Comprehensive animation system for smooth transitions
  - `timeline.rs`: Keyframe-based animation timelines with easing support
  - `easing.rs`: Complete easing functions library (linear, bezier, bounce, elastic, etc.)
  - `properties.rs`: Animation property interpolation with color and transform support
- **Plugins (`src/plugins/`)**: Modular plugin system with production-ready scratchpads plugin

## Common Commands

### Development
```bash
# Run in development mode with debug output
make run
cargo run --bin rustrland -- --debug --foreground

# Run with example configuration
make run-example
cargo run --bin rustrland -- --config examples/rustrland.toml --debug --foreground

# Auto-reload during development (requires cargo-watch)
make dev

# Run client commands
cargo run --bin rustr -- toggle term        # Toggle terminal scratchpad
cargo run --bin rustr -- toggle browser     # Toggle browser scratchpad  
cargo run --bin rustr -- toggle filemanager # Toggle file manager scratchpad
cargo run --bin rustr -- expose             # Show window overview (Mission Control)
cargo run --bin rustr -- expose next        # Navigate to next window
cargo run --bin rustr -- expose exit        # Exit expose mode
cargo run --bin rustr -- workspace switch 2 # Switch to workspace 2 (cross-monitor)
cargo run --bin rustr -- workspace change +1 # Next workspace
cargo run --bin rustr -- workspace list     # List workspaces and monitors
cargo run --bin rustr -- list               # List all available scratchpads
cargo run --bin rustr -- status             # Show daemon status and uptime
```

### Build and Test
```bash
# Build project
make build
cargo build

# Run tests
make test  
cargo test

# Check compilation without building
make check
cargo check
```

### Code Quality
```bash
# Format code
make fmt
cargo fmt

# Check format without changing files
make fmt-check
cargo fmt --check

# Lint with clippy (fails on warnings)
make lint
cargo clippy --lib --bins -- -D warnings

# Full CI pipeline (matches GitHub CI exactly)
make ci  # Equivalent to: fmt-check lint test build-release

# Pre-push validation (run before every push)
./scripts/pre-push.sh
```

### **IMPORTANT: Pre-Push Validation**

**Always run pre-push validation before pushing to GitHub:**

```bash
# Option 1: Use the make target
make ci

# Option 2: Use the dedicated script  
./scripts/pre-push.sh

# Option 3: Git pre-push hook (automatic)
# Hook is already installed and runs automatically on git push
```

**The pre-push hook will automatically run these checks:**
- ✅ Code formatting (`cargo fmt --check`)
- ✅ Clippy linting (`cargo clippy --lib --bins -- -D warnings`) 
- ✅ Unit tests (`cargo test --lib`)
- ✅ Release build (`cargo build --release`)

**If any check fails, the push is blocked.** This ensures professional code quality.

### Installation
```bash
# Install locally
make install
cargo install --path .
```

## Configuration

Rustrland supports dual configuration formats for maximum compatibility:

### Configuration Formats
- **Pyprland Format**: `[pyprland]` - Full compatibility with existing Pyprland configs
- **Rustrland Format**: `[rustrland]` - Native format with enhanced features  
- **Dual Format**: Both sections in one file - Rustrland merges them intelligently

### Configuration Files
- Default location: `~/.config/hypr/rustrland.toml`
- Example configurations: `examples/pyprland-compatible.toml`, `examples/rustrland-native.toml`, `examples/dual-config.toml`
- Supports variable substitution with `[variable_name]` syntax in both formats

### Configuration Merging Rules
- Both `[pyprland]` and `[rustrland]` sections are processed
- Plugin lists are merged (duplicates removed)
- Variables from `[rustrland.variables]` override `[pyprland.variables]`
- Existing Pyprland configurations work without modification

## Keyboard Integration

For seamless usage, add these keybindings to your `~/.config/hypr/hyprland.conf`:

```bash
# Rustrland Scratchpad Keybindings
bind = SUPER, grave, exec, rustr toggle term        # Super + ` (backtick)
bind = SUPER, B, exec, rustr toggle browser         # Super + B
bind = SUPER, F, exec, rustr toggle filemanager     # Super + F  
bind = SUPER, M, exec, rustr toggle music           # Super + M
bind = SUPER, TAB, exec, rustr expose               # Super + Tab (expose)
bind = SUPER, 1, exec, rustr workspace switch 1     # Super + 1 (workspace 1)
bind = SUPER, 2, exec, rustr workspace switch 2     # Super + 2 (workspace 2)
bind = SUPER, Right, exec, rustr workspace change +1 # Super + Right (next workspace)
bind = SUPER, Left, exec, rustr workspace change -- -1 # Super + Left (prev workspace)
bind = SUPER, L, exec, rustr list                   # Super + L (list all)
bind = SUPER_SHIFT, S, exec, rustr status           # Super + Shift + S
bind = SUPER_SHIFT, R, exec, rustr reload           # Super + Shift + R (hot reload)
```

See `KEYBINDINGS.md` for complete setup guide and alternative key schemes.

## API Reference

**⚠️ BEFORE IMPLEMENTING: Check this section for existing functions to avoid duplication**

### Animation System (`src/animation/`)

#### Core Engine
- `AnimationEngine::new()` - Create animation engine
- `AnimationEngine::start_animation()` - Start new animation
- `AnimationEngine::stop_animation()` - Stop animation by ID
- `AnimationEngine::pause_animation()` - Pause/resume animation
- `AnimationEngine::get_current_properties()` - Get animation properties
- `AnimationEngine::is_easing_supported()` - Check easing support
- `AnimationEngine::get_supported_easings()` - List all easing functions
- `AnimationEngine::get_performance_stats()` - Get performance metrics

#### Easing Functions
- `EasingFunction::from_name()` - Parse easing from string (36+ types)
- `EasingFunction::apply()` - Apply easing to progress value

#### Timeline System
- `Timeline::new()` - Create timeline with duration
- `Timeline::with_keyframes()` - Timeline with initial keyframes
- `Timeline::add_keyframe()` - Add keyframe at time
- `Timeline::get_progress()` - Get current progress
- `Timeline::get_value_at_progress()` - Interpolate value
- `Timeline::fade_timeline()` - Pre-built fade animation
- `Timeline::scale_timeline()` - Pre-built scale animation
- `Timeline::slide_timeline()` - Pre-built slide animation
- `Timeline::bounce_timeline()` - Pre-built bounce animation
- `Timeline::elastic_timeline()` - Pre-built elastic animation
- `TimelineBuilder::new()` - Fluent timeline builder

#### Property System
- `PropertyValue::interpolate()` - Interpolate between values
- `PropertyValue::from_string()` - Parse property from string
- `Color::new()` - Create RGBA color
- `Color::interpolate()` - Smooth color transitions
- `Color::from_rgb_string()` / `from_rgba_string()` / `from_hex_string()` - Color parsing
- `Transform::new()` - Create 2D transform
- `Transform::interpolate()` - Transform interpolation

#### Window Animator
- `WindowAnimator::new()` - Create window animator
- `WindowAnimator::is_animating()` - Check if window animating
- `WindowAnimator::calculate_offscreen_position()` - Calculate positions

### IPC System (`src/ipc/`)

#### Enhanced Client
- `EnhancedHyprlandClient::new()` - Create enhanced client with reconnection
- `EnhancedHyprlandClient::get_hyprland_instance()` - Get Hyprland socket info

#### Core Client
- `HyprlandClient::new()` - Basic Hyprland client
- `HyprlandClient::connect()` - Connect to Hyprland
- `HyprlandClient::get_windows()` - Get window list
- `HyprlandClient::get_monitors()` - Get monitor info
- `HyprlandClient::dispatch()` - Send Hyprland command

#### Protocol
- `get_socket_path()` - Get IPC socket path
- `ClientMessage::from_args()` - Parse client command

#### Server
- `IpcServer::new()` - Create IPC server for daemon

### Configuration System (`src/config/`)

#### Config Management
- `Config::from_toml_value()` - Parse TOML configuration
- `Config::get_plugins()` - Get enabled plugins list
- `Config::get_variables()` - Get config variables
- `Config::uses_rustrland_config()` - Check format type
- `Config::uses_pyprland_config()` - Check format type

#### Data Structures
- `RustrlandConfig` - Native config format
- `PyprlandConfig` - Pyprland compatibility format

### Core System (`src/core/`)

#### Daemon Management
- `Daemon::new()` - Create daemon with config path
- `Daemon::run()` - Start daemon event loop

#### Plugin Management
- `PluginManager::new()` - Create plugin manager
- `PluginManager::load_plugins()` - Load plugins from config
- `PluginManager::handle_event()` - Process Hyprland events
- `PluginManager::handle_command()` - Process client commands
- `PluginManager::get_plugin_count()` - Get loaded plugin count
- `PluginManager::get_global_cache()` - Access shared cache

#### Hot Reload System
- `HotReloadManager::new()` - Create hot reload manager
- `HotReloadManager::start()` - Start file watching
- `HotReloadManager::reload_now()` - Manual reload trigger
- `HotReloadManager::stop()` - Stop file watching
- `HotReloadManager::get_stats()` - Get reload statistics
- `HotReloadManager::subscribe()` - Subscribe to reload events
- `HotReloadable` trait - Plugin hot reload interface
- `ConfigExt` trait - Config extension methods

#### Global Cache System
- `GlobalStateCache::new()` - Create global cache
- `GlobalStateCache::get_monitor()` - Get monitor by name
- `GlobalStateCache::get_workspace()` - Get workspace by ID
- `GlobalStateCache::update_monitors()` - Update monitor cache
- `GlobalStateCache::is_cache_valid()` - Check cache validity
- `GlobalStateCache::get_monitor_cache()` - Access monitor cache
- `GlobalStateCache::get_workspace_cache()` - Access workspace cache
- `GlobalStateCache::store_config()` - Store plugin config
- `GlobalStateCache::get_config()` - Retrieve plugin config
- `GlobalStateCache::store_variables()` - Store config variables
- `GlobalStateCache::get_variables()` - Access config variables
- `GlobalStateCache::get_memory_stats()` - Get memory usage stats

#### Event Handling
- `EventHandler::new()` - Create event handler
- `EventHandler::handle_event()` - Process system events

## Key Dependencies

- **hyprland**: Hyprland IPC client (beta version)
- **tokio**: Async runtime with full features
- **clap**: Command-line argument parsing
- **serde/toml**: Configuration serialization
- **tracing**: Structured logging
- **anyhow/thiserror**: Error handling

## Hyprland-rs Library Documentation

**📚 Complete hyprland-rs documentation is available in [`docs/hyprland-rs/`](docs/hyprland-rs/)**

### Quick Reference
- **[Overview](docs/hyprland-rs/overview.md)**: Library architecture and core concepts
- **[Module Documentation](docs/hyprland-rs/modules.md)**: Detailed module APIs and usage patterns
- **[Examples](docs/hyprland-rs/examples.md)**: Practical code examples and common patterns
- **[API Reference](docs/hyprland-rs/api-reference.md)**: Complete function signatures and types
- **[Integration Notes](docs/hyprland-rs/integration-notes.md)**: Rustrland-specific integration patterns

### Key hyprland-rs Modules Used in Rustrland
- **`data`**: Monitor, workspace, and window information retrieval
- **`dispatch`**: Window management and command execution
- **`event_listener`**: Real-time Hyprland event monitoring
- **`keyword`**: Dynamic configuration management
- **`ctl`**: Advanced Hyprland control operations

### Version Information
- **Current**: hyprland-rs 0.4.0-beta.2
- **Repository**: https://github.com/hyprland-community/hyprland-rs
- **Documentation**: https://docs.rs/hyprland/latest/hyprland/

### 🔧 System Integration Status
- **Hot Reload System**: ✅ **FULLY INTEGRATED** - Production-ready with daemon integration complete
- **Animation System**: Available and functional, ready for plugin integration

## Development Notes

- The project uses Rust 2021 edition
- Release builds are optimized with LTO and strip symbols
- Client-daemon architecture allows for hot-reloading of configuration
- Plugin system designed for extensibility with async trait support
- Requires `HYPRLAND_INSTANCE_SIGNATURE` environment variable to be set