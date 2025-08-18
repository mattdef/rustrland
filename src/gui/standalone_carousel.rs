use egui::{Context, Vec2, Color32, Sense, TextureHandle, ColorImage};
use std::path::PathBuf;
use std::collections::HashMap;
use tokio::sync::mpsc;
use anyhow::Result;
use tracing::{debug, info, warn, error};

use crate::plugins::wallpapers::WallpaperInfo;
use super::wallpaper_carousel::{CarouselConfig, CarouselSelection, CarouselCommand};

/// Standalone carousel app that can run on main thread
pub struct StandaloneCarouselApp {
    // Wallpaper data
    wallpapers: Vec<WallpaperInfo>,
    
    // Navigation state
    selected_index: usize,
    
    // Grid layout
    grid_columns: usize,
    thumbnail_size: Vec2,
    
    // UI state
    should_close: bool,
    hover_index: Option<usize>,
    search_text: String,
    filtered_indices: Vec<usize>,
    
    // Thumbnails
    thumbnails: HashMap<PathBuf, TextureHandle>,
    loading_thumbnails: std::collections::HashSet<PathBuf>,
    
    // Configuration
    config: CarouselConfig,
    
    // Communication
    selection_sender: mpsc::Sender<CarouselSelection>,
    command_receiver: mpsc::Receiver<CarouselCommand>,
}

impl StandaloneCarouselApp {
    pub fn new(
        config: CarouselConfig,
        wallpapers: Vec<WallpaperInfo>,
        selection_sender: mpsc::Sender<CarouselSelection>,
        command_receiver: mpsc::Receiver<CarouselCommand>,
    ) -> Self {
        let thumbnail_size = Vec2::splat(config.thumbnail_size as f32);
        
        // Initialize filtered indices to show all wallpapers
        let filtered_indices = (0..wallpapers.len()).collect();
        
        Self {
            wallpapers,
            selected_index: 0,
            grid_columns: config.grid_columns,
            thumbnail_size,
            should_close: false,
            hover_index: None,
            search_text: String::new(),
            filtered_indices,
            thumbnails: HashMap::new(),
            loading_thumbnails: std::collections::HashSet::new(),
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
        if !self.filtered_indices.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.filtered_indices.len();
        }
    }
    
    fn navigate_left(&mut self) {
        if !self.filtered_indices.is_empty() {
            self.selected_index = if self.selected_index == 0 {
                self.filtered_indices.len() - 1
            } else {
                self.selected_index - 1
            };
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
    
    fn toggle_preview(&mut self) {
        if let Some(&wallpaper_index) = self.filtered_indices.get(self.selected_index) {
            if let Some(wallpaper) = self.wallpapers.get(wallpaper_index) {
                debug!("👁️ Preview requested for: {}", wallpaper.filename);
                let _ = self.selection_sender.try_send(
                    CarouselSelection::PreviewRequested(wallpaper.path.clone())
                );
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
    }
    
    fn render_header(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("🎠 Wallpaper Carousel");
            
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Close button
                if ui.button("❌ Close").clicked() {
                    self.should_close = true;
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
        
        let available_width = ui.available_width();
        let thumbnail_width = self.thumbnail_size.x + self.config.spacing;
        let columns = std::cmp::max(1, (available_width / thumbnail_width).floor() as usize);
        
        egui::ScrollArea::vertical()
            .max_height(ui.available_height())
            .show(ui, |ui| {
                egui::Grid::new("wallpaper_grid")
                    .num_columns(columns)
                    .spacing([self.config.spacing, self.config.spacing])
                    .show(ui, |ui| {
                        // Clone both filtered indices and wallpapers to avoid borrowing issues
                        let filtered_indices = self.filtered_indices.clone();
                        let wallpapers = self.wallpapers.clone();
                        
                        for (display_index, &wallpaper_index) in filtered_indices.iter().enumerate() {
                            if let Some(wallpaper) = wallpapers.get(wallpaper_index) {
                                self.render_thumbnail(ui, display_index, wallpaper);
                            }
                            
                            if (display_index + 1) % columns == 0 {
                                ui.end_row();
                            }
                        }
                    });
            });
    }
    
    fn render_thumbnail(&mut self, ui: &mut egui::Ui, display_index: usize, wallpaper: &WallpaperInfo) {
        let is_selected = display_index == self.selected_index;
        let is_hovered = self.hover_index == Some(display_index);
        
        let color = if is_selected {
            self.config.selection_color
        } else if is_hovered {
            self.config.hover_color
        } else {
            Color32::TRANSPARENT
        };
        
        let response = ui.allocate_response(self.thumbnail_size, Sense::click());
        
        // Handle hover
        if response.hovered() {
            self.hover_index = Some(display_index);
        }
        
        // Handle click
        if response.clicked() {
            self.selected_index = display_index;
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
        
        // Display thumbnail image if available, otherwise show filename
        if let Some(texture) = self.thumbnails.get(&wallpaper.path) {
            // Display the actual image thumbnail
            ui.put(
                response.rect.shrink(2.0),
                egui::Image::from_texture(texture)
                    .fit_to_exact_size(self.thumbnail_size - Vec2::splat(4.0))
                    .rounding(egui::Rounding::same(3.0))
            );
        } else {
            // Show loading placeholder or filename
            let loading_text = if self.loading_thumbnails.contains(&wallpaper.path) {
                "Loading..."
            } else {
                &wallpaper.filename
            };
            
            ui.put(
                response.rect,
                egui::Label::new(loading_text)
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
    
    fn load_thumbnail(&mut self, ctx: &Context, wallpaper: &WallpaperInfo) {
        let path = &wallpaper.path;
        
        // Don't load if already loading or loaded
        if self.loading_thumbnails.contains(path) || self.thumbnails.contains_key(path) {
            return;
        }
        
        self.loading_thumbnails.insert(path.clone());
        
        // Try to load the image using the image crate
        match image::open(path) {
            Ok(img) => {
                // Resize to thumbnail size while maintaining aspect ratio
                let thumb_size = self.config.thumbnail_size as u32;
                let resized = img.thumbnail(thumb_size, thumb_size);
                let rgba_img = resized.to_rgba8();
                
                // Convert to egui ColorImage
                let pixels: Vec<Color32> = rgba_img
                    .pixels()
                    .map(|p| Color32::from_rgba_unmultiplied(p[0], p[1], p[2], p[3]))
                    .collect();
                
                let color_image = ColorImage {
                    size: [resized.width() as usize, resized.height() as usize],
                    pixels,
                };
                
                // Create texture
                let texture = ctx.load_texture(
                    format!("wallpaper_{}", wallpaper.filename),
                    color_image,
                    egui::TextureOptions::default()
                );
                
                self.thumbnails.insert(path.clone(), texture);
                info!("✅ Loaded thumbnail for: {}", wallpaper.filename);
            }
            Err(e) => {
                warn!("⚠️ Failed to load image {}: {}", wallpaper.filename, e);
            }
        }
        
        self.loading_thumbnails.remove(path);
    }
    
    fn ensure_thumbnails_loaded(&mut self, ctx: &Context) {
        // Load thumbnails for visible wallpapers - clone to avoid borrowing issues
        let visible_wallpapers: Vec<_> = self.filtered_indices
            .iter()
            .take(20) // Load first 20 visible thumbnails
            .filter_map(|&i| self.wallpapers.get(i).cloned())
            .collect();
            
        for wallpaper in visible_wallpapers {
            self.load_thumbnail(ctx, &wallpaper);
        }
    }

    fn process_commands(&mut self) {
        while let Ok(command) = self.command_receiver.try_recv() {
            match command {
                CarouselCommand::ShowCarousel(wallpapers) => {
                    debug!("GUI: Showing carousel with {} wallpapers", wallpapers.len());
                    self.wallpapers = wallpapers;
                    self.filtered_indices = (0..self.wallpapers.len()).collect();
                    self.selected_index = 0;
                }
                CarouselCommand::UpdateWallpapers(wallpapers) => {
                    debug!("GUI: Updating wallpapers ({} items)", wallpapers.len());
                    self.wallpapers = wallpapers;
                    self.filtered_indices = (0..self.wallpapers.len()).collect();
                    if self.selected_index >= self.wallpapers.len() {
                        self.selected_index = 0;
                    }
                }
                CarouselCommand::HighlightWallpaper(path) => {
                    if let Some(index) = self.wallpapers.iter().position(|w| w.path == path) {
                        // Find the display index in filtered results
                        if let Some(pos) = self.filtered_indices.iter().position(|&i| i == index) {
                            self.selected_index = pos;
                        }
                    }
                }
                CarouselCommand::CloseCarousel => {
                    self.should_close = true;
                }
            }
        }
    }
}

impl eframe::App for StandaloneCarouselApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        // Process incoming commands
        self.process_commands();
        
        // Ensure thumbnails are loaded for visible wallpapers
        self.ensure_thumbnails_loaded(ctx);
        
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
                ui.vertical(|ui| {
                    // Header with title and search
                    self.render_header(ui);
                    
                    ui.add_space(5.0);
                    
                    // Instructions
                    self.render_instructions(ui);
                    
                    ui.add_space(10.0);
                    
                    // Show current selection info
                    if let Some(&wallpaper_index) = self.filtered_indices.get(self.selected_index) {
                        if let Some(wallpaper) = self.wallpapers.get(wallpaper_index) {
                            ui.label(format!(
                                "Selected: {} ({}/{})",
                                wallpaper.filename,
                                self.selected_index + 1,
                                self.filtered_indices.len()
                            ));
                        }
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