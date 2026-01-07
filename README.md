# vfx-rs

High-performance image processing library for VFX pipelines, written in Rust.

A modern alternative to OpenImageIO with GPU acceleration, streaming tiled processing, and Python bindings.

## Features

- **GPU Compute** — wgpu/CUDA backends for accelerated image operations
- **Streaming Tiles** — memory-efficient processing of large images
- **OIIO-compatible API** — ImageBuf, ImageSpec, imagebufalgo functions
- **Color Management** — ACES, OCIO, ICC profiles, LUT support
- **Format Support** — EXR, PNG, JPEG, TIFF, DPX, and more
- **Python Bindings** — PyO3-based module for pipeline integration
- **CLI Tools** — convert, resize, maketx, color transforms, compositing

## Crates

| Crate | Description |
|-------|-------------|
| `vfx-core` | Base types: ImageSpec, DataFormat, ROI |
| `vfx-io` | Image I/O, ImageBuf, imagebufalgo |
| `vfx-compute` | GPU backends (CPU/wgpu/CUDA) |
| `vfx-color` | Color space transforms |
| `vfx-ocio` | OpenColorIO integration |
| `vfx-cli` | Command-line tools |
| `vfx-rs-py` | Python bindings |

## Quick Start

```bash
# Build
cargo build --release

# CLI usage
vfx convert input.exr output.png
vfx resize input.exr -s 1920x1080 output.exr
vfx maketx input.exr output.tx

# Run tests
cargo test
```

## Rust API

```rust
use vfx_io::{ImageBuf, ImageSpec, InitializePixels};
use vfx_io::imagebufalgo::{resize, rotate, composite_over};
use vfx_compute::{ImageProcessor, Backend};

// Load image
let img = ImageBuf::open("input.exr")?;

// GPU-accelerated resize
let processor = ImageProcessor::new(Backend::Auto)?;
let resized = processor.resize(&img, 1920, 1080, ResizeFilter::Lanczos3)?;

// Save
resized.write("output.exr")?;
```

## Python API

```python
import vfx

# Load and process
img = vfx.ImageBuf("input.exr")
resized = vfx.resize(img, 1920, 1080)
resized.write("output.exr")

# Color transform
vfx.colorconvert(img, "ACEScg", "sRGB")
```

## Building

Requirements:
- Rust 1.85+
- CMake (for some dependencies)

```bash
# Full build with all features
cargo build --release --all-features

# Python module
cd crates/vfx-rs-py
maturin develop --release
```

## License

MIT OR Apache-2.0
