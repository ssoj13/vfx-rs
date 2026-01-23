# ImageBufAlgo Reference

ImageBufAlgo provides OIIO-compatible image processing operations.

## Module Overview

| Module | Operations |
|--------|------------|
| [Patterns](./patterns.md) | fill, checker, noise, zero |
| [Channels](./channels.md) | channel_append, channels, extract_channel |
| [Geometry](./geometry.md) | crop, resize, rotate, flip, flop, warp |
| [Arithmetic](./arithmetic.md) | add, sub, mul, div, pow, abs, clamp |
| [Color](./color.md) | premult, unpremult, saturate, srgb_to_linear |
| [Composite](./composite.md) | over, add_blend, multiply, screen |
| [Statistics](./stats.md) | compute_pixel_stats, is_constant_color |
| [Filters](./filters.md) | blur, sharpen, median, dilate, erode |
| [Deep](./deep.md) | deep_merge, flatten_deep |

## Usage Pattern

All ImageBufAlgo operations follow consistent patterns:

### Creating New Images

```rust
use vfx_io::ImageBuf;
use vfx_io::imagebufalgo;

let image = ImageBuf::from_file("input.exr")?;

// Create result in new buffer
let blurred = imagebufalgo::blur(&image, 5.0, None);
```

### In-Place Operations

```rust
// In-place modification (use _into variants)
let src = ImageBuf::from_file("input.exr")?;
let mut dst = ImageBuf::new(spec);
imagebufalgo::blur_into(&mut dst, &src, 2.0, None);
```

### Arithmetic with Constants

```rust
// Add constant to image
let brightened = imagebufalgo::add(&image, &[0.1, 0.1, 0.1, 0.0], None);

// Multiply by constant
let scaled = imagebufalgo::mul(&image, &[2.0, 2.0, 2.0, 1.0], None);
```

## Quick Examples

### Pattern Generation

```rust
use vfx_io::imagebufalgo;
use vfx_io::imagebufalgo::{Roi3D, NoiseType};

// Create solid color fill (roi defines dimensions)
let roi = Roi3D::from_xywh(0, 0, 100, 100, 3);
let red = imagebufalgo::fill(&[1.0, 0.0, 0.0], roi);

// Create checker pattern
let roi = Roi3D::from_xywh(0, 0, 256, 256, 3);
let checker = imagebufalgo::checker(
    16, 16, 1,                    // check_width, check_height, check_depth
    &[0.0, 0.0, 0.0],            // color1
    &[1.0, 1.0, 1.0],            // color2
    [0, 0, 0],                    // offset
    roi
);

// Create noise
let roi = Roi3D::from_xywh(0, 0, 512, 512, 1);
let noise = imagebufalgo::noise(
    NoiseType::Gaussian,
    0.5,    // a (mean for gaussian)
    0.2,    // b (stddev for gaussian)
    true,   // mono
    42,     // seed
    roi
);
```

### Geometry Operations

```rust
use vfx_io::imagebufalgo::{ResizeFilter, Roi3D};

// Resize with filter
let resized = imagebufalgo::resize(&image, 1920, 1080, ResizeFilter::Lanczos, None);

// Crop region
let roi = Some(Roi3D::from_xywh(100, 100, 500, 300, image.channels()));
let cropped = imagebufalgo::crop(&image, roi);

// Rotate (angle in radians)
let rotated = imagebufalgo::rotate(&image, 0.785, None); // 45 degrees

// Flip vertical
let flipped = imagebufalgo::flip(&image, None);

// Flip horizontal (flop)
let flopped = imagebufalgo::flop(&image, None);
```

### Color Operations

```rust
// Premultiply alpha
let premult = imagebufalgo::premult(&image, None);

// Unpremultiply
let unpremult = imagebufalgo::unpremult(&image, None);

// sRGB to linear
let linear = imagebufalgo::srgb_to_linear(&image, None);

// Linear to sRGB
let srgb = imagebufalgo::linear_to_srgb(&image, None);

// Adjust saturation (0.0 = grayscale, 1.0 = original, 2.0 = oversaturated)
let saturated = imagebufalgo::saturate(&image, 1.5, None);
```

### Compositing

```rust
// Alpha over
let comp = imagebufalgo::over(&fg, &bg, None);

// Add blend (with alpha handling)
let comp = imagebufalgo::add_blend(&a, &b, None);

// Multiply blend
let comp = imagebufalgo::multiply(&a, &b, None);

// Screen blend
let comp = imagebufalgo::screen(&a, &b, None);
```

### Filters

```rust
// Gaussian blur
let blurred = imagebufalgo::blur(&image, 5.0, None);

// Sharpen
let sharp = imagebufalgo::sharpen(&image, 1.5, None);

// Median filter (denoise)
let denoised = imagebufalgo::median(&image, 3, None);

// Unsharp mask
let sharpened = imagebufalgo::unsharp_mask(&image, 2.0, 1.5, 0.0, None);
```

### Statistics

```rust
// Get pixel statistics
let stats = imagebufalgo::compute_pixel_stats(&image, None);
println!("Min: {:?}", stats.min);
println!("Max: {:?}", stats.max);
println!("Mean: {:?}", stats.avg);

// Check if constant color
let (is_const, color) = imagebufalgo::is_constant_color(&image, 0.001, None);
if is_const {
    println!("Constant color: {:?}", color);
}
```

## OIIO Compatibility

| OIIO Function | vfx-rs Equivalent |
|---------------|-------------------|
| `IBA::zero` | `imagebufalgo::zero` |
| `IBA::fill` | `imagebufalgo::fill` |
| `IBA::checker` | `imagebufalgo::checker` |
| `IBA::noise` | `imagebufalgo::noise` |
| `IBA::channels` | `imagebufalgo::channels` |
| `IBA::crop` | `imagebufalgo::crop` |
| `IBA::resize` | `imagebufalgo::resize` |
| `IBA::rotate` | `imagebufalgo::rotate` |
| `IBA::flip` | `imagebufalgo::flip` |
| `IBA::flop` | `imagebufalgo::flop` |
| `IBA::add` | `imagebufalgo::add` |
| `IBA::sub` | `imagebufalgo::sub` |
| `IBA::mul` | `imagebufalgo::mul` |
| `IBA::div` | `imagebufalgo::div` |
| `IBA::over` | `imagebufalgo::over` |
| `IBA::premult` | `imagebufalgo::premult` |
| `IBA::unpremult` | `imagebufalgo::unpremult` |
| `IBA::blur` | `imagebufalgo::blur` |
| `IBA::median` | `imagebufalgo::median` |
| `IBA::unsharp_mask` | `imagebufalgo::unsharp_mask` |
| `IBA::laplacian` | `imagebufalgo::laplacian` |
| `IBA::dilate` | `imagebufalgo::dilate` |
| `IBA::erode` | `imagebufalgo::erode` |
| `IBA::deep_merge` | `imagebufalgo::deep_merge` |
| `IBA::flatten` | `imagebufalgo::flatten_deep` |
| `IBA::computePixelStats` | `imagebufalgo::compute_pixel_stats` |
| `IBA::isConstantColor` | `imagebufalgo::is_constant_color` |
