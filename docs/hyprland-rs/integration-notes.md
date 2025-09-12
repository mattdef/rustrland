# Hyprland-rs Integration Notes for Rustrland

This document outlines how hyprland-rs is integrated into Rustrland and provides guidance for leveraging its features effectively.

## Current Integration Status

### Version Information
- **Rustrland Uses**: hyprland-rs 0.4.0-beta.2
- **Reason for Beta**: Accessing latest features and improvements
- **Stability**: Beta version is stable for our use cases

### Integration Points

#### 1. IPC Client (`src/ipc/mod.rs`)
Rustrland uses hyprland-rs as the foundation for Hyprland communication:

```rust
// Current integration pattern
use hyprland::data::*;
use hyprland::dispatch::*;
use hyprland::shared::HResult;

pub struct HyprlandClient {
    // Wraps hyprland-rs functionality
}

impl HyprlandClient {
    pub fn get_windows() -> HResult<Vec<Client>> {
        Clients::get()
    }
    
    pub fn get_monitors() -> HResult<Vec<Monitor>> {
        Monitors::get()
    }
    
    pub fn dispatch(&self, command: DispatchType) -> HResult<()> {
        Dispatch::call(command)
    }
}
```

#### 2. Enhanced Client (`src/ipc/enhanced_client.rs`)
Extended functionality with reconnection logic and performance optimizations:

```rust
use hyprland::event_listener::AsyncEventListener;

pub struct EnhancedHyprlandClient {
    event_listener: Option<AsyncEventListener>,
    reconnect_attempts: u32,
    last_connection_time: Instant,
}

impl EnhancedHyprlandClient {
    pub async fn start_event_monitoring(&mut self) -> HResult<()> {
        let mut listener = AsyncEventListener::new();
        
        // Add handlers for Rustrland-specific events
        listener.add_window_open_handler(|event| async move {
            // Forward to plugin system
            self.notify_plugins(PluginEvent::WindowOpened(event)).await;
        });
        
        self.event_listener = Some(listener);
        Ok(())
    }
}
```

## Recommended Usage Patterns

### 1. Data Retrieval
Use hyprland-rs data module for efficient state queries:

```rust
use hyprland::data::*;

// Efficient batch queries
async fn get_compositor_state() -> CompositorState {
    let (monitors, workspaces, clients) = tokio::try_join!(
        tokio::task::spawn_blocking(|| Monitors::get()),
        tokio::task::spawn_blocking(|| Workspaces::get()),
        tokio::task::spawn_blocking(|| Clients::get()),
    ).unwrap();
    
    CompositorState {
        monitors: monitors.unwrap(),
        workspaces: workspaces.unwrap(),
        clients: clients.unwrap(),
    }
}
```

### 2. Event-Driven Architecture
Leverage async event listeners for responsive plugin behavior:

```rust
use hyprland::event_listener::AsyncEventListener;

pub async fn setup_plugin_events(plugin_manager: Arc<PluginManager>) -> HResult<()> {
    let mut listener = AsyncEventListener::new();
    
    let pm = plugin_manager.clone();
    listener.add_window_open_handler(move |event| {
        let pm = pm.clone();
        async move {
            pm.handle_window_event(WindowEvent::Opened(event)).await;
        }
    });
    
    let pm = plugin_manager.clone();
    listener.add_workspace_changed_handler(move |event| {
        let pm = pm.clone();
        async move {
            pm.handle_workspace_event(WorkspaceEvent::Changed(event)).await;
        }
    });
    
    listener.start_listener().await
}
```

### 3. Command Dispatch Integration
Integrate dispatch commands with Rustrland's command system:

```rust
use hyprland::dispatch::*;

pub enum RustrCommand {
    Toggle(String),
    Expose,
    Workspace(WorkspaceOp),
    // ... other commands
}

impl RustrCommand {
    pub async fn execute(&self) -> HResult<()> {
        match self {
            RustrCommand::Workspace(WorkspaceOp::Switch(id)) => {
                Dispatch::call(DispatchType::Workspace(
                    WorkspaceIdentifier::Id(*id)
                ))?;
            },
            RustrCommand::Expose => {
                // Custom expose implementation using hyprland-rs
                self.execute_expose().await?;
            },
            // ... other command implementations
        }
        Ok(())
    }
}
```

## Plugin Integration Patterns

### 1. Scratchpads Plugin
The scratchpads plugin leverages hyprland-rs for window management:

```rust
use hyprland::data::Clients;
use hyprland::dispatch::*;

impl ScratchpadsPlugin {
    async fn toggle_scratchpad(&self, name: &str) -> HResult<()> {
        let clients = Clients::get()?;
        let scratchpad_window = clients.iter()
            .find(|c| c.class == name || c.title.contains(name));
        
        match scratchpad_window {
            Some(window) if window.workspace.name.contains("special") => {
                // Show scratchpad
                Dispatch::call(DispatchType::ToggleSpecialWorkspace(
                    Some(format!("scratch_{}", name))
                ))?;
            },
            Some(_) => {
                // Hide scratchpad
                Dispatch::call(DispatchType::MoveToSpecialWorkspace(
                    Some(format!("scratch_{}", name))
                ))?;
            },
            None => {
                // Create new scratchpad
                self.create_scratchpad(name).await?;
            }
        }
        Ok(())
    }
}
```

### 2. Animation Integration
Combine hyprland-rs with Rustrland's animation system:

```rust
use hyprland::dispatch::*;
use crate::animation::*;

impl WindowAnimator {
    pub async fn animate_window_movement(
        &self, 
        window_id: &str, 
        target_pos: (i32, i32)
    ) -> HResult<()> {
        // Use hyprland-rs to get current position
        let clients = Clients::get()?;
        let window = clients.iter()
            .find(|c| c.address == window_id)
            .ok_or_else(|| HyprError::CommandFailed("Window not found".to_string()))?;
        
        let current_pos = window.at;
        
        // Create animation timeline
        let timeline = Timeline::slide_timeline(
            Duration::from_millis(300),
            current_pos,
            target_pos,
            EasingFunction::EaseOutCubic
        );
        
        // Animate using Rustrland's animation engine
        let animation_id = self.animation_engine.start_animation(timeline).await?;
        
        // Apply final position via hyprland-rs
        Dispatch::call(DispatchType::MoveWindowPixel(
            target_pos.0 - current_pos.0,
            target_pos.1 - current_pos.1
        ))?;
        
        Ok(())
    }
}
```

## Configuration Integration

### Keyword Management
Use hyprland-rs for dynamic configuration:

```rust
use hyprland::keyword::Keyword;

pub struct ConfigManager {
    // Configuration state
}

impl ConfigManager {
    pub async fn apply_theme(&self, theme: &Theme) -> HResult<()> {
        // Apply colors
        Keyword::set("general:col.active_border", &theme.active_border)?;
        Keyword::set("general:col.inactive_border", &theme.inactive_border)?;
        
        // Apply gaps
        Keyword::set("general:gaps_in", &theme.gaps_inner.to_string())?;
        Keyword::set("general:gaps_out", &theme.gaps_outer.to_string())?;
        
        // Apply animations
        if theme.animations_enabled {
            Keyword::set("animations:enabled", "1")?;
            for (anim_type, config) in &theme.animations {
                let keyword = format!("animation:{}", anim_type);
                Keyword::set(&keyword, config)?;
            }
        } else {
            Keyword::set("animations:enabled", "0")?;
        }
        
        Ok(())
    }
}
```

## Performance Considerations

### 1. Batch Operations
Group hyprland-rs calls for efficiency:

```rust
pub async fn batch_window_operations(operations: Vec<WindowOp>) -> HResult<()> {
    // Collect all dispatch calls
    let dispatches: Vec<DispatchType> = operations.iter()
        .map(|op| op.to_dispatch_type())
        .collect();
    
    // Execute in sequence (hyprland-rs doesn't support true batching)
    for dispatch in dispatches {
        Dispatch::call(dispatch)?;
        // Small delay to prevent overwhelming Hyprland
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
    
    Ok(())
}
```

### 2. Caching Strategy
Cache hyprland-rs data when appropriate:

```rust
pub struct CachedHyprlandState {
    monitors: Option<(Instant, Vec<Monitor>)>,
    workspaces: Option<(Instant, Vec<Workspace>)>,
    clients: Option<(Instant, Vec<Client>)>,
    cache_duration: Duration,
}

impl CachedHyprlandState {
    pub async fn get_monitors(&mut self) -> HResult<&Vec<Monitor>> {
        if let Some((timestamp, ref monitors)) = &self.monitors {
            if timestamp.elapsed() < self.cache_duration {
                return Ok(monitors);
            }
        }
        
        let monitors = Monitors::get()?;
        self.monitors = Some((Instant::now(), monitors));
        Ok(&self.monitors.as_ref().unwrap().1)
    }
}
```

## Error Handling Strategies

### Robust Error Recovery
Handle hyprland-rs errors gracefully:

```rust
pub async fn safe_dispatch(command: DispatchType) -> Result<(), RustrError> {
    match Dispatch::call(command.clone()) {
        Ok(_) => Ok(()),
        Err(HyprError::SocketNotFound) => {
            // Hyprland might be restarting
            tokio::time::sleep(Duration::from_millis(100)).await;
            Dispatch::call(command).map_err(|e| RustrError::HyprlandError(e))
        },
        Err(e) => Err(RustrError::HyprlandError(e)),
    }
}
```

## Future Integration Opportunities

### 1. Enhanced Event System
When hyprland-rs 0.4 stabilizes, consider:
- More granular event filtering
- Event batching capabilities
- Better async integration

### 2. Configuration Validation
- Use hyprland-rs to validate configuration before applying
- Implement configuration rollback on errors
- Dynamic configuration reloading

### 3. Plugin Communication
- Use hyprland-rs as a bridge between Rustrland plugins
- Implement plugin-to-plugin communication via Hyprland events
- Shared state management through Hyprland properties

## Best Practices

1. **Always handle errors**: hyprland-rs operations can fail
2. **Use async patterns**: Leverage tokio for non-blocking operations
3. **Cache appropriately**: Balance freshness with performance
4. **Batch when possible**: Group related operations
5. **Monitor events**: Use event listeners for reactive behavior
6. **Test thoroughly**: Validate integration with real Hyprland instances

## Troubleshooting

### Common Issues
1. **Socket errors**: Check HYPRLAND_INSTANCE_SIGNATURE environment variable
2. **Version mismatches**: Ensure Hyprland version compatibility
3. **Permission issues**: Verify socket access permissions
4. **Event handling**: Check event listener setup and error handling

### Debug Strategies
```rust
// Enable debug logging for hyprland-rs operations
use tracing::{debug, error};

pub async fn debug_dispatch(command: DispatchType) -> HResult<()> {
    debug!("Executing Hyprland command: {:?}", command);
    
    match Dispatch::call(command) {
        Ok(_) => {
            debug!("Command executed successfully");
            Ok(())
        },
        Err(e) => {
            error!("Command failed: {}", e);
            Err(e)
        }
    }
}
```