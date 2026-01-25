# GPU Compute Internals

Low-level GPU architecture and compute shader implementation details.

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                  User-facing API (ops.rs)                       │
│           ColorProcessor / ImageProcessor                       │
├─────────────────────────────────────────────────────────────────┤
│                     AnyExecutor (enum)                          │
│         Cpu(TiledExecutor) | Wgpu(TiledExecutor) | Cuda(...)   │
├─────────────────────────────────────────────────────────────────┤
│              TiledExecutor<G: GpuPrimitives>                    │
│           Automatic tiling for large images                     │
├─────────────────────────────────────────────────────────────────┤
│                   GpuPrimitives trait                           │
│   upload/download + exec_* operations with associated types     │
├──────────────────┬─────────────────────┬────────────────────────┤
│   CpuPrimitives  │   WgpuPrimitives    │    CudaPrimitives      │
│     (rayon)      │  (compute shaders)  │     (cudarc)           │
└──────────────────┴─────────────────────┴────────────────────────┘
```

## GpuPrimitives Trait

Core abstraction with **associated types** for zero-cost abstraction:

```rust
pub trait GpuPrimitives: Send + Sync {
    /// Backend-specific image handle type.
    type Handle: ImageHandle;

    // === Memory Management ===
    fn upload(&self, data: &[f32], width: u32, height: u32, channels: u32) -> ComputeResult<Self::Handle>;
    fn download(&self, handle: &Self::Handle) -> ComputeResult<Vec<f32>>;
    fn allocate(&self, width: u32, height: u32, channels: u32) -> ComputeResult<Self::Handle>;

    // === Color Operations (src/dst pattern) ===
    fn exec_matrix(&self, src: &Self::Handle, dst: &mut Self::Handle, matrix: &[f32; 16]) -> ComputeResult<()>;
    fn exec_cdl(&self, src: &Self::Handle, dst: &mut Self::Handle,
                slope: [f32; 3], offset: [f32; 3], power: [f32; 3], sat: f32) -> ComputeResult<()>;
    fn exec_lut1d(&self, src: &Self::Handle, dst: &mut Self::Handle,
                  lut: &[f32], channels: u32) -> ComputeResult<()>;
    fn exec_lut3d(&self, src: &Self::Handle, dst: &mut Self::Handle,
                  lut: &[f32], size: u32) -> ComputeResult<()>;
    fn exec_lut3d_tetrahedral(&self, src: &Self::Handle, dst: &mut Self::Handle,
                              lut: &[f32], size: u32) -> ComputeResult<()>;

    // === Image Operations (src/dst pattern) ===
    fn exec_resize(&self, src: &Self::Handle, dst: &mut Self::Handle, filter: u32) -> ComputeResult<()>;
    fn exec_blur(&self, src: &Self::Handle, dst: &mut Self::Handle, radius: f32) -> ComputeResult<()>;

    // === Info ===
    fn limits(&self) -> &GpuLimits;  // Returns reference, not owned
    fn name(&self) -> &'static str;
}
```

**Note:** There is no `exec_exposure()` in GpuPrimitives. Exposure is handled via `exec_matrix()` with an exposure-scaling matrix. Compositing operations (`exec_composite_over`, `exec_blend`) and transform operations (`exec_flip_h`, `exec_flip_v`, `exec_rotate_90`) are also not part of the trait - they're implemented at higher levels.

## ImageHandle Trait

Metadata for GPU image buffers:

```rust
pub trait ImageHandle {
    fn dimensions(&self) -> (u32, u32, u32);  // width, height, channels
    fn size_bytes(&self) -> u64;  // Note: size_bytes(), not byte_size()
}
```

### Backend Implementations

Backend handle types are named `*Image`, not `*Handle`:

```rust
// CPU - just Vec<f32> in memory
pub struct CpuImage {
    pub data: Vec<f32>,
    pub width: u32,
    pub height: u32,
    pub channels: u32,
}

// wgpu - GPU buffer with staging
pub struct WgpuImage {
    pub buffer: wgpu::Buffer,
    pub staging: wgpu::Buffer,  // for readback
    pub width: u32,
    pub height: u32,
    pub channels: u32,
    pub size_bytes: u64,
}

// CUDA - device memory pointer
pub struct CudaImage {
    pub buffer: CudaSlice<f32>,
    pub width: u32,
    pub height: u32,
    pub channels: u32,
}
```

## TiledExecutor

Automatic tiling for large images. The actual implementation uses configuration, planner, and cache components:

```rust
// TiledExecutor uses internal components for tiling decisions
pub struct TiledExecutor<G: GpuPrimitives> {
    gpu: G,
    config: TilingConfig,
    planner: TilePlanner,
    cache: TileCache,
}

impl<G: GpuPrimitives> TiledExecutor<G> {
    pub fn new(gpu: G) -> Self { /* uses internal config/planner */ }

    /// Access underlying GPU primitives
    pub fn gpu(&self) -> &G { &self.gpu }

    /// Execute color operation with automatic tiling
    pub fn execute_color(&self, image: &mut ComputeImage, op: &ColorOp) -> ComputeResult<()>;

    /// Execute color operation with streaming I/O for huge files
    pub fn execute_color_streaming<S, O>(&self, src: &mut S, dst: &mut O, op: &ColorOp) -> ComputeResult<()>
    where S: StreamingSource, O: StreamingOutput;
}
```

**Note:** There is no `execute_tiled()` method taking a closure. Use `execute_color()` with `ColorOp` enum or `execute_color_streaming()` for streaming workflows.

### Tile Processing Flow

```
┌─────────────────────────────────────────────────────────────┐
│                    Original Image (8K x 8K)                  │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────┬─────────┬─────────┬─────────┐
│ Tile 0  │ Tile 1  │ Tile 2  │ Tile 3  │  ← Split into tiles
├─────────┼─────────┼─────────┼─────────┤    that fit in VRAM
│ Tile 4  │ Tile 5  │ Tile 6  │ Tile 7  │
├─────────┼─────────┼─────────┼─────────┤
│ Tile 8  │ Tile 9  │ Tile 10 │ Tile 11 │
├─────────┼─────────┼─────────┼─────────┤
│ Tile 12 │ Tile 13 │ Tile 14 │ Tile 15 │
└─────────┴─────────┴─────────┴─────────┘
                              │
              For each tile:  │
              ┌───────────────┴───────────────┐
              │ 1. Extract from source        │
              │ 2. Upload to GPU              │
              │ 3. Execute operation          │
              │ 4. Download from GPU          │
              │ 5. Insert back to destination │
              └───────────────────────────────┘
```

## wgpu Backend Implementation

### Device Initialization

```rust
impl WgpuPrimitives {
    pub fn new() -> ComputeResult<Self> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        
        let adapter = pollster::block_on(instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: None,
            },
        ))?;
        
        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor::default(),
            None,
        ))?;
        
        // Create all compute pipelines
        let pipelines = Pipelines::new(&device);
        
        Ok(Self { device, queue, pipelines })
    }
}
```

### Buffer Upload/Download

```rust
fn upload(&self, data: &[f32], w: u32, h: u32, c: u32) -> Result<WgpuHandle> {
    let size = (w * h * c) as usize * 4;
    
    // GPU storage buffer
    let buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Image"),
        contents: bytemuck::cast_slice(data),
        usage: wgpu::BufferUsages::STORAGE 
             | wgpu::BufferUsages::COPY_SRC 
             | wgpu::BufferUsages::COPY_DST,
    });
    
    // CPU-readable staging buffer
    let staging = self.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Staging"),
        size: size as u64,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    
    Ok(WgpuHandle { buffer, staging, width: w, height: h, channels: c })
}

fn download(&self, handle: &WgpuHandle) -> Result<Vec<f32>> {
    let size = handle.byte_size();
    
    // Copy GPU buffer → staging
    let mut encoder = self.device.create_command_encoder(&Default::default());
    encoder.copy_buffer_to_buffer(&handle.buffer, 0, &handle.staging, 0, size as u64);
    self.queue.submit([encoder.finish()]);
    
    // Map and read
    let slice = handle.staging.slice(..);
    slice.map_async(wgpu::MapMode::Read, |_| {});
    self.device.poll(wgpu::Maintain::Wait);
    
    let data = slice.get_mapped_range();
    let result: Vec<f32> = bytemuck::cast_slice(&data).to_vec();
    
    drop(data);
    handle.staging.unmap();
    
    Ok(result)
}
```

### Compute Shader Dispatch

```rust
fn exec_exposure(&self, handle: &mut WgpuHandle, stops: f32) -> Result<()> {
    let factor = 2.0f32.powf(stops);
    let pixel_count = handle.width * handle.height * handle.channels;
    
    // Create uniform buffer with parameters
    let params = ExposureParams { factor, pixel_count, _pad: [0; 2] };
    let params_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Params"),
        contents: bytemuck::bytes_of(&params),
        usage: wgpu::BufferUsages::UNIFORM,
    });
    
    // Create bind group
    let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Exposure"),
        layout: &self.pipelines.exposure.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: handle.buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: params_buffer.as_entire_binding(),
            },
        ],
    });
    
    // Dispatch compute shader
    let mut encoder = self.device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_compute_pass(&Default::default());
        pass.set_pipeline(&self.pipelines.exposure);
        pass.set_bind_group(0, &bind_group, &[]);
        
        let workgroups = (pixel_count + 255) / 256;
        pass.dispatch_workgroups(workgroups, 1, 1);
    }
    
    self.queue.submit([encoder.finish()]);
    Ok(())
}
```

## WGSL Shaders

### Exposure Shader

```wgsl
@group(0) @binding(0) var<storage, read_write> pixels: array<f32>;

struct Params {
    factor: f32,
    count: u32,
    _pad: vec2<u32>,
}

@group(0) @binding(1) var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let idx = id.x;
    if idx >= params.count { return; }
    pixels[idx] = pixels[idx] * params.factor;
}
```

### Color Matrix Shader

```wgsl
@group(0) @binding(0) var<storage, read_write> pixels: array<f32>;

struct Params {
    matrix: mat4x4<f32>,
    width: u32,
    height: u32,
    channels: u32,
    _pad: u32,
}

@group(0) @binding(1) var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let pixel_idx = id.x;
    let total_pixels = params.width * params.height;
    if pixel_idx >= total_pixels { return; }
    
    let base = pixel_idx * params.channels;
    
    // Load RGB (or RGBA)
    var color = vec4<f32>(
        pixels[base],
        pixels[base + 1],
        pixels[base + 2],
        select(1.0, pixels[base + 3], params.channels > 3)
    );
    
    // Apply 4x4 matrix
    let result = params.matrix * color;
    
    // Store back
    pixels[base] = result.x;
    pixels[base + 1] = result.y;
    pixels[base + 2] = result.z;
    if params.channels > 3 {
        pixels[base + 3] = result.w;
    }
}
```

### 3D LUT Shader

```wgsl
@group(0) @binding(0) var<storage, read_write> pixels: array<f32>;
@group(0) @binding(1) var lut: texture_3d<f32>;
@group(0) @binding(2) var lut_sampler: sampler;

struct Params {
    width: u32,
    height: u32,
    channels: u32,
    lut_size: u32,
}

@group(0) @binding(3) var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let pixel_idx = id.x;
    if pixel_idx >= params.width * params.height { return; }
    
    let base = pixel_idx * params.channels;
    
    // Get input RGB
    let r = clamp(pixels[base], 0.0, 1.0);
    let g = clamp(pixels[base + 1], 0.0, 1.0);
    let b = clamp(pixels[base + 2], 0.0, 1.0);
    
    // Sample 3D LUT with trilinear interpolation
    let coords = vec3<f32>(r, g, b);
    let result = textureSampleLevel(lut, lut_sampler, coords, 0.0);
    
    // Store result
    pixels[base] = result.r;
    pixels[base + 1] = result.g;
    pixels[base + 2] = result.b;
}
```

### Blend Mode Shader

```wgsl
@group(0) @binding(0) var<storage, read> fg: array<f32>;
@group(0) @binding(1) var<storage, read_write> bg: array<f32>;

struct Params {
    count: u32,
    channels: u32,
    mode: u32,     // 0=Normal, 1=Multiply, 2=Screen, etc.
    opacity: f32,
}

@group(0) @binding(2) var<uniform> params: Params;

fn blend_multiply(a: vec3<f32>, b: vec3<f32>) -> vec3<f32> {
    return a * b;
}

fn blend_screen(a: vec3<f32>, b: vec3<f32>) -> vec3<f32> {
    return 1.0 - (1.0 - a) * (1.0 - b);
}

fn blend_overlay(a: vec3<f32>, b: vec3<f32>) -> vec3<f32> {
    return select(
        1.0 - 2.0 * (1.0 - a) * (1.0 - b),
        2.0 * a * b,
        b < vec3<f32>(0.5)
    );
}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let idx = id.x;
    if idx >= params.count { return; }
    
    let base = idx * params.channels;
    
    let fg_rgb = vec3<f32>(fg[base], fg[base + 1], fg[base + 2]);
    let bg_rgb = vec3<f32>(bg[base], bg[base + 1], bg[base + 2]);
    
    var result: vec3<f32>;
    switch params.mode {
        case 0u: { result = fg_rgb; }                      // Normal
        case 1u: { result = blend_multiply(fg_rgb, bg_rgb); }
        case 2u: { result = blend_screen(fg_rgb, bg_rgb); }
        case 5u: { result = blend_overlay(fg_rgb, bg_rgb); }
        default: { result = fg_rgb; }
    }
    
    // Apply opacity
    result = mix(bg_rgb, result, params.opacity);
    
    bg[base] = result.x;
    bg[base + 1] = result.y;
    bg[base + 2] = result.z;
}
```

## CPU Backend Implementation

Uses rayon for parallel processing:

```rust
impl GpuPrimitives for CpuPrimitives {
    type Handle = CpuHandle;
    
    fn exec_exposure(&self, handle: &mut CpuHandle, stops: f32) -> Result<()> {
        let factor = 2.0f32.powf(stops);
        handle.data.par_iter_mut().for_each(|v| *v *= factor);
        Ok(())
    }
    
    fn exec_matrix(&self, handle: &mut CpuHandle, matrix: &[f32; 16]) -> Result<()> {
        let c = handle.channels as usize;
        handle.data.par_chunks_mut(c).for_each(|pixel| {
            let r = pixel[0];
            let g = pixel[1];
            let b = pixel[2];
            let a = if c > 3 { pixel[3] } else { 1.0 };
            
            // mat4 × vec4
            pixel[0] = matrix[0]*r + matrix[4]*g + matrix[8]*b  + matrix[12]*a;
            pixel[1] = matrix[1]*r + matrix[5]*g + matrix[9]*b  + matrix[13]*a;
            pixel[2] = matrix[2]*r + matrix[6]*g + matrix[10]*b + matrix[14]*a;
            if c > 3 {
                pixel[3] = matrix[3]*r + matrix[7]*g + matrix[11]*b + matrix[15]*a;
            }
        });
        Ok(())
    }
}
```

## VRAM Detection

Cross-platform GPU memory detection:

| Platform | Method | API |
|----------|--------|-----|
| Windows | DXGI | `IDXGIAdapter3::QueryVideoMemoryInfo` |
| macOS | Metal | `MTLDevice::recommendedMaxWorkingSetSize` |
| Linux NVIDIA | NVML | `nvmlDeviceGetMemoryInfo` |
| Linux AMD/Intel | sysfs | `/sys/class/drm/card*/device/mem_info_vram_*` |
| Fallback | wgpu | `Adapter::limits().max_buffer_size` |

```rust
pub fn detect_vram() -> VramInfo {
    #[cfg(windows)]     { detect_dxgi() }
    #[cfg(target_os = "macos")] { detect_metal() }
    #[cfg(target_os = "linux")] { detect_nvml().or_else(detect_sysfs) }
    .unwrap_or_else(|| detect_wgpu().unwrap_or_default())
}
```

## Backend Selection

Priority-based auto-selection:

| Backend | Priority | Condition |
|---------|----------|-----------|
| CUDA | 150 | NVIDIA GPU + CUDA toolkit |
| wgpu (discrete) | 100 | Discrete GPU detected |
| wgpu (integrated) | 50 | Integrated GPU only |
| CPU | 10 | Always available |

```rust
pub fn select_best_backend() -> Backend {
    // Check VFX_BACKEND env override first
    if let Some(name) = std::env::var("VFX_BACKEND").ok() {
        match name.to_lowercase().as_str() {
            "cpu" => return Backend::Cpu,
            "wgpu" | "gpu" => return Backend::Wgpu,
            "cuda" => return Backend::Cuda,
            _ => {}
        }
    }
    
    // Auto-select by priority
    detect_backends()
        .into_iter()
        .filter(|b| b.available)
        .max_by_key(|b| b.priority)
        .map(|b| b.backend)
        .unwrap_or(Backend::Cpu)
}
```

## Performance Tips

### Workgroup Size

```wgsl
// Good: Power of 2, 64-256 typical
@compute @workgroup_size(256)

// Bad: Non-power-of-2
@compute @workgroup_size(100)
```

### Memory Coalescing

```wgsl
// Good: Sequential access (coalesced)
let idx = global_id.x;
pixels[idx] = process(pixels[idx]);

// Bad: Strided access (cache misses)
let idx = global_id.x * stride;
```

### Minimize Transfers

```rust
// Bad: Transfer for each operation
for op in operations {
    let handle = gpu.upload(&data, w, h, c)?;
    gpu.exec_op(&mut handle, op)?;
    data = gpu.download(&handle)?;
}

// Good: Batch on GPU
let mut handle = gpu.upload(&data, w, h, c)?;
for op in operations {
    gpu.exec_op(&mut handle, op)?;
}
data = gpu.download(&handle)?;
```

### Use ColorOp Sequence

```rust
// Optimal: Fused operations via apply_color_ops
let ops = vec![
    ColorOp::Matrix(srgb_to_linear),
    ColorOp::Cdl { slope, offset, power, sat },
    ColorOp::Lut3d { lut: &lut_data, size: 33 },
];

processor.apply_color_ops(&mut img, &ops)?;  // Single upload/download
```

**Note:** Use `apply_color_ops()` instead of the non-existent `apply_batch()`. Exposure is applied via a matrix operation.
