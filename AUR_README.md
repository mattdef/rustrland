# Rustrland - AUR Package Files

This directory contains all the files needed to submit Rustrland to the Arch User Repository (AUR).

## 📦 Package Files

| File | Description |
|------|-------------|
| `PKGBUILD` | Main package build script for stable releases |
| `PKGBUILD.git` | Package build script for git/development version |
| `.SRCINFO` | Package metadata for AUR (stable version) |
| `AUR_SUBMISSION.md` | Detailed submission guide |
| `test-pkgbuild.sh` | Local testing script |

## 🚀 Quick Start

### For Package Maintainers

1. **Test Locally:**
   ```bash
   ./test-pkgbuild.sh
   ```

2. **Submit to AUR:**
   ```bash
   # Clone AUR repo
   git clone ssh://aur@aur.archlinux.org/rustrland.git
   cd rustrland
   
   # Copy files
   cp ../PKGBUILD .
   cp ../.SRCINFO .
   
   # Submit
   git add PKGBUILD .SRCINFO
   git commit -m "Initial upload: rustrland v0.3.5"
   git push origin master
   ```

### For Users

Install from AUR (once submitted):
```bash
# Using yay
yay -S rustrland

# Using paru
paru -S rustrland

# Manual build
git clone https://aur.archlinux.org/rustrland.git
cd rustrland
makepkg -si
```

## 📋 Package Information

- **Package Name:** `rustrland`
- **Version:** 0.3.5
- **Architecture:** x86_64, aarch64
- **License:** MIT
- **Upstream:** https://github.com/mattdef/rustrland

### Dependencies

**Required:**
- `hyprland` - The Hyprland compositor

**Build Dependencies:**
- `rust` - Rust compiler (1.81+)
- `cargo` - Rust package manager

**Optional Dependencies:**
- `swaybg` - Default wallpaper backend
- `swww` - Alternative animated wallpaper backend
- `wpaperd` - Per-workspace wallpaper backend
- `imagemagick` - Hardware acceleration for image processing
- `foot` - Recommended terminal emulator
- `firefox` - Browser support for scratchpads
- `thunar` - File manager support for scratchpads

## 🔧 Package Features

### Binaries Installed
- `/usr/bin/rustrland` - Main daemon
- `/usr/bin/rustr` - Command-line client

### Documentation
- `/usr/share/doc/rustrland/` - Complete documentation
- `/usr/share/rustrland/examples/` - Configuration examples

### Integration
- Shell completions (if available)
- Systemd user service (if available)
- Desktop entry (if available)

## 🧪 Testing

The package includes comprehensive testing:

```bash
# Test build
makepkg -s

# Test with dependencies
makepkg -si

# Test functionality
rustrland --version
rustr --help
```

### Test Coverage
- 112 total tests in upstream
- Build verification
- Basic functionality checks
- Configuration validation

## 🔄 Maintenance

### Version Updates

1. Update `pkgver` in PKGBUILD
2. Reset `pkgrel` to 1
3. Update source URL and checksum
4. Regenerate `.SRCINFO`:
   ```bash
   makepkg --printsrcinfo > .SRCINFO
   ```
5. Test and commit

### Git Version

For development builds, use `PKGBUILD.git` as template for `rustrland-git` package.

## 📊 Build Statistics

| Metric | Value |
|--------|-------|
| Build Time | ~3-5 minutes |
| Package Size | ~8MB |
| Dependencies | Minimal |
| Test Suite | 112 tests |
| Memory Usage | ~50MB during build |

## 🎯 Quality Assurance

- ✅ Follows Arch packaging guidelines
- ✅ Comprehensive dependency handling
- ✅ Proper file permissions and locations
- ✅ Complete documentation installation
- ✅ Optional dependency suggestions
- ✅ Clean build environment
- ✅ Verified checksums

## 📞 Support

- **AUR Comments:** For package-specific issues
- **GitHub Issues:** For upstream bugs
- **Documentation:** See installed docs in `/usr/share/doc/rustrland/`

## 🤝 Contributing

Contributions to the PKGBUILD are welcome:

1. Test changes locally with `test-pkgbuild.sh`
2. Follow Arch packaging standards
3. Submit via AUR comments or GitHub
4. Maintain compatibility with official repos

---

*This package provides a production-ready Rust implementation of Pyprland for Hyprland users on Arch Linux.*