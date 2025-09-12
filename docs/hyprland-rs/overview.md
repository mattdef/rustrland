# Hyprland-rs Library Overview

## About

hyprland-rs is an unofficial Rust wrapper for Hyprland's Inter-Process Communication (IPC) system. It provides a comprehensive, type-safe interface for interacting with the Hyprland window manager from Rust applications.

## Architecture

The library is organized into 6 main modules (+1 for shared functionality):

### Core Modules

1. **`data`** - Retrieving compositor information
2. **`event_listener`** - Event handling and monitoring  
3. **`dispatch`** - Calling window management dispatchers
4. **`keyword`** - Managing configuration values
5. **`config::binds`** - Modifying keybindings
6. **`ctl`** - Executing hyprctl commands

### Shared Module

- **`shared`** - Common types, error handling, and utilities

## Key Features

### Data Retrieval
- Monitor information and properties
- Workspace details and states  
- Client (window) information
- Active window tracking
- Layer surface management
- Input device information
- Compositor version and status
- Cursor position tracking

### Event System
- Real-time event listening with `EventListener`
- Asynchronous event handling with `AsyncEventListener`
- Mutable event listeners for stateful operations
- Comprehensive event types covering all Hyprland operations

### Dispatch System
- Execute Hyprland commands programmatically
- Window management operations
- Workspace switching and management
- Application launching
- Layout modifications

### Configuration Management
- Read and modify Hyprland configuration keywords
- Dynamic configuration updates
- Keybinding management

### Control Functions
- Kill mode functionality
- Notification system
- Virtual output management
- Plugin communication
- Configuration reloading
- Cursor theme management
- Property setting

## Type System

hyprland-rs includes extensive type definitions:

- **Window Identifiers**: Various ways to reference windows
- **Workspace Identifiers**: Methods to specify workspaces
- **Direction Enums**: For movement operations
- **Animation Styles**: Animation configuration options
- **Device Types**: Input device categorization
- **Transform Types**: Display transformation options

## Async Support

The library provides both synchronous and asynchronous APIs:

- Sync: Direct blocking operations
- Async: Non-blocking operations with futures support
- Compatible with tokio, async-std, and other async runtimes

## Error Handling

Comprehensive error handling with:
- `HResult<T>` type for operation results
- Detailed error types for different failure modes
- Helpful error messages for debugging

## Platform Support

- **Primary**: x86_64-unknown-linux-gnu
- **Additional**: i686-unknown-linux-gnu, x86_64-apple-darwin
- **Requirement**: Hyprland window manager

## Dependencies

### Core Dependencies
- Serde for serialization
- Tokio for async operations (optional)
- Unix socket communication

### Optional Features
- `async-net`: Async networking support
- `async-std`: Async standard library support  
- `tokio`: Tokio runtime support
- `futures`: Futures utilities

## Version Information

- **Current Stable**: 0.3.13
- **Beta Version**: 0.4.0-beta.2 (used in Rustrland)
- **Development**: Active development on master branch
- **Breaking Changes**: Version 0.4 introduces significant API improvements

## Documentation Quality

- **Coverage**: 99.25% of public APIs documented
- **Examples**: Comprehensive example collection
- **API Docs**: Complete docs.rs documentation
- **Community**: Active Discord and GitHub support

## Integration Notes

The library is designed for:
- Window manager enhancement tools
- System monitoring applications  
- Automation scripts
- Custom desktop environments
- Plugin development
- Configuration management tools

## Performance Characteristics

- **Low Overhead**: Minimal resource usage
- **Fast IPC**: Efficient Unix socket communication
- **Type Safety**: Zero-cost abstractions
- **Memory Efficient**: Careful memory management
- **Hot Path Optimization**: Critical paths optimized for performance