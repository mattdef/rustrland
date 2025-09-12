# Hyprland-rs Examples

This document contains practical examples demonstrating how to use the hyprland-rs library effectively.

## Basic Data Retrieval

### Getting Monitor Information

```rust
use hyprland::data::Monitors;
use hyprland::prelude::*;

fn main() -> HResult<()> {
    let monitors = Monitors::get()?;
    
    for monitor in monitors {
        println!("Monitor: {}", monitor.name);
        println!("  Resolution: {}x{}", monitor.width, monitor.height);
        println!("  Position: ({}, {})", monitor.x, monitor.y);
        println!("  Scale: {}", monitor.scale);
        println!("  Refresh Rate: {}Hz", monitor.refresh_rate);
        println!("  Active Workspace: {}", monitor.active_workspace.id);
    }
    
    Ok(())
}
```

### Getting Workspace Information

```rust
use hyprland::data::Workspaces;
use hyprland::prelude::*;

fn main() -> HResult<()> {
    let workspaces = Workspaces::get()?;
    
    for workspace in workspaces {
        println!("Workspace {}: {}", workspace.id, workspace.name);
        println!("  Monitor: {}", workspace.monitor);
        println!("  Windows: {}", workspace.windows);
        println!("  Has Fullscreen: {}", workspace.has_fullscreen);
    }
    
    Ok(())
}
```

### Getting Window/Client Information

```rust
use hyprland::data::Clients;
use hyprland::prelude::*;

fn main() -> HResult<()> {
    let clients = Clients::get()?;
    
    for client in clients {
        println!("Window: {}", client.title);
        println!("  Class: {}", client.class);
        println!("  PID: {}", client.pid);
        println!("  Position: ({}, {})", client.at.0, client.at.1);
        println!("  Size: {}x{}", client.size.0, client.size.1);
        println!("  Workspace: {}", client.workspace.id);
        println!("  Floating: {}", client.floating);
        println!("  Fullscreen: {}", client.fullscreen);
    }
    
    Ok(())
}
```

## Event Listening

### Basic Event Listener

```rust
use hyprland::event_listener::EventListener;
use hyprland::shared::HResult;

fn main() -> HResult<()> {
    let mut event_listener = EventListener::new();
    
    // Window events
    event_listener.add_window_open_handler(|event| {
        println!("🪟 Window opened: {} ({})", event.window_title, event.window_class);
    });
    
    event_listener.add_window_close_handler(|event| {
        println!("❌ Window closed: {}", event.window_address);
    });
    
    // Workspace events
    event_listener.add_workspace_changed_handler(|event| {
        println!("🔄 Switched to workspace: {}", event.workspace_name);
    });
    
    // Monitor events
    event_listener.add_monitor_added_handler(|event| {
        println!("🖥️ Monitor added: {}", event.monitor_name);
    });
    
    println!("Starting event listener...");
    event_listener.start_listener()?;
    
    Ok(())
}
```

### Async Event Listener

```rust
use hyprland::event_listener::AsyncEventListener;
use hyprland::shared::HResult;
use tokio;

#[tokio::main]
async fn main() -> HResult<()> {
    let mut event_listener = AsyncEventListener::new();
    
    event_listener.add_window_open_handler(|event| async move {
        // Simulate async work
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        println!("🪟 Async: Window opened - {}", event.window_class);
        
        // Could perform async operations like database updates, API calls, etc.
    });
    
    event_listener.add_workspace_changed_handler(|event| async move {
        println!("🔄 Async: Workspace changed to {}", event.workspace_name);
    });
    
    println!("Starting async event listener...");
    event_listener.start_listener().await?;
    
    Ok(())
}
```

## Dispatch Operations

### Window Management

```rust
use hyprland::dispatch::{Dispatch, DispatchType, Direction, WindowIdentifier};
use hyprland::shared::HResult;

fn main() -> HResult<()> {
    // Launch applications
    Dispatch::call(DispatchType::Exec("kitty"))?;
    Dispatch::call(DispatchType::Exec("firefox"))?;
    
    // Wait for windows to open
    std::thread::sleep(std::time::Duration::from_millis(1000));
    
    // Move windows
    Dispatch::call(DispatchType::MoveWindow(Direction::Left))?;
    Dispatch::call(DispatchType::MoveWindow(Direction::Right))?;
    
    // Resize windows
    Dispatch::call(DispatchType::ResizeWindow(Direction::Up))?;
    
    // Toggle floating mode
    Dispatch::call(DispatchType::ToggleFloating)?;
    
    // Focus specific window by class
    Dispatch::call(DispatchType::FocusWindow(WindowIdentifier::ClassRegex("kitty".to_string())))?;
    
    Ok(())
}
```

### Workspace Management

```rust
use hyprland::dispatch::{Dispatch, DispatchType, WorkspaceIdentifier};
use hyprland::shared::HResult;

fn main() -> HResult<()> {
    // Switch to specific workspaces
    Dispatch::call(DispatchType::Workspace(WorkspaceIdentifier::Id(1)))?;
    std::thread::sleep(std::time::Duration::from_millis(500));
    
    Dispatch::call(DispatchType::Workspace(WorkspaceIdentifier::Id(2)))?;
    std::thread::sleep(std::time::Duration::from_millis(500));
    
    // Switch to workspace by name
    Dispatch::call(DispatchType::Workspace(WorkspaceIdentifier::Name("web".to_string())))?;
    
    // Move window to workspace
    Dispatch::call(DispatchType::MoveToWorkspace(WorkspaceIdentifier::Id(3)))?;
    
    // Move window to workspace and follow
    Dispatch::call(DispatchType::MoveToWorkspaceSilent(WorkspaceIdentifier::Id(4)))?;
    
    Ok(())
}
```

## Configuration Management

### Reading and Setting Keywords

```rust
use hyprland::keyword::Keyword;
use hyprland::shared::HResult;

fn main() -> HResult<()> {
    // Read current configuration values
    let border_size = Keyword::get("general:border_size")?;
    let gaps_inner = Keyword::get("general:gaps_in")?;
    let gaps_outer = Keyword::get("general:gaps_out")?;
    
    println!("Current Configuration:");
    println!("  Border Size: {}", border_size);
    println!("  Inner Gaps: {}", gaps_inner);
    println!("  Outer Gaps: {}", gaps_outer);
    
    // Modify configuration
    Keyword::set("general:border_size", "3")?;
    Keyword::set("general:gaps_in", "8")?;
    Keyword::set("general:gaps_out", "16")?;
    
    // Check animations
    let animations_enabled = Keyword::get("animations:enabled")?;
    println!("Animations enabled: {}", animations_enabled);
    
    // Toggle animations
    if animations_enabled == "1" {
        Keyword::set("animations:enabled", "0")?;
        println!("Animations disabled");
    } else {
        Keyword::set("animations:enabled", "1")?;
        println!("Animations enabled");
    }
    
    Ok(())
}
```

## Control Operations

### Notifications and System Control

```rust
use hyprland::ctl::*;
use hyprland::shared::HResult;

fn main() -> HResult<()> {
    // Send notifications
    notify::notify("System", "Hyprland control example started", 3000)?;
    
    // Reload configuration
    println!("Reloading Hyprland configuration...");
    reload::reload_config()?;
    
    // Set cursor theme
    set_cursor::set_cursor_theme("Adwaita")?;
    
    // Create an error message (for demonstration)
    set_error::set_error("This is a test error message")?;
    
    // Switch keyboard layout
    switch_xkb_layout::switch_layout(0, "us")?;
    
    notify::notify("System", "Control operations completed", 2000)?;
    
    Ok(())
}
```

## Advanced Examples

### Window Monitor with Auto-Organization

```rust
use hyprland::{data::*, dispatch::*, event_listener::EventListener};
use hyprland::shared::HResult;
use std::collections::HashMap;

fn main() -> HResult<()> {
    let mut event_listener = EventListener::new();
    
    // Define application workspace mappings
    let mut app_workspaces: HashMap<&str, i32> = HashMap::new();
    app_workspaces.insert("firefox", 2);
    app_workspaces.insert("code", 3);
    app_workspaces.insert("discord", 4);
    app_workspaces.insert("spotify", 5);
    
    event_listener.add_window_open_handler(move |event| {
        println!("New window: {} ({})", event.window_title, event.window_class);
        
        // Auto-organize windows by class
        if let Some(&target_workspace) = app_workspaces.get(event.window_class.as_str()) {
            println!("Moving {} to workspace {}", event.window_class, target_workspace);
            
            if let Err(e) = Dispatch::call(DispatchType::MoveToWorkspaceSilent(
                WorkspaceIdentifier::Id(target_workspace)
            )) {
                eprintln!("Failed to move window: {}", e);
            }
        }
    });
    
    println!("Starting auto-organization monitor...");
    event_listener.start_listener()?;
    
    Ok(())
}
```

### System Status Monitor

```rust
use hyprland::data::*;
use hyprland::shared::HResult;
use std::time::Duration;
use std::thread;

fn main() -> HResult<()> {
    loop {
        // Clear screen
        print!("\x1B[2J\x1B[1;1H");
        
        // Get system state
        let monitors = Monitors::get()?;
        let workspaces = Workspaces::get()?;
        let clients = Clients::get()?;
        let version = Version::get()?;
        
        println!("=== Hyprland Status ===");
        println!("Version: {}", version.tag);
        println!("Branch: {}", version.branch);
        println!();
        
        println!("Monitors: {}", monitors.len());
        for monitor in &monitors {
            println!("  📺 {}: {}x{}@{}Hz (Workspace: {})", 
                monitor.name, monitor.width, monitor.height, 
                monitor.refresh_rate, monitor.active_workspace.id);
        }
        println!();
        
        println!("Workspaces: {}", workspaces.len());
        for workspace in &workspaces {
            let indicator = if workspace.windows > 0 { "🪟" } else { "📂" };
            println!("  {} Workspace {}: {} windows", 
                indicator, workspace.id, workspace.windows);
        }
        println!();
        
        println!("Active Windows: {}", clients.len());
        for client in &clients {
            let status = if client.fullscreen { "[F]" } 
                        else if client.floating { "[Float]" } 
                        else { "[Tiled]" };
            println!("  🪟 {} {} (WS: {})", 
                client.class, status, client.workspace.id);
        }
        
        // Update every 2 seconds
        thread::sleep(Duration::from_secs(2));
    }
}
```

### Dynamic Keybinding Manager

```rust
use hyprland::{keyword::Keyword, dispatch::*};
use hyprland::shared::HResult;

fn setup_dynamic_bindings() -> HResult<()> {
    // Define dynamic bindings based on time of day
    let hour = chrono::Local::now().hour();
    
    if hour >= 9 && hour < 17 {
        // Work hours - productivity bindings
        Keyword::set("bind", "SUPER, B, exec, firefox --new-window https://calendar.google.com")?;
        Keyword::set("bind", "SUPER, M, exec, thunderbird")?;
        Keyword::set("bind", "SUPER, T, exec, code")?;
        println!("Work mode bindings activated");
    } else {
        // Leisure hours - entertainment bindings  
        Keyword::set("bind", "SUPER, B, exec, firefox --new-window https://youtube.com")?;
        Keyword::set("bind", "SUPER, M, exec, spotify")?;
        Keyword::set("bind", "SUPER, T, exec, steam")?;
        println!("Leisure mode bindings activated");
    }
    
    Ok(())
}

fn main() -> HResult<()> {
    setup_dynamic_bindings()?;
    
    // Set up event listener to monitor workspace changes
    let mut event_listener = EventListener::new();
    
    event_listener.add_workspace_changed_handler(|event| {
        println!("Workspace changed to: {}", event.workspace_name);
        
        // Could modify bindings based on workspace
        match event.workspace_name.as_str() {
            "coding" => {
                // Development-specific bindings
                if let Err(e) = Keyword::set("bind", "SUPER, R, exec, cargo run") {
                    eprintln!("Failed to set binding: {}", e);
                }
            },
            "media" => {
                // Media-specific bindings
                if let Err(e) = Keyword::set("bind", "SUPER, P, exec, playerctl play-pause") {
                    eprintln!("Failed to set binding: {}", e);
                }
            },
            _ => {}
        }
    });
    
    event_listener.start_listener()?;
    Ok(())
}
```

## Error Handling Patterns

### Robust Error Handling

```rust
use hyprland::{data::*, dispatch::*};
use hyprland::shared::{HResult, HyprError};

fn safe_workspace_switch(workspace_id: i32) -> HResult<()> {
    // Check if workspace exists first
    let workspaces = Workspaces::get()?;
    let workspace_exists = workspaces.iter().any(|ws| ws.id == workspace_id);
    
    if !workspace_exists {
        return Err(HyprError::CommandFailed(
            format!("Workspace {} does not exist", workspace_id)
        ));
    }
    
    // Perform the switch
    Dispatch::call(DispatchType::Workspace(WorkspaceIdentifier::Id(workspace_id)))?;
    
    println!("Successfully switched to workspace {}", workspace_id);
    Ok(())
}

fn main() -> HResult<()> {
    match safe_workspace_switch(10) {
        Ok(_) => println!("Workspace switch successful"),
        Err(e) => {
            eprintln!("Workspace switch failed: {}", e);
            // Fallback to workspace 1
            safe_workspace_switch(1)?;
        }
    }
    
    Ok(())
}
```

These examples demonstrate the core functionality and patterns for using hyprland-rs effectively in real-world applications.