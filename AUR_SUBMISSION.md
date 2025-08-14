# AUR Submission Guide for Rustrland

This document provides instructions for submitting Rustrland to the Arch User Repository (AUR).

## Files Created

- `PKGBUILD` - Main package build script
- `.SRCINFO` - Package metadata for AUR
- `AUR_SUBMISSION.md` - This guide

## Prerequisites

1. **AUR Account**: Create an account at [aur.archlinux.org](https://aur.archlinux.org)
2. **SSH Key**: Add your SSH public key to your AUR account
3. **Git**: Ensure git is installed and configured

## Submission Steps

### 1. Generate Checksum

First, generate the actual SHA256 checksum for the release:

```bash
# Download the release tarball
curl -L -o rustrland-0.3.2.tar.gz "https://github.com/mattdef/rustrland/archive/v0.3.2.tar.gz"

# Generate checksum
sha256sum rustrland-0.3.2.tar.gz

# Update the PKGBUILD file with the actual checksum
sed -i "s/sha256sums=('SKIP')/sha256sums=('ACTUAL_CHECKSUM_HERE')/" PKGBUILD
```

### 2. Test the Package

Before submitting, test the package locally:

```bash
# Install build dependencies
sudo pacman -S base-devel rust

# Test build
makepkg -si

# Verify installation
rustrland --version
rustr --help
```

### 3. Clone AUR Repository

```bash
# Clone the AUR repository (replace with actual package name)
git clone ssh://aur@aur.archlinux.org/rustrland.git aur-rustrland
cd aur-rustrland
```

### 4. Add Package Files

```bash
# Copy package files
cp ../PKGBUILD .
cp ../.SRCINFO .

# Generate .SRCINFO (if modified)
makepkg --printsrcinfo > .SRCINFO
```

### 5. Commit and Push

```bash
# Add files
git add PKGBUILD .SRCINFO

# Commit
git commit -m "Initial upload: rustrland v0.3.2

- Rust implementation of Pyprland for Hyprland
- Fast, reliable plugin system with memory safety
- Complete scratchpad, wallpaper, monitor, and animation support
- 112 comprehensive tests and production-ready CI/CD"

# Push to AUR
git push origin master
```

## Package Features

### Core Functionality
- **Daemon**: Main `rustrland` binary with plugin system
- **Client**: `rustr` command-line client for IPC communication
- **Plugins**: Scratchpads, wallpapers, monitors, expose, workspaces, magnify

### Dependencies
- **Required**: hyprland
- **Optional**: swaybg, swww, wpaperd, imagemagick, foot, firefox, thunar

### Documentation
- README.md - Main project documentation
- PLUGINS.md - Comprehensive plugin documentation  
- KEYBINDINGS.md - Keyboard integration guide
- CLAUDE.md - Development and project instructions
- examples/ - Configuration examples

## Maintenance

### Updating the Package

When a new version is released:

1. Update `pkgver` in PKGBUILD
2. Update `pkgrel` to 1
3. Update source URL and checksum
4. Regenerate .SRCINFO: `makepkg --printsrcinfo > .SRCINFO`
5. Test build: `makepkg -si`
6. Commit and push changes

### Version Naming Convention

- **Stable releases**: `rustrland` (this package)
- **Development builds**: `rustrland-git` (separate package)
- **Beta releases**: `rustrland-beta` (if needed)

## Support

- **Package Issues**: Report to AUR comments or GitHub issues
- **Build Issues**: Check build logs and dependencies
- **Runtime Issues**: Check Hyprland compatibility and configuration

## Contributing

Contributions to the PKGBUILD are welcome:
1. Test changes locally
2. Submit patches via AUR comments or GitHub
3. Follow Arch packaging guidelines
4. Maintain compatibility with official repositories

---

*For more information about AUR submissions, see the [Arch Wiki](https://wiki.archlinux.org/title/Arch_User_Repository).*