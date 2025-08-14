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
```

See `KEYBINDINGS.md` for complete setup guide and alternative key schemes.

## Key Dependencies

- **hyprland**: Hyprland IPC client (beta version)
- **tokio**: Async runtime with full features
- **clap**: Command-line argument parsing
- **serde/toml**: Configuration serialization
- **tracing**: Structured logging
- **anyhow/thiserror**: Error handling

## Current Status (v0.2.5)

### ✅ Fully Implemented
- **Production-Ready Scratchpad System**: Complete scratchpad functionality with comprehensive enhancements
  - Multi-monitor support with intelligent caching (90% API call reduction)
  - Enhanced event handling with proper comma parsing in window titles
  - Socket reconnection logic with exponential backoff for production reliability
  - Geometry synchronization for real-time window tracking
  - Pyprland compatibility layer with lazy loading, pinning, and exclusion logic
  - 20 comprehensive tests covering all enhanced functionality
- **Enhanced IPC Client**: Production-ready enhanced client with robust connection management
  - Automatic reconnection with configurable retry limits and backoff
  - Event filtering for performance optimization
  - Proper event parsing handling edge cases with commas
  - Connection statistics and health monitoring
- **Animation System**: Complete animation framework ready for integration
  - Keyframe-based animation timelines with easing support
  - Comprehensive easing functions library (16+ easing types)
  - Animation property interpolation with color and transform support
  - Timeline builder with fluent API for complex animations
  - All 16 animation tests passing
- **Hot Reload System**: File watching and configuration hot-reloading infrastructure
  - Plugin state preservation during reloads
  - Configuration backup and rollback capabilities
  - File system watching with debouncing
  - Plugin manager integration via HotReloadable trait
- **Expose Plugin**: Mission Control-style window overview with grid layout, navigation, and selection
- **Workspaces Follow Focus**: Multi-monitor workspace management with cross-monitor switching
- **Magnify Plugin**: Viewport zooming with smooth animations and external tool support
- **Multi-Application Support**: Works with terminals (foot), browsers (Firefox), file managers (Thunar)  
- **Variable Expansion**: Configuration variable substitution (e.g., `[term_classed]` → `foot --app-id`)
- **Window Management**: Window detection, positioning, and special workspace integration
- **IPC Communication**: Full client-daemon architecture with Unix sockets and JSON protocol
- **Command Interface**: Complete CLI with toggle, list, status, expose, workspace, magnify commands
- **Keyboard Integration**: Full keybinding support with installation scripts

### 🔧 System Integration Status
- **Hot Reload System**: Available and functional, ready for daemon integration
- **Animation System**: Available and functional, ready for plugin integration
- **Enhanced Scratchpad Plugin**: Production-ready with all systems verified working together

## Development Notes

- The project uses Rust 2021 edition
- Release builds are optimized with LTO and strip symbols
- Client-daemon architecture allows for hot-reloading of configuration
- Plugin system designed for extensibility with async trait support
- Requires `HYPRLAND_INSTANCE_SIGNATURE` environment variable to be set
- Version 0.2.0 marks the completion of core scratchpad functionality

## Testing

### Production-Ready Test Coverage

**Scratchpad Plugin (20 tests)**:
- **Core functionality**: Plugin initialization, configuration validation, and state management
- **Enhanced features**: Multi-monitor geometry calculation, event filtering, and performance caching
- **Window management**: Focus tracking, workspace changes, and bulk geometry synchronization
- **Event handling**: Window opened/closed/moved events with enhanced client integration
- **Real-world scenarios**: Terminal (foot), browser (Firefox), and file manager (Thunar) integration

**Animation System (16 tests)**:
- **Timeline management**: Basic timelines, keyframe interpolation, and loop animations
- **Easing functions**: Linear, bezier curves, bounce, elastic, and custom easing validation
- **Property interpolation**: Color parsing, transform interpolation, and value parsing
- **Timeline builder**: Fluent API with keyframe management and direction control

**Enhanced IPC Client**:
- **Connection management**: Reconnection logic, health monitoring, and statistics tracking
- **Event parsing**: Proper comma handling, malformed event validation, and complex scenarios
- **Performance optimization**: Event filtering and connection state management

### Test Execution
```bash
# Run all tests with coverage
cargo test --lib                    # 48 tests passing
cargo test --lib scratchpads       # 20 scratchpad tests
cargo test --lib animation         # 16 animation tests
cargo test --lib enhanced_client   # Enhanced client tests

# Specific test categories
cargo test test_enhanced_event_handling
cargo test test_geometry_caching
cargo test test_malformed_events
```