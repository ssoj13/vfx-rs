# Filter Operations

Image filtering for blur, sharpen, denoise, and morphological operations.

## Blur Operations

### Gaussian Blur

```rust
use vfx_io::imagebufalgo::blur;

// Blur with sigma (standard deviation)
let blurred = blur(&image, 5.0, None);

// With explicit ROI
let roi = Roi3D::from_xywh(0, 0, 100, 100, image.nchannels() as i32);
let blurred = blur(&image, 5.0, Some(roi));
```

### Box Blur

```rust
use vfx_io::imagebufalgo::box_blur;

// Fast box blur (averaging) with kernel size
let blurred = box_blur(&image, 5, None);
```

## Sharpening

### Unsharp Mask

```rust
use vfx_io::imagebufalgo::unsharp_mask;

// Standard unsharp mask
// Parameters: src, sigma, amount, threshold, roi
let sharp = unsharp_mask(&image, 2.0, 1.5, 0.0, None);

// With threshold (only sharpen where contrast > threshold)
let sharp = unsharp_mask(&image, 2.0, 1.0, 0.1, None);
```

### Sharpen

```rust
use vfx_io::imagebufalgo::sharpen;

// Simple sharpen by amount
let sharp = sharpen(&image, 1.5, None);
```

### Laplacian

```rust
use vfx_io::imagebufalgo::laplacian;

// Edge detection via Laplacian
let edges = laplacian(&image, None);
```

## Denoise

### Median Filter

```rust
use vfx_io::imagebufalgo::median;

// Median filter for noise removal
let denoised = median(&image, 3, None);  // 3x3 kernel
let denoised = median(&image, 5, None);  // 5x5 kernel
```

## Morphological Operations

### Dilate/Erode

```rust
use vfx_io::imagebufalgo::{dilate, erode};

// Dilate (grow bright areas)
let dilated = dilate(&image, 3, None);  // 3x3 kernel

// Erode (shrink bright areas)
let eroded = erode(&image, 3, None);
```

### Open/Close

```rust
use vfx_io::imagebufalgo::{morph_open, morph_close};

// Morphological open (erode then dilate)
// Removes small bright spots
let opened = morph_open(&image, 3, None);

// Morphological close (dilate then erode)
// Removes small dark spots
let closed = morph_close(&image, 3, None);
```

## Edge Detection

### Sobel

```rust
use vfx_io::imagebufalgo::sobel;

// Sobel edge detection
let edges = sobel(&image, None);
```

## Convolution

### Custom Kernel

```rust
use vfx_io::imagebufalgo::convolve;

// Custom 3x3 kernel (sharpen example)
let kernel = [
    0.0, -1.0, 0.0,
    -1.0, 5.0, -1.0,
    0.0, -1.0, 0.0,
];
// Parameters: src, kernel, kernel_width, kernel_height, roi
let result = convolve(&image, &kernel, 3, 3, None);
```

## Examples

### Denoise Workflow

```rust
use vfx_io::imagebufalgo::{median, blur};

// Multi-pass denoise
let image = vfx_io::read("noisy.exr")?;

// Light median for impulse noise
let denoised = median(&image, 3, None);

// Light blur for remaining noise
let final_img = blur(&denoised, 0.5, None);
```

### Edge Enhancement

```rust
use vfx_io::imagebufalgo::{laplacian, add};

// Subtle edge enhancement
let edges = laplacian(&image, None);
// Add edges back to original (ImageOrConst accepts both &ImageBuf and scalars)
let enhanced = add(&image, &edges, None);
```

### Mask Cleanup

```rust
use vfx_io::imagebufalgo::{morph_close, morph_open};

// Clean up binary mask
let mask = vfx_io::read("rough_mask.exr")?;

// Remove small holes
let closed = morph_close(&mask, 5, None);

// Remove small particles
let cleaned = morph_open(&closed, 5, None);
```

### Bloom Effect

```rust
use vfx_io::imagebufalgo::{clamp, blur, add};

// Create bloom from highlights
let image = vfx_io::read("render.exr")?;

// Threshold to keep only bright areas (per-channel min/max)
let bright = clamp(&image, &[0.8, 0.8, 0.8], &[1000.0, 1000.0, 1000.0], None);

// Heavy blur
let glow = blur(&bright, 20.0, None);

// Add back to original
let bloomed = add(&image, &glow, None);
```

## Performance Notes

| Operation | Complexity | Notes |
|-----------|------------|-------|
| Box blur | O(n) | Fast, separable |
| Gaussian blur | O(n) | Separable implementation |
| Median | O(n log n) | Per-pixel sorting |
| Morphology | O(n × k²) | k = kernel size |
| Convolution | O(n × k²) | k = kernel size |

All operations use parallel processing via rayon.

## API Reference

All filter functions follow the pattern:

- `fn_name(src, params..., roi: Option<Roi3D>) -> ImageBuf` - creates new output
- `fn_name_into(dst, src, params..., roi: Option<Roi3D>)` - writes to existing buffer

Functions return `ImageBuf` directly, not `Result`. Invalid inputs produce default/empty output.
