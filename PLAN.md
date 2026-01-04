# VFX-RS: Project Status & Roadmap

> Consolidated from docs/, docs2/, ANALYSIS.md. Last updated: 2026-01-04

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
vfx-compute    CPU (rayon) + wgpu backends
               color_ops: matrix, CDL, transfer, LUT
               image_ops: resize, blur, composite, transform

Layer 2: High-Level APIs
------------------------
vfx-color      Color pipeline builder, ColorProcessor
vfx-ocio       OCIO config parsing (v1+v2), display pipeline
vfx-icc        ICC profile support (lcms2)

Layer 3: I/O & Applications
---------------------------
vfx-io         Format readers/writers, TextureSystem, ImageCache
vfx-ops        CPU image operations
vfx-cli        Command-line tools
```

---

## Implementation Status

### DONE - Core Types (vfx-core)
- [x] ColorSpace enum + traits  `crates/vfx-core/src/colorspace.rs`
- [x] Image<C,T,N> buffer  `crates/vfx-core/src/image.rs`
- [x] Pixel types (Rgba, Rgb)  `crates/vfx-core/src/pixel.rs`
- [x] Metadata container  `crates/vfx-core/src/metadata.rs`

### DONE - Math (vfx-math)
- [x] Vec3, Mat3, Mat4  `crates/vfx-math/src/vector.rs`, `matrix.rs`
- [x] Chromatic adaptation (Bradford, Von Kries, XYZ)  `crates/vfx-math/src/adaptation.rs`

### DONE - LUT (vfx-lut)
- [x] Lut1D + interpolation  `crates/vfx-lut/src/lut1d.rs`
- [x] Lut3D + trilinear/tetrahedral  `crates/vfx-lut/src/lut3d.rs`
- [x] .cube parser  `crates/vfx-lut/src/formats/cube.rs`
- [x] .clf parser  `crates/vfx-lut/src/formats/clf.rs`
- [x] .spi1d/.spi3d parser  `crates/vfx-lut/src/formats/spi.rs`
- [x] .cdl parser  `crates/vfx-lut/src/formats/cdl.rs`
- [x] Domain support  `crates/vfx-lut/src/lut3d.rs`

### DONE - Transfer Functions (vfx-transfer)
- [x] sRGB  `crates/vfx-transfer/src/srgb.rs`
- [x] Gamma 2.2/2.4  `crates/vfx-transfer/src/gamma.rs`
- [x] Rec.709 BT.1886  `crates/vfx-transfer/src/bt1886.rs`
- [x] PQ (ST 2084)  `crates/vfx-transfer/src/pq.rs`
- [x] HLG (BT.2100)  `crates/vfx-transfer/src/hlg.rs`
- [x] ACEScct/ACEScc  `crates/vfx-transfer/src/acescct.rs`
- [x] LogC (ARRI)  `crates/vfx-transfer/src/logc.rs`
- [x] S-Log3 (Sony)  `crates/vfx-transfer/src/slog.rs`
- [x] V-Log (Panasonic)  `crates/vfx-transfer/src/vlog.rs`

### DONE - Color Primaries (vfx-primaries)
- [x] sRGB, Rec.709  `crates/vfx-primaries/src/lib.rs`
- [x] Rec.2020  
- [x] DCI-P3, Display P3  
- [x] ACES AP0/AP1  
- [x] ProPhoto RGB, Adobe RGB  
- [x] RGB<->XYZ matrices  

### DONE - Color Pipeline (vfx-color)
- [x] ColorProcessor  `crates/vfx-color/src/processor.rs`
- [x] Pipeline builder  `crates/vfx-color/src/pipeline.rs`
- [x] CDL (ASC-CDL)  `crates/vfx-color/src/cdl.rs`
- [x] ACES RRT/ODT/IDT  `crates/vfx-color/src/aces.rs`

### DONE - OCIO Support (vfx-ocio)
- [x] Config parsing v1 + v2  `crates/vfx-ocio/src/config.rs` (ConfigVersion::V1, V2)
- [x] Colorspace definitions  
- [x] Display/View pipeline  
- [x] View transforms (v2)  `crates/vfx-ocio/src/display.rs`
- [x] Looks  
- [x] File rules (v2)  `crates/vfx-ocio/src/file_rules.rs`
- [x] Context variables ($SHOT, $SEQ)  `crates/vfx-ocio/src/context.rs`
- [x] Processor ops: Matrix, LUT1D/3D, CDL, Exponent, Log, Range, Group  `crates/vfx-ocio/src/processor.rs`
- [x] FixedFunction (RGB<->HSV, ACES Gamut)  
- [x] ExposureContrast  
- [x] BuiltinTransform (ACES)  `crates/vfx-ocio/src/builtin.rs`

### DONE - ICC Profiles (vfx-icc)
- [x] ICC v2/v4 via lcms2  `crates/vfx-icc/src/lib.rs`

### DONE - Image I/O (vfx-io)
- [x] EXR (multi-layer read/write)  `crates/vfx-io/src/exr.rs`
- [x] PNG  `crates/vfx-io/src/png.rs`
- [x] JPEG  `crates/vfx-io/src/jpeg.rs`
- [x] TIFF  `crates/vfx-io/src/tiff.rs`
- [x] DPX (8/10/16-bit)  `crates/vfx-io/src/dpx.rs`
- [x] HDR/RGBE  `crates/vfx-io/src/hdr.rs`
- [x] HEIF/HEIC + HDR metadata (PQ/HLG)  `crates/vfx-io/src/heif.rs`
- [x] Format detection (magic bytes)  `crates/vfx-io/src/detect.rs`
- [x] Metadata extraction  `crates/vfx-io/src/metadata.rs`
- [x] Image sequences  `crates/vfx-io/src/sequence.rs`
- [x] UDIM detection  `crates/vfx-io/src/udim.rs`
- [x] ImageCache (LRU, tile-based)  `crates/vfx-io/src/cache.rs`
- [x] TextureSystem (MIP, filtering, wrap modes)  `crates/vfx-io/src/texture.rs`

**NOTE:** ImageCache/TextureSystem load full images then extract tiles.
For >RAM images, need true tiled I/O (see P1.7).

### DONE - Image Operations (vfx-ops)
- [x] Resize (Nearest, Bilinear, Lanczos3, Mitchell)  `crates/vfx-ops/src/resize.rs`
- [x] Transform (crop, flip, rotate90/180/270, pad, tile)  `crates/vfx-ops/src/transform.rs`
- [x] Paste (with alpha)  `crates/vfx-ops/src/transform.rs`
- [x] Rotate arbitrary angle  `crates/vfx-ops/src/transform.rs`
- [x] Composite (Porter-Duff over)  `crates/vfx-ops/src/composite.rs`
- [x] Blend modes (10 modes)  `crates/vfx-ops/src/composite.rs`
- [x] Filter/convolution (blur, sharpen, edge)  `crates/vfx-ops/src/filter.rs`
- [x] Warp/distortion (barrel, pincushion, fisheye, twist, wave, spherize, ripple)  `crates/vfx-ops/src/warp.rs`
- [x] Premult/unpremult  `crates/vfx-ops/src/composite.rs`
- [x] Layer ops  `crates/vfx-ops/src/layer_ops.rs`
- [x] Parallel execution  `crates/vfx-ops/src/parallel.rs`

### DONE - GPU Compute (vfx-compute)
- [x] Backend trait  `crates/vfx-compute/src/backend/mod.rs`
- [x] CPU backend (rayon)  `crates/vfx-compute/src/backend/cpu_backend.rs`
- [x] wgpu backend  `crates/vfx-compute/src/backend/wgpu_backend.rs`
- [x] Tiling support  `crates/vfx-compute/src/backend/tiling.rs`
- [x] Backend detection (priority: wgpu > CPU)  `crates/vfx-compute/src/backend/detect.rs`
- [x] Color matrix shader  `crates/vfx-compute/src/shaders/color_matrix.wgsl`
- [x] CDL shader  `crates/vfx-compute/src/shaders/cdl.wgsl`
- [x] LUT1D/3D shaders  `crates/vfx-compute/src/shaders/lut1d.wgsl`, `lut3d.wgsl`
- [x] Resize shader  `crates/vfx-compute/src/shaders/resize.wgsl`
- [x] Blur shader  `crates/vfx-compute/src/shaders/blur.wgsl`
- [x] Composite shader  `crates/vfx-compute/src/shaders/composite.wgsl`
- [x] Transform shaders (flip, rotate)  `crates/vfx-compute/src/shaders/`
- [x] ColorProcessor API  `crates/vfx-compute/src/color.rs`
- [x] ImageProcessor API  `crates/vfx-compute/src/ops.rs`
- [x] Processor (unified)  `crates/vfx-compute/src/processor.rs`

### DONE - CLI (vfx-cli)
- [x] info, convert, resize, crop, diff, composite  `crates/vfx-cli/src/commands/`
- [x] blur, sharpen, color, lut, transform, maketx  
- [x] grep, batch  
- [x] layers, extract-layer, merge-layers  
- [x] channel-shuffle, channel-extract  
- [x] paste, rotate, warp, aces  

---

## TODO - Priority 1 (Important)

### P1.1 Additional Formats
- [ ] WebP (via image crate, feature gate)
- [ ] AVIF (via image crate, feature gate)
- [ ] JPEG2000 (JP2)

### P1.2 More Camera Curves
- [ ] S-Log2 (Sony)
- [ ] REDLog (RED)
- [ ] BMDFilm Gen5 (Blackmagic)

### P1.3 LUT Formats
- [ ] .3DL (Lustre/Flame/Nuke)
- [ ] .CTF (OCIO v2)

### P1.4 Image Processing
- [ ] FFT/IFFT
- [ ] Median filter
- [ ] Morphology (dilate/erode)

### P1.5 OCIO Config Parsing (complete v2)
- [ ] FixedFunction in config YAML (impl done in processor)
- [ ] ExposureContrast in config YAML
- [ ] LookTransform in config
- [ ] DisplayViewTransform in config

### P1.6 ACES Verification
- [ ] Create test suite comparing output vs ACES reference images
- [ ] Bit-accurate validation (within tolerance)

### P1.7 True Tiled I/O (CRITICAL for >RAM images)
Current ImageCache loads full image then extracts tiles.
For 8K+ EXR/TIFF that exceed RAM, need streaming I/O:
- [ ] StreamingSource trait (random-access tile provider)
- [ ] EXR tiled block reading via `exr::block::FilteredChunksReader`
- [ ] Double-buffered producer-consumer (overlap I/O and compute)
- [ ] Memory budgeting (auto-size tiles to fit VRAM)

---

## TODO - Priority 2 (Nice to Have)

### P2.1 Deep EXR (niche but important for compositing)
- [ ] Deep data model (per-pixel sample arrays)
- [ ] Deep EXR read/write (currently `.no_deep_data()`)
- [ ] Deep composite operations

### P2.2 GPU Shader Export
- [ ] GLSL shader generation
- [ ] HLSL shader generation
- [ ] Real-time preview pipeline

### P2.3 Advanced I/O
- [ ] Video I/O (FFmpeg integration)
- [ ] RAW camera support (libraw)
- [ ] PSD read-only

### P2.4 Advanced Ops (ImageBufAlgo parity)
- [ ] Text rendering (Freetype)
- [ ] Noise generation (Perlin)
- [ ] Demosaic
- [ ] Transpose
- [ ] Reorient (EXIF)
- [ ] Color matching
- [ ] Feature detection

### P2.5 EXR Advanced
- [ ] Tiled image read/write
- [ ] Full mipchain maketx output

### P2.5b TIFF Advanced
- [ ] Tiled image read/write
- [ ] Full mipchain maketx output

### P2.6 Python Bindings
- [ ] PyO3 bindings for pipeline integration
- [ ] NumPy array interop

### P2.7 Benchmarks
- [ ] Use vfx-bench to measure throughput vs C++ references
- [ ] Regression tests for performance
- [ ] Copy OpenImageIO test suite files in there if applicable

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

## Key Decisions Made

1. **vfx-gpu renamed to vfx-compute** - more accurate name (CPU + GPU)

2. **HEIF HDR metadata** - using libheif-rs NCLX color profiles
   - Read: extract TransferCharacteristics (PQ/HLG), ColorPrimaries
   - Write: only set_color_primaries() available (API limitation)

3. **Multi-layer EXR** - implemented via ExrReader::read_layers, ExrWriter::write_layers

4. **Warp operations** - 7 distortion types with bilinear interpolation

5. **Backend selection** - Auto (best available), CPU (always), wgpu (feature)
   - Priority system in detect.rs: wgpu (100) > CPU (10)

6. **OCIO v1 + v2** - Both config versions supported, auto-detected from version field

7. **Context variables** - Full support for $SHOT, $SEQ etc. in file paths

---

## Comparison with OIIO/OCIO

### Coverage vs OpenImageIO (~60%)
| Feature | Status | Notes |
|---------|--------|-------|
| Basic formats | DONE | EXR/PNG/JPEG/TIFF/DPX/HDR/HEIF |
| Multi-layer EXR | DONE | read_layers/write_layers |
| Deep images | TODO | P2.1 |
| Resize/transform/composite | DONE | |
| Blend modes | DONE | 10 modes |
| Warp/distortion | DONE | 7 types |
| ImageCache | PARTIAL | Exists but loads full image |
| TextureSystem | PARTIAL | MIP/filtering done, no true tiled I/O |
| ImageBufAlgo breadth | TODO | ~30% coverage |

### Coverage vs OpenColorIO (~85%)
| Feature | Status | Notes |
|---------|--------|-------|
| Config parsing v1/v2 | DONE | |
| Colorspaces/displays/looks | DONE | |
| Matrix/LUT/CDL/Exponent/Log | DONE | |
| ACES transforms | DONE | RRT/ODT/IDT |
| File rules (v2) | DONE | |
| Context variables | DONE | $SHOT, $SEQ |
| View transforms (v2) | DONE | |
| GPU processing | DONE | wgpu backend |
| Shader generation | TODO | P2.2 |
| Dynamic properties | PARTIAL | Context done, runtime update TBD |

---

## ANALYSIS.md Verification Summary

| Claim | Verified | Notes |
|-------|----------|-------|
| OCIO v1 + v2 parsing | YES | ConfigVersion::V1, V2 in config.rs |
| Transform Engine complete | YES | Matrix, CDL, LUT, curves all in processor.rs |
| TextureSystem with MIP | YES | texture.rs has bilinear/trilinear |
| ImageCache I/O issue | YES | Loads full image, not true tiled |
| Context variables | YES | context.rs with resolve() |
| Unified Compute Backend | YES | vfx-compute with CPU+wgpu |
| Backend detection priority | YES | detect.rs: wgpu=100, CPU=10 |
| Deep Data missing | YES | .no_deep_data() in exr.rs |
| Python bindings missing | YES | No PyO3 bindings yet |


## От человека:
Нужно унифицировать загрузку и запись всех изображений, сделать как в _ref/stool-rs стриминговую загрузку / запись _по умолчанию, СРАЗУ_. Тайлами или целиком - можно также использовать автовыбор стратегии из stool-rs, там всё есть. Вообще посмотри на stool-rs, там есть все три backend (включая cuda которую мы тоже можем оттуда спереть), все шейдера, и stool-rs отлажен и заточен на работу с огромными изображениями.
Мы можем всё целиком оттуда стянуть. Нужно подробное исследование, ultrathink here.
Нужна унификация всего, меньше clone, больше экономии памяти и скорости работы.
Берём все три backend из stool-rs (два уже есть), дорабатываем всё до parity с OpenImageIO, только ещё лучше - со встроенным color pipelint и стриминг тайловым IO по умолчанию.
В stool-rs также есть функции конвертации буферов, посмотри, надо ли использовать?

Deep EXR пока не надо, мы потом сделаем, это наинизший приоритет.
Python bidings - только после стабилизации Rust API.
Обратную совместимость не надо, ломаем.
Нужно чтобы было похоже на OIIO/OCIO, но можно улучшать и оптимизировать, делать лучше.

---

## STREAMING I/O MIGRATION PLAN

> Based on analysis of `_ref/stool-rs`. Last updated: 2026-01-04

### Key Decisions

1. **Keep multi-format PixelFormat** (U8/U16/F16/F32/U32) - unique vfx-rs feature
2. **Keep LayeredImage** for multi-layer EXR - critical for VFX
3. **Integrate color pipeline** into streaming - apply transforms on-the-fly
4. **Keep trait-based API** - adapt streaming to current style
5. **Always rustdocs + comments** - explain what/why/where

### Phase 1: Streaming Traits (vfx-io)

Add streaming infrastructure to `vfx-io`:

```
crates/vfx-io/src/
├── streaming/
│   ├── mod.rs           # Re-exports, should_use_streaming()
│   ├── traits.rs        # StreamingSource, StreamingOutput traits
│   ├── memory.rs        # MemorySource, MemoryOutput (fallback)
│   ├── tiff_stream.rs   # TiffStreamingSource (true random access)
│   ├── exr_stream.rs    # ExrStreamingSource (lazy loading)
│   └── factory.rs       # open_streaming(), create_streaming_output()
├── format.rs            # native_bpp(), MemoryEstimate (from stool-rs)
└── ... existing files
```

**Traits to port from stool-rs:**

```rust
/// Streaming source for reading image regions from disk.
/// 
/// Implementations can read arbitrary rectangular regions. Some formats
/// support true random access (TIFF), while others require full decode
/// on first access (PNG, JPEG, EXR currently).
pub trait StreamingSource: Send {
    /// Image dimensions (width, height).
    fn dimensions(&self) -> (u32, u32);
    
    /// Read a rectangular region, returning in native PixelFormat.
    fn read_region(&mut self, x: u32, y: u32, w: u32, h: u32) -> IoResult<ImageData>;
    
    /// Read region with color transform applied on-the-fly.
    fn read_region_with_color<P: ColorProcessor>(
        &mut self, x: u32, y: u32, w: u32, h: u32, processor: &P
    ) -> IoResult<ImageData>;
    
    /// True if format supports efficient region reading.
    fn supports_random_access(&self) -> bool;
    
    /// Native pixel format of source file.
    fn native_format(&self) -> PixelFormat;
    
    /// Native tile/strip size if tiled format.
    fn native_tile_size(&self) -> Option<(u32, u32)>;
}

/// Streaming output for writing image tiles to disk.
pub trait StreamingOutput: Send {
    /// Initialize output with dimensions and format.
    fn init(&mut self, width: u32, height: u32, format: PixelFormat) -> IoResult<()>;
    
    /// Write a tile with optional color transform.
    fn write_tile(&mut self, tile: &ImageData, x: u32, y: u32) -> IoResult<()>;
    
    /// Write tile with color transform applied.
    fn write_tile_with_color<P: ColorProcessor>(
        &mut self, tile: &ImageData, x: u32, y: u32, processor: &P
    ) -> IoResult<()>;
    
    /// Finalize and close the output file.
    fn finish(&mut self) -> IoResult<()>;
}
```

### Phase 2: Format Detection (vfx-io)

Add `format.rs` with memory estimation:

```rust
/// Memory estimate for a file.
pub struct MemoryEstimate {
    /// Bytes in native format (as stored in file).
    pub native_bytes: u64,
    /// Bytes if converted to f32 RGBA.
    pub f32_bytes: u64,
    /// Native bytes per pixel.
    pub native_bpp: u64,
    /// Dimensions.
    pub width: u32,
    pub height: u32,
}

/// Get actual bytes per pixel by reading file header.
/// Works for EXR (reads channel sample types), TIFF, PNG, JPEG, etc.
pub fn native_bpp<P: AsRef<Path>>(path: P) -> Option<u64>;

/// Estimate memory for a file.
pub fn estimate_file_memory<P: AsRef<Path>>(path: P) -> Option<MemoryEstimate>;

/// Check if streaming is recommended based on available RAM.
pub fn should_use_streaming(src_dims: (u32, u32), out_dims: (u32, u32)) -> bool;
```

### Phase 3: TIFF Streaming (vfx-io)

True random access via `tiff::decoder::read_chunk()`:

```rust
/// TIFF streaming source with true chunk-based random access.
/// 
/// Uses the `tiff` crate's `read_chunk()` API to read only the strips
/// or tiles needed for a requested region, without loading the entire image.
pub struct TiffStreamingSource {
    path: PathBuf,
    width: u32,
    height: u32,
    chunk_dims: (u32, u32),  // Strip height or tile dims
    is_tiled: bool,
    native_format: PixelFormat,
    decoder: tiff::decoder::Decoder<BufReader<File>>,
}
```

### Phase 4: EXR Streaming (vfx-io)

Lazy loading (header-only until first read):

```rust
/// EXR streaming source with lazy loading.
/// 
/// Currently loads the entire image on first region request and caches it.
/// True block-level access via exr::block API is planned for future.
pub struct ExrStreamingSource {
    path: PathBuf,
    width: u32,
    height: u32,
    cached_image: Option<ImageData>,  // Lazy-loaded
    layers: Option<LayeredImage>,     // For multi-layer support
}
```

### Phase 5: Auto-Streaming read/write (vfx-io)

Modify top-level API to auto-select streaming:

```rust
/// Reads an image, automatically using streaming for large files.
/// 
/// If the file exceeds 80% of available RAM, uses streaming mode.
/// Otherwise loads fully into memory for faster processing.
pub fn read<P: AsRef<Path>>(path: P) -> IoResult<ImageData> {
    let estimate = estimate_file_memory(&path)?;
    if should_use_streaming_for_estimate(&estimate) {
        // Return streaming wrapper that loads on demand
        read_streaming(path)
    } else {
        read_full(path)  // Current behavior
    }
}

/// Opens a streaming source for explicit control.
pub fn open_streaming<P: AsRef<Path>>(path: P) -> IoResult<Box<dyn StreamingSource>>;

/// Creates a streaming output for explicit control.
pub fn create_streaming_output<P: AsRef<Path>>(path: P) -> IoResult<Box<dyn StreamingOutput>>;
```

### Phase 6: GpuPrimitives Trait (vfx-compute)

Unified backend interface:

```rust
/// Core GPU operations abstracted for all backends.
/// 
/// Associated types allow backends to use their native handle types.
pub trait GpuPrimitives: Send + Sync {
    /// Backend-specific source handle type.
    type Source: SourceHandle;
    /// Backend-specific output handle type.
    type Output: OutputHandle;
    
    /// Upload image region to GPU memory.
    fn upload_source(&self, image: &ImageData) -> Result<Self::Source>;
    
    /// Upload without clone (takes ownership).
    fn upload_source_owned(&self, image: ImageData) -> Result<Self::Source>;
    
    /// Allocate output buffer on GPU.
    fn allocate_output(&self, width: u32, height: u32, format: PixelFormat) -> Result<Self::Output>;
    
    /// Download output from GPU to CPU.
    fn download_output(&self, output: &Self::Output) -> Result<ImageData>;
    
    /// Execute color transform kernel.
    fn execute_color<P: ColorProcessor>(
        &self,
        source: &Self::Source,
        output: &Self::Output,
        processor: &P,
    ) -> Result<()>;
    
    /// Get GPU limits for tiling decisions.
    fn limits(&self) -> &GpuLimits;
}
```

### Phase 7: Strategy Executor (vfx-compute)

Auto-select processing strategy:

```rust
/// Strategy for handling source image during processing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProcessingStrategy {
    /// Source fits comfortably in VRAM (<=40%).
    /// Upload entire source once, process all tiles with same GPU buffer.
    FullSource,
    
    /// Source partially fits in VRAM (40-80%).
    /// Cluster tiles by source region overlap to maximize reuse.
    RegionCache,
    
    /// Source exceeds VRAM (>80%) or exceeds texture limit.
    /// Adaptive tiling with dynamic subdivision.
    AdaptiveTiled,
    
    /// Source exceeds RAM - use StreamingSource.
    Streaming,
}

/// Unified executor for all strategies.
pub struct ProcessingExecutor<G: GpuPrimitives> {
    gpu: G,
    region_cache: Option<RegionCache<G::Source>>,
}
```

### Phase 8: Color Pipeline Integration

Apply color transforms during streaming:

```rust
/// Process large image with color transform, streaming if needed.
pub fn process_with_color<P: AsRef<Path>>(
    input: P,
    output: P,
    processor: &ColorProcessor,
) -> IoResult<()> {
    let estimate = estimate_file_memory(&input)?;
    let strategy = ProcessingStrategy::select(&estimate, &gpu.limits());
    
    match strategy {
        ProcessingStrategy::Streaming => {
            let mut source = open_streaming(&input)?;
            let mut output = create_streaming_output(&output)?;
            
            // Process tile by tile with color transform
            for tile in generate_tiles(source.dimensions(), tile_size) {
                let region = source.read_region(tile.x, tile.y, tile.w, tile.h)?;
                let processed = processor.apply(&region)?;
                output.write_tile(&processed, tile.x, tile.y)?;
            }
            
            output.finish()
        }
        _ => {
            // In-memory processing
            ...
        }
    }
}
```

### Phase 9: CUDA Backend (Later)

Port from stool-rs when ready:

```
crates/vfx-compute/src/backend/
├── cuda_backend.rs      # CudaBackend impl
├── cuda_primitives.rs   # CudaPrimitives impl
└── cuda_kernels.cu      # CUDA kernels (or PTX)
```

### Files to Port from stool-rs

| stool-rs file | → vfx-rs location | Notes |
|--------------|-------------------|-------|
| `streaming_io.rs` | `vfx-io/src/streaming/` | Split into traits.rs, memory.rs, etc. |
| `format.rs` | `vfx-io/src/format.rs` | Adapt for multi-format PixelFormat |
| `gpu_primitives.rs` | `vfx-compute/src/primitives.rs` | Adapt for ImageData |
| `strategy.rs` | `vfx-compute/src/strategy.rs` | Add Streaming strategy |
| `planner.rs` | `vfx-compute/src/planner.rs` | Morton sorting, binary search |
| `tiling.rs` | `vfx-compute/src/tiling.rs` | Tile, SourceRegion, generate_tiles |
| `region_cache.rs` | `vfx-compute/src/region_cache.rs` | LRU cache for GPU regions |
| `cuda_*.rs` | `vfx-compute/src/backend/cuda_*.rs` | Later |

### API Changes (Breaking)

1. `read()` may return lazy-loading wrapper for large files
2. `ImageData` gains `.region()` method for tile access
3. `FormatReader` trait extends with `read_region()` 
4. New `StreamingSource`/`StreamingOutput` traits
5. `Backend` trait in vfx-compute becomes `GpuPrimitives`
6. New `ProcessingStrategy` enum

### Memory Savings Example

65K×65K JPEG processing:

| Approach | RAM Usage |
|----------|----------|
| Current (full f32 RGBA) | ~64 GB |
| Native format (u8 RGB) | ~12 GB |
| Streaming (tiles) | ~400 MB |

