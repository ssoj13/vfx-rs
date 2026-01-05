# vfx-ops

Image processing operations for VFX pipelines.

## Purpose

Fundamental image operations used in compositing and visual effects: resize, blur, composite, transform. Designed as a Rust alternative to OIIO image processing.

## Modules

- `resize` - Image scaling and resampling
- `filter` - Blur, sharpen, convolve
- `composite` - Layer blending
- `transform` - Geometric transforms
- `warp` - Distortion and warping

## Resize

### Basic Resize

```rust
use vfx_ops::resize::{resize_f32, Filter};

let resized = resize_f32(
    &src_data,
    src_width, src_height, channels,
    dst_width, dst_height,
    Filter::Lanczos3
)?;
```

### Filters

```rust
use vfx_ops::resize::Filter;

// Fast, blocky (good for pixel art)
let nearest = Filter::Nearest;

// Fast, smooth
let bilinear = Filter::Bilinear;

// Good quality
let bicubic = Filter::Bicubic;

// Best quality (default)
let lanczos = Filter::Lanczos3;
```

### Scale Factor

```rust
// Calculate dimensions from scale
let scale = 0.5;
let dst_w = (src_w as f32 * scale) as usize;
let dst_h = (src_h as f32 * scale) as usize;
```

## Filter Operations

### Box Blur

```rust
use vfx_ops::filter::box_blur;

let blurred = box_blur(&data, width, height, channels, radius)?;
```

### Gaussian Blur

```rust
use vfx_ops::filter::{Kernel, convolve};

let kernel = Kernel::gaussian(radius * 2 + 1, sigma);
let blurred = convolve(&data, width, height, channels, &kernel)?;
```

### Sharpen

```rust
use vfx_ops::filter::{Kernel, convolve};

// Unsharp mask: original + (original - blurred) * amount
let kernel = Kernel::sharpen(amount);
let sharpened = convolve(&data, width, height, channels, &kernel)?;
```

### Custom Kernels

```rust
use vfx_ops::filter::Kernel;

// Edge detection (Sobel)
let sobel_x = Kernel::from_data(3, 3, vec![
    -1.0, 0.0, 1.0,
    -2.0, 0.0, 2.0,
    -1.0, 0.0, 1.0,
]);

let edges = convolve(&data, w, h, c, &sobel_x)?;
```

## Composite

### Porter-Duff Operations

```rust
use vfx_ops::composite::{over, blend, BlendMode};

// A over B (standard compositing)
let result = over(&foreground, &background)?;

// With blend mode
let multiplied = blend(&a, &b, BlendMode::Multiply)?;
```

### Blend Modes

```rust
use vfx_ops::composite::BlendMode;

BlendMode::Over       // Standard alpha composite
BlendMode::Add        // Additive (lighten)
BlendMode::Multiply   // Darken
BlendMode::Screen     // Lighten
BlendMode::Overlay    // Contrast
BlendMode::SoftLight  // Subtle contrast
BlendMode::HardLight  // Strong contrast
BlendMode::Difference // Invert
BlendMode::Exclusion  // Softer difference
```

### Premultiplied Alpha

VFX standard is premultiplied alpha:

```rust
use vfx_ops::composite::{premultiply, unpremultiply};

// Convert straight → premultiplied
premultiply(&mut rgba_data);

// Convert premultiplied → straight
unpremultiply(&mut rgba_data);
```

## Transform

### Flip/Rotate

```rust
use vfx_ops::transform::{flip_h, flip_v, rotate_90, rotate_180, rotate_270};

let flipped = flip_h(&data, w, h, c)?;
let rotated = rotate_90(&data, w, h, c)?;
```

### Crop

```rust
use vfx_ops::transform::crop;

let cropped = crop(&data, src_w, src_h, c, x, y, crop_w, crop_h)?;
```

### Pad

```rust
use vfx_ops::transform::pad;

// Pad with specified color
let padded = pad(&data, w, h, c, left, right, top, bottom, &fill_color)?;
```

## Warp

### Lens Distortion

```rust
use vfx_ops::warp::{barrel_distort, pincushion_distort};

// Barrel (fisheye-like)
let warped = barrel_distort(&data, w, h, c, k1, k2)?;

// Pincushion (opposite)
let warped = pincushion_distort(&data, w, h, c, k1, k2)?;
```

### ST Map

Apply UV distortion from ST map:

```rust
use vfx_ops::warp::st_map;

// st_map contains UV coordinates per pixel
let warped = st_map(&data, w, h, c, &st_data)?;
```

## Layer Operations

Process specific layers in multi-layer images:

```rust
use vfx_ops::layer_ops::{apply_to_layer, LayerMask};

// Apply operation to specific layer
apply_to_layer(&mut layered_image, "diffuse", |data| {
    // Process layer data
    Ok(())
})?;
```

## Parallel Processing

Operations use Rayon for multi-threading:

```rust
// Feature enabled by default
// Parallel processing is automatic for large images

// Disable for debugging
#[cfg(not(feature = "parallel"))]
```

## FFT Operations

Frequency-domain processing (optional):

```rust
#[cfg(feature = "fft")]
use vfx_ops::fft::{fft_blur, fft_sharpen};

// FFT-based blur (better for very large radii)
let blurred = fft_blur(&data, w, h, c, radius)?;
```

## Guard (Safety Checks)

Validate operations before executing:

```rust
use vfx_ops::guard::ensure_color_processing;

// Check if image is safe for color operations
// (e.g., not an ID pass or depth)
ensure_color_processing(&image, "blur", allow_non_color)?;
```

## Error Handling

```rust
use vfx_ops::OpsError;

match result {
    Err(OpsError::InvalidDimensions(msg)) => println!("Bad size: {}", msg),
    Err(OpsError::UnsupportedChannels(n)) => println!("Need 3+ channels, got {}", n),
    Err(OpsError::FilterError(msg)) => println!("Filter failed: {}", msg),
    _ => {}
}
```

## Performance Tips

1. **Use appropriate filter** - Nearest for speed, Lanczos for quality
2. **Process in tiles** - For very large images
3. **Enable parallel feature** - Automatic multi-threading
4. **Pre-multiply alpha** - Before compositing operations
5. **Use FFT blur** - For radius > 50 pixels

## Dependencies

- `vfx-core` - Core types
- `vfx-io` - Image data types
- `vfx-math` - Interpolation
- `vfx-compute` - GPU acceleration
- `rayon` - Parallelism (optional)
- `rustfft` - FFT (optional)
