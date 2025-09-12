# Hyprland-rs Documentation

This directory contains comprehensive documentation for the hyprland-rs library used in Rustrland.

## Overview

hyprland-rs is an unofficial Rust wrapper for Hyprland's Inter-Process Communication (IPC) system. It provides comprehensive tools for interacting with the Hyprland window manager programmatically.

- **Current Version**: 0.4.0-beta.2 (used in Rustrland)
- **Latest Stable**: 0.3.13
- **Maintainer**: [@yavko](https://github.com/yavko)
- **License**: GPL-3.0-or-later
- **Repository**: https://github.com/hyprland-community/hyprland-rs
- **Documentation**: https://docs.rs/hyprland/latest/hyprland/

## Installation

Add to Cargo.toml:
```toml
hyprland = "0.4.0-beta.2"
```

Or use the master branch for latest features:
```toml
hyprland = { git = "https://github.com/hyprland-community/hyprland-rs", branch = "master" }
```

## Documentation Files

- [overview.md](./overview.md) - Complete library overview and architecture
- [modules.md](./modules.md) - Detailed module documentation
- [examples.md](./examples.md) - Code examples and usage patterns
- [api-reference.md](./api-reference.md) - Complete API reference
- [integration-notes.md](./integration-notes.md) - Rustrland-specific integration notes

## Important Notes

⚠️ **Version Compatibility**: If something doesn't work, make sure you are on the latest version (or commit) of Hyprland before making an issue!

📖 **Documentation Coverage**: 99.25% documented

🔧 **Development**: Currently seeking help for version 0.4 development

## Quick Start

```rust
use hyprland::data::*;
use hyprland::dispatch::{Dispatch, DispatchType};
use hyprland::prelude::*;

fn main() -> HResult<()> {
    // Get compositor information
    let monitors = Monitors::get()?.to_vec();
    let workspaces = Workspaces::get()?.to_vec();
    let clients = Clients::get()?.to_vec();
    
    // Execute commands
    Dispatch::call(DispatchType::Exec("kitty"))?;
    
    Ok(())
}
```

## Community

- **GitHub**: https://github.com/hyprland-community/hyprland-rs
- **Discord**: https://discord.gg/zzWqvcKRMy
- **Documentation**: https://docs.rs/hyprland