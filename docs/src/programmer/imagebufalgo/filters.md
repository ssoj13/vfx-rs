# Filter Operations

Image filtering for blur, sharpen, denoise, and morphological operations.

## Blur Operations

### Gaussian Blur

```rust
use vfx_io::imagebufalgo::blur;

// Blur with radius (sigma)
let blurred = blur(&image, 5.0)?;

// Separate X/Y blur
let blurred = blur_xy(&image, 10.0, 2.0)?;
```

### Box Blur

```rust
use vfx_io::imagebufalgo::box_blur;

// Fast box blur (averaging)
let blurred = box_blur(&image, 5)?;
```

## Sharpening

### Unsharp Mask

```rust
use vfx_io::imagebufalgo::unsharp_mask;

// Standard unsharp mask
// Parameters: image, sigma, strength, threshold
let sharp = unsharp_mask(&image, 2.0, 1.5, 0.0)?;

// With threshold (only sharpen above threshold)
let sharp = unsharp_mask(&image, 2.0, 1.0, 0.1)?;
```

### Sharpen

```rust
use vfx_io::imagebufalgo::sharpen;

// Simple sharpen (wrapper around unsharp_mask)
let sharp = sharpen(&image, 1.5)?;
```

### Laplacian

```rust
use vfx_io::imagebufalgo::laplacian;

// Edge detection via Laplacian
let edges = laplacian(&image)?;
```

## Denoise

### Median Filter

```rust
use vfx_io::imagebufalgo::median;

// Median filter for noise removal
let denoised = median(&image, 3)?;  // 3x3 kernel
let denoised = median(&image, 5)?;  // 5x5 kernel
```

## Morphological Operations

### Dilate/Erode

```rust
use vfx_io::imagebufalgo::{dilate, erode};

// Dilate (grow bright areas)
let dilated = dilate(&image, 3)?;  // 3x3 kernel

// Erode (shrink bright areas)
let eroded = erode(&image, 3)?;
```

### Open/Close

```rust
use vfx_io::imagebufalgo::{morph_open, morph_close};

// Morphological open (erode then dilate)
// Removes small bright spots
let opened = morph_open(&image, 3)?;

// Morphological close (dilate then erode)
// Removes small dark spots
let closed = morph_close(&image, 3)?;
```

## Edge Detection

### Sobel

```rust
use vfx_io::imagebufalgo::sobel;

// Sobel edge detection
let edges = sobel(&image)?;
```

## Convolution

### Custom Kernel

```rust
use vfx_io::imagebufalgo::convolve;

// Custom 3x3 kernel
let kernel = [
    0.0, -1.0, 0.0,
    -1.0, 5.0, -1.0,
    0.0, -1.0, 0.0,
];
let result = convolve(&image, 3, 3, &kernel)?;
```

## Examples

### Denoise Workflow

```rust
// Multi-pass denoise
let mut image = vfx_io::read("noisy.exr")?;

// Light median for impulse noise
let denoised = median(&image, 3)?;

// Light blur for remaining noise
let final_img = blur(&denoised, 0.5)?;
```

### Edge Enhancement

```rust
// Subtle edge enhancement
let edges = laplacian(&image)?;
let enhanced = imagebufalgo::add(&image, &edges, 0.3)?;
```

### Mask Cleanup

```rust
// Clean up binary mask
let mut mask = vfx_io::read("rough_mask.exr")?;

// Remove small holes
let closed = morph_close(&mask, 5)?;

// Remove small particles
let cleaned = morph_open(&closed, 5)?;
```

### Bloom Effect

```rust
// Create bloom from highlights
let mut bright = image.clone();

// Threshold to keep only bright areas
imagebufalgo::clamp(&mut bright, 0.8, 1000.0)?;

// Heavy blur
let glow = blur(&bright, 20.0)?;

// Add back to original
let bloomed = imagebufalgo::add(&image, &glow)?;
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
