# GPU Backend Architecture

## Overview

`vfx-compute` provides GPU-accelerated image processing with automatic fallback to CPU. The architecture is based on:

- **GpuPrimitives trait** - Low-level GPU operations with associated types
- **TiledExecutor** - Automatic tiling for large images
- **AnyExecutor** - Dynamic dispatch for processors
- **Multi-backend support** - CPU (always), wgpu (Vulkan/Metal/DX12), CUDA

## Architecture Layers

```
+------------------------------------------+
|              User API                    |
|  (ColorProcessor, ImageProcessor, ops)   |
+------------------------------------------+
|              AnyExecutor                 |
|    (enum dispatch: Cpu/Wgpu/Cuda)        |
+------------------------------------------+
|            TiledExecutor<G>              |
|   (automatic tiling, region caching)     |
+------------------------------------------+
|          GpuPrimitives trait             |
|   (CpuPrimitives, WgpuPrimitives, etc)   |
+------------------------------------------+
|        Backend-specific impl             |
|    (rayon, wgpu shaders, CUDA PTX)       |
+------------------------------------------+
```

## Core Components

### GpuPrimitives Trait

The foundation of all GPU operations. Uses associated types for zero-cost abstraction:

```rust
pub trait GpuPrimitives: Send + Sync + 'static {
    type Handle: ImageHandle;
    
    // Memory management
    fn upload(&self, data: &[f32], w: u32, h: u32, c: u32) -> Result<Self::Handle>;
    fn download(&self, handle: &Self::Handle) -> Result<Vec<f32>>;
    fn allocate(&self, w: u32, h: u32, c: u32) -> Result<Self::Handle>;
    
    // Color operations
    fn exec_matrix(&self, src: &Self::Handle, dst: &mut Self::Handle, m: &[f32; 16]) -> Result<()>;
    fn exec_cdl(&self, src: &Self::Handle, dst: &mut Self::Handle, slope, offset, power, sat) -> Result<()>;
    fn exec_lut1d(&self, src: &Self::Handle, dst: &mut Self::Handle, lut: &[f32], channels: u32) -> Result<()>;
    fn exec_lut3d(&self, src: &Self::Handle, dst: &mut Self::Handle, lut: &[f32], size: u32) -> Result<()>;
    
    // Image operations
    fn exec_resize(&self, src: &Self::Handle, dst: &mut Self::Handle, filter: u32) -> Result<()>;
    fn exec_blur(&self, src: &Self::Handle, dst: &mut Self::Handle, radius: f32) -> Result<()>;
    
    // Compositing
    fn exec_composite_over(&self, fg: &Self::Handle, bg: &mut Self::Handle) -> Result<()>;
    fn exec_blend(&self, fg: &Self::Handle, bg: &mut Self::Handle, mode: u32, opacity: f32) -> Result<()>;
    
    // Transforms
    fn exec_flip_h(&self, handle: &mut Self::Handle) -> Result<()>;
    fn exec_flip_v(&self, handle: &mut Self::Handle) -> Result<()>;
    fn exec_rotate_90(&self, handle: &Self::Handle, n: u32) -> Result<Self::Handle>;
    fn exec_crop(&self, handle: &Self::Handle, x, y, w, h) -> Result<Self::Handle>;
    
    // Metadata
    fn limits(&self) -> &GpuLimits;
    fn name(&self) -> &'static str;
}
```

### ImageHandle Trait

Backend-specific image representation:

```rust
pub trait ImageHandle: Send + Sync + AsAny {
    fn dimensions(&self) -> (u32, u32, u32);  // width, height, channels
    fn size_bytes(&self) -> u64;
}
```

Implementations:
- `CpuImage` - `Vec<f32>` with dimensions
- `WgpuImage` - `wgpu::Buffer` with metadata
- `CudaImage` - `CudaSlice<f32>` with metadata

### TiledExecutor

Wraps any `GpuPrimitives` implementation for automatic tiling:

```rust
pub struct TiledExecutor<G: GpuPrimitives> {
    gpu: G,
    planner: TilePlanner,
    cache: RegionCache<G::Handle>,
}

impl<G: GpuPrimitives> TiledExecutor<G> {
    pub fn execute<F>(&self, w: u32, h: u32, c: u32, op: F) -> Result<Vec<f32>>
    where
        F: Fn(&G, &G::Handle, &mut G::Handle) -> Result<()>
    {
        // 1. Plan tiles based on GPU limits
        // 2. Process each tile
        // 3. Stitch results
    }
}
```

### AnyExecutor

Dynamic dispatch for user-facing API:

```rust
pub enum AnyExecutor {
    Cpu(TiledExecutor<CpuPrimitives>),
    #[cfg(feature = "wgpu")]
    Wgpu(TiledExecutor<WgpuPrimitives>),
    #[cfg(feature = "cuda")]
    Cuda(TiledExecutor<CudaPrimitives>),
}

impl AnyExecutor {
    pub fn execute_color(&self, img: &mut ComputeImage, op: &ColorOp) -> Result<()>;
    pub fn execute_color_chain(&self, img: &mut ComputeImage, ops: &[ColorOp]) -> Result<()>;
    pub fn execute_blur(&self, img: &mut ComputeImage, radius: f32) -> Result<()>;
    pub fn execute_resize(&self, img: &ComputeImage, w: u32, h: u32, filter: ResizeFilter) -> Result<ComputeImage>;
    // ...
}
```

## Backend Implementations

### CpuPrimitives

CPU backend using rayon for parallelization:

```rust
impl GpuPrimitives for CpuPrimitives {
    type Handle = CpuImage;
    
    fn exec_matrix(&self, src: &CpuImage, dst: &mut CpuImage, matrix: &[f32; 16]) -> Result<()> {
        dst.data.par_chunks_mut(channels)
            .zip(src.data.par_chunks(channels))
            .for_each(|(out, inp)| {
                // Matrix multiply in parallel
            });
        Ok(())
    }
}
```

Key features:
- Zero-copy when possible
- Parallel processing with rayon
- No GPU dependencies (always available)

### WgpuPrimitives

GPU backend using wgpu (Vulkan/Metal/DX12):

```rust
impl GpuPrimitives for WgpuPrimitives {
    type Handle = WgpuImage;
    
    fn exec_matrix(&self, src: &WgpuImage, dst: &mut WgpuImage, matrix: &[f32; 16]) -> Result<()> {
        // 1. Create bind group with matrix uniform
        // 2. Dispatch compute shader
        // 3. Wait for completion
    }
}
```

Shaders are compiled at build time via `build.rs` and embedded in the binary.

Key shaders (in `src/shaders/`):
- `matrix.wgsl` - Color matrix transform
- `cdl.wgsl` - CDL color correction
- `lut1d.wgsl`, `lut3d.wgsl` - LUT application
- `resize.wgsl` - Image scaling
- `blur.wgsl` - Gaussian blur (separable)
- `composite.wgsl` - Porter-Duff compositing
- `blend.wgsl` - Photoshop blend modes
- `flip.wgsl`, `rotate.wgsl`, `crop.wgsl` - Transforms

### CudaPrimitives

CUDA backend for NVIDIA GPUs:

```rust
impl GpuPrimitives for CudaPrimitives {
    type Handle = CudaImage;
    
    fn exec_matrix(&self, src: &CudaImage, dst: &mut CudaImage, matrix: &[f32; 16]) -> Result<()> {
        // Launch CUDA kernel
        unsafe { kernel.launch(cfg, src, dst, matrix) }?;
        Ok(())
    }
}
```

CUDA kernels are written in PTX and compiled at runtime using `cudarc`.

## Backend Selection

### Automatic Selection

```rust
pub fn select_best_backend() -> Backend {
    // Check VFX_BACKEND env var override
    // Try CUDA (priority 150)
    // Try wgpu discrete GPU (priority 100)
    // Try wgpu integrated GPU (priority 50)
    // Fallback to CPU (priority 10)
}
```

Priority rules:
1. CUDA > wgpu discrete > wgpu integrated > CPU
2. Software renderers (llvmpipe, swiftshader) are rejected
3. Environment variable `VFX_BACKEND=cpu|wgpu|cuda` overrides

### VRAM Detection

Cross-platform VRAM detection:

| Platform | Method |
|----------|--------|
| Windows | DXGI `IDXGIAdapter3::QueryVideoMemoryInfo` |
| macOS | Metal `MTLDevice::recommendedMaxWorkingSetSize` |
| Linux NVIDIA | NVML `nvmlDeviceGetMemoryInfo` |
| Linux AMD/Intel | sysfs `/sys/class/drm/card*/device/mem_info_vram_*` |
| Fallback | wgpu adapter limits |

## Memory Management

### GpuLimits

```rust
pub struct GpuLimits {
    pub max_tile_dim: u32,        // Max dimension per tile (e.g., 8192)
    pub max_buffer_bytes: u64,    // Max single buffer size
    pub total_memory: u64,        // Total GPU memory
    pub available_memory: u64,    // Free GPU memory
    pub detected: bool,           // True if values are from actual detection
}
```

### Tiling Strategy

For images exceeding GPU limits:

1. **TilePlanner** calculates optimal tile size based on:
   - GPU memory limits
   - Image dimensions
   - Operation requirements (blur needs padding)

2. **TileCluster** groups tiles by source regions to minimize uploads

3. **RegionCache** caches uploaded regions for reuse

## User-Facing API

### ColorProcessor

```rust
let processor = ColorProcessor::new(Backend::Auto)?;
processor.apply_matrix(&mut img, &matrix)?;
processor.apply_cdl(&mut img, &cdl)?;
processor.apply_lut1d(&mut img, &lut, 3)?;
```

### ImageProcessor

```rust
let processor = ImageProcessor::new(Backend::Wgpu)?;
let resized = processor.resize(&img, 1920, 1080, ResizeFilter::Lanczos)?;
processor.blur(&mut img, 5.0)?;
processor.composite_over(&fg, &mut bg)?;
```

### Processor (Full-featured)

```rust
let processor = Processor::builder()
    .backend(Backend::Auto)
    .tile_size(4096)
    .build()?;

// Color operations
processor.apply_matrix(&mut img, &matrix)?;
processor.apply_lut3d(&mut img, &lut, 33)?;

// Image operations
let result = processor.resize(&img, 1920, 1080, ResizeFilter::Bicubic)?;
processor.blur(&mut result, 2.5)?;

// Compositing
processor.composite_over(&fg, &mut bg)?;
processor.blend(&fg, &mut bg, BlendMode::Multiply, 0.5)?;
```

## Color Operations

### ColorOp Enum

```rust
pub enum ColorOp {
    Matrix([f32; 16]),
    Cdl { slope: [f32; 3], offset: [f32; 3], power: [f32; 3], saturation: f32 },
    Lut1d { lut: Vec<f32>, channels: u32 },
    Lut3d { lut: Vec<f32>, size: u32 },
}
```

### Chained Operations

Multiple color operations are fused when possible:

```rust
// These are fused into a single GPU pass when possible
let ops = vec![
    ColorOp::Matrix(exposure_matrix),
    ColorOp::Cdl { ... },
    ColorOp::Matrix(color_space_matrix),
];
executor.execute_color_chain(&mut img, &ops)?;
```

## Blend Modes

```rust
pub enum BlendMode {
    Normal = 0,
    Multiply = 1,
    Screen = 2,
    Add = 3,
    Subtract = 4,
    Overlay = 5,
    SoftLight = 6,
    HardLight = 7,
    Difference = 8,
}
```

## Feature Flags

```toml
[dependencies.vfx-compute]
version = "0.1"
features = ["wgpu"]  # or "cuda", or both
```

| Feature | Description |
|---------|-------------|
| `wgpu` | Enable wgpu backend (Vulkan/Metal/DX12) |
| `cuda` | Enable CUDA backend (requires NVIDIA GPU + CUDA toolkit) |

Without any features, only CPU backend is available.

## Error Handling

```rust
pub enum ComputeError {
    BackendNotAvailable(String),
    DeviceCreation(String),
    BufferCreation(String),
    ShaderCompilation(String),
    OperationFailed(String),
    InvalidDimensions(u32, u32),
    TilingFailed(String),
}
```

## Testing

Backend tests in `tests/backend_test.rs`:

```rust
#[test]
fn test_color_matrix_identity() {
    let processor = ColorProcessor::new(Backend::Cpu).unwrap();
    // Test with identity matrix...
}

#[cfg(feature = "wgpu")]
#[test]
fn test_wgpu_backend_check() {
    if Backend::Wgpu.is_available() {
        let processor = ColorProcessor::new(Backend::Wgpu).unwrap();
        assert_eq!(processor.backend_name(), "wgpu");
    }
}
```

## Performance Considerations

1. **Minimize uploads/downloads** - Keep data on GPU between operations
2. **Use operation chaining** - `execute_color_chain` fuses operations
3. **Choose appropriate backend** - CUDA for NVIDIA, wgpu for cross-platform
4. **Configure tile size** - Larger tiles = fewer transfers, but more memory
5. **Monitor VRAM** - Use `available_memory()` to check before large operations

## Changelog

### v0.1.0 (2026-01)
- Initial release with CPU/wgpu/CUDA backends
- GpuPrimitives trait with associated types
- TiledExecutor for automatic tiling
- AnyExecutor for dynamic dispatch
- Color operations: matrix, CDL, LUT1D, LUT3D
- Image operations: resize, blur, sharpen
- Compositing: Porter-Duff Over, blend modes
- Transforms: flip, rotate, crop
- Cross-platform VRAM detection
