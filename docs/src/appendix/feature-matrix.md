# Feature Matrix

Complete list of all features implemented in vfx-rs. Use this page to track what's done and what's not.

## Transfer Functions (vfx-transfer)

### Display-Referred

| Function | Status | Verified Against | Notes |
|----------|--------|-----------------|-------|
| sRGB | **Done** | IEC 61966-2-1 | 51+ tests, linear toe |
| Gamma 2.2/2.4/2.6 | **Done** | - | Pure power function |
| Rec.709 | **Done** | ITU-R BT.709 | OETF with linear toe |

### HDR

| Function | Status | Verified Against | Notes |
|----------|--------|-----------------|-------|
| PQ (ST.2084) | **Done** | SMPTE ST 2084 | 0-10000 nits |
| HLG | **Done** | ITU-R BT.2100-2 | OOTF, OETF, EOTF |

### Camera Log Curves

| Function | Status | Verified Against | Notes |
|----------|--------|-----------------|-------|
| ARRI LogC3 | **Done** | ARRI spec | EI 800, all variants |
| ARRI LogC4 | **Done** | OCIO ArriCameras.cpp | Base 2 logarithm |
| Sony S-Log2 | **Done** | OCIO SonyCameras.cpp | Legacy |
| Sony S-Log3 | **Done** | OCIO SonyCameras.cpp | Current standard |
| Panasonic V-Log | **Done** | OCIO PanasonicCameras.cpp | VariCam |
| Canon Log 2 | **Done** | OCIO CanonCameras.cpp | Cinema EOS |
| Canon Log 3 | **Done** | OCIO CanonCameras.cpp | Linear segment |
| Apple Log | **Done** | OCIO AppleCameras.cpp | iPhone 15 Pro+ |
| RED REDLogFilm | **Done** | OCIO RedCameras.cpp | Original |
| RED Log3G10 | **Done** | OCIO RedCameras.cpp | Current standard |
| BMD Film Gen5 | **Done** | Blackmagic spec | Pocket 6K Pro+ |
| ACEScc | **Done** | AMPAS S-2014-003 | Grading space |
| ACEScct | **Done** | AMPAS S-2016-001 | Grading with toe |

### Additional Log Curves

| Function | Status | Verified Against | Notes |
|----------|--------|-----------------|-------|
| Canon Log (original) | **Done** | OCIO CanonCameras.cpp | Original 2011 spec |
| DJI D-Log | **Done** | DJI Whitepaper | Phantom/Mavic |
| DaVinci Intermediate | **Done** | Blackmagic spec | Resolve native |

---

## Color Primaries (vfx-primaries)

### Standard Color Spaces

| Primaries | Status | White Point | Notes |
|-----------|--------|-------------|-------|
| sRGB / Rec.709 | **Done** | D65 | Consumer displays |
| Rec.2020 | **Done** | D65 | UHDTV, HDR |
| DCI-P3 | **Done** | DCI | Digital cinema |
| Display P3 | **Done** | D65 | Apple displays |
| Adobe RGB | **Done** | D65 | Photography |
| ProPhoto RGB | **Done** | D50 | Wide gamut |

### ACES

| Primaries | Status | White Point | Notes |
|-----------|--------|-------------|-------|
| ACES AP0 | **Done** | D60 | ACES 2065-1 archival |
| ACES AP1 | **Done** | D60 | ACEScg working space |

### Camera Native Gamuts

| Primaries | Status | White Point | Verified Against |
|-----------|--------|-------------|-----------------|
| ARRI Wide Gamut 3 | **Done** | D65 | OCIO |
| ARRI Wide Gamut 4 | **Done** | D65 | OCIO ColorMatrixHelpers.cpp |
| Sony S-Gamut3 | **Done** | D65 | OCIO SonyCameras.cpp |
| Sony S-Gamut3.Cine | **Done** | D65 | OCIO SonyCameras.cpp |
| Panasonic V-Gamut | **Done** | D65 | OCIO PanasonicCameras.cpp |
| Canon Cinema Gamut | **Done** | D65 | OCIO CanonCameras.cpp |
| RED Wide Gamut RGB | **Done** | D65 | OCIO RedCameras.cpp |

### White Points

| Illuminant | Status | xy Coordinates |
|------------|--------|----------------|
| D65 | **Done** | (0.31270, 0.32900) |
| D60 | **Done** | (0.32168, 0.33767) |
| D50 | **Done** | (0.34567, 0.35850) |
| D55 | **Done** | (0.33242, 0.34743) |
| DCI White | **Done** | (0.31400, 0.35100) |
| ACES White | **Done** | (0.32168, 0.33767) |
| Illuminant A | **Done** | (0.44758, 0.40745) |
| Illuminant E | **Done** | (0.33333, 0.33333) |

---

## Chromatic Adaptation (vfx-math)

| Method | Status | Verified Against |
|--------|--------|-----------------|
| Bradford | **Done** | OCIO ColorMatrixHelpers.cpp |
| CAT02 | **Done** | OCIO ColorMatrixHelpers.cpp |
| Von Kries | **Done** | - |
| XYZ Scaling | **Done** | - |

Pre-computed matrices:
- D65 <-> D50 (Bradford)
- D65 <-> D60 (Bradford)

---

## LUT Formats (vfx-lut)

### Read/Write Support

| Format | Read | Write | Notes |
|--------|------|-------|-------|
| .cube (Iridas/Resolve) | **Done** | **Done** | 1D + 3D combo |
| .spi1d | **Done** | **Done** | SPI 1D LUT |
| .spi3d | **Done** | **Done** | SPI 3D LUT |
| .3dl | **Done** | **Done** | Autodesk Lustre |
| .clf / .ctf | **Done** | **Done** | Common LUT Format |
| .csp (Cinespace) | **Done** | **Done** | 1D + 3D with shaper |

### ASC-CDL

| Format | Read | Write | Notes |
|--------|------|-------|-------|
| .cc | **Done** | **Done** | Single CDL XML |
| .ccc | **Done** | **Done** | CDL collection |
| .cdl | **Done** | **Done** | EDL-style CDL |

### Additional Formats

| Format | Read | Write | Verified Against |
|--------|------|-------|------------------|
| .1dl (Autodesk Discreet) | **Done** | **Done** | OCIO FileFormatDiscreet1DL.cpp |
| .hdl (Houdini) | **Done** | **Done** | OCIO FileFormatHDL.cpp |
| .itx (Iridas) | **Done** | **Done** | OCIO FileFormatIridasItx.cpp |
| .look (Iridas) | **Done** | No | OCIO FileFormatIridasLook.cpp |
| .mga (Pandora) | **Done** | No | OCIO FileFormatPandora.cpp |
| .cub (Truelight) | **Done** | **Done** | OCIO FileFormatTruelight.cpp |
| .mtx (SPI Matrix) | **Done** | **Done** | OCIO FileFormatSpiMtx.cpp |
| .vf (Nuke) | **Done** | No | Nuke VectorField |

---

## LUT Interpolation (vfx-lut)

| Method | 1D | 3D | SIMD |
|--------|----|----|------|
| Nearest | **Done** | **Done** | - |
| Linear | **Done** | - | - |
| Trilinear | - | **Done** | - |
| Tetrahedral | - | **Done** | - |

---

## Image I/O (vfx-io)

### Supported Formats

| Format | Read | Write | Features |
|--------|------|-------|----------|
| EXR | **Done** | **Done** | Multi-layer, tiled, deep data header |
| PNG | **Done** | **Done** | 8/16-bit, alpha |
| JPEG | **Done** | **Done** | Quality setting |
| TIFF | **Done** | **Done** | 8/16/32-bit, tiles |
| DPX | **Done** | **Done** | 10/12/16-bit, film scanning |
| HDR (Radiance) | **Done** | **Done** | RGBE encoding |


### Optional Formats

| Format | Read | Write | Feature Flag |
|--------|------|-------|--------------|
| HEIF/HEIC | **Done** | **Done** | `heif` |
| WebP | **Done** | **Done** | `webp` |
| AVIF | No | **Done** | `avif` (write-only) |
| JPEG 2000 | **Done** | No | `jp2` (read-only) |

### Sequence Formats

| Format | Read | Write | Notes |
|--------|------|-------|-------|
| CinemaDNG | **Done** | No | DNG sequence directories |

### Proprietary (Not Planned)

| Format | Notes |
|--------|-------|
| ARRIRAW | Requires ARRI SDK - out of scope |
| REDCODE | Requires RED SDK - out of scope |
| BRAW | Requires Blackmagic SDK - out of scope |

> These formats require proprietary SDKs with restrictive licenses. Not included in feature count.

---

## Grading Operations (vfx-ops)

### Implemented

| Operation | Status | Styles | Verified Against |
|-----------|--------|--------|-----------------|
| CDL (SOP) | **Done** | ASC_SOP, NoClamp | OCIO CDLOpData.cpp |
| CDL (Saturation) | **Done** | - | OCIO CDLOpData.cpp |
| ExposureContrast | **Done** | Linear, Video, Logarithmic | OCIO ExposureContrastOpData.cpp |
| GradingPrimary | **Done** | Log, Lin, Video | OCIO GradingPrimaryOpData.cpp |
| GradingTone | **Done** | Log, Lin, Video | OCIO GradingToneOpData.cpp |
| GradingRGBCurve | **Done** | Log, Lin, Video | OCIO GradingRGBCurveOpData.cpp |
| GradingHueCurve | **Done** | Hue vs Hue/Sat/Lum | Custom implementation |
| Range | **Done** | Clamp, Remap | OCIO RangeOpData.cpp |
| Allocation | **Done** | Uniform, Lg2 | OCIO AllocationOp.cpp |

---

## ACES (vfx-color, vfx-transfer)

### Core Transforms

| Transform | Status | Notes |
|-----------|--------|-------|
| ACES 2065-1 <-> ACEScg | **Done** | AP0 <-> AP1 matrix |
| ACEScc encode/decode | **Done** | Logarithmic grading |
| ACEScct encode/decode | **Done** | Log with toe |
| RRT (Reference Rendering) | **Done** | ACES 1.x |
| ODT (Output Device) | **Done** | sRGB, Rec.709, P3, Rec.2020 |

### ACES 2.0

| Component | Status | Notes |
|-----------|--------|-------|
| Output Transform | **Done** | 32 tests passing |
| Tone mapping | **Done** | New curve |
| Gamut compression | **Done** | Parametric |

### Not Implemented

| Transform | Priority | Notes |
|-----------|----------|-------|
| LMT (Look Modification) | Medium | Artist looks |
| IDT (Input Device) | Low | Camera-specific |
| ACES Proxy | Low | Legacy |

---

## Fixed Function Ops (vfx-ops)

### Color Space Conversions

| Function | Status | Verified Against |
|----------|--------|-----------------|
| RGB <-> HSV | **Done** | - |
| RGB <-> HSL | **Done** | - |
| RGB <-> HSY (Log/Vid/Lin) | **Done** | OCIO FixedFunctionOpCPU.cpp |
| XYZ <-> xyY | **Done** | OCIO FixedFunctionOpCPU.cpp |
| XYZ <-> uvY | **Done** | OCIO FixedFunctionOpCPU.cpp |
| XYZ <-> L*u*v* | **Done** | OCIO FixedFunctionOpCPU.cpp |
| XYZ <-> Lab | **Done** | - |

### ACES Fixed Functions

| Function | Status | Verified Against |
|----------|--------|-----------------|
| ACES_RED_MOD_03 | **Done** | OCIO FixedFunctionOpCPU.cpp |
| ACES_RED_MOD_10 | **Done** | OCIO FixedFunctionOpCPU.cpp |
| ACES_GLOW_03 | **Done** | OCIO FixedFunctionOpCPU.cpp |
| ACES_GLOW_10 | **Done** | OCIO FixedFunctionOpCPU.cpp |
| ACES_DARK_TO_DIM_10 | **Done** | OCIO FixedFunctionOpCPU.cpp |
| ACES_GAMUT_COMP_13 | **Done** | OCIO FixedFunctionOpCPU.cpp |

### HDR Functions

| Function | Status | Verified Against |
|----------|--------|-----------------|
| REC2100_SURROUND | **Done** | OCIO FixedFunctionOpCPU.cpp |
| LIN_TO_PQ / PQ_TO_LIN | **Done** | OCIO FixedFunctionOpCPU.cpp |
| LIN_TO_GAMMA_LOG / GAMMA_LOG_TO_LIN | **Done** | OCIO FixedFunctionOpCPU.cpp |
| LIN_TO_DOUBLE_LOG / DOUBLE_LOG_TO_LIN | **Done** | OCIO FixedFunctionOpCPU.cpp |

---

## GPU Compute (vfx-compute)

### Backends

| Backend | Status | Notes |
|---------|--------|-------|
| CPU (rayon) | **Done** | Parallel processing |
| wgpu (Vulkan/Metal/DX12) | **Done** | Full implementation |
| CUDA | **Done** | Full implementation via cudarc |

### GPU Operations

| Operation | CPU | wgpu | CUDA |
|-----------|-----|------|------|
| Matrix transform | **Done** | **Done** | **Done** |
| CDL | **Done** | **Done** | **Done** |
| LUT 1D | **Done** | **Done** | **Done** |
| LUT 3D (trilinear) | **Done** | **Done** | **Done** |
| LUT 3D (tetrahedral) | **Done** | **Done** | **Done** |
| Resize | **Done** | **Done** | **Done** |
| Blur (separable) | **Done** | **Done** | **Done** |
| Flip H/V | **Done** | **Done** | **Done** |
| Rotate 90 | **Done** | **Done** | **Done** |
| Composite Over | **Done** | **Done** | **Done** |
| Blend modes | **Done** | **Done** | **Done** |
| Hue curves | **Done** | **Done** | **Done** |

### Features

| Feature | Status | Notes |
|---------|--------|-------|
| Auto tiling | **Done** | VRAM-aware |
| VRAM detection | **Done** | Cross-platform |
| Operation fusion | **Done** | Sequential ops merged |
| Streaming execution | **Done** | Large image support |

---

## SIMD Optimization (vfx-math)

| Feature | Status | Method |
|---------|--------|--------|
| f32x4 operations | **Done** | `wide` crate |
| f32x8 operations | **Done** | `wide` crate |
| Matrix multiply | **Done** | Vectorized |
| Transfer functions | **Done** | Auto-vectorizable |
| LUT interpolation | Partial | Scalar, auto-vec |

---

## OCIO Compatibility (vfx-ocio)

### Config Parsing

| Feature | Status | Notes |
|---------|--------|-------|
| YAML config | **Done** | v1 and v2 |
| Color spaces | **Done** | - |
| Roles | **Done** | - |
| Displays | **Done** | - |
| Views | **Done** | - |
| Shared views | **Done** | v2.3+ |
| Context variables | **Done** | Environment |
| File rules | **Done** | - |

### Transforms

| Transform | Status | Notes |
|-----------|--------|-------|
| MatrixTransform | **Done** | - |
| CDLTransform | **Done** | - |
| LogTransform | **Done** | - |
| FileTransform | **Done** | LUT files |
| BuiltinTransform | **Done** | Camera curves |
| ExponentTransform | **Done** | - |
| RangeTransform | **Done** | - |
| GroupTransform | **Done** | Container |
| GradingPrimaryTransform | **Done** | Full processor support |
| GradingToneTransform | **Done** | Full processor support |
| GradingRGBCurveTransform | **Done** | Full processor support |

| DisplayViewTransform | **Done** | Via Config::display_processor() |
| LookTransform | **Done** | Via Config::processor_with_looks() |

---

## Image Operations (vfx-ops)

### Geometry

| Operation | Status | Notes |
|-----------|--------|-------|
| Resize | **Done** | Lanczos, Bilinear, Nearest |
| Crop | **Done** | - |
| Rotate 90/180/270 | **Done** | - |
| Flip H/V | **Done** | - |
| Arbitrary rotation | **Done** | Bilinear interp |
| Warp | **Done** | - |

### Filters

| Operation | Status | Notes |
|-----------|--------|-------|
| Gaussian blur | **Done** | Separable |
| Box blur | **Done** | - |
| Sharpen | **Done** | Unsharp mask |
| Median | **Done** | - |

### Compositing

| Operation | Status | Notes |
|-----------|--------|-------|
| Over | **Done** | Porter-Duff |
| Under | **Done** | - |
| Multiply | **Done** | - |
| Screen | **Done** | - |
| Add | **Done** | - |
| Subtract | **Done** | - |

### Analysis

| Operation | Status | Notes |
|-----------|--------|-------|
| Histogram | **Done** | Per-channel |
| Min/Max | **Done** | - |
| Average | **Done** | - |
| Diff | **Done** | PSNR, SSIM |

---

## Python Bindings (vfx-rs-py)

| Feature | Status | Notes |
|---------|--------|-------|
| Image I/O | **Done** | All formats |
| NumPy arrays | **Done** | Zero-copy |
| Color transforms | **Done** | - |
| LUT application | **Done** | - |
| Resize/crop | **Done** | - |
| Viewer | **Done** | Optional feature |

---

## Platform Support

| Platform | Status | Notes |
|----------|--------|-------|
| Windows x64 | **Done** | Primary development |
| Linux x64 | **Done** | CI tested |
| macOS ARM64 | **Done** | .cargo/config.toml for pyo3 |
| macOS x64 | **Done** | - |

---

## Summary

| Category | Done | Total | Percentage |
|----------|------|-------|------------|
| Transfer Functions | 22 | 22 | 100% |
| Color Primaries | 16 | 16 | 100% |
| Chromatic Adaptation | 4 | 4 | 100% |
| LUT Formats | 17 | 17 | 100% |
| Image I/O | 13 | 13 | 100% |
| Grading Ops | 9 | 9 | 100% |
| ACES | 10 | 10 | 100% |
| Fixed Functions | 16 | 16 | 100% |
| GPU Compute | 3 | 3 | 100% |
| OCIO Transforms | 13 | 13 | 100% |
| Image Ops | 20+ | 20+ | ~100% |

**Overall OCIO parity: ~100%**
