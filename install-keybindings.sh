#!/bin/bash
# Rustrland Keybinding Installation Script

set -e

echo "🦀 Rustrland Keybinding Setup"
echo "=============================="

# Build the project first
echo "📦 Building Rustrland..."
cargo build --release

# Check if ~/.local/bin exists
if [ ! -d "$HOME/.local/bin" ]; then
    echo "📁 Creating ~/.local/bin directory..."
    mkdir -p "$HOME/.local/bin"
fi

# Install rustr to ~/.local/bin
echo "🔧 Installing rustr to ~/.local/bin..."
cp target/release/rustr "$HOME/.local/bin/rustr"
chmod +x "$HOME/.local/bin/rustr"

# Check if ~/.local/bin is in PATH
if [[ ":$PATH:" != *":$HOME/.local/bin:"* ]]; then
    echo "⚠️  Warning: ~/.local/bin is not in your PATH"
    echo "   Add this line to your shell profile (~/.bashrc, ~/.zshrc, etc.):"
    echo "   export PATH=\"\$HOME/.local/bin:\$PATH\""
    echo ""
    echo "   Or run: echo 'export PATH=\"\$HOME/.local/bin:\$PATH\"' >> ~/.bashrc"
    echo ""
fi

# Test the installation
echo "🧪 Testing rustr installation..."
if "$HOME/.local/bin/rustr" --help > /dev/null 2>&1; then
    echo "✅ rustr installed successfully!"
else
    echo "❌ rustr installation failed"
    exit 1
fi

echo ""
echo "🎹 Now add these keybindings to your ~/.config/hypr/hyprland.conf:"
echo ""
cat << 'EOF'
# Rustrland Scratchpad Keybindings
bind = SUPER, grave, exec, rustr toggle term
bind = SUPER, B, exec, rustr toggle browser  
bind = SUPER, F, exec, rustr toggle filemanager
bind = SUPER, M, exec, rustr toggle music
bind = SUPER, L, exec, rustr list
bind = SUPER_SHIFT, S, exec, rustr status
EOF

echo ""
echo "🔄 Then reload Hyprland: hyprctl reload"
echo ""
echo "🚀 Make sure rustrland daemon is running:"
echo "   cd $(pwd) && make run &"
echo ""
echo "✨ Setup complete! Press Super+\` to toggle terminal scratchpad"