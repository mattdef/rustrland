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
plugins = ["scratchpads"]

[rustrland.variables]
term_classed = "foot --app-id"

[scratchpads.term]
command = "[term_classed] dropterm"
class = "dropterm"
size = "75% 60%"
animation = "fromTop"
```

See `examples/` for more configuration options.

### Usage

```bash
# Start daemon
rustrland --config ~/.config/hypr/rustrland.toml

# Use client commands
rustr toggle term        # Toggle terminal scratchpad
rustr toggle browser     # Toggle browser scratchpad
rustr list              # List available scratchpads
rustr status            # Check daemon status
```

### Keyboard Integration

Add to your `~/.config/hypr/hyprland.conf` for keyboard access:

```bash
bind = SUPER, grave, exec, rustr toggle term     # Super + ` 
bind = SUPER, B, exec, rustr toggle browser      # Super + B
bind = SUPER, F, exec, rustr toggle filemanager  # Super + F
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
- 🚧 **magnify**: Viewport zooming (planned)
- 🚧 **expose**: Window overview (planned)
- 🚧 **workspaces_follow_focus**: Multi-monitor improvements (planned)

## License

MIT License - see [LICENSE](LICENSE) for details.

## Contributing

Contributions welcome! See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.
