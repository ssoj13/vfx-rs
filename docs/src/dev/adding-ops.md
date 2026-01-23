# Adding Operations

Guide for adding new image processing operations to `vfx-ops`.

## Architecture Overview

Operations in `vfx-ops` are organized by category:

```
vfx-ops/src/
    filter.rs       <- Convolution, blur, sharpen, morphological ops
    transform.rs    <- Flip, rotate, crop, pad
    resize.rs       <- Image scaling with various filters
    composite.rs    <- Porter-Duff, blend modes
    warp.rs         <- Lens distortion, creative warps
    layer_ops.rs    <- Operations on ImageLayer structs
    parallel.rs     <- Parallelized versions of ops
    fft.rs          <- Frequency-domain processing
```

## Design Principles

1. **Pure functions** - Take input, return output, no side effects
2. **Float-based** - Work on `f32` data for precision  
3. **Channel-agnostic** - Handle any channel count
4. **Error handling** - Return `OpsResult<T>` for fallible operations
5. **Parallel ready** - Use rayon for large images

## Step-by-Step: Adding Edge Detection

### 1. Identify the Right Module

Edge detection uses convolution, so it belongs in `filter.rs`.

### 2. Add Kernel Factory

In `vfx-ops/src/filter.rs`, add a new `Kernel` constructor:

```rust
impl Kernel {
    // ... existing methods ...
    
    /// Creates a Sobel X edge detection kernel.
    pub fn sobel_x() -> Self {
        Self {
            data: vec![
                -1.0, 0.0, 1.0,
                -2.0, 0.0, 2.0,
                -1.0, 0.0, 1.0,
            ],
            width: 3,
            height: 3,
        }
    }
    
    /// Creates a Sobel Y edge detection kernel.
    pub fn sobel_y() -> Self {
        Self {
            data: vec![
                -1.0, -2.0, -1.0,
                 0.0,  0.0,  0.0,
                 1.0,  2.0,  1.0,
            ],
            width: 3,
            height: 3,
        }
    }
}
```

### 3. Add Operation Function

```rust
/// Detect edges using Sobel operator.
///
/// Returns gradient magnitude: sqrt(gx^2 + gy^2)
pub fn sobel_edges(
    data: &[f32],
    width: usize,
    height: usize,
    channels: usize,
) -> OpsResult<Vec<f32>> {
    let gx = convolve(data, width, height, channels, &Kernel::sobel_x())?;
    let gy = convolve(data, width, height, channels, &Kernel::sobel_y())?;
    
    let mut result = vec![0.0; data.len()];
    for i in 0..data.len() {
        result[i] = (gx[i] * gx[i] + gy[i] * gy[i]).sqrt();
    }
    
    Ok(result)
}
```

### 4. Add Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_sobel_edges() {
        // Vertical edge: left half = 0, right half = 1
        let mut data = vec![0.0f32; 8 * 8 * 1];
        for y in 0..8 {
            for x in 4..8 {
                data[y * 8 + x] = 1.0;
            }
        }
        
        let edges = sobel_edges(&data, 8, 8, 1).unwrap();
        
        // Edge should be detected at x=4 boundary
        // Check center row (y=4)
        let row = &edges[4 * 8..(4 + 1) * 8];
        
        // Pixels at edge should have high gradient
        assert!(row[3] > 0.5 || row[4] > 0.5);
        
        // Pixels away from edge should have low gradient
        assert!(row[0] < 0.1);
        assert!(row[7] < 0.1);
    }
    
    #[test]
    fn test_flat_image_no_edges() {
        let data = vec![0.5f32; 16 * 16 * 3];
        let edges = sobel_edges(&data, 16, 16, 3).unwrap();
        
        // Center pixels should have zero gradient
        for y in 2..14 {
            for x in 2..14 {
                for c in 0..3 {
                    let idx = (y * 16 + x) * 3 + c;
                    assert!(edges[idx].abs() < 0.01);
                }
            }
        }
    }
}
```

### 5. Export from lib.rs

In `vfx-ops/src/lib.rs`:

```rust
pub use filter::sobel_edges;
```

### 6. Add CLI Command (Optional)

If the operation should be available from CLI, add to `vfx-cli`:

In `vfx-cli/src/main.rs`:

```rust
/// Detect edges in image
#[derive(Parser)]
pub struct EdgesArgs {
    /// Input image
    input: PathBuf,
    
    /// Output image
    #[arg(short, long)]
    output: PathBuf,
}

#[derive(Subcommand)]
enum Commands {
    // ... existing commands
    
    /// Detect edges using Sobel operator
    Edges(EdgesArgs),
}
```

In `vfx-cli/src/commands/edges.rs`:

```rust
use crate::EdgesArgs;
use anyhow::Result;
use vfx_ops::filter::sobel_edges;

pub fn run(args: EdgesArgs, verbose: u8, allow_non_color: bool) -> Result<()> {
    let image = super::load_image(&args.input)?;
    super::ensure_color_processing(&image, "edges", allow_non_color)?;
    
    let data = image.to_f32();
    let edges = sobel_edges(
        &data,
        image.width as usize,
        image.height as usize,
        image.channels as usize,
    )?;
    
    let output = vfx_io::ImageData::from_f32(
        image.width, image.height, image.channels, edges
    );
    
    super::save_image(&args.output, &output)?;
    
    if verbose > 0 {
        println!("Edge detection complete.");
    }
    
    Ok(())
}
```

## Operation Patterns

### Custom Kernel Convolution

```rust
use vfx_ops::filter::{Kernel, convolve};

// Create custom kernel: NOTE order is (data, width, height)
let emboss = Kernel::new(vec![
    -2.0, -1.0, 0.0,
    -1.0,  1.0, 1.0,
     0.0,  1.0, 2.0,
], 3, 3)?;

let result = convolve(&data, width, height, channels, &emboss)?;
```

### Using Built-in Kernels

```rust
use vfx_ops::filter::Kernel;

let blur = Kernel::box_blur(5);        // 5x5 box blur
let gauss = Kernel::gaussian(5, 1.0);  // 5x5 gaussian, sigma=1.0
let sharp = Kernel::sharpen(0.5);      // sharpen with amount 0.5
let edges = Kernel::edge_detect();     // laplacian edge detection
```

### Per-Pixel Operations

```rust
/// Invert image colors.
pub fn invert(data: &[f32]) -> Vec<f32> {
    data.iter().map(|&v| 1.0 - v).collect()
}

/// Clamp values to range.
pub fn clamp_range(data: &[f32], min: f32, max: f32) -> Vec<f32> {
    data.iter().map(|&v| v.clamp(min, max)).collect()
}
```

### Neighborhood Operations

```rust
use crate::OpsResult;

/// Median filter for noise reduction.
pub fn median(
    data: &[f32],
    w: usize,
    h: usize,
    ch: usize,
    radius: usize,
) -> OpsResult<Vec<f32>> {
    let mut result = vec![0.0; data.len()];
    let size = (radius * 2 + 1) * (radius * 2 + 1);
    
    for y in 0..h {
        for x in 0..w {
            for c in 0..ch {
                let mut samples = Vec::with_capacity(size);
                
                for dy in -(radius as isize)..=(radius as isize) {
                    for dx in -(radius as isize)..=(radius as isize) {
                        let nx = (x as isize + dx).clamp(0, w as isize - 1) as usize;
                        let ny = (y as isize + dy).clamp(0, h as isize - 1) as usize;
                        samples.push(data[(ny * w + nx) * ch + c]);
                    }
                }
                
                samples.sort_by(|a, b| a.partial_cmp(b).unwrap());
                result[(y * w + x) * ch + c] = samples[samples.len() / 2];
            }
        }
    }
    
    Ok(result)
}
```

### Parallel Processing

```rust
use rayon::prelude::*;

/// Parallel per-row processing.
pub fn parallel_brightness(
    data: &[f32],
    w: usize,
    h: usize,
    ch: usize,
    factor: f32,
) -> Vec<f32> {
    let mut result = vec![0.0; data.len()];
    let row_size = w * ch;
    
    result.par_chunks_mut(row_size)
        .enumerate()
        .for_each(|(y, row)| {
            let src_row = &data[y * row_size..(y + 1) * row_size];
            for (dst, &src) in row.iter_mut().zip(src_row.iter()) {
                *dst = src * factor;
            }
        });
    
    result
}
```

## Testing Guidelines

1. **Dimension preservation** - Output size matches input
2. **Identity cases** - Zero strength/amount = no change
3. **Bounds checking** - Test edge pixels
4. **Channel handling** - Test 1, 3, 4 channels
5. **Known values** - Test with synthetic images where result is predictable

## Checklist

- [ ] Identify correct module (`filter.rs`, `transform.rs`, etc.)
- [ ] Add function with proper documentation
- [ ] Use `OpsResult<T>` for error handling
- [ ] Add unit tests
- [ ] Export from `lib.rs` if public API
- [ ] Add CLI command (if user-facing)
- [ ] Update `docs/src/crates/ops.md`
- [ ] Add benchmark (if performance-critical)
