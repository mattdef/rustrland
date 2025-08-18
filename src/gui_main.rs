use clap::{Arg, Command};
use anyhow::Result;
use std::path::PathBuf;
use tracing::{info, error};

use rustrland::gui::{CarouselConfig, CarouselSelection, StandaloneCarouselApp};
use rustrland::plugins::wallpapers::WallpaperInfo;

fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    // Parse command line arguments
    let matches = Command::new("rustrland-gui")
        .version(env!("CARGO_PKG_VERSION"))
        .about("GUI components for Rustrland wallpaper management")
        .arg(
            Arg::new("wallpapers")
                .long("wallpapers")
                .value_name("FILE")
                .help("JSON file containing wallpaper data")
                .required(true)
        )
        .arg(
            Arg::new("mode")
                .long("mode")
                .value_name("MODE")
                .help("GUI mode to run")
                .default_value("carousel")
        )
        .get_matches();

    let wallpapers_file = matches.get_one::<String>("wallpapers").unwrap();
    let mode = matches.get_one::<String>("mode").unwrap();

    match mode.as_str() {
        "carousel" => run_carousel(wallpapers_file),
        _ => {
            error!("Unknown mode: {}", mode);
            std::process::exit(1);
        }
    }
}

fn run_carousel(wallpapers_file: &str) -> Result<()> {
    info!("🎠 Starting wallpaper carousel GUI");

    // Load wallpapers from JSON file
    let wallpapers_data = std::fs::read_to_string(wallpapers_file)?;
    let wallpapers: Vec<WallpaperInfo> = serde_json::from_str(&wallpapers_data)?;
    
    info!("📂 Loaded {} wallpapers", wallpapers.len());

    if wallpapers.is_empty() {
        eprintln!("No wallpapers provided");
        std::process::exit(1);
    }

    // Create GUI configuration
    let config = CarouselConfig::default();

    // Create channel for receiving selection
    let (selection_tx, mut selection_rx) = tokio::sync::mpsc::channel::<CarouselSelection>(32);

    // Since we need to communicate selection back to the CLI, we'll use a simple approach:
    // Store the selected wallpaper path in a shared variable that the main thread can access
    use std::sync::{Arc, Mutex};
    let selected_path: Arc<Mutex<Option<PathBuf>>> = Arc::new(Mutex::new(None));
    let selected_path_clone = Arc::clone(&selected_path);

    // Spawn a task to handle selection messages
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            while let Some(selection) = selection_rx.recv().await {
                match selection {
                    CarouselSelection::Selected(path) => {
                        *selected_path_clone.lock().unwrap() = Some(path);
                        break;
                    }
                    CarouselSelection::Cancelled => {
                        break;
                    }
                    CarouselSelection::PreviewRequested(_) => {
                        // Handle preview if needed
                    }
                }
            }
        });
    });

    // Create channel for commands (not used in this simple version)
    let (_command_tx, command_rx) = tokio::sync::mpsc::channel(32);

    // Create and run the carousel app on main thread
    let app = StandaloneCarouselApp::new(
        config,
        wallpapers,
        selection_tx,
        command_rx,
    );

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_title("rustrland-wallpaper-carousel")
            .with_app_id("rustrland-wallpaper-carousel") // Set app ID for window class
            .with_resizable(true)
            .with_always_on_top()
            .with_decorations(false), // Remove window decorations for floating look
        ..Default::default()
    };

    // Run the GUI on the main thread  
    let result = eframe::run_native(
        "rustrland-wallpaper-carousel",
        options,
        Box::new(|_cc| {
            Box::new(app)
        }),
    );

    // After GUI closes, output the selected path
    if let Ok(path_guard) = selected_path.lock() {
        if let Some(path) = &*path_guard {
            println!("{}", path.display());
        } else {
            println!("cancelled");
        }
    } else {
        println!("cancelled");
    }

    result.map_err(|e| anyhow::anyhow!("GUI error: {}", e))
}