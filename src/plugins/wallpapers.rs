use anyhow::Result;
use async_trait::async_trait;
use rand::seq::SliceRandom;
use rand::thread_rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::fs;
use tokio::sync::Mutex;
use tokio::time::{interval, sleep};
use tracing::{debug, error, info, warn};

use crate::ipc::{HyprlandClient, HyprlandEvent};
use crate::plugins::Plugin;

use hyprland::data::Monitors;
use hyprland::shared::{HyprData, HyprDataVec};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct WallpapersConfig {
    /// Wallpaper directories to scan
    pub path: WallpaperPath,

    /// Interval between wallpaper changes in seconds (default: 600)
    #[serde(default = "default_interval")]
    pub interval: u64,

    /// Supported image file extensions
    #[serde(default = "default_extensions")]
    pub extensions: Vec<String>,

    /// Recursively search subdirectories (default: false)
    #[serde(default)]
    pub recurse: bool,

    /// Set different wallpaper for each monitor (default: false)
    #[serde(default)]
    pub unique: bool,

    /// Command template to set wallpaper (default: uses swaybg)
    #[serde(default = "default_command")]
    pub command: String,

    /// Command to clear wallpapers
    #[serde(default)]
    pub clear_command: Option<String>,

    /// Enable debug logging (default: false)
    #[serde(default)]
    pub debug_logging: bool,

    /// Preload next wallpapers for faster switching (default: 3)
    #[serde(default = "default_preload_count")]
    pub preload_count: usize,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)]
pub enum WallpaperPath {
    Single(PathBuf),
    Multiple(Vec<PathBuf>),
}

fn default_interval() -> u64 {
    600 // 10 minutes
}

fn default_extensions() -> Vec<String> {
    vec![
        "jpg".to_string(),
        "jpeg".to_string(),
        "png".to_string(),
        "webp".to_string(),
        "bmp".to_string(),
    ]
}

fn default_command() -> String {
    "swaybg -i \"[file]\" -m fill".to_string()
}

fn default_preload_count() -> usize {
    3
}

impl Default for WallpapersConfig {
    fn default() -> Self {
        Self {
            path: WallpaperPath::Single(PathBuf::from("~/Pictures/wallpapers")),
            interval: 600,
            extensions: default_extensions(),
            recurse: false,
            unique: false,
            command: default_command(),
            clear_command: None,
            debug_logging: false,
            preload_count: 3,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WallpaperInfo {
    pub path: PathBuf,
    pub filename: String,
    pub size_bytes: u64,
    pub last_modified: std::time::SystemTime,
    pub dimensions: Option<(u32, u32)>,
}

#[derive(Debug)]
pub struct MonitorState {
    pub name: String,
    pub current_wallpaper: Option<PathBuf>,
    pub wallpaper_index: usize,
    pub last_change: Instant,
}

pub struct WallpapersPlugin {
    config: WallpapersConfig,
    wallpapers: Vec<WallpaperInfo>,
    monitors: HashMap<String, MonitorState>,
    hyprland_client: Arc<Mutex<Option<Arc<HyprlandClient>>>>,
    rotation_handle: Option<tokio::task::JoinHandle<()>>,
    last_scan: Option<Instant>,
    preloaded_images: HashMap<PathBuf, Vec<u8>>, // Cache for better performance
    active_processes: HashMap<String, u32>, // Track active wallpaper backend processes per monitor
}

impl WallpapersPlugin {
    pub fn new() -> Self {
        Self {
            config: WallpapersConfig::default(),
            wallpapers: Vec::new(),
            monitors: HashMap::new(),
            hyprland_client: Arc::new(Mutex::new(None)),
            rotation_handle: None,
            last_scan: None,
            preloaded_images: HashMap::new(),
            active_processes: HashMap::new(),
        }
    }

    /// Expand tilde in paths to home directory
    fn expand_path(&self, path: &Path) -> Result<PathBuf> {
        if path.starts_with("~") {
            if let Some(home) = dirs::home_dir() {
                Ok(home.join(path.strip_prefix("~")?))
            } else {
                Err(anyhow::anyhow!("Could not determine home directory"))
            }
        } else {
            Ok(path.to_path_buf())
        }
    }

    /// Scan wallpaper directories and populate the wallpapers list
    async fn scan_wallpapers(&mut self) -> Result<()> {
        self.wallpapers.clear();
        let mut wallpapers = Vec::new();

        match &self.config.path {
            WallpaperPath::Single(path) => {
                let expanded_path = self.expand_path(path)?;
                if expanded_path.exists() {
                    if self.config.recurse {
                        self.scan_directory_recursive(&expanded_path, &mut wallpapers)
                            .await?;
                    } else {
                        self.scan_directory(&expanded_path, &mut wallpapers).await?;
                    }
                } else {
                    warn!("Wallpaper path does not exist: {}", expanded_path.display());
                }
            }
            WallpaperPath::Multiple(paths) => {
                for path in paths {
                    let expanded_path = self.expand_path(path)?;
                    if expanded_path.exists() {
                        if self.config.recurse {
                            self.scan_directory_recursive(&expanded_path, &mut wallpapers)
                                .await?;
                        } else {
                            self.scan_directory(&expanded_path, &mut wallpapers).await?;
                        }
                    } else {
                        warn!("Wallpaper path does not exist: {}", expanded_path.display());
                    }
                }
            }
        }

        // Randomize order
        wallpapers.shuffle(&mut thread_rng());

        self.wallpapers = wallpapers;
        self.last_scan = Some(Instant::now());

        info!("🖼️  Found {} wallpapers", self.wallpapers.len());

        // Preload some images for better performance
        self.preload_images().await?;

        Ok(())
    }

    /// Scan a single directory for images
    async fn scan_directory(&self, path: &Path, wallpapers: &mut Vec<WallpaperInfo>) -> Result<()> {
        let mut entries = fs::read_dir(path).await?;

        while let Some(entry) = entries.next_entry().await? {
            let file_path = entry.path();
            if file_path.is_file() {
                if let Some(extension) = file_path.extension() {
                    if self.config.extensions.contains(&extension.to_string_lossy().to_lowercase()) {
                        let metadata = entry.metadata().await?;
                        let wallpaper_info = WallpaperInfo {
                            path: file_path.clone(),
                            filename: file_path
                                .file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .to_string(),
                            size_bytes: metadata.len(),
                            last_modified: metadata.modified().unwrap_or(std::time::UNIX_EPOCH),
                            dimensions: None,
                        };
                        wallpapers.push(wallpaper_info);
                    }
                }
            }
        }

        Ok(())
    }

    /// Recursively scan directories for images
    fn scan_directory_recursive<'a>(&'a self, path: &'a Path, wallpapers: &'a mut Vec<WallpaperInfo>) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
            self.scan_directory(path, wallpapers).await?;

            let mut entries = fs::read_dir(path).await?;
            while let Some(entry) = entries.next_entry().await? {
                let entry_path = entry.path();
                if entry_path.is_dir() {
                    self.scan_directory_recursive(&entry_path, wallpapers)
                        .await?;
                }
            }

            Ok(())
        })
    }

    /// Preload some images for better performance
    async fn preload_images(&mut self) -> Result<()> {
        let preload_count = self.config.preload_count.min(self.wallpapers.len());
        
        for wallpaper in self.wallpapers.iter().take(preload_count) {
            if !self.preloaded_images.contains_key(&wallpaper.path) {
                match fs::read(&wallpaper.path).await {
                    Ok(data) => {
                        self.preloaded_images.insert(wallpaper.path.clone(), data);
                        debug!("📦 Preloaded: {}", wallpaper.filename);
                    }
                    Err(e) => {
                        warn!("⚠️ Failed to preload {}: {}", wallpaper.filename, e);
                    }
                }
            }
        }

        Ok(())
    }

    /// Get the next wallpaper for a specific monitor
    fn get_next_wallpaper(&mut self, monitor_name: &str) -> Option<PathBuf> {
        if self.wallpapers.is_empty() {
            return None;
        }

        let monitor_count = self.monitors.len();
        let monitor_state = self.monitors.entry(monitor_name.to_string()).or_insert_with(|| {
            MonitorState {
                name: monitor_name.to_string(),
                current_wallpaper: None,
                wallpaper_index: 0,
                last_change: Instant::now(),
            }
        });

        if self.config.unique {
            // Each monitor gets a different wallpaper
            let wallpaper_index = (monitor_state.wallpaper_index + monitor_count) % self.wallpapers.len();
            monitor_state.wallpaper_index = wallpaper_index;
        } else {
            // All monitors get the same wallpaper
            monitor_state.wallpaper_index = (monitor_state.wallpaper_index + 1) % self.wallpapers.len();
        }

        self.wallpapers.get(monitor_state.wallpaper_index).map(|w| w.path.clone())
    }

    /// Set wallpaper for a specific monitor
    async fn set_wallpaper(&mut self, monitor_name: Option<&str>, wallpaper_path: &Path) -> Result<()> {
        let monitors = if let Some(monitor) = monitor_name {
            vec![monitor.to_string()]
        } else {
            // Get all monitors if none specified
            self.get_monitor_names().await?
        };

        for monitor in monitors {
            self.set_wallpaper_for_monitor(&monitor, wallpaper_path).await?;
        }

        Ok(())
    }

    /// Set wallpaper for a specific monitor
    async fn set_wallpaper_for_monitor(&mut self, monitor_name: &str, wallpaper_path: &Path) -> Result<()> {
        // Kill existing process for this monitor if any
        if let Some(old_pid) = self.active_processes.get(monitor_name) {
            if let Err(e) = Command::new("kill").arg(old_pid.to_string()).output() {
                debug!("Failed to kill old wallpaper process {}: {}", old_pid, e);
            }
        }

        // Replace [file] placeholder with actual file path
        let command = self.config.command.replace("[file]", &wallpaper_path.to_string_lossy());
        
        // Add monitor specification if supported
        let full_command = if command.contains("swaybg") {
            format!("{} -o {}", command, monitor_name)
        } else {
            command
        };

        debug!("🖼️  Setting wallpaper on {}: {}", monitor_name, wallpaper_path.display());

        // Execute the wallpaper command
        let child = Command::new("sh")
            .arg("-c")
            .arg(&full_command)
            .spawn()?;

        // Store the process ID for cleanup
        let pid = child.id();
        self.active_processes.insert(monitor_name.to_string(), pid);

        // Update monitor state
        let monitor_state = self.monitors.entry(monitor_name.to_string()).or_insert_with(|| {
            MonitorState {
                name: monitor_name.to_string(),
                current_wallpaper: None,
                wallpaper_index: 0,
                last_change: Instant::now(),
            }
        });

        monitor_state.current_wallpaper = Some(wallpaper_path.to_path_buf());
        monitor_state.last_change = Instant::now();

        Ok(())
    }

    /// Get list of monitor names from Hyprland
    async fn get_monitor_names(&self) -> Result<Vec<String>> {
        match Monitors::get() {
            Ok(monitors) => {
                Ok(monitors.iter().map(|m| m.name.clone()).collect())
            }
            Err(e) => {
                warn!("Failed to get monitors from Hyprland: {}", e);
                Ok(vec!["DP-1".to_string()]) // Default fallback
            }
        }
    }

    /// Clear all wallpapers
    async fn clear_wallpapers(&mut self) -> Result<()> {
        // Kill all active processes
        for (monitor, pid) in &self.active_processes {
            if let Err(e) = Command::new("kill").arg(pid.to_string()).output() {
                debug!("Failed to kill wallpaper process {} for {}: {}", pid, monitor, e);
            }
        }
        self.active_processes.clear();

        // Execute clear command if specified
        if let Some(clear_cmd) = &self.config.clear_command {
            Command::new("sh").arg("-c").arg(clear_cmd).output()?;
        }

        // Clear monitor states
        for monitor_state in self.monitors.values_mut() {
            monitor_state.current_wallpaper = None;
        }

        Ok(())
    }

    /// Start automatic wallpaper rotation
    async fn start_rotation(&mut self) -> Result<()> {
        if self.rotation_handle.is_some() {
            return Ok(()); // Already running
        }

        let interval_secs = self.config.interval;
        let unique = self.config.unique;
        
        info!("🔄 Starting wallpaper rotation (interval: {}s)", interval_secs);

        // Clone necessary data for the background task
        let wallpapers = self.wallpapers.clone();
        let monitors = self.get_monitor_names().await?;
        let command = self.config.command.clone();

        let handle = tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(interval_secs));
            let mut wallpaper_index = 0;

            loop {
                interval.tick().await;

                if wallpapers.is_empty() {
                    debug!("No wallpapers available for rotation");
                    continue;
                }

                for (monitor_idx, monitor_name) in monitors.iter().enumerate() {
                    let wallpaper = if unique {
                        // Each monitor gets a different wallpaper
                        &wallpapers[(wallpaper_index + monitor_idx) % wallpapers.len()]
                    } else {
                        // All monitors get the same wallpaper
                        &wallpapers[wallpaper_index % wallpapers.len()]
                    };

                    let full_command = command.replace("[file]", &wallpaper.path.to_string_lossy());
                    let full_command = if full_command.contains("swaybg") {
                        format!("{} -o {}", full_command, monitor_name)
                    } else {
                        full_command
                    };

                    if let Err(e) = Command::new("sh").arg("-c").arg(&full_command).spawn() {
                        error!("Failed to set wallpaper: {}", e);
                    } else {
                        debug!("🖼️  Set wallpaper on {}: {}", monitor_name, wallpaper.filename);
                    }
                }

                wallpaper_index = (wallpaper_index + 1) % wallpapers.len();
            }
        });

        self.rotation_handle = Some(handle);
        Ok(())
    }

    /// Stop automatic wallpaper rotation
    async fn stop_rotation(&mut self) -> Result<()> {
        if let Some(handle) = self.rotation_handle.take() {
            handle.abort();
            info!("⏹️  Stopped wallpaper rotation");
        }
        Ok(())
    }

    /// Get status information
    fn get_status(&self) -> String {
        let mut output = String::new();
        
        output.push_str(&format!("📊 Wallpapers Plugin Status\n"));
        output.push_str(&format!("  • Wallpapers loaded: {}\n", self.wallpapers.len()));
        output.push_str(&format!("  • Monitors tracked: {}\n", self.monitors.len()));
        output.push_str(&format!("  • Rotation active: {}\n", self.rotation_handle.is_some()));
        output.push_str(&format!("  • Interval: {}s\n", self.config.interval));
        output.push_str(&format!("  • Unique per monitor: {}\n", self.config.unique));

        if let Some(last_scan) = self.last_scan {
            let elapsed = last_scan.elapsed().as_secs();
            output.push_str(&format!("  • Last scan: {}s ago\n", elapsed));
        }

        for monitor_state in self.monitors.values() {
            output.push_str(&format!("  • {}: ", monitor_state.name));
            if let Some(ref wallpaper) = monitor_state.current_wallpaper {
                output.push_str(&format!("{}\n", wallpaper.display()));
            } else {
                output.push_str("None\n");
            }
        }

        output
    }

    /// List all available wallpapers
    fn list_wallpapers(&self) -> String {
        let mut output = String::new();
        
        if self.wallpapers.is_empty() {
            output.push_str("No wallpapers found. Run 'wall scan' first.\n");
            return output;
        }

        output.push_str(&format!("📋 Available Wallpapers ({})\n", self.wallpapers.len()));
        
        for (i, wallpaper) in self.wallpapers.iter().enumerate() {
            let size_mb = wallpaper.size_bytes as f64 / 1_048_576.0;
            output.push_str(&format!(
                "  {}. {} ({:.1} MB)\n",
                i + 1,
                wallpaper.filename,
                size_mb
            ));
        }

        output
    }
}

#[async_trait]
impl Plugin for WallpapersPlugin {
    fn name(&self) -> &str {
        "wallpapers"
    }

    async fn init(&mut self, config: &toml::Value) -> Result<()> {
        info!("🖼️  Initializing wallpapers plugin");
        
        // Load configuration from plugin section
        if let Ok(wallpapers_config) = toml::from_str::<WallpapersConfig>(&config.to_string()) {
            self.config = wallpapers_config;
        }

        // Scan for wallpapers
        self.scan_wallpapers().await?;

        Ok(())
    }

    async fn handle_event(&mut self, _event: &HyprlandEvent) -> Result<()> {
        // Wallpapers plugin doesn't need to handle events
        Ok(())
    }

    async fn handle_command(&mut self, command: &str, args: &[&str]) -> Result<String> {

        match command {
            "next" => {
                if self.wallpapers.is_empty() {
                    self.scan_wallpapers().await?;
                    if self.wallpapers.is_empty() {
                        return Ok("No wallpapers found".to_string());
                    }
                }

                let monitors = self.get_monitor_names().await?;
                let mut results = Vec::new();

                for monitor_name in &monitors {
                    if let Some(wallpaper_path) = self.get_next_wallpaper(monitor_name) {
                        let filename = wallpaper_path.file_name()
                            .unwrap_or_default()
                            .to_string_lossy();
                        self.set_wallpaper_for_monitor(monitor_name, &wallpaper_path).await?;
                        results.push(format!("{}: {}", monitor_name, filename));
                    }
                }

                Ok(format!("Set wallpapers: {}", results.join(", ")))
            }

            "set" => {
                if args.is_empty() {
                    return Err(anyhow::anyhow!("Usage: wall set <path|filename>"));
                }

                let input = args[0];
                
                // First, try to find by filename in the scanned wallpapers
                if let Some(wallpaper) = self.wallpapers.iter().find(|w| w.filename == input) {
                    let wallpaper_path = wallpaper.path.clone();
                    let wallpaper_filename = wallpaper.filename.clone();
                    self.set_wallpaper(None, &wallpaper_path).await?;
                    return Ok(format!("Set wallpaper: {}", wallpaper_filename));
                }

                // If not found by filename, try as a full path
                let wallpaper_path = PathBuf::from(input);
                let expanded_path = self.expand_path(&wallpaper_path)?;

                if !expanded_path.exists() {
                    // If still not found, suggest available wallpapers
                    let available: Vec<String> = self.wallpapers.iter()
                        .take(5)
                        .map(|w| w.filename.clone())
                        .collect();
                    
                    let suggestion = if available.is_empty() {
                        "Run 'wall scan' first to discover wallpapers".to_string()
                    } else {
                        format!("Available wallpapers: {}", available.join(", "))
                    };
                    
                    return Err(anyhow::anyhow!("File not found: {}. {}", input, suggestion));
                }

                self.set_wallpaper(None, &expanded_path).await?;
                Ok(format!("Set wallpaper: {}", expanded_path.display()))
            }

            "scan" => {
                self.scan_wallpapers().await?;
                Ok(format!("Scanned wallpapers: {} found", self.wallpapers.len()))
            }

            "list" => Ok(self.list_wallpapers()),

            "status" => Ok(self.get_status()),

            "clear" => {
                self.clear_wallpapers().await?;
                Ok("Cleared all wallpapers".to_string())
            }

            "start" => {
                self.start_rotation().await?;
                Ok("Started wallpaper rotation".to_string())
            }

            "stop" => {
                self.stop_rotation().await?;
                Ok("Stopped wallpaper rotation".to_string())
            }

            _ => Ok(format!("Unknown wallpapers command: {command}. Available: next, set, scan, list, status, clear, start, stop")),
        }
    }

    async fn cleanup(&mut self) -> Result<()> {
        info!("🧹 Cleaning up wallpapers plugin");

        // Stop rotation if running
        if let Some(handle) = self.rotation_handle.take() {
            handle.abort();
            debug!("❌ Cancelled wallpaper rotation task");
        }

        // Clean up all active wallpaper backend processes
        if !self.active_processes.is_empty() {
            debug!("🔪 Terminating {} active wallpaper processes", self.active_processes.len());
            let _ = self.clear_wallpapers().await;
        }

        debug!("✅ Wallpapers plugin cleanup complete");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_wallpaper(filename: &str) -> WallpaperInfo {
        WallpaperInfo {
            path: PathBuf::from(format!("/tmp/{}", filename)),
            filename: filename.to_string(),
            size_bytes: 1024,
            last_modified: std::time::SystemTime::now(),
            dimensions: Some((1920, 1080)),
        }
    }

    #[test]
    fn test_plugin_creation() {
        let plugin = WallpapersPlugin::new();
        assert_eq!(plugin.name(), "wallpapers");
        assert_eq!(plugin.wallpapers.len(), 0);
    }

    #[test]
    fn test_config_default() {
        let config = WallpapersConfig::default();
        assert_eq!(config.interval, 600);
        assert_eq!(config.extensions.len(), 5);
        assert!(!config.recurse);
        assert!(!config.unique);
        assert!(!config.debug_logging);
        assert_eq!(config.preload_count, 3);
    }

    #[test]
    fn test_wallpaper_info_creation() {
        let wallpaper = create_test_wallpaper("test.jpg");
        assert_eq!(wallpaper.filename, "test.jpg");
        assert_eq!(wallpaper.size_bytes, 1024);
        assert_eq!(wallpaper.dimensions, Some((1920, 1080)));
    }

    #[test]
    fn test_wallpaper_path_enum() {
        let single_path = WallpaperPath::Single(PathBuf::from("/home/user/Pictures"));
        let multiple_paths = WallpaperPath::Multiple(vec![
            PathBuf::from("/home/user/Pictures"),
            PathBuf::from("/home/user/Wallpapers"),
        ]);

        match single_path {
            WallpaperPath::Single(_) => (),
            _ => panic!("Expected Single variant"),
        }

        match multiple_paths {
            WallpaperPath::Multiple(paths) => assert_eq!(paths.len(), 2),
            _ => panic!("Expected Multiple variant"),
        }
    }

    #[test]
    fn test_monitor_state_creation() {
        let monitor = MonitorState {
            name: "DP-1".to_string(),
            current_wallpaper: None,
            wallpaper_index: 0,
            last_change: Instant::now(),
        };

        assert_eq!(monitor.name, "DP-1");
        assert!(monitor.current_wallpaper.is_none());
        assert_eq!(monitor.wallpaper_index, 0);
    }

    #[test]
    fn test_config_serialization() {
        let config = WallpapersConfig::default();
        let toml_str = toml::to_string(&config).unwrap();
        
        assert!(toml_str.contains("interval"));
        assert!(toml_str.contains("extensions"));
    }

    #[tokio::test]
    async fn test_plugin_wallpaper_list() {
        let mut plugin = WallpapersPlugin::new();
        
        // Add some test wallpapers
        plugin.wallpapers = vec![
            create_test_wallpaper("test1.jpg"),
            create_test_wallpaper("test2.png"),
        ];

        let list = plugin.list_wallpapers();
        assert!(list.contains("test1.jpg"));
        assert!(list.contains("test2.png"));
        assert!(list.contains("Available Wallpapers (2)"));
    }

    #[tokio::test]
    async fn test_plugin_status() {
        let mut plugin = WallpapersPlugin::new();
        plugin.wallpapers = vec![create_test_wallpaper("test.jpg")];

        let status = plugin.get_status();
        assert!(status.contains("Wallpapers loaded: 1"));
        assert!(status.contains("Rotation active: false"));
    }
}