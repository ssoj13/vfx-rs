# VFX-RS: Project Status & Roadmap

> Consolidated from docs/, docs2/. Last updated: 2026-01-04

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
vfx-ocio       OCIO config parsing, display pipeline
vfx-icc        ICC profile support (lcms2)

Layer 3: I/O & Applications
---------------------------
vfx-io         Format readers/writers
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
- [x] Config parsing (YAML)  `crates/vfx-ocio/src/config.rs`
- [x] Colorspace definitions  
- [x] Display/View pipeline  
- [x] Looks  
- [x] File rules  `crates/vfx-ocio/src/file_rules.rs`
- [x] Processor ops: Matrix, LUT1D/3D, CDL, Exponent, Log, Range, Group  `crates/vfx-ocio/src/processor.rs`
- [x] FixedFunction (RGB<->HSV, ACES Gamut)  
- [x] ExposureContrast  
- [x] BuiltinTransform (ACES minimal)  `crates/vfx-ocio/src/builtin.rs`

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
- [x] Backend detection/auto-select  `crates/vfx-compute/src/backend/detect.rs`
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

### P1.1 Deep EXR - POSTPONE FOR NOW, SKIP IT.
- [ ] Deep data model (per-pixel sample arrays)
- [ ] Deep EXR read/write (currently disabled via `.no_deep_data()`)
- [ ] Deep composite operations

### P1.2 Additional Formats
- [ ] WebP (via image crate, feature gate)
- [ ] AVIF (via image crate, feature gate)
- [ ] JPEG2000 (JP2)

### P1.3 More Camera Curves
- [ ] S-Log2 (Sony)
- [ ] REDLog (RED)
- [ ] BMDFilm Gen5 (Blackmagic)

### P1.4 LUT Formats
- [ ] .3DL (Lustre/Flame/Nuke)
- [ ] .CTF (OCIO v2)

### P1.5 Image Processing
- [ ] FFT/IFFT
- [ ] Median filter
- [ ] Morphology (dilate/erode)

### P1.6 OCIO Config Parsing
- [ ] FixedFunction in config (done in processor, not config parser)
- [ ] ExposureContrast in config
- [ ] LookTransform
- [ ] DisplayViewTransform

---

## TODO - Priority 2 (Nice to Have)

### P2.1 Caching & Textures
- [ ] ImageCache system
- [ ] TextureSystem
- [ ] Full UDIM support (detection done, loading TODO)

### P2.2 GPU
- [ ] Shader generation (GLSL/HLSL export)
- [ ] Real-time preview pipeline

### P2.3 Advanced I/O
- [ ] Video I/O (FFmpeg integration)
- [ ] RAW camera support (libraw)
- [ ] PSD read-only

### P2.4 Advanced Ops
- [ ] Text rendering (Freetype)
- [ ] Noise generation (Perlin)
- [ ] Demosaic
- [ ] Transpose
- [ ] Reorient (EXIF)

### P2.5 EXR Advanced
- [ ] Tiled images
- [ ] Mipmap levels
- [ ] Full mipchain maketx output

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

---

## File Reference

| Module | Key Files |
|--------|-----------|
| Core types | `vfx-core/src/{colorspace,image,pixel}.rs` |
| Math | `vfx-math/src/{vector,matrix,adaptation}.rs` |
| LUT | `vfx-lut/src/{lut1d,lut3d}.rs`, `formats/{cube,clf,spi,cdl}.rs` |
| Transfer | `vfx-transfer/src/{srgb,pq,hlg,logc,slog,vlog,acescct}.rs` |
| Primaries | `vfx-primaries/src/lib.rs` |
| Color | `vfx-color/src/{processor,pipeline,cdl,aces}.rs` |
| OCIO | `vfx-ocio/src/{config,processor,builtin,file_rules}.rs` |
| I/O | `vfx-io/src/{exr,dpx,hdr,heif,png,jpeg,tiff,detect}.rs` |
| Ops | `vfx-ops/src/{resize,transform,composite,filter,warp}.rs` |
| Compute | `vfx-compute/src/{backend,shaders,color,ops,processor}.rs` |
| CLI | `vfx-cli/src/commands/*.rs` |

---

## Comparison with OIIO/OCIO

### Coverage vs OpenImageIO
| Feature | Status |
|---------|--------|
| Basic formats (EXR/PNG/JPEG/TIFF/DPX/HDR) | DONE |
| Multi-layer EXR | DONE |
| Deep images | TODO |
| Resize/transform/composite | DONE |
| Blend modes | DONE |
| Warp/distortion | DONE |
| ImageCache | TODO |
| TextureSystem | TODO |

### Coverage vs OpenColorIO
| Feature | Status |
|---------|--------|
| Config parsing | DONE |
| Colorspaces/displays/looks | DONE |
| Matrix/LUT/CDL/Exponent/Log | DONE |
| ACES transforms | DONE |
| File rules | DONE |
| GPU processing | DONE (wgpu) |
| Shader generation | TODO |

---

*Generated: 2026-01-04*
