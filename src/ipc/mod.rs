use anyhow::Result;
use hyprland::data::{Client, Clients, Monitor, Monitors};
use hyprland::dispatch::{Dispatch, DispatchType};
use hyprland::event_listener::EventListener;
use hyprland::shared::{HyprData, HyprDataActiveOptional, WorkspaceType};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tokio::time::{timeout, Duration};
use tracing::{debug, info, warn};

pub mod enhanced_client;
pub mod protocol;
pub mod server;

pub use enhanced_client::{ConnectionStats, EnhancedHyprlandClient, WindowGeometry};
pub use protocol::{ClientMessage, DaemonResponse};

/// Timeout duration for Hyprland API calls
const HYPRLAND_API_TIMEOUT: Duration = Duration::from_secs(5);

/// Execute a blocking Hyprland API call with timeout
async fn with_hyprland_timeout<T, F>(operation: F) -> Result<T>
where
    F: FnOnce() -> Result<T, hyprland::shared::HyprError> + Send + 'static,
    T: Send + 'static,
{
    timeout(HYPRLAND_API_TIMEOUT, tokio::task::spawn_blocking(operation))
        .await
        .map_err(|_| anyhow::anyhow!("Hyprland API call timeout after {:?}", HYPRLAND_API_TIMEOUT))?
        .map_err(|e| anyhow::anyhow!("Failed to spawn Hyprland task: {}", e))?
        .map_err(|e| anyhow::anyhow!("Hyprland API error: {}", e))
}

// Define a basic event type for now
#[derive(Debug, Clone)]
pub enum HyprlandEvent {
    WorkspaceChanged { workspace: String },
    WindowOpened { window: String },
    WindowClosed { window: String },
    WindowMoved { window: String },
    WindowFocusChanged { window: String },
    MonitorChanged { monitor: String },
    Other(String),
}

#[derive(Clone)]
pub struct HyprlandClient {
    event_receiver: Arc<Mutex<Option<mpsc::Receiver<HyprlandEvent>>>>,
}

impl HyprlandClient {
    pub async fn new() -> Result<Self> {
        debug!("🔌 Creating Hyprland client");
        Ok(Self {
            event_receiver: Arc::new(Mutex::new(None)),
        })
    }

    pub async fn test_connection(&self) -> Result<()> {
        debug!("🧪 Testing Hyprland connection");

        // Test basic connectivity with timeout
        let _monitors = with_hyprland_timeout(hyprland::data::Monitors::get).await?;

        info!("✅ Hyprland connection test successful");
        Ok(())
    }

    pub async fn create_event_listener(&self) -> Result<()> {
        debug!("📡 Creating event listener");

        let (tx, rx) = mpsc::channel::<HyprlandEvent>(100);

        // Store the receiver
        let mut receiver_guard = self.event_receiver.lock().await;
        *receiver_guard = Some(rx);

        // Spawn background task to handle events with focus tracking
        tokio::spawn(async move {
            debug!("🎧 Starting focus tracking event system");

            let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(500)); // Check every 500ms for responsive focus tracking
            let mut last_focused_window: Option<String> = None;

            loop {
                interval.tick().await;

                // Try to get the currently focused window using activewindow
                match with_hyprland_timeout(|| {
                    use hyprland::data::Client;

                    // Get the active window directly
                    match Client::get_active() {
                        Ok(Some(client)) => Ok(Some(client.address.to_string())),
                        Ok(None) => Ok(None),
                        Err(e) => Err(e),
                    }
                })
                .await
                {
                    Ok(current_focused) => {
                        // Check if focus has changed
                        if current_focused != last_focused_window {
                            if let Some(ref current_window) = current_focused {
                                debug!("👁️ Focus changed to: {}", current_window);

                                if let Err(e) = tx
                                    .send(HyprlandEvent::WindowFocusChanged {
                                        window: current_window.clone(),
                                    })
                                    .await
                                {
                                    warn!("Failed to send window focus event: {}", e);
                                }
                            } else if last_focused_window.is_some() {
                                debug!("👁️ Focus lost (no focused window)");

                                if let Err(e) = tx
                                    .send(HyprlandEvent::WindowFocusChanged {
                                        window: "none".to_string(),
                                    })
                                    .await
                                {
                                    warn!("Failed to send window focus lost event: {}", e);
                                }
                            }

                            last_focused_window = current_focused;
                        }
                    }
                    Err(e) => {
                        debug!("Failed to get focused window: {}", e);
                    }
                }

                // Send periodic heartbeat for other functionality
                if let Err(_e) = tx.send(HyprlandEvent::Other("heartbeat".to_string())).await {
                    warn!("Event receiver dropped, stopping event listener");
                    break;
                }
            }
        });

        Ok(())
    }

    pub async fn get_next_event(&self) -> Result<HyprlandEvent> {
        let mut receiver_guard = self.event_receiver.lock().await;

        if let Some(receiver) = receiver_guard.as_mut() {
            match receiver.recv().await {
                Some(event) => Ok(event),
                None => Err(anyhow::anyhow!("Event channel closed")),
            }
        } else {
            Err(anyhow::anyhow!("Event listener not initialized"))
        }
    }

    /// Find a window by its class name
    pub async fn find_window_by_class(&self, class: &str) -> Result<Option<Client>> {
        debug!("🔍 Looking for window with class: {}", class);

        let clients = with_hyprland_timeout(Clients::get).await?;

        for client in clients.iter() {
            if client.class == class {
                debug!("✅ Found window: {} ({})", client.title, client.class);
                return Ok(Some(client.clone()));
            }
        }

        debug!("❌ No window found with class: {}", class);
        Ok(None)
    }

    /// Execute a Hyprland dispatch command
    pub async fn dispatch(&self, command: DispatchType<'static>) -> Result<()> {
        debug!("📤 Dispatching command: {:?}", command);

        tokio::task::spawn_blocking(move || Dispatch::call(command)).await??;

        debug!("✅ Command dispatched successfully");
        Ok(())
    }

    /// Spawn a new application
    pub async fn spawn_app(&self, command: &str) -> Result<()> {
        info!("🚀 Spawning application: {}", command);

        let command = command.to_string();
        self.dispatch(DispatchType::Exec(Box::leak(command.into_boxed_str())))
            .await?;

        Ok(())
    }

    /// Focus a specific window
    pub async fn focus_window(&self, address: &str) -> Result<()> {
        debug!("🎯 Focusing window: {}", address);

        use hyprland::dispatch::WindowIdentifier;
        use hyprland::shared::Address;

        let address = address.to_string();
        let window_id =
            WindowIdentifier::Address(Address::new(Box::leak(address.into_boxed_str())));
        self.dispatch(DispatchType::FocusWindow(window_id)).await?;

        Ok(())
    }

    /// Move and resize a window (simplified - just move to special workspace for now)
    pub async fn move_resize_window(
        &self,
        address: &str,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> Result<()> {
        debug!(
            "📐 Moving/resizing window: {} to {}x{} at ({}, {})",
            address, width, height, x, y
        );

        use hyprland::dispatch::{
            DispatchType, Position, WindowIdentifier, WorkspaceIdentifierWithSpecial,
        };
        use hyprland::shared::Address;

        // Move to special workspace first
        let workspace = WorkspaceIdentifierWithSpecial::Special(Some("scratchpad"));
        self.dispatch(DispatchType::MoveToWorkspaceSilent(workspace, None))
            .await?;

        // Apply the geometry using Hyprland's move and resize commands
        let window_id = WindowIdentifier::Address(Address::new(Box::leak(
            address.to_string().into_boxed_str(),
        )));

        // Resize the window using pixel dimensions
        debug!("📏 Resizing window {} to {}x{}", address, width, height);
        self.dispatch(DispatchType::ResizeWindowPixel(
            Position::Exact(width as i16, height as i16),
            window_id.clone(),
        ))
        .await?;

        // Move the window to the specified position using pixel coordinates
        debug!("📍 Moving window {} to position ({}, {})", address, x, y);
        self.dispatch(DispatchType::MoveWindowPixel(
            Position::Exact(x as i16, y as i16),
            window_id,
        ))
        .await?;

        Ok(())
    }

    /// Move and resize a window without changing workspace
    pub async fn resize_and_position_window(
        &self,
        address: &str,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> Result<()> {
        debug!(
            "📐 Resizing and positioning window: {} to {}x{} at ({}, {})",
            address, width, height, x, y
        );

        use hyprland::dispatch::{DispatchType, Position, WindowIdentifier};
        use hyprland::shared::Address;

        let window_id = WindowIdentifier::Address(Address::new(Box::leak(
            address.to_string().into_boxed_str(),
        )));

        // Resize the window using pixel dimensions
        debug!("📏 Resizing window {} to {}x{}", address, width, height);
        self.dispatch(DispatchType::ResizeWindowPixel(
            Position::Exact(width as i16, height as i16),
            window_id.clone(),
        ))
        .await?;

        // Move the window to the specified position using pixel coordinates
        debug!("📍 Moving window {} to position ({}, {})", address, x, y);
        self.dispatch(DispatchType::MoveWindowPixel(
            Position::Exact(x as i16, y as i16),
            window_id,
        ))
        .await?;

        Ok(())
    }

    /// Toggle window visibility using special workspace
    pub async fn toggle_window_visibility(&self, _address: &str) -> Result<()> {
        debug!("👁️ Toggling scratchpad visibility");

        // Toggle special workspace for scratchpads
        self.dispatch(DispatchType::ToggleSpecialWorkspace(Some(
            "scratchpad".to_string(),
        )))
        .await?;

        Ok(())
    }

    /// Move window to specific position (for animations)
    pub async fn move_window(&self, address: &str, x: i32, y: i32) -> Result<()> {
        debug!("📍 Moving window {} to position ({}, {})", address, x, y);

        // Use hyprctl to move window
        let address = address.to_string();

        tokio::task::spawn_blocking(move || {
            std::process::Command::new("hyprctl")
                .arg("dispatch")
                .arg("movewindow")
                .arg(format!("address:{address}"))
                .arg(format!("{x} {y}"))
                .output()
        })
        .await??;

        Ok(())
    }

    /// Resize window to specific size (for animations)
    pub async fn resize_window(&self, address: &str, width: i32, height: i32) -> Result<()> {
        debug!(
            "📏 Resizing window {} to size ({}x{})",
            address, width, height
        );

        let address = address.to_string();

        tokio::task::spawn_blocking(move || {
            std::process::Command::new("hyprctl")
                .arg("dispatch")
                .arg("resizewindow")
                .arg(format!("address:{address}"))
                .arg(format!("{width} {height}"))
                .output()
        })
        .await??;

        Ok(())
    }

    /// Set window opacity (for fade animations)
    pub async fn set_window_opacity(&self, address: &str, opacity: f32) -> Result<()> {
        debug!("🌟 Setting window {} opacity to {}", address, opacity);

        let address = address.to_string();
        let opacity_value = (opacity * 255.0) as u8;

        tokio::task::spawn_blocking(move || {
            std::process::Command::new("hyprctl")
                .arg("dispatch")
                .arg("setprop")
                .arg(format!("address:{address}"))
                .arg("alpha")
                .arg(format!("{opacity_value}"))
                .output()
        })
        .await??;

        Ok(())
    }

    /// Get window properties for animation calculations
    pub async fn get_window_properties(&self, address: &str) -> Result<WindowProperties> {
        debug!("🔍 Getting properties for window {}", address);

        let clients = with_hyprland_timeout(Clients::get).await?;

        for client in clients.iter() {
            if client.address.to_string() == address {
                return Ok(WindowProperties {
                    x: client.at.0 as i32,
                    y: client.at.1 as i32,
                    width: client.size.0 as i32,
                    height: client.size.1 as i32,
                    workspace: client.workspace.id.to_string(),
                });
            }
        }

        Err(anyhow::anyhow!("Window not found: {}", address))
    }

    /// Get monitors information
    pub async fn get_monitors(&self) -> Result<Vec<Monitor>> {
        debug!("🖥️ Getting monitors information");

        let monitors = with_hyprland_timeout(Monitors::get).await?;

        use hyprland::shared::HyprDataVec;
        Ok(monitors.to_vec())
    }

    /// Find windows by class name
    pub async fn find_windows_by_class(&self, class: &str) -> Result<Vec<Client>> {
        debug!("🔍 Finding windows with class: {}", class);

        let target_class = class.to_string();
        let clients = with_hyprland_timeout(Clients::get).await?;

        let matching_windows: Vec<Client> = clients
            .into_iter()
            .filter(|client| client.class == target_class)
            .collect();

        debug!(
            "Found {} windows with class '{}'",
            matching_windows.len(),
            class
        );
        Ok(matching_windows)
    }

    /// Get window information by address
    pub async fn get_window_info(&self, address: &str) -> Result<Client> {
        debug!("🔍 Getting window info for: {}", address);

        let target_address = address.to_string();
        let clients = tokio::task::spawn_blocking(Clients::get).await??;

        for client in clients {
            if client.address.to_string() == target_address {
                return Ok(client);
            }
        }

        Err(anyhow::anyhow!("Window not found: {}", address))
    }

    /// Get all windows/clients
    pub async fn get_windows(&self) -> Result<Vec<Client>> {
        debug!("🪟 Getting all windows");

        let clients = with_hyprland_timeout(Clients::get).await?;
        use hyprland::shared::HyprDataVec;
        Ok(clients.to_vec())
    }

    /// Get current active workspace
    pub async fn get_active_workspace(&self) -> Result<String> {
        debug!("🖥️ Getting active workspace");

        use hyprland::data::{Workspace, Workspaces};

        let workspaces = with_hyprland_timeout(Workspaces::get).await?;

        // Find the focused workspace
        for workspace in workspaces.iter() {
            if workspace.id > 0 && workspace.windows > 0 {
                // For now, return the first regular workspace with windows
                // In a real implementation, we'd check which one is actually focused
                return Ok(workspace.id.to_string());
            }
        }

        // Fallback to workspace 1
        Ok("1".to_string())
    }

    /// Move window to workspace
    pub async fn move_window_to_workspace(&self, address: &str, workspace: &str) -> Result<()> {
        debug!("📍 Moving window {} to workspace {}", address, workspace);

        use hyprland::dispatch::{WindowIdentifier, WorkspaceIdentifierWithSpecial};
        use hyprland::shared::Address;

        let address = address.to_string();
        let workspace = workspace.to_string();

        let window_id =
            WindowIdentifier::Address(Address::new(Box::leak(address.into_boxed_str())));
        let workspace_id = if workspace.starts_with("special:") {
            let special_name = workspace.strip_prefix("special:").unwrap_or("").to_string();
            WorkspaceIdentifierWithSpecial::Special(Some(Box::leak(special_name.into_boxed_str())))
        } else {
            WorkspaceIdentifierWithSpecial::Id(workspace.parse().unwrap_or(1))
        };

        self.dispatch(DispatchType::MoveToWorkspaceSilent(
            workspace_id,
            Some(window_id),
        ))
        .await?;

        Ok(())
    }

    /// Move window to specific position
    pub async fn move_window_to_position(&self, address: &str, x: i32, y: i32) -> Result<()> {
        debug!("📍 Moving window {} to position ({}, {})", address, x, y);

        // Use hyprctl movewindowpixel for exact positioning
        let address = address.to_string();

        tokio::task::spawn_blocking(move || {
            std::process::Command::new("hyprctl")
                .arg("dispatch")
                .arg("movewindowpixel")
                .arg(format!("exact {x} {y},address:{address}"))
                .output()
        })
        .await??;

        Ok(())
    }

    /// Show a window
    pub async fn show_window(&self, address: &str) -> Result<()> {
        debug!("👁️ Showing window: {}", address);

        let address = address.to_string();
        tokio::task::spawn_blocking(move || {
            std::process::Command::new("hyprctl")
                .arg("dispatch")
                .arg("movetoworkspace")
                .arg("e+0")
                .arg(format!("address:{address}"))
                .output()
        })
        .await??;

        Ok(())
    }

    /// Hide a window
    pub async fn hide_window(&self, address: &str) -> Result<()> {
        debug!("🙈 Hiding window: {}", address);

        let address = address.to_string();
        tokio::task::spawn_blocking(move || {
            std::process::Command::new("hyprctl")
                .arg("dispatch")
                .arg("movetoworkspace")
                .arg("special:hidden")
                .arg(format!("address:{address}"))
                .output()
        })
        .await??;

        Ok(())
    }

    /// Close a window
    pub async fn close_window(&self, address: &str) -> Result<()> {
        debug!("❌ Closing window: {}", address);

        let address = address.to_string();
        tokio::task::spawn_blocking(move || {
            std::process::Command::new("hyprctl")
                .arg("dispatch")
                .arg("closewindow")
                .arg(format!("address:{address}"))
                .output()
        })
        .await??;

        Ok(())
    }

    /// Toggle floating mode for a window
    pub async fn toggle_floating(&self, address: &str) -> Result<()> {
        debug!("🎈 Toggling floating for window: {}", address);

        let address = address.to_string();
        tokio::task::spawn_blocking(move || {
            std::process::Command::new("hyprctl")
                .arg("dispatch")
                .arg("togglefloating")
                .arg(format!("address:{address}"))
                .output()
        })
        .await??;

        Ok(())
    }
}

/// Window properties for animations
#[derive(Debug, Clone)]
pub struct WindowProperties {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub workspace: String,
}

/// Monitor information for multi-monitor support
#[derive(Debug, Clone)]
pub struct MonitorInfo {
    pub name: String,
    pub width: i32,
    pub height: i32,
    pub x: i32,
    pub y: i32,
    pub scale: f32,
    pub is_focused: bool,
    pub active_workspace_id: i32,
}
