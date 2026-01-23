# Core API

The foundational types for working with images in vfx-rs.

## ImageData (vfx-io)

The primary image container, storing pixel data with metadata.

**Note:** `ImageData` is defined in `vfx_io`, not `vfx_core`.

### Creating Images

```rust
use vfx_io::{ImageData, PixelFormat};

// Create empty image with specific format
let image = ImageData::new(1920, 1080, 4, PixelFormat::F32);

// From raw f32 pixel data
let pixels = vec![0.5f32; 1920 * 1080 * 3];
let image = ImageData::from_f32(1920, 1080, 3, pixels);

// From u8 pixel data
let pixels = vec![128u8; 1920 * 1080 * 3];
let image = ImageData::from_u8(1920, 1080, 3, pixels);
```

### Structure

```rust
pub struct ImageData {
    pub width: u32,
    pub height: u32,
    pub channels: u32,
    pub format: PixelFormat,    // U8, U16, U32, F16, F32
    pub data: PixelData,        // The actual pixel buffer
    pub metadata: Metadata,     // Colorspace, DPI, attrs
}
```

### Data Access

```rust
// Get as f32 Vec (converts if needed)
let data: Vec<f32> = image.to_f32();

// Get as u8 Vec (converts if needed)
let data: Vec<u8> = image.to_u8();

// Access raw data enum directly
match &image.data {
    PixelData::F32(vec) => { /* work with Vec<f32> */ }
    PixelData::U8(vec) => { /* work with Vec<u8> */ }
    PixelData::U16(vec) => { /* work with Vec<u16> */ }
    PixelData::U32(vec) => { /* work with Vec<u32> */ }
}

// Properties
let pixels = image.pixel_count();    // width * height
let samples = image.sample_count();  // width * height * channels
```

### Conversion

```rust
// Convert pixel format
let f32_image = image.convert_to(PixelFormat::F32);
let u8_image = image.convert_to(PixelFormat::U8);

// Create layer for EXR output
let layer = image.to_layer("beauty");
let layered = image.to_layered("rgba");
```

## vfx-core Types

The `vfx_core` crate provides generic typed image containers and color science primitives.

### Image<C, T, N>

A strongly-typed image container with compile-time color space and format.

```rust
use vfx_core::{Image, Linear, Srgb, ColorSpace};

// Create typed image
let img: Image<Linear, f32, 3> = Image::new(1920, 1080);

// Access dimensions
let width = img.width();
let height = img.height();

// Convert color space
let srgb_img: Image<Srgb, f32, 3> = img.convert_colorspace();
```

### ColorSpace

Marker types for color spaces:

```rust
use vfx_core::{Linear, Srgb, AcesCg, Rec2020, DciP3};

// Each implements the ColorSpace trait
trait ColorSpace: Clone + Default + Send + Sync + 'static {
    const NAME: &'static str;
}
```

### PixelFormat Trait

Trait for pixel data types:

```rust
use vfx_core::PixelFormat;

// Implemented for: u8, u16, f16 (half), f32
trait PixelFormat: Copy + Clone + Default + Send + Sync + 'static {
    const BITS: u32;
    const IS_FLOAT: bool;
    const MAX_VALUE: f32;

    fn to_f32(self) -> f32;
    fn from_f32(v: f32) -> Self;
}
```

### Rgb / Rgba

Typed pixel values:

```rust
use vfx_core::{Rgb, Rgba, Linear};

let color: Rgb<Linear, f32> = Rgb::new(0.5, 0.3, 0.2);
let rgba: Rgba<Linear, f32> = Rgba::new(0.5, 0.3, 0.2, 1.0);

// Component access
let r = color.r;
let g = color.g;
let b = color.b;
```

## Error Handling

```rust
use vfx_io::{IoError, IoResult};

fn process_image(path: &Path) -> IoResult<ImageData> {
    let image = vfx_io::read(path)?;

    if image.channels < 3 {
        return Err(IoError::InvalidChannelCount(image.channels));
    }

    Ok(image)
}
```

### vfx-core Errors

```rust
use vfx_core::{Error, Result};

pub enum Error {
    InvalidDimensions { width: u32, height: u32 },
    InvalidChannels(usize),
    OutOfBounds { x: u32, y: u32 },
    // ...
}
```

## Thread Safety

`ImageData` is `Send + Sync` and can be safely shared across threads:

```rust
use rayon::prelude::*;

let data = image.to_f32();
let width = image.width as usize;
let channels = image.channels as usize;

// Parallel row processing
let results: Vec<_> = data
    .par_chunks(width * channels)
    .map(|row| process_row(row))
    .collect();
```
