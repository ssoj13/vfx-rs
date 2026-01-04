# GPU Color Processing Integration Plan

## Source: stool-rs/warper

Берём архитектуру GPU backend из stool-rs и адаптируем для color processing.

## Phase 1: Infrastructure (vfx-gpu/src/backend/)

### 1.1 Core Traits
- [ ] `gpu_primitives.rs` - GpuPrimitives trait адаптированный под color ops
- [ ] `tiling.rs` - GpuLimits, Tile, generate_tiles()
- [ ] `detect.rs` - Backend detection и auto-select

### 1.2 CPU Backend
- [ ] `cpu_backend.rs` - CpuBackend с rayon
- [ ] `cpu_primitives.rs` - CpuPrimitives implementation

### 1.3 wgpu Backend  
- [ ] `wgpu_backend.rs` - WgpuBackend
- [ ] `wgpu_primitives.rs` - WgpuPrimitives implementation

## Phase 2: Color Shaders (vfx-gpu/src/shaders/)

### 2.1 Basic Color Ops
- [ ] `color_matrix.wgsl` - 4x4 matrix transform (RGB primaries, etc)
- [ ] `cdl.wgsl` - CDL: slope, offset, power, saturation
- [ ] `exposure.wgsl` - exposure, gamma, contrast

### 2.2 LUT Application
- [ ] `lut1d.wgsl` - 1D LUT with interpolation
- [ ] `lut3d.wgsl` - 3D LUT tetrahedral interpolation

### 2.3 Transfer Functions
- [ ] `transfer.wgsl` - sRGB, Rec709, PQ, HLG, Log curves

## Phase 3: Image Ops Shaders (vfx-gpu/src/shaders/)

### 3.1 Resize
- [ ] `resize.wgsl` - Bilinear, Bicubic, Lanczos filters

### 3.2 Filters
- [ ] `blur.wgsl` - Gaussian blur (separable)
- [ ] `sharpen.wgsl` - Unsharp mask

## Phase 4: High-Level API

### 4.1 ColorProcessor
```rust
pub struct ColorProcessor {
    backend: Box<dyn ColorBackend>,
}

impl ColorProcessor {
    pub fn apply_matrix(&self, img: &mut GpuImage, matrix: &[f32; 16]);
    pub fn apply_cdl(&self, img: &mut GpuImage, cdl: &Cdl);
    pub fn apply_lut3d(&self, img: &mut GpuImage, lut: &Lut3D);
}
```

### 4.2 ImageProcessor
```rust
pub struct ImageProcessor {
    backend: Box<dyn ImageBackend>,
}

impl ImageProcessor {
    pub fn resize(&self, img: &GpuImage, w: u32, h: u32, filter: Filter) -> GpuImage;
    pub fn blur(&self, img: &mut GpuImage, radius: f32);
    pub fn sharpen(&self, img: &mut GpuImage, amount: f32);
}
```

## File Structure

```
crates/vfx-gpu/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── error.rs
    ├── backend/
    │   ├── mod.rs
    │   ├── gpu_primitives.rs
    │   ├── tiling.rs
    │   ├── detect.rs
    │   ├── cpu_backend.rs
    │   ├── cpu_primitives.rs
    │   ├── wgpu_backend.rs
    │   └── wgpu_primitives.rs
    ├── shaders/
    │   ├── mod.rs
    │   ├── color_matrix.wgsl
    │   ├── cdl.wgsl
    │   ├── lut1d.wgsl
    │   ├── lut3d.wgsl
    │   ├── transfer.wgsl
    │   ├── resize.wgsl
    │   ├── blur.wgsl
    │   └── sharpen.wgsl
    ├── image.rs          # GpuImage
    ├── color.rs          # ColorProcessor
    └── ops.rs            # ImageProcessor
```

## Dependencies

```toml
[dependencies]
wgpu = "24"
bytemuck = { version = "1.21", features = ["derive"] }
pollster = "0.4"
rayon = "1.11"
```
