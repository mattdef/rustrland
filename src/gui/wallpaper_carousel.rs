use egui::{Context, TextureHandle, Vec2, Color32};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::sync::mpsc;
use anyhow::Result;
use tracing::{debug, info, warn, error};

use crate::plugins::wallpapers::WallpaperInfo;

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

/// Commands from plugin to GUI
#[derive(Debug, Clone)]
pub enum CarouselCommand {
    ShowCarousel(Vec<WallpaperInfo>),
    UpdateWallpapers(Vec<WallpaperInfo>),
    HighlightWallpaper(PathBuf),
    CloseCarousel,
}

/// Main GUI carousel application
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
    
    // Configuration
    config: CarouselConfig,
    
    // Communication
    selection_sender: mpsc::Sender<CarouselSelection>,
    command_receiver: mpsc::Receiver<CarouselCommand>,
}

impl CarouselApp {
    pub fn new(
        config: CarouselConfig,
        selection_sender: mpsc::Sender<CarouselSelection>,
        command_receiver: mpsc::Receiver<CarouselCommand>,
    ) -> Self {
        let thumbnail_size = Vec2::splat(config.thumbnail_size as f32);
        
        Self {
            wallpapers: Vec::new(),
            thumbnails: HashMap::new(),
            selected_index: 0,
            scroll_offset: 0.0,
            grid_columns: config.grid_columns,
            grid_rows: config.grid_rows,
            thumbnail_size,
            should_close: false,
            hover_index: None,
            preview_mode: false,
            config,
            selection_sender,
            command_receiver,
        }
    }
    
    fn handle_input(&mut self, ctx: &Context) {
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
                self.should_close = true;
            }
            if i.key_pressed(egui::Key::Space) {
                self.toggle_preview();
            }
        });
    }
    
    fn navigate_right(&mut self) {
        if !self.wallpapers.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.wallpapers.len();
        }
    }
    
    fn navigate_left(&mut self) {
        if !self.wallpapers.is_empty() {
            self.selected_index = if self.selected_index == 0 {
                self.wallpapers.len() - 1
            } else {
                self.selected_index - 1
            };
        }
    }
    
    fn navigate_down(&mut self) {
        if !self.wallpapers.is_empty() {
            let new_index = self.selected_index + self.grid_columns;
            self.selected_index = if new_index < self.wallpapers.len() {
                new_index
            } else {
                self.selected_index % self.grid_columns
            };
        }
    }
    
    fn navigate_up(&mut self) {
        if !self.wallpapers.is_empty() {
            let current_row = self.selected_index / self.grid_columns;
            if current_row == 0 {
                // Wrap to bottom
                let last_row = (self.wallpapers.len() - 1) / self.grid_columns;
                let col = self.selected_index % self.grid_columns;
                self.selected_index = std::cmp::min(
                    last_row * self.grid_columns + col,
                    self.wallpapers.len() - 1
                );
            } else {
                self.selected_index -= self.grid_columns;
            }
        }
    }
    
    fn select_current(&mut self) {
        if let Some(wallpaper) = self.wallpapers.get(self.selected_index) {
            let _ = self.selection_sender.try_send(
                CarouselSelection::Selected(wallpaper.path.clone())
            );
            self.should_close = true;
        }
    }
    
    fn toggle_preview(&mut self) {
        self.preview_mode = !self.preview_mode;
        if self.preview_mode {
            if let Some(wallpaper) = self.wallpapers.get(self.selected_index) {
                let _ = self.selection_sender.try_send(
                    CarouselSelection::PreviewRequested(wallpaper.path.clone())
                );
            }
        }
    }
    
    fn render_wallpaper_grid(&mut self, ui: &mut egui::Ui) {
        let available_width = ui.available_width();
        let thumbnail_width = self.thumbnail_size.x + self.config.spacing;
        let columns = (available_width / thumbnail_width).floor() as usize;
        
        // Clone wallpapers to avoid borrowing issues
        let wallpapers = self.wallpapers.clone();
        
        egui::ScrollArea::vertical()
            .max_height(ui.available_height())
            .show(ui, |ui| {
                egui::Grid::new("wallpaper_grid")
                    .num_columns(columns)
                    .spacing([self.config.spacing, self.config.spacing])
                    .show(ui, |ui| {
                        for (index, wallpaper) in wallpapers.iter().enumerate() {
                            self.render_thumbnail(ui, index, wallpaper);
                            
                            if (index + 1) % columns == 0 {
                                ui.end_row();
                            }
                        }
                    });
            });
    }
    
    fn render_thumbnail(&mut self, ui: &mut egui::Ui, index: usize, wallpaper: &WallpaperInfo) {
        let is_selected = index == self.selected_index;
        let is_hovered = self.hover_index == Some(index);
        
        let color = if is_selected {
            self.config.selection_color
        } else if is_hovered {
            self.config.hover_color
        } else {
            Color32::TRANSPARENT
        };
        
        let response = ui.allocate_response(self.thumbnail_size, egui::Sense::click());
        
        // Handle hover
        if response.hovered() {
            self.hover_index = Some(index);
        }
        
        // Handle click
        if response.clicked() {
            self.selected_index = index;
            let _ = self.selection_sender.try_send(
                CarouselSelection::Selected(wallpaper.path.clone())
            );
            self.should_close = true;
        }
        
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
                egui::Label::new(&wallpaper.filename)
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
    }
    
    fn process_commands(&mut self) {
        while let Ok(command) = self.command_receiver.try_recv() {
            match command {
                CarouselCommand::ShowCarousel(wallpapers) => {
                    debug!("GUI: Showing carousel with {} wallpapers", wallpapers.len());
                    self.wallpapers = wallpapers;
                    self.selected_index = 0;
                }
                CarouselCommand::UpdateWallpapers(wallpapers) => {
                    debug!("GUI: Updating wallpapers ({} items)", wallpapers.len());
                    self.wallpapers = wallpapers;
                    if self.selected_index >= self.wallpapers.len() {
                        self.selected_index = 0;
                    }
                }
                CarouselCommand::HighlightWallpaper(path) => {
                    if let Some(index) = self.wallpapers.iter().position(|w| w.path == path) {
                        self.selected_index = index;
                    }
                }
                CarouselCommand::CloseCarousel => {
                    self.should_close = true;
                }
            }
        }
    }
}

impl eframe::App for CarouselApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        // Process incoming commands
        self.process_commands();
        
        // Handle input
        self.handle_input(ctx);
        
        // Check if we should close
        if self.should_close {
            let _ = self.selection_sender.try_send(CarouselSelection::Cancelled);
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        
        // Main panel
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(self.config.background_color))
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    // Title
                    ui.heading("🎠 Wallpaper Carousel");
                    ui.separator();
                    
                    // Instructions
                    ui.horizontal(|ui| {
                        ui.label("Navigation: Arrow keys | Select: Enter | Preview: Space | Exit: Esc");
                    });
                    
                    ui.add_space(10.0);
                    
                    // Show current selection info
                    if let Some(wallpaper) = self.wallpapers.get(self.selected_index) {
                        ui.label(format!(
                            "Selected: {} ({}/{})",
                            wallpaper.filename,
                            self.selected_index + 1,
                            self.wallpapers.len()
                        ));
                    }
                    
                    ui.add_space(10.0);
                    
                    // Wallpaper grid
                    if !self.wallpapers.is_empty() {
                        self.render_wallpaper_grid(ui);
                    } else {
                        ui.label("No wallpapers available");
                    }
                });
            });
    }
    
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        let _ = self.selection_sender.try_send(CarouselSelection::Cancelled);
    }
}

/// Main wallpaper carousel GUI wrapper
pub struct WallpaperCarouselGUI {
    // Communication channels
    selection_sender: mpsc::Sender<CarouselSelection>,
    selection_receiver: mpsc::Receiver<CarouselSelection>,
    command_sender: mpsc::Sender<CarouselCommand>,
    
    // Configuration
    config: CarouselConfig,
    
    // GUI state
    gui_handle: Option<tokio::task::JoinHandle<Result<()>>>,
    is_running: bool,
}

impl WallpaperCarouselGUI {
    pub fn new(config: CarouselConfig) -> Self {
        let (selection_sender, selection_receiver) = mpsc::channel(32);
        let (command_sender, _) = mpsc::channel(32);
        
        Self {
            selection_sender,
            selection_receiver,
            command_sender,
            config,
            gui_handle: None,
            is_running: false,
        }
    }
    
    pub async fn show_carousel(&mut self, wallpapers: Vec<WallpaperInfo>) -> Result<()> {
        if self.is_running {
            // Update existing carousel
            let _ = self.command_sender.send(CarouselCommand::ShowCarousel(wallpapers)).await;
            return Ok(());
        }
        
        info!("🚀 Launching wallpaper carousel GUI");
        
        let (command_sender, command_receiver) = mpsc::channel(32);
        self.command_sender = command_sender.clone();
        
        let selection_sender = self.selection_sender.clone();
        let config = self.config.clone();
        
        // Send initial wallpapers
        let _ = command_sender.send(CarouselCommand::ShowCarousel(wallpapers)).await;
        
        let handle = tokio::task::spawn_blocking(move || {
            let options = eframe::NativeOptions {
                viewport: egui::ViewportBuilder::default()
                    .with_inner_size(config.window_size)
                    .with_title("Wallpaper Carousel")
                    .with_resizable(true),
                ..Default::default()
            };
            
            let app = CarouselApp::new(config, selection_sender, command_receiver);
            
            eframe::run_native(
                "Wallpaper Carousel",
                options,
                Box::new(|_cc| Box::new(app)),
            ).map_err(|e| anyhow::anyhow!("GUI error: {}", e))
        });
        
        self.gui_handle = Some(handle);
        self.is_running = true;
        
        Ok(())
    }
    
    pub async fn close_carousel(&mut self) -> Result<()> {
        if self.is_running {
            let _ = self.command_sender.send(CarouselCommand::CloseCarousel).await;
            
            if let Some(handle) = self.gui_handle.take() {
                let _ = handle.await;
            }
            
            self.is_running = false;
            info!("🛑 Closed wallpaper carousel GUI");
        }
        
        Ok(())
    }
    
    pub async fn update_wallpapers(&mut self, wallpapers: Vec<WallpaperInfo>) -> Result<()> {
        if self.is_running {
            let _ = self.command_sender.send(CarouselCommand::UpdateWallpapers(wallpapers)).await;
        }
        Ok(())
    }
    
    pub async fn highlight_wallpaper(&mut self, path: PathBuf) -> Result<()> {
        if self.is_running {
            let _ = self.command_sender.send(CarouselCommand::HighlightWallpaper(path)).await;
        }
        Ok(())
    }
    
    pub fn try_recv_selection(&mut self) -> Option<CarouselSelection> {
        self.selection_receiver.try_recv().ok()
    }
    
    pub fn is_running(&self) -> bool {
        self.is_running
    }
}

impl Drop for WallpaperCarouselGUI {
    fn drop(&mut self) {
        if let Some(handle) = self.gui_handle.take() {
            handle.abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;

    fn create_test_wallpaper(filename: &str) -> WallpaperInfo {
        WallpaperInfo {
            path: PathBuf::from(format!("/test/{}", filename)),
            filename: filename.to_string(),
            size_bytes: 1024 * 1024,
            last_modified: SystemTime::now(),
            thumbnail_path: None,
            dimensions: Some((1920, 1080)),
        }
    }

    #[test]
    fn test_carousel_config_default() {
        let config = CarouselConfig::default();
        assert_eq!(config.window_size, Vec2::new(1200.0, 800.0));
        assert_eq!(config.grid_columns, 5);
        assert_eq!(config.grid_rows, 3);
        assert_eq!(config.thumbnail_size, 200);
        assert_eq!(config.spacing, 10.0);
    }

    #[test]
    fn test_carousel_selection_enum() {
        let selection = CarouselSelection::Selected(PathBuf::from("/test/wallpaper.jpg"));
        match selection {
            CarouselSelection::Selected(path) => {
                assert_eq!(path, PathBuf::from("/test/wallpaper.jpg"));
            }
            _ => panic!("Expected Selected variant"),
        }
    }

    #[test]
    fn test_carousel_command_enum() {
        let wallpapers = vec![create_test_wallpaper("test.jpg")];
        let command = CarouselCommand::ShowCarousel(wallpapers.clone());
        
        match command {
            CarouselCommand::ShowCarousel(w) => {
                assert_eq!(w.len(), 1);
                assert_eq!(w[0].filename, "test.jpg");
            }
            _ => panic!("Expected ShowCarousel variant"),
        }
    }

    #[tokio::test]
    async fn test_wallpaper_carousel_gui_creation() {
        let config = CarouselConfig::default();
        let gui = WallpaperCarouselGUI::new(config);
        
        assert!(!gui.is_running());
    }

    #[tokio::test]
    async fn test_selection_communication() {
        let config = CarouselConfig::default();
        let mut gui = WallpaperCarouselGUI::new(config);
        
        // No selection should be available initially
        assert!(gui.try_recv_selection().is_none());
    }
}