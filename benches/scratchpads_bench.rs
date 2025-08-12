use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rustrland::plugins::scratchpads::*;
use rustrland::plugins::Plugin;
use rustrland::ipc::MonitorInfo;
use std::collections::HashMap;
use tokio::runtime::Runtime;

// Performance benchmarks for scratchpad operations
fn bench_scratchpad_initialization(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let config: toml::Value = toml::from_str(r#"
        [term]
        command = "foot"
        class = "foot"
        size = "75% 60%"
        lazy = false
        pinned = true
        
        [browser]
        command = "firefox"
        class = "firefox"
        size = "80% 70%"
        lazy = true
        excludes = ["term"]
        
        [editor]
        command = "code"
        class = "code"
        size = "90% 90%"
        multi_window = true
        max_instances = 5
        
        [variables]
        term_class = "foot"
        browser_class = "firefox"
    "#).unwrap();
    
    c.bench_function("scratchpad_initialization", |b| {
        b.iter(|| {
            rt.block_on(async {
                let mut plugin = ScratchpadsPlugin::new();
                plugin.init(&black_box(config.clone())).await.unwrap();
                black_box(plugin);
            });
        });
    });
}

fn bench_size_parsing(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let plugin = ScratchpadsPlugin::new();
    let monitor = MonitorInfo {
        name: "DP-1".to_string(),
        width: 1920,
        height: 1080,
        x: 0,
        y: 0,
        scale: 1.0,
        is_focused: true,
    };
    
    c.bench_function("size_parsing_percentage", |b| {
        b.iter(|| {
            rt.block_on(async {
                let result = plugin.parse_size(
                    black_box("75% 60%"), 
                    black_box(&monitor), 
                    None
                ).await.unwrap();
                black_box(result);
            });
        });
    });
    
    c.bench_function("size_parsing_pixels", |b| {
        b.iter(|| {
            rt.block_on(async {
                let result = plugin.parse_size(
                    black_box("800px 600px"), 
                    black_box(&monitor), 
                    None
                ).await.unwrap();
                black_box(result);
            });
        });
    });
    
    c.bench_function("size_parsing_with_max_size", |b| {
        b.iter(|| {
            rt.block_on(async {
                let result = plugin.parse_size(
                    black_box("90% 90%"), 
                    black_box(&monitor), 
                    Some(black_box("1600px 900px"))
                ).await.unwrap();
                black_box(result);
            });
        });
    });
}

fn bench_dimension_parsing(c: &mut Criterion) {
    let plugin = ScratchpadsPlugin::new();
    
    c.bench_function("dimension_parsing_percentage", |b| {
        b.iter(|| {
            let result = plugin.parse_dimension(black_box("75%"), black_box(1920)).unwrap();
            black_box(result);
        });
    });
    
    c.bench_function("dimension_parsing_pixels", |b| {
        b.iter(|| {
            let result = plugin.parse_dimension(black_box("800px"), black_box(1920)).unwrap();
            black_box(result);
        });
    });
    
    c.bench_function("dimension_parsing_raw", |b| {
        b.iter(|| {
            let result = plugin.parse_dimension(black_box("800"), black_box(1920)).unwrap();
            black_box(result);
        });
    });
}

fn bench_variable_expansion(c: &mut Criterion) {
    let plugin = ScratchpadsPlugin::new();
    let mut variables = HashMap::new();
    variables.insert("term_class".to_string(), "foot".to_string());
    variables.insert("browser_class".to_string(), "firefox".to_string());
    variables.insert("editor_class".to_string(), "code".to_string());
    
    c.bench_function("variable_expansion_single", |b| {
        b.iter(|| {
            let result = plugin.expand_command(
                black_box("foot --app-id=[term_class]"), 
                black_box(&variables)
            );
            black_box(result);
        });
    });
    
    c.bench_function("variable_expansion_multiple", |b| {
        b.iter(|| {
            let result = plugin.expand_command(
                black_box("script [term_class] [browser_class] [editor_class]"), 
                black_box(&variables)
            );
            black_box(result);
        });
    });
    
    c.bench_function("variable_expansion_no_vars", |b| {
        b.iter(|| {
            let result = plugin.expand_command(
                black_box("simple command with no variables"), 
                black_box(&variables)
            );
            black_box(result);
        });
    });
}

fn bench_memory_usage(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    c.bench_function("memory_usage_large_config", |b| {
        b.iter(|| {
            rt.block_on(async {
                let mut plugin = ScratchpadsPlugin::new();
                
                // Create config with many scratchpads
                let mut config_str = String::from("[variables]\nterm_class = \"foot\"\n");
                for i in 0..100 {
                    config_str.push_str(&format!(
                        r#"
[scratchpad_{}]
command = "app_{}"
class = "class_{}"
size = "{}% {}%"
lazy = true
pinned = true
margin = {}
                        "#,
                        i, i, i, 50 + (i % 50), 50 + (i % 40), i % 20
                    ));
                }
                
                let config: toml::Value = toml::from_str(&config_str).unwrap();
                plugin.init(&black_box(config)).await.unwrap();
                black_box(plugin);
            });
        });
    });
}

fn bench_concurrent_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    c.bench_function("concurrent_parsing", |b| {
        b.iter(|| {
            rt.block_on(async {
                let plugin = ScratchpadsPlugin::new();
                let monitor = MonitorInfo {
                    name: "DP-1".to_string(),
                    width: 1920,
                    height: 1080,
                    x: 0,
                    y: 0,
                    scale: 1.0,
                    is_focused: true,
                };
                
                // Simulate concurrent size parsing operations
                let tasks = (0..10).map(|i| {
                    let plugin = &plugin;
                    let monitor = &monitor;
                    async move {
                        plugin.parse_size(
                            &format!("{}% {}%", 50 + i, 40 + i), 
                            monitor, 
                            None
                        ).await.unwrap()
                    }
                });
                
                let results = futures::future::join_all(tasks).await;
                black_box(results);
            });
        });
    });
}

criterion_group!(
    benches,
    bench_scratchpad_initialization,
    bench_size_parsing,
    bench_dimension_parsing,
    bench_variable_expansion,
    bench_memory_usage,
    bench_concurrent_operations
);
criterion_main!(benches);