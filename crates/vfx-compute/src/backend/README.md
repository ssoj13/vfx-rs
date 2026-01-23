# Backend Architecture

Unified compute backends for GPU/CPU image processing with automatic tiling and region-based I/O.

## Core Design

```
TiledExecutor<G: GpuPrimitives>
    +-- CpuPrimitives  (rayon parallelization)
    +-- WgpuPrimitives (Vulkan/Metal/DX12)
    +-- CudaPrimitives (NVIDIA CUDA)
```

All backends implement `GpuPrimitives` trait and use the same `TiledExecutor` for:
- Automatic VRAM-aware tiling for large images
- Region-based I/O for tiled processing
- Unified API regardless of backend

## Modules

| Module | Description |
|--------|-------------|
| `gpu_primitives.rs` | Core `GpuPrimitives` trait defining GPU operations |
| `executor.rs` | `TiledExecutor<G>` - unified executor with auto-tiling |
| `streaming.rs` | `StreamingSource`/`StreamingOutput` traits for large files |
| `tiling.rs` | `GpuLimits`, `ProcessingStrategy`, tile generation |
| `detect.rs` | Backend detection and auto-selection |
| `cpu_backend.rs` | `CpuPrimitives` - rayon-parallelized CPU ops |
| `wgpu_backend.rs` | `WgpuPrimitives` - wgpu compute shaders |
| `cuda_backend.rs` | `CudaPrimitives` - CUDA PTX kernels |

## GpuPrimitives Trait

Core abstraction for all backends:

```rust
pub trait GpuPrimitives: Send + Sync {
    type Handle: ImageHandle;
    
    // Transfer
    fn upload(&self, data: &[f32], w: u32, h: u32, c: u32) -> Result<Self::Handle>;
    fn download(&self, handle: &Self::Handle) -> Result<Vec<f32>>;
    fn allocate(&self, w: u32, h: u32, c: u32) -> Result<Self::Handle>;
    
    // Color operations
    fn exec_matrix(&self, src: &Self::Handle, dst: &mut Self::Handle, matrix: &[f32; 16]) -> Result<()>;
    fn exec_cdl(&self, src: &Self::Handle, dst: &mut Self::Handle, slope, offset, power, sat) -> Result<()>;
    fn exec_lut1d(&self, src: &Self::Handle, dst: &mut Self::Handle, lut: &[f32], channels: u32) -> Result<()>;
    fn exec_lut3d(&self, src: &Self::Handle, dst: &mut Self::Handle, lut: &[f32], size: u32) -> Result<()>;
    
    // Image operations
    fn exec_resize(&self, src: &Self::Handle, dst: &mut Self::Handle, filter: u32) -> Result<()>;
    fn exec_blur(&self, src: &Self::Handle, dst: &mut Self::Handle, radius: f32) -> Result<()>;
    
    // Info
    fn limits(&self) -> &GpuLimits;
    fn name(&self) -> &'static str;
}
```

## TiledExecutor

Wraps any `GpuPrimitives` implementation with automatic tiling:

```rust
let executor = TiledExecutor::new(WgpuPrimitives::new()?);

// Auto-tiles if image exceeds VRAM
executor.execute_color(&mut image, &ColorOp::Cdl { ... })?;

// Streaming for huge files
executor.execute_color_streaming(&mut exr_source, &mut exr_output, &op)?;
```

### ColorOp / ImageOp

```rust
pub enum ColorOp {
    Matrix([f32; 16]),
    Cdl { slope: [f32; 3], offset: [f32; 3], power: [f32; 3], saturation: f32 },
    Lut1d { lut: Vec<f32>, channels: u32 },
    Lut3d { lut: Vec<f32>, size: u32 },
}

pub enum ImageOp {
    Resize { width: u32, height: u32, filter: u32 },
    Blur { radius: f32 },
}
```

## Processing Strategy

Automatic strategy selection based on image size and GPU limits:

```rust
pub enum ProcessingStrategy {
    SinglePass,                          // Fits in VRAM
    Tiled { tile_size: u32, num_tiles: u32 },  // Exceeds VRAM, fits RAM
    Streaming { tile_size: u32 },        // Exceeds RAM
}

let strategy = ProcessingStrategy::recommend(width, height, channels, &limits);
```

Decision tree:
1. `SinglePass` - image fits in VRAM with 40% safety margin
2. `Tiled` - exceeds VRAM but fits in RAM (<8GB threshold)
3. `Streaming` - exceeds RAM, uses file-based I/O

## GpuLimits

VRAM-aware resource management:

```rust
pub struct GpuLimits {
    pub max_tile_dim: u32,      // Max texture dimension (e.g., 16384)
    pub max_buffer_bytes: u64,  // Max buffer size
    pub total_memory: u64,      // Total VRAM
    pub available_memory: u64,  // Usable VRAM (60% of total)
    pub detected: bool,         // Auto-detected vs defaults
}

let limits = GpuLimits::with_vram(8 * 1024 * 1024 * 1024); // 8 GB
let tile_size = limits.optimal_tile_size(8192, 8192, 4);   // Power-of-2 aligned
```

## Region-Based I/O

For tiled processing of large images:

```rust
pub trait StreamingSource: Send {
    fn dims(&self) -> (u32, u32);
    fn channels(&self) -> u32;
    fn read_region(&mut self, x: u32, y: u32, w: u32, h: u32) -> Result<Vec<f32>>;
}

pub trait StreamingOutput: Send {
    fn init(&mut self, width: u32, height: u32, channels: u32) -> Result<()>;
    fn write_region(&mut self, x: u32, y: u32, w: u32, h: u32, data: &[f32]) -> Result<()>;
    fn finish(&mut self) -> Result<()>;
}
```

Implementations:
- `MemorySource` / `MemoryOutput` - in-memory buffers
- `ExrStreamingSource` / `ExrStreamingOutput` - EXR files (feature = "io")
  - **Note:** Currently loads full file into memory; true tile-on-demand streaming is planned

## Backend Detection

Priority: CUDA (150) > wgpu (100) > CPU (10)

```rust
let backends = detect_backends();  // Vec<BackendInfo>
let best = select_best_backend();  // Backend::Cuda | Wgpu | Cpu

// Or create executor directly
let executor = create_executor(Backend::Auto)?;
```

## AnyExecutor

Dynamic dispatch when backend chosen at runtime:

```rust
pub enum AnyExecutor {
    Cpu(TiledExecutor<CpuPrimitives>),
    Wgpu(TiledExecutor<WgpuPrimitives>),  // feature = "wgpu"
    Cuda(TiledExecutor<CudaPrimitives>),  // feature = "cuda"
}

let executor = create_executor(Backend::Auto)?;
match executor {
    AnyExecutor::Cuda(e) => e.execute_color(&mut img, &op)?,
    AnyExecutor::Wgpu(e) => e.execute_color(&mut img, &op)?,
    AnyExecutor::Cpu(e)  => e.execute_color(&mut img, &op)?,
}
```

## Features

```toml
[features]
wgpu = ["dep:wgpu", "dep:pollster", "dep:bytemuck"]
cuda = ["dep:cudarc"]
io = ["dep:vfx-io"]  # For EXR streaming
```

## Constants

```rust
const VRAM_SAFETY_MARGIN: f64 = 0.4;  // Use max 60% of VRAM
const VRAM_TILE_OVERHEAD: f64 = 3.0;  // src + dst + intermediate
const DEFAULT_VRAM_BYTES: u64 = 2 GB;
const DEFAULT_MAX_TEXTURE_DIM: u32 = 16384;
```
