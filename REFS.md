# Reference Code Catalog

Каталог полезных референсов из _ref/ для реализации vfx-compute.

## WGSL / wgpu Patterns

### stool-rs/warper - Warp Shader
**Path:** `_ref/stool-rs/warper/src/backend/warp.wgsl`

| Lines | Description |
|-------|-------------|
| 1-30 | Uniforms struct pattern: `struct Uniforms { width, height, filter_type, edge_mode }` |
| 32-50 | Texture/sampler bindings: `@group(0) @binding(0) var input_tex: texture_2d<f32>` |
| 52-80 | Edge handling: clamp, wrap, mirror, black modes |
| 82-150 | Filter kernels: bilinear, bicubic, mitchell, lanczos3 |
| 152-200 | Bicubic weights calculation |
| 202-280 | Mitchell-Netravali filter implementation |
| 282-350 | Lanczos3 with sinc function |
| 352-420 | EWA (Elliptical Weighted Average) sampling |
| 422-480 | Jacobian computation for adaptive AA |
| 482-523 | Main compute entry point pattern |

**Key patterns:**
```wgsl
@group(0) @binding(0) var input_tex: texture_2d<f32>;
@group(0) @binding(1) var output_tex: texture_storage_2d<rgba32float, write>;
@group(0) @binding(2) var tex_sampler: sampler;
@group(0) @binding(3) var<uniform> params: Uniforms;

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = textureDimensions(input_tex);
    if (gid.x >= dims.x || gid.y >= dims.y) { return; }
    // ...
}
```

### stool-rs/warper - Backend Implementation
**Path:** `_ref/stool-rs/warper/src/backend/mod.rs`

| Lines | Description |
|-------|-------------|
| 1-50 | WgpuBackend struct with device, queue, pipelines |
| 52-120 | Pipeline creation with compute shader |
| 122-180 | Buffer management: create_buffer, upload, download |
| 182-250 | Bind group creation pattern |
| 252-320 | Dispatch compute shader with workgroups |

---

## CPU Compositor Patterns

### playa/compositor - CPU Blend Modes
**Path:** `_ref/playa/src/entities/compositor.rs`

| Lines | Description |
|-------|-------------|
| 1-30 | BlendMode enum definition |
| 32-80 | `apply_blend(a, b, mode)` - all blend mode formulas |
| 82-120 | Porter-Duff Over implementation |
| 122-180 | CpuCompositor struct |
| 182-250 | Layer compositing loop |

**Blend mode formulas:**
```rust
match mode {
    Normal => a,
    Multiply => a * b,
    Screen => 1.0 - (1.0 - a) * (1.0 - b),
    Overlay => if b < 0.5 { 2.0 * a * b } else { 1.0 - 2.0 * (1.0 - a) * (1.0 - b) },
    Add => (a + b).min(1.0),
    Subtract => (a - b).max(0.0),
    SoftLight => /* ... */,
    HardLight => /* ... */,
    Difference => (a - b).abs(),
}
```

---

## GPU Compositor Patterns

### playa/gpu_compositor - OpenGL/GLSL
**Path:** `_ref/playa/src/entities/gpu_compositor.rs`

| Lines | Description |
|-------|-------------|
| 1-50 | GpuCompositor struct with FBO, textures |
| 52-120 | Shader compilation and program linking |
| 122-200 | GLSL blend shader source (all modes) |
| 202-280 | TextureGuard RAII pattern for cleanup |
| 282-350 | `upload_frame_to_texture()` |
| 352-420 | `download_texture_to_frame()` |
| 422-500 | Render pass setup |
| 502-600 | Blend mode uniform setting |
| 602-700 | Transform matrix uniforms |
| 702-800 | Multi-layer compositing |
| 802-892 | Cleanup and resource management |

**GLSL blend shader pattern (convertible to WGSL):**
```glsl
vec3 blend(vec3 a, vec3 b, int mode) {
    switch(mode) {
        case 0: return a; // Normal
        case 1: return a * b; // Multiply
        case 2: return 1.0 - (1.0 - a) * (1.0 - b); // Screen
        // ...
    }
}
```

---

## Image Buffer Patterns

### playa/frame - Frame Buffer
**Path:** `_ref/playa/src/entities/frame.rs`

| Lines | Description |
|-------|-------------|
| 1-40 | Frame struct: width, height, channels, data |
| 42-80 | Pixel access: `get_pixel(x, y)`, `set_pixel(x, y, rgba)` |
| 82-120 | Row iteration for parallel processing |
| 122-160 | Format conversion (u8 <-> f32) |

---

## Resize/Filter Patterns

### stool-rs/warper - Filter Kernels
**Path:** `_ref/stool-rs/warper/src/filters.rs`

| Lines | Description |
|-------|-------------|
| 1-30 | FilterType enum |
| 32-60 | Bilinear interpolation |
| 62-100 | Bicubic with configurable B,C |
| 102-140 | Mitchell-Netravali (B=1/3, C=1/3) |
| 142-180 | Lanczos with window size |
| 182-220 | Kernel weight calculation |

---

## Transform Patterns

### playa/transform - CPU Transforms
**Path:** `_ref/playa/src/entities/transform.rs`

| Lines | Description |
|-------|-------------|
| 1-40 | Flip horizontal/vertical |
| 42-80 | Rotate 90/180/270 |
| 82-120 | Crop with bounds checking |
| 122-160 | Scale with filter selection |

---

## Async/Parallel Patterns

### stool-rs - Rayon Integration
**Path:** `_ref/stool-rs/warper/src/cpu.rs`

| Lines | Description |
|-------|-------------|
| 1-40 | `par_chunks_mut` for row-parallel processing |
| 42-80 | Thread-local scratch buffers |
| 82-120 | Progress callback integration |

---

## Notes

- **WGSL workgroup size:** Use `@workgroup_size(16, 16)` for 2D image ops, `@workgroup_size(256)` for 1D
- **Buffer layout:** Always use `rgba32float` for compute storage textures
- **Dispatch calculation:** `dispatch_x = (width + 15) / 16`, `dispatch_y = (height + 15) / 16`
- **Edge handling:** Prefer clamp for most ops, black for composite
