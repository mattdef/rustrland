# Workspaces Follow Focus Plugin

**Status**: ⚠️ **UNDER ACTIVE DEVELOPMENT** - Core functionality broken | **Tests**: Limited

⚠️ **CRITICAL ISSUES**: This plugin is currently unstable with major functionality broken. See TODO section below for required fixes.

Basic workspace management plugin inspired by Pyprland's workspaces_follow_focus. Currently supports limited workspace listing and partial navigation.

## Currently Working Features

✅ **Workspace Listing**: Display current workspaces and monitors (`rustr workspace list`)
✅ **Status Display**: Show plugin status and configuration (`rustr workspace status`)
✅ **Partial Relative Navigation**: Forward navigation only (`rustr workspace change +1`)

## Broken Features (Need Fixes)

❌ **Direct Workspace Switching**: `rustr workspace switch N` fails with "Previous workspace doesn't exist"
❌ **Backward Navigation**: `rustr workspace change -- -1` fails with range errors
❌ **Animation System**: Completely disabled due to circular dependencies
❌ **Most Advanced Features**: Monitor-specific rules, persistence, templates, etc.

## Configuration

### Basic Working Configuration

```toml
[workspaces_follow_focus]
# Basic settings that actually work
follow_window_focus = true
allow_cross_monitor_switch = true
workspace_switching_delay = 100
debug_logging = true  # Recommended for troubleshooting

# Workspace rules (basic monitor locking)
workspace_rules = { "1" = "DP-1", "2" = "DP-2" }
```

### ⚠️ Configuration Options Not Yet Implemented

Many configuration options shown in the code are not yet functional:
- `max_workspaces`, `auto_create`, `start_workspace`
- Complex monitor strategies and workspace ranges
- Named workspaces and templates
- Persistence and history features
- Advanced focus following options

## Commands

### ✅ Working Commands

```bash
# Workspace information (WORKS)
rustr workspace list            # List all workspaces and monitors
rustr workspace status          # Show detailed workspace status

# Partial relative navigation (WORKS - forward only)
rustr workspace change +1       # Next workspace
```

### ❌ Broken Commands (Need Fixes)

```bash
# Direct workspace switching (BROKEN)
rustr workspace switch 1        # ❌ Fails: "Previous workspace doesn't exist"
rustr workspace switch 2        # ❌ Fails: Hyprland dispatcher error

# Backward navigation (BROKEN) 
rustr workspace change -- -1    # ❌ Fails: "Workspace 0 out of range"
rustr workspace change +2       # ⚠️  May work depending on current workspace
```

### 🚫 Not Yet Implemented Commands

The following commands are documented but don't exist in the current implementation:
- `current`, `create`, `remove`, `rename`
- `move-to-monitor`, `switch-monitor`
- `move-window`, `follow-window`
- `list-monitors`, `list-windows`, `history`
- `reload`, `reset`, `back`, `forward`

## Keybindings

### ⚠️ Temporary Working Keybindings

Add to your `~/.config/hypr/hyprland.conf` (only working commands):

```bash
# Working commands only
bind = SUPER, Right, exec, rustr workspace change +1   # Next workspace (works)
bind = SUPER, End, exec, rustr workspace list         # Show workspace list (works)
bind = SUPER_SHIFT, S, exec, rustr workspace status   # Show status (works)
```

### ❌ Don't Use These (Broken)

```bash
# These keybindings will fail - avoid until fixed
# bind = SUPER, 1, exec, rustr workspace switch 1      # BROKEN
# bind = SUPER, Left, exec, rustr workspace change -- -1  # BROKEN
```

Use Hyprland's native workspace switching as workaround:
```bash
# Hyprland native (working alternative)
bind = SUPER, 1, workspace, 1
bind = SUPER, 2, workspace, 2
bind = SUPER, Left, workspace, -1
bind = SUPER, Right, workspace, +1
```

---

# 🚧 TODO: Critical Development Tasks

## 🔥 Priority 1: Critical Fixes (Required for Basic Functionality)

### 1.1 Fix Direct Workspace Switching
- **Issue**: `rustr workspace switch N` fails with "Previous workspace doesn't exist"
- **Location**: `src/plugins/workspaces_follow_focus.rs:461-465`
- **Cause**: Incorrect Hyprland dispatcher call
- **Fix Required**: 
  - Use correct `WorkspaceIdentifier` type instead of `WorkspaceIdentifierWithSpecial`
  - Test dispatcher calls with real Hyprland instance
  - Validate workspace existence before switching

### 1.2 Fix Backward Navigation
- **Issue**: `rustr workspace change -- -1` fails with "Workspace 0 out of range"
- **Location**: `src/plugins/workspaces_follow_focus.rs:497-505`
- **Cause**: No bounds checking or wrap-around logic
- **Fix Required**:
  - Implement proper wrap-around (workspace 1 → 10, 10 → 1)
  - Handle negative offsets correctly
  - Add boundary validation

### 1.3 Simplify Command Interface
- **Issue**: Over-complex interface compared to Pyprland original
- **Goal**: Align with Pyprland's simple `change_workspace [direction]`
- **Fix Required**:
  - Implement `change_workspace` as primary command
  - Keep `switch` and `change` as aliases for compatibility
  - Remove non-functional commands from documentation

## 🛠️ Priority 2: Technical Improvements

### 2.1 Repair Animation System
- **Issue**: Animation system disabled due to circular dependencies
- **Location**: Multiple commented sections in plugin code
- **Fix Required**:
  - Resolve circular dependency between animation and plugin modules
  - Re-enable animation timeline functionality
  - Test smooth workspace transitions

### 2.2 Optimize Performance
- **Issue**: Excessive calls to `update_monitors()` and `update_workspaces()`
- **Fix Required**:
  - Implement intelligent caching with cache invalidation
  - Reduce API calls by batching operations
  - Add performance metrics and monitoring

### 2.3 Add Missing Commands
- **Issue**: Many documented commands don't exist
- **Fix Required**:
  - Implement `current` command
  - Add basic `create` and `remove` functionality
  - Return proper error messages for unimplemented features

## 📝 Priority 3: Documentation and Polish

### 3.1 Update Documentation Status
- **Task**: Mark non-working features clearly
- **Update**: Configuration examples to show only working options
- **Add**: Troubleshooting section with common issues
- **Create**: Migration guide from current broken state

### 3.2 Add Integration Tests
- **Task**: Test with real Hyprland instance
- **Validate**: All basic commands work correctly
- **Test**: Multi-monitor scenarios
- **Verify**: Error handling and edge cases

### 3.3 Improve Error Messages
- **Task**: Replace generic errors with helpful messages
- **Add**: Suggestions for working alternatives
- **Include**: Debug information in error outputs

## 🎯 Success Criteria

**Milestone 1 - Basic Functionality**:
- ✅ `rustr workspace switch N` works without errors
- ✅ `rustr workspace change +1` and `rustr workspace change -- -1` both work
- ✅ Commands match Pyprland's `change_workspace` behavior

**Milestone 2 - Stability**:
- ✅ Animation system functional
- ✅ Performance optimized (minimal API calls)
- ✅ Multi-monitor switching stable

**Milestone 3 - Polish**:
- ✅ Documentation accurate and up-to-date
- ✅ Integration tests passing
- ✅ Error handling comprehensive

## 📞 Development Notes

- **Test Environment**: Requires active Hyprland session with multiple monitors
- **Dependencies**: Fix animation system circular dependencies first
- **Validation**: Compare behavior with Pyprland original
- **Performance**: Profile before/after optimizations

---

## ⚠️ Current Limitations & Workarounds

### Known Issues

**Direct workspace switching fails:**
```bash
# This command fails with "Previous workspace doesn't exist"
rustr workspace switch 1

# Workaround: Use Hyprland's native command
hyprctl dispatch workspace 1
```

**Backward navigation broken:**
```bash
# This fails with "Workspace 0 out of range"
rustr workspace change -- -1

# Workaround: Use Hyprland's native command
hyprctl dispatch workspace -1
```

**Animation system disabled:**
- All animation features are currently non-functional
- No smooth transitions between workspaces
- Plugin loads but animations are skipped

### Working Debug Commands

```bash
# These commands work for debugging
rustr workspace status          # Shows plugin status and config
rustr workspace list            # Lists workspaces and monitors
rustr workspace change +1       # Forward navigation only
```

### Temporary Solution

Until the plugin is fixed, use Hyprland's native workspace switching:

```bash
# Native Hyprland commands (reliable alternative)
hyprctl dispatch workspace 1    # Switch to workspace 1
hyprctl dispatch workspace +1    # Next workspace
hyprctl dispatch workspace -1    # Previous workspace
```

## Development Status

This plugin is actively being developed to achieve compatibility with Pyprland's workspaces_follow_focus. Current implementation provides ~30% of intended functionality. See TODO section above for development roadmap.