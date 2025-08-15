# Scratchpad Research Documentation

This document contains comprehensive research on how scratchpads work in Hyprland and Pyprland to ensure proper implementation in Rustrland.

## Hyprland Special Workspaces (Native Scratchpads)

### Core Concept
- **Special Workspace**: What Hyprland calls a "scratchpad" - a workspace that can be toggled on/off on any monitor
- **Purpose**: For applications you don't always want visible but need quick access to
- **Limitation**: You cannot have floating windows in special workspaces - making a window floating sends it to the currently active real workspace

### Key Hyprland Commands

#### Primary Dispatchers
- `togglespecialworkspace` - Toggles a special workspace on/off
  - `togglespecialworkspace` - Toggles the first special workspace
  - `togglespecialworkspace name` - Toggles a specific named special workspace
  
- `movetoworkspace special` - Moves window to special workspace
- `movetoworkspacesilent special` - Moves window to special workspace silently
- `movetoworkspace special:name` - Moves to named special workspace

#### Important Notes
- The `special` parameter is ONLY supported on `movetoworkspace` and `movetoworkspacesilent`
- Any other dispatcher with special workspaces results in undocumented behavior
- Maximum of 97 named special workspaces at a time

### Configuration Examples
```toml
# Workspace rules
workspace = special:scratchpad, on-created-empty:foot

# Keybindings
bind = ALT, X, togglespecialworkspace, ferdium
bind = ALT, E, togglespecialworkspace, mail
bind = ALT, B, togglespecialworkspace, logseq
```

## Pyprland Scratchpad Plugin

### Enhancement over Native Hyprland
Pyprland builds upon Hyprland's special workspace foundation, providing more sophisticated management:

### Core Features
1. **Automatic Window Management**: Launches applications automatically and manages positioning, sizing, and animations
2. **Advanced Toggle Behavior**: Enhanced logic for showing/hiding windows
3. **Multi-Monitor Support**: Intelligent handling across multiple monitors
4. **Smart Focus**: Improved focus management

### Configuration Options

#### Basic Configuration
```toml
[scratchpads.term]
animation = "fromTop"
command = "kitty --class main-dropterm"
class = "main-dropterm"
size = "75% 60%"
max_size = "1920px 100%"
```

#### Advanced Options
- `alt_toggle`: Changes behavior when triggered on non-focused screen
- `smart_focus`: Can be disabled if workspace changes are spontaneous
- `allow_special_workspaces`: Allows toggling on special workspaces (Hyprland 0.39+)
- `unfocus`: Can hide window when focus is lost (set to "hide")
- `hysteresis`: Controls reactivity of hiding on unfocus
- `excludes`: Hide other scratchpads when this one is displayed (or "*" for all)
- `restore_excluded`: Remember and restore previously hidden scratchpads
- `pinned`: Scratchpads are "pinned by default"
- `preserve_aspect`: Maintains aspect ratio

### Toggle Behavior
When a scratchpad is toggled:
1. Moves scratchpad to currently focused monitor
2. Restores window's previous focus state  
3. Uses "smart focus" for improved UX
4. Handles workspace isolation properly

## Critical Implementation Requirements

### Proper Scratchpad Logic
1. **If window doesn't exist**: Spawn it and show on current workspace
2. **If window exists and visible on current workspace**: Hide it (move to special workspace)
3. **If window exists but hidden**: Show it on current workspace
4. **Never affect other windows**: Operations must be scoped to target window only

### Workspace Management
- **Current Workspace Detection**: Must properly detect active workspace
- **Visibility Detection**: Check if window is on current workspace vs special workspace
- **Workspace Isolation**: Don't move windows between workspaces unintentionally

### Window State Management
- **Floating State**: Make scratchpad windows floating
- **Geometry**: Apply proper size and position
- **Focus**: Handle focus according to configuration
- **Animation**: Perform smooth transitions without state corruption

## Common Issues and Solutions

### Issue: Windows Disappearing
**Cause**: Moving windows to wrong workspace or special workspace when showing
**Solution**: Only move to special workspace when hiding, stay on current workspace when showing

### Issue: Wrong Windows Becoming Floating
**Cause**: Improper window targeting or using generic commands
**Solution**: Always target specific window by address, never use generic toggles

### Issue: Animation State Corruption
**Cause**: Setting opacity to 0 or moving windows off-screen without proper restoration
**Solution**: Use safe positioning-only animations, maintain opacity at 1.0

### Issue: Workspace Isolation Problems
**Cause**: Not properly detecting current workspace or moving between workspaces unexpectedly
**Solution**: Implement proper workspace detection and only change workspace when explicitly hiding/showing

## Key Hyprland API Methods

### Window Management
- `Clients::get()` - Get all windows/clients
- `Workspaces::get()` - Get workspace information
- `Monitors::get()` - Get monitor information

### Window Operations
- `DispatchType::MoveToWorkspace` - Move window to specific workspace
- `DispatchType::MoveToWorkspaceSilent` - Move window silently
- `DispatchType::ResizeWindowPixel` - Resize window by pixels
- `DispatchType::MoveWindowPixel` - Move window by pixels
- `DispatchType::ToggleFloating` - Toggle floating state
- `DispatchType::FocusWindow` - Focus specific window

### Workspace Types
- Regular workspace: ID number (1, 2, 3, etc.)
- Special workspace: `special:name` format
- Current workspace detection: Check workspace.id and filter for active

## Implementation Strategy

### Phase 1: Core Toggle Logic
1. Implement proper workspace detection
2. Create window visibility detection
3. Implement basic show/hide without animations

### Phase 2: Window Management
1. Proper window spawning and configuration
2. Geometry calculation and application
3. Focus management

### Phase 3: Advanced Features
1. Safe animation system
2. Multi-monitor support
3. Advanced configuration options

### Phase 4: Polish
1. Error handling and edge cases
2. Performance optimization
3. Comprehensive testing

## Testing Scenarios

### Basic Functionality
1. Toggle non-existent window (should spawn)
2. Toggle visible window (should hide)
3. Toggle hidden window (should show)
4. Multiple toggles in sequence

### Edge Cases
1. Multiple windows of same class
2. Cross-workspace operations
3. Multi-monitor scenarios
4. Window already floating
5. Special workspace interactions

### Integration Testing
1. Works with other applications running
2. Doesn't affect non-scratchpad windows
3. Proper workspace isolation
4. Animation system stability

## Hyprland Native Animation System

### Animation Configuration Syntax
Animations are declared with: `animation = NAME, ONOFF, SPEED, CURVE [,STYLE]`
- **ONOFF**: 0 to disable, 1 to enable
- **SPEED**: Animation speed/duration
- **CURVE**: Easing curve (default, linear, ease, etc.)
- **STYLE**: Animation style (varies by type)

### Animation Types for Scratchpads

#### Window Animations
- **windowsIn**: Window open animation
- **windowsOut**: Window close animation  
- **windowsMove**: Moving, dragging, resizing
- **Styles**: `slide`, `popin`, `gnomed`

#### Workspace Animations  
- **workspaces**: General workspace transitions
- **specialWorkspace**: Special workspace transitions
- **specialWorkspaceIn**: Special workspace appearing
- **specialWorkspaceOut**: Special workspace disappearing
- **Styles**: `slide`, `slidevert`, `fade`, `slidefade`, `slidefadevert`

#### Fade Animations
- **fadeIn**: Fade in for window open
- **fadeOut**: Fade out for window close
- **fadeSwitch**: Fade on changing activewindow
- **fadeDim**: Dimming of inactive windows

### Key Insights for Scratchpad Implementation

#### Problem with Manual Animation
Our current manual step-by-step animation approach is wrong! Hyprland has native animation support that should be leveraged instead.

#### Correct Approach for "fromTop" Animation
1. **Use Hyprland's native special workspace animations**
2. **Configure `specialWorkspaceIn` with `slidevert` style**
3. **Let Hyprland handle the animation automatically**
4. **Don't manually move windows step-by-step**

#### Critical Finding: Special Workspace Direction Limitations
- Special workspaces sliding from top is not natively supported in Hyprland
- Regular workspaces can slide directionally based on next/previous
- Pyprland likely implements custom positioning to simulate "fromTop"

### Proper Implementation Strategy

#### Instead of Manual Animation:
```rust
// WRONG: Manual step-by-step movement
for step in 1..=steps {
    client.move_window_to_position(address, x, y).await?;
    sleep(16ms).await;
}
```

#### Use Hyprland Native Animations:
```rust
// CORRECT: Let Hyprland animate automatically
client.dispatch(DispatchType::MoveToWorkspace(workspace)).await?;
// Hyprland handles animation based on its configuration
```

#### Configuration Examples:
```toml
# In Hyprland config
animation = specialWorkspace, 1, 8, default, slidevert
animation = windows, 1, 10, default, slide
```

This research forms the foundation for implementing a robust, Pyprland-compatible scratchpad system in Rustrland.