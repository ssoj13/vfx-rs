# VFX-RS OCIO Parity Checklist

Детальный список недостающих фич для паритета с OpenColorIO.
Каждая фича должна быть реализована и отмечена галочкой.

---

## 1. Ops (операции над цветом)

### 1.1 GradingTone (DONE)
**Reference:** `_ref/OpenColorIO/src/OpenColorIO/ops/gradingtone/`

Тональная коррекция с 5 зонами:
- [x] `GradingTone` struct с полями:
  - [x] `blacks` (start, width)
  - [x] `shadows` (start, pivot)
  - [x] `midtones`
  - [x] `highlights` (start, pivot)
  - [x] `whites` (start, width)
  - [x] `s_contrast` (S-curve contrast)
- [x] `GradingStyle`: LOG, LIN, VIDEO
- [x] Forward/Inverse transform
- [x] 6-point spline for midtones
- [x] Faux-cubic curves for highlights/shadows
- [x] Lin-Log conversion for LINEAR style
- [x] Файл: `crates/vfx-ops/src/grading_tone/`

### 1.2 GradingRGBCurve (DONE)
**Reference:** `_ref/OpenColorIO/src/OpenColorIO/ops/gradingrgbcurve/`

RGB кривые с B-spline интерполяцией:
- [x] `GradingRGBCurves` struct:
  - [x] Master curve
  - [x] Red curve
  - [x] Green curve
  - [x] Blue curve
- [x] `BSplineCurve` - контрольные точки и slopes
- [x] B-spline интерполяция (quadratic polynomials)
- [x] Slope estimation for smooth curves
- [x] `GradingStyle`: LOG, LINEAR, VIDEO
- [x] Lin-Log conversion for LINEAR style
- [x] Forward/Inverse evaluation
- [x] Файл: `crates/vfx-ops/src/grading_rgb_curve/`

### 1.3 GradingHueCurve (MISSING)
**Reference:** OCIO не имеет отдельного файла, это часть GradingRGBCurve

Hue-based коррекции:
- [ ] Hue vs Hue
- [ ] Hue vs Saturation
- [ ] Hue vs Luminance
- [ ] Saturation vs Saturation
- [ ] Saturation vs Luminance
- [ ] Luminance vs Saturation
- [ ] Файл: `crates/vfx-ops/src/grading_hue_curve.rs`

### 1.4 RangeOp (DONE)
**Reference:** `_ref/OpenColorIO/src/OpenColorIO/ops/range/`

Clamping и remapping:
- [x] `Range` struct:
  - [x] `min_in`, `max_in`
  - [x] `min_out`, `max_out`
- [x] Clamp mode
- [x] Scale + clamp mode
- [x] NaN handling (becomes lower bound)
- [x] Файл: `crates/vfx-ops/src/range.rs`

### 1.5 AllocationOp (DONE)
**Reference:** `_ref/OpenColorIO/src/OpenColorIO/ops/allocation/`

LUT shaping для оптимального распределения:
- [x] `Allocation` enum: Uniform, Lg2
- [x] `AllocationOp` struct: min, max, offset
- [x] Forward/Inverse transforms
- [x] Uniform: линейный fit [min, max] -> [0, 1]
- [x] Lg2: log2 + fit с offset
- [x] 13 tests (roundtrip, known values, edge cases)
- [x] Файл: `crates/vfx-ops/src/allocation.rs`

### 1.6 LogOp (generic) (DONE)
**Reference:** `_ref/OpenColorIO/src/OpenColorIO/ops/log/`

Generic log transform (не camera-specific):
- [x] `LogStyle` enum:
  - [x] Log10 / AntiLog10
  - [x] Log2 / AntiLog2
  - [x] LinToLog / LogToLin
  - [x] CameraLinToLog / CameraLogToLin
- [x] `LogParams` struct:
  - [x] `log_side_slope`, `log_side_offset`
  - [x] `lin_side_slope`, `lin_side_offset`
  - [x] `lin_side_break` (for camera log)
  - [x] `linear_slope` (for camera log)
- [x] `LogOp` struct with per-channel params
- [x] Forward/Inverse transforms
- [x] 13 tests (roundtrip, known values, edge cases)
- [x] Файл: `crates/vfx-ops/src/log_op.rs`

### 1.7 ExponentOp (DONE)
**Reference:** `_ref/OpenColorIO/src/OpenColorIO/ops/exponent/`

Per-channel exponent:
- [x] `ExponentOp` struct с 4 компонентами (RGBA)
- [x] `NegativeStyle` enum: Clamp, Mirror, PassThru
- [x] Forward/Inverse transforms
- [x] Combine (multiply exponents)
- [x] 17 tests (roundtrip, negative handling, edge cases)
- [x] Файл: `crates/vfx-ops/src/exponent.rs`

### 1.8 FixedFunctionOp (PARTIAL)
**Reference:** `_ref/OpenColorIO/src/OpenColorIO/ops/fixedfunction/`

Специальные функции ACES и др:
- [ ] `ACES_RED_MOD_03` (forward/inverse)
- [x] `ACES_RED_MOD_10` (forward/inverse) - Note: inverse not exact per ACES 1.0 spec
- [ ] `ACES_GLOW_03` (forward/inverse)
- [x] `ACES_GLOW_10` (forward/inverse)
- [ ] `ACES_DARK_TO_DIM_10` (forward/inverse)
- [ ] `ACES_GAMUT_COMP_13` (forward/inverse)
- [x] `REC2100_SURROUND` (forward/inverse)
- [x] `RGB_TO_HSV` / `HSV_TO_RGB` (есть в vfx-color)
- [x] `XYZ_TO_xyY` / `xyY_TO_XYZ`
- [x] `XYZ_TO_uvY` / `uvY_TO_XYZ`
- [ ] `XYZ_TO_LUV` / `LUV_TO_XYZ`
- [ ] `LIN_TO_PQ` / `PQ_TO_LIN` (fixed params version)
- [ ] `LIN_TO_GAMMA_LOG` / `GAMMA_LOG_TO_LIN`
- [ ] `LIN_TO_DOUBLE_LOG` / `DOUBLE_LOG_TO_LIN`
- [x] `ACES_OUTPUT_TRANSFORM_20` (есть в vfx-color/aces2)
- [ ] `ACES_RGB_TO_JMh_20` / `ACES_JMh_TO_RGB_20`
- [ ] `ACES_TONESCALE_COMPRESS_20`
- [ ] `ACES_GAMUT_COMPRESS_20`
- [ ] `RGB_TO_HSY_LIN/LOG/VID` / `HSY_TO_RGB`
- [x] Файл: `crates/vfx-ops/src/fixed_function.rs`

---

## 2. LUT Formats

### 2.1 Autodesk Discreet 1DL (DONE)
**Reference:** `_ref/OpenColorIO/src/OpenColorIO/fileformats/FileFormatDiscreet1DL.cpp`

- [x] Parse 1DL header (LUT: numtables length [dstDepth])
- [x] Read LUT data (old format + new format)
- [x] Write support (mono/RGB, integer/float)
- [x] BitDepth enum (8/10/12/16/16f/32f)
- [x] Interleaved and separate block formats
- [x] 8 tests
- [x] Файл: `crates/vfx-lut/src/discreet1dl.rs`

### 2.2 Houdini HDL (DONE)
**Reference:** `_ref/OpenColorIO/src/OpenColorIO/fileformats/FileFormatHDL.cpp`

- [x] Parse HDL format (version 1/2/3)
- [x] 1D LUT (type C/RGB)
- [x] 3D LUT (type 3D)
- [x] 3D+1D LUT with prelut
- [x] Read/write support
- [x] 6 tests
- [x] Файл: `crates/vfx-lut/src/hdl.rs`

### 2.3 Iridas ITX (MISSING)
**Reference:** `_ref/OpenColorIO/src/OpenColorIO/fileformats/FileFormatIridasItx.cpp`

- [ ] Parse ITX format
- [ ] Файл: `crates/vfx-lut/src/iridas_itx.rs`

### 2.4 Iridas Look (MISSING)
**Reference:** `_ref/OpenColorIO/src/OpenColorIO/fileformats/FileFormatIridasLook.cpp`

- [ ] Parse .look format
- [ ] Файл: `crates/vfx-lut/src/iridas_look.rs`

### 2.5 Pandora (MISSING)
**Reference:** `_ref/OpenColorIO/src/OpenColorIO/fileformats/FileFormatPandora.cpp`

- [ ] Parse .mga format
- [ ] Файл: `crates/vfx-lut/src/pandora.rs`

### 2.6 SPI Matrix (DONE)
**Reference:** `_ref/OpenColorIO/src/OpenColorIO/fileformats/FileFormatSpiMtx.cpp`

- [x] Parse .spimtx format (12 floats)
- [x] 3x3 matrix + RGB offset (offset/65535)
- [x] SpiMatrix struct with apply, inverse, compose
- [x] Read/write support
- [x] 10 tests
- [x] Файл: `crates/vfx-lut/src/spi_mtx.rs`

### 2.7 Truelight (DONE)
**Reference:** `_ref/OpenColorIO/src/OpenColorIO/fileformats/FileFormatTruelight.cpp`

- [x] Parse .cub format (v2.0)
- [x] 3D cube LUT
- [x] Optional 1D shaper (InputLUT)
- [x] Shaper descaling (0..size-1 to 0..1)
- [x] Read/write support
- [x] 5 tests
- [x] Файл: `crates/vfx-lut/src/truelight.rs`

### 2.8 VF Format (MISSING)
**Reference:** `_ref/OpenColorIO/src/OpenColorIO/fileformats/FileFormatVF.cpp`

- [ ] Parse .vf format
- [ ] Файл: `crates/vfx-lut/src/vf.rs`

---

## 3. Transfer Functions

### 3.1 Canon Log (original) (MISSING)
**Reference:** `_ref/OpenColorIO/src/OpenColorIO/transforms/builtins/CanonCameras.cpp`

- [ ] `clog_encode` / `clog_decode` (original Canon Log, not Log2/3)
- [ ] Добавить в `crates/vfx-transfer/src/canon_log.rs`

### 3.2 DJI D-Log (MISSING)
- [ ] Research D-Log specification
- [ ] Implement encode/decode
- [ ] Файл: `crates/vfx-transfer/src/dji_dlog.rs`

### 3.3 Moncurve Gamma (PARTIAL)
**Reference:** `_ref/OpenColorIO/src/OpenColorIO/ops/gamma/GammaOpData.cpp`

- [ ] `moncurve_fwd` / `moncurve_rev` с параметрами gamma и offset
- [ ] Mirror mode для negative values
- [ ] Добавить в `crates/vfx-transfer/src/gamma.rs`

---

## 4. OCIO Transforms (config parsing)

### 4.1 GradingPrimaryTransform (PARTIAL)
- [x] Parsing from YAML
- [ ] Evaluation через GradingPrimary op
- [ ] Dynamic properties

### 4.2 GradingToneTransform (MISSING)
- [ ] YAML parsing
- [ ] Link to GradingTone op
- [ ] Dynamic properties

### 4.3 GradingRGBCurveTransform (MISSING)
- [ ] YAML parsing
- [ ] B-spline curve data
- [ ] Link to GradingRGBCurve op

### 4.4 AllocationTransform (MISSING)
- [ ] YAML parsing
- [ ] Link to AllocationOp

### 4.5 LogAffineTransform (MISSING)
- [ ] YAML parsing
- [ ] Parameters: base, logSlope, logOffset, linSlope, linOffset

### 4.6 LogCameraTransform (MISSING)
- [ ] YAML parsing
- [ ] Parameters: linBreak, linearSlope, base

### 4.7 FixedFunctionTransform (MISSING)
- [ ] YAML parsing
- [ ] Style enum mapping
- [ ] Parameters array

### 4.8 GroupTransform (MISSING)
- [ ] Container for multiple transforms
- [ ] Sequential evaluation

### 4.9 LookTransform (MISSING)
- [ ] Look application
- [ ] Process space handling

---

## 5. GPU Compute

### 5.1 wgpu Shaders (MISSING)
- [ ] Color matrix shader
- [ ] LUT1D shader
- [ ] LUT3D shader (tetrahedral)
- [ ] CDL shader
- [ ] Transfer function shaders
- [ ] Файлы: `crates/vfx-compute/src/shaders/`

### 5.2 CUDA Backend (MISSING)
- [ ] cudarc integration
- [ ] CUDA kernels
- [ ] Файл: `crates/vfx-compute/src/cuda/`

---

## 6. Primaries (additional)

### 6.1 Additional White Points
- [ ] D75
- [ ] Illuminant F series (F2, F7, F11)
- [ ] Добавить в `crates/vfx-primaries/src/lib.rs`

### 6.2 Additional Gamuts
- [ ] Blackmagic Wide Gamut
- [ ] DJI D-Gamut
- [ ] Добавить в `crates/vfx-primaries/src/lib.rs`

---

## 7. Testing & Validation

### 7.1 Cross-validation with OCIO
- [ ] Generate reference values from ocioconvert
- [ ] Compare output with vfx-rs
- [ ] Document precision differences

### 7.2 Edge Cases
- [ ] NaN handling in all ops
- [ ] Inf handling in all ops
- [ ] Negative value handling
- [ ] Out-of-range clamping

---

## Summary

| Category | Total | Done | Missing |
|----------|-------|------|---------|
| Ops | 8 | 7 | 1 |
| LUT Formats | 8 | 4 | 4 |
| Transfer Functions | 3 | 0 | 3 |
| OCIO Transforms | 9 | 1 | 8 |
| GPU Compute | 2 | 0 | 2 |
| Primaries | 2 | 0 | 2 |
| FixedFunction styles | 20 | 8 | 12 |
| **TOTAL** | **52** | **20** | **32** |

---

## Implementation Order (Priority)

### Phase A - Core Ops (HIGH)
1. [x] RangeOp - базовый clamping
2. [x] GradingTone - тональная коррекция
3. [x] GradingRGBCurve - RGB кривые
4. [x] LogOp (generic)
5. [x] ExponentOp
6. [x] AllocationOp

### Phase B - FixedFunction (HIGH)
6. [x] XYZ_TO_xyY / xyY_TO_XYZ
7. [x] XYZ_TO_uvY / uvY_TO_XYZ
8. [x] ACES_RED_MOD_10
9. [x] ACES_GLOW_10
10. [x] REC2100_SURROUND

### Phase C - LUT Formats (MEDIUM)
11. [x] Discreet 1DL
12. [x] SPI Matrix
13. [x] HDL (Houdini)
14. [x] Truelight

### Phase D - OCIO Transforms (MEDIUM)
15. [ ] GradingToneTransform parsing
16. [ ] GradingRGBCurveTransform parsing
17. [ ] FixedFunctionTransform parsing
18. [ ] AllocationTransform parsing

### Phase E - GPU (LOW for now)
19. [ ] wgpu color matrix shader
20. [ ] wgpu LUT3D shader

---

## Current Task

**Next:** Phase C - LUT Formats (Discreet 1DL, SPI Matrix, HDL)
