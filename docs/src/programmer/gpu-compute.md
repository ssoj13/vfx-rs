# GPU Compute

Hardware-accelerated image processing using the vfx-compute crate.

## Overview

vfx-compute provides unified CPU/GPU processing:

```
┌─────────────────────────────────────────┐
│              Processor                   │
│         (Unified API)                    │
├─────────────────────────────────────────┤
│  Backend Detection & Selection          │
├──────────────────┬──────────────────────┤
│   CPU Backend    │    GPU Backend       │
│   (rayon)        │    (wgpu)            │
├──────────────────┴──────────────────────┤
│            GpuPrimitives Trait          │
└─────────────────────────────────────────┘
```

## Quick Start

```rust
use vfx_compute::{Processor, ComputeImage};

// Auto-select best backend
let proc = Processor::auto()?;

// Create compute image
let mut img = ComputeImage::from_f32(&data, 1920, 1080, 3)?;

// Apply operations
proc.apply_exposure(&mut img, 1.5)?;
proc.apply_saturation(&mut img, 1.2)?;

// Get result
let pixels = img.to_f32()?;
```

## Backend Selection

### Automatic Selection

```rust
use vfx_compute::Processor;

// Prefers GPU, falls back to CPU
let proc = Processor::auto()?;
```

### Manual Selection

```rust
use vfx_compute::{Processor, Backend};

// Force GPU (fails if unavailable)
let proc = Processor::new(Backend::Wgpu)?;

// Force CPU
let proc = Processor::new(Backend::Cpu)?;
```

### Check Available Backends

```rust
use vfx_compute::{detect_backends, describe_backends};

// Get available backends
let backends = detect_backends();
for backend in &backends {
    println!("{:?}", backend);
}

// Human-readable descriptions
let desc = describe_backends();
println!("{}", desc);
```

## ComputeImage

GPU-compatible image container.

### Creating Images

```rust
use vfx_compute::ComputeImage;

// From raw data
let img = ComputeImage::from_f32(&pixels, 1920, 1080, 4)?;

// From vfx_io::ImageData
let image_data = vfx_io::read("input.exr")?;
let img = ComputeImage::from_image_data(&image_data)?;

// Empty image
let img = ComputeImage::new(1920, 1080, 4)?;
```

### Converting Back

```rust
// To raw pixels
let pixels: Vec<f32> = img.to_f32()?;

// To ImageData
let image_data = img.to_image_data()?;
vfx_io::write("output.exr", &image_data)?;
```

## Color Operations

### Exposure

```rust
// Exposure in stops
proc.apply_exposure(&mut img, 1.5)?;   // +1.5 stops
proc.apply_exposure(&mut img, -0.5)?;  // -0.5 stops
```

### Saturation

```rust
// Saturation multiplier
proc.apply_saturation(&mut img, 1.2)?;  // +20%
proc.apply_saturation(&mut img, 0.8)?;  // -20%
proc.apply_saturation(&mut img, 0.0)?;  // Grayscale
```

### Color Matrix

```rust
// Apply 3x3 color matrix
let matrix = [
    [1.1, -0.05, -0.05],
    [-0.02, 1.05, -0.03],
    [0.0, -0.1, 1.1],
];
proc.apply_color_matrix(&mut img, &matrix)?;
```

### CDL (Color Decision List)

```rust
use vfx_compute::Cdl;

let cdl = Cdl {
    slope: [1.1, 1.0, 0.9],
    offset: [0.01, 0.0, -0.01],
    power: [1.0, 1.0, 1.0],
    saturation: 1.1,
};
proc.apply_cdl(&mut img, &cdl)?;
```

## Image Operations

### Resize

```rust
use vfx_compute::ResizeFilter;

// Resize with Lanczos filter
proc.resize(&mut img, 960, 540, ResizeFilter::Lanczos)?;

// Other filters
proc.resize(&mut img, 960, 540, ResizeFilter::Bilinear)?;
proc.resize(&mut img, 960, 540, ResizeFilter::Mitchell)?;
```

### Transform

```rust
// Flip operations
proc.flip_horizontal(&mut img)?;
proc.flip_vertical(&mut img)?;

// Rotate 90 degrees
proc.rotate_90(&mut img)?;
proc.rotate_180(&mut img)?;
proc.rotate_270(&mut img)?;
```

## Batch Processing

### Color Batch

```rust
use vfx_compute::{ColorOpBatch, BatchOp};

// Create batch of operations
let batch = ColorOpBatch::new()
    .add(BatchOp::Exposure(0.5))
    .add(BatchOp::Saturation(1.1))
    .add(BatchOp::Matrix(color_matrix));

// Apply all at once (more efficient)
proc.apply_batch(&mut img, &batch)?;
```

## Tiled Processing

For large images that exceed GPU memory:

```rust
use vfx_compute::{ProcessorBuilder, ProcessorConfig};

let config = ProcessorConfig {
    tile_size: 2048,
    max_ram_percent: 0.5,
    ..Default::default()
};

let proc = ProcessorBuilder::new()
    .config(config)
    .build()?;

// Large images processed in tiles automatically
proc.apply_exposure(&mut huge_img, 1.5)?;
```

## GPU Limits

```rust
use vfx_compute::GpuLimits;

// Query GPU capabilities
if let Some(limits) = proc.gpu_limits() {
    println!("Max texture size: {}", limits.max_texture_dimension);
    println!("Max buffer size: {}", limits.max_buffer_size);
    println!("Compute workgroup size: {:?}", limits.max_workgroup_size);
}
```

## Error Handling

```rust
use vfx_compute::{ComputeError, ComputeResult};

fn process(path: &Path) -> ComputeResult<()> {
    let proc = Processor::auto().map_err(|e| {
        ComputeError::BackendNotAvailable(e.to_string())
    })?;

    let img = ComputeImage::from_f32(&data, w, h, ch)?;

    // Handle specific errors
    match proc.apply_exposure(&mut img, 1.5) {
        Ok(_) => {},
        Err(ComputeError::ImageTooLarge { width, height, limit }) => {
            eprintln!("Image {}x{} exceeds GPU limit {}", width, height, limit);
        },
        Err(e) => return Err(e),
    }

    Ok(())
}
```

## Performance Comparison

| Operation | CPU (rayon) | GPU (wgpu) |
|-----------|-------------|------------|
| Exposure 4K | ~15ms | ~2ms |
| Saturation 4K | ~18ms | ~2ms |
| Color Matrix 4K | ~20ms | ~3ms |
| Resize 4K→HD | ~50ms | ~8ms |

GPU acceleration provides 5-10x speedup for most operations.
