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

Create `~/.config/hypr/rustrland.toml`:

```toml
[pyprland]
plugins = ["scratchpads"]

[scratchpads.term]
command = "kitty --class dropterm"
class = "dropterm"
size = "75% 60%"
animation = "fromTop"
```

### Usage

```bash
# Start daemon
rustrland --config ~/.config/hypr/rustrland.toml

# Use client
rustr toggle term      # Toggle terminal scratchpad
rustr expose          # Show all windows
rustr status          # Check daemon status
```

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
- 🚧 **magnify**: Viewport zooming (planned)
- 🚧 **expose**: Window overview (planned)
- 🚧 **workspaces_follow_focus**: Multi-monitor improvements (planned)

## License

MIT License - see [LICENSE](LICENSE) for details.

## Contributing

Contributions welcome! See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.
