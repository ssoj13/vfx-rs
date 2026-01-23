# Architecture

vfx-rs is a modular Rust workspace consisting of 17 crates, designed as a native alternative to OpenColorIO and OpenImageIO. This chapter covers the high-level architecture, crate organization, and key design decisions.

## Design Philosophy

### Native Rust, No Bindings

Unlike many Rust graphics libraries that wrap C/C++ libraries, vfx-rs is written in pure Rust (with few exceptions for ICC profiles and optional GPU backends). This provides:

- **Memory safety** - no segfaults, no buffer overflows
- **Fearless concurrency** - Rayon-powered parallel processing with compile-time guarantees
- **Zero-cost abstractions** - generic code that compiles to optimal machine code
- **Easy cross-compilation** - build for any target Rust supports

### Layered Crate Design

Crates are organized in dependency layers:

```
Layer 4 (Apps):     vfx-cli, vfx-view, vfx-rs-py
                         |
Layer 3 (High):     vfx-ops, vfx-color, vfx-ocio, vfx-icc
                         |
Layer 2 (Mid):      vfx-io, vfx-compute, vfx-lut, vfx-transfer, vfx-primaries
                         |
Layer 1 (Base):     vfx-core, vfx-math
```

Lower layers never depend on higher layers. This enables:

- Use `vfx-io` alone for image I/O without color management
- Use `vfx-color` for color math without the CLI
- Embed only the crates you need in your application

### OIIO/OCIO Functional Parity

The goal is to cover the same functionality as:

| OpenImageIO | vfx-rs equivalent |
|-------------|-------------------|
| `ImageBuf` | `vfx_io::ImageData` |
| `ImageSpec` | `vfx_core::ImageSpec` |
| `oiiotool` | `vfx-cli` (`vfx` binary) |
| `iinfo` | `vfx info` |
| `iconvert` | `vfx convert` |

| OpenColorIO | vfx-rs equivalent |
|-------------|-------------------|
| `Config` | `vfx_ocio::Config` |
| `ColorSpace` | `vfx_core::ColorSpace` |
| `Processor` | Direct function calls |
| `ociocheck` | `vfx-ocio` validation |

## Crate Categories

### Foundation (Layer 1)

- **vfx-core** - Core types: `ImageSpec`, `ColorSpace`, `PixelFormat`, `Error`
- **vfx-math** - Matrices, interpolation, color math utilities

### Color Science (Layer 2)

- **vfx-transfer** - Transfer functions (OETF/EOTF): sRGB, Rec.709, PQ, HLG, ACEScct
- **vfx-primaries** - Color primaries and white points: sRGB, DCI-P3, Rec.2020, ACES
- **vfx-lut** - LUT types and parsers: 1D/3D LUTs, .cube, .clf, .csp formats

### I/O and Compute (Layer 2)

- **vfx-io** - Image formats: EXR, PNG, JPEG, TIFF, DPX, HDR, WebP, AVIF, HEIF
- **vfx-compute** - Compute backends: CPU (Rayon), GPU (wgpu), CUDA

### Color Management (Layer 3)

- **vfx-color** - Unified color transforms: gamut mapping, ACES, tone mapping
- **vfx-ocio** - OCIO config parsing and color space transformations
- **vfx-icc** - ICC profile support via lcms2

### Image Processing (Layer 3)

- **vfx-ops** - Image operations: resize, blur, sharpen, composite, convolve

### Applications (Layer 4)

- **vfx-cli** - Command-line tool (`vfx` binary)
- **vfx-view** - Interactive image viewer with OCIO support
- **vfx-rs-py** - Python bindings via PyO3

### Testing and Benchmarks

- **vfx-tests** - Integration tests
- **vfx-bench** - Performance benchmarks

## Feature Flags

Most crates use feature flags for optional functionality:

```toml
# vfx-io: Format support
vfx-io = { version = "0.1", features = ["exr", "png", "jpeg"] }

# vfx-compute: GPU backends
vfx-compute = { version = "0.1", features = ["wgpu", "cuda"] }

# vfx-color: GPU-accelerated color
vfx-color = { version = "0.1", features = ["gpu"] }
```

This keeps binaries small and compilation fast when full functionality isn't needed.

## Next Steps

- [Crate Graph](crate-graph.md) - Visual dependency diagram
- [Data Flow](data-flow.md) - How data moves through the system
- [Design Decisions](decisions.md) - Key architectural choices and tradeoffs
