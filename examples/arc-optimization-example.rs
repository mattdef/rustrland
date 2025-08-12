// Arc Memory Optimization Example for Rustrland Plugin System
// This demonstrates how to use Arc to optimize memory usage in plugins

use std::sync::{Arc, RwLock};
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

// =====================================================
// 1. CONFIGURATION OPTIMIZATION
// =====================================================

// Before: Expensive cloning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraditionalConfig {
    pub command: String,
    pub class: String,
    pub workspace_rules: HashMap<String, String>,
    pub excludes: Vec<String>,
    // ... 15+ more fields
    // Total size: ~500-2000 bytes per clone
}

// After: Arc-shared configuration
#[derive(Debug, Serialize, Deserialize)]
pub struct OptimizedConfig {
    pub command: String,
    pub class: String,
    pub workspace_rules: HashMap<String, String>,
    pub excludes: Vec<String>,
    // Same fields, but no Clone derive
}

pub type ConfigRef = Arc<OptimizedConfig>;

// Usage comparison:
fn traditional_approach(config: &TraditionalConfig) {
    // Expensive: ~1000 bytes copied
    let config_copy = config.clone();
    process_config(config_copy);
}

fn arc_approach(config: ConfigRef) {
    // Cheap: 8 bytes (pointer + ref count)
    let config_ref = Arc::clone(&config);
    process_config_ref(config_ref);
}

// =====================================================
// 2. SHARED STATE OPTIMIZATION
// =====================================================

// Before: Each plugin maintains separate monitor state
pub struct TraditionalPlugin {
    monitors: HashMap<String, MonitorInfo>, // Duplicated across plugins
    workspaces: HashMap<i32, WorkspaceInfo>, // Duplicated across plugins
}

// After: Shared state via Arc
pub struct OptimizedPlugin {
    monitor_cache: MonitorCacheRef,   // Shared reference
    workspace_cache: WorkspaceCacheRef, // Shared reference
}

#[derive(Debug)]
pub struct MonitorInfo {
    pub id: i128,
    pub name: String,
    pub focused: bool,
    pub active_workspace: i32,
    pub width: u16,
    pub height: u16,
    pub x: i32,
    pub y: i32,
}

#[derive(Debug)]
pub struct WorkspaceInfo {
    pub id: i32,
    pub name: String,
    pub monitor: String,
    pub windows: u16,
}

// Shared cache types
pub type MonitorInfoRef = Arc<RwLock<MonitorInfo>>;
pub type WorkspaceInfoRef = Arc<RwLock<WorkspaceInfo>>;
pub type MonitorCacheRef = Arc<RwLock<HashMap<String, MonitorInfoRef>>>;
pub type WorkspaceCacheRef = Arc<RwLock<HashMap<i32, WorkspaceInfoRef>>>;

// =====================================================
// 3. GLOBAL SHARED CACHE SYSTEM
// =====================================================

pub struct GlobalStateCache {
    monitors: MonitorCacheRef,
    workspaces: WorkspaceCacheRef,
    configs: Arc<RwLock<HashMap<String, ConfigRef>>>,
}

impl GlobalStateCache {
    pub fn new() -> Self {
        Self {
            monitors: Arc::new(RwLock::new(HashMap::new())),
            workspaces: Arc::new(RwLock::new(HashMap::new())),
            configs: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    // Get monitor info with automatic caching
    pub async fn get_monitor(&self, name: &str) -> Option<MonitorInfoRef> {
        let monitors = self.monitors.read().unwrap();
        monitors.get(name).cloned()
    }
    
    // Update monitor info (all plugins see update automatically)
    pub async fn update_monitor(&self, name: String, info: MonitorInfo) {
        let mut monitors = self.monitors.write().unwrap();
        let monitor_ref = Arc::new(RwLock::new(info));
        monitors.insert(name, monitor_ref);
    }
    
    // Get references for sharing with plugins
    pub fn get_monitor_cache(&self) -> MonitorCacheRef {
        Arc::clone(&self.monitors)
    }
    
    pub fn get_workspace_cache(&self) -> WorkspaceCacheRef {
        Arc::clone(&self.workspaces)
    }
}

// =====================================================
// 4. OPTIMIZED PLUGIN TRAIT
// =====================================================

use async_trait::async_trait;

#[async_trait]
pub trait OptimizedPlugin {
    fn name(&self) -> &str;
    
    // Receive shared state references instead of owning data
    async fn init(&mut self, 
        config: ConfigRef,
        global_cache: &GlobalStateCache
    ) -> Result<(), Box<dyn std::error::Error>>;
    
    // Work with shared references
    async fn handle_event(&mut self, event: &HyprlandEvent) -> Result<(), Box<dyn std::error::Error>>;
}

// =====================================================
// 5. PERFORMANCE COMPARISON
// =====================================================

pub fn memory_usage_comparison() {
    println!("Memory Usage Comparison:");
    println!("========================");
    
    println!("Traditional Cloning:");
    println!("- Config clone: ~1000 bytes");
    println!("- Monitor info (5 plugins): 5 × 200 bytes = 1000 bytes");
    println!("- Window state (20 windows): 20 × 300 bytes = 6000 bytes");
    println!("- Total per update: ~8000 bytes");
    
    println!("\nArc-Optimized:");
    println!("- Config reference: 8 bytes");
    println!("- Monitor cache reference: 8 bytes");
    println!("- Window state references: 20 × 8 bytes = 160 bytes");
    println!("- Total per update: ~176 bytes");
    
    println!("\nMemory Savings: ~95% reduction (8000 → 176 bytes)");
    println!("CPU Savings: ~90% reduction (no expensive clones)");
}

// =====================================================
// 6. WHEN TO USE ARC VS CLONE
// =====================================================

pub fn optimization_guidelines() {
    println!("Arc Optimization Guidelines:");
    println!("============================");
    
    println!("✅ USE ARC FOR:");
    println!("- Configuration objects (read-mostly)");
    println!("- Shared state between plugins");
    println!("- Large data structures (>100 bytes)");
    println!("- Cached computation results");
    println!("- Monitor/workspace information");
    
    println!("\n❌ KEEP CLONE FOR:");
    println!("- Small primitives (i32, bool, etc.)");
    println!("- Temporary data");
    println!("- Data that needs mutation without synchronization");
    println!("- Short-lived objects");
    
    println!("\n🔒 USE ARC<RWLOCK<T>> FOR:");
    println!("- Mutable shared state");
    println!("- Data updated by multiple threads");
    println!("- State that needs consistency guarantees");
    
    println!("\n⚡ PERFORMANCE NOTES:");
    println!("- Arc clone: ~2ns (pointer copy)");
    println!("- String clone: ~50-500ns (depends on size)");
    println!("- HashMap clone: ~1000-10000ns (depends on size)");
    println!("- RwLock read: ~10-20ns");
    println!("- RwLock write: ~20-50ns");
}

// =====================================================
// 7. IMPLEMENTATION STRATEGY
// =====================================================

pub fn implementation_phases() {
    println!("Arc Implementation Strategy:");
    println!("============================");
    
    println!("Phase 1: Configuration Objects");
    println!("- Convert plugin configs to Arc<Config>");
    println!("- Estimated savings: 30-50% memory reduction");
    println!("- Risk: Low (mostly read-only)");
    
    println!("\nPhase 2: Shared State Cache");
    println!("- Create global monitor/workspace cache");
    println!("- Estimated savings: 70-80% memory reduction"); 
    println!("- Risk: Medium (needs synchronization)");
    
    println!("\nPhase 3: Window State Management");
    println!("- Arc-optimize large window state structures");
    println!("- Estimated savings: 60-70% memory reduction");
    println!("- Risk: Medium (complex state management)");
    
    println!("\nPhase 4: Event Processing");
    println!("- Optimize event data sharing");
    println!("- Estimated savings: 20-30% memory reduction");
    println!("- Risk: Low (mostly immutable data)");
}

// Dummy types for compilation
#[derive(Debug)]
pub struct HyprlandEvent;

fn process_config(_config: TraditionalConfig) {}
fn process_config_ref(_config: ConfigRef) {}