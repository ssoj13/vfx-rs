# vfx-rs

**A native Rust alternative to OpenImageIO and OpenColorIO for VFX pipelines.**

## What is vfx-rs?

vfx-rs is a pure Rust image processing toolkit designed for visual effects and film production workflows. It aims to provide the core functionality of industry-standard tools like OIIO (OpenImageIO) and OCIO (OpenColorIO) without C++ dependencies.

## Why Rust?

- **No C++ toolchain required** - Simpler builds, easier deployment
- **Memory safety** - No buffer overflows or use-after-free bugs
- **Performance** - Zero-cost abstractions, SIMD, GPU acceleration via wgpu
- **Cross-platform** - Windows, Linux, macOS from same codebase
- **Python bindings** - PyO3 integration for pipeline scripts

## Core Features

| Feature | OIIO Equivalent | Status |
|---------|-----------------|--------|
| Image I/O (EXR, PNG, JPEG, TIFF, DPX) | `oiiotool` | ✅ |
| Format conversion | `iconvert` | ✅ |
| Image info | `iinfo` | ✅ |
| Image diff | `idiff` | ✅ |
| Texture creation | `maketx` | ✅ |
| Resize/filter | `oiiotool --resize` | ✅ |
| Color transforms | OCIO | ✅ |
| ACES workflow | OCIO ACES config | ✅ |
| LUT application | OCIO | ✅ |
| EXR layers | OIIO | ✅ |
| GPU acceleration | - | ✅ (wgpu) |

## Quick Start

```bash
# Install
cargo install vfx-cli

# Get image info
vfx info render.exr

# Convert EXR to PNG with sRGB transform
vfx convert render.exr output.png

# Resize with Lanczos filter
vfx resize input.exr -w 1920 -h 1080 -o output.exr

# Apply ACES RRT+ODT
vfx aces input.exr -o display.exr --transform rrt-odt

# Batch process
vfx batch "renders/*.exr" -o processed/ --op resize --args width=1920
```

## Project Structure

```
vfx-rs/
├── crates/
│   ├── vfx-core      # Primitives: PixelFormat, Roi
│   ├── vfx-math      # Linear algebra: Vec3, Mat3
│   ├── vfx-io        # Image I/O: EXR, PNG, JPEG, TIFF, DPX
│   ├── vfx-ops       # Operations: resize, blur, composite
│   ├── vfx-compute   # GPU backend: wgpu shaders
│   ├── vfx-lut       # LUT parsing: .cube, .clf
│   ├── vfx-transfer  # Transfer functions: sRGB, PQ, HLG
│   ├── vfx-primaries # Color primaries: Rec709, P3, Rec2020
│   ├── vfx-color     # Color pipeline: conversions
│   ├── vfx-icc       # ICC profiles: lcms2 bindings
│   ├── vfx-ocio      # OCIO config generation
│   ├── vfx-view      # Image viewer: eframe/egui
│   ├── vfx-cli       # CLI tool: vfx command
│   └── vfx-rs-py     # Python bindings: PyO3
├── test/             # Test assets and scripts
└── docs/             # This documentation
```

## Philosophy

1. **Correctness over speed** - Color math must be accurate
2. **VFX-first** - Designed for film/TV workflows, not web images
3. **Composable** - Small crates that do one thing well
4. **Observable** - Structured logging with tracing
5. **Documented** - Every public API has examples

## Code Quality (2026-01-14)

All critical and high-priority bugs have been addressed:

| Category | Status |
|----------|--------|
| PIZ compression | ✅ Overflow protection |
| ACES transforms | ✅ NaN prevention |
| OCIO parity | ✅ ~99% compatible |
| Deep EXR | ✅ Full read/write |
| Color grading | ✅ Division-by-zero protection |

See implementation source for full details.
