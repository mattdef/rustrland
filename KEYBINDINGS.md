# Rustrland Keybinding Setup

This guide shows how to integrate Rustrland scratchpads with Hyprland keybindings for seamless keyboard access.

## Quick Setup

Add these lines to your `~/.config/hypr/hyprland.conf` file:

```bash
# Rustrland Scratchpad Keybindings
# Make sure rustrland daemon is running: make run (in background)

# Terminal scratchpad - Super + ` (backtick/grave accent)
bind = SUPER, grave, exec, rustr toggle term

# Browser scratchpad - Super + B  
bind = SUPER, B, exec, rustr toggle browser

# File manager scratchpad - Super + F
bind = SUPER, F, exec, rustr toggle filemanager

# Music/Spotify scratchpad - Super + M
bind = SUPER, M, exec, rustr toggle music

# Window overview (Expose/Mission Control) - Super + Tab
bind = SUPER, TAB, exec, rustr expose

# List all scratchpads - Super + L
bind = SUPER, L, exec, rustr list

# Show daemon status - Super + Shift + S
bind = SUPER_SHIFT, S, exec, rustr status
```

## Alternative Keybinding Schemes

### Option 1: Function Keys
```bash
# F1-F4 for scratchpads
bind = , F1, exec, rustr toggle term
bind = , F2, exec, rustr toggle browser  
bind = , F3, exec, rustr toggle filemanager
bind = , F4, exec, rustr toggle music
```

### Option 2: Super + Number Keys
```bash
# Super + 1-4 for scratchpads
bind = SUPER, 1, exec, rustr toggle term
bind = SUPER, 2, exec, rustr toggle browser
bind = SUPER, 3, exec, rustr toggle filemanager  
bind = SUPER, 4, exec, rustr toggle music
```

### Option 3: Alt-based (if Super is busy)
```bash
# Alt + ` and Alt + letters
bind = ALT, grave, exec, rustr toggle term
bind = ALT, B, exec, rustr toggle browser
bind = ALT, F, exec, rustr toggle filemanager
bind = ALT, M, exec, rustr toggle music
bind = ALT, TAB, exec, rustr expose
```

## Expose (Window Overview) Keybindings

The expose plugin provides Mission Control-style window overview functionality:

```bash
# Basic expose (toggle)
bind = SUPER, TAB, exec, rustr expose

# Expose with navigation (while expose is active)
bind = SUPER, Right, exec, rustr expose next
bind = SUPER, Left, exec, rustr expose prev
bind = SUPER, Return, exec, rustr expose exit

# Alternative expose keybindings
bind = SUPER, F3, exec, rustr expose           # F3 like macOS
bind = CTRL_ALT, Up, exec, rustr expose        # GNOME-style
```

## Quick Installation

Use the provided installation script for automated setup:

```bash
# Run the installation script
./install-keybindings.sh

# This will:
# - Build rustrland and install rustr to ~/.local/bin
# - Show you the keybindings to add to hyprland.conf
# - Verify everything works
```

## Manual Installation Steps

1. **Install rustr command globally:**
   ```bash
   # Build and install rustr
   cargo build --release
   mkdir -p ~/.local/bin
   cp target/release/rustr ~/.local/bin/rustr
   
   # Add ~/.local/bin to PATH (if not already)
   echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc
   source ~/.bashrc
   ```

2. **Make sure rustrland daemon is running:**
   ```bash
   cd /home/matt/Dev/rust/rustrland
   make run &  # Run in background
   ```

3. **Add keybindings to Hyprland config:**
   ```bash
   # Edit your Hyprland configuration
   nano ~/.config/hypr/hyprland.conf
   
   # Add the keybinding lines from above
   # Choose the scheme that works best for your workflow
   ```

4. **Reload Hyprland configuration:**
   ```bash
   hyprctl reload
   ```

5. **Test the keybindings:**
   - Press `Super + `` (backtick) to toggle terminal
   - Press `Super + B` to toggle browser  
   - Press `Super + F` to toggle file manager

## Configuration Formats

Rustrland supports both legacy Pyprland format and native Rustrland format:

### Option 1: Pyprland-Compatible Format
Perfect for migrating from Pyprland with no changes needed:

```toml
[pyprland]
plugins = ["scratchpads"]

[pyprland.variables]
term_classed = "foot --app-id"

[scratchpads.term]
animation = "fromTop"
command = "[term_classed] main-dropterm"
class = "main-dropterm"
size = "75% 60%"
max_size = "1920px 100%"
```

### Option 2: Native Rustrland Format  
New format designed specifically for Rustrland:

```toml
[rustrland]
plugins = ["scratchpads"]

[rustrland.variables]
term_classed = "foot --app-id"
editor = "code"

[scratchpads.term]
animation = "fromTop"
command = "[term_classed] main-dropterm"
class = "main-dropterm"
size = "75% 60%"

[scratchpads.editor]
animation = "fromLeft"
command = "[editor]"
class = "code"
size = "90% 80%"
```

### Option 3: Dual Configuration
You can use both formats in the same file - Rustrland merges them intelligently:

```toml
# Legacy Pyprland settings
[pyprland]
plugins = ["scratchpads"]

[pyprland.variables]
browser_cmd = "firefox"

# New Rustrland settings (takes precedence)
[rustrland]
plugins = ["scratchpads"]  # Merged with pyprland (no duplicates)

[rustrland.variables]
term_classed = "foot --app-id"  # Overrides pyprland variables
file_manager = "thunar"         # Additional rustrland-only variable
```

See `examples/` directory for complete configuration examples.

## Key Advantages

- **Instant Access**: No need to type commands - just press a key
- **Consistent**: Same keybindings work across all applications
- **Efficient**: Toggle scratchpads without leaving your current window
- **Memorable**: Intuitive key combinations (B for browser, F for files, etc.)

## Troubleshooting

### Keybinding Not Working?
1. Check that rustrland daemon is running: `rustr status`
2. Verify Hyprland config syntax: `hyprctl reload`
3. Test command manually: `rustr toggle term`

### Conflicts with Other Keybindings?
1. Check existing bindings: `hyprctl binds`
2. Choose different key combinations from the alternatives above
3. Use `hyprctl unbind` to remove conflicting bindings

### Want to Change Applications?
1. Edit `~/.config/hypr/rustrland.toml`
2. Update the `command` field for each scratchpad
3. Restart daemon: `pkill rustrland && make run &`

## Advanced Usage

### Auto-start Daemon
Add to your Hyprland config for automatic daemon startup:
```bash
exec-once = cd /home/matt/Dev/rust/rustrland && make run &
```

### Custom Scratchpad Creation
You can create new scratchpads by adding sections to the config:
```toml
[scratchpads.calculator]
command = "gnome-calculator"
class = "gnome-calculator"
size = "30% 40%"
animation = "fromTop"
```

Then add a keybinding:
```bash
bind = SUPER, C, exec, rustr toggle calculator
```