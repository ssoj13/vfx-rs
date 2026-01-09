# VFX-RS Full Parity Plan (PLAN7)

**Date:** 2026-01-08
**Status:** Comprehensive audit and remaining work analysis

---

## Executive Summary

| Category | Done | Partial | Missing | Progress |
|----------|------|---------|---------|----------|
| Transfer Functions | 19 | 0 | 1 | 95% |
| LUT Formats | 15 | 0 | 0 | 100% |
| Ops | 12 | 0 | 1 | 92% |
| OCIO Config/Transforms | 31 | 1 | 1 | 94% |
| ImageBufAlgo | 18 | 0 | 0 | 100% |
| GPU Compute | 12 | 2 | 1 | 80% |
| Primaries/Adaptation | 18 | 0 | 4 | 82% |
| **TOTAL** | **125** | **3** | **8** | **~92%** |

---

## 1. Transfer Functions (vfx-transfer)

### 1.1 DONE (17 functions)

| Function | File | Tests | Notes |
|----------|------|-------|-------|
| sRGB | `srgb.rs` | Yes | IEC 61966-2-1 |
| Rec.709 | `rec709.rs` | Yes | ITU-R BT.709-6 OETF |
| Gamma 2.2/2.4/2.6 | `gamma.rs` | Yes | Pure power law |
| PQ (ST.2084) | `pq.rs` | Yes | HDR, verified constants |
| HLG | `hlg.rs` | Yes | ITU-R BT.2100-2 |
| LogC3 | `log_c.rs` | Yes | ARRI, all EI variants |
| LogC4 | `log_c4.rs` | Yes | ARRI, verified vs OCIO |
| S-Log2 | `s_log2.rs` | Yes | Sony |
| S-Log3 | `s_log3.rs` | Yes | Sony |
| V-Log | `v_log.rs` | Yes | Panasonic |
| ACEScc | `acescc.rs` | Yes | AMPAS S-2014-003 |
| ACEScct | `acescct.rs` | Yes | AMPAS S-2016-001 |
| REDLogFilm | `red_log.rs` | Yes | RED |
| REDLog3G10 | `red_log.rs` | Yes | RED |
| BMDFilm Gen5 | `bmd_film.rs` | Yes | Blackmagic |
| Canon Log | `canon_log.rs` | Yes | Thorpe 2012, original |
| Canon Log 2 | `canon_log.rs` | Yes | Verified vs OCIO |
| Canon Log 3 | `canon_log.rs` | Yes | Verified vs OCIO |
| Apple Log | `apple_log.rs` | Yes | Verified vs OCIO |
| Moncurve Gamma | `gamma.rs` | Yes | fwd/rev/mirror_fwd/mirror_rev |

### 1.2 MISSING (1 function)

| Function | Reference | Priority |
|----------|-----------|----------|
| DJI D-Log | Research needed | LOW |

**DJI D-Log:** Research required - no OCIO reference exists.

---

## 2. LUT Formats (vfx-lut) - 100% DONE

All 15 formats implemented and tested:

| Format | File | Read | Write | Tests |
|--------|------|------|-------|-------|
| .cube (Iridas/Resolve) | `cube.rs` | Yes | Yes | Yes |
| .spi1d | `spi.rs` | Yes | Yes | Yes |
| .spi3d | `spi.rs` | Yes | Yes | Yes |
| .3dl | `threedl.rs` | Yes | Yes | Yes |
| .clf (ACES CLF) | `clf.rs` | Yes | Yes | Yes |
| .ctf (ACES CTF) | `clf.rs` | Yes | Yes | Yes |
| .csp (Cinespace) | `csp.rs` | Yes | Yes | Yes |
| .cdl (ASC-CDL) | `cdl.rs` | Yes | Yes | Yes |
| .1dl (Autodesk Discreet) | `discreet1dl.rs` | Yes | Yes | 8 tests |
| .hdl (Houdini) | `hdl.rs` | Yes | Yes | 6 tests |
| .itx (Iridas ITX) | `iridas_itx.rs` | Yes | Yes | 5 tests |
| .look (Iridas Look) | `iridas_look.rs` | Yes | Yes | 4 tests |
| .mga/.m3d (Pandora) | `pandora.rs` | Yes | Yes | 3 tests |
| .spimtx (SPI Matrix) | `spi_mtx.rs` | Yes | Yes | 10 tests |
| .cub (Truelight) | `truelight.rs` | Yes | Yes | 5 tests |
| .vf (Nuke VF) | `nuke_vf.rs` | Yes | Yes | 4 tests |

---

## 3. Ops (vfx-ops)

### 3.1 DONE (12 ops)

| Op | Module | Tests | Notes |
|----|--------|-------|-------|
| CDL | `cdl.rs` | Yes | ASC-CDL with SOP, saturation |
| ExposureContrast | `exposure_contrast.rs` | Yes | Linear, Video, Logarithmic styles |
| GradingPrimary | `grading_primary.rs` | Yes | Lift/gamma/gain, brightness/contrast |
| GradingTone | `grading_tone/` | Yes | 5-zone tonal correction |
| GradingRGBCurve | `grading_rgb_curve/` | Yes | B-spline interpolation |
| Range | `range.rs` | Yes | Clamping and remapping |
| Allocation | `allocation.rs` | Yes | Uniform, Lg2 |
| LogOp | `log_op.rs` | Yes | Generic log transform |
| Exponent | `exponent.rs` | Yes | Per-channel, NegativeStyle |
| FixedFunction | `fixed_function.rs` | Yes | 19+ styles (ACES, color space conversions) |
| FFT | `fft.rs` | Yes | Forward/inverse FFT |
| Composite | `composite.rs` | Yes | Porter-Duff, blend modes |

### 3.2 MISSING (1 op)

| Op | Reference | Priority | Work Required |
|----|-----------|----------|---------------|
| GradingHueCurve | OCIO GradingRGBCurve variant | LOW | Hue-based corrections |

**GradingHueCurve:** Part of color grading toolkit for:
- Hue vs Hue
- Hue vs Saturation  
- Hue vs Luminance
- Saturation vs Saturation
- Saturation vs Luminance
- Luminance vs Saturation

**Implementation location:** `vfx-ops/src/grading_hue_curve.rs` (new file)

---

## 4. OCIO Config & Transforms (vfx-ocio)

### 4.1 DONE (28 features)

**Config Parsing:**
- [x] YAML config loading
- [x] ColorSpace definitions
- [x] Roles
- [x] Displays/Views
- [x] Looks
- [x] Context variables ($SHOT, $SEQ, etc.)
- [x] FileRules
- [x] NamedTransforms (v2)
- [x] SharedViews (v2.3)
- [x] InactiveColorSpaces

**Transforms:**
- [x] MatrixTransform
- [x] CDLTransform
- [x] ExponentTransform
- [x] ExponentWithLinearTransform
- [x] LogTransform
- [x] LogAffineTransform
- [x] LogCameraTransform
- [x] RangeTransform
- [x] FileTransform (.cube, .spi1d, .spi3d, .clf, .ctf)
- [x] BuiltinTransform
- [x] GroupTransform
- [x] AllocationTransform
- [x] GradingPrimaryTransform
- [x] GradingRGBCurveTransform
- [x] GradingToneTransform
- [x] FixedFunctionTransform
- [x] ExposureContrastTransform
- [x] ColorSpaceTransform

**Additional Completed (not in list above):**
- [x] LookTransform - handled at Config level via `look_pipeline()`
- [x] DisplayViewTransform - handled at Config level via `display_view_pipeline()`

### 4.2 PARTIAL (1 feature)

| Feature | Status | Work Required |
|---------|--------|---------------|
| Processor optimization | Basic works | Matrix chaining, LUT fusion |

### 4.3 MISSING (1 feature)

| Feature | Reference | Priority |
|---------|-----------|----------|
| viewing_rules | OCIO v2.0+ | LOW |

---

## 5. ImageBufAlgo (vfx-io) - 100% DONE

All 18 modules implemented:

| Module | Functions | Status |
|--------|-----------|--------|
| `patterns` | zero, fill, checker, noise | DONE |
| `channels` | channels, channel_append, channel_sum, extract_channel, flatten, get_alpha | DONE |
| `geometry` | crop, cut, flip, flop, transpose, rotate90/180/270, resize, resample, fit, rotate, circular_shift, paste, reorient | DONE |
| `arithmetic` | add, sub, mul, div, abs, absdiff, pow, clamp, invert, over, max, min, mad, normalize | DONE |
| `color` | premult, unpremult, repremult, saturate, contrast_remap, color_map, colormatrixtransform, rangecompress, rangeexpand, srgb_to_linear, linear_to_srgb | DONE |
| `composite` | under, in_op, out, atop, xor, screen, multiply, overlay, hardlight, softlight, difference, exclusion, colordodge, colorburn, add_blend, zover | DONE |
| `stats` | compute_pixel_stats, compare, compare_relative, is_constant_color, is_constant_channel, is_monochrome, histogram, maxchan, minchan, color_range_check, color_count, unique_color_count | DONE |
| `ocio` | colorconvert, ociodisplay, ociolook, ociofiletransform, ocionamedtransform, equivalent_colorspace | DONE |
| `deep` | flatten_deep, deepen, deep_merge, deep_holdout, deep_tidy, deep_stats | DONE |
| `filters` | median, blur, unsharp_mask, dilate, erode, morph_open, morph_close, laplacian, sharpen, sobel, convolve, box_blur, bilateral | DONE |
| `fft` | fft, ifft, complex_to_polar, polar_to_complex | DONE |
| `drawing` | render_point, render_line, render_box, render_circle, render_ellipse, render_polygon | DONE |
| `warp` | warp, st_warp, matrix_* | DONE |
| `demosaic` | demosaic (multiple algorithms, Bayer patterns) | DONE |
| `texture` | make_texture, make_mip_level, mip_level_count, mip_dimensions | DONE |
| `fillholes` | fillholes_pushpull, has_holes, count_holes | DONE |
| `text` | render_text (feature-gated) | DONE |

---

## 6. GPU Compute (vfx-compute)

### 6.1 DONE (12 features)

**WGSL Shaders (already implemented in `shaders/mod.rs`):**
- [x] COLOR_MATRIX - 4x4 matrix transform
- [x] CDL - Color Decision List
- [x] LUT1D - 1D LUT interpolation
- [x] LUT3D - 3D LUT trilinear interpolation
- [x] RESIZE - Bilinear resize
- [x] BLUR_H / BLUR_V - Gaussian blur (separable)
- [x] COMPOSITE_OVER - Porter-Duff Over
- [x] BLEND - Photoshop blend modes (9 modes)
- [x] CROP - Region extraction
- [x] FLIP_H / FLIP_V - Flip operations
- [x] ROTATE_90 - Rotation (90/180/270)

**Infrastructure:**
- [x] GpuPrimitives trait with associated types
- [x] CpuPrimitives backend (rayon parallel)
- [x] TiledExecutor for automatic tiling
- [x] AnyExecutor for dynamic dispatch
- [x] VRAM detection (cross-platform)
- [x] Backend selection logic

### 6.2 PARTIAL (2 features)

| Feature | Status | Work Required |
|---------|--------|---------------|
| WgpuPrimitives | Shaders exist, integration incomplete | Wire shaders to GpuPrimitives impl |
| CudaPrimitives | PTX kernels started | Complete cudarc integration |

### 6.3 MISSING (1 feature)

| Feature | Priority | Notes |
|---------|----------|-------|
| LUT3D Tetrahedral GPU | MEDIUM | Current is trilinear, tetrahedral more accurate |

---

## 7. Primaries & Adaptation (vfx-primaries, vfx-math)

### 7.1 DONE (18 items)

**Color Primaries:**
- [x] ACES AP0, AP1
- [x] sRGB / Rec.709
- [x] Rec.2020
- [x] DCI-P3, Display P3
- [x] Adobe RGB
- [x] ProPhoto RGB
- [x] Canon CGamut
- [x] ARRI Wide Gamut 3, AWG4
- [x] RED Wide Gamut RGB
- [x] S-Gamut3, S-Gamut3.Cine
- [x] V-Gamut

**Chromatic Adaptation (vfx-math/adapt.rs):**
- [x] Bradford matrix
- [x] CAT02 matrix
- [x] Von Kries
- [x] XYZ Scaling

**White Points:**
- [x] D50, D55, D60, D65
- [x] DCI White
- [x] ACES White
- [x] Illuminant A, C, E

### 7.2 MISSING (4 items)

| Item | Category | Priority |
|------|----------|----------|
| D75 | White point | LOW |
| Illuminant F series (F2, F7, F11) | White points | LOW |
| Blackmagic Wide Gamut | Primaries | MEDIUM |
| DJI D-Gamut | Primaries | LOW |

---

## 8. Known Bugs

### 8.1 HIGH Priority

| ID | Location | Description | Status |
|----|----------|-------------|--------|
| BUG-003 | `vfx-ocio/processor.rs` | Matrix inversion for inverse direction | FIXED (uses glam::Mat4::inverse) |
| BUG-004 | `vfx-ocio/config.rs` | View transform dual-reference logic | FIXED (proper from_scene/to_display) |

### 8.2 MEDIUM Priority

| ID | Location | Description | Status |
|----|----------|-------------|--------|
| BUG-005 | `vfx-ocio/config.rs` | File rule `Default` validation only checks position | OPEN |
| BUG-006 | `vfx-ocio/context.rs` | Unresolved variables left in path silently | OPEN |

### 8.3 LOW Priority

| ID | Location | Description | Status |
|----|----------|-------------|--------|
| BUG-008 | `vfx-lut/clf.rs` | Log/Exponent nodes not fully serialized in writer | OPEN |
| BUG-009 | `vfx-ocio` | OptimizationLevel defined but unused | OPEN |

---

## 9. Implementation Order

### Phase A - Bug Fixes (HIGH) - COMPLETED
1. [x] BUG-003: Matrix inversion for inverse direction (already fixed)
2. [x] BUG-004: View transform dual-reference logic (already fixed)

### Phase B - Transfer Functions (MEDIUM) - MOSTLY DONE
3. [x] Moncurve gamma (full implementation) - DONE 2026-01-09
4. [x] Canon Log (original) - DONE 2026-01-09
5. [ ] DJI D-Log (if specification found)

### Phase C - OCIO Completion (MEDIUM) - MOSTLY DONE
6. [x] LookTransform - handled at Config level
7. [x] DisplayViewTransform - handled at Config level
8. [ ] viewing_rules parsing

### Phase D - GPU Backend (MEDIUM)
9. [ ] WgpuPrimitives full integration
10. [ ] CudaPrimitives full integration
11. [ ] LUT3D tetrahedral shader

### Phase E - Ops Completion (LOW)
12. [ ] GradingHueCurve op

### Phase F - Primaries (LOW)
13. [ ] Blackmagic Wide Gamut
14. [ ] D75, F-series illuminants
15. [ ] DJI D-Gamut

---

## 10. Architecture Notes

### Error Handling
7 error types exist (fragmented):
- `vfx-core::Error`
- `vfx-io::IoError`
- `vfx-color::ColorError`
- `vfx-ops::OpsError`
- `vfx-icc::IccError`
- `vfx-ocio::OcioError`
- `vfx-lut::LutError`

**Recommendation:** Consider unified `VfxError` with `From` impls (LOW priority).

### Image Container
3 containers exist (by design):
- `vfx-core::Image<C, T>` - compile-time typed
- `vfx-io::ImageData` / `ImageBuf` - format-agnostic
- `vfx-compute::ComputeImage` - GPU-optimized

This is intentional separation of concerns, not duplication.

---

## 11. Test Coverage

| Crate | Tests | Status |
|-------|-------|--------|
| vfx-transfer | 51+ | All passing |
| vfx-math | 5+ | All passing |
| vfx-primaries | 7+ | All passing |
| vfx-lut | 60+ | All passing |
| vfx-ocio | 20+ | All passing |
| vfx-ops | 50+ | All passing |
| vfx-io | 100+ | All passing |
| vfx-compute | 10+ | All passing |
| **TOTAL** | **1200+** | **All passing** |

---

## 12. Summary

**Production-Ready Components:**
- Transfer functions (all camera logs, HDR curves)
- LUT formats (complete OCIO/OIIO parity)
- Color primaries and adaptation
- ImageBufAlgo operations
- Core ops (CDL, grading, range, etc.)

**Needs Completion:**
- GPU backend integration (shaders exist, wiring needed)
- Minor items: DJI D-Log, viewing_rules, GradingHueCurve

**Overall Assessment:** ~92% complete, ready for production use in most workflows.

---

*Generated: 2026-01-08*
*Updated: 2026-01-09 - Completed moncurve gamma, Canon Log original, verified LookTransform/DisplayViewTransform*
