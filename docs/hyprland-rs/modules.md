# Hyprland-rs Module Documentation

## Module: `data`

**Purpose**: Retrieve information about the Hyprland compositor

### Key Structs

- **`Monitors`**: Display monitor information
- **`Workspaces`**: Workspace details and states
- **`Clients`**: Window/client information
- **`Version`**: Compositor version details
- **`CursorPosition`**: Current cursor location
- **`Devices`**: Input device information

### Key Enums

- **`AnimationStyle`**: Animation configuration options
- **`TabletType`**: Tablet device categorization
- **`Transforms`**: Display transformation types

### Usage Example

```rust
use hyprland::data::*;
use hyprland::prelude::*;

fn main() -> HResult<()> {
    let monitors = Monitors::get()?.to_vec();
    let workspaces = Workspaces::get()?.to_vec();
    let clients = Clients::get()?.to_vec();
    let version = Version::get()?;
    let cursor = CursorPosition::get()?;
    
    println!("Monitors: {}", monitors.len());
    println!("Workspaces: {}", workspaces.len());
    println!("Windows: {}", clients.len());
    
    Ok(())
}
```

---

## Module: `event_listener`

**Purpose**: Listen and react to Hyprland events in real-time

### Key Structs

- **`EventListener`**: Synchronous event listener
- **`AsyncEventListener`**: Asynchronous event listener  
- **`EventListenerMutable`**: Mutable event listener for stateful operations

### Event Types

- **`WindowEventData`**: Window-related events (open, close, move, resize)
- **`MonitorEventData`**: Monitor changes and configuration
- **`LayoutEvent`**: Layout switching and modifications
- **`WindowOpenEvent`**: Window creation events
- **Workspace Events**: Workspace switching and renaming
- **Screencast Events**: Screen recording and streaming

### Usage Example

```rust
use hyprland::event_listener::EventListener;
use hyprland::shared::HResult;

fn main() -> HResult<()> {
    let mut event_listener = EventListener::new();
    
    event_listener.add_window_open_handler(|data| {
        println!("Window opened: {}", data.window_class);
    });
    
    event_listener.add_workspace_changed_handler(|data| {
        println!("Switched to workspace: {}", data.workspace_name);
    });
    
    event_listener.start_listener()?;
    Ok(())
}
```

---

## Module: `dispatch`

**Purpose**: Execute Hyprland dispatchers and window management commands

### Key Structs

- **`Dispatch`**: Main dispatch interface

### Key Enums

- **`DispatchType`**: All available dispatcher commands
- **`Direction`**: Movement directions (Up, Down, Left, Right)
- **`WindowIdentifier`**: Methods to identify windows
- **`WorkspaceIdentifier`**: Methods to identify workspaces

### Common Dispatch Types

- **`Exec(command)`**: Execute external commands
- **`MoveWindow(direction)`**: Move windows in specified direction
- **`Workspace(identifier)`**: Switch to workspace
- **`ToggleFloating`**: Toggle window floating state
- **`Fullscreen`**: Toggle fullscreen mode
- **`CloseWindow`**: Close active window

### Usage Example

```rust
use hyprland::dispatch::{Dispatch, DispatchType, Direction, WorkspaceIdentifier};
use hyprland::shared::HResult;

fn main() -> HResult<()> {
    // Launch application
    Dispatch::call(DispatchType::Exec("kitty"))?;
    
    // Move window
    Dispatch::call(DispatchType::MoveWindow(Direction::Left))?;
    
    // Switch workspace
    Dispatch::call(DispatchType::Workspace(WorkspaceIdentifier::Id(2)))?;
    
    // Toggle floating
    Dispatch::call(DispatchType::ToggleFloating)?;
    
    Ok(())
}
```

---

## Module: `keyword`

**Purpose**: Read and modify Hyprland configuration keywords

### Key Structs

- **`Keyword`**: Configuration keyword interface

### Key Enums

- **`OptionValue`**: Possible configuration value types

### Main Functions

- **`Keyword::get(name)`**: Retrieve keyword value
- **`Keyword::set(name, value)`**: Set keyword value

### Usage Example

```rust
use hyprland::keyword::Keyword;
use hyprland::shared::HResult;

fn main() -> HResult<()> {
    // Get current value
    let border_size = Keyword::get("general:border_size")?;
    println!("Current border size: {}", border_size);
    
    // Set new value
    Keyword::set("general:border_size", "3")?;
    
    // Get animation setting
    let animations = Keyword::get("animations:enabled")?;
    println!("Animations enabled: {}", animations);
    
    Ok(())
}
```

---

## Module: `config::binds`

**Purpose**: Manage keybindings dynamically

### Functionality

- Add new keybindings at runtime
- Remove existing bindings
- Modify binding behavior
- Query current bindings

### Usage Context

This module is particularly useful for:
- Dynamic hotkey management
- Plugin-based binding systems
- User customization interfaces
- Temporary binding modifications

---

## Module: `ctl`

**Purpose**: Execute hyprctl commands for advanced control

### Submodules

- **`kill`**: Enter kill mode (similar to xkill)
- **`notify`**: Create system notifications
- **`output`**: Manage virtual outputs and displays
- **`plugin`**: Communicate with Hyprland plugins
- **`reload`**: Reload Hyprland configuration
- **`set_cursor`**: Set cursor theme and properties
- **`set_error`**: Create error messages for display
- **`set_prop`**: Set window and system properties
- **`switch_xkb_layout`**: Switch keyboard layouts

### Key Structs

- **`Color`**: 8-bit RGBA color representation

### Usage Example

```rust
use hyprland::ctl::*;
use hyprland::shared::HResult;

fn main() -> HResult<()> {
    // Create notification
    notify::notify("Title", "Message body", 5000)?;
    
    // Reload configuration
    reload::reload_config()?;
    
    // Set cursor theme
    set_cursor::set_cursor_theme("default")?;
    
    // Switch keyboard layout
    switch_xkb_layout::switch_layout(0, "us")?;
    
    Ok(())
}
```

---

## Module: `shared`

**Purpose**: Common types, utilities, and error handling

### Key Types

- **`HResult<T>`**: Result type for Hyprland operations
- **`HyprError`**: Error types for different failure modes
- **Common Traits**: Shared behavior across modules

### Error Handling

```rust
use hyprland::shared::{HResult, HyprError};

fn example_function() -> HResult<String> {
    match some_operation() {
        Ok(value) => Ok(value),
        Err(e) => Err(HyprError::CommandFailed(e.to_string())),
    }
}
```

---

## Module Integration Patterns

### Combining Modules

```rust
use hyprland::{data::*, dispatch::*, event_listener::*, shared::*};

fn advanced_window_manager() -> HResult<()> {
    // Get current state
    let clients = Clients::get()?;
    let workspaces = Workspaces::get()?;
    
    // Set up event monitoring
    let mut listener = EventListener::new();
    listener.add_window_open_handler(|event| {
        println!("New window: {}", event.window_class);
    });
    
    // Execute commands based on state
    if clients.len() > 10 {
        Dispatch::call(DispatchType::Workspace(WorkspaceIdentifier::Id(2)))?;
    }
    
    Ok(())
}
```

### Async Integration

```rust
use hyprland::{data::*, dispatch::*, event_listener::AsyncEventListener};
use tokio;

#[tokio::main]
async fn async_example() -> HResult<()> {
    let mut listener = AsyncEventListener::new();
    
    listener.add_window_open_handler(|event| async move {
        // Async event handling
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        println!("Async: Window opened - {}", event.window_class);
    });
    
    listener.start_listener().await?;
    Ok(())
}
```