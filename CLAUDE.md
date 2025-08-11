# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Rustrland is a Rust implementation of Pyprland for Hyprland - a fast, reliable plugin system that provides dropdown terminals, window management, and other window manager enhancements. It's designed as a drop-in replacement for Pyprland with better performance and memory safety.

## Architecture

The codebase follows a modular architecture with clear separation of concerns:

- **Daemon (`src/main.rs`)**: Main daemon process that runs continuously, manages Hyprland connections and coordinates plugins
- **Client (`src/client.rs`)**: Command-line client (`rustr`) for sending commands to the running daemon via IPC
- **Core System (`src/core/`)**: 
  - `daemon.rs`: Core daemon lifecycle and event loop management
  - `plugin_manager.rs`: Loads and manages plugins dynamically
  - `event_handler.rs`: Processes Hyprland window manager events
- **Configuration (`src/config/`)**: TOML-based configuration system compatible with Pyprland syntax
- **IPC (`src/ipc/`)**: Hyprland IPC client for window manager communication
- **Plugins (`src/plugins/`)**: Modular plugin system (currently implements scratchpads)

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
cargo run --bin rustr -- toggle term
cargo run --bin rustr -- status
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

The daemon uses TOML configuration files compatible with Pyprland:
- Default location: `~/.config/hypr/rustrland.toml`
- Example configuration: `examples/rustrland.toml`
- Supports variable substitution and plugin-specific sections

## Key Dependencies

- **hyprland**: Hyprland IPC client (beta version)
- **tokio**: Async runtime with full features
- **clap**: Command-line argument parsing
- **serde/toml**: Configuration serialization
- **tracing**: Structured logging
- **anyhow/thiserror**: Error handling

## Development Notes

- The project uses Rust 2021 edition
- Release builds are optimized with LTO and strip symbols
- Client-daemon architecture allows for hot-reloading of configuration
- Plugin system designed for extensibility (scratchpads implemented, magnify/expose planned)
- Requires `HYPRLAND_INSTANCE_SIGNATURE` environment variable to be set