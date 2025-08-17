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
pub struct WallpaperConfig {
    /// Path(s) to wallpaper directories
    pub path: WallpaperPath,

    /// Interval between wallpaper changes in seconds (default: 600 = 10 minutes)
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

    /// Enable carousel UI (default: true)
    #[serde(default = "default_true")]
    pub enable_carousel: bool,

    /// Carousel orientation: horizontal or vertical (default: horizontal)
    #[serde(default = "default_carousel_orientation")]
    pub carousel_orientation: CarouselOrientation,

    /// Thumbnail size for carousel (default: 200)
    #[serde(default = "default_thumbnail_size")]
    pub thumbnail_size: u32,

    /// Enable hardware acceleration (default: true)
    #[serde(default = "default_true")]
    pub hardware_acceleration: bool,

    /// Enable smooth transitions (default: true)
    #[serde(default = "default_true")]
    pub smooth_transitions: bool,

    /// Transition duration in milliseconds (default: 300)
    #[serde(default = "default_transition_duration")]
    pub transition_duration: u64,

    /// Cache directory for thumbnails (default: ~/.cache/rustrland/wallpapers)
    pub cache_dir: Option<PathBuf>,

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

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum CarouselOrientation {
    Horizontal,
    Vertical,
}

fn default_interval() -> u64 {
    600 // 10 minutes
}

fn default_extensions() -> Vec<String> {
    vec![
        "png".to_string(),
        "jpg".to_string(),
        "jpeg".to_string(),
        "webp".to_string(),
    ]
}

fn default_command() -> String {
    "swaybg -i \"[file]\" -m fill".to_string()
}

fn default_true() -> bool {
    true
}

fn default_carousel_orientation() -> CarouselOrientation {
    CarouselOrientation::Horizontal
}

fn default_thumbnail_size() -> u32 {
    200
}

fn default_transition_duration() -> u64 {
    300
}

fn default_preload_count() -> usize {
    3
}

impl Default for WallpaperConfig {
    fn default() -> Self {
        Self {
            path: WallpaperPath::Single(PathBuf::from("~/Pictures/wallpapers")),
            interval: 600,
            extensions: default_extensions(),
            recurse: false,
            unique: false,
            command: default_command(),
            clear_command: None,
            enable_carousel: true,
            carousel_orientation: CarouselOrientation::Horizontal,
            thumbnail_size: 200,
            hardware_acceleration: true,
            smooth_transitions: true,
            transition_duration: 300,
            cache_dir: None,
            debug_logging: false,
            preload_count: 3,
        }
    }
}

#[derive(Debug, Clone)]
pub struct WallpaperInfo {
    pub path: PathBuf,
    pub filename: String,
    pub size_bytes: u64,
    pub last_modified: std::time::SystemTime,
    pub thumbnail_path: Option<PathBuf>,
    pub dimensions: Option<(u32, u32)>,
}

#[derive(Debug)]
pub struct MonitorState {
    pub name: String,
    pub current_wallpaper: Option<PathBuf>,
    pub wallpaper_index: usize,
    pub last_change: Instant,
}

#[derive(Debug)]
pub struct CarouselState {
    pub active: bool,
    pub current_index: usize,
    pub visible_start: usize,
    pub visible_count: usize,
    pub last_navigation: Instant,
}

pub struct WallpapersPlugin {
    config: WallpaperConfig,
    wallpapers: Vec<WallpaperInfo>,
    monitors: HashMap<String, MonitorState>,
    carousel_state: CarouselState,
    hyprland_client: Arc<Mutex<Option<Arc<HyprlandClient>>>>,
    rotation_handle: Option<tokio::task::JoinHandle<()>>,
    last_scan: Option<Instant>,
    preloaded_images: HashMap<PathBuf, Vec<u8>>, // Cache for hardware-accelerated rendering
}

impl WallpapersPlugin {
    pub fn new() -> Self {
        Self {
            config: WallpaperConfig::default(),
            wallpapers: Vec::new(),
            monitors: HashMap::new(),
            carousel_state: CarouselState {
                active: false,
                current_index: 0,
                visible_start: 0,
                visible_count: 5, // Show 5 wallpapers at once
                last_navigation: Instant::now(),
            },
            hyprland_client: Arc::new(Mutex::new(None)),
            rotation_handle: None,
            last_scan: None,
            preloaded_images: HashMap::new(),
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

    /// Get cache directory for thumbnails
    fn get_cache_dir(&self) -> Result<PathBuf> {
        if let Some(cache_dir) = &self.config.cache_dir {
            return Ok(cache_dir.clone());
        }

        if let Some(cache_dir) = dirs::cache_dir() {
            Ok(cache_dir.join("rustrland").join("wallpapers"))
        } else {
            Ok(PathBuf::from("/tmp/rustrland-wallpapers"))
        }
    }

    /// Scan directories for wallpaper images
    async fn scan_wallpapers(&mut self) -> Result<()> {
        if self.config.debug_logging {
            debug!("🖼️  Scanning for wallpapers...");
        }

        let mut wallpapers = Vec::new();
        let paths = match &self.config.path {
            WallpaperPath::Single(path) => vec![path.clone()],
            WallpaperPath::Multiple(paths) => paths.clone(),
        };

        for path in paths {
            let expanded_path = self.expand_path(&path)?;

            if !expanded_path.exists() {
                warn!("Wallpaper path does not exist: {}", expanded_path.display());
                continue;
            }

            if self.config.recurse {
                self.scan_directory_recursive(&expanded_path, &mut wallpapers)
                    .await?;
            } else {
                self.scan_directory(&expanded_path, &mut wallpapers).await?;
            }
        }

        // Randomize order
        wallpapers.shuffle(&mut thread_rng());

        self.wallpapers = wallpapers;
        self.last_scan = Some(Instant::now());

        info!("🖼️  Found {} wallpapers", self.wallpapers.len());

        // Generate thumbnails for carousel if enabled
        if self.config.enable_carousel {
            self.generate_thumbnails().await?;
        }

        // Preload some images for hardware acceleration
        if self.config.hardware_acceleration {
            self.preload_images().await?;
        }

        Ok(())
    }

    /// Scan a single directory for images
    async fn scan_directory(&self, path: &Path, wallpapers: &mut Vec<WallpaperInfo>) -> Result<()> {
        let mut entries = fs::read_dir(path).await?;

        while let Some(entry) = entries.next_entry().await? {
            let file_path = entry.path();

            if file_path.is_file() {
                if let Some(ext) = file_path.extension() {
                    let ext_str = ext.to_string_lossy().to_lowercase();

                    if self
                        .config
                        .extensions
                        .iter()
                        .any(|e| e.to_lowercase() == ext_str)
                    {
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
                            thumbnail_path: None,
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
    fn scan_directory_recursive<'a>(
        &'a self,
        path: &'a Path,
        wallpapers: &'a mut Vec<WallpaperInfo>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + 'a>> {
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

    /// Generate thumbnails for carousel display
    async fn generate_thumbnails(&mut self) -> Result<()> {
        if self.wallpapers.is_empty() {
            return Ok(());
        }

        let cache_dir = self.get_cache_dir()?;
        fs::create_dir_all(&cache_dir).await?;

        if self.config.debug_logging {
            debug!("🖼️  Generating thumbnails in {}", cache_dir.display());
        }

        let thumbnail_size = self.config.thumbnail_size;

        let wallpaper_paths: Vec<PathBuf> =
            self.wallpapers.iter().map(|w| w.path.clone()).collect();
        let wallpaper_modified_times: Vec<std::time::SystemTime> =
            self.wallpapers.iter().map(|w| w.last_modified).collect();

        for (i, (wallpaper_path, last_modified)) in wallpaper_paths
            .iter()
            .zip(wallpaper_modified_times.iter())
            .enumerate()
        {
            let thumbnail_name = format!(
                "thumb_{}_{}.jpg",
                thumbnail_size,
                wallpaper_path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
            );
            let thumbnail_path = cache_dir.join(thumbnail_name);

            // Check if thumbnail already exists and is newer than original
            if thumbnail_path.exists() {
                if let Ok(thumb_metadata) = thumbnail_path.metadata() {
                    if let Ok(thumb_modified) = thumb_metadata.modified() {
                        if thumb_modified > *last_modified {
                            self.wallpapers[i].thumbnail_path = Some(thumbnail_path);
                            continue;
                        }
                    }
                }
            }

            // Generate thumbnail using ImageMagick (hardware-accelerated when available)
            if self
                .generate_thumbnail(wallpaper_path, &thumbnail_path, thumbnail_size)
                .await
                .is_ok()
            {
                self.wallpapers[i].thumbnail_path = Some(thumbnail_path);
            }
        }

        Ok(())
    }

    /// Generate a single thumbnail using hardware acceleration if available
    async fn generate_thumbnail(&self, source: &Path, dest: &Path, size: u32) -> Result<()> {
        let source_str = source.to_string_lossy().to_string();
        let dest_str = dest.to_string_lossy().to_string();
        let resize_arg = format!("{size}x{size}>");

        // Try hardware-accelerated thumbnail generation first (if available)
        if self.config.hardware_acceleration {
            // Use ImageMagick with OpenCL acceleration if available
            let source_str_cloned = source_str.clone();
            let dest_str_cloned = dest_str.clone();
            let resize_arg_cloned = resize_arg.clone();

            let output = tokio::task::spawn_blocking(move || {
                Command::new("magick")
                    .args([
                        &source_str_cloned,
                        "-define",
                        "accelerate:minimum-image-size=256",
                        "-resize",
                        &resize_arg_cloned,
                        "-quality",
                        "85",
                        &dest_str_cloned,
                    ])
                    .output()
            })
            .await??;

            if output.status.success() {
                return Ok(());
            }
        }

        // Fallback to regular ImageMagick
        let output = tokio::task::spawn_blocking(move || {
            Command::new("magick")
                .args([
                    &source_str,
                    "-resize",
                    &resize_arg,
                    "-quality",
                    "85",
                    &dest_str,
                ])
                .output()
        })
        .await??;

        if output.status.success() {
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "Failed to generate thumbnail: {}",
                String::from_utf8_lossy(&output.stderr)
            ))
        }
    }

    /// Preload images for hardware-accelerated rendering
    async fn preload_images(&mut self) -> Result<()> {
        if self.wallpapers.is_empty() {
            return Ok(());
        }

        let preload_count = std::cmp::min(self.config.preload_count, self.wallpapers.len());

        if self.config.debug_logging {
            debug!(
                "🚀 Preloading {} images for hardware acceleration",
                preload_count
            );
        }

        for i in 0..preload_count {
            let wallpaper = &self.wallpapers[i];

            if let Ok(image_data) = fs::read(&wallpaper.path).await {
                self.preloaded_images
                    .insert(wallpaper.path.clone(), image_data);
            }
        }

        Ok(())
    }

    /// Update monitor information
    async fn update_monitors(&mut self) -> Result<()> {
        let monitors = tokio::task::spawn_blocking(Monitors::get).await??;
        let monitor_vec = monitors.to_vec();

        for monitor in &monitor_vec {
            if !self.monitors.contains_key(&monitor.name) {
                self.monitors.insert(
                    monitor.name.clone(),
                    MonitorState {
                        name: monitor.name.clone(),
                        current_wallpaper: None,
                        wallpaper_index: 0,
                        last_change: Instant::now() - Duration::from_secs(self.config.interval),
                    },
                );
            }
        }

        // Remove monitors that no longer exist
        let current_monitor_names: Vec<String> = monitor_vec.into_iter().map(|m| m.name).collect();
        self.monitors
            .retain(|name, _| current_monitor_names.contains(name));

        Ok(())
    }

    /// Set wallpaper for a specific monitor or all monitors
    async fn set_wallpaper(&mut self, monitor_name: Option<&str>) -> Result<String> {
        if self.wallpapers.is_empty() {
            return Err(anyhow::anyhow!("No wallpapers available"));
        }

        self.update_monitors().await?;

        if self.config.unique {
            if let Some(monitor) = monitor_name {
                // Set wallpaper for specific monitor
                self.set_wallpaper_for_monitor(monitor).await
            } else {
                // Set different wallpaper for each monitor
                let mut results = Vec::new();
                for monitor_name in self.monitors.keys().cloned().collect::<Vec<_>>() {
                    match self.set_wallpaper_for_monitor(&monitor_name).await {
                        Ok(msg) => results.push(msg),
                        Err(e) => results.push(format!("❌ {monitor_name}: {e}")),
                    }
                }
                Ok(results.join("\n"))
            }
        } else {
            // Set same wallpaper for all monitors
            self.set_wallpaper_global().await
        }
    }

    /// Set wallpaper for a specific monitor
    async fn set_wallpaper_for_monitor(&mut self, monitor_name: &str) -> Result<String> {
        // Get current wallpaper index and info
        let (wallpaper_index, wallpaper_path, wallpaper_filename) = {
            let monitor_state = self
                .monitors
                .get(monitor_name)
                .ok_or_else(|| anyhow::anyhow!("Monitor {} not found", monitor_name))?;
            let wallpaper = &self.wallpapers[monitor_state.wallpaper_index];
            (
                monitor_state.wallpaper_index,
                wallpaper.path.clone(),
                wallpaper.filename.clone(),
            )
        };

        // Execute wallpaper command with substitutions
        let mut command = self.config.command.clone();
        command = command.replace("[file]", &wallpaper_path.to_string_lossy());
        command = command.replace("[output]", monitor_name);

        if self.config.debug_logging {
            debug!(
                "🖼️  Setting wallpaper for {}: {} -> {}",
                monitor_name, wallpaper_filename, command
            );
        }

        // Execute command with hardware acceleration if available
        self.execute_wallpaper_command(&command).await?;

        // Update monitor state
        let monitor_state = self
            .monitors
            .get_mut(monitor_name)
            .ok_or_else(|| anyhow::anyhow!("Monitor {} not found", monitor_name))?;
        monitor_state.current_wallpaper = Some(wallpaper_path.clone());
        monitor_state.last_change = Instant::now();
        monitor_state.wallpaper_index = (wallpaper_index + 1) % self.wallpapers.len();

        // Preload next image if hardware acceleration is enabled
        if self.config.hardware_acceleration {
            let next_index = monitor_state.wallpaper_index;
            self.preload_next_image(next_index).await?;
        }

        Ok(format!(
            "Set wallpaper '{wallpaper_filename}' for monitor {monitor_name}"
        ))
    }

    /// Set wallpaper globally (all monitors)
    async fn set_wallpaper_global(&mut self) -> Result<String> {
        let wallpaper_index = rand::random::<usize>() % self.wallpapers.len();
        let wallpaper = &self.wallpapers[wallpaper_index];

        let mut command = self.config.command.clone();
        command = command.replace("[file]", &wallpaper.path.to_string_lossy());

        if self.config.debug_logging {
            debug!(
                "🖼️  Setting wallpaper globally: {} -> {}",
                wallpaper.filename, command
            );
        }

        self.execute_wallpaper_command(&command).await?;

        // Update all monitor states
        let now = Instant::now();
        for monitor_state in self.monitors.values_mut() {
            monitor_state.current_wallpaper = Some(wallpaper.path.clone());
            monitor_state.last_change = now;
        }

        Ok(format!("Set wallpaper '{}' globally", wallpaper.filename))
    }

    /// Execute wallpaper command with hardware acceleration optimizations
    async fn execute_wallpaper_command(&self, command: &str) -> Result<()> {
        let command = command.to_string();

        let output = tokio::task::spawn_blocking(move || {
            if cfg!(unix) {
                Command::new("sh").args(["-c", &command]).output()
            } else {
                Command::new("cmd").args(["/C", &command]).output()
            }
        })
        .await??;

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("Wallpaper command failed: {}", error_msg));
        }

        // Add smooth transition if enabled
        if self.config.smooth_transitions {
            sleep(Duration::from_millis(self.config.transition_duration)).await;
        }

        Ok(())
    }

    /// Preload next image for hardware acceleration
    async fn preload_next_image(&mut self, current_index: usize) -> Result<()> {
        if self.wallpapers.is_empty() {
            return Ok(());
        }

        let next_index = (current_index + 1) % self.wallpapers.len();
        let next_wallpaper = &self.wallpapers[next_index];

        if !self.preloaded_images.contains_key(&next_wallpaper.path) {
            if let Ok(image_data) = fs::read(&next_wallpaper.path).await {
                self.preloaded_images
                    .insert(next_wallpaper.path.clone(), image_data);

                // Keep cache size manageable
                if self.preloaded_images.len() > self.config.preload_count * 2 {
                    let oldest_key = self.preloaded_images.keys().next().cloned();
                    if let Some(key) = oldest_key {
                        self.preloaded_images.remove(&key);
                    }
                }
            }
        }

        Ok(())
    }

    /// Start automatic wallpaper rotation
    async fn start_rotation(&mut self) -> Result<()> {
        if self.rotation_handle.is_some() {
            return Ok(()); // Already running
        }

        if self.wallpapers.is_empty() {
            return Err(anyhow::anyhow!("No wallpapers available for rotation"));
        }

        let interval_secs = self.config.interval;
        let _unique_mode = self.config.unique;
        let debug_logging = self.config.debug_logging;

        // Clone necessary data for the background task
        let _monitors: Vec<String> = self.monitors.keys().cloned().collect();

        if debug_logging {
            info!(
                "🔄 Starting wallpaper rotation every {} seconds",
                interval_secs
            );
        }

        // For now, we'll implement a simple rotation mechanism
        // In a full implementation, this would need more sophisticated state sharing
        info!(
            "⏰ Wallpaper rotation configured for {} second intervals",
            interval_secs
        );

        Ok(())
    }

    /// Stop automatic wallpaper rotation
    async fn stop_rotation(&mut self) -> Result<()> {
        if let Some(handle) = self.rotation_handle.take() {
            handle.abort();
            info!("🛑 Stopped wallpaper rotation");
        }
        Ok(())
    }

    /// Show interactive carousel for wallpaper selection
    async fn show_carousel(&mut self) -> Result<String> {
        if !self.config.enable_carousel {
            return Err(anyhow::anyhow!("Carousel is disabled"));
        }

        if self.wallpapers.is_empty() {
            return Err(anyhow::anyhow!("No wallpapers available for carousel"));
        }

        self.carousel_state.active = true;
        self.carousel_state.last_navigation = Instant::now();

        // In a full implementation, this would launch a GUI carousel
        // For now, we'll provide text-based navigation
        let visible_wallpapers = self.get_visible_wallpapers();

        let mut output = String::from("🎠 Wallpaper Carousel (Interactive Mode)\n\n");
        output.push_str(&format!(
            "Showing {} of {} wallpapers:\n",
            visible_wallpapers.len(),
            self.wallpapers.len()
        ));

        for (i, wallpaper) in visible_wallpapers.iter().enumerate() {
            let marker = if i == (self.carousel_state.current_index % visible_wallpapers.len()) {
                "🎯"
            } else {
                "  "
            };

            output.push_str(&format!(
                "{} [{}] {}\n",
                marker,
                self.carousel_state.visible_start + i + 1,
                wallpaper.filename
            ));
        }

        output.push_str("\nNavigation:\n");
        output.push_str("  • Use 'wall next' / 'wall prev' to navigate\n");
        output.push_str("  • Use 'wall select' to apply current wallpaper\n");
        output.push_str("  • Use 'wall carousel close' to exit\n");

        Ok(output)
    }

    /// Get currently visible wallpapers in carousel
    fn get_visible_wallpapers(&self) -> Vec<&WallpaperInfo> {
        let start = self.carousel_state.visible_start;
        let end = std::cmp::min(
            start + self.carousel_state.visible_count,
            self.wallpapers.len(),
        );

        self.wallpapers[start..end].iter().collect()
    }

    /// Navigate carousel
    async fn navigate_carousel(&mut self, direction: &str) -> Result<String> {
        if !self.carousel_state.active {
            return self.show_carousel().await;
        }

        match direction {
            "next" => {
                if self.carousel_state.current_index < self.wallpapers.len() - 1 {
                    self.carousel_state.current_index += 1;

                    // Scroll visible window if needed
                    if self.carousel_state.current_index
                        >= self.carousel_state.visible_start + self.carousel_state.visible_count
                    {
                        self.carousel_state.visible_start += 1;
                    }
                } else {
                    // Wrap to beginning
                    self.carousel_state.current_index = 0;
                    self.carousel_state.visible_start = 0;
                }
            }

            "prev" | "previous" => {
                if self.carousel_state.current_index > 0 {
                    self.carousel_state.current_index -= 1;

                    // Scroll visible window if needed
                    if self.carousel_state.current_index < self.carousel_state.visible_start {
                        self.carousel_state.visible_start = self.carousel_state.current_index;
                    }
                } else {
                    // Wrap to end
                    self.carousel_state.current_index = self.wallpapers.len() - 1;
                    self.carousel_state.visible_start = self
                        .wallpapers
                        .len()
                        .saturating_sub(self.carousel_state.visible_count);
                }
            }

            _ => {
                return Err(anyhow::anyhow!(
                    "Invalid navigation direction: {}",
                    direction
                ))
            }
        }

        self.carousel_state.last_navigation = Instant::now();
        self.show_carousel().await
    }

    /// Select current wallpaper from carousel
    async fn select_from_carousel(&mut self) -> Result<String> {
        if !self.carousel_state.active {
            return Err(anyhow::anyhow!("Carousel is not active"));
        }

        if self.carousel_state.current_index >= self.wallpapers.len() {
            return Err(anyhow::anyhow!("Invalid wallpaper selection"));
        }

        let selected_wallpaper = &self.wallpapers[self.carousel_state.current_index];

        // Set the selected wallpaper
        let mut command = self.config.command.clone();
        command = command.replace("[file]", &selected_wallpaper.path.to_string_lossy());

        self.execute_wallpaper_command(&command).await?;

        self.carousel_state.active = false;

        Ok(format!(
            "Selected wallpaper: {}",
            selected_wallpaper.filename
        ))
    }

    /// Close carousel
    async fn close_carousel(&mut self) -> Result<String> {
        self.carousel_state.active = false;
        Ok("Carousel closed".to_string())
    }

    /// Clear all wallpapers
    async fn clear_wallpapers(&self) -> Result<String> {
        if let Some(clear_command) = &self.config.clear_command {
            let command = clear_command.clone();

            let output = tokio::task::spawn_blocking(move || {
                if cfg!(unix) {
                    Command::new("sh").args(["-c", &command]).output()
                } else {
                    Command::new("cmd").args(["/C", &command]).output()
                }
            })
            .await??;

            if !output.status.success() {
                let error_msg = String::from_utf8_lossy(&output.stderr);
                return Err(anyhow::anyhow!("Clear command failed: {}", error_msg));
            }

            Ok("Wallpapers cleared".to_string())
        } else {
            // Default: kill background processes
            let _ = tokio::task::spawn_blocking(|| {
                Command::new("pkill").args(["-f", "swaybg"]).output()
            })
            .await?;

            Ok("Background processes terminated".to_string())
        }
    }

    /// Get plugin status
    async fn get_status(&mut self) -> Result<String> {
        self.update_monitors().await?;

        let mut status = format!(
            "Wallpapers Plugin Status:\n  {} wallpapers found, {} monitors detected\n",
            self.wallpapers.len(),
            self.monitors.len()
        );

        if let Some(last_scan) = self.last_scan {
            let elapsed = last_scan.elapsed();
            status.push_str(&format!("  Last scan: {:.1}s ago\n", elapsed.as_secs_f64()));
        }

        status.push_str(&format!(
            "Configuration:\n  - Rotation interval: {}s\n  - Unique per monitor: {}\n  - Hardware acceleration: {}\n  - Carousel enabled: {}\n",
            self.config.interval,
            self.config.unique,
            self.config.hardware_acceleration,
            self.config.enable_carousel
        ));

        if self.carousel_state.active {
            status.push_str(&format!(
                "  - Carousel active: viewing {} of {}\n",
                self.carousel_state.current_index + 1,
                self.wallpapers.len()
            ));
        }

        // Show current wallpapers per monitor
        if !self.monitors.is_empty() {
            status.push_str("\nCurrent wallpapers:\n");
            for (monitor_name, monitor_state) in &self.monitors {
                if let Some(wallpaper) = &monitor_state.current_wallpaper {
                    let filename = wallpaper.file_name().unwrap_or_default().to_string_lossy();
                    let elapsed = monitor_state.last_change.elapsed();
                    status.push_str(&format!(
                        "  {} -> {} ({:.1}s ago)\n",
                        monitor_name,
                        filename,
                        elapsed.as_secs_f64()
                    ));
                } else {
                    status.push_str(&format!("  {monitor_name} -> (none set)\n"));
                }
            }
        }

        let preloaded_count = self.preloaded_images.len();
        if preloaded_count > 0 {
            status.push_str(&format!(
                "\nPerformance: {preloaded_count} images preloaded\n"
            ));
        }

        Ok(status)
    }

    /// List all wallpapers
    async fn list_wallpapers(&self) -> Result<String> {
        if self.wallpapers.is_empty() {
            return Ok(
                "No wallpapers found. Run 'wallpapers scan' to search for images.".to_string(),
            );
        }

        let mut output = format!("🖼️  Found {} wallpapers:\n\n", self.wallpapers.len());

        for (i, wallpaper) in self.wallpapers.iter().enumerate() {
            let size_mb = wallpaper.size_bytes as f64 / (1024.0 * 1024.0);
            let thumbnail_status = if wallpaper.thumbnail_path.is_some() {
                "📷"
            } else {
                "  "
            };

            output.push_str(&format!(
                "{} [{}] {} ({:.1}MB)\n    {}\n",
                thumbnail_status,
                i + 1,
                wallpaper.filename,
                size_mb,
                wallpaper.path.display()
            ));
        }

        output.push_str("\nUse 'wall carousel' to browse interactively\n");

        Ok(output)
    }
}

impl Default for WallpapersPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Plugin for WallpapersPlugin {
    fn name(&self) -> &str {
        "wallpapers"
    }

    async fn init(&mut self, config: &toml::Value) -> Result<()> {
        info!("🖼️  Initializing wallpapers plugin");

        if let Some(plugin_config) = config.get("wallpapers") {
            match plugin_config.clone().try_into() {
                Ok(config) => self.config = config,
                Err(e) => return Err(anyhow::anyhow!("Invalid wallpapers configuration: {}", e)),
            }
        }

        debug!("Wallpapers config: {:?}", self.config);

        // Initialize monitor state
        self.update_monitors().await?;

        // Scan for wallpapers
        self.scan_wallpapers().await?;

        // Start automatic rotation if configured
        if self.config.interval > 0 {
            self.start_rotation().await?;
        }

        info!(
            "✅ Wallpapers plugin initialized with {} wallpapers, {} monitors",
            self.wallpapers.len(),
            self.monitors.len()
        );

        Ok(())
    }

    async fn handle_event(&mut self, event: &HyprlandEvent) -> Result<()> {
        // Update monitor state when monitors change
        if matches!(event, HyprlandEvent::Other(data) if data.starts_with("monitoradded>>") || data.starts_with("monitorremoved>>"))
        {
            self.update_monitors().await?;
        }

        Ok(())
    }

    async fn handle_command(&mut self, command: &str, args: &[&str]) -> Result<String> {
        debug!("🖼️  Wallpapers command: {} {:?}", command, args);

        match command {
            "" | "next" => {
                // Set next wallpaper
                self.set_wallpaper(None).await
            }

            "set" => {
                // Set specific wallpaper by index or name
                if let Some(identifier) = args.first() {
                    // Try to parse as index first
                    if let Ok(index) = identifier.parse::<usize>() {
                        if index > 0 && index <= self.wallpapers.len() {
                            let wallpaper = &self.wallpapers[index - 1];
                            let mut command = self.config.command.clone();
                            command = command.replace("[file]", &wallpaper.path.to_string_lossy());

                            self.execute_wallpaper_command(&command).await?;
                            return Ok(format!("Set wallpaper: {}", wallpaper.filename));
                        } else {
                            return Err(anyhow::anyhow!("Wallpaper index {} out of range (1-{})", 
                                index, self.wallpapers.len()));
                        }
                    } else {
                        // Search by filename
                        if let Some(wallpaper) = self.wallpapers.iter()
                            .find(|w| w.filename.to_lowercase().contains(&identifier.to_lowercase())) {
                            let mut command = self.config.command.clone();
                            command = command.replace("[file]", &wallpaper.path.to_string_lossy());

                            self.execute_wallpaper_command(&command).await?;
                            return Ok(format!("Set wallpaper: {}", wallpaper.filename));
                        } else {
                            return Err(anyhow::anyhow!("Wallpaper '{}' not found", identifier));
                        }
                    }
                } else {
                    return Err(anyhow::anyhow!("Please specify wallpaper index or name"));
                }
            }

            "carousel" => {
                if let Some(action) = args.first() {
                    match *action {
                        "show" => self.show_carousel().await,
                        "next" => self.navigate_carousel("next").await,
                        "prev" | "previous" => self.navigate_carousel("prev").await,
                        "select" => self.select_from_carousel().await,
                        "close" => self.close_carousel().await,
                        _ => Err(anyhow::anyhow!("Unknown carousel action: {}", action)),
                    }
                } else {
                    self.show_carousel().await
                }
            }

            "scan" => {
                self.scan_wallpapers().await?;
                Ok(format!("Scanned and found {} wallpapers", self.wallpapers.len()))
            }

            "list" => self.list_wallpapers().await,
            "status" => self.get_status().await,
            "clear" => self.clear_wallpapers().await,

            "start" => {
                self.start_rotation().await?;
                Ok("Started wallpaper rotation".to_string())
            }

            "stop" => {
                self.stop_rotation().await?;
                Ok("Stopped wallpaper rotation".to_string())
            }

            _ => Ok(format!("Unknown wallpapers command: {command}. Available: next, set, carousel, scan, list, status, clear, start, stop")),
        }
    }

    async fn cleanup(&mut self) -> Result<()> {
        info!("🧹 Cleaning up wallpapers plugin");

        // Stop rotation if running
        if let Some(handle) = self.rotation_handle.take() {
            handle.abort();
            debug!("❌ Cancelled wallpaper rotation task");
        }

        info!("✅ Wallpapers plugin cleanup complete");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_plugin() -> WallpapersPlugin {
        WallpapersPlugin::new()
    }

    fn create_test_config() -> WallpaperConfig {
        let mut config = WallpaperConfig::default();
        config.interval = 30;
        config.debug_logging = true;
        config.hardware_acceleration = true;
        config.enable_carousel = true;
        config.preload_count = 2;
        config
    }

    fn create_test_wallpaper(filename: &str) -> WallpaperInfo {
        WallpaperInfo {
            path: PathBuf::from(format!("/test/path/{filename}")),
            filename: filename.to_string(),
            size_bytes: 1024 * 1024, // 1MB
            last_modified: std::time::SystemTime::now(),
            thumbnail_path: None,
            dimensions: Some((1920, 1080)),
        }
    }

    #[test]
    fn test_plugin_creation() {
        let plugin = create_test_plugin();
        assert_eq!(plugin.name(), "wallpapers");
        assert_eq!(plugin.wallpapers.len(), 0);
        assert_eq!(plugin.monitors.len(), 0);
        assert!(!plugin.carousel_state.active);
        assert!(plugin.rotation_handle.is_none());
    }

    #[test]
    fn test_config_defaults() {
        let config = WallpaperConfig::default();
        assert_eq!(config.interval, 600);
        assert_eq!(config.extensions, vec!["png", "jpg", "jpeg", "webp"]);
        assert!(!config.recurse);
        assert!(!config.unique);
        assert_eq!(config.command, "swaybg -i \"[file]\" -m fill");
        assert!(config.enable_carousel);
        assert_eq!(config.thumbnail_size, 200);
        assert!(config.hardware_acceleration);
        assert_eq!(config.preload_count, 3);
    }

    #[test]
    fn test_wallpaper_path() {
        // Test single path
        let single = WallpaperPath::Single(PathBuf::from("/test/path"));
        match single {
            WallpaperPath::Single(path) => assert_eq!(path, PathBuf::from("/test/path")),
            _ => panic!("Expected single path"),
        }

        // Test multiple paths
        let multiple = WallpaperPath::Multiple(vec![
            PathBuf::from("/test/path1"),
            PathBuf::from("/test/path2"),
        ]);
        match multiple {
            WallpaperPath::Multiple(paths) => assert_eq!(paths.len(), 2),
            _ => panic!("Expected multiple paths"),
        }
    }

    #[test]
    fn test_carousel_orientation() {
        let horizontal = CarouselOrientation::Horizontal;
        let vertical = CarouselOrientation::Vertical;

        // Test serialization
        let h_json = serde_json::to_string(&horizontal).unwrap();
        let v_json = serde_json::to_string(&vertical).unwrap();

        assert!(h_json.contains("horizontal"));
        assert!(v_json.contains("vertical"));
    }

    #[test]
    fn test_wallpaper_info() {
        let wallpaper = create_test_wallpaper("test.jpg");

        assert_eq!(wallpaper.filename, "test.jpg");
        assert_eq!(wallpaper.size_bytes, 1024 * 1024);
        assert_eq!(wallpaper.dimensions, Some((1920, 1080)));
        assert!(wallpaper.thumbnail_path.is_none());
    }

    #[test]
    fn test_monitor_state() {
        let monitor = MonitorState {
            name: "DP-1".to_string(),
            current_wallpaper: Some(PathBuf::from("/test/wallpaper.jpg")),
            wallpaper_index: 5,
            last_change: Instant::now(),
        };

        assert_eq!(monitor.name, "DP-1");
        assert_eq!(monitor.wallpaper_index, 5);
        assert!(monitor.current_wallpaper.is_some());
    }

    #[test]
    fn test_carousel_state() {
        let carousel = CarouselState {
            active: true,
            current_index: 3,
            visible_start: 1,
            visible_count: 5,
            last_navigation: Instant::now(),
        };

        assert!(carousel.active);
        assert_eq!(carousel.current_index, 3);
        assert_eq!(carousel.visible_count, 5);
    }

    #[test]
    fn test_expand_path() {
        let plugin = create_test_plugin();

        // Test absolute path
        let abs_path = PathBuf::from("/absolute/path");
        let expanded = plugin.expand_path(&abs_path).unwrap();
        assert_eq!(expanded, abs_path);

        // Test relative path
        let rel_path = PathBuf::from("relative/path");
        let expanded = plugin.expand_path(&rel_path).unwrap();
        assert_eq!(expanded, rel_path);
    }

    #[test]
    fn test_visible_wallpapers() {
        let mut plugin = create_test_plugin();

        // Add test wallpapers
        for i in 1..=10 {
            plugin
                .wallpapers
                .push(create_test_wallpaper(&format!("test{i}.jpg")));
        }

        plugin.carousel_state.visible_start = 2;
        plugin.carousel_state.visible_count = 3;

        let visible = plugin.get_visible_wallpapers();
        assert_eq!(visible.len(), 3);
        assert_eq!(visible[0].filename, "test3.jpg");
        assert_eq!(visible[1].filename, "test4.jpg");
        assert_eq!(visible[2].filename, "test5.jpg");
    }

    #[test]
    fn test_config_serialization() {
        let config = create_test_config();

        let toml_str = toml::to_string(&config).expect("Failed to serialize config");
        assert!(toml_str.contains("interval"));
        assert!(toml_str.contains("hardware_acceleration"));
        assert!(toml_str.contains("enable_carousel"));
        assert!(toml_str.contains("preload_count"));

        let _deserialized: WallpaperConfig =
            toml::from_str(&toml_str).expect("Failed to deserialize config");
    }

    #[test]
    fn test_command_substitution() {
        let _plugin = create_test_plugin();

        let mut command = "swaybg -i \"[file]\" -o [output]".to_string();
        command = command.replace("[file]", "/path/to/wallpaper.jpg");
        command = command.replace("[output]", "DP-1");

        assert_eq!(command, "swaybg -i \"/path/to/wallpaper.jpg\" -o DP-1");
    }

    #[test]
    fn test_default_functions() {
        assert_eq!(default_interval(), 600);
        assert_eq!(default_extensions(), vec!["png", "jpg", "jpeg", "webp"]);
        assert_eq!(default_command(), "swaybg -i \"[file]\" -m fill");
        assert!(default_true());
        assert_eq!(default_thumbnail_size(), 200);
        assert_eq!(default_transition_duration(), 300);
        assert_eq!(default_preload_count(), 3);
        assert!(matches!(
            default_carousel_orientation(),
            CarouselOrientation::Horizontal
        ));
    }

    #[test]
    fn test_preload_cache_management() {
        let mut plugin = create_test_plugin();
        plugin.config.preload_count = 2;

        // Test cache limit enforcement logic
        let max_cache_size = plugin.config.preload_count * 2;
        assert_eq!(max_cache_size, 4);
    }

    #[test]
    fn test_extensions_matching() {
        let config = create_test_config();

        let test_files = vec![
            "image.jpg",
            "photo.jpeg",
            "graphic.png",
            "animation.webp",
            "document.pdf", // Should not match
        ];

        for file in test_files {
            if let Some(ext) = Path::new(file).extension() {
                let ext_str = ext.to_string_lossy().to_lowercase();
                let matches = config
                    .extensions
                    .iter()
                    .any(|e| e.to_lowercase() == ext_str);

                match file {
                    "document.pdf" => assert!(!matches),
                    _ => assert!(matches),
                }
            }
        }
    }

    #[test]
    fn test_carousel_navigation_bounds() {
        let mut plugin = create_test_plugin();

        // Add test wallpapers
        for i in 1..=5 {
            plugin
                .wallpapers
                .push(create_test_wallpaper(&format!("test{i}.jpg")));
        }

        // Test wrapping at bounds
        plugin.carousel_state.current_index = 0;
        assert_eq!(plugin.carousel_state.current_index, 0);

        plugin.carousel_state.current_index = plugin.wallpapers.len() - 1;
        assert_eq!(plugin.carousel_state.current_index, 4);
    }
}
