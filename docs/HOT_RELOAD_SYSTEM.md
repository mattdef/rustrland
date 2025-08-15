# 🔥 Rustrland Advanced Hot Reload System

**The most sophisticated configuration reload system for any Hyprland manager - Far superior to Pyprland**

## 🚀 Key Features

### ✨ **Superior to Pyprland in Every Way**

| Feature | Pyprland | Rustrland |
|---------|----------|-----------|
| Configuration reload | Manual restart required | ✅ Instant hot reload |
| Plugin reloading | ❌ Full restart | ✅ Individual plugin reload |
| State preservation | ❌ Lost on restart | ✅ Maintains active states |
| File watching | ❌ Manual only | ✅ Automatic file watching |
| Validation | ❌ Breaks on errors | ✅ Pre-validation with rollback |
| Smart reloading | ❌ Reloads everything | ✅ Only changed plugins |
| Animation preservation | ❌ N/A | ✅ Active animations continue |
| Error recovery | ❌ N/A | ✅ Automatic rollback |

---

## 🎯 Hot Reload Types

### **1. Manual Reload** ⚡
```bash
rustr reload
```
- Instant configuration reload
- Smart plugin comparison
- State preservation
- Error validation and recovery

### **2. Automatic File Watching** 👀
```toml
[hot_reload]
auto_reload = true
debounce_ms = 500
validate_before_apply = true
```
- Watches configuration files for changes
- 500ms debounce to prevent rapid reloads
- Automatic validation before applying

### **3. Partial Reload** 🎯
- Only reloads changed plugins
- Preserves unchanged plugin states
- Adds/removes plugins as needed
- Zero downtime for unchanged components

---

## 🔧 Configuration

### **Basic Hot Reload Config**
```toml
[hot_reload]
auto_reload = true              # Enable automatic file watching
debounce_ms = 500              # Debounce rapid file changes
validate_before_apply = true   # Validate before applying
backup_on_reload = true        # Create backup of working config
preserve_plugin_state = true   # Maintain plugin states
partial_reload = true          # Only reload changed plugins
```

### **Advanced Settings**
```toml
[hot_reload]
# Performance settings
max_concurrent_reloads = 1
reload_timeout_ms = 5000

# Error handling  
rollback_on_error = true
max_rollback_attempts = 3
preserve_animations = true

# Notifications
notify_on_reload = true
log_reload_stats = true
```

---

## 🎬 Live Reload Examples

### **Scenario 1: Add a New Plugin**

**Before:**
```toml
[rustrland]
plugins = ["scratchpads", "expose"]
```

**After (just save and it auto-reloads):**
```toml
[rustrland]
plugins = ["scratchpads", "expose", "magnify"]  # Added magnify

# Add magnify config
[magnify]
factor = 2.0
duration = 300
```

**Result:** `➕ Added: magnify` - New plugin loads instantly!

### **Scenario 2: Modify Animation Settings**

**Change this:**
```toml
[scratchpads.terminal]

[scratchpads.terminal.animation_config]
duration = 300
easing = "ease-out"
```

**To this:**
```toml
[scratchpads.terminal]

[scratchpads.terminal.animation_config]
duration = 500
easing = "bounce"
```

**Result:** `🔄 Reloaded: scratchpads` - Animation changes instantly!

### **Scenario 3: Remove a Plugin**

**Before:**
```toml
[rustrland]
plugins = ["scratchpads", "expose", "magnify", "workspaces_follow_focus"]
```

**After:**
```toml
[rustrland]
plugins = ["scratchpads", "expose", "magnify"]  # Removed workspaces
```

**Result:** `🗑️ Removed: workspaces_follow_focus` - Plugin unloads safely!

---

## 🧠 Smart Reload Logic

### **Plugin Comparison Algorithm**
```rust
// Rustrland analyzes your config changes:
let current_plugins = get_loaded_plugins();
let new_plugins = parse_new_config();

// Smart decisions:
let added = new_plugins - current_plugins;      // Load these
let removed = current_plugins - new_plugins;    // Unload these  
let modified = changed_configs(intersection);   // Reload these
let unchanged = identical_configs(intersection); // Keep these running
```

### **State Preservation**
- **Active scratchpads remain open** during reload
- **Window positions preserved**
- **Animation states maintained**
- **Plugin internal state saved and restored**

### **Validation Pipeline**
1. **📄 Parse new configuration** - TOML syntax check
2. **🔍 Validate plugin configs** - Schema validation  
3. **⚠️ Pre-flight checks** - Ensure plugins exist
4. **💾 Create backup** - Save current working state
5. **🔄 Apply changes** - Hot swap plugins
6. **✅ Success confirmation** - Report what changed

---

## ⚡ Performance Features

### **Blazing Fast Reloads**
- **Sub-second reload times** for most changes
- **Parallel plugin loading** where possible
- **Minimal disruption** to running processes
- **Memory efficient** state preservation

### **Intelligent Debouncing**
```toml
debounce_ms = 500  # Wait 500ms for multiple rapid changes
```
- Prevents reload storms during editing
- Batches multiple file changes
- Waits for editor save completion

### **Resource Management**
- **Plugin lifecycle management**
- **Memory cleanup** on plugin unload
- **Handle cleanup** for removed plugins  
- **Animation cleanup** for changed configs

---

## 🛡️ Error Recovery

### **Validation Errors**
```bash
$ rustr reload
❌ Reload failed: Invalid configuration: Unknown plugin 'typo'

# Your running config is preserved - no disruption!
```

### **Automatic Rollback**
```toml
[hot_reload]
rollback_on_error = true
```
- Configuration errors don't break your session
- Automatic rollback to last working state
- Detailed error reporting for fixing issues

### **Graceful Degradation**
- **Individual plugin failures** don't break others
- **Partial success handling** - good changes applied
- **Detailed failure reporting** for troubleshooting

---

## 📊 Real-time Monitoring

### **Reload Statistics**
```bash
$ rustr reload
🔍 Comparing configurations:
   Current plugins: ["scratchpads", "expose"]  
   New plugins: ["scratchpads", "expose", "magnify"]
   
🔄 Reload complete: ➕ Added: magnify
   ⏱️  Reload time: 127ms
   💾 Memory usage: +2.3MB  
   ✅ All plugins healthy
```

### **File Watching Status**
```rust
Hot Reload Stats:
- Auto-reload: ✅ Enabled
- Watched paths: 2 files
- Last reload: 12s ago  
- Success rate: 100% (15/15)
- Average reload time: 89ms
```

---

## 🚀 Advanced Usage

### **Multiple Configuration Files**
```bash
# Watch multiple config files
rustrland --config main.toml --config plugins.toml --config animations.toml
```
- Changes to any file trigger reload
- Merged configuration handling
- Smart conflict resolution

### **Development Mode**
```bash
# Ultra-responsive development mode
export RUSTRLAND_DEV_MODE=1
rustrland --config dev.toml
```
- 100ms debounce (vs 500ms normal)
- Detailed reload logging  
- Performance profiling
- Hot reload statistics

### **Plugin Development**
```toml
[hot_reload]
# Perfect for plugin development
auto_reload = true
debounce_ms = 100        # Fast iteration
validate_before_apply = true  # Catch errors early
preserve_plugin_state = false  # Fresh state for testing
```

---

## 🎯 Migration from Pyprland

### **Pyprland (Old Way)**
```bash
# Change config file
nano ~/.config/hypr/pyprland.toml

# Kill and restart (LOSES ALL STATE!)
pkill pyprland
pyprland &

# Hope nothing broke 🤞
```

### **Rustrland (New Way)** ✨
```bash
# Change config file  
nano ~/.config/hypr/rustrland.toml

# Hot reload (PRESERVES ALL STATE!)
rustr reload
# OR: Just save - it auto-reloads!

# Guaranteed to work or rollback ✅
```

---

## 📈 Technical Implementation

### **File Watching Engine**
- **Cross-platform file monitoring** with `notify` crate
- **Efficient polling** with 100ms intervals
- **Smart debouncing** to handle editor quirks
- **Recursive directory watching** for config includes

### **Plugin Lifecycle Management**
```rust
// Plugin hot-swapping process
async fn reload_plugin(name: &str) {
    let state = capture_plugin_state(name).await;
    unload_plugin(name).await;
    load_plugin(name, new_config).await;
    restore_plugin_state(name, state).await;
}
```

### **State Serialization**
- **JSON-based state preservation**
- **Plugin-specific state handlers** 
- **Window position tracking**
- **Animation state preservation**

### **Configuration Diffing**
```rust
// Smart configuration comparison
fn compare_configs(old: &Config, new: &Config) -> Diff {
    Diff {
        added_plugins: new.plugins - old.plugins,
        removed_plugins: old.plugins - new.plugins,  
        modified_configs: find_changed_configs(old, new),
        unchanged: find_identical_configs(old, new),
    }
}
```

---

## 🎬 Live Demo

```bash
# Try the hot reload demo
cp examples/hot-reload-demo.toml ~/.config/hypr/rustrland.toml

# Start Rustrland  
rustrland &

# In another terminal, edit the config:
nano ~/.config/hypr/rustrland.toml
# (make changes and save)

# Watch it reload automatically!
# OR manually trigger:  
rustr reload
```

**🔥 Experience the smoothest, most reliable configuration management system for Hyprland. No more restarts, no more lost state, no more broken sessions. Just pure, instant configuration bliss.**

---

**Rustrland's hot reload system represents a quantum leap beyond Pyprland's primitive restart-based approach. Every change is instant, safe, and reversible.**