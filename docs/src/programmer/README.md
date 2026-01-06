# Programmer Guide

This guide covers using vfx-rs as a library for building VFX applications.

## Overview

vfx-rs provides a modular architecture for image processing:

```
┌─────────────────────────────────────────────────────────────────────┐
│                         Application Layer                           │
├─────────────────────────────────────────────────────────────────────┤
│  vfx-cli    │   vfx-view   │   vfx-rs-py   │   Your App           │
├─────────────────────────────────────────────────────────────────────┤
│                         Operations Layer                            │
├─────────────────────────────────────────────────────────────────────┤
│  vfx-ops    │   vfx-compute   │   vfx-color   │   vfx-ocio         │
├─────────────────────────────────────────────────────────────────────┤
│                         Core Layer                                  │
├─────────────────────────────────────────────────────────────────────┤
│  vfx-io     │   vfx-core   │   vfx-lut   │   vfx-icc   │   vfx-math│
├─────────────────────────────────────────────────────────────────────┤
│                      Foundation Layer                               │
├─────────────────────────────────────────────────────────────────────┤
│  vfx-primaries   │   vfx-transfer   │                               │
└─────────────────────────────────────────────────────────────────────┘
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
use vfx_io::{read, write};
use vfx_ops::resize;

fn main() -> anyhow::Result<()> {
    // Read image
    let mut image = read("input.exr")?;

    // Resize to half
    let resized = resize(&image, 960, 540)?;

    // Write output
    write("output.exr", &resized)?;

    Ok(())
}
```

## Key Concepts

### ImageData

The core image container:

```rust
use vfx_core::ImageData;

let image = ImageData::new(1920, 1080, 4);  // RGBA
println!("{}x{} with {} channels",
    image.width(),
    image.height(),
    image.channels()
);
```

### ImageSpec

Metadata container (OIIO-compatible):

```rust
use vfx_core::ImageSpec;

let mut spec = ImageSpec::new(1920, 1080, 4);
spec.set_attribute("author", "VFX Artist");
spec.set_attribute("compression", "piz");
```

### Channel Types

Semantic channel classification:

```rust
use vfx_core::ChannelType;

match channel_type {
    ChannelType::Color => // RGB, RGBA
    ChannelType::Alpha => // Transparency
    ChannelType::Depth => // Z-depth
    ChannelType::Normal => // Surface normals
    ChannelType::Id => // Object/material IDs
    ChannelType::Mask => // Binary masks
    ChannelType::Generic => // Unclassified
}
```

## Crate Overview

| Crate | Purpose |
|-------|---------|
| `vfx-core` | Core types: ImageData, ImageSpec, ChannelType |
| `vfx-io` | File I/O, format support |
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
