# Core API

The foundational types for working with images in vfx-rs.

## ImageData

The primary image container, storing pixel data with metadata.

### Creating Images

```rust
use vfx_core::ImageData;

// Create empty image
let image = ImageData::new(1920, 1080, 4);  // RGBA

// Create with fill color
let red = ImageData::constant(100, 100, 3, &[1.0, 0.0, 0.0]);

// From raw pixel data
let pixels = vec![0.5f32; 1920 * 1080 * 3];
let image = ImageData::from_f32(&pixels, 1920, 1080, 3)?;
```

### Accessing Pixels

```rust
// Get pixel at (x, y)
let pixel = image.get_pixel(100, 50);

// Set pixel at (x, y)
image.set_pixel(100, 50, &[1.0, 0.5, 0.25, 1.0]);

// Iterate all pixels
for y in 0..image.height() {
    for x in 0..image.width() {
        let pixel = image.get_pixel(x, y);
        // Process pixel...
    }
}

// Access raw buffer (fastest)
let buffer = image.as_f32_slice();
```

### Properties

```rust
let image: ImageData = ...;

image.width()       // Image width in pixels
image.height()      // Image height in pixels
image.channels()    // Number of channels (3=RGB, 4=RGBA)
image.pixel_count() // Total pixels (width * height)
image.byte_size()   // Total buffer size in bytes
image.spec()        // ImageSpec with metadata
```

### Conversion

```rust
// Clone image
let copy = image.clone();

// Create from ImageSpec
let spec = ImageSpec::new(1920, 1080, 4);
let image = ImageData::from_spec(&spec);
```

## ImageSpec

Metadata container compatible with OpenImageIO.

### Creating Specs

```rust
use vfx_core::ImageSpec;

// Basic spec
let spec = ImageSpec::new(1920, 1080, 4);

// With channel names
let spec = ImageSpec::with_channels(1920, 1080, &["R", "G", "B", "A"]);

// Copy from existing image
let spec = image.spec().clone();
```

### Properties

```rust
let spec: ImageSpec = ...;

// Resolution
spec.width()
spec.height()
spec.depth()  // For 3D volumes

// Display/data window
spec.full_width()    // Display window width
spec.full_height()   // Display window height
spec.x()             // Data window x offset
spec.y()             // Data window y offset

// Channels
spec.channels()
spec.channel_names()

// Tile info
spec.tile_width()
spec.tile_height()

// Format
spec.format()        // Pixel data type
```

### Attributes

```rust
// Set attribute
spec.set_attribute("author", "VFX Artist");
spec.set_attribute("compression", "piz");
spec.set_attribute("framesPerSecond", 24.0);

// Get attribute
let author: Option<&str> = spec.get_string("author");
let fps: Option<f64> = spec.get_float("framesPerSecond");

// Iterate attributes
for (key, value) in spec.attributes() {
    println!("{}: {:?}", key, value);
}
```

### Standard Attributes

| Attribute | Type | Description |
|-----------|------|-------------|
| `compression` | String | EXR compression (piz, zip, dwaa) |
| `oiio:ColorSpace` | String | Color space name |
| `chromaticities` | Float[8] | Color primaries |
| `author` | String | Creator name |
| `DateTime` | String | Creation timestamp |
| `framesPerSecond` | Rational | Frame rate |
| `worldtocamera` | Matrix | Camera transform |

## ChannelType

Semantic classification of image channels.

### Types

```rust
use vfx_core::ChannelType;

pub enum ChannelType {
    Color,      // RGB, RGBA color data
    Alpha,      // Transparency
    Depth,      // Z-depth
    Normal,     // Surface normals (N.x, N.y, N.z)
    Position,   // World position (P.x, P.y, P.z)
    Velocity,   // Motion vectors
    Id,         // Object/material IDs
    Mask,       // Binary masks
    Generic,    // Unclassified
}
```

### Classification

```rust
use vfx_core::classify_channel;

let channel_type = classify_channel("R");        // Color
let channel_type = classify_channel("A");        // Alpha
let channel_type = classify_channel("Z");        // Depth
let channel_type = classify_channel("N.x");      // Normal
let channel_type = classify_channel("crypto");   // Id
```

### Usage

```rust
// Check if operation should process channel
fn should_process(channel: &str) -> bool {
    match classify_channel(channel) {
        ChannelType::Color | ChannelType::Alpha => true,
        ChannelType::Id | ChannelType::Mask => false,
        _ => false,
    }
}
```

## Error Handling

```rust
use vfx_core::{CoreError, CoreResult};

fn process_image(path: &Path) -> CoreResult<ImageData> {
    let image = vfx_io::read(path)?;

    if image.channels() < 3 {
        return Err(CoreError::InvalidFormat(
            "Need at least 3 channels".into()
        ));
    }

    Ok(image)
}
```

### Error Types

```rust
pub enum CoreError {
    InvalidDimensions(u32, u32),
    InvalidChannels(usize),
    InvalidFormat(String),
    BufferSizeMismatch { expected: usize, actual: usize },
    IoError(std::io::Error),
}
```

## Thread Safety

ImageData is `Send + Sync` and can be safely shared across threads:

```rust
use rayon::prelude::*;

// Parallel pixel processing
let pixels: Vec<_> = (0..image.height())
    .into_par_iter()
    .flat_map(|y| {
        (0..image.width()).map(move |x| (x, y))
    })
    .map(|(x, y)| {
        let pixel = image.get_pixel(x, y);
        process_pixel(pixel)
    })
    .collect();
```
