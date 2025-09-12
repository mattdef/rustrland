# Hyprland-rs API Reference

## Core Types and Traits

### `HResult<T>`
```rust
type HResult<T> = Result<T, HyprError>;
```
Standard result type for all hyprland-rs operations.

### `HyprError`
```rust
enum HyprError {
    CommandFailed(String),
    SocketNotFound,
    SocketError(String),
    ParseError(String),
    // ... other variants
}
```

## Module: `data`

### Structs

#### `Monitor`
```rust
pub struct Monitor {
    pub id: i32,
    pub name: String,
    pub description: String,
    pub make: String,
    pub model: String,
    pub serial: String,
    pub width: i32,
    pub height: i32,
    pub refresh_rate: f32,
    pub x: i32,
    pub y: i32,
    pub active_workspace: WorkspaceBasic,
    pub special_workspace: Option<WorkspaceBasic>,
    pub reserved: [i32; 4],
    pub scale: f32,
    pub transform: Transform,
    pub focused: bool,
    pub dp_ms_av_one: f32,
    pub dp_ms_av_ten: f32,
    pub dp_ms_av_hundred: f32,
}
```

#### `Workspace`
```rust
pub struct Workspace {
    pub id: i32,
    pub name: String,
    pub monitor: String,
    pub monitor_id: i32,
    pub windows: i32,
    pub has_fullscreen: bool,
    pub last_window: String,
    pub last_window_title: String,
}
```

#### `Client`
```rust
pub struct Client {
    pub address: String,
    pub mapped: bool,
    pub hidden: bool,
    pub at: (i32, i32),
    pub size: (i32, i32),
    pub workspace: WorkspaceBasic,
    pub floating: bool,
    pub monitor: i32,
    pub class: String,
    pub title: String,
    pub initial_class: String,
    pub initial_title: String,
    pub pid: i32,
    pub xwayland: bool,
    pub pinned: bool,
    pub fullscreen: bool,
    pub fullscreen_mode: i32,
    pub fake_fullscreen: bool,
    pub grouped: Vec<String>,
    pub tags: Vec<String>,
    pub swallowing: String,
    pub focus_history_id: i32,
}
```

#### `Version`
```rust
pub struct Version {
    pub branch: String,
    pub commit: String,
    pub dirty: bool,
    pub commit_message: String,
    pub commit_date: String,
    pub tag: String,
    pub commits: String,
    pub flags: Vec<String>,
}
```

#### `CursorPosition`
```rust
pub struct CursorPosition {
    pub x: f32,
    pub y: f32,
}
```

### Functions

#### `Monitors`
```rust
impl Monitors {
    pub fn get() -> HResult<Vec<Monitor>>;
}
```

#### `Workspaces`
```rust
impl Workspaces {
    pub fn get() -> HResult<Vec<Workspace>>;
}
```

#### `Clients`
```rust
impl Clients {
    pub fn get() -> HResult<Vec<Client>>;
}
```

#### `Version`
```rust
impl Version {
    pub fn get() -> HResult<Version>;
}
```

#### `CursorPosition`
```rust
impl CursorPosition {
    pub fn get() -> HResult<CursorPosition>;
}
```

## Module: `dispatch`

### Enums

#### `DispatchType`
```rust
pub enum DispatchType {
    // Application Management
    Exec(String),
    KillActiveWindow,
    CloseWindow,
    
    // Window Movement
    MoveWindow(Direction),
    ResizeWindow(Direction),
    MoveWindowPixel(i32, i32),
    ResizeWindowPixel(i32, i32),
    
    // Window State
    ToggleFloating,
    Fullscreen(FullscreenType),
    FakeFullscreen,
    Pin,
    
    // Focus Management
    MoveFocus(Direction),
    FocusWindow(WindowIdentifier),
    FocusMonitor(MonitorIdentifier),
    
    // Workspace Management
    Workspace(WorkspaceIdentifier),
    MoveToWorkspace(WorkspaceIdentifier),
    MoveToWorkspaceSilent(WorkspaceIdentifier),
    
    // Layout Management
    NextLayout,
    PreviousLayout,
    OrientationNext,
    OrientationPrevious,
    OrientationTop,
    OrientationRight,
    OrientationBottom,
    OrientationLeft,
    OrientationCenter,
    
    // Group Management
    ToggleGroup,
    ChangeGroupActive(Direction),
    
    // Special
    ToggleSpecialWorkspace(Option<String>),
    MoveToSpecialWorkspace(Option<String>),
    
    // Other
    CenterWindow,
    Pseudo,
    // ... many more variants
}
```

#### `Direction`
```rust
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}
```

#### `WindowIdentifier`
```rust
pub enum WindowIdentifier {
    Address(String),
    ProcessId(u32),
    Class(String),
    ClassRegex(String),
    Title(String),
    TitleRegex(String),
    InitialClass(String),
    InitialTitle(String),
}
```

#### `WorkspaceIdentifier`
```rust
pub enum WorkspaceIdentifier {
    Id(i32),
    Name(String),
    Relative(i32),
    Previous,
    Empty,
    Special(Option<String>),
}
```

### Functions

#### `Dispatch`
```rust
impl Dispatch {
    pub fn call(dispatch_type: DispatchType) -> HResult<()>;
}
```

## Module: `event_listener`

### Structs

#### `EventListener`
```rust
pub struct EventListener {
    // internal fields
}
```

#### `AsyncEventListener`
```rust
pub struct AsyncEventListener {
    // internal fields  
}
```

### Event Handler Types

```rust
// Window Events
pub type WindowOpenHandler = Box<dyn Fn(WindowOpenEvent) + Send>;
pub type WindowCloseHandler = Box<dyn Fn(WindowCloseEvent) + Send>;
pub type WindowMoveHandler = Box<dyn Fn(WindowMoveEvent) + Send>;

// Workspace Events  
pub type WorkspaceChangedHandler = Box<dyn Fn(WorkspaceEvent) + Send>;
pub type WorkspaceAddedHandler = Box<dyn Fn(WorkspaceEvent) + Send>;
pub type WorkspaceDestroyedHandler = Box<dyn Fn(WorkspaceEvent) + Send>;

// Monitor Events
pub type MonitorAddedHandler = Box<dyn Fn(MonitorEvent) + Send>;
pub type MonitorRemovedHandler = Box<dyn Fn(MonitorEvent) + Send>;
```

### Event Data Structures

#### `WindowOpenEvent`
```rust
pub struct WindowOpenEvent {
    pub window_address: String,
    pub workspace_name: String,
    pub window_class: String,
    pub window_title: String,
}
```

#### `WindowCloseEvent`
```rust
pub struct WindowCloseEvent {
    pub window_address: String,
}
```

#### `WorkspaceEvent`
```rust
pub struct WorkspaceEvent {
    pub workspace_id: i32,
    pub workspace_name: String,
}
```

### Functions

#### `EventListener` Methods
```rust
impl EventListener {
    pub fn new() -> Self;
    
    // Window event handlers
    pub fn add_window_open_handler<F>(&mut self, handler: F)
    where F: Fn(WindowOpenEvent) + Send + 'static;
    
    pub fn add_window_close_handler<F>(&mut self, handler: F)
    where F: Fn(WindowCloseEvent) + Send + 'static;
    
    pub fn add_window_move_handler<F>(&mut self, handler: F)
    where F: Fn(WindowMoveEvent) + Send + 'static;
    
    // Workspace event handlers
    pub fn add_workspace_changed_handler<F>(&mut self, handler: F)
    where F: Fn(WorkspaceEvent) + Send + 'static;
    
    pub fn add_workspace_added_handler<F>(&mut self, handler: F)
    where F: Fn(WorkspaceEvent) + Send + 'static;
    
    // Monitor event handlers
    pub fn add_monitor_added_handler<F>(&mut self, handler: F)
    where F: Fn(MonitorEvent) + Send + 'static;
    
    // Start listening
    pub fn start_listener(self) -> HResult<()>;
}
```

#### `AsyncEventListener` Methods
```rust
impl AsyncEventListener {
    pub fn new() -> Self;
    
    // Async window event handlers
    pub fn add_window_open_handler<F, Fut>(&mut self, handler: F)
    where 
        F: Fn(WindowOpenEvent) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static;
    
    // Similar pattern for other async handlers...
    
    pub async fn start_listener(self) -> HResult<()>;
}
```

## Module: `keyword`

### Structs

#### `Keyword`
```rust
pub struct Keyword;
```

### Enums

#### `OptionValue`
```rust
pub enum OptionValue {
    String(String),
    Int(i32),
    Float(f32),
    Bool(bool),
    Color(Color),
}
```

### Functions

```rust
impl Keyword {
    pub fn get(keyword: &str) -> HResult<String>;
    pub fn set(keyword: &str, value: &str) -> HResult<()>;
}
```

## Module: `ctl`

### Submodules

#### `notify`
```rust
pub fn notify(title: &str, message: &str, timeout: u32) -> HResult<()>;
```

#### `reload`
```rust
pub fn reload_config() -> HResult<()>;
```

#### `set_cursor`
```rust
pub fn set_cursor_theme(theme: &str) -> HResult<()>;
pub fn set_cursor_size(size: u32) -> HResult<()>;
```

#### `switch_xkb_layout`
```rust
pub fn switch_layout(device_id: u32, layout: &str) -> HResult<()>;
```

#### `kill`
```rust
pub fn enter_kill_mode() -> HResult<()>;
```

#### `output`
```rust
pub fn create_headless(width: u32, height: u32) -> HResult<()>;
pub fn remove_headless(name: &str) -> HResult<()>;
```

#### `plugin`
```rust
pub fn load_plugin(path: &str) -> HResult<()>;
pub fn unload_plugin(name: &str) -> HResult<()>;
pub fn list_plugins() -> HResult<Vec<String>>;
```

#### `set_prop`
```rust
pub fn set_window_property(
    identifier: WindowIdentifier,
    property: &str, 
    value: &str
) -> HResult<()>;
```

#### `set_error`
```rust
pub fn set_error(message: &str) -> HResult<()>;
```

### Structs

#### `Color`
```rust
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub fn new(r: u8, g: u8, b: u8, a: u8) -> Self;
    pub fn from_hex(hex: &str) -> HResult<Self>;
    pub fn to_hex(&self) -> String;
}
```

## Prelude

The `prelude` module re-exports commonly used types:

```rust
pub use crate::shared::{HResult, HyprError};
pub use crate::data::*;
pub use crate::dispatch::{Dispatch, DispatchType};
pub use crate::event_listener::{EventListener, AsyncEventListener};
```

## Usage Patterns

### Error Handling
```rust
use hyprland::prelude::*;

fn example() -> HResult<()> {
    match Monitors::get() {
        Ok(monitors) => {
            // Handle success
            Ok(())
        },
        Err(e) => {
            eprintln!("Error: {}", e);
            Err(e)
        }
    }
}
```

### Chaining Operations
```rust
use hyprland::prelude::*;

fn workspace_management() -> HResult<()> {
    Dispatch::call(DispatchType::Workspace(WorkspaceIdentifier::Id(2)))?;
    Dispatch::call(DispatchType::Exec("kitty".to_string()))?;
    Dispatch::call(DispatchType::ToggleFloating)?;
    Ok(())
}
```

### Event-Driven Architecture
```rust
use hyprland::event_listener::EventListener;

fn setup_automation() -> HResult<()> {
    let mut listener = EventListener::new();
    
    listener.add_window_open_handler(|event| {
        if event.window_class.contains("important") {
            // Auto-focus important windows
            let _ = Dispatch::call(DispatchType::FocusWindow(
                WindowIdentifier::Class(event.window_class)
            ));
        }
    });
    
    listener.start_listener()
}
```