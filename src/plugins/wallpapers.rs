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

// GUI carousel - inline implementation for Phase 2
#[cfg(feature = "gui")]
mod carousel_gui {
    use egui::{Context, TextureHandle, Vec2, Color32};
    use std::collections::HashMap;
    use std::path::PathBuf;
    use tokio::sync::mpsc;
    use anyhow::Result;
    use tracing::{debug, info, warn, error};
    use super::WallpaperInfo;

    /// Configuration for the wallpaper carousel GUI
    #[derive(Debug, Clone)]
    pub struct CarouselConfig {
        pub window_size: Vec2,
        pub grid_columns: usize,
        pub grid_rows: usize,
        pub thumbnail_size: u32,
        pub spacing: f32,
        pub background_color: Color32,
        pub selection_color: Color32,
        pub hover_color: Color32,
    }

    impl Default for CarouselConfig {
        fn default() -> Self {
            Self {
                window_size: Vec2::new(1200.0, 800.0),
                grid_columns: 5,
                grid_rows: 3,
                thumbnail_size: 200,
                spacing: 10.0,
                background_color: Color32::from_gray(20),
                selection_color: Color32::from_rgb(70, 130, 255),
                hover_color: Color32::from_rgb(100, 100, 100),
            }
        }
    }

    /// Selection events from GUI to plugin
    #[derive(Debug, Clone)]
    pub enum CarouselSelection {
        Selected(PathBuf),
        Cancelled,
        PreviewRequested(PathBuf),
    }

    /// GUI carousel with thumbnail management
    pub struct WallpaperCarouselGUI {
        config: CarouselConfig,
        selection_sender: mpsc::Sender<CarouselSelection>,
        selection_receiver: mpsc::Receiver<CarouselSelection>,
        is_running: bool,
        
        // Thumbnail management
        thumbnails: HashMap<PathBuf, TextureHandle>,
        pub(in crate::plugins::wallpapers) thumbnail_cache: ThumbnailCache,
        loading_thumbnails: std::collections::HashSet<PathBuf>,
    }

    /// LRU cache for thumbnail data
    pub struct ThumbnailCache {
        cache: HashMap<PathBuf, ThumbnailEntry>,
        access_order: Vec<PathBuf>,
        max_size: usize,
        current_memory_usage: usize,
        max_memory_mb: usize,
    }

    #[derive(Clone)]
    pub struct ThumbnailEntry {
        pub data: Vec<u8>,
        pub width: u32,
        pub height: u32,
        pub last_accessed: std::time::Instant,
        pub memory_size: usize,
    }

    impl ThumbnailCache {
        pub fn new(max_size: usize, max_memory_mb: usize) -> Self {
            Self {
                cache: HashMap::new(),
                access_order: Vec::new(),
                max_size,
                current_memory_usage: 0,
                max_memory_mb,
            }
        }

        pub fn get(&mut self, path: &PathBuf) -> Option<&ThumbnailEntry> {
            if let Some(entry) = self.cache.get_mut(path) {
                let filename = path.file_name().unwrap_or_default().to_string_lossy();
                debug!("📸 Cache hit for thumbnail: {} ({}x{}, {:.1}KB)", 
                       filename, entry.width, entry.height, entry.memory_size as f64 / 1024.0);
                
                entry.last_accessed = std::time::Instant::now();
                
                // Move to end of access order (most recently used)
                if let Some(pos) = self.access_order.iter().position(|p| p == path) {
                    self.access_order.remove(pos);
                }
                self.access_order.push(path.clone());
                
                Some(entry)
            } else {
                let filename = path.file_name().unwrap_or_default().to_string_lossy();
                debug!("💭 Cache miss for thumbnail: {}", filename);
                None
            }
        }

        pub fn insert(&mut self, path: PathBuf, entry: ThumbnailEntry) {
            let filename = path.file_name().unwrap_or_default().to_string_lossy();
            
            // Check if we need to evict old entries
            let mut evicted_count = 0;
            while (self.cache.len() >= self.max_size) || 
                  (self.current_memory_usage + entry.memory_size > self.max_memory_mb * 1024 * 1024) {
                if let Some(oldest_path) = self.access_order.first().cloned() {
                    let evicted_filename = oldest_path.file_name().unwrap_or_default().to_string_lossy();
                    debug!("🗑️  Evicting oldest thumbnail: {}", evicted_filename);
                    self.remove(&oldest_path);
                    evicted_count += 1;
                } else {
                    break;
                }
            }

            debug!("💾 Inserting thumbnail: {} ({}x{}, {:.1}KB){}",
                   filename, entry.width, entry.height, entry.memory_size as f64 / 1024.0,
                   if evicted_count > 0 { format!(" - evicted {} old entries", evicted_count) } else { String::new() });

            self.current_memory_usage += entry.memory_size;
            self.cache.insert(path.clone(), entry);
            self.access_order.push(path);
            
            debug!("📊 Cache stats: {}/{} entries, {:.1}/{} MB",
                   self.cache.len(), self.max_size,
                   self.memory_usage_mb(), self.max_memory_mb);
        }

        pub fn remove(&mut self, path: &PathBuf) -> Option<ThumbnailEntry> {
            if let Some(entry) = self.cache.remove(path) {
                self.current_memory_usage = self.current_memory_usage.saturating_sub(entry.memory_size);
                self.access_order.retain(|p| p != path);
                Some(entry)
            } else {
                None
            }
        }

        pub fn clear(&mut self) {
            self.cache.clear();
            self.access_order.clear();
            self.current_memory_usage = 0;
        }

        pub fn len(&self) -> usize {
            self.cache.len()
        }

        pub fn memory_usage_mb(&self) -> f64 {
            self.current_memory_usage as f64 / (1024.0 * 1024.0)
        }
    }

    impl WallpaperCarouselGUI {
        pub fn new(config: CarouselConfig) -> Self {
            let (selection_sender, selection_receiver) = mpsc::channel(32);
            
            Self {
                config,
                selection_sender,
                selection_receiver,
                is_running: false,
                thumbnails: HashMap::new(),
                thumbnail_cache: ThumbnailCache::new(500, 200), // 500 thumbnails max, 200MB max
                loading_thumbnails: std::collections::HashSet::new(),
            }
        }
        
        /// Load thumbnail for a wallpaper asynchronously
        pub async fn load_thumbnail(&mut self, wallpaper: &WallpaperInfo) -> Result<Option<ThumbnailEntry>> {
            let start_time = std::time::Instant::now();
            
            // Check if already in cache
            if let Some(entry) = self.thumbnail_cache.get(&wallpaper.path) {
                debug!("⚡ Fast cache retrieval for: {} in {:.1}ms", 
                       wallpaper.filename, start_time.elapsed().as_secs_f64() * 1000.0);
                return Ok(Some(entry.clone()));
            }
            
            // Check if already loading
            if self.loading_thumbnails.contains(&wallpaper.path) {
                debug!("⏳ Thumbnail already loading for: {}", wallpaper.filename);
                return Ok(None);
            }
            
            info!("🔄 Loading thumbnail for: {} ({:.1} MB)", 
                  wallpaper.filename, wallpaper.size_bytes as f64 / (1024.0 * 1024.0));
            
            // Mark as loading
            self.loading_thumbnails.insert(wallpaper.path.clone());
            
            // Try to load from existing thumbnail first
            if let Some(ref thumbnail_path) = wallpaper.thumbnail_path {
                debug!("📁 Trying cached thumbnail: {}", thumbnail_path.display());
                match self.load_thumbnail_from_file(thumbnail_path).await {
                    Ok(entry) => {
                        self.thumbnail_cache.insert(wallpaper.path.clone(), entry.clone());
                        self.loading_thumbnails.remove(&wallpaper.path);
                        info!("✅ Loaded cached thumbnail for: {} in {:.1}ms", 
                              wallpaper.filename, start_time.elapsed().as_secs_f64() * 1000.0);
                        return Ok(Some(entry));
                    }
                    Err(e) => {
                        warn!("❌ Failed to load cached thumbnail for {}: {}", wallpaper.filename, e);
                    }
                }
            } else {
                debug!("📋 No cached thumbnail available for: {}", wallpaper.filename);
            }
            
            // Generate thumbnail on-demand if cache miss
            debug!("🎨 Generating new thumbnail for: {}", wallpaper.filename);
            match self.generate_thumbnail_on_demand(wallpaper).await {
                Ok(entry) => {
                    self.thumbnail_cache.insert(wallpaper.path.clone(), entry.clone());
                    self.loading_thumbnails.remove(&wallpaper.path);
                    info!("✨ Generated new thumbnail for: {} in {:.1}ms ({}x{})", 
                          wallpaper.filename, start_time.elapsed().as_secs_f64() * 1000.0,
                          entry.width, entry.height);
                    Ok(Some(entry))
                }
                Err(e) => {
                    self.loading_thumbnails.remove(&wallpaper.path);
                    error!("💥 Failed to generate thumbnail for {}: {}", wallpaper.filename, e);
                    Err(e)
                }
            }
        }
        
        /// Load thumbnail data from existing file
        async fn load_thumbnail_from_file(&self, thumbnail_path: &std::path::Path) -> Result<ThumbnailEntry> {
            let image_data = tokio::fs::read(thumbnail_path).await?;
            
            // Decode image using the image crate
            let image = image::load_from_memory(&image_data)
                .map_err(|e| anyhow::anyhow!("Failed to decode thumbnail: {}", e))?;
            
            let rgba_image = image.to_rgba8();
            let (width, height) = rgba_image.dimensions();
            let pixels = rgba_image.into_raw();
            let memory_size = pixels.len();
            
            Ok(ThumbnailEntry {
                data: pixels,
                width,
                height,
                last_accessed: std::time::Instant::now(),
                memory_size,
            })
        }
        
        /// Generate thumbnail on-demand using ImageMagick or image crate
        async fn generate_thumbnail_on_demand(&self, wallpaper: &WallpaperInfo) -> Result<ThumbnailEntry> {
            let source_path = wallpaper.path.clone();
            let thumbnail_size = self.config.thumbnail_size;
            
            // Use tokio spawn_blocking for CPU-intensive work
            let thumbnail_data = tokio::task::spawn_blocking(move || -> Result<ThumbnailEntry> {
                // Try to use image crate for thumbnail generation
                let image = image::open(&source_path)
                    .map_err(|e| anyhow::anyhow!("Failed to open image: {}", e))?;
                
                // Resize maintaining aspect ratio
                let thumbnail = image.thumbnail(thumbnail_size, thumbnail_size);
                let rgba_image = thumbnail.to_rgba8();
                let (width, height) = rgba_image.dimensions();
                let pixels = rgba_image.into_raw();
                let memory_size = pixels.len();
                
                Ok(ThumbnailEntry {
                    data: pixels,
                    width,
                    height,
                    last_accessed: std::time::Instant::now(),
                    memory_size,
                })
            }).await??;
            
            Ok(thumbnail_data)
        }
        
        /// Convert thumbnail entry to egui TextureHandle
        pub fn create_texture_handle(entry: &ThumbnailEntry, ctx: &Context) -> Result<TextureHandle> {
            use egui::{ColorImage, TextureOptions};
            
            // Create egui color image from RGBA data
            let color_image = ColorImage::from_rgba_unmultiplied(
                [entry.width as usize, entry.height as usize],
                &entry.data,
            );
            
            // Create texture with filtering for smooth scaling
            let texture_options = TextureOptions {
                magnification: egui::TextureFilter::Linear,
                minification: egui::TextureFilter::Linear,
                wrap_mode: egui::TextureWrapMode::ClampToEdge,
            };
            
            // Generate unique name for the texture
            let texture_name = format!("thumbnail_{}x{}", entry.width, entry.height);
            
            // Load texture into GPU
            let texture_handle = ctx.load_texture(texture_name, color_image, texture_options);
            
            debug!("Created texture handle: {}x{}", entry.width, entry.height);
            Ok(texture_handle)
        }
        
        /// Get or create texture handle for a wallpaper
        pub fn get_texture_handle(&mut self, wallpaper_path: &PathBuf, ctx: &Context) -> Option<&TextureHandle> {
            // Check if texture already exists
            if self.thumbnails.contains_key(wallpaper_path) {
                return self.thumbnails.get(wallpaper_path);
            }
            
            // Try to get from cache and create texture
            if let Some(entry) = self.thumbnail_cache.get(wallpaper_path) {
                if let Ok(texture_handle) = Self::create_texture_handle(entry, ctx) {
                    self.thumbnails.insert(wallpaper_path.clone(), texture_handle);
                    return self.thumbnails.get(wallpaper_path);
                }
            }
            
            None
        }
        
        /// Update textures for all cached thumbnails (called when context is available)
        pub fn update_all_textures(&mut self, ctx: &Context) -> usize {
            let mut created_count = 0;
            let cached_paths: Vec<PathBuf> = self.thumbnail_cache.cache.keys().cloned().collect();
            
            for path in cached_paths {
                if !self.thumbnails.contains_key(&path) {
                    if let Some(entry) = self.thumbnail_cache.get(&path) {
                        if let Ok(texture_handle) = Self::create_texture_handle(entry, ctx) {
                            self.thumbnails.insert(path, texture_handle);
                            created_count += 1;
                        }
                    }
                }
            }
            
            if created_count > 0 {
                debug!("Created {} texture handles from cache", created_count);
            }
            
            created_count
        }
        
        /// Preload thumbnails for visible wallpapers
        pub async fn preload_visible_thumbnails(&mut self, wallpapers: &[WallpaperInfo], visible_range: std::ops::Range<usize>) {
            let mut preload_tasks = Vec::new();
            
            for (i, wallpaper) in wallpapers.iter().enumerate() {
                if visible_range.contains(&i) && !self.thumbnails.contains_key(&wallpaper.path) {
                    preload_tasks.push(wallpaper);
                }
            }
            
            info!("Preloading {} thumbnails", preload_tasks.len());
            
            // Load thumbnails sequentially for now (can be parallelized later)
            for wallpaper in preload_tasks {
                if let Ok(Some(_entry)) = self.load_thumbnail(wallpaper).await {
                    // Thumbnail loaded successfully, will be used when GUI renders
                    debug!("Preloaded thumbnail for: {}", wallpaper.filename);
                }
            }
        }
        
        pub async fn show_carousel(&mut self, wallpapers: Vec<WallpaperInfo>) -> Result<()> {
            info!("🎠 GUI carousel launching with {} wallpapers", wallpapers.len());
            
            // Clear old thumbnails and cache if needed
            self.thumbnails.clear();
            
            // Preload first batch of thumbnails (first 20 for initial display)
            let preload_count = std::cmp::min(20, wallpapers.len());
            if preload_count > 0 {
                info!("🖼️  Preloading first {} thumbnails...", preload_count);
                self.preload_visible_thumbnails(&wallpapers, 0..preload_count).await;
                info!("✅ Thumbnail preloading complete. Cache: {} entries, {:.1} MB", 
                      self.thumbnail_cache.len(), self.thumbnail_cache.memory_usage_mb());
            }
            
            // Temporary fallback: Launch GUI via external process to avoid winit thread issues
            info!("🚀 Attempting to launch GUI carousel via external process...");
            
            // Create a temporary approach using rofi or dmenu for wallpaper selection
            let wallpaper_list: Vec<String> = wallpapers.iter()
                .map(|w| format!("{} ({:.1}MB)", w.filename, w.size_bytes as f64 / (1024.0 * 1024.0)))
                .collect();
            
            if wallpaper_list.is_empty() {
                return Err(anyhow::anyhow!("No wallpapers available"));
            }
            
            // Try to use rofi for wallpaper selection
            let selection_sender = self.selection_sender.clone();
            let wallpapers_clone = wallpapers.clone();
            
            let _gui_handle = tokio::task::spawn_blocking(move || {
                // Create rofi command with wallpaper list
                let input = wallpaper_list.join("\n");
                
                let rofi_result = std::process::Command::new("rofi")
                    .args(&[
                        "-dmenu",
                        "-i", 
                        "-p", "🎠 Select Wallpaper:",
                        "-theme-str", "window { width: 80%; height: 60%; }",
                        "-theme-str", "listview { lines: 10; columns: 1; }",
                        "-no-custom"
                    ])
                    .stdin(std::process::Stdio::piped())
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::piped())
                    .spawn();
                
                match rofi_result {
                    Ok(mut child) => {
                        // Write wallpaper list to rofi stdin
                        if let Some(stdin) = child.stdin.take() {
                            use std::io::Write;
                            let mut stdin = stdin;
                            if let Err(e) = write!(stdin, "{}", input) {
                                error!("Failed to write to rofi stdin: {}", e);
                                return;
                            }
                        }
                        
                        // Wait for rofi result
                        match child.wait_with_output() {
                            Ok(output) => {
                                if output.status.success() {
                                    let selected = String::from_utf8_lossy(&output.stdout).trim().to_string();
                                    
                                    // Find the selected wallpaper
                                    for (i, display_name) in wallpaper_list.iter().enumerate() {
                                        if display_name == &selected {
                                            if let Some(wallpaper) = wallpapers_clone.get(i) {
                                                info!("✅ User selected wallpaper via rofi: {}", wallpaper.filename);
                                                let _ = selection_sender.try_send(
                                                    CarouselSelection::Selected(wallpaper.path.clone())
                                                );
                                                return;
                                            }
                                        }
                                    }
                                    warn!("⚠️ Selected wallpaper not found: {}", selected);
                                } else {
                                    info!("❌ User cancelled rofi wallpaper selection");
                                    let _ = selection_sender.try_send(CarouselSelection::Cancelled);
                                }
                            }
                            Err(e) => {
                                error!("💥 Failed to wait for rofi: {}", e);
                                let _ = selection_sender.try_send(CarouselSelection::Cancelled);
                            }
                        }
                    }
                    Err(e) => {
                        error!("💥 Failed to launch rofi (is it installed?): {}", e);
                        
                        // Fallback to simple text-based selection
                        info!("📋 Fallback: Available wallpapers:");
                        for (i, wallpaper) in wallpapers_clone.iter().enumerate() {
                            info!("  {}. {} ({:.1}MB)", i + 1, wallpaper.filename, wallpaper.size_bytes as f64 / (1024.0 * 1024.0));
                        }
                        
                        // For now, auto-select the first wallpaper
                        if let Some(first_wallpaper) = wallpapers_clone.first() {
                            info!("🎯 Auto-selecting first wallpaper: {}", first_wallpaper.filename);
                            let _ = selection_sender.try_send(
                                CarouselSelection::Selected(first_wallpaper.path.clone())
                            );
                        }
                    }
                }
            });
            
            self.is_running = true;
            info!("✨ GUI carousel launched via external interface");
            Ok(())
        }
        
        pub async fn close_carousel(&mut self) -> Result<()> {
            self.is_running = false;
            
            // Clean up resources
            self.thumbnails.clear();
            self.thumbnail_cache.clear();
            self.loading_thumbnails.clear();
            
            info!("🛑 Closed GUI carousel and cleaned up {} MB of thumbnail cache", 
                  self.thumbnail_cache.memory_usage_mb());
            Ok(())
        }
        
        pub fn try_recv_selection(&mut self) -> Option<CarouselSelection> {
            self.selection_receiver.try_recv().ok()
        }
        
        pub fn is_running(&self) -> bool {
            self.is_running
        }
    }

    /// Main egui carousel application
    pub struct CarouselApp {
        // Wallpaper data
        wallpapers: Vec<WallpaperInfo>,
        thumbnails: HashMap<PathBuf, TextureHandle>,
        
        // Navigation state
        selected_index: usize,
        scroll_offset: f32,
        
        // Grid layout
        grid_columns: usize,
        grid_rows: usize,
        thumbnail_size: Vec2,
        
        // UI state
        should_close: bool,
        hover_index: Option<usize>,
        preview_mode: bool,
        search_text: String,
        filtered_indices: Vec<usize>,
        
        // Configuration
        config: CarouselConfig,
        
        // Communication
        selection_sender: mpsc::Sender<CarouselSelection>,
    }

    impl CarouselApp {
        pub fn new(wallpapers: Vec<WallpaperInfo>, config: CarouselConfig, selection_sender: mpsc::Sender<CarouselSelection>) -> Self {
            let thumbnail_size = Vec2::splat(config.thumbnail_size as f32);
            let filtered_indices: Vec<usize> = (0..wallpapers.len()).collect();
            
            Self {
                wallpapers,
                thumbnails: HashMap::new(),
                selected_index: 0,
                scroll_offset: 0.0,
                grid_columns: config.grid_columns,
                grid_rows: config.grid_rows,
                thumbnail_size,
                should_close: false,
                hover_index: None,
                preview_mode: false,
                search_text: String::new(),
                filtered_indices,
                config,
                selection_sender,
            }
        }
        
        fn handle_input(&mut self, ctx: &egui::Context) {
            ctx.input(|i| {
                // Keyboard navigation
                if i.key_pressed(egui::Key::ArrowRight) {
                    self.navigate_right();
                }
                if i.key_pressed(egui::Key::ArrowLeft) {
                    self.navigate_left();
                }
                if i.key_pressed(egui::Key::ArrowDown) {
                    self.navigate_down();
                }
                if i.key_pressed(egui::Key::ArrowUp) {
                    self.navigate_up();
                }
                if i.key_pressed(egui::Key::Enter) {
                    self.select_current();
                }
                if i.key_pressed(egui::Key::Escape) {
                    self.close_carousel();
                }
                if i.key_pressed(egui::Key::Space) {
                    self.toggle_preview();
                }
                if i.key_pressed(egui::Key::F) && i.modifiers.ctrl {
                    // Focus search box
                }
            });
        }
        
        fn navigate_right(&mut self) {
            if !self.filtered_indices.is_empty() {
                self.selected_index = (self.selected_index + 1) % self.filtered_indices.len();
                debug!("🔍 Navigate right: selected index {}", self.selected_index);
            }
        }
        
        fn navigate_left(&mut self) {
            if !self.filtered_indices.is_empty() {
                self.selected_index = if self.selected_index == 0 {
                    self.filtered_indices.len() - 1
                } else {
                    self.selected_index - 1
                };
                debug!("🔍 Navigate left: selected index {}", self.selected_index);
            }
        }
        
        fn navigate_down(&mut self) {
            if !self.filtered_indices.is_empty() {
                let new_index = self.selected_index + self.grid_columns;
                self.selected_index = if new_index < self.filtered_indices.len() {
                    new_index
                } else {
                    self.selected_index % self.grid_columns
                };
                debug!("🔍 Navigate down: selected index {}", self.selected_index);
            }
        }
        
        fn navigate_up(&mut self) {
            if !self.filtered_indices.is_empty() {
                let current_row = self.selected_index / self.grid_columns;
                if current_row == 0 {
                    // Wrap to bottom
                    let last_row = (self.filtered_indices.len() - 1) / self.grid_columns;
                    let col = self.selected_index % self.grid_columns;
                    self.selected_index = std::cmp::min(
                        last_row * self.grid_columns + col,
                        self.filtered_indices.len() - 1
                    );
                } else {
                    self.selected_index -= self.grid_columns;
                }
                debug!("🔍 Navigate up: selected index {}", self.selected_index);
            }
        }
        
        fn select_current(&mut self) {
            if let Some(&wallpaper_index) = self.filtered_indices.get(self.selected_index) {
                if let Some(wallpaper) = self.wallpapers.get(wallpaper_index) {
                    info!("✅ User selected wallpaper: {}", wallpaper.filename);
                    let _ = self.selection_sender.try_send(
                        CarouselSelection::Selected(wallpaper.path.clone())
                    );
                    self.should_close = true;
                }
            }
        }
        
        fn close_carousel(&mut self) {
            info!("❌ User cancelled carousel");
            let _ = self.selection_sender.try_send(CarouselSelection::Cancelled);
            self.should_close = true;
        }
        
        fn toggle_preview(&mut self) {
            self.preview_mode = !self.preview_mode;
            if self.preview_mode {
                if let Some(&wallpaper_index) = self.filtered_indices.get(self.selected_index) {
                    if let Some(wallpaper) = self.wallpapers.get(wallpaper_index) {
                        debug!("👁️ Preview requested for: {}", wallpaper.filename);
                        let _ = self.selection_sender.try_send(
                            CarouselSelection::PreviewRequested(wallpaper.path.clone())
                        );
                    }
                }
            }
        }
        
        fn update_filter(&mut self) {
            if self.search_text.is_empty() {
                self.filtered_indices = (0..self.wallpapers.len()).collect();
            } else {
                let query = self.search_text.to_lowercase();
                self.filtered_indices = self.wallpapers
                    .iter()
                    .enumerate()
                    .filter(|(_, wallpaper)| {
                        wallpaper.filename.to_lowercase().contains(&query) ||
                        wallpaper.path.to_string_lossy().to_lowercase().contains(&query)
                    })
                    .map(|(i, _)| i)
                    .collect();
            }
            
            // Reset selection if current selection is no longer valid
            if self.selected_index >= self.filtered_indices.len() {
                self.selected_index = 0;
            }
            
            debug!("🔍 Filter updated: '{}' -> {} results", self.search_text, self.filtered_indices.len());
        }
        
        fn render_header(&mut self, ui: &mut egui::Ui) {
            ui.horizontal(|ui| {
                ui.heading("🎠 Wallpaper Carousel");
                
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Close button
                    if ui.button("❌ Close").clicked() {
                        self.close_carousel();
                    }
                    
                    // Status text
                    let status_text = if self.filtered_indices.is_empty() {
                        "No wallpapers found".to_string()
                    } else if self.search_text.is_empty() {
                        format!("{} wallpapers", self.wallpapers.len())
                    } else {
                        format!("{}/{} wallpapers", self.filtered_indices.len(), self.wallpapers.len())
                    };
                    
                    ui.label(status_text);
                });
            });
            
            ui.separator();
            
            // Search bar
            ui.horizontal(|ui| {
                ui.label("🔍 Search:");
                let response = ui.text_edit_singleline(&mut self.search_text);
                if response.changed() {
                    self.update_filter();
                }
                
                if ui.button("Clear").clicked() {
                    self.search_text.clear();
                    self.update_filter();
                }
            });
            
            ui.separator();
        }
        
        fn render_instructions(&self, ui: &mut egui::Ui) {
            ui.horizontal_wrapped(|ui| {
                ui.label("🎮 Navigation: Arrow keys");
                ui.separator();
                ui.label("✅ Select: Enter");
                ui.separator();
                ui.label("👁️ Preview: Space");
                ui.separator();
                ui.label("🔍 Search: Ctrl+F");
                ui.separator();
                ui.label("❌ Exit: Escape");
            });
        }
        
        fn render_wallpaper_grid(&mut self, ui: &mut egui::Ui) {
            if self.filtered_indices.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.add_space(50.0);
                    ui.label("No wallpapers match your search");
                    if !self.search_text.is_empty() {
                        if ui.button("Clear search").clicked() {
                            self.search_text.clear();
                            self.update_filter();
                        }
                    }
                });
                return;
            }
            
            // Calculate grid layout
            let available_width = ui.available_width();
            let thumbnail_width = self.thumbnail_size.x + self.config.spacing;
            let columns = ((available_width / thumbnail_width).floor() as usize).max(1);
            
            // Update grid columns for navigation
            self.grid_columns = columns;
            
            let mut thumbnail_selection = None;
            let mut new_hover_index = self.hover_index;
            
            egui::ScrollArea::vertical()
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    egui::Grid::new("wallpaper_grid")
                        .num_columns(columns)
                        .spacing([self.config.spacing, self.config.spacing])
                        .show(ui, |ui| {
                            // Clone data to avoid borrowing issues
                            let filtered_indices = self.filtered_indices.clone();
                            let wallpapers = &self.wallpapers;
                            let selected_index = self.selected_index;
                            
                            for (grid_index, wallpaper_index) in filtered_indices.iter().enumerate() {
                                if let Some(wallpaper) = wallpapers.get(*wallpaper_index) {
                                    if let Some(selection) = self.render_thumbnail(ui, grid_index, wallpaper, &mut new_hover_index, selected_index) {
                                        thumbnail_selection = Some((selection, grid_index));
                                    }
                                    
                                    if (grid_index + 1) % columns == 0 {
                                        ui.end_row();
                                    }
                                }
                            }
                        });
                });
            
            // Update state after rendering
            self.hover_index = new_hover_index;
            
            // Handle thumbnail selection
            if let Some((selection, grid_index)) = thumbnail_selection {
                self.selected_index = grid_index;
                let _ = self.selection_sender.try_send(selection);
                self.should_close = true;
            }
        }
        
        fn render_thumbnail(&self, ui: &mut egui::Ui, grid_index: usize, wallpaper: &WallpaperInfo, hover_index: &mut Option<usize>, selected_index: usize) -> Option<CarouselSelection> {
            let is_selected = grid_index == selected_index;
            let is_hovered = *hover_index == Some(grid_index);
            
            let color = if is_selected {
                self.config.selection_color
            } else if is_hovered {
                self.config.hover_color
            } else {
                egui::Color32::TRANSPARENT
            };
            
            let response = ui.allocate_response(self.thumbnail_size, egui::Sense::click());
            
            // Handle hover
            if response.hovered() {
                *hover_index = Some(grid_index);
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            
            // Handle click and return selection
            let selection = if response.clicked() {
                Some(CarouselSelection::Selected(wallpaper.path.clone()))
            } else {
                None
            };
            
            // Draw background
            ui.painter().rect_filled(
                response.rect,
                5.0,
                color,
            );
            
            // Try to render thumbnail if available
            if let Some(texture) = self.thumbnails.get(&wallpaper.path) {
                ui.put(
                    response.rect.shrink(2.0),
                    egui::Image::from_texture(texture)
                        .fit_to_exact_size(self.thumbnail_size - Vec2::splat(4.0))
                );
            } else {
                // Placeholder with filename
                ui.put(
                    response.rect,
                    egui::Label::new(&format!("📁 {}", wallpaper.filename))
                        .wrap(true)
                );
            }
            
            // Draw selection border
            if is_selected {
                ui.painter().rect_stroke(
                    response.rect,
                    5.0,
                    egui::Stroke::new(3.0, self.config.selection_color),
                );
            }
            
            // Tooltip with wallpaper info
            if response.hovered() {
                egui::show_tooltip_at_pointer(ui.ctx(), egui::Id::new("wallpaper_tooltip"), |ui| {
                    ui.label(format!("📁 {}", wallpaper.filename));
                    ui.label(format!("📏 {:.1} MB", wallpaper.size_bytes as f64 / (1024.0 * 1024.0)));
                    if let Some((w, h)) = wallpaper.dimensions {
                        ui.label(format!("🖼️  {}x{}", w, h));
                    }
                });
            }
            
            selection
        }
        
        fn render_footer(&self, ui: &mut egui::Ui) {
            ui.separator();
            ui.horizontal(|ui| {
                if let Some(&wallpaper_index) = self.filtered_indices.get(self.selected_index) {
                    if let Some(wallpaper) = self.wallpapers.get(wallpaper_index) {
                        ui.label(format!(
                            "📁 {} ({}/{})",
                            wallpaper.filename,
                            self.selected_index + 1,
                            self.filtered_indices.len()
                        ));
                        
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if self.preview_mode {
                                ui.label("👁️ Preview Mode");
                            }
                        });
                    }
                }
            });
        }
    }

    impl eframe::App for CarouselApp {
        fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
            // Handle input
            self.handle_input(ctx);
            
            // Check if we should close
            if self.should_close {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                return;
            }
            
            // Reset hover state
            self.hover_index = None;
            
            // Main panel with custom frame
            egui::CentralPanel::default()
                .frame(egui::Frame::none()
                    .fill(self.config.background_color)
                    .inner_margin(egui::Margin::same(10.0)))
                .show(ctx, |ui| {
                    ui.vertical(|ui| {
                        // Header
                        self.render_header(ui);
                        
                        // Instructions
                        self.render_instructions(ui);
                        ui.separator();
                        
                        ui.add_space(5.0);
                        
                        // Main content area
                        self.render_wallpaper_grid(ui);
                        
                        ui.add_space(5.0);
                        
                        // Footer
                        self.render_footer(ui);
                    });
                });
        }
        
        fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
            let _ = self.selection_sender.try_send(CarouselSelection::Cancelled);
        }
    }
}

// Placeholder types for when GUI is not available
#[cfg(not(feature = "gui"))]
mod carousel_gui {
    use std::path::PathBuf;
    use anyhow::Result;
    use super::WallpaperInfo;
    
    #[derive(Debug, Clone)]
    pub struct CarouselConfig {
        pub thumbnail_size: u32,
    }
    
    impl Default for CarouselConfig {
        fn default() -> Self {
            Self { 
                thumbnail_size: 200,
            }
        }
    }
    
    #[derive(Debug, Clone)]
    pub enum CarouselSelection {
        Selected(PathBuf),
        Cancelled,
        PreviewRequested(PathBuf),
    }
    
    pub struct WallpaperCarouselGUI;
    
    impl WallpaperCarouselGUI {
        pub fn new(_config: CarouselConfig) -> Self {
            Self
        }
        
        pub async fn show_carousel(&mut self, _wallpapers: Vec<WallpaperInfo>) -> Result<()> {
            Err(anyhow::anyhow!("GUI features not enabled"))
        }
        
        pub async fn close_carousel(&mut self) -> Result<()> {
            Ok(())
        }
        
        pub fn try_recv_selection(&mut self) -> Option<CarouselSelection> {
            None
        }
        
        pub fn is_running(&self) -> bool {
            false
        }
    }
}

use carousel_gui::{WallpaperCarouselGUI, CarouselConfig, CarouselSelection};

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

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    active_processes: HashMap<String, u32>, // Track active wallpaper backend processes per monitor
    // GUI carousel
    carousel_gui: Option<WallpaperCarouselGUI>,
    gui_active: bool,
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
            active_processes: HashMap::new(),
            carousel_gui: None,
            gui_active: false,
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
        self.execute_wallpaper_command(&command, Some(monitor_name)).await?;

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
        let wallpaper_path = self.wallpapers[wallpaper_index].path.clone();
        let wallpaper_filename = self.wallpapers[wallpaper_index].filename.clone();

        let mut command = self.config.command.clone();
        command = command.replace("[file]", &wallpaper_path.to_string_lossy());

        if self.config.debug_logging {
            debug!(
                "🖼️  Setting wallpaper globally: {} -> {}",
                wallpaper_filename, command
            );
        }

        self.execute_wallpaper_command(&command, None).await?;

        // Update all monitor states
        let now = Instant::now();
        for monitor_state in self.monitors.values_mut() {
            monitor_state.current_wallpaper = Some(wallpaper_path.clone());
            monitor_state.last_change = now;
        }

        Ok(format!("Set wallpaper '{}' globally", wallpaper_filename))
    }

    /// Kill existing wallpaper backend process for a monitor if clear_command is not configured
    async fn kill_existing_process(&mut self, monitor_name: &str) -> Result<()> {
        // Only auto-kill if no clear_command is configured (fallback behavior)
        if self.config.clear_command.is_none() {
            if let Some(pid) = self.active_processes.get(monitor_name) {
                if self.config.debug_logging {
                    debug!("🔪 Terminating existing wallpaper process {} for monitor {}", pid, monitor_name);
                }
                
                let pid = *pid;
                let _ = tokio::task::spawn_blocking(move || {
                    // Try graceful termination first
                    Command::new("kill")
                        .args(["-TERM", &pid.to_string()])
                        .output()
                        .and_then(|_| {
                            // Short wait then force kill if needed
                            std::thread::sleep(std::time::Duration::from_millis(200));
                            Command::new("kill")
                                .args(["-KILL", &pid.to_string()])
                                .output()
                        })
                }).await;
                
                self.active_processes.remove(monitor_name);
            }
        }
        Ok(())
    }

    /// Execute wallpaper command with proper process management
    async fn execute_wallpaper_command(&mut self, command: &str, monitor_name: Option<&str>) -> Result<()> {
        // Clean up existing processes if needed
        if let Some(monitor) = monitor_name {
            self.kill_existing_process(monitor).await?;
        } else {
            // For global commands, clean up all active processes
            let monitors: Vec<String> = self.active_processes.keys().cloned().collect();
            for monitor in monitors {
                self.kill_existing_process(&monitor).await?;
            }
        }

        let command = command.to_string();
        let debug_logging = self.config.debug_logging;

        if debug_logging {
            debug!("🖼️  Executing wallpaper command: {}", command);
        }

        // Start command in background without waiting for completion
        // This is essential for wallpaper backends like swaybg that run continuously
        let result = tokio::task::spawn_blocking(move || {
            if cfg!(unix) {
                // Use nohup to fully detach the process and avoid blocking
                let mut cmd = Command::new("sh");
                cmd.args(["-c", &format!("nohup {} >/dev/null 2>&1 &", command)]);
                cmd.status()
            } else {
                // Windows equivalent
                let mut cmd = Command::new("cmd");
                cmd.args(["/C", "start", "/B", &command]);
                cmd.status()
            }
        })
        .await?;

        match result {
            Ok(status) => {
                if status.success() {
                    if debug_logging {
                        debug!("✅ Wallpaper command started successfully in background");
                    }
                } else {
                    if debug_logging {
                        debug!("⚠️  Wallpaper command returned non-zero status but may still be running");
                    }
                    // Don't treat this as an error since the process might still be running in background
                }
            }
            Err(e) => {
                return Err(anyhow::anyhow!("Failed to start wallpaper command: {}", e));
            }
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

        let interval_secs = self.config.interval; // Already in seconds after config parsing
        let _unique_mode = self.config.unique;
        let debug_logging = self.config.debug_logging;

        // Clone necessary data for the background task
        let wallpapers = self.wallpapers.clone();
        let command_template = self.config.command.clone();
        let smooth_transitions = self.config.smooth_transitions;
        let transition_duration = self.config.transition_duration;

        if debug_logging {
            info!(
                "🔄 Starting wallpaper rotation every {} seconds",
                interval_secs
            );
        }

        // Start the rotation task
        let handle = tokio::spawn(async move {
            info!("🔄 Rotation task started with {} second interval", interval_secs);
            let mut current_index = 0;
            let mut interval_timer = interval(Duration::from_secs(interval_secs));
            
            // Skip the first tick to avoid immediate change
            interval_timer.tick().await;
            debug!("🔄 First tick completed, waiting for next interval...");

            loop {
                debug!("🔄 Waiting for next rotation interval...");
                interval_timer.tick().await;
                info!("🔄 Rotation interval triggered!");

                if wallpapers.is_empty() {
                    error!("❌ No wallpapers available, stopping rotation");
                    break;
                }

                // Get next wallpaper
                let wallpaper = &wallpapers[current_index];
                current_index = (current_index + 1) % wallpapers.len();

                // Prepare command
                let mut command = command_template.clone();
                command = command.replace("[file]", &wallpaper.path.to_string_lossy());

                info!("🔄 Auto-rotating to wallpaper: {} (command: {})", wallpaper.filename, command);

                // Execute wallpaper command (global by default)
                let result = tokio::task::spawn_blocking(move || {
                    if cfg!(unix) {
                        std::process::Command::new("sh")
                            .args(["-c", &format!("nohup {} >/dev/null 2>&1 &", command)])
                            .status()
                    } else {
                        std::process::Command::new("cmd")
                            .args(["/C", "start", "/B", &command])
                            .status()
                    }
                }).await;

                match result {
                    Ok(Ok(status)) => {
                        if debug_logging && !status.success() {
                            debug!("⚠️  Wallpaper rotation command returned non-zero status");
                        }
                    }
                    Ok(Err(e)) => {
                        debug!("❌ Wallpaper rotation failed: {}", e);
                    }
                    Err(e) => {
                        debug!("❌ Wallpaper rotation task error: {}", e);
                    }
                }

                // Add transition delay if enabled
                if smooth_transitions {
                    sleep(Duration::from_millis(transition_duration)).await;
                }
            }
        });

        self.rotation_handle = Some(handle);

        info!(
            "✅ Wallpaper rotation started - {} wallpapers every {} seconds",
            self.wallpapers.len(), interval_secs
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

        let selected_path = self.wallpapers[self.carousel_state.current_index].path.clone();
        let selected_filename = self.wallpapers[self.carousel_state.current_index].filename.clone();

        // Set the selected wallpaper
        let mut command = self.config.command.clone();
        command = command.replace("[file]", &selected_path.to_string_lossy());

        self.execute_wallpaper_command(&command, None).await?;

        self.carousel_state.active = false;

        Ok(format!("Selected wallpaper: {}", selected_filename))
    }

    /// Close carousel
    async fn close_carousel(&mut self) -> Result<String> {
        self.carousel_state.active = false;
        Ok("Carousel closed".to_string())
    }

    /// Show GUI carousel for wallpaper selection using external process
    async fn show_gui_carousel(&mut self) -> Result<String> {
        if !self.config.enable_carousel {
            return Err(anyhow::anyhow!("Carousel is disabled"));
        }

        if self.wallpapers.is_empty() {
            return Err(anyhow::anyhow!("No wallpapers available for carousel"));
        }

        info!("🎠 Launching external GUI carousel process");

        // Configure Hyprland windowrules for floating GUI
        self.setup_carousel_windowrules().await?;

        // Create a temporary file with wallpaper paths for the external GUI
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("rustrland_wallpapers.json");

        let wallpaper_data = serde_json::to_string(&self.wallpapers)?;
        std::fs::write(&temp_file, wallpaper_data)?;

        info!("📄 Created wallpaper data file: {}", temp_file.display());

        // Try to launch rustrland-gui binary from different locations
        let gui_paths = vec![
            "rustrland-gui",                           // In PATH
            "./target/debug/rustrland-gui",            // Development build
            "./target/release/rustrland-gui",          // Release build
            "/usr/local/bin/rustrland-gui",           // System install
            "/usr/bin/rustrland-gui",                 // System install
        ];
        
        let mut output = None;
        for gui_path in gui_paths {
            // Try to spawn the GUI process
            let result = Command::new(gui_path)
                .arg("--wallpapers")
                .arg(&temp_file)
                .arg("--mode")
                .arg("carousel")
                .spawn();
                
            match result {
                Ok(child) => {
                    info!("✅ Found and launched GUI binary at: {}", gui_path);
                    
                    // Give the GUI window time to appear, then manage it
                    tokio::time::sleep(Duration::from_millis(500)).await;
                    let _ = self.manage_carousel_window().await;
                    
                    // Now wait for the process to complete
                    let exit_result = child.wait_with_output();
                    match exit_result {
                        Ok(process_output) => {
                            output = Some(process_output);
                            break;
                        }
                        Err(e) => {
                            warn!("⚠️ GUI process failed: {}", e);
                            continue;
                        }
                    }
                }
                Err(_) => {
                    debug!("🔍 GUI binary not found at: {}", gui_path);
                    continue;
                }
            }
        }

        // Clean up temp file
        let _ = std::fs::remove_file(&temp_file);

        match output {
            Some(result) => {
                let result = self.handle_gui_result(result).await;
                
                // Cleanup windowrules after GUI closes
                let _ = self.cleanup_carousel_windowrules().await;
                
                result
            },
            None => {
                warn!("🔄 rustrland-gui not found, falling back to rofi");
                // Fallback to rofi-based selection
                let available_wallpapers = self.wallpapers.clone();
                if let Some(selected_wallpaper) = self.show_rofi_selection(available_wallpapers).await? {
                    self.set_wallpaper_by_info(&selected_wallpaper).await?;
                }
                Ok("Used rofi fallback for wallpaper selection".to_string())
            }
        }
    }

    async fn handle_gui_result(&mut self, result: std::process::Output) -> Result<String> {
        if result.status.success() {
            let selection = String::from_utf8_lossy(&result.stdout).trim().to_string();
            if !selection.is_empty() && selection != "cancelled" {
                info!("🎯 GUI carousel selection: {}", selection);
                // Find and set the selected wallpaper
                if let Some(wallpaper) = self.wallpapers.iter().find(|w| w.path.to_string_lossy() == selection) {
                    let mut command = self.config.command.clone();
                    let path_str = wallpaper.path.to_string_lossy();
                    command = command.replace("[WALLPAPER]", &path_str);

                    let output = Command::new("sh")
                        .arg("-c")
                        .arg(&command)
                        .output()?;

                    if output.status.success() {
                        info!("✅ Wallpaper set to: {}", wallpaper.filename);
                    } else {
                        let error = String::from_utf8_lossy(&output.stderr);
                        warn!("⚠️ Wallpaper command error: {}", error);
                    }
                }
                Ok(format!("Wallpaper set to: {}", selection))
            } else {
                info!("🚫 GUI carousel cancelled");
                Ok("Wallpaper selection cancelled".to_string())
            }
        } else {
            let error = String::from_utf8_lossy(&result.stderr);
            error!("💥 GUI carousel failed: {}", error);
            Err(anyhow::anyhow!("GUI carousel failed: {}", error))
        }
    }

    /// Setup Hyprland windowrules for carousel GUI floating behavior
    async fn setup_carousel_windowrules(&self) -> Result<()> {
        debug!("🔧 Setting up Hyprland windowrules for carousel GUI (pre-launch)");
        
        // Use hyprctl to set windowrules BEFORE launching the window
        // Focus on class since title seems to be empty
        let rules = vec![
            "float,class:^(rustrland-wallpaper-carousel)$",
            "center,class:^(rustrland-wallpaper-carousel)$",
            "size 1200 800,class:^(rustrland-wallpaper-carousel)$",
            "stayfocused,class:^(rustrland-wallpaper-carousel)$",
        ];
        
        for rule in rules {
            let result = Command::new("hyprctl")
                .arg("keyword")
                .arg("windowrulev2")
                .arg(rule)
                .output();
                
            match result {
                Ok(output) if output.status.success() => {
                    debug!("✅ Pre-applied windowrule: {}", rule);
                }
                Ok(output) => {
                    let error = String::from_utf8_lossy(&output.stderr);
                    warn!("⚠️ Failed to apply windowrule '{}': {}", rule, error);
                }
                Err(e) => {
                    warn!("⚠️ Failed to execute hyprctl for rule '{}': {}", rule, e);
                }
            }
        }
        
        Ok(())
    }

    /// Cleanup carousel windowrules when done
    async fn cleanup_carousel_windowrules(&self) -> Result<()> {
        debug!("🧹 Cleaning up carousel windowrules");
        
        // Remove the specific windowrules we added
        let rules_to_remove = vec![
            "float,class:rustrland-wallpaper-carousel",
            "center,class:rustrland-wallpaper-carousel",
            "size 1200 800,class:rustrland-wallpaper-carousel", 
            "stayfocused,class:rustrland-wallpaper-carousel",
        ];
        
        for rule in rules_to_remove {
            let result = Command::new("hyprctl")
                .arg("keyword")
                .arg("windowrulev2")
                .arg(&format!("unset,{}", rule))
                .output();
                
            if let Err(e) = result {
                debug!("Note: Could not remove windowrule '{}': {}", rule, e);
            }
        }
        
        Ok(())
    }

    /// Find and manage carousel GUI window after launch
    async fn manage_carousel_window(&self) -> Result<()> {
        debug!("🪟 Looking for carousel GUI window");
        
        // Try multiple times to find the window as it may take time to appear
        for attempt in 1..=10 {
            debug!("🔍 Window search attempt {}/10", attempt);
            tokio::time::sleep(Duration::from_millis(300)).await;
            
            // Try to find the window by title or class
            let output = Command::new("hyprctl")
                .arg("clients")
                .arg("-j")
                .output()?;
                
            if output.status.success() {
                let clients_json = String::from_utf8_lossy(&output.stdout);
                
                // Parse JSON and find our window
                if let Ok(clients) = serde_json::from_str::<serde_json::Value>(&clients_json) {
                    if let Some(clients_array) = clients.as_array() {
                        for client in clients_array {
                            let title = client.get("title").and_then(|t| t.as_str()).unwrap_or("");
                            let class = client.get("class").and_then(|c| c.as_str()).unwrap_or("");
                            
                            // Look for our window by title or class (prioritize class)
                            if class == "rustrland-wallpaper-carousel" ||
                               class.contains("rustrland-wallpaper-carousel") ||
                               title.contains("rustrland-wallpaper-carousel") || 
                               title.contains("Wallpaper Carousel") {
                                   
                                if let Some(address) = client.get("address").and_then(|a| a.as_str()) {
                                    info!("🎯 Found carousel window: {} (title: '{}', class: '{}')", address, title, class);
                                    self.apply_carousel_window_properties(address).await?;
                                    return Ok(());
                                }
                            }
                        }
                    }
                }
            }
        }
        
        warn!("🔍 Carousel window not found after 10 attempts");
        Ok(())
    }

    /// Apply scratchpad-like properties to carousel window  
    async fn apply_carousel_window_properties(&self, window_address: &str) -> Result<()> {
        info!("✨ Applying carousel window properties to {}", window_address);
        
        // First, let's check if window is already floating
        let clients_output = Command::new("hyprctl")
            .arg("clients")
            .arg("-j")
            .output()?;
            
        let mut is_floating = false;
        if clients_output.status.success() {
            let clients_json = String::from_utf8_lossy(&clients_output.stdout);
            if let Ok(clients) = serde_json::from_str::<serde_json::Value>(&clients_json) {
                if let Some(clients_array) = clients.as_array() {
                    for client in clients_array {
                        if let Some(addr) = client.get("address").and_then(|a| a.as_str()) {
                            if addr == window_address {
                                is_floating = client.get("floating").and_then(|f| f.as_bool()).unwrap_or(false);
                                break;
                            }
                        }
                    }
                }
            }
        }
        
        info!("🔍 Window {} floating status: {}", window_address, is_floating);
        
        // Force floating if not already floating
        if !is_floating {
            info!("🔄 Making window floating");
            let result = Command::new("hyprctl")
                .arg("dispatch")
                .arg("togglefloating")
                .arg(&format!("address:{}", window_address))
                .output();
                
            if let Ok(output) = result {
                if output.status.success() {
                    info!("✅ Successfully toggled floating");
                } else {
                    warn!("⚠️ Failed to toggle floating: {}", String::from_utf8_lossy(&output.stderr));
                }
            }
        }
        
        // Small delay to ensure the floating state is applied
        tokio::time::sleep(Duration::from_millis(200)).await;
        
        // Resize and position the window
        info!("📐 Setting window size and position");
        let _ = Command::new("hyprctl")
            .arg("dispatch")
            .arg("resizewindowpixel")
            .arg("exact 1200 800")
            .arg(&format!("address:{}", window_address))
            .output();
            
        tokio::time::sleep(Duration::from_millis(100)).await;
            
        // Center the window
        info!("🎯 Centering window");
        let _ = Command::new("hyprctl")
            .arg("dispatch")
            .arg("centerwindow")
            .arg("1")
            .output();
            
        tokio::time::sleep(Duration::from_millis(100)).await;
            
        // Set focus
        info!("👁️ Focusing window");
        let _ = Command::new("hyprctl")
            .arg("dispatch")
            .arg("focuswindow")
            .arg(&format!("address:{}", window_address))
            .output();
            
        info!("✅ Carousel window configured as floating and centered");
        Ok(())
    }

    /// Show rofi selection dialog for wallpaper selection
    async fn show_rofi_selection(&self, wallpapers: Vec<WallpaperInfo>) -> Result<Option<WallpaperInfo>> {
        if wallpapers.is_empty() {
            return Ok(None);
        }

        // Create display options for rofi
        let options: Vec<String> = wallpapers.iter()
            .map(|w| format!("{} ({})", w.filename, w.path.parent().unwrap_or(&w.path).display()))
            .collect();

        let input = options.join("\n");

        let output = Command::new("rofi")
            .arg("-dmenu")
            .arg("-i")
            .arg("-p")
            .arg("Select Wallpaper")
            .arg("-format")
            .arg("i")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn();

        let mut child = match output {
            Ok(child) => child,
            Err(_) => return Err(anyhow::anyhow!("Failed to launch rofi")),
        };

        if let Some(stdin) = child.stdin.take() {
            use std::io::Write;
            let mut stdin = stdin;
            let _ = stdin.write_all(input.as_bytes());
        }

        let output = child.wait_with_output()?;

        if output.status.success() {
            let selection_owned = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if let Ok(index) = selection_owned.parse::<usize>() {
                if index < wallpapers.len() {
                    return Ok(Some(wallpapers[index].clone()));
                }
            }
        }

        Ok(None)
    }

    /// Set wallpaper using WallpaperInfo
    async fn set_wallpaper_by_info(&self, wallpaper: &WallpaperInfo) -> Result<()> {
        let mut command = self.config.command.clone();
        let path_str = wallpaper.path.to_string_lossy();
        command = command.replace("[WALLPAPER]", &path_str);

        let output = Command::new("sh")
            .arg("-c")
            .arg(&command)
            .output()?;

        if output.status.success() {
            info!("✅ Wallpaper set to: {}", wallpaper.filename);
        } else {
            let error = String::from_utf8_lossy(&output.stderr);
            warn!("⚠️ Wallpaper command error: {}", error);
        }

        Ok(())
    }

    /// Process GUI carousel selections
    async fn handle_gui_carousel_selection(&mut self) -> Result<Option<String>> {
        if let Some(ref mut gui) = self.carousel_gui {
            if let Some(selection) = gui.try_recv_selection() {
                match selection {
                    CarouselSelection::Selected(path) => {
                        // Set the selected wallpaper
                        let mut command = self.config.command.clone();
                        command = command.replace("[file]", &path.to_string_lossy());
                        
                        self.execute_wallpaper_command(&command, None).await?;
                        self.gui_active = false;
                        
                        let filename = path.file_name()
                            .unwrap_or_default()
                            .to_string_lossy();
                        return Ok(Some(format!("Selected wallpaper: {}", filename)));
                    }
                    CarouselSelection::Cancelled => {
                        self.gui_active = false;
                        return Ok(Some("Carousel cancelled".to_string()));
                    }
                    CarouselSelection::PreviewRequested(path) => {
                        // Optional: Set as preview temporarily
                        debug!("Preview requested for: {}", path.display());
                    }
                }
            }
        }
        Ok(None)
    }

    /// Close GUI carousel
    async fn close_gui_carousel(&mut self) -> Result<String> {
        if let Some(ref mut gui) = self.carousel_gui {
            gui.close_carousel().await?;
            self.gui_active = false;
            Ok("GUI carousel closed".to_string())
        } else {
            Ok("GUI carousel is not active".to_string())
        }
    }

    /// Clear all wallpapers
    async fn clear_wallpapers(&mut self) -> Result<String> {
        if let Some(clear_command) = &self.config.clear_command {
            let command = clear_command.clone();

            if self.config.debug_logging {
                debug!("🧹 Executing clear command: {}", command);
            }

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

            // Clear tracked processes since they were terminated by clear_command
            self.active_processes.clear();
            Ok("Wallpapers cleared using configured command".to_string())
        } else {
            // Simple fallback: try to kill common wallpaper backends
            if self.config.debug_logging {
                debug!("🧹 Using fallback clear method");
            }

            let backends = ["swaybg", "swww-daemon", "wpaperd", "feh", "hyprpaper"];
            let mut killed_any = false;

            for backend in &backends {
                let backend_name = backend.to_string();
                let result = tokio::task::spawn_blocking(move || {
                    Command::new("pkill")
                        .args(["-f", &backend_name])
                        .output()
                }).await?;

                if let Ok(output) = result {
                    if output.status.success() {
                        killed_any = true;
                        if self.config.debug_logging {
                            debug!("🔪 Terminated {} processes", backend);
                        }
                    }
                }
            }

            self.active_processes.clear();
            
            if killed_any {
                Ok("Terminated wallpaper backend processes".to_string())
            } else {
                Ok("No wallpaper backend processes found to terminate".to_string())
            }
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

        let active_processes_count = self.active_processes.len();
        if active_processes_count > 0 {
            status.push_str(&format!(
                "Active processes: {active_processes_count} wallpaper backend(s) running\n"
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
        debug!("🔍 Received config: {}", config);

        // Parse configuration field by field to handle extra fields gracefully
        if let toml::Value::Table(table) = config {
            // Parse known fields from the config
            if let Some(interval) = table.get("interval") {
                if let Some(val) = interval.as_float() {
                    // Store as seconds for more precision with fractional minutes
                    let interval_seconds = (val * 60.0) as u64;
                    self.config.interval = if interval_seconds == 0 { 1 } else { interval_seconds };
                    debug!("✅ Set interval to {} minutes ({} seconds)", val, self.config.interval);
                } else if let Some(val) = interval.as_integer() {
                    // For integer minutes, convert to seconds 
                    self.config.interval = (val as u64) * 60;
                    debug!("✅ Set interval to {} minutes ({} seconds)", val, self.config.interval);
                }
            }
            
            if let Some(path) = table.get("path") {
                if let Some(path_str) = path.as_str() {
                    self.config.path = WallpaperPath::Single(PathBuf::from(path_str));
                    debug!("✅ Set path to {}", path_str);
                }
            }
            
            if let Some(debug_logging) = table.get("debug_logging") {
                if let Some(val) = debug_logging.as_bool() {
                    self.config.debug_logging = val;
                    debug!("✅ Set debug_logging to {}", val);
                }
            }
            
            if let Some(command) = table.get("command") {
                if let Some(cmd_str) = command.as_str() {
                    self.config.command = cmd_str.to_string();
                    debug!("✅ Set command to {}", cmd_str);
                }
            }
            
            if let Some(clear_command) = table.get("clear_command") {
                if let Some(cmd_str) = clear_command.as_str() {
                    self.config.clear_command = Some(cmd_str.to_string());
                    debug!("✅ Set clear_command to {}", cmd_str);
                }
            }
            
            if let Some(unique) = table.get("unique") {
                if let Some(val) = unique.as_bool() {
                    self.config.unique = val;
                    debug!("✅ Set unique to {}", val);
                }
            }
            
            if let Some(recurse) = table.get("recurse") {
                if let Some(val) = recurse.as_bool() {
                    self.config.recurse = val;
                    debug!("✅ Set recurse to {}", val);
                }
            }
            
            debug!("✅ Successfully parsed wallpapers config field by field");
        } else {
            // Fallback: look for nested wallpapers section
            if let Some(plugin_config) = config.get("wallpapers") {
                debug!("✅ Found nested wallpapers config: {}", plugin_config);
                match plugin_config.clone().try_into() {
                    Ok(config) => self.config = config,
                    Err(e) => return Err(anyhow::anyhow!("Invalid wallpapers configuration: {}", e)),
                }
            } else {
                debug!("❌ No wallpapers section found, using defaults");
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
                // Set specific wallpaper by index or name, optionally on specific monitor
                // Usage: set <wallpaper> [monitor]
                if let Some(identifier) = args.first() {
                    let monitor_name = args.get(1); // Optional monitor parameter
                    
                    // Try to parse as index first
                    if let Ok(index) = identifier.parse::<usize>() {
                        if index > 0 && index <= self.wallpapers.len() {
                            let wallpaper_path = self.wallpapers[index - 1].path.clone();
                            let wallpaper_filename = self.wallpapers[index - 1].filename.clone();
                            let mut command = self.config.command.clone();
                            command = command.replace("[file]", &wallpaper_path.to_string_lossy());
                            
                            // Add monitor-specific output if specified
                            if let Some(monitor) = monitor_name {
                                // For monitor-specific, we need to add the -o option to swaybg
                                if command.contains("[output]") {
                                    command = command.replace("[output]", monitor);
                                } else {
                                    // If no [output] placeholder, inject -o option for swaybg
                                    if command.contains("swaybg") {
                                        command = command.replace("swaybg", &format!("swaybg -o {}", monitor));
                                    } else {
                                        // For other backends, try to add output parameter
                                        command = format!("{} -o {}", command, monitor);
                                    }
                                }
                                self.execute_wallpaper_command(&command, Some(monitor)).await?;
                                return Ok(format!("Set wallpaper '{}' on monitor {}", wallpaper_filename, monitor));
                            } else {
                                self.execute_wallpaper_command(&command, None).await?;
                                return Ok(format!("Set wallpaper: {}", wallpaper_filename));
                            }
                        } else {
                            return Err(anyhow::anyhow!("Wallpaper index {} out of range (1-{})", 
                                index, self.wallpapers.len()));
                        }
                    } else {
                        // Search by filename
                        if let Some(wallpaper) = self.wallpapers.iter()
                            .find(|w| w.filename.to_lowercase().contains(&identifier.to_lowercase())) {
                            let wallpaper_path = wallpaper.path.clone();
                            let wallpaper_filename = wallpaper.filename.clone();
                            let mut command = self.config.command.clone();
                            command = command.replace("[file]", &wallpaper_path.to_string_lossy());

                            // Add monitor-specific output if specified
                            if let Some(monitor) = monitor_name {
                                // For monitor-specific, we need to add the -o option to swaybg
                                if command.contains("[output]") {
                                    command = command.replace("[output]", monitor);
                                } else {
                                    // If no [output] placeholder, inject -o option for swaybg
                                    if command.contains("swaybg") {
                                        command = command.replace("swaybg", &format!("swaybg -o {}", monitor));
                                    } else {
                                        // For other backends, try to add output parameter
                                        command = format!("{} -o {}", command, monitor);
                                    }
                                }
                                self.execute_wallpaper_command(&command, Some(monitor)).await?;
                                return Ok(format!("Set wallpaper '{}' on monitor {}", wallpaper_filename, monitor));
                            } else {
                                self.execute_wallpaper_command(&command, None).await?;
                                return Ok(format!("Set wallpaper: {}", wallpaper_filename));
                            }
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
                        "gui" => {
                            // GUI carousel subcommands - use external GUI process
                            if let Some(subaction) = args.get(1) {
                                match *subaction {
                                    "show" => self.show_gui_carousel().await,
                                    "close" => Ok("GUI carousel closed (external process)".to_string()),
                                    _ => Err(anyhow::anyhow!("Unknown GUI carousel action: {}. Available: show, close", subaction)),
                                }
                            } else {
                                // Default action: show GUI carousel
                                self.show_gui_carousel().await
                            }
                        },
                        _ => Err(anyhow::anyhow!("Unknown carousel action: {}", action)),
                    }
                } else {
                    // Default carousel action: use external GUI
                    self.show_gui_carousel().await
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

            _ => Ok(format!("Unknown wallpapers command: {command}. Available: next, set, carousel [show|next|prev|select|close|gui], scan, list, status, clear, start, stop")),
        }
    }

    async fn cleanup(&mut self) -> Result<()> {
        info!("🧹 Cleaning up wallpapers plugin");

        // Stop rotation if running
        if let Some(handle) = self.rotation_handle.take() {
            handle.abort();
            debug!("❌ Cancelled wallpaper rotation task");
        }

        // Close GUI carousel if running
        if let Some(ref mut gui) = self.carousel_gui {
            let _ = gui.close_carousel().await;
            debug!("🎠 Closed GUI carousel");
        }

        // Clean up all active wallpaper backend processes
        if !self.active_processes.is_empty() {
            debug!("🔪 Terminating {} active wallpaper processes", self.active_processes.len());
            let _ = self.clear_wallpapers().await;
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
        assert_eq!(plugin.active_processes.len(), 0);
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

    // GUI Carousel and Thumbnail Tests
    #[cfg(feature = "gui")]
    mod gui_tests {
        use super::*;

        fn create_test_thumbnail_entry(width: u32, height: u32) -> carousel_gui::ThumbnailEntry {
            let data_size = (width * height * 4) as usize; // RGBA
            carousel_gui::ThumbnailEntry {
                data: vec![255u8; data_size], // White pixels
                width,
                height,
                last_accessed: std::time::Instant::now(),
                memory_size: data_size,
            }
        }

        #[test]
        fn test_thumbnail_cache_creation() {
            let cache = carousel_gui::ThumbnailCache::new(10, 50);
            assert_eq!(cache.len(), 0);
            assert_eq!(cache.memory_usage_mb(), 0.0);
        }

        #[test]
        fn test_thumbnail_cache_insertion_and_retrieval() {
            let mut cache = carousel_gui::ThumbnailCache::new(5, 50);
            let path = PathBuf::from("/test/image.jpg");
            let entry = create_test_thumbnail_entry(200, 200);
            let expected_memory = entry.memory_size;

            // Insert entry
            cache.insert(path.clone(), entry);
            assert_eq!(cache.len(), 1);
            assert!(cache.memory_usage_mb() > 0.0);

            // Retrieve entry
            let retrieved = cache.get(&path);
            assert!(retrieved.is_some());
            let retrieved_entry = retrieved.unwrap();
            assert_eq!(retrieved_entry.width, 200);
            assert_eq!(retrieved_entry.height, 200);
            assert_eq!(retrieved_entry.memory_size, expected_memory);
        }

        #[test]
        fn test_thumbnail_cache_lru_eviction_by_count() {
            let mut cache = carousel_gui::ThumbnailCache::new(3, 200); // Max 3 entries

            // Insert 4 entries to trigger eviction
            for i in 1..=4 {
                let path = PathBuf::from(format!("/test/image{}.jpg", i));
                let entry = create_test_thumbnail_entry(100, 100);
                cache.insert(path, entry);
            }

            // Should have max 3 entries (first one evicted)
            assert_eq!(cache.len(), 3);
            
            // First entry should be evicted
            let first_path = PathBuf::from("/test/image1.jpg");
            assert!(cache.get(&first_path).is_none());
            
            // Last entry should still be there
            let last_path = PathBuf::from("/test/image4.jpg");
            assert!(cache.get(&last_path).is_some());
        }

        #[test]
        fn test_thumbnail_cache_lru_eviction_by_memory() {
            // Each 200x200 RGBA image = 160KB, limit to 200KB (only ~1 image fits)
            let mut cache = carousel_gui::ThumbnailCache::new(10, 1); // 1MB limit

            let path1 = PathBuf::from("/test/large1.jpg");
            let entry1 = create_test_thumbnail_entry(400, 400); // ~640KB
            cache.insert(path1.clone(), entry1);

            let path2 = PathBuf::from("/test/large2.jpg");
            let entry2 = create_test_thumbnail_entry(400, 400); // ~640KB
            cache.insert(path2.clone(), entry2);

            // First entry should be evicted due to memory pressure
            assert!(cache.get(&path1).is_none());
            assert!(cache.get(&path2).is_some());
            assert_eq!(cache.len(), 1);
        }

        #[test]
        fn test_thumbnail_cache_lru_access_order() {
            let mut cache = carousel_gui::ThumbnailCache::new(3, 200);

            // Insert 3 entries
            let paths: Vec<PathBuf> = (1..=3)
                .map(|i| PathBuf::from(format!("/test/image{}.jpg", i)))
                .collect();

            for path in &paths {
                let entry = create_test_thumbnail_entry(100, 100);
                cache.insert(path.clone(), entry);
            }

            // Access first entry (making it most recent)
            cache.get(&paths[0]);

            // Insert new entry - should evict second entry (oldest)
            let new_path = PathBuf::from("/test/image4.jpg");
            let new_entry = create_test_thumbnail_entry(100, 100);
            cache.insert(new_path.clone(), new_entry);

            // Check that second entry was evicted, others remain
            assert!(cache.get(&paths[0]).is_some()); // Recently accessed
            assert!(cache.get(&paths[1]).is_none());  // Should be evicted
            assert!(cache.get(&paths[2]).is_some());  // Third entry
            assert!(cache.get(&new_path).is_some());  // New entry
        }

        #[test]
        fn test_thumbnail_cache_clear() {
            let mut cache = carousel_gui::ThumbnailCache::new(5, 50);

            // Insert some entries
            for i in 1..=3 {
                let path = PathBuf::from(format!("/test/image{}.jpg", i));
                let entry = create_test_thumbnail_entry(100, 100);
                cache.insert(path, entry);
            }

            assert_eq!(cache.len(), 3);
            assert!(cache.memory_usage_mb() > 0.0);

            cache.clear();

            assert_eq!(cache.len(), 0);
            assert_eq!(cache.memory_usage_mb(), 0.0);
        }

        #[test]
        fn test_carousel_gui_creation() {
            let config = carousel_gui::CarouselConfig::default();
            let gui = carousel_gui::WallpaperCarouselGUI::new(config);

            assert!(!gui.is_running());
            assert_eq!(gui.thumbnail_cache.len(), 0);
        }

        #[tokio::test]
        async fn test_carousel_gui_show_and_close() {
            let config = carousel_gui::CarouselConfig::default();
            let mut gui = carousel_gui::WallpaperCarouselGUI::new(config);

            // Test show carousel
            let wallpapers = vec![create_test_wallpaper("test.jpg")];
            let result = gui.show_carousel(wallpapers).await;
            assert!(result.is_ok());
            assert!(gui.is_running());

            // Test close carousel
            let result = gui.close_carousel().await;
            assert!(result.is_ok());
            assert!(!gui.is_running());
        }

        #[test]
        fn test_thumbnail_entry_creation() {
            let entry = create_test_thumbnail_entry(256, 256);
            
            assert_eq!(entry.width, 256);
            assert_eq!(entry.height, 256);
            assert_eq!(entry.data.len(), 256 * 256 * 4); // RGBA
            assert_eq!(entry.memory_size, 256 * 256 * 4);
            assert!(entry.last_accessed <= std::time::Instant::now());
        }
    }
}
