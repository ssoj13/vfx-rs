# Installation

## Requirements

- **Rust 1.85+** (edition 2024)
- **GPU** (optional): Vulkan/Metal/DX12 for wgpu acceleration

### Optional System Dependencies

| Feature | Dependency | Platform |
|---------|------------|----------|
| `heif` | libheif >= 1.17 | All |
| `jp2` | OpenJPEG | All |
| `icc` | lcms2 | All |

## Quick Install

```bash
# From crates.io (when published)
cargo install vfx-cli

# From source
git clone https://github.com/vfx-rs/vfx-rs
cd vfx-rs
cargo install --path crates/vfx-cli
```

## Verify Installation

```bash
vfx --version
vfx info --help
```

## Platform Notes

### Windows

vcpkg is recommended for optional dependencies:

```powershell
# Set vcpkg root
$env:VCPKG_ROOT = "C:\vcpkg"

# Install optional deps
vcpkg install libheif:x64-windows
vcpkg install lcms2:x64-windows
```

### Linux

```bash
# Debian/Ubuntu
sudo apt install libheif-dev liblcms2-dev

# Fedora
sudo dnf install libheif-devel lcms2-devel
```

### macOS

```bash
brew install libheif little-cms2
```

## Next Steps

- [Building from Source](./installation/building.md) - Custom builds
- [Feature Flags](./installation/features.md) - Enable/disable formats
