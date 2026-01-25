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

### Fit/Fill Dimensions

```rust
use vfx_ops::resize::{fit_dimensions, fill_dimensions};

// Calculate dimensions to fit inside target (preserves aspect ratio)
let (fit_w, fit_h) = fit_dimensions(src_w, src_h, max_w, max_h);

// Calculate dimensions to fill target (may crop)
let (fill_w, fill_h) = fill_dimensions(src_w, src_h, target_w, target_h);
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

// Edge detection (Sobel) - NOTE: Kernel::new(data, width, height)
let sobel_x = Kernel::new(vec![
    -1.0, 0.0, 1.0,
    -2.0, 0.0, 2.0,
    -1.0, 0.0, 1.0,
], 3, 3)?;

let edges = convolve(&data, w, h, c, &sobel_x)?;
```

### Morphological Operations

```rust
use vfx_ops::filter::{dilate, erode, morph_open, morph_close, morph_gradient};

let dilated = dilate(&data, w, h, ch, radius)?;
let eroded = erode(&data, w, h, ch, radius)?;
let opened = morph_open(&data, w, h, ch, radius)?;
let closed = morph_close(&data, w, h, ch, radius)?;
let gradient = morph_gradient(&data, w, h, ch, radius)?;
```

## Composite

### Porter-Duff Operations

```rust
use vfx_ops::composite::{over, blend, BlendMode};

// A over B (standard compositing)
let result = over(&foreground, &background, fg_w, fg_h)?;

// With blend mode
let multiplied = blend(&a, &b, width, height, BlendMode::Multiply)?;
```

### Blend Modes

```rust
use vfx_ops::composite::BlendMode;

BlendMode::Normal     // Standard alpha composite (over)
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
use vfx_ops::composite::{premultiply, unpremultiply, premultiply_inplace};

// Per-pixel (returns new pixel)
let premul_pixel = premultiply([r, g, b, a]);
let straight_pixel = unpremultiply([r, g, b, a]);

// In-place buffer operations
premultiply_inplace(&mut rgba_buffer);
unpremultiply_inplace(&mut rgba_buffer);
```

## Transform

### Flip/Rotate

```rust
use vfx_ops::transform::{flip_h, flip_v, rotate_90_cw, rotate_90_ccw, rotate_180};

// Horizontal flip (mirror)
let flipped = flip_h(&data, w, h, c);

// Vertical flip
let flipped = flip_v(&data, w, h, c);

// Rotate 90 degrees clockwise
let rotated = rotate_90_cw(&data, w, h, c);

// Rotate 90 degrees counter-clockwise
let rotated = rotate_90_ccw(&data, w, h, c);

// Rotate 180 degrees
let rotated = rotate_180(&data, w, h, c);
```

### Arbitrary Rotation

```rust
use vfx_ops::transform::rotate;

// Rotate by any angle (degrees), with fill color for uncovered areas
let rotated = rotate(&data, w, h, c, angle_deg, &fill_color)?;
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

### Tile

```rust
use vfx_ops::transform::tile;

// Create tiled pattern
let tiled = tile(&data, src_w, src_h, c, dst_w, dst_h)?;
```

### Paste

```rust
use vfx_ops::transform::paste;

// Paste source onto destination at position
paste(&src, src_w, src_h, &mut dst, dst_w, dst_h, c, dst_x, dst_y)?;
```

## Warp

### Lens Distortion

```rust
use vfx_ops::warp::{barrel, pincushion, fisheye};

// Barrel distortion (fisheye-like)
let warped = barrel(&data, w, h, c, k1, k2);

// Pincushion distortion (opposite of barrel)
let warped = pincushion(&data, w, h, c, k1, k2);

// Fisheye effect
let warped = fisheye(&data, w, h, c, strength);
```

### Creative Warps

```rust
use vfx_ops::warp::{twist, wave, spherize, ripple};

// Twist/twirl effect
let warped = twist(&data, w, h, c, angle_deg, radius);

// Wave distortion
let warped = wave(&data, w, h, c, amplitude, frequency);

// Spherize (bulge)
let warped = spherize(&data, w, h, c, strength, radius);

// Ripple effect
let warped = ripple(&data, w, h, c, amplitude, frequency, decay);
```

## Layer Operations

Process ImageLayer objects:

```rust
use vfx_ops::layer_ops::{resize_layer, blur_layer, crop_layer, sharpen_layer};
use vfx_ops::resize::Filter;

// Resize a layer
let resized = resize_layer(&layer, new_w, new_h, Filter::Lanczos3)?;

// Blur a layer
let blurred = blur_layer(&layer, radius)?;

// Crop a layer
let cropped = crop_layer(&layer, x, y, crop_w, crop_h)?;

// Sharpen a layer
let sharpened = sharpen_layer(&layer, amount)?;
```

## Parallel Processing

Operations use Rayon for multi-threading:

```rust
use vfx_ops::parallel;

// Parallel versions of common operations
let blurred = parallel::box_blur(&data, w, h, c, radius)?;
let resized = parallel::resize(&data, src_w, src_h, dst_w, dst_h, c, filter)?;
let composited = parallel::over(&fg, &bg, w, h)?;
let filtered = parallel::convolve(&data, w, h, c, &kernel)?;
```

## FFT Operations

Frequency-domain processing (optional):

```rust
#[cfg(feature = "fft")]
use vfx_ops::fft::{fft_blur, fft_sharpen, fft_highpass, fft_convolve};

// FFT-based blur (better for very large radii)
let blurred = fft_blur(&data, w, h, c, sigma)?;

// FFT-based sharpen
let sharpened = fft_sharpen(&data, w, h, c, sigma, amount)?;

// FFT highpass filter
let highpass = fft_highpass(&data, w, h, c, cutoff)?;

// FFT convolution (faster for large kernels)
let convolved = fft_convolve(&data, w, h, c, &kernel)?;
```

## Guard (Safety Checks)

Validate operations before executing:

```rust
use vfx_ops::guard::ensure_color_channels;

// Check if image has enough channels for color operations
ensure_color_channels(&spec, "blur", allow_non_color)?;
```

## Error Handling

```rust
use vfx_ops::OpsError;

match result {
    Err(OpsError::InvalidDimensions(msg)) => println!("Bad size: {}", msg),
    Err(OpsError::UnsupportedChannels(n)) => println!("Need 3+ channels, got {}", n),
    Err(OpsError::InvalidParameter(msg)) => println!("Bad parameter: {}", msg),
    _ => {}
}
```

## Performance Tips

1. **Use appropriate filter** - Nearest for speed, Lanczos for quality
2. **Use parallel module** - For automatic multi-threading on large images
3. **Use FFT blur** - For radius > 50 pixels
4. **Pre-multiply alpha** - Before compositing operations

## Dependencies

- `vfx-core` - Core types
- `vfx-io` - Image data types
- `vfx-color` - Color operations
- `vfx-math` - Interpolation
- `rayon` - Parallelism
- `rustfft` - FFT (optional)
