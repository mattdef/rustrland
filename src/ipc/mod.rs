use anyhow::Result;
use hyprland::event_listener::EventListener;
use hyprland::shared::HyprData;
use hyprland::data::{Client, Clients};
use hyprland::dispatch::{Dispatch, DispatchType};
use tracing::{info, debug, warn};
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};

pub mod protocol;
pub mod server;

pub use protocol::{ClientMessage, DaemonResponse};

// Define a basic event type for now
#[derive(Debug, Clone)]
pub enum HyprlandEvent {
    WorkspaceChanged { workspace: String },
    WindowOpened { window: String },
    WindowClosed { window: String },
    WindowMoved { window: String },
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
        
        // Test basic connectivity
        let _monitors = hyprland::data::Monitors::get()
            .map_err(|e| anyhow::anyhow!("Failed to connect to Hyprland: {}", e))?;
        
        info!("✅ Hyprland connection test successful");
        Ok(())
    }
    
    pub async fn create_event_listener(&self) -> Result<()> {
        debug!("📡 Creating event listener");
        
        let (tx, rx) = mpsc::channel::<HyprlandEvent>(100);
        
        // Store the receiver
        let mut receiver_guard = self.event_receiver.lock().await;
        *receiver_guard = Some(rx);
        
        // Spawn background task to handle events
        tokio::spawn(async move {
            // For now, create a basic event listener that generates periodic events
            // This should be replaced with actual Hyprland event listening
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(5));
            
            loop {
                interval.tick().await;
                
                // Generate a dummy event for testing
                let event = HyprlandEvent::Other("heartbeat".to_string());
                
                if tx.send(event).await.is_err() {
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
                None => Err(anyhow::anyhow!("Event channel closed"))
            }
        } else {
            Err(anyhow::anyhow!("Event listener not initialized"))
        }
    }
    
    /// Find a window by its class name
    pub async fn find_window_by_class(&self, class: &str) -> Result<Option<Client>> {
        debug!("🔍 Looking for window with class: {}", class);
        
        let clients = tokio::task::spawn_blocking(move || {
            Clients::get()
        }).await??;
        
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
        
        tokio::task::spawn_blocking(move || {
            Dispatch::call(command)
        }).await??;
        
        debug!("✅ Command dispatched successfully");
        Ok(())
    }
    
    /// Spawn a new application
    pub async fn spawn_app(&self, command: &str) -> Result<()> {
        info!("🚀 Spawning application: {}", command);
        
        let command = command.to_string();
        self.dispatch(DispatchType::Exec(Box::leak(command.into_boxed_str()))).await?;
        
        Ok(())
    }
    
    /// Focus a specific window
    pub async fn focus_window(&self, address: &str) -> Result<()> {
        debug!("🎯 Focusing window: {}", address);
        
        use hyprland::dispatch::WindowIdentifier;
        use hyprland::shared::Address;
        
        let address = address.to_string();
        let window_id = WindowIdentifier::Address(Address::new(Box::leak(address.into_boxed_str())));
        self.dispatch(DispatchType::FocusWindow(window_id)).await?;
        
        Ok(())
    }
    
    /// Move and resize a window (simplified - just move to special workspace for now)
    pub async fn move_resize_window(&self, _address: &str, _x: i32, _y: i32, _width: i32, _height: i32) -> Result<()> {
        debug!("📐 Moving/resizing window (simplified implementation)");
        
        use hyprland::dispatch::WorkspaceIdentifierWithSpecial;
        
        // For now, we'll just move to special workspace
        // A full implementation would need proper window positioning
        let workspace = WorkspaceIdentifierWithSpecial::Special(Some("scratchpad"));
        self.dispatch(DispatchType::MoveToWorkspaceSilent(workspace, None)).await?;
        
        Ok(())
    }
    
    /// Toggle window visibility using special workspace
    pub async fn toggle_window_visibility(&self, _address: &str) -> Result<()> {
        debug!("👁️ Toggling scratchpad visibility");
        
        // Toggle special workspace for scratchpads
        self.dispatch(DispatchType::ToggleSpecialWorkspace(Some("scratchpad".to_string()))).await?;
        
        Ok(())
    }
}
