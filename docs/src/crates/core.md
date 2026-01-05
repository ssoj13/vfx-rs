# vfx-core

Core types for VFX image and color processing.

## Purpose

`vfx-core` is the foundation crate with zero internal dependencies. All other vfx-rs crates depend on it.

## Key Types

### ColorSpace

Compile-time color space tracking using marker types:

```rust
use vfx_core::prelude::*;

// Type-safe color spaces
let srgb: Rgb<Srgb> = Rgb::new(0.5, 0.3, 0.2);
let aces: Rgb<AcesCg> = Rgb::new(0.18, 0.18, 0.18);

// Compile error: can't mix color spaces
// let bad = srgb + aces;
```

Available color spaces:
- `Srgb`, `LinearSrgb` - Standard RGB
- `Rec709`, `Rec2020` - Broadcast standards
- `DciP3`, `DisplayP3` - Cinema and Apple displays
- `Aces2065`, `AcesCg`, `AcesCc`, `AcesCct` - ACES family

### Image

Zero-copy image buffer with color space awareness:

```rust
use vfx_core::{Image, ImageView, ImageViewMut};

// Create 1920x1080 RGB image
let mut img: Image<Srgb, f32> = Image::new(1920, 1080);

// Immutable view
let view: ImageView<Srgb, f32> = img.view();

// Mutable view
let mut_view: ImageViewMut<Srgb, f32> = img.view_mut();
```

### Pixel Types

Generic RGB/RGBA with color space tracking:

```rust
use vfx_core::prelude::*;

let rgb = Rgb::<Srgb>::new(1.0, 0.5, 0.25);
let rgba = Rgba::<AcesCg>::new(0.18, 0.18, 0.18, 1.0);

// Access components
println!("Red: {}", rgb.r);
println!("Alpha: {}", rgba.a);
```

### Rect and Roi

Region of interest types:

```rust
use vfx_core::{Rect, Roi};

// Rectangle with origin and size
let rect = Rect::new(100, 100, 800, 600);

// Region of interest (bounds)
let roi = Roi::new(100, 100, 900, 700);
```

### Error Handling

Standard error types used throughout vfx-rs:

```rust
use vfx_core::{Error, Result};

fn process() -> Result<()> {
    // Errors propagate with ?
    Ok(())
}
```

## Design Philosophy

The key principle is **compile-time color space safety**:

```rust
// This compiles - explicit conversion
let linear = srgb_to_linear(srgb_value);

// This doesn't compile - no implicit mixing
// let bad = srgb_image.blend(aces_image);
```

Benefits:
- Catches color space bugs at compile time
- No runtime overhead for color space tracking
- Self-documenting code

## DataFormat

Pixel data formats for I/O:

```rust
use vfx_core::DataFormat;

match format {
    DataFormat::U8 => println!("8-bit"),
    DataFormat::U16 => println!("16-bit"),
    DataFormat::F16 => println!("16-bit float"),
    DataFormat::F32 => println!("32-bit float"),
    DataFormat::U32 => println!("32-bit unsigned"),
}
```

## Dependencies

External:
- `half` - f16 float support
- `thiserror` - error types
- `rayon` - optional parallelism

## Used By

All other vfx-rs crates depend on vfx-core.
