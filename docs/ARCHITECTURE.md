# VFX-RS Clean Architecture

## Current State

```
vfx-core          Base types (ColorSpace, Image)
vfx-math          Vec3, Mat3, chromatic adaptation
vfx-lut           Lut1D, Lut3D
vfx-transfer      Transfer functions (sRGB, PQ, LogC, etc.)
vfx-primaries     Color space primaries and matrices
vfx-color         High-level color pipeline API
vfx-ops           Image operations (resize, blur, composite)
vfx-gpu           GPU backends (isolated, not integrated)
```

**Problems:**
1. vfx-color works with `[f32; 3]` pixels, no parallelism
2. vfx-ops works with `&[f32]` buffers, separate `parallel` module
3. vfx-gpu duplicates functionality, not integrated

## Target Architecture

```
Layer 0: Types & Pure Math (no execution)
──────────────────────────────────────────
vfx-core          Image<C,T,N>, ColorSpace, traits
vfx-math          Vec3, Mat3, Mat4, adaptation matrices
vfx-lut           Lut1D, Lut3D (data structures only)
vfx-transfer      Transfer functions (pure f32 -> f32)
vfx-primaries     Primaries, RGB<->XYZ matrices


Layer 1: Execution (single crate for all backends)
──────────────────────────────────────────────────
vfx-compute       Unified execution layer
├── Backend trait
├── CpuBackend    (rayon parallelism)
├── WgpuBackend   (GPU compute shaders)
├── Operations:
│   ├── color_ops.rs    matrix, CDL, transfer, LUT
│   └── image_ops.rs    resize, blur, composite, transform
└── Tiling support for large images


Layer 2: High-Level APIs
────────────────────────
vfx-color         Color pipeline builder (uses vfx-compute)
                  Pipeline, ColorProcessor, ColorSpace conversions

vfx-ocio          OCIO config support


Layer 3: Applications
─────────────────────
vfx-cli           Command-line tools
vfx-io            File I/O
```

## Key Changes

### 1. Rename vfx-gpu → vfx-compute

More accurate name - it's not GPU-only, it's the compute layer.

### 2. vfx-compute Operations

```rust
// color_ops.rs
pub trait ColorOps {
    fn apply_matrix(&self, data: &mut [f32], w: u32, h: u32, c: u32, matrix: &[f32; 16]);
    fn apply_cdl(&self, data: &mut [f32], w: u32, h: u32, c: u32, cdl: &Cdl);
    fn apply_transfer(&self, data: &mut [f32], w: u32, h: u32, c: u32, func: fn(f32) -> f32);
    fn apply_lut1d(&self, data: &mut [f32], w: u32, h: u32, c: u32, lut: &Lut1D);
    fn apply_lut3d(&self, data: &mut [f32], w: u32, h: u32, c: u32, lut: &Lut3D);
}

// image_ops.rs  
pub trait ImageOps {
    fn resize(&self, src: &[f32], sw: u32, sh: u32, c: u32, dw: u32, dh: u32, filter: Filter) -> Vec<f32>;
    fn blur(&self, data: &mut [f32], w: u32, h: u32, c: u32, radius: f32);
    fn sharpen(&self, data: &mut [f32], w: u32, h: u32, c: u32, amount: f32);
    fn composite(&self, fg: &[f32], bg: &[f32], w: u32, h: u32, c: u32, mode: BlendMode) -> Vec<f32>;
}
```

### 3. Backend Selection

```rust
pub enum Backend {
    Auto,       // Best available (wgpu > CPU)
    Cpu,        // Always available
    Wgpu,       // GPU compute (optional feature)
}

pub fn create_processor(backend: Backend) -> Box<dyn Processor>;
```

### 4. vfx-color Integration

```rust
// vfx-color uses vfx-compute for execution
impl ColorProcessor {
    pub fn new() -> Self {
        Self::with_backend(Backend::Auto)
    }
    
    pub fn with_backend(backend: Backend) -> Self {
        Self {
            compute: vfx_compute::create_processor(backend),
            // ...
        }
    }
    
    pub fn apply_batch(&mut self, pipeline: &Pipeline, pixels: &mut [[f32; 3]]) {
        // Uses self.compute internally
    }
}
```

### 5. vfx-ops Deprecation

Option A: Remove vfx-ops, move code to vfx-compute
Option B: vfx-ops becomes thin wrapper over vfx-compute
Option C: Keep vfx-ops as "simple CPU-only" for users who don't want backends

**Recommendation:** Option A - consolidate in vfx-compute

## Migration Path

### Phase 1: Prepare vfx-compute
1. Rename vfx-gpu → vfx-compute
2. Add color_ops.rs and image_ops.rs modules
3. Ensure CpuBackend covers all vfx-ops functionality
4. Add WgpuBackend implementations

### Phase 2: Integrate vfx-color
1. Add vfx-compute dependency to vfx-color
2. Add optional `backend` parameter to ColorProcessor
3. Implement parallel batch processing via vfx-compute

### Phase 3: Migrate vfx-ops users
1. Update vfx-cli to use vfx-compute
2. Update vfx-tests
3. Deprecate vfx-ops (keep as alias or remove)

### Phase 4: Cleanup
1. Remove duplicate code
2. Update documentation
3. Add benchmarks comparing backends

## API Examples

### Before (current)
```rust
// vfx-color - no parallelism
let mut proc = ColorProcessor::new();
let results: Vec<_> = pixels.iter().map(|p| proc.apply(&pipeline, *p)).collect();

// vfx-ops - separate parallel module
use vfx_ops::parallel;
let blurred = parallel::box_blur(&data, w, h, c, radius)?;

// vfx-gpu - isolated
let gpu = vfx_gpu::ColorProcessor::new(Backend::Wgpu)?;
gpu.apply_matrix(&mut img, &matrix)?;
```

### After (clean)
```rust
// vfx-color - with backend selection
let proc = ColorProcessor::with_backend(Backend::Auto);
proc.apply_batch(&pipeline, &mut pixels);  // Uses best backend

// vfx-compute - direct use
let compute = vfx_compute::create_processor(Backend::Wgpu)?;
compute.blur(&mut data, w, h, c, radius);
compute.resize(&data, sw, sh, c, dw, dh, Filter::Lanczos3);

// Same API, different backends
let cpu = vfx_compute::create_processor(Backend::Cpu)?;
let gpu = vfx_compute::create_processor(Backend::Wgpu)?;
// Both have identical methods
```

## File Structure After Refactoring

```
crates/
├── vfx-core/           # Types, traits
├── vfx-math/           # Math utilities  
├── vfx-lut/            # LUT data structures
├── vfx-transfer/       # Transfer functions
├── vfx-primaries/      # Color primaries
├── vfx-compute/        # RENAMED from vfx-gpu
│   ├── src/
│   │   ├── lib.rs
│   │   ├── backend/
│   │   │   ├── mod.rs
│   │   │   ├── traits.rs       # ColorOps, ImageOps traits
│   │   │   ├── cpu_backend.rs
│   │   │   ├── wgpu_backend.rs
│   │   │   └── tiling.rs
│   │   ├── color_ops.rs        # Matrix, CDL, LUT implementations
│   │   ├── image_ops.rs        # Resize, blur, composite
│   │   └── shaders/
│   └── Cargo.toml
├── vfx-color/          # High-level color API (uses vfx-compute)
├── vfx-ocio/           # OCIO support
├── vfx-io/             # File I/O
├── vfx-icc/            # ICC profiles
├── vfx-cli/            # CLI tools (uses vfx-compute)
├── vfx-bench/          # Benchmarks
└── vfx-tests/          # Integration tests
```
