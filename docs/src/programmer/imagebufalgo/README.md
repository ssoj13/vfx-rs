# ImageBufAlgo Reference

ImageBufAlgo provides OIIO-compatible image processing operations. The vfx-rs implementation covers the complete OIIO ImageBufAlgo API.

## Module Overview

| Module | Operations |
|--------|------------|
| [Patterns](./patterns.md) | fill, checker, noise, constant, zero |
| [Channels](./channels.md) | channel_append, channels, shuffle |
| [Geometry](./geometry.md) | crop, resize, rotate, flip, warp |
| [Arithmetic](./arithmetic.md) | add, sub, mul, div, pow, abs |
| [Color](./color.md) | colorconvert, premult, unpremult |
| [Composite](./composite.md) | over, add, multiply, screen |
| [Statistics](./stats.md) | computePixelStats, isConstantColor |
| [Filters](./filters.md) | blur, sharpen, median, morphology |
| [Deep](./deep.md) | deepMerge, flatten, holdout |
| [OCIO](./ocio.md) | ocioconvert, ociodisplay, ociolook |

## Usage Pattern

All ImageBufAlgo operations follow consistent patterns:

### In-Place Operations

```rust
use vfx_io::ImageData;
use vfx_io::imagebufalgo;

let mut image = vfx_io::read("input.exr")?;

// In-place modification
imagebufalgo::add_constant(&mut image, &[0.1, 0.1, 0.1, 0.0])?;
```

### Creating New Images

```rust
// Create result in new buffer
let blurred = imagebufalgo::blur(&image, 5.0)?;
```

### Chaining Operations

```rust
let mut image = vfx_io::read("input.exr")?;

imagebufalgo::resize(&mut image, 1920, 1080)?;
imagebufalgo::blur_inplace(&mut image, 2.0)?;
imagebufalgo::colorconvert(&mut image, "sRGB", "ACEScg")?;

vfx_io::write("output.exr", &image)?;
```

## Quick Examples

### Pattern Generation

```rust
use vfx_io::imagebufalgo;

// Create solid color
let red = imagebufalgo::fill(100, 100, 3, &[1.0, 0.0, 0.0])?;

// Create checker pattern
let checker = imagebufalgo::checker(256, 256, 3, 16,
    &[0.0, 0.0, 0.0], &[1.0, 1.0, 1.0])?;

// Create noise
let noise = imagebufalgo::noise(512, 512, 1, "gaussian", 0.5, 0.2, 42)?;
```

### Geometry Operations

```rust
// Resize with filter
let resized = imagebufalgo::resize(&image, 1920, 1080, "lanczos")?;

// Crop region
let cropped = imagebufalgo::crop(&image, 100, 100, 500, 300)?;

// Rotate
let rotated = imagebufalgo::rotate(&image, 45.0)?;

// Flip
let flipped = imagebufalgo::flip_horizontal(&image)?;
```

### Color Operations

```rust
// Color space conversion
imagebufalgo::colorconvert(&mut image, "sRGB", "ACEScg")?;

// Premultiply alpha
imagebufalgo::premult(&mut image)?;

// Unpremultiply
imagebufalgo::unpremult(&mut image)?;
```

### Compositing

```rust
// Alpha over
let comp = imagebufalgo::over(&fg, &bg)?;

// Add blend
let comp = imagebufalgo::add(&a, &b)?;

// Multiply blend
let comp = imagebufalgo::multiply(&a, &b)?;
```

### Filters

```rust
// Gaussian blur
let blurred = imagebufalgo::blur(&image, 5.0)?;

// Sharpen
let sharp = imagebufalgo::sharpen(&image, 1.5)?;

// Median filter (denoise)
let denoised = imagebufalgo::median(&image, 3)?;
```

### Statistics

```rust
// Get pixel statistics
let stats = imagebufalgo::computePixelStats(&image)?;
println!("Min: {:?}", stats.min);
println!("Max: {:?}", stats.max);
println!("Mean: {:?}", stats.avg);

// Check if constant color
let is_const = imagebufalgo::isConstantColor(&image, 0.001)?;
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
| `IBA::flip` | `imagebufalgo::flip_*` |
| `IBA::add` | `imagebufalgo::add` |
| `IBA::sub` | `imagebufalgo::sub` |
| `IBA::mul` | `imagebufalgo::mul` |
| `IBA::div` | `imagebufalgo::div` |
| `IBA::over` | `imagebufalgo::over` |
| `IBA::colorconvert` | `imagebufalgo::colorconvert` |
| `IBA::premult` | `imagebufalgo::premult` |
| `IBA::unpremult` | `imagebufalgo::unpremult` |
| `IBA::blur` | `imagebufalgo::blur` |
| `IBA::median` | `imagebufalgo::median` |
| `IBA::unsharp_mask` | `imagebufalgo::unsharp_mask` |
| `IBA::laplacian` | `imagebufalgo::laplacian` |
| `IBA::dilate` | `imagebufalgo::dilate` |
| `IBA::erode` | `imagebufalgo::erode` |
| `IBA::deep_merge` | `imagebufalgo::deep_merge` |
| `IBA::flatten` | `imagebufalgo::deep_flatten` |
| `IBA::ocioconvert` | `imagebufalgo::ocioconvert` |
| `IBA::ociodisplay` | `imagebufalgo::ociodisplay` |
