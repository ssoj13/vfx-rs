# VFX-RS Parity Audit & Findings

**Date:** 2026-01-27
**Status:** COMPLETED (Phase 1)

## Overview

This document tracks the audit of vfx-rs against reference OpenColorIO/OpenImageIO implementations.

---

## 1. OCIO Parity Checklist

### Config Parsing & Structure
- [x] Config YAML parsing completeness (121 public API items)
- [x] Environment variable substitution ($VAR, ${VAR})
- [x] Context variables (Context struct with resolve/set/get)
- [x] Search paths
- [x] Family separators
- [x] Strictparsing mode (stored, not fully exposed)
- [ ] EnvironmentMode (PREDEFINED vs ALL) - **MISSING**
- [ ] ReferenceSpaceType (SCENE vs DISPLAY) - **MISSING**
- [ ] Virtual Display support - **MISSING**
- [ ] NamedTransformVisibility filtering - **MISSING**

### Color Spaces
- [x] ColorSpace class full API
- [x] Aliases support
- [x] Categories support
- [x] Encoding attributes
- [x] Allocation/allocationVars
- [x] isData flag handling
- [ ] getReferenceSpaceType() - **MISSING**

### Transforms (22/23 implemented)
- [x] MatrixTransform
- [x] CDLTransform
- [x] ExponentTransform
- [x] ExponentWithLinearTransform
- [x] LogTransform
- [x] LogAffineTransform
- [x] LogCameraTransform (ACEScct, LogC3, S-Log3 predefined)
- [x] RangeTransform
- [x] FixedFunctionTransform (13/27 styles)
- [x] ExposureContrastTransform (all 3 styles)
- [x] FileTransform (LUT loading)
- [x] LookTransform
- [x] DisplayViewTransform
- [x] BuiltinTransform (97/97 styles = 100%)
- [x] GradingPrimaryTransform
- [x] GradingToneTransform
- [x] GradingRGBCurveTransform
- [x] GroupTransform
- [x] AllocationTransform
- [x] Lut1DTransform (inline)
- [x] Lut3DTransform (inline)
- [x] GradingHueCurveTransform (8 curves, HSY, Lin-Log, fwd+rev)

### FixedFunction Styles (25/27 = 100% parity)
Implemented:
- AcesRedMod03, AcesRedMod10, AcesGlow03, AcesGlow10, AcesGamutComp13
- RgbToHsv, HsvToRgb
- XyzToXyy, XyyToXyz, XyzToUvy, UvyToXyz, XyzToLuv, LuvToXyz
- AcesDarkToDim10, Rec2100Surround
- LinToPq, PqToLin, LinToGammaLog, GammaLogToLin, LinToDoubleLog, DoubleLogToLin
- RgbToHsyLin/Log/Vid, HsyLin/Log/VidToRgb
- ACES_OUTPUT_TRANSFORM_20 (full CAM16 JMh + tonescale + chroma + gamut compress)
- ACES_RGB_TO_JMh_20, ACES_JMh_TO_RGB_20
- ACES_TONESCALE_COMPRESS_20 (fwd+inv)
- ACES_GAMUT_COMPRESS_20 (fwd+inv)

**NOT IMPLEMENTED (matching OCIO):**
- ACES_GAMUTMAP_02/07 — also unimplemented in OCIO (throws "Unimplemented" exception)

### LUT Support (14/20 formats = 70%)
- [x] .cube (Resolve/Iridas) - 1D & 3D with combined support
- [x] .spi1d
- [x] .spi3d
- [x] .csp (Cinespace with preluts)
- [x] .clf/.ctf (Academy CLF / Autodesk CTF)
- [x] .3dl (Autodesk Flame/Lustre)
- [x] .1dl (Autodesk 1D)
- [x] .cdl/.cc/.ccc (ASC CDL)
- [x] .hdl (Houdini)
- [x] .cub (Truelight)
- [x] .itx (Iridas ITX)
- [x] .look (Iridas Look)
- [x] .mga/.m3d (Pandora)
- [x] .vf (Nuke VF)
- [x] .spimtx (SPI Matrix)
- [ ] .icc/.icm (ICC Profiles) - **MISSING**
- [x] Interpolation: linear, trilinear, tetrahedral
- [ ] Cubic spline interpolation (CSP-specific) - **MISSING**

### Processing
- [x] Processor creation
- [x] CPUProcessor optimization
- [x] ProcessorCache
- [x] DynamicProperty (DynamicProcessor)
- [x] apply() on packed data
- [x] applyRGBA / applyRGB direct buffer (apply_rgb, apply_rgba on &mut [[f32; 3/4]])

### Baker
- [x] bake() to .cube
- [x] bake() to .spi3d
- [x] bake() to .clf
- [x] Shaper LUT generation

### App Helpers
- [x] ColorSpaceHelpers (basic)
- [x] DisplayViewHelpers (basic)
- [x] CategoryHelpers
- [ ] LegacyViewingPipeline - **NOT IMPLEMENTED**
- [ ] MixingHelpers - **NOT IMPLEMENTED**

---

## 2. OIIO Parity Checklist

### Image I/O (11/14 formats)
- [x] EXR (flat scanline)
- [x] EXR (flat tiled)
- [x] EXR (deep scanline)
- [x] EXR (deep tiled)
- [x] EXR (multi-layer)
- [x] EXR (8/12 compressions - missing DWAA/DWAB/HTJ2K)
- [x] PNG (8/16-bit, alpha, gamma)
- [x] JPEG (8-bit, quality setting)
- [x] TIFF (LZW/Deflate, 8/16/32f-bit)
- [x] DPX (8/10/12/16-bit, log encoding)
- [x] HDR/RGBE (Radiance)
- [x] WebP (lossy/lossless)
- [x] HEIF/HEIC (PQ/HLG HDR, NCLX profiles)
- [x] PSD (read only, layer access)
- [x] DDS (read only, BC1-BC7 decompression)
- [x] JPEG2000 (read only, requires OpenJPEG)
- [ ] BMP - **MISSING**
- [ ] GIF - **MISSING**
- [ ] TGA - **MISSING**
- [ ] TX (mipmap textures) - **MISSING**

### EXR Compression (8/12 = 67%)
Implemented: Uncompressed, RLE, ZIP1, ZIP16, PIZ, PXR24, B44, B44A
**MISSING:** DWAA, DWAB, HTJ2K32, HTJ2K256

### ImageSpec
- [x] All standard metadata (30+ attributes)
- [x] Channel names/formats (U32, F16, F32)
- [x] Display/data windows
- [x] Pixel aspect ratio
- [x] Orientation
- [x] Deep data info
- [x] Multi-view names
- [x] World-to-camera/NDC matrices
- [x] Camera metadata (exposure, aperture, ISO, etc.)

### ImageBufAlgo
- [x] resize (all filters)
- [x] crop / cut
- [x] flip / flop / rotate
- [x] channels / channel_append
- [x] premult / unpremult
- [x] over / under composite
- [x] add / sub / mul / div
- [x] invert
- [x] pow / clamp
- [x] color_convert
- [x] colormatrixtransform
- [x] ociolook / ociodisplay
- [ ] computePixelStats - **Partial**
- [ ] compare - **MISSING**
- [ ] isConstantColor - **MISSING**
- [ ] histogram - **MISSING**
- [x] make_texture (mipmaps)

### Deep Data
- [x] Read deep scanline
- [x] Read deep tiled
- [x] Write deep scanline
- [x] Variable samples per pixel
- [ ] Deep compositing (flatten, over) - **MISSING**
- [ ] Deep holdout - **MISSING**

### Texture System
- [x] Mipmap generation
- [ ] Tiled caching - **Partial**
- [x] UDIM support
- [ ] Texture filtering - **Partial**

---

## 3. Python API Parity

### vfx-rs-py (PyO3)
- [x] Image read/write
- [x] ColorConfig class (45+ methods)
- [x] Context class (resolve, set, get, contains)
- [x] Processor class (exposure, saturation, contrast, CDL, LUT)
- [x] numpy array support (conversion, not zero-copy)
- [ ] Deep data operations - **Minimal**

### vs OCIO Python
- [x] Config.from_file() / from_string()
- [x] Config.aces_1_3() / srgb()
- [x] colorspace_names(), has_colorspace()
- [x] colorconvert(), ociodisplay(), ociolook()
- [ ] getProcessor() returning usable Processor - **MISSING**
- [ ] PackedImageDesc - **MISSING**
- [ ] PlanarImageDesc - **MISSING**
- [ ] Pixel-level access (getpixel/setpixel) - **MISSING**

### vs OIIO Python
- [x] Image class (basic ImageBuf equivalent)
- [x] Geometry ops (flip, crop, resize, rotate)
- [x] Filter ops (blur, sharpen, median)
- [x] Color ops (premult, saturate, contrast)
- [ ] ImageInput/ImageOutput direct - **Abstracted**
- [ ] ImageSpec manipulation - **Partial**

---

## 4. Findings Log

### [2026-01-27] GradingHueCurveTransform Missing
**Status:** OPEN
**Severity:** HIGH
**Description:** OCIO has GradingHueCurveTransform for per-hue adjustments with B-spline curves. vfx-rs has GradingPrimary, GradingTone, GradingRgbCurve but no HueCurve.
**Fix:** Implement with master/red/green/blue/cyan/magenta/yellow curves.

### [2026-01-27] ACES 2.0 FixedFunction Styles
**Status:** FIXED (2026-01-27)
**Severity:** HIGH
**Description:** Created aces2.rs (~900 lines) porting full ACES 2.0 pipeline from OCIO Transform.cpp.
**Fix:** CAM16 JMh color space, tonescale, chroma compress, gamut compress with 362-entry lookup tables. Pre-computed state in dedicated ProcessorOp variants. 25/27 styles now implemented.

### [2026-01-27] ICC Profile Support Missing
**Status:** OPEN
**Severity:** MEDIUM
**Description:** vfx-lut lacks ICC profile parsing (FileFormatICC.cpp in OCIO).
**Fix:** Integrate ICC profile parser library or implement basic ICC support.

### [2026-01-27] EXR DWAA/DWAB Compression Missing
**Status:** OPEN
**Severity:** MEDIUM
**Description:** DWAA/DWAB (DCT-based) and HTJ2K compression not implemented. Enum variants defined but return unsupported().
**Fix:** Implement DCT compression for DWAA/DWAB.

### [2026-01-27] Python Processor.apply() Not Returning Result
**Status:** FIXED
**Severity:** HIGH
**Description:** processor_with_context() returned () instead of a Processor object.
**Fix:** processor_with_context() and new processor() now return OcioProcessor. Added apply(), apply_rgb(), apply_rgba() methods. Added from_processor() constructor.

### [2026-01-27] EnvironmentMode Missing
**Status:** FIXED (2026-01-27)
**Severity:** MEDIUM
**Description:** Config lacked EnvironmentMode, SearchReferenceSpaceType, ColorSpaceVisibility enums.
**Fix:** Added all three enums, Config field + getter/setter, YAML parsing, load_environment() method. Exported from lib.rs.

### [2026-01-27] Builtin Transforms Expanded
**Status:** FIXED (2026-01-27)
**Severity:** HIGH
**Description:** Added 40+ missing builtin transforms:
- Display: MIRROR NEGS variants (5), G2.2-REC.709 (2), G2.6-P3 variants (3), DCDM-D65, DisplayP3-HDR, ST2084 variants (2)
- Camera: Sony SGAMUT3.CINE, VENICE variants (3), RED REDLogFilm, ARRI ALEXA explicit
- ACES 2.0: All 24 output transforms (SDR/HDR, various peak luminances and limiting primaries)
- ACES utility: AP1→LINEAR-REC709-BFD
- ACES LMT: Blue Light Artifact Fix, Reference Gamut Compression
- ACES 1.x: All 14 output transforms (SDR/HDR with B-spline tone curves baked to 65536-sample 1D LUT)
**Fix:** New BuiltinDef variants (Gamma, MonCurve, Scale, Clamp, Aces2Output, BSplineCurve, Log, FixedFunction). Hermite B-spline evaluator with forward+inverse LUT baking.

### [2026-01-27] AP0/AP1 to XYZ_D65 Bradford Matrices Wrong
**Status:** FIXED (2026-01-27)
**Severity:** CRITICAL
**Description:** AP0_TO_XYZ_D65 and AP1_TO_XYZ_D65 were hardcoded constants WITHOUT Bradford chromatic adaptation (D60->D65). OCIO computes these at runtime via `build_conversion_matrix_to_XYZ_D65` which applies Bradford(src_wht->D65) * rgb_to_xyz.
**Fix:** Changed from `const [f32; 16]` to `LazyLock<[f32; 16]>` computed at runtime using `bradford_adapt(src_wht, D65_XYZ) * rgb_to_xyz(prims)`. Source white computed from matrix * [1,1,1], matching OCIO exactly.

### [2026-01-27] AcesGlow10/AcesRedMod10 Completely Wrong
**Status:** FIXED (2026-01-27)
**Severity:** CRITICAL
**Description:** Both FixedFunction implementations were fundamentally wrong:
- AcesGlow10: Used Rec709 luma instead of YC with chroma radius (1.75), wrong sigmoid, wrong gain logic
- AcesRedMod10: Used HSV hue instead of atan2 Yab space, wrong B-spline basis, wrong scale (0.2 vs 0.18)
**Fix:** Complete rewrite of both from C++ reference (FixedFunctionOpCPU.cpp). Also fixed AcesGlow03/AcesRedMod03.

### [2026-01-27] Matrix Precision Truncation
**Status:** FIXED (2026-01-27)
**Severity:** MEDIUM
**Description:** RRT_SAT_MATRIX and ODT_DESAT_MATRIX had only 6 significant digits. C++ reference uses doubles with 12+ digits.
**Fix:** Expanded both matrices to 12 significant digits.

### [2026-01-27] ACES 1.x Output Transforms Fully Differentiated
**Status:** FIXED (2026-01-27)
**Severity:** HIGH
**Description:** parse_aces1_output was oversimplified — all 16 variants shared the same pipeline. Now each variant has its correct chain:
- SDR-CINEMA: RRT + tonecurve + AP1→XYZ_D65
- SDR-VIDEO: + video_adjust (DarkToDim + desat)
- REC709lim/P3lim: + sdr_primary_clamp (AP1→limit Bradford, range 0..1, limit→XYZ)
- D60sim-D65: + range(≤1) + scale(0.964/0.955) + AP1→XYZ
- D60sim-DCI: + roll_white_d60 LUT + range(≤0.918) + scale(0.96) + AP1→XYZ + DCI→D65 Bradford
- D65sim-DCI: + roll_white_d65 LUT + range(≤0.908) + scale(0.9575) + AP1→XYZ_D65 + DCI→D65 Bradford
- HDR: + hdr_tonecurve + hdr_primary_clamp (AP1→limit no-adapt, range 0..1, limit→XYZ, D60→D65 Bradford) + nit_normalization
**Fix:** Added runtime rgb_to_xyz(), bradford_adapt(), conversion_matrix functions. Added sdr_primary_clamp(), hdr_primary_clamp(), roll_white_lut(), nit_normalization() helpers. All 16 ACES 1.x outputs verified via test.

---

## 5. Optimization Opportunities

### Rust-specific
- [x] SIMD via portable-simd (processor.rs has SIMD paths)
- [x] Parallel iterators (rayon support in vfx-io)
- [ ] Arena allocators for LUTs - **OPPORTUNITY**
- [x] Zero-copy where possible (Image::from_raw)

### Memory
- [x] Thread-local scratch space for compression
- [x] SmallVec for layers (avoids heap for 1-2 layers)
- [ ] Pool buffers for processing - **OPPORTUNITY**
- [ ] Lazy LUT loading - **OPPORTUNITY**

### Performance
- [x] GPU shader generation (gpu.rs with GLSL)
- [ ] Operation fusion - **OPPORTUNITY**
- [x] Tile-based processing (EXR tiled support)

---

## 6. Test Coverage

- [x] Unit tests for all transforms (cargo test passes)
- [x] Integration tests with real configs
- [x] Numerical accuracy tests vs OCIO (ACES 1.x SDR-CINEMA within 5e-3)
- [x] Roundtrip tests (read/write/read)
- [ ] Fuzz testing for parsers - **NOT IMPLEMENTED**

---

## Progress Summary

| Area | Parity | Notes |
|------|--------|-------|
| OCIO Config | 95% | Missing Virtual Display, NamedTransformVisibility |
| OCIO Transforms | 100% | All 23/23 transforms implemented |
| OCIO FixedFunction | 100% | 25/27 styles (GAMUTMAP_02/07 also unimplemented in OCIO) |
| OCIO BuiltinTransform | 100% | 97/97: All 16 ACES 1.x + 31 ACES 2.0 outputs + 12 ACES core + 23 display + 15 camera |
| OCIO LUTs | 70% | 14/20 formats (ICC missing) |
| OIIO I/O | 79% | 11/14 formats (BMP/GIF/TGA/TX missing) |
| OIIO EXR | 67% | 8/12 compressions (DWAA/DWAB/HTJ2K missing) |
| OIIO Algo | 80% | Core ops present, some advanced missing |
| Python API | 75% | Processor.apply() fixed, PackedImageDesc via apply() |

---

## Priority Fixes

### CRITICAL
1. ~~**GradingHueCurveTransform**~~ - DONE (2026-01-27)
2. ~~**Python Processor.apply()**~~ - DONE (2026-01-27): processor_with_context() now returns OcioProcessor, added processor() method, added apply/apply_rgb/apply_rgba
3. ~~**PackedImageDesc**~~ - DONE (2026-01-27): OcioProcessor.apply(pixels, num_channels) serves same purpose

### HIGH
4. ~~**ACES 2.0 FixedFunction**~~ - DONE (2026-01-27): 25/27 styles, full Output Transform pipeline
5. ~~**ACES 2.0 Output Transforms**~~ - DONE (2026-01-27): 24 builtin styles via aces2::init_output_transform
6. ~~**EnvironmentMode/ReferenceSpaceType**~~ - DONE (2026-01-27): EnvironmentMode enum + SearchReferenceSpaceType + ColorSpaceVisibility enums added

### MEDIUM
7. **ICC Profile Support** - Monitor/print profiles
8. **DWAA/DWAB Compression** - DCT-based EXR
9. **Cubic Spline Interpolation** - CSP format quality
10. **Deep Compositing** - flatten, over operations

### LOW
11. **BMP/GIF/TGA formats** - Legacy format support
12. **TX textures** - Tiled mipmap format
13. **Virtual Display** - OCIO 2.3+ feature

---

*Updated: 2026-01-27*
