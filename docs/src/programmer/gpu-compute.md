# GPU Compute

Hardware-accelerated image processing using the vfx-compute crate.

## Overview

vfx-compute provides unified CPU/GPU processing with automatic backend selection:

```
+------------------------------------------+
|         User API (Processor)             |
+------------------------------------------+
|           AnyExecutor                    |
|   (enum dispatch: Cpu/Wgpu/Cuda)         |
+------------------------------------------+
|         TiledExecutor<G>                 |
|   (automatic tiling, region caching)     |
+------------------------------------------+
|        GpuPrimitives trait               |
|   (CpuPrimitives, WgpuPrimitives, etc)   |
+------------------------------------------+
|      Backend Implementation              |
|   (rayon, wgpu shaders, CUDA PTX)        |
+------------------------------------------+
```

## Quick Start

```rust
use vfx_compute::{Processor, ComputeImage, Backend};

// Auto-select best backend (CUDA > wgpu > CPU)
let proc = Processor::new(Backend::Auto)?;

// Create compute image
let mut img = ComputeImage::from_f32(data, 1920, 1080, 3)?;

// Apply operations
proc.apply_exposure(&mut img, 1.5)?;
proc.apply_cdl(&mut img, &cdl)?;

// Get result
let pixels = img.data();
```

## Backend Selection

### Automatic Selection

```rust
use vfx_compute::{Processor, Backend};

// Prefers CUDA > wgpu discrete > wgpu integrated > CPU
let proc = Processor::new(Backend::Auto)?;

println!("Using backend: {}", proc.backend_name());
```

### Manual Selection

```rust
use vfx_compute::{Processor, Backend};

// Force specific backend
let proc = Processor::new(Backend::Wgpu)?;   // GPU via wgpu
let proc = Processor::new(Backend::Cuda)?;   // NVIDIA CUDA
let proc = Processor::new(Backend::Cpu)?;    // CPU with rayon
```

### Environment Override

```bash
# Force CPU backend
VFX_BACKEND=cpu myapp

# Force wgpu
VFX_BACKEND=wgpu myapp
```

### Check Available Backends

```rust
use vfx_compute::{detect_backends, describe_backends, Backend};

// List all backends with details
for info in detect_backends() {
    println!("[{}] {}: {}", 
        if info.available { "+" } else { "-" },
        info.name,
        info.description
    );
    if let Some(device) = &info.device {
        println!("  Device: {}", device);
    }
    if let Some(vram) = info.vram_total {
        println!("  VRAM: {} MB", vram / 1024 / 1024);
    }
}

// Or get formatted string
println!("{}", describe_backends());
```

## ComputeImage

GPU-compatible image container.

### Creating Images

```rust
use vfx_compute::ComputeImage;

// From raw f32 data (RGB)
let img = ComputeImage::from_f32(pixels, 1920, 1080, 3)?;

// From raw f32 data (RGBA)
let img = ComputeImage::from_f32(pixels, 1920, 1080, 4)?;

// Access data
assert_eq!(img.width, 1920);
assert_eq!(img.height, 1080);
assert_eq!(img.channels, 3);
let data: &[f32] = img.data();
```

## Color Operations

### Color Matrix

```rust
// 4x4 color matrix transform
let matrix = [
    1.1, 0.0, 0.0, 0.0,   // R
    0.0, 1.0, 0.0, 0.0,   // G
    0.0, 0.0, 0.9, 0.0,   // B
    0.0, 0.0, 0.0, 1.0,   // A (offset row)
];
proc.apply_matrix(&mut img, &matrix)?;
```

### Exposure

```rust
// Exposure in stops (uses matrix internally)
proc.apply_exposure(&mut img, 1.5)?;   // +1.5 stops = 2.83x
proc.apply_exposure(&mut img, -0.5)?;  // -0.5 stops = 0.71x
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

### 1D LUT

```rust
// Apply 1D LUT (e.g., gamma curve)
// LUT data: [R0, G0, B0, R1, G1, B1, ...]
let lut_size = 256;
let lut: Vec<f32> = (0..lut_size)
    .flat_map(|i| {
        let v = (i as f32 / 255.0).powf(1.0/2.2);  // gamma 2.2
        [v, v, v]
    })
    .collect();

proc.apply_lut1d(&mut img, &lut, 3)?;
```

### 3D LUT

```rust
// Apply 3D LUT (e.g., color grade)
// LUT data: size^3 * 3 floats
let size = 33;
let lut: Vec<f32> = load_cube_lut("grade.cube")?;

proc.apply_lut3d(&mut img, &lut, size)?;
```

## Image Operations

### Resize

```rust
use vfx_compute::ResizeFilter;

// Resize with different filters
let result = proc.resize(&img, 960, 540, ResizeFilter::Lanczos)?;
let result = proc.resize(&img, 960, 540, ResizeFilter::Bicubic)?;
let result = proc.resize(&img, 960, 540, ResizeFilter::Bilinear)?;
let result = proc.resize(&img, 960, 540, ResizeFilter::Nearest)?;
```

### Blur

```rust
// Gaussian blur with radius in pixels
proc.blur(&mut img, 5.0)?;
```

### Sharpen

```rust
// Unsharp mask sharpening
proc.sharpen(&mut img, 0.5)?;  // amount 0.0-1.0+
```

## Compositing

### Porter-Duff Over

```rust
// Composite foreground over background
// Both images must have alpha channel (4 channels)
proc.composite_over(&foreground, &mut background)?;
```

### Blend Modes

```rust
use vfx_compute::BlendMode;

// Blend with mode and opacity
proc.blend(&fg, &mut bg, BlendMode::Multiply, 1.0)?;
proc.blend(&fg, &mut bg, BlendMode::Screen, 0.5)?;
proc.blend(&fg, &mut bg, BlendMode::Overlay, 0.8)?;

// Available modes:
// Normal, Multiply, Screen, Add, Subtract,
// Overlay, SoftLight, HardLight, Difference
```

## Transform Operations

### Flip

```rust
proc.flip_h(&mut img)?;  // Horizontal flip
proc.flip_v(&mut img)?;  // Vertical flip
```

### Rotate

```rust
// Rotate 90 degrees clockwise (n times)
let rotated = proc.rotate_90(&img, 1)?;  // 90 CW
let rotated = proc.rotate_90(&img, 2)?;  // 180
let rotated = proc.rotate_90(&img, 3)?;  // 270 CW
```

### Crop

```rust
// Crop region (x, y, width, height)
let cropped = proc.crop(&img, 100, 100, 800, 600)?;
```

## Batch Processing

For efficiency, batch multiple color operations:

```rust
use vfx_compute::{ColorOpBatch, BatchOp};

// Create batch
let batch = ColorOpBatch::new()
    .add(BatchOp::Matrix(exposure_matrix))
    .add(BatchOp::Cdl(cdl))
    .add(BatchOp::Matrix(color_space_matrix));

// Apply all at once (fused when possible)
proc.apply_color_ops(&mut img, &batch)?;
```

## Processor Configuration

```rust
use vfx_compute::{Processor, ProcessorConfig};

let config = ProcessorConfig {
    tile_size: 4096,              // Max tile dimension
    stream_threshold: 128 * 1024 * 1024,  // 128MB
    ..Default::default()
};

let proc = Processor::builder()
    .backend(Backend::Auto)
    .config(config)
    .build()?;
```

## GPU Limits

```rust
// Query GPU capabilities
let limits = proc.limits();
println!("Max tile size: {}", limits.max_tile_dim);
println!("Max buffer: {} MB", limits.max_buffer_bytes / 1024 / 1024);
println!("Total VRAM: {} MB", limits.total_memory / 1024 / 1024);
println!("Available VRAM: {} MB", limits.available_memory / 1024 / 1024);
```

## Error Handling

```rust
use vfx_compute::{ComputeError, ComputeResult};

fn process() -> ComputeResult<()> {
    let proc = Processor::new(Backend::Auto)?;
    let mut img = ComputeImage::from_f32(data, w, h, 3)?;
    
    match proc.apply_exposure(&mut img, 1.5) {
        Ok(_) => println!("Success"),
        Err(ComputeError::BackendNotAvailable(msg)) => {
            eprintln!("Backend error: {}", msg);
        }
        Err(ComputeError::InvalidDimensions(w, h)) => {
            eprintln!("Invalid size: {}x{}", w, h);
        }
        Err(e) => return Err(e),
    }
    
    Ok(())
}
```

## Performance Tips

1. **Minimize uploads/downloads** - Keep data on GPU between operations
2. **Use batch operations** - `apply_color_ops` fuses matrix operations
3. **Choose right backend** - CUDA for NVIDIA, wgpu for cross-platform
4. **Configure tile size** - Larger = fewer transfers, more memory
5. **Check VRAM before large operations** - Use `limits().available_memory`

## Feature Flags

```toml
[dependencies.vfx-compute]
version = "0.1"
default-features = false
features = ["wgpu"]  # or "cuda"
```

| Feature | Description |
|---------|-------------|
| (none) | CPU backend only |
| `wgpu` | wgpu backend (Vulkan/Metal/DX12) |
| `cuda` | CUDA backend (NVIDIA GPUs) |
