# vfx-rs

**Native Rust image processing toolkit for VFX and film production pipelines.**

> A pure Rust alternative to OpenImageIO + OpenColorIO - no C++ toolchain required.

[![Rust 1.85+](https://img.shields.io/badge/rust-1.85+-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)

---

## Why vfx-rs?

OpenImageIO and OpenColorIO are industry-standard tools that power countless productions. vfx-rs offers a complementary approach for teams who prefer Rust's ecosystem:

- **Simple builds** - `cargo build` handles everything, no separate toolchain setup
- **Memory safety** - Rust's guarantees help prevent common bugs in image processing code
- **GPU acceleration** - wgpu (Vulkan/Metal/DX12) and CUDA backends
- **Python bindings** - PyO3 for pipeline integration
- **Cross-platform** - Same code runs on Windows, Linux, macOS

## Feature Overview

| Category | Features |
|----------|----------|
| **Formats** | EXR, PNG, JPEG, TIFF, DPX, HDR, WebP, PSD (read), TX |
| **Color** | sRGB, Rec.709, Rec.2020, DCI-P3, ACEScg, ACES2065-1 |
| **Transfer Functions** | sRGB, PQ, HLG, LogC3, LogC4, S-Log2/3, V-Log, Canon Log 2/3, Apple Log, ACEScc/cct, REDLog |
| **LUTs** | .cube, .clf, .spi1d/.spi3d, .csp |
| **Operations** | Resize, Crop, Rotate, Flip, Blur, Sharpen, Composite |
| **ACES** | IDT, RRT, ODT, LMT transforms |
| **I/O** | Streaming read/write, tiled caching, multi-layer EXR |
| **GPU** | Auto backend selection, automatic tiling, operation fusion |

## Quick Start

```bash
# Install CLI
cargo install vfx-cli

# Get image info
vfx info render.exr

# Convert EXR to PNG with sRGB transform
vfx convert render.exr output.png

# Resize with Lanczos filter
vfx resize input.exr -w 1920 -h 1080 -o output.exr

# Apply ACES RRT+ODT for viewing
vfx aces linear.exr -o preview.png -t rrt-odt

# Batch process directory
vfx batch "renders/*.exr" -o processed/ --op resize --args scale=0.5
```

## ACES Workflow Example

```bash
# 1. Camera footage -> ACEScg (Input Device Transform)
vfx aces camera.dpx -o working.exr -t idt

# 2. Grade in ACEScg (linear, wide-gamut)
vfx color working.exr -o graded.exr --exposure 0.3 --saturation 1.1

# 3. ACEScg -> sRGB display (Reference Rendering + Output Transform)
vfx aces graded.exr -o final.png -t rrt-odt
```

## Library Usage

```rust
use vfx_io::{read, write};
use vfx_ops::resize;
use vfx_color::ColorProcessor;

fn main() -> anyhow::Result<()> {
    // Read any supported format
    let image = read("input.exr")?;
    
    // Resize with Lanczos
    let resized = resize(&image, 1920, 1080)?;
    
    // Color transform (GPU-accelerated when available)
    let processor = ColorProcessor::new(Backend::Auto)?;
    processor.apply_matrix(&mut resized, &exposure_matrix)?;
    
    // Write output
    write("output.exr", &resized)?;
    Ok(())
}
```

### OCIO Integration

```rust
use vfx_ocio::{Config, Processor};

fn main() -> anyhow::Result<()> {
    // Load config from $OCIO or explicit path
    let config = Config::from_env()?;  // or Config::from_file("config.ocio")?
    
    // List available color spaces
    for cs in config.color_spaces() {
        println!("{}: {}", cs.name(), cs.family());
    }
    
    // Create processor for color space conversion
    let processor = config.processor("ACEScg", "sRGB")?;
    
    // Apply to image data
    let mut pixels: Vec<f32> = read_pixels("render.exr")?;
    processor.apply(&mut pixels, 4)?;  // 4 channels (RGBA)
    
    // Display transform with look
    let display_processor = config.display_processor(
        "ACEScg",      // input space
        "sRGB",        // display
        "Film",        // view
        Some("Warm"),  // optional look
    )?;
    display_processor.apply(&mut pixels, 4)?;
    
    Ok(())
}
```

## Architecture

```
vfx-rs/
├── vfx-core        # ImageData, ImageSpec, ChannelType
├── vfx-math        # Vec3, Mat3, Mat4, SIMD (f32x4/x8), chromatic adaptation
├── vfx-transfer    # Transfer functions (sRGB, PQ, HLG, LogC3/4, Canon Log...)
├── vfx-primaries   # Color primaries (Rec.709, P3, Rec.2020, AWG4, CGamut...)
├── vfx-lut         # LUT parsing (.cube, .clf, .spi, .csp, CDL)
├── vfx-color       # Color pipeline, ACES 2.0, grading ops
├── vfx-io          # Image I/O (EXR, PNG, JPEG, TIFF, DPX, HDR...)
├── vfx-icc         # ICC profile support (lcms2)
├── vfx-ocio        # OpenColorIO config compatibility
├── vfx-ops         # Image operations (resize, blur, composite, FFT)
├── vfx-compute     # GPU backends (CPU/wgpu/CUDA)
├── vfx-view        # Image viewer (egui)
├── vfx-cli         # Command-line tool
└── vfx-rs-py       # Python bindings (PyO3)
```

### Camera Color Spaces

Full camera gamut support verified against OCIO ColorMatrixHelpers.cpp:

| Camera | Gamut | Transfer | Status |
|--------|-------|----------|--------|
| ARRI Alexa | AWG3, AWG4 | LogC3, LogC4 | Verified |
| Sony Venice/FX | S-Gamut3, S-Gamut3.Cine | S-Log3 | Verified |
| Canon C500/C300 | Cinema Gamut | Canon Log 2/3 | Verified |
| RED | REDWideGamutRGB | Log3G10, REDLogFilm | Verified |
| Panasonic VariCam | V-Gamut | V-Log | Verified |
| Apple iPhone 15 Pro+ | - | Apple Log | Verified |

## Parity Status

### vs OpenImageIO (~50%)

| Feature | Status |
|---------|--------|
| Image I/O (11 formats) | **Production-ready** |
| Multi-layer EXR | **Production-ready** |
| Streaming I/O | **Production-ready** - progressive read/write for large files |
| Tiled image caching | **Production-ready** - on-demand tile loading |
| UDIM textures | **Production-ready** |
| ImageBufAlgo (100+ functions) | Partial |
| Deep data | Not implemented |
| Plugin system | Not implemented |

### vs OpenColorIO (~90%)

| Feature | Status |
|---------|--------|
| Config YAML parsing | **Production-ready** |
| ColorSpace definitions | **Production-ready** |
| Roles, Displays, Views | **Production-ready** |
| Shared Views (v2.3+) | **Production-ready** |
| Context variables | **Production-ready** |
| Matrix/CDL/Log/Range transforms | **Production-ready** |
| BuiltinTransform | **Production-ready** |
| FileTransform (LUTs) | **Production-ready** - .cube, .spi1d, .spi3d, .clf, .ctf |
| GradingPrimary/Tone/Curve | **Production-ready** |

### Transfer Functions (100%)

All major camera log curves verified against OCIO reference code:

| Function | Verified Against | Status |
|----------|-----------------|--------|
| sRGB | IEC 61966-2-1 | 51+ tests passing |
| PQ (HDR10) | SMPTE ST 2084 | Verified |
| HLG | ITU-R BT.2100-2 | Verified |
| ARRI LogC3 | OCIO ArriCameras.cpp | Verified |
| ARRI LogC4 | OCIO ArriCameras.cpp | Verified (new) |
| Sony S-Log2/3 | OCIO SonyCameras.cpp | Verified |
| Panasonic V-Log | OCIO PanasonicCameras.cpp | Verified |
| Canon Log 2/3 | OCIO CanonCameras.cpp | Verified (new) |
| Apple Log | OCIO AppleCameras.cpp | Verified (new) |
| RED REDLogFilm/3G10 | OCIO RedCameras.cpp | Verified |
| Blackmagic Film Gen5 | BMD spec | Verified |
| ACEScc/cct | AMPAS | Verified |

## GPU Acceleration

vfx-compute provides transparent GPU acceleration with smart defaults:

```rust
let processor = Processor::builder()
    .backend(Backend::Auto)  // Auto-select best available
    .build()?;

// Operations run on GPU when available
processor.apply_lut3d(&mut img, &lut, 33)?;
processor.blur(&mut img, 5.0)?;
processor.composite_over(&fg, &mut bg)?;
```

**Key features:**

| Feature | Description |
|---------|-------------|
| **Auto backend selection** | Automatically picks best GPU: CUDA > wgpu (discrete) > wgpu (integrated) > CPU |
| **Automatic tiling** | Large images split into tiles based on VRAM limits, stitched transparently |
| **VRAM detection** | Cross-platform GPU memory detection (DXGI, Metal, NVML, sysfs) |
| **Operation fusion** | Sequential color ops merged into single GPU pass when possible |

**Supported backends:**
- **CPU** - Always available, parallel via rayon
- **wgpu** - Vulkan, Metal, DX12 (feature: `wgpu`)
- **CUDA** - NVIDIA GPUs (feature: `cuda`)

## CLI Reference

| Command | OIIO Equivalent | Description |
|---------|-----------------|-------------|
| `vfx info` | `iinfo` | Image metadata and statistics |
| `vfx convert` | `iconvert` | Format conversion |
| `vfx diff` | `idiff` | Image comparison |
| `vfx maketx` | `maketx` | Texture creation |
| `vfx resize` | `oiiotool --resize` | Image scaling |
| `vfx crop` | `oiiotool --crop` | Region extraction |
| `vfx composite` | `oiiotool --over` | Alpha compositing |
| `vfx color` | - | Color transforms |
| `vfx aces` | - | ACES IDT/RRT/ODT |
| `vfx lut` | - | LUT application |
| `vfx layers` | - | List EXR layers |
| `vfx view` | `iv` | Image viewer |
| `vfx batch` | - | Batch processing |

## Building from Source

```bash
git clone https://github.com/vfx-rs/vfx-rs
cd vfx-rs
cargo build --release

# With GPU support
cargo build --release --features wgpu
cargo build --release --features cuda  # Requires CUDA toolkit
```

## Documentation

Full documentation available via mdbook:

```bash
cd docs
mdbook serve
# Open http://localhost:3000
```

**Contents:**
- User Guide - Quick start, CLI reference
- ACES Workflows - Understanding and using ACES
- Programmer Guide - Library API reference
- Crate Reference - Individual crate documentation
- Internals - Architecture and implementation details

## Design Philosophy

1. **Correctness over speed** - Color math must be bit-accurate
2. **VFX-first** - Designed for film/TV, not web images
3. **Composable** - Small crates that do one thing well
4. **Observable** - Structured logging with `tracing`
5. **Documented** - Every public API has examples

## Contributing

Contributions welcome! Areas that need work:

- [ ] ImageBufAlgo functions (resize, composite done; 50+ more to go)
- [ ] Deep data support for EXR
- [ ] OCIO GradingTransform parsing
- [ ] More LUT formats (.3dl, .look)
- [ ] Additional image formats (ARRIRAW, REDCODE)

## License

Dual-licensed under MIT and Apache 2.0. Choose whichever suits your needs.

---

*Built with Rust for the VFX industry.*
