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
use vfx_compute::{Processor, ComputeImage, Backend};

// Auto-select best backend
let proc = Processor::auto()?;

// Or explicit backend selection
let proc = Processor::new(Backend::Cpu)?;

// Load image into compute format
let mut img = ComputeImage::from_f32(data, 1920, 1080, 3)?;

// Apply operations
proc.apply_exposure(&mut img, 1.5)?;
proc.apply_saturation(&mut img, 1.2)?;

// Get result (consuming) or borrow data
let result = img.into_vec();   // or img.data() for reference
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
let img = ComputeImage::from_f32(data, width, height, channels);

// From vfx-io ImageData (via convert module)
use vfx_compute::convert::{from_image_data, to_image_data};
let img = from_image_data(&image_data);

// To raw data
let data: Vec<f32> = img.into_vec();  // consuming
// or borrow: img.data()

// Back to ImageData
let image_data = to_image_data(&img);
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
// 4x4 matrix (row-major, 16 floats)
// RGB transform uses top-left 3x3, fourth column for offset
let matrix: [f32; 16] = [
    1.1, 0.0, 0.0, 0.0,   // R scale
    0.0, 1.0, 0.0, 0.0,   // G (unchanged)
    0.0, 0.0, 0.9, 0.0,   // B scale
    0.0, 0.0, 0.0, 1.0,   // identity
];
proc.apply_matrix(&mut img, &matrix)?;
```

## Image Operations

### Resize

```rust
use vfx_compute::ResizeFilter;

let resized = proc.resize(&img, 1920, 1080, ResizeFilter::Lanczos)?;
```

### Blur

```rust
proc.blur(&mut img, 5.0)?;  // Gaussian blur, radius in pixels
```

## Pipeline API

Use `ComputePipeline` for processing workflows:

```rust
use vfx_compute::{ComputePipeline, ImageInput, ImageOutput};

// Create pipeline with auto backend
let mut pipeline = ComputePipeline::auto()?;

// Or explicit CPU/GPU
let mut pipeline = ComputePipeline::cpu()?;

// Process image
let input = ImageInput::from_path("input.exr")?;
let output = ImageOutput::path("output.exr");
pipeline.process(&input, &output)?;
```

## ProcessorBuilder

Fine-grained control over the processor:

```rust
use vfx_compute::{ProcessorBuilder, Backend};

let proc = ProcessorBuilder::new()
    .backend(Backend::Wgpu)  // Force GPU
    .tile_size(512)          // Tile size for GPU
    .ram_limit_mb(8192)      // 8GB RAM limit
    .ram_percent(75)         // Or use 75% of system RAM
    .verbose(true)           // Enable debug output
    .build()?;
```

## ComputePipelineBuilder

Configure pipeline processing strategy:

```rust
use vfx_compute::{ComputePipeline, ProcessingStrategy};

let pipeline = ComputePipeline::builder()
    .backend(Backend::Auto)
    .strategy(ProcessingStrategy::Tiled)
    .tile_size(1024)
    .verbose(true)
    .build()?;
```

## Processing Strategy

The pipeline can use different strategies:

```rust
use vfx_compute::ProcessingStrategy;

// Whole image at once (small images)
ProcessingStrategy::WholeImage

// Process in tiles (large images, GPU)
ProcessingStrategy::Tiled

// Stream from disk (huge images)
ProcessingStrategy::Streaming
```

## TileWorkflow

Hint for tile size optimization based on operation type:

```rust
use vfx_compute::TileWorkflow;

// Different operation types need different tile sizes
TileWorkflow::ColorTransform     // Standard tiles
TileWorkflow::Convolution { kernel_radius: 5 }  // Larger for kernel overlap
TileWorkflow::Warp               // May need larger for sampling
TileWorkflow::Composite          // Standard tiles
```

## GPU Limits

Check hardware constraints:

```rust
use vfx_compute::GpuLimits;

let limits = proc.limits();
println!("Max tile: {}x{}", limits.max_tile_dim, limits.max_tile_dim);
println!("Max buffer: {} bytes", limits.max_buffer_bytes);
println!("Available memory: {} bytes", limits.available_memory);
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

The `Processor::auto()` selects backend based on availability.

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
