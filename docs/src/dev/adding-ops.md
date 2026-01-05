# Adding Operations

This guide covers adding new image processing operations to `vfx-ops`.

## Operation Design Principles

1. **Pure functions** - take input, return output, no side effects
2. **Float-based** - work on `f32` data for precision
3. **Channel-agnostic** - handle any channel count
4. **Error handling** - return `Result` for fallible operations
5. **Testable** - easy to unit test with synthetic data

## Step-by-Step: Adding Sharpen

### 1. Create Module

Create `vfx-ops/src/sharpen.rs`:

```rust
//! Sharpen filter operations.
//!
//! Unsharp mask and edge enhancement.

use anyhow::Result;

/// Apply unsharp mask sharpening.
///
/// # Arguments
/// * `data` - Pixel data (f32, any channel count)
/// * `width`, `height` - Image dimensions
/// * `channels` - Number of channels
/// * `amount` - Sharpening strength (0.0 = none, 1.0 = normal)
/// * `radius` - Blur radius for the mask
/// * `threshold` - Minimum difference to sharpen
///
/// # Returns
/// Sharpened pixel data.
pub fn unsharp_mask(
    data: &[f32],
    width: usize,
    height: usize,
    channels: usize,
    amount: f32,
    radius: usize,
    threshold: f32,
) -> Result<Vec<f32>> {
    // Validate input
    let expected = width * height * channels;
    if data.len() != expected {
        anyhow::bail!(
            "Data length {} doesn't match {}x{}x{}",
            data.len(), width, height, channels
        );
    }
    
    // Create blurred version
    let blurred = crate::filter::box_blur(data, width, height, channels, radius)?;
    
    // Apply unsharp mask: result = original + amount * (original - blurred)
    let mut result = Vec::with_capacity(data.len());
    
    for i in 0..data.len() {
        let original = data[i];
        let blur = blurred[i];
        let diff = original - blur;
        
        // Apply threshold
        let sharpened = if diff.abs() > threshold {
            original + amount * diff
        } else {
            original
        };
        
        result.push(sharpened);
    }
    
    Ok(result)
}

/// Simple laplacian sharpening.
pub fn laplacian_sharpen(
    data: &[f32],
    width: usize,
    height: usize,
    channels: usize,
    strength: f32,
) -> Result<Vec<f32>> {
    use crate::filter::{Kernel, convolve};
    
    // Laplacian kernel
    let kernel = Kernel::new(3, 3, vec![
         0.0, -1.0,  0.0,
        -1.0,  4.0, -1.0,
         0.0, -1.0,  0.0,
    ]);
    
    let edges = convolve(data, width, height, channels, &kernel)?;
    
    // Add edges to original
    let result: Vec<f32> = data.iter()
        .zip(edges.iter())
        .map(|(&orig, &edge)| orig + strength * edge)
        .collect();
    
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_unsharp_mask_dimensions() {
        let data = vec![0.5f32; 100 * 100 * 3];
        let result = unsharp_mask(&data, 100, 100, 3, 1.0, 2, 0.0).unwrap();
        assert_eq!(result.len(), data.len());
    }
    
    #[test]
    fn test_unsharp_zero_amount() {
        let data: Vec<f32> = (0..100).map(|i| i as f32 / 100.0).collect();
        let result = unsharp_mask(&data, 10, 10, 1, 0.0, 2, 0.0).unwrap();
        
        // With amount=0, output should equal input
        for (a, b) in data.iter().zip(result.iter()) {
            assert!((a - b).abs() < 1e-6);
        }
    }
    
    #[test]
    fn test_laplacian_flat_image() {
        // Flat image should have no edges
        let data = vec![0.5f32; 100 * 100 * 1];
        let result = laplacian_sharpen(&data, 100, 100, 1, 1.0).unwrap();
        
        // Center pixels should be unchanged (edge pixels may differ)
        for y in 2..98 {
            for x in 2..98 {
                let idx = y * 100 + x;
                assert!((result[idx] - 0.5).abs() < 0.01);
            }
        }
    }
}
```

### 2. Export from Module

In `vfx-ops/src/lib.rs`:

```rust
pub mod sharpen;

pub use sharpen::{unsharp_mask, laplacian_sharpen};
```

### 3. Add CLI Command

In `vfx-cli/src/commands/sharpen.rs`:

```rust
//! Sharpen command.

use crate::SharpenArgs;
use anyhow::Result;
use vfx_io::ImageData;
use vfx_ops::sharpen::unsharp_mask;

pub fn run(args: SharpenArgs, verbose: u8, allow_non_color: bool) -> Result<()> {
    let image = super::load_image_layer(&args.input, args.layer.as_deref())?;
    super::ensure_color_processing(&image, "sharpen", allow_non_color)?;
    
    let w = image.width as usize;
    let h = image.height as usize;
    let c = image.channels as usize;
    
    if verbose > 0 {
        println!("Sharpening {} (amount={}, radius={})",
            args.input.display(), args.amount, args.radius);
    }
    
    let src = image.to_f32();
    let sharpened = unsharp_mask(&src, w, h, c, args.amount, args.radius, args.threshold)?;
    
    let output = ImageData::from_f32(image.width, image.height, image.channels, sharpened);
    super::save_image_layer(&args.output, &output, args.layer.as_deref())?;
    
    if verbose > 0 {
        println!("Done.");
    }
    
    Ok(())
}
```

### 4. Define CLI Arguments

In `vfx-cli/src/main.rs`:

```rust
/// Sharpen image using unsharp mask
#[derive(Parser)]
pub struct SharpenArgs {
    /// Input image
    #[arg(short, long)]
    input: PathBuf,
    
    /// Output image
    #[arg(short, long)]
    output: PathBuf,
    
    /// Sharpening amount (0.0-2.0, default 1.0)
    #[arg(short, long, default_value = "1.0")]
    amount: f32,
    
    /// Blur radius for mask (default 2)
    #[arg(short, long, default_value = "2")]
    radius: usize,
    
    /// Threshold (default 0.0)
    #[arg(short, long, default_value = "0.0")]
    threshold: f32,
    
    /// Process specific layer (EXR)
    #[arg(long)]
    layer: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    // ... existing commands
    
    /// Sharpen image
    Sharpen(SharpenArgs),
}
```

### 5. Add Integration Test

In `vfx-tests/tests/ops_tests.rs`:

```rust
#[test]
fn test_sharpen_pipeline() {
    let input = test_image::gradient(256, 256, 3);
    
    let sharpened = unsharp_mask(
        &input.to_f32(),
        256, 256, 3,
        1.0, 2, 0.0
    ).unwrap();
    
    // Sharpening should increase contrast at edges
    // Test that some pixels changed
    let original = input.to_f32();
    let mut diff_count = 0;
    for (a, b) in original.iter().zip(sharpened.iter()) {
        if (a - b).abs() > 0.01 {
            diff_count += 1;
        }
    }
    assert!(diff_count > 0, "Sharpening should modify pixels");
}
```

### 6. Document the Operation

In docs:

```markdown
## Sharpen

Unsharp mask sharpening:

```bash
vfx sharpen -i input.exr -o output.exr --amount 1.5 --radius 3
```

Parameters:
- `--amount`: Strength (0-2, default 1.0)
- `--radius`: Blur radius (default 2)
- `--threshold`: Min difference to sharpen (default 0.0)
```

## Operation Patterns

### Simple Per-Pixel

```rust
pub fn invert(data: &[f32]) -> Vec<f32> {
    data.iter().map(|&v| 1.0 - v).collect()
}
```

### With Neighborhood

```rust
pub fn median_filter(data: &[f32], w: usize, h: usize, c: usize, radius: usize) -> Vec<f32> {
    let mut result = vec![0.0; data.len()];
    
    for y in 0..h {
        for x in 0..w {
            for ch in 0..c {
                let mut samples = Vec::new();
                
                // Gather neighborhood
                for dy in -(radius as isize)..=(radius as isize) {
                    for dx in -(radius as isize)..=(radius as isize) {
                        let nx = (x as isize + dx).clamp(0, w as isize - 1) as usize;
                        let ny = (y as isize + dy).clamp(0, h as isize - 1) as usize;
                        samples.push(data[(ny * w + nx) * c + ch]);
                    }
                }
                
                // Median
                samples.sort_by(|a, b| a.partial_cmp(b).unwrap());
                result[(y * w + x) * c + ch] = samples[samples.len() / 2];
            }
        }
    }
    
    result
}
```

### Parallel Processing

```rust
use rayon::prelude::*;

pub fn parallel_op(data: &[f32], w: usize, h: usize, c: usize) -> Vec<f32> {
    let mut result = vec![0.0; data.len()];
    
    result.par_chunks_mut(w * c)
        .enumerate()
        .for_each(|(y, row)| {
            for x in 0..w {
                for ch in 0..c {
                    row[x * c + ch] = process_pixel(data, x, y, ch, w, h, c);
                }
            }
        });
    
    result
}
```

## Testing Guidelines

1. **Dimension preservation** - output size matches input
2. **Identity cases** - zero strength = no change
3. **Bounds checking** - test edge pixels
4. **Channel handling** - test 1, 3, 4 channels
5. **Performance** - add benchmark for costly ops

## Checklist

- [ ] Create module in `vfx-ops/src/`
- [ ] Export from `lib.rs`
- [ ] Add unit tests
- [ ] Create CLI command (if user-facing)
- [ ] Add CLI args struct
- [ ] Register in command dispatch
- [ ] Add integration test
- [ ] Update documentation
- [ ] Add benchmark (if performance-critical)
