# VFX-RS: Project Status & Roadmap

> Last updated: 2026-01-04

## Architecture

```
Layer 0: Types & Pure Math
--------------------------
vfx-core       Image<C,T,N>, ColorSpace, traits
vfx-math       Vec3, Mat3, Mat4, chromatic adaptation
vfx-lut        Lut1D, Lut3D (data + interpolation)
vfx-transfer   Transfer functions (sRGB, PQ, LogC, etc.)
vfx-primaries  Primaries, RGB<->XYZ matrices

Layer 1: Execution (unified backend)
------------------------------------
vfx-compute    TiledExecutor<G: GpuPrimitives>
               +-- CpuPrimitives  (rayon)
               +-- WgpuPrimitives (Vulkan/Metal/DX12)
               +-- CudaPrimitives (NVIDIA CUDA)
               
               Auto-tiling, streaming I/O, VRAM-aware

Layer 2: High-Level APIs
------------------------
vfx-color      Color pipeline builder, ColorProcessor
vfx-ocio       OCIO config parsing (v1+v2), display pipeline
vfx-icc        ICC profile support (lcms2)

Layer 3: I/O & Applications
---------------------------
vfx-io         Format readers/writers, TextureSystem, ImageCache
               Streaming I/O for large images
vfx-ops        CPU image operations
vfx-cli        Command-line tools
```

---

## Compute Backend Architecture

Unified backend design ported from stool-rs:

```
TiledExecutor<G: GpuPrimitives>
    +-- CpuPrimitives  (rayon parallelization)
    +-- WgpuPrimitives (Vulkan/Metal/DX12 via wgpu)
    +-- CudaPrimitives (NVIDIA CUDA via cudarc)
```

### GpuPrimitives Trait

```rust
pub trait GpuPrimitives: Send + Sync {
    type Handle: ImageHandle;
    
    fn upload(&self, data: &[f32], w: u32, h: u32, c: u32) -> Result<Self::Handle>;
    fn download(&self, handle: &Self::Handle) -> Result<Vec<f32>>;
    fn allocate(&self, w: u32, h: u32, c: u32) -> Result<Self::Handle>;
    
    fn exec_matrix(&self, src: &Handle, dst: &mut Handle, matrix: &[f32; 16]) -> Result<()>;
    fn exec_cdl(&self, src: &Handle, dst: &mut Handle, slope, offset, power, sat) -> Result<()>;
    fn exec_lut1d(&self, src: &Handle, dst: &mut Handle, lut: &[f32], channels: u32) -> Result<()>;
    fn exec_lut3d(&self, src: &Handle, dst: &mut Handle, lut: &[f32], size: u32) -> Result<()>;
    fn exec_resize(&self, src: &Handle, dst: &mut Handle, filter: u32) -> Result<()>;
    fn exec_blur(&self, src: &Handle, dst: &mut Handle, radius: f32) -> Result<()>;
    
    fn limits(&self) -> &GpuLimits;
    fn name(&self) -> &'static str;
}
```

### Processing Strategy

```rust
pub enum ProcessingStrategy {
    SinglePass,                              // Fits in VRAM
    Tiled { tile_size: u32, num_tiles: u32 }, // Exceeds VRAM, fits RAM  
    Streaming { tile_size: u32 },            // Exceeds RAM
}
```

Decision: VRAM < 60% = SinglePass, else Tiled, RAM > 8GB = Streaming.

### Backend Priority

```
CUDA (150) > wgpu (100) > CPU (10)
```

Auto-detection in `detect.rs`. CUDA preferred when available (faster for large images).

### Streaming I/O

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

Implementations: MemorySource, MemoryOutput, ExrStreamingSource, ExrStreamingOutput.

### Key Files

| File | Description |
|------|-------------|
| `backend/gpu_primitives.rs` | GpuPrimitives trait |
| `backend/executor.rs` | TiledExecutor with auto-tiling |
| `backend/streaming.rs` | StreamingSource/StreamingOutput |
| `backend/tiling.rs` | GpuLimits, ProcessingStrategy, Tile |
| `backend/detect.rs` | Backend detection (CUDA > wgpu > CPU) |
| `backend/cpu_backend.rs` | CpuPrimitives (rayon) |
| `backend/wgpu_backend.rs` | WgpuPrimitives (compute shaders) |
| `backend/cuda_backend.rs` | CudaPrimitives (PTX kernels) |

---

## Implementation Status

### DONE - Core (vfx-core, vfx-math, vfx-lut, vfx-transfer, vfx-primaries)
- [x] ColorSpace, Image<C,T,N>, Pixel types, Metadata
- [x] Vec3, Mat3, Mat4, chromatic adaptation
- [x] Lut1D, Lut3D with trilinear/tetrahedral interpolation
- [x] .cube, .clf, .spi1d/.spi3d, .cdl, .3dl, .ctf parsers
- [x] Transfer functions: sRGB, Gamma, BT.1886, PQ, HLG, ACEScct/cc, LogC, S-Log2/3, V-Log, REDLog, BMDFilm
- [x] Primaries: sRGB, Rec.709, Rec.2020, DCI-P3, Display P3, ACES AP0/AP1, ProPhoto, Adobe RGB

### DONE - Color Pipeline (vfx-color)
- [x] ColorProcessor, Pipeline builder, CDL, ACES RRT/ODT/IDT

### DONE - OCIO (vfx-ocio)
- [x] Config v1 + v2 parsing, colorspaces, displays, views, looks
- [x] File rules, context variables ($SHOT, $SEQ)
- [x] Processor ops: Matrix, LUT1D/3D, CDL, Exponent, Log, Range, Group
- [x] FixedFunction, ExposureContrast, BuiltinTransform

### DONE - ICC (vfx-icc)
- [x] ICC v2/v4 via lcms2

### DONE - I/O (vfx-io)
- [x] EXR (multi-layer), PNG, JPEG, TIFF, DPX, HDR/RGBE, HEIF/HEIC
- [x] Format detection, metadata extraction
- [x] Image sequences, UDIM detection
- [x] ImageCache (LRU, streaming for >512MB)
- [x] TextureSystem (MIP, filtering, wrap modes)
- [x] Streaming I/O: StreamingSource/StreamingOutput, TiffStreamingSource, ExrStreamingSource

### DONE - Operations (vfx-ops)
- [x] Resize (Nearest, Bilinear, Lanczos3, Mitchell)
- [x] Transform (crop, flip, rotate, pad, tile, paste)
- [x] Composite (Porter-Duff over, 10 blend modes)
- [x] Filter (blur, sharpen, edge, median, morphology)
- [x] Warp (barrel, pincushion, fisheye, twist, wave, spherize, ripple)
- [x] FFT/IFFT, premult/unpremult, layer ops

### DONE - Compute (vfx-compute)
- [x] TiledExecutor<G: GpuPrimitives> - unified executor
- [x] CpuPrimitives (rayon)
- [x] WgpuPrimitives (Vulkan/Metal/DX12)
- [x] CudaPrimitives (NVIDIA CUDA)
- [x] ProcessingStrategy (SinglePass/Tiled/Streaming)
- [x] GpuLimits, VRAM-aware tiling
- [x] StreamingSource/StreamingOutput in compute layer
- [x] Backend detection: CUDA (150) > wgpu (100) > CPU (10)
- [x] Shaders: color_matrix, cdl, lut1d, lut3d, resize, blur, composite, flip, rotate, crop

### DONE - CLI (vfx-cli)
- [x] info, convert, resize, crop, diff, composite, blur, sharpen
- [x] color, lut, transform, maketx, grep, batch
- [x] layers, extract-layer, merge-layers, channel-shuffle, channel-extract
- [x] paste, rotate, warp, aces

---

## TODO

### Priority 1 - Important (DONE)

| Task | Status | Notes |
|------|--------|-------|
| **WebP format** | DONE | Read/write via image crate |
| **AVIF format** | DONE | Write-only (dav1d decoder needs pkg-config setup) |
| **JPEG2000** | DONE | Read-only via jpeg2k crate |
| **EXR tiled blocks** | DONE | Tile structure detection, TODO: true block-level reading |

### Priority 2 - Nice to Have

| Task | Description |
|------|-------------|
| **GLSL export** | Generate GLSL shaders from color pipeline |
| **HLSL export** | Generate HLSL shaders from color pipeline |
| **Real-time preview** | Preview pipeline for interactive apps |
| **Video I/O** | FFmpeg integration for video read/write |
| **RAW camera** | libraw integration for camera RAW files |
| **PSD read** | Read-only PSD support |
| **Text rendering** | Freetype integration |
| **Noise generation** | Perlin noise |
| **Demosaic** | Bayer demosaicing |
| **Transpose** | Matrix transpose for images |
| **Reorient (EXIF)** | Auto-rotate based on EXIF orientation |
| **Color matching** | Histogram matching, color transfer |
| **Feature detection** | Edge detection, corner detection |
| **EXR tiled write** | Write tiled EXR files |
| **TIFF tiled I/O** | Read/write tiled TIFF |
| **Mipchain maketx** | Full mipmap chain generation |
| **Benchmarks** | vfx-bench suite, regression tests |

### Priority 3 - Later

| Task | Description |
|------|-------------|
| **Deep EXR** | Deep data model, deep composite |
| **Python bindings** | PyO3 bindings (after API stabilization) |

---

## Key Decisions

1. **TiledExecutor<G>** - All backends use same executor, auto-tiling/streaming
2. **CUDA priority** - CUDA (150) > wgpu (100) > CPU (10) when available
3. **Streaming by default** - Large images auto-stream, configurable threshold
4. **VRAM safety** - 40% margin, 3x overhead for src+dst+intermediate
5. **Power-of-2 tiles** - GPU efficiency, min 256px
6. **No backward compat** - Breaking changes OK, optimize for performance
7. **OCIO v1+v2** - Both supported, auto-detected
8. **Multi-format PixelFormat** - U8/U16/F16/F32/U32 preserved through pipeline

---

## Crate Dependency Graph

```
                        vfx-cli
                           |
       +-------+-------+---+---+-------+
       |       |       |       |       |
   vfx-io  vfx-ops  vfx-color  vfx-ocio  vfx-compute
       |       |       |       |           |
       +-------+---+---+-------+-----------+
                   |
               vfx-core
                   |
       +-----------+-----------+
       |           |           |
   vfx-lut   vfx-transfer  vfx-primaries
       |           |           |
       +-----------+-----------+
                   |
               vfx-math
```

---

## Comparison with OIIO/OCIO

### vs OpenImageIO (~65%)
| Feature | Status |
|---------|--------|
| Basic formats (EXR/PNG/JPEG/TIFF/DPX/HDR/HEIF) | DONE |
| Multi-layer EXR | DONE |
| Deep images | TODO (P3) |
| Resize/transform/composite | DONE |
| Blend modes | DONE (10 modes) |
| Warp/distortion | DONE (7 types) |
| ImageCache | DONE (with streaming) |
| TextureSystem | DONE (MIP/filtering) |
| ImageBufAlgo breadth | ~35% |

### vs OpenColorIO (~85%)
| Feature | Status |
|---------|--------|
| Config parsing v1/v2 | DONE |
| Colorspaces/displays/looks | DONE |
| Matrix/LUT/CDL/Exponent/Log | DONE |
| ACES transforms | DONE |
| File rules (v2) | DONE |
| Context variables | DONE |
| GPU processing | DONE (CPU/wgpu/CUDA) |
| Shader generation | TODO (P2) |
