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
  - `plugin_manager.rs`: Loads and manages plugins dynamically with Hyprland client injection
  - `event_handler.rs`: Processes Hyprland window manager events
- **Configuration (`src/config/`)**: TOML-based configuration system compatible with Pyprland syntax
- **IPC (`src/ipc/`)**: 
  - `mod.rs`: Hyprland IPC client with window management functions
  - `protocol.rs`: Client-daemon IPC message definitions
  - `server.rs`: Unix socket server for client-daemon communication
- **Plugins (`src/plugins/`)**: Modular plugin system with fully functional scratchpads plugin

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

# Lint with clippy (fails on warnings)
make lint
cargo clippy -- -D warnings

# Full CI pipeline
make ci  # Equivalent to: fmt lint test build
```

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

## Current Status (v0.2.2)

### ✅ Fully Implemented
- **Scratchpad System**: Complete scratchpad functionality with toggle, spawn, and positioning
- **Expose Plugin**: Mission Control-style window overview with grid layout, navigation, and selection
- **Workspaces Follow Focus**: Multi-monitor workspace management with cross-monitor switching
- **Multi-Application Support**: Works with terminals (foot), browsers (Firefox), file managers (Thunar)  
- **Variable Expansion**: Configuration variable substitution (e.g., `[term_classed]` → `foot --app-id`)
- **Window Management**: Window detection, positioning, and special workspace integration
- **IPC Communication**: Full client-daemon architecture with Unix sockets and JSON protocol
- **Command Interface**: Complete CLI with toggle, list, status, expose, workspace commands
- **Keyboard Integration**: Full keybinding support with installation scripts

### 🚧 Planned Features
- **magnify**: Window zoom functionality  
- **Animation support**: Implement animation configs (fromTop, fromRight, etc.)

## Development Notes

- The project uses Rust 2021 edition
- Release builds are optimized with LTO and strip symbols
- Client-daemon architecture allows for hot-reloading of configuration
- Plugin system designed for extensibility with async trait support
- Requires `HYPRLAND_INSTANCE_SIGNATURE` environment variable to be set
- Version 0.2.0 marks the completion of core scratchpad functionality

## Testing

Scratchpad functionality has been thoroughly tested with:
- **Terminal scratchpads**: foot terminal with proper app-id handling
- **Browser scratchpads**: Firefox with show/hide toggle functionality  
- **File manager scratchpads**: Thunar with window spawning and positioning
- **State management**: Proper window detection and visibility tracking
- **Configuration**: Variable expansion and multi-scratchpad support