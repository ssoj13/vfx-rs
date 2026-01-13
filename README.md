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
- **Deep data support** - Full OpenEXR deep compositing via vfx-exr
- **GPU acceleration** - wgpu (Vulkan/Metal/DX12) and CUDA backends
- **Python bindings** - PyO3 for pipeline integration
- **Cross-platform** - Same code runs on Windows, Linux, macOS

## Feature Overview

| Category | Features |
|----------|----------|
| **EXR** | Deep data, multi-layer, mip/rip maps, tiled, all compression (except DWA) |
| **Formats** | EXR, PNG, JPEG, TIFF, DPX, HDR, WebP, HEIF, PSD (read), TX |
| **Color** | sRGB, Rec.709, Rec.2020, DCI-P3, ACEScg, ACES2065-1 |
| **Transfer Functions** | sRGB, PQ, HLG, LogC3, LogC4, S-Log2/3, V-Log, Canon Log 2/3, Apple Log, ACEScc/cct, REDLog |
| **LUTs** | .cube, .clf, .spi1d/.spi3d, .csp, .cdl, 15 formats total |
| **Operations** | Resize, Crop, Rotate, Flip, Blur, Sharpen, Composite, Grade, FFT |
| **ACES** | IDT, RRT, ODT, LMT transforms |
| **I/O** | Streaming read/write, tiled caching, multi-layer EXR, deep data |
| **GPU** | Auto backend selection, automatic tiling, operation fusion |

## Quick Start

```bash
# Install CLI
cargo install vfx-cli

# Get image info (works with deep EXR)
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

## Deep EXR Support

vfx-rs includes **vfx-exr**, a full fork of [exrs](https://github.com/johannesvollmer/exrs) with complete deep data support:

```rust
use vfx_io::exr_deep::{read_exr_deep, write_exr_deep, DeepImage};

// Read deep EXR file
let deep_image: DeepImage = read_exr_deep("deep_render.exr")?;

// Access deep samples per pixel
for pixel in deep_image.layer.channel_data.deep_samples.pixels() {
    println!("Pixel has {} samples", pixel.sample_count);
    for i in 0..pixel.sample_count {
        let depth = pixel.get_f32(0, i);  // Z channel
        let alpha = pixel.get_f32(1, i);  // A channel
    }
}

// Write deep EXR
write_exr_deep("output_deep.exr", &deep_image)?;
```

### Deep Data Features

| Feature | Status |
|---------|--------|
| Deep scanline read | Production-ready |
| Deep scanline write | Production-ready |
| Variable samples per pixel | Full support |
| All sample types (f16, f32, u32) | Full support |
| ZIP/ZIPS compression | Full support |
| Multi-layer deep | Full support |

## Library Usage

```rust
use vfx_io::{read, write};
use vfx_ops::resize;
use vfx_color::ColorProcessor;

fn main() -> anyhow::Result<()> {
    // Read any supported format (including deep EXR)
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
├── vfx-exr         # OpenEXR I/O with deep data (fork of exrs)
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

### vs OpenImageIO (~60%)

| Feature | Status |
|---------|--------|
| Image I/O (11 formats) | **Production-ready** |
| Multi-layer EXR | **Production-ready** |
| Deep data EXR | **Production-ready** - full read/write support |
| Streaming I/O | **Production-ready** - progressive read/write for large files |
| Tiled image caching | **Production-ready** - on-demand tile loading |
| UDIM textures | **Production-ready** |
| ImageBufAlgo (100+ functions) | Partial |
| Plugin system | Not implemented |

### vs OpenColorIO (~98%)

| Feature | Status |
|---------|--------|
| Config YAML parsing | **Production-ready** |
| ColorSpace definitions | **Production-ready** |
| Aliases & Categories | **Production-ready** |
| Roles, Displays, Views | **Production-ready** |
| Shared Views (v2.3+) | **Production-ready** |
| Context variables | **Production-ready** |
| Matrix/CDL/Log/Range transforms | **Production-ready** |
| BuiltinTransform | **Production-ready** - 20+ ACES styles |
| FileTransform (LUTs) | **Production-ready** - .cube, .spi1d, .spi3d, .clf, .ctf |
| GradingPrimary/Tone/Curve | **Production-ready** |
| Baker (LUT export) | **Production-ready** |
| DynamicProcessor | **Production-ready** |
| ProcessorCache | **Production-ready** |

### OCIO Algorithm Parity

Numerical accuracy verified against OpenColorIO 2.5.1:

| Component | Max Diff | Max ULP | Notes |
|-----------|----------|---------|-------|
| LUT3D Index | 0 | 0 | Blue-major order (`B + dim*G + dim²*R`) |
| LUT3D Tetrahedral | 1.19e-07 | - | All 6 tetrahedra match OCIO |
| LUT3D Trilinear | 0.0 | 0 | Perfect match (B→G→R order) |
| CDL (power=1.0) | 0.0 | 0 | Bit-perfect |
| CDL (power≠1.0) | 2.98e-07 | 8-22 | Uses OCIO-identical `fast_pow` |
| sRGB | 2.41e-05 | - | f32 precision |
| PQ (ST-2084) | 2.74e-06 | - | Relative error |
| HLG | 6.66e-16 | - | Machine epsilon |
| Canon Log 2/3 | 4.20e-05 | - | Analytical vs OCIO LUT |

**Key implementation details:**
- `fast_pow` uses identical Chebyshev polynomial coefficients as OCIO SSE.h
- CDL saturation uses OCIO-compatible operation order (multiply then sum)
- All constants verified against OCIO source code

See [docs/OCIO_PARITY_AUDIT.md](docs/OCIO_PARITY_AUDIT.md) for full details.

### EXR Compression Support

| Method | Read | Write | Type |
|--------|------|-------|------|
| None | Yes | Yes | Uncompressed |
| RLE | Yes | Yes | Lossless |
| ZIP | Yes | Yes | Lossless |
| ZIPS | Yes | Yes | Lossless |
| PIZ | Yes | Yes | Lossless (wavelet) |
| PXR24 | Yes | Yes | Lossless (f16/u32) |
| B44 | Yes | Yes | Lossy (fixed) |
| B44A | Yes | Yes | Lossy (adaptive) |
| DWAA | No | No | Help wanted |
| DWAB | No | No | Help wanted |

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
| `vfx view` | `iv` | Image viewer with pixel inspector |
| `vfx batch` | - | Batch processing |
| `vfx grade` | - | CDL grading (slope/offset/power/sat) |
| `vfx clamp` | - | Clamp pixel values |
| `vfx premult` | - | Alpha premultiplication |

## Building from Source

```bash
git clone https://github.com/vfx-rs/vfx-rs
cd vfx-rs
cargo build --release

# With GPU support
cargo build --release --features wgpu
cargo build --release --features cuda  # Requires CUDA toolkit

# With EXR viewer
cargo build --release -p vfx-exr --features view
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
6. **Safe** - No unsafe code in core crates (`#[forbid(unsafe_code)]`)

## Contributing

Contributions welcome! Areas that need work:

- [ ] DWA/DWAB compression for EXR
- [ ] More ImageBufAlgo functions
- [ ] OCIO GradingTransform parsing
- [ ] Additional image formats (ARRIRAW, REDCODE)
- [ ] Deep data compositing operations

## License

Dual-licensed under MIT and Apache 2.0. Choose whichever suits your needs.

vfx-exr is licensed under BSD-3-Clause (inherited from exrs).

---

*Built with Rust for the VFX industry.*
