# Programmer Guide

This guide covers using vfx-rs as a library for building VFX applications.

## Overview

vfx-rs provides a modular architecture for image processing:

```
+---------------------------------------------------------------------+
|                         Application Layer                           |
+---------------------------------------------------------------------+
|  vfx-cli    |   vfx-view   |   vfx-rs-py   |   Your App           |
+---------------------------------------------------------------------+
|                         Operations Layer                            |
+---------------------------------------------------------------------+
|  vfx-ops    |   vfx-compute   |   vfx-color   |   vfx-ocio         |
+---------------------------------------------------------------------+
|                         Core Layer                                  |
+---------------------------------------------------------------------+
|  vfx-io     |   vfx-core   |   vfx-lut   |   vfx-icc   |   vfx-math|
+---------------------------------------------------------------------+
|                      Foundation Layer                               |
+---------------------------------------------------------------------+
|  vfx-primaries   |   vfx-transfer   |                               |
+---------------------------------------------------------------------+
```

## Getting Started

### Add Dependencies

```toml
[dependencies]
vfx-io = "0.1"
vfx-core = "0.1"
vfx-color = "0.1"
vfx-ops = "0.1"
```

### Basic Example

```rust
use vfx_io::{read, write, ImageData};
use vfx_ops::{resize_f32, Filter};

fn main() -> anyhow::Result<()> {
    // Read image
    let image = read("input.exr")?;

    // Resize to half (resize_f32 operates on pixel buffers)
    let src_data = image.to_f32();
    let resized_data = resize_f32(
        &src_data,
        image.width as usize,
        image.height as usize,
        image.channels as usize,
        960, 540,
        Filter::Bilinear
    )?;
    let resized = ImageData::from_f32(960, 540, image.channels as usize, resized_data);

    // Write output
    write("output.exr", &resized)?;

    Ok(())
}
```

## Key Concepts

### ImageData

The core image container (in `vfx_io`):

```rust
use vfx_io::{ImageData, PixelFormat};

// Create with specific format
let image = ImageData::new(1920, 1080, 4, PixelFormat::F32);

// Or create from data
let data: Vec<f32> = vec![0.0; 1920 * 1080 * 4];
let image = ImageData::from_f32(1920, 1080, 4, data);

// Access public fields directly
println!("{}x{} with {} channels",
    image.width,
    image.height,
    image.channels
);

// Get pixel data
let pixels = image.to_f32();
```

### ImageSpec

Metadata container (in `vfx_core`):

```rust
use vfx_core::{ImageSpec, DataFormat};

// Create with dimensions, channels, and format
let spec = ImageSpec::new(1920, 1080, 4, DataFormat::F32);

// Or use convenience constructors
let spec = ImageSpec::rgba(1920, 1080);  // RGBA F32
let spec = ImageSpec::rgb(1920, 1080);   // RGB F32

// Set colorspace
let mut spec = ImageSpec::rgba(1920, 1080);
spec.set_colorspace("ACEScg");

// Access metadata via extra_attribs field
// spec.extra_attribs is a HashMap<String, AttrValue>
```

### PixelFormat and DataFormat

Pixel type enumerations:

```rust
use vfx_io::PixelFormat;      // In vfx_io
use vfx_core::DataFormat;     // In vfx_core

// PixelFormat (for ImageData)
match format {
    PixelFormat::U8 => // 8-bit unsigned
    PixelFormat::U16 => // 16-bit unsigned
    PixelFormat::U32 => // 32-bit unsigned
    PixelFormat::F16 => // 16-bit float
    PixelFormat::F32 => // 32-bit float
}

// DataFormat (for ImageSpec)
match format {
    DataFormat::U8 => // 8-bit unsigned
    DataFormat::U16 => // 16-bit unsigned
    DataFormat::U32 => // 32-bit unsigned
    DataFormat::F16 => // 16-bit float
    DataFormat::F32 => // 32-bit float
}
```

## Crate Overview

| Crate | Purpose |
|-------|---------|
| `vfx-io` | File I/O, ImageData, format support |
| `vfx-core` | Core types: Image<C,T,N>, ImageSpec, ColorSpace |
| `vfx-ops` | Image operations (resize, composite, etc.) |
| `vfx-color` | Color transforms, ACES |
| `vfx-compute` | GPU acceleration |
| `vfx-ocio` | OpenColorIO integration |
| `vfx-lut` | LUT loading/application |
| `vfx-icc` | ICC profile support |
| `vfx-transfer` | Transfer functions (gamma, log, PQ) |
| `vfx-primaries` | Color space primaries and matrices |
| `vfx-math` | Math utilities |

## Documentation Structure

- **[Core API](./core-api.md)** - ImageData, ImageSpec, types
- **[ImageBufAlgo](./imagebufalgo/README.md)** - OIIO-compatible operations
- **[Color Management](./color-management.md)** - Color spaces and transforms
- **[OCIO Integration](./ocio-integration.md)** - OpenColorIO workflows
- **[GPU Compute](./gpu-compute.md)** - Hardware acceleration

## Next Steps

1. Start with [Core API](./core-api.md) to understand data types
2. See [ImageBufAlgo Reference](./imagebufalgo/README.md) for operations
3. Read [Color Management](./color-management.md) for ACES workflows
