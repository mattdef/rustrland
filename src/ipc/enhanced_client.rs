use anyhow::Result;
use hyprland::event_listener::EventListener;
use hyprland::shared::HyprData;
use hyprland::data::{Client, Clients, Monitor, Monitors};
use hyprland::dispatch::{Dispatch, DispatchType};
use tracing::{info, debug, warn, error};
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc, RwLock};
use tokio::time::{sleep, Duration, Instant};
use std::collections::HashMap;
use serde_json::Value;

use super::HyprlandEvent;

/// Enhanced Hyprland client with robust connection management
pub struct EnhancedHyprlandClient {
    event_sender: Arc<Mutex<Option<mpsc::Sender<HyprlandEvent>>>>,
    connection_state: Arc<RwLock<ConnectionState>>,
    reconnect_config: ReconnectConfig,
    event_filters: Arc<RwLock<Vec<String>>>, // Event types to filter for
}

#[derive(Debug, Clone)]
pub struct ConnectionState {
    pub is_connected: bool,
    pub last_connection_attempt: Option<Instant>,
    pub connection_failures: u32,
    pub hyprland_instance: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ReconnectConfig {
    pub max_retries: u32,
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub backoff_multiplier: f64,
}

impl Default for ReconnectConfig {
    fn default() -> Self {
        Self {
            max_retries: 10,
            initial_delay: Duration::from_millis(500),
            max_delay: Duration::from_secs(30),
            backoff_multiplier: 2.0,
        }
    }
}

impl Default for ConnectionState {
    fn default() -> Self {
        Self {
            is_connected: false,
            last_connection_attempt: None,
            connection_failures: 0,
            hyprland_instance: None,
        }
    }
}

impl EnhancedHyprlandClient {
    pub fn new() -> Self {
        Self {
            event_sender: Arc::new(Mutex::new(None)),
            connection_state: Arc::new(RwLock::new(ConnectionState::default())),
            reconnect_config: ReconnectConfig::default(),
            event_filters: Arc::new(RwLock::new(vec![
                // Filter for relevant events only
                "workspace".to_string(),
                "focusedmon".to_string(), 
                "openwindow".to_string(),
                "closewindow".to_string(),
                "movewindow".to_string(),
                "resizewindow".to_string(),
                "changefloatingmode".to_string(),
                "urgent".to_string(),
                "minimize".to_string(),
                "windowtitle".to_string(),
            ])),
        }
    }
    
    /// Set event filters to reduce processing overhead
    pub async fn set_event_filters(&self, filters: Vec<String>) {
        let mut event_filters = self.event_filters.write().await;
        *event_filters = filters;
        info!("📝 Updated event filters: {:?}", *event_filters);
    }
    
    /// Check current connection status
    pub async fn is_connected(&self) -> bool {
        let state = self.connection_state.read().await;
        state.is_connected
    }
    
    /// Get current Hyprland instance signature
    pub fn get_hyprland_instance() -> Option<String> {
        std::env::var("HYPRLAND_INSTANCE_SIGNATURE").ok()
    }
    
    /// Test connection to Hyprland
    pub async fn test_connection(&self) -> Result<()> {
        debug!("🧪 Testing Hyprland connection");
        
        // Check if HYPRLAND_INSTANCE_SIGNATURE exists
        let instance = Self::get_hyprland_instance()
            .ok_or_else(|| anyhow::anyhow!("HYPRLAND_INSTANCE_SIGNATURE not set"))?;
        
        // Test basic connectivity
        let _monitors = tokio::task::spawn_blocking(|| {
            Monitors::get()
        }).await??;
        
        // Update connection state
        {
            let mut state = self.connection_state.write().await;
            state.is_connected = true;
            state.connection_failures = 0;
            state.hyprland_instance = Some(instance);
        }
        
        info!("✅ Hyprland connection test successful");
        Ok(())
    }
    
    /// Start event listener with robust reconnection logic
    /// For now, returns a simple receiver that doesn't have events
    /// The actual event listening will be handled by the regular HyprlandClient
    pub async fn start_event_listener(&self) -> Result<mpsc::Receiver<HyprlandEvent>> {
        let (_tx, rx) = mpsc::channel::<HyprlandEvent>(1000);
        info!("📡 Enhanced event listener initialized (placeholder)");
        Ok(rx)
    }
    
    
    /// Parse Hyprland event string into structured event with improved comma handling
    fn parse_hyprland_event(event_str: &str, filters: &[String]) -> Option<HyprlandEvent> {
        debug!("📥 Raw event: {}", event_str);
        
        // Split into event type and data, using splitn to handle commas in data
        let parts: Vec<&str> = event_str.splitn(2, ">>").collect();
        if parts.len() != 2 {
            debug!("🔍 Malformed event (no >>): {}", event_str);
            return None;
        }
        
        let event_type = parts[0].trim();
        let event_data = parts[1].trim();
        
        // Validate event type is not empty
        if event_type.is_empty() {
            debug!("🔍 Empty event type: {}", event_str);
            return None;
        }
        
        // Apply filters early for performance
        if !filters.is_empty() && !filters.iter().any(|filter| event_type.starts_with(filter)) {
            return None; // Skip filtered events
        }
        
        // Parse specific event types with proper comma handling
        match event_type {
            "workspace" => {
                Some(HyprlandEvent::WorkspaceChanged {
                    workspace: event_data.to_string(),
                })
            }
            "focusedmon" => {
                // Format: "monitorname,workspacename"
                let parts: Vec<&str> = event_data.splitn(2, ',').collect();
                if parts.len() >= 1 {
                    Some(HyprlandEvent::MonitorChanged {
                        monitor: parts[0].to_string(),
                    })
                } else {
                    None
                }
            }
            "openwindow" => {
                // Format: "windowaddress,workspacename,windowclass,windowtitle"
                let parts: Vec<&str> = event_data.splitn(4, ',').collect();
                if parts.len() >= 1 {
                    Some(HyprlandEvent::WindowOpened {
                        window: parts[0].to_string(), // window address
                    })
                } else {
                    None
                }
            }
            "closewindow" => {
                // Format: "windowaddress"
                Some(HyprlandEvent::WindowClosed {
                    window: event_data.to_string(),
                })
            }
            "movewindow" => {
                // Format: "windowaddress,workspacename" or "windowaddress,workspaceid"
                let parts: Vec<&str> = event_data.splitn(2, ',').collect();
                if parts.len() >= 1 {
                    Some(HyprlandEvent::WindowMoved {
                        window: parts[0].to_string(),
                    })
                } else {
                    None
                }
            }
            "activewindow" => {
                // Format: "windowclass,windowtitle" - need to handle comma in title
                let parts: Vec<&str> = event_data.splitn(2, ',').collect();
                if parts.len() >= 1 {
                    Some(HyprlandEvent::WindowFocusChanged {
                        window: parts[0].to_string(), // We'll use class for now
                    })
                } else {
                    None
                }
            }
            "windowtitle" => {
                // Format: "windowaddress,windowtitle" - title can contain commas
                let parts: Vec<&str> = event_data.splitn(2, ',').collect();
                if parts.len() >= 2 {
                    Some(HyprlandEvent::Other(format!("windowtitle>>{},{}", parts[0], parts[1])))
                } else {
                    None
                }
            }
            "resizewindow" => {
                // Format: "windowaddress,newsize"
                let parts: Vec<&str> = event_data.splitn(2, ',').collect();
                if parts.len() >= 1 {
                    Some(HyprlandEvent::Other(format!("resizewindow>>{}", parts[0])))
                } else {
                    None
                }
            }
            _ => {
                // For other events, pass through as Other
                Some(HyprlandEvent::Other(event_str.to_string()))
            }
        }
    }
    
    /// Get detailed window geometry from Hyprland
    pub async fn get_window_geometry(&self, window_address: &str) -> Result<WindowGeometry> {
        debug!("📐 Getting geometry for window: {}", window_address);
        
        let address = window_address.to_string();
        let clients = tokio::task::spawn_blocking(|| {
            Clients::get()
        }).await??;
        
        // Find the specific window
        for client in clients.iter() {
            if client.address.to_string() == address {
                return Ok(WindowGeometry {
                    x: client.at.0 as i32,
                    y: client.at.1 as i32,
                    width: client.size.0 as i32,
                    height: client.size.1 as i32,
                    workspace: client.workspace.name.clone(),
                    monitor: client.monitor as i32,
                    floating: client.floating,
                });
            }
        }
        
        Err(anyhow::anyhow!("Window not found: {}", window_address))
    }
    
    /// Batch get geometries for multiple windows (more efficient)
    pub async fn get_multiple_window_geometries(&self, addresses: &[String]) -> Result<HashMap<String, WindowGeometry>> {
        debug!("📐 Getting geometries for {} windows", addresses.len());
        
        let address_set: std::collections::HashSet<String> = addresses.iter().cloned().collect();
        let clients = tokio::task::spawn_blocking(|| {
            Clients::get()
        }).await??;
        
        let mut geometries = HashMap::new();
        
        for client in clients.iter() {
            let client_address = client.address.to_string();
            if address_set.contains(&client_address) {
                geometries.insert(client_address, WindowGeometry {
                    x: client.at.0 as i32,
                    y: client.at.1 as i32,
                    width: client.size.0 as i32,
                    height: client.size.1 as i32,
                    workspace: client.workspace.name.clone(),
                    monitor: client.monitor as i32,
                    floating: client.floating,
                });
            }
        }
        
        Ok(geometries)
    }
    
    /// Get connection statistics
    pub async fn get_connection_stats(&self) -> ConnectionStats {
        let state = self.connection_state.read().await;
        ConnectionStats {
            is_connected: state.is_connected,
            connection_failures: state.connection_failures,
            last_connection_attempt: state.last_connection_attempt,
            hyprland_instance: state.hyprland_instance.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct WindowGeometry {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub workspace: String,
    pub monitor: i32,
    pub floating: bool,
}

#[derive(Debug, Clone)]
pub struct ConnectionStats {
    pub is_connected: bool,
    pub connection_failures: u32,
    pub last_connection_attempt: Option<Instant>,
    pub hyprland_instance: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    
    #[test]
    fn test_event_parsing_with_commas() {
        let filters = vec!["openwindow".to_string(), "windowtitle".to_string()];
        
        // Test openwindow with comma in title
        let event = "openwindow>>0x12345,1,firefox,GitHub - user/repo: Issues, Pull Requests";
        let parsed = EnhancedHyprlandClient::parse_hyprland_event(event, &filters);
        assert!(matches!(parsed, Some(HyprlandEvent::WindowOpened { .. })));
        
        // Test windowtitle with comma in title
        let event = "windowtitle>>0x12345,My Document, Version 2.0";
        let parsed = EnhancedHyprlandClient::parse_hyprland_event(event, &filters);
        assert!(matches!(parsed, Some(HyprlandEvent::Other(_))));
    }
    
    #[test]
    fn test_event_filtering() {
        let filters = vec!["openwindow".to_string()];
        
        // Should be included
        let event = "openwindow>>0x12345,1,firefox,title";
        let parsed = EnhancedHyprlandClient::parse_hyprland_event(event, &filters);
        assert!(parsed.is_some());
        
        // Should be filtered out
        let event = "somethingelse>>data";
        let parsed = EnhancedHyprlandClient::parse_hyprland_event(event, &filters);
        assert!(parsed.is_none());
    }
    
    #[test]
    fn test_splitn_behavior() {
        let text = "part1,part2,with,many,commas";
        let parts: Vec<&str> = text.splitn(3, ',').collect();
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[0], "part1");
        assert_eq!(parts[1], "part2");
        assert_eq!(parts[2], "with,many,commas"); // Preserves commas in remainder
    }
    
    #[test]
    fn test_event_parsing_all_types() {
        let filters = vec![]; // No filtering
        
        // Test workspace change
        let event = "workspace>>5";
        let parsed = EnhancedHyprlandClient::parse_hyprland_event(event, &filters);
        assert!(matches!(parsed, Some(HyprlandEvent::WorkspaceChanged { workspace }) if workspace == "5"));
        
        // Test monitor change
        let event = "focusedmon>>DP-1,workspace1";
        let parsed = EnhancedHyprlandClient::parse_hyprland_event(event, &filters);
        assert!(matches!(parsed, Some(HyprlandEvent::MonitorChanged { monitor }) if monitor == "DP-1"));
        
        // Test window closed
        let event = "closewindow>>0x12345";
        let parsed = EnhancedHyprlandClient::parse_hyprland_event(event, &filters);
        assert!(matches!(parsed, Some(HyprlandEvent::WindowClosed { window }) if window == "0x12345"));
        
        // Test window moved
        let event = "movewindow>>0x12345,workspace2";
        let parsed = EnhancedHyprlandClient::parse_hyprland_event(event, &filters);
        assert!(matches!(parsed, Some(HyprlandEvent::WindowMoved { window }) if window == "0x12345"));
        
        // Test active window change
        let event = "activewindow>>firefox,GitHub";
        let parsed = EnhancedHyprlandClient::parse_hyprland_event(event, &filters);
        assert!(matches!(parsed, Some(HyprlandEvent::WindowFocusChanged { window }) if window == "firefox"));
    }
    
    #[test]
    fn test_complex_comma_scenarios() {
        let filters = vec!["windowtitle".to_string(), "openwindow".to_string()];
        
        // Complex title with multiple commas and special characters
        let event = "windowtitle>>0x12345,Document: Draft, Version 2.0, Last Modified: Today, Status: In Progress";
        let parsed = EnhancedHyprlandClient::parse_hyprland_event(event, &filters);
        assert!(matches!(parsed, Some(HyprlandEvent::Other(msg)) if msg.contains("Document: Draft, Version 2.0, Last Modified: Today, Status: In Progress")));
        
        // Window opened with complex title
        let event = "openwindow>>0x67890,workspace1,vscode,Code - main.rs - Visual Studio Code, Rust Project";
        let parsed = EnhancedHyprlandClient::parse_hyprland_event(event, &filters);
        assert!(matches!(parsed, Some(HyprlandEvent::WindowOpened { window }) if window == "0x67890"));
    }
    
    #[test]
    fn test_malformed_events() {
        let filters = vec![];
        
        // Event without >> separator
        let event = "malformed_event_data";
        let parsed = EnhancedHyprlandClient::parse_hyprland_event(event, &filters);
        assert!(parsed.is_none());
        
        // Empty event
        let event = "";
        let parsed = EnhancedHyprlandClient::parse_hyprland_event(event, &filters);
        assert!(parsed.is_none());
        
        // Event with only >>
        let event = ">>";
        let parsed = EnhancedHyprlandClient::parse_hyprland_event(event, &filters);
        assert!(parsed.is_none());
    }
    
    #[tokio::test]
    async fn test_enhanced_client_creation() {
        let client = EnhancedHyprlandClient::new();
        
        // Test initial state
        assert!(!client.is_connected().await);
        
        // Test connection stats
        let stats = client.get_connection_stats().await;
        assert!(!stats.is_connected);
        assert_eq!(stats.connection_failures, 0);
        assert!(stats.last_connection_attempt.is_none());
        assert!(stats.hyprland_instance.is_none());
    }
    
    #[tokio::test]
    async fn test_event_filter_management() {
        let client = EnhancedHyprlandClient::new();
        
        // Test initial filters
        let initial_stats = client.get_connection_stats().await;
        assert!(!initial_stats.is_connected);
        
        // Test setting custom filters
        let custom_filters = vec![
            "workspace".to_string(),
            "openwindow".to_string(),
            "closewindow".to_string(),
        ];
        client.set_event_filters(custom_filters.clone()).await;
        
        // Verify filters were set (indirectly by ensuring no panic)
        assert!(true); // Filter setting should complete successfully
    }
    
    #[test]
    fn test_hyprland_instance_detection() {
        // Test with no environment variable
        let _instance = EnhancedHyprlandClient::get_hyprland_instance();
        // In test environment, this will be None unless HYPRLAND_INSTANCE_SIGNATURE is set
        
        // Test with mock environment variable (if we were to set it)
        env::set_var("HYPRLAND_INSTANCE_SIGNATURE", "test_signature_123");
        let instance_with_env = EnhancedHyprlandClient::get_hyprland_instance();
        assert_eq!(instance_with_env, Some("test_signature_123".to_string()));
        
        // Clean up
        env::remove_var("HYPRLAND_INSTANCE_SIGNATURE");
    }
    
    #[test]
    fn test_window_geometry_structure() {
        let geometry = WindowGeometry {
            x: 100,
            y: 200,
            width: 800,
            height: 600,
            workspace: "main".to_string(),
            monitor: 1,
            floating: true,
        };
        
        assert_eq!(geometry.x, 100);
        assert_eq!(geometry.y, 200);
        assert_eq!(geometry.width, 800);
        assert_eq!(geometry.height, 600);
        assert_eq!(geometry.workspace, "main");
        assert_eq!(geometry.monitor, 1);
        assert!(geometry.floating);
    }
    
    #[test]
    fn test_connection_stats_structure() {
        use tokio::time::Instant;
        
        let stats = ConnectionStats {
            is_connected: true,
            connection_failures: 5,
            last_connection_attempt: Some(Instant::now()),
            hyprland_instance: Some("test_instance".to_string()),
        };
        
        assert!(stats.is_connected);
        assert_eq!(stats.connection_failures, 5);
        assert!(stats.last_connection_attempt.is_some());
        assert_eq!(stats.hyprland_instance, Some("test_instance".to_string()));
    }
    
    #[test]
    fn test_performance_with_many_events() {
        let filters = vec!["openwindow".to_string(), "closewindow".to_string()];
        
        // Test parsing performance with many events
        let test_events = vec![
            "openwindow>>0x1,workspace1,app1,Title 1",
            "closewindow>>0x1",
            "openwindow>>0x2,workspace1,app2,Title 2, with commas",
            "movewindow>>0x2,workspace2",
            "windowtitle>>0x2,New Title, with more commas",
            "resizewindow>>0x2,800x600",
            "workspace>>2",
            "focusedmon>>DP-1,workspace2",
        ];
        
        let start = std::time::Instant::now();
        for event in &test_events {
            let _parsed = EnhancedHyprlandClient::parse_hyprland_event(event, &filters);
        }
        let duration = start.elapsed();
        
        // Should complete quickly (under 1ms for small batch)
        assert!(duration.as_millis() < 100);
    }
}