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
| RED Log3G12 | **Done** | OCIO RedCameras.cpp | Extended |
| BMD Film Gen5 | **Done** | Blackmagic spec | Pocket 6K Pro+ |
| ACEScc | **Done** | AMPAS S-2014-003 | Grading space |
| ACEScct | **Done** | AMPAS S-2016-001 | Grading with toe |

### Not Implemented

| Function | Priority | Reference |
|----------|----------|-----------|
| Canon Log (original) | Low | CanonCameras.cpp |
| DJI D-Log | Low | - |
| GoPro Protune | Low | - |

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
| CIE RGB | **Done** | E | Reference |

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
- D65 ↔ D50 (Bradford)
- D65 ↔ D60 (Bradford)

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

### Not Implemented

| Format | Priority | Reference |
|--------|----------|-----------|
| .1dl (Autodesk Discreet) | Medium | FileFormatDiscreet1DL.cpp |
| .hdl (Houdini) | Low | FileFormatHDL.cpp |
| .itx/.look (Iridas) | Low | FileFormatIridasItx.cpp |
| .mga (Pandora) | Low | FileFormatPandora.cpp |
| .cub (Truelight) | Low | FileFormatTruelight.cpp |
| .mtx (SPI Matrix) | Low | FileFormatSpiMtx.cpp |

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
| PSD | **Done** | No | Layers read only |
| TX (tiled) | **Done** | **Done** | Mipmapped textures |

### Optional Formats

| Format | Read | Write | Feature Flag |
|--------|------|-------|--------------|
| HEIF/HEIC | **Done** | **Done** | `heif` |
| WebP | **Done** | **Done** | `webp` |
| AVIF | **Done** | **Done** | `avif` |
| JPEG 2000 | **Done** | **Done** | `jp2` |

### Not Implemented

| Format | Priority | Notes |
|--------|----------|-------|
| ARRIRAW | Medium | Proprietary |
| REDCODE | Medium | Proprietary SDK |
| BRAW | Medium | Blackmagic SDK |
| CinemaDNG | Low | Raw DNG sequence |

---

## Grading Operations (vfx-ops)

### Implemented

| Operation | Status | Styles | Verified Against |
|-----------|--------|--------|-----------------|
| CDL (SOP) | **Done** | ASC_SOP, NoClamp | OCIO CDLOpData.cpp |
| CDL (Saturation) | **Done** | - | OCIO CDLOpData.cpp |
| ExposureContrast | **Done** | Linear, Video, Logarithmic | OCIO ExposureContrastOpData.cpp |
| GradingPrimary | **Done** | Log, Lin, Video | OCIO GradingPrimaryOpData.cpp |

### Not Implemented

| Operation | Priority | Reference |
|-----------|----------|-----------|
| GradingTone | High | GradingToneOpData.cpp |
| GradingRGBCurve | High | GradingRGBCurveOpData.cpp |
| GradingHueCurve | Medium | - |
| Range | Medium | RangeOpData.cpp |
| Allocation | Low | AllocationOpData.cpp |

---

## ACES (vfx-color, vfx-transfer)

### Core Transforms

| Transform | Status | Notes |
|-----------|--------|-------|
| ACES 2065-1 ↔ ACEScg | **Done** | AP0 ↔ AP1 matrix |
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

## Fixed Function Ops

### Implemented

| Function | Status | Notes |
|----------|--------|-------|
| RGB ↔ HSV | **Done** | - |
| RGB ↔ HSL | **Done** | - |
| XYZ ↔ xyY | **Done** | - |
| XYZ ↔ Lab | **Done** | - |

### Not Implemented (from OCIO)

| Function | Priority | Notes |
|----------|----------|-------|
| ACES_RED_MOD_03 | Medium | Red modifier |
| ACES_RED_MOD_10 | Medium | - |
| ACES_GLOW_03 | Medium | Glow effect |
| ACES_GLOW_10 | Medium | - |
| ACES_DARK_TO_DIM_10 | Low | - |
| REC2100_SURROUND | Low | - |
| XYZ_TO_uvY | Low | - |
| XYZ_TO_LUV | Low | - |

---

## GPU Compute (vfx-compute)

### Backends

| Backend | Status | Notes |
|---------|--------|-------|
| CPU (rayon) | **Done** | Parallel processing |
| wgpu (Vulkan/Metal/DX12) | Partial | Framework ready |
| CUDA | Not done | Planned via cudarc |

### Features

| Feature | Status | Notes |
|---------|--------|-------|
| Auto tiling | **Done** | VRAM-aware |
| VRAM detection | **Done** | Cross-platform |
| Operation fusion | **Done** | Sequential ops merged |

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

### Not Implemented

| Transform | Priority | Notes |
|-----------|----------|-------|
| GradingPrimaryTransform | High | Parsing ready |
| GradingToneTransform | High | - |
| GradingRGBCurveTransform | Medium | - |
| GroupTransform | Low | Container |
| DisplayViewTransform | Low | - |
| LookTransform | Low | - |

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
| Transfer Functions | 17 | 20 | 85% |
| Color Primaries | 16 | 16 | 100% |
| Chromatic Adaptation | 4 | 4 | 100% |
| LUT Formats | 9 | 15 | 60% |
| Image I/O | 12 | 16 | 75% |
| Grading Ops | 4 | 9 | 44% |
| ACES | 7 | 10 | 70% |
| Fixed Functions | 4 | 12 | 33% |
| GPU Compute | 1 | 3 | 33% |
| OCIO Transforms | 7 | 13 | 54% |
| Image Ops | 20+ | 20+ | ~100% |

**Overall OCIO parity: ~70%**
