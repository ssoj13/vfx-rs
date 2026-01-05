# vfx-compute

Unified compute backend for VFX workflows.

## Purpose

Provides CPU and GPU backends for color transforms and image operations. Abstracts hardware differences behind a unified API.

## Architecture

```
Processor (unified API)
    └── Backend
            ├── CPU (Rayon parallel)
            └── GPU (wgpu compute shaders)
```

## Quick Start

```rust
use vfx_compute::{Processor, ComputeImage};

// Auto-select best backend
let proc = Processor::auto()?;

// Load image into compute format
let mut img = ComputeImage::from_f32(data, 1920, 1080, 3)?;

// Apply operations
proc.apply_exposure(&mut img, 1.5)?;
proc.apply_saturation(&mut img, 1.2)?;

// Get result
let result = img.to_vec();
```

## Backend Selection

### Automatic

```rust
// Picks GPU if available, falls back to CPU
let proc = Processor::auto()?;
```

### Explicit

```rust
use vfx_compute::{Processor, Backend};

// Force CPU
let cpu_proc = Processor::new(Backend::Cpu)?;

// Force GPU (wgpu)
let gpu_proc = Processor::new(Backend::Wgpu)?;
```

### Discovery

```rust
use vfx_compute::{detect_backends, describe_backends};

// List available backends
let backends = detect_backends();
for backend in &backends {
    println!("{:?}", backend);
}

// Detailed info
println!("{}", describe_backends());
```

## ComputeImage

GPU-ready image container:

```rust
use vfx_compute::ComputeImage;

// From raw data
let img = ComputeImage::from_f32(data, width, height, channels)?;

// From vfx-io ImageData
let img = ComputeImage::from_image_data(&image_data)?;

// To raw data
let data: Vec<f32> = img.to_vec();

// Back to ImageData
let image_data = img.to_image_data();
```

## Color Operations

### Exposure

```rust
// Adjust exposure in stops
proc.apply_exposure(&mut img, 1.0)?;   // +1 stop (2x brighter)
proc.apply_exposure(&mut img, -0.5)?;  // -0.5 stops
```

### Saturation

```rust
// Adjust saturation (1.0 = unchanged)
proc.apply_saturation(&mut img, 1.2)?;  // More saturated
proc.apply_saturation(&mut img, 0.0)?;  // Grayscale
```

### CDL (Color Decision List)

```rust
use vfx_compute::Cdl;

let cdl = Cdl {
    slope: [1.1, 1.0, 0.9],
    offset: [0.0, 0.0, 0.0],
    power: [1.0, 1.0, 1.0],
    saturation: 1.0,
};

proc.apply_cdl(&mut img, &cdl)?;
```

### Matrix Transform

```rust
use vfx_math::Mat3;

let matrix = Mat3::from_rows([...]);
proc.apply_matrix(&mut img, &matrix)?;
```

## Image Operations

### Resize

```rust
use vfx_compute::ResizeFilter;

let resized = proc.resize(&img, 1920, 1080, ResizeFilter::Lanczos3)?;
```

### Blur

```rust
proc.blur(&mut img, 5.0)?;  // Gaussian blur, radius in pixels
```

## Pipeline API

Chain operations for efficiency:

```rust
use vfx_compute::{ComputePipeline, ComputeOp};

let pipeline = ComputePipeline::builder()
    .add(ComputeOp::Exposure(1.0))
    .add(ComputeOp::Saturation(1.2))
    .add(ComputeOp::Matrix(my_matrix))
    .build()?;

// Apply all at once (GPU batching)
pipeline.apply(&proc, &mut img)?;
```

## ProcessorBuilder

Fine-grained control:

```rust
use vfx_compute::ProcessorBuilder;

let proc = ProcessorBuilder::new()
    .prefer_gpu(true)
    .tile_size(512)
    .ram_percent(75)
    .build()?;
```

## Tile Processing

For large images that don't fit in GPU memory:

```rust
use vfx_compute::TileWorkflow;

let workflow = TileWorkflow::new(proc, 1024);  // 1024x1024 tiles

workflow.process(&mut huge_image, |tile| {
    // Process each tile
    proc.apply_exposure(tile, 1.0)?;
    Ok(())
})?;
```

## GPU Limits

Check hardware constraints:

```rust
use vfx_compute::GpuLimits;

let limits = proc.limits()?;
println!("Max texture: {}x{}", limits.max_texture_dimension, limits.max_texture_dimension);
println!("Max buffer: {} bytes", limits.max_buffer_size);
```

## Layer Processing

Process specific EXR layers:

```rust
use vfx_compute::{LayerProcessor, ChannelGroup};

let layer_proc = LayerProcessor::new(&proc);

// Process only color channels, skip IDs
layer_proc.process_groups(&mut layered_image, |group, data| {
    match group {
        ChannelGroup::Color => proc.apply_exposure(data, 1.0)?,
        ChannelGroup::Depth => {},  // Skip
        ChannelGroup::Id => {},     // Skip
    }
    Ok(())
})?;
```

## Feature Flags

```toml
[dependencies]
vfx-compute = { version = "0.1", features = ["wgpu", "cuda"] }
```

| Feature | Backend | Requirements |
|---------|---------|--------------|
| (default) | CPU | None |
| `wgpu` | GPU | Vulkan/Metal/DX12 |
| `cuda` | NVIDIA | CUDA toolkit |

## When to Use GPU

GPU is faster for:
- Large images (4K+)
- Batch processing
- Complex operations (3D LUT, convolution)

CPU is faster for:
- Small images
- Simple operations
- When upload/download overhead dominates

The `Processor::auto()` heuristic considers image size.

## Error Handling

```rust
use vfx_compute::ComputeError;

match result {
    Err(ComputeError::NoAdapter) => println!("No GPU found"),
    Err(ComputeError::ImageTooLarge { width, height, limit }) => {
        println!("Image {}x{} exceeds GPU limit {}", width, height, limit);
    }
    Err(e) => println!("Error: {}", e),
    Ok(_) => {}
}
```

## Dependencies

- `vfx-core` - Core types
- `rayon` - CPU parallelism
- `wgpu` - GPU compute (optional)
- `cudarc` - CUDA (optional)
- `bytemuck` - Safe casting
