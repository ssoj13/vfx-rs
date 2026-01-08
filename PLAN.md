# VFX-RS Implementation Plan

## Python 3.14 Note
Maturin пока не поддерживает Python 3.14. Варианты:
1. Дождаться обновления maturin (следить за https://github.com/PyO3/maturin/issues)
2. Использовать setuptools-rust напрямую
3. Собирать wheel вручную через `cargo build` + правильную структуру

---

## Phase 1: Transfer Functions (vfx-transfer)

### 1.1 Missing Camera Logs

- [ ] **ARRI LogC4** (ArriCameras.cpp:200+)
  - [ ] Изучить спецификацию LogC4
  - [ ] Реализовать lin_to_logc4()
  - [ ] Реализовать logc4_to_lin()
  - [ ] Добавить тесты с референсными значениями
  - [ ] Добавить SIMD версию

- [ ] **Canon Log** (CanonCameras.cpp)
  - [ ] Canon Log (original)
    - [ ] lin_to_clog()
    - [ ] clog_to_lin()
  - [ ] Canon Log2
    - [ ] lin_to_clog2()
    - [ ] clog2_to_lin()
  - [ ] Canon Log3
    - [ ] lin_to_clog3()
    - [ ] clog3_to_lin()
  - [ ] Тесты для всех вариантов

- [ ] **Apple Log** (AppleCameras.cpp)
  - [ ] lin_to_applelog()
  - [ ] applelog_to_lin()
  - [ ] Тесты

### 1.2 Validate Existing Transfer Functions

- [ ] **gamma.rs** vs GammaOpData.cpp
  - [ ] Проверить basic gamma
  - [ ] Проверить moncurve gamma
  - [ ] Проверить mirror mode
  - [ ] Проверить pass-through mode

- [ ] **pq.rs** vs OCIO ST2084
  - [ ] Проверить константы (m1, m2, c1, c2, c3)
  - [ ] Проверить нормализацию (10000 nits)
  - [ ] Сравнить с референсными значениями

- [ ] **hlg.rs** vs OCIO HLG
  - [ ] Проверить OOTF
  - [ ] Проверить OETF/EOTF
  - [ ] Проверить параметры (a, b, c)

- [ ] **log_c.rs** (LogC3) vs ArriCameras.cpp
  - [ ] Проверить все EI варианты (200-3200)
  - [ ] Проверить cut/slope параметры
  - [ ] Сравнить с ARRI калькулятором

- [ ] **s_log2.rs** vs SonyCameras.cpp
  - [ ] Проверить константы
  - [ ] Сравнить с Sony спецификацией

- [ ] **s_log3.rs** vs SonyCameras.cpp
  - [ ] Проверить константы
  - [ ] Сравнить с Sony спецификацией

- [ ] **v_log.rs** vs PanasonicCameras.cpp
  - [ ] Проверить cut point
  - [ ] Проверить коэффициенты

- [ ] **red_log.rs** vs RedCameras.cpp
  - [ ] REDLogFilm
  - [ ] Log3G10
  - [ ] Log3G12

- [ ] **acescc.rs** vs ACES.cpp
  - [ ] Проверить lin_cut
  - [ ] Проверить формулу

- [ ] **acescct.rs** vs ACES.cpp
  - [ ] Проверить lin_cut (0.0078125)
  - [ ] Проверить формулу перехода

- [ ] **srgb.rs**
  - [ ] Проверить threshold (0.0031308)
  - [ ] Проверить коэффициенты

- [ ] **rec709.rs**
  - [ ] Проверить что это OETF, не EOTF
  - [ ] Проверить константы

---

## Phase 2: LUT Formats (vfx-lut)

### 2.1 Missing Formats

- [ ] **ASC CDL formats** (HIGH PRIORITY)
  - [ ] FileFormatCC.cpp → cc.rs
    - [ ] XML parsing
    - [ ] SOPNode (slope, offset, power)
    - [ ] SatNode
    - [ ] Serialization
  - [ ] FileFormatCCC.cpp → ccc.rs
    - [ ] Multiple CDL collection
    - [ ] ID-based lookup
  - [ ] FileFormatCDL.cpp → cdl_file.rs
    - [ ] EDL-style CDL

- [ ] **Cinespace CSP** (MED)
  - [ ] FileFormatCSP.cpp → csp.rs
  - [ ] 1D + 3D LUT combo
  - [ ] Pre-LUT shaper

- [ ] **Autodesk 1DL** (MED)
  - [ ] FileFormatDiscreet1DL.cpp → discreet1dl.rs
  - [ ] Flame/Smoke format

- [ ] **Resolve Cube** (MED)
  - [ ] FileFormatResolveCube.cpp → resolve_cube.rs
  - [ ] Отличия от Iridas Cube

- [ ] **SPI Matrix** (MED)
  - [ ] FileFormatSpiMtx.cpp → spi_mtx.rs
  - [ ] 3x3 или 3x4 матрица

- [ ] **Houdini HDL** (LOW)
  - [ ] FileFormatHDL.cpp → hdl.rs

- [ ] **Iridas ITX/Look** (LOW)
  - [ ] FileFormatIridasItx.cpp → iridas_itx.rs
  - [ ] FileFormatIridasLook.cpp → iridas_look.rs

- [ ] **Pandora** (LOW)
  - [ ] FileFormatPandora.cpp → pandora.rs

- [ ] **Truelight** (LOW)
  - [ ] FileFormatTruelight.cpp → truelight.rs

### 2.2 Validate Existing LUT Formats

- [ ] **cube.rs** vs FileFormatIridasCube.cpp
  - [ ] TITLE parsing
  - [ ] DOMAIN_MIN/MAX
  - [ ] LUT1D + LUT3D combo
  - [ ] Comment handling

- [ ] **spi.rs** vs FileFormatSpi1D/3D.cpp
  - [ ] Version handling
  - [ ] From/To ranges
  - [ ] Components

- [ ] **threedl.rs** vs FileFormat3DL.cpp
  - [ ] Mesh size detection
  - [ ] Input bit depth
  - [ ] Output bit depth

- [ ] **clf.rs** vs FileFormatCTF.cpp
  - [ ] ProcessList
  - [ ] All ProcessNode types
  - [ ] IndexMap
  - [ ] Metadata

### 2.3 LUT Interpolation

- [ ] **lut1d.rs** interpolation vs Lut1DOpCPU.cpp
  - [ ] Linear interpolation
  - [ ] Nearest neighbor
  - [ ] Tetrahedral (если есть)
  - [ ] Half-domain LUT

- [ ] **lut3d.rs** interpolation vs Lut3DOpCPU.cpp
  - [ ] Trilinear
  - [ ] Tetrahedral
  - [ ] Проверить edge cases (clamp, extrapolate)

---

## Phase 3: Core Ops (vfx-ops, vfx-color)

### 3.1 CDL Operations (vfx-color/cdl.rs)

- [ ] Validate vs ops/cdl/CDLOpData.cpp
  - [ ] ASC_SOP (slope * in + offset)^power
  - [ ] ASC_SAT (saturation)
  - [ ] Clamping modes
  - [ ] Reverse transform
  - [ ] NoClamp style

### 3.2 Missing Ops

- [ ] **ExposureContrastOp**
  - [ ] ops/exposurecontrast/*.cpp
  - [ ] exposure (stops)
  - [ ] contrast
  - [ ] gamma
  - [ ] pivot point
  - [ ] Dynamic properties

- [ ] **RangeOp**
  - [ ] ops/range/*.cpp
  - [ ] min_in, max_in, min_out, max_out
  - [ ] Clamping
  - [ ] NoClamp style

- [ ] **AllocationOp**
  - [ ] ops/allocation/*.cpp
  - [ ] Uniform allocation
  - [ ] Log allocation
  - [ ] vars[] interpretation

- [ ] **LogOp** (generic log)
  - [ ] ops/log/*.cpp
  - [ ] LogAffine (base, logSlope, logOffset, linSlope, linOffset)
  - [ ] LogCamera (linBreak, linearSlope)
  - [ ] Direction (lin->log, log->lin)

- [ ] **ExponentOp**
  - [ ] ops/exponent/*.cpp
  - [ ] Per-channel exponents
  - [ ] Negative handling

### 3.3 Grading Ops (NEW - all missing)

- [ ] **GradingPrimaryOp**
  - [ ] ops/gradingprimary/*.cpp
  - [ ] Brightness
  - [ ] Contrast
  - [ ] Gamma
  - [ ] Saturation
  - [ ] Pivot
  - [ ] Clamp
  - [ ] LOG/LIN/VIDEO styles

- [ ] **GradingToneOp**
  - [ ] ops/gradingtone/*.cpp
  - [ ] Blacks (start, width)
  - [ ] Shadows (start, pivot)
  - [ ] Midtones
  - [ ] Highlights (start, pivot)
  - [ ] Whites (start, width)
  - [ ] S-contrast

- [ ] **GradingRGBCurveOp**
  - [ ] ops/gradingrgbcurve/*.cpp
  - [ ] Master curve
  - [ ] R/G/B curves
  - [ ] B-spline interpolation
  - [ ] Control points

- [ ] **GradingHueCurveOp**
  - [ ] ops/gradinghuecurve/*.cpp
  - [ ] Hue vs Hue
  - [ ] Hue vs Sat
  - [ ] Hue vs Lum
  - [ ] Sat vs Sat
  - [ ] Sat vs Lum
  - [ ] Lum vs Sat

### 3.4 FixedFunction Ops

- [ ] **FixedFunctionOp** (ops/fixedfunction/*.cpp)
  - [ ] ACES_RED_MOD_03
  - [ ] ACES_RED_MOD_10
  - [ ] ACES_GLOW_03
  - [ ] ACES_GLOW_10
  - [ ] ACES_DARK_TO_DIM_10
  - [ ] REC2100_SURROUND
  - [ ] RGB_TO_HSV
  - [ ] HSV_TO_RGB
  - [ ] XYZ_TO_xyY
  - [ ] xyY_TO_XYZ
  - [ ] XYZ_TO_uvY
  - [ ] uvY_TO_XYZ
  - [ ] XYZ_TO_LUV
  - [ ] LUV_TO_XYZ
  - [ ] LIN_TO_PQ, PQ_TO_LIN (fixed params)
  - [ ] и другие...

### 3.5 ACES 2.0 (ops/fixedfunction/ACES2/)

- [ ] **ACES2 Transform.cpp**
  - [ ] New tone mapping
  - [ ] Gamut compression
  - [ ] ColorLib.h functions
  - [ ] MatrixLib.h functions

---

## Phase 4: OCIO Config & Transforms (vfx-ocio)

### 4.1 Missing Transforms

- [ ] **AllocationTransform**
- [ ] **DisplayViewTransform**
- [ ] **ExponentTransform**
- [ ] **ExponentWithLinearTransform**
- [ ] **ExposureContrastTransform**
- [ ] **FixedFunctionTransform**
- [ ] **GradingHueCurveTransform**
- [ ] **GradingPrimaryTransform**
- [ ] **GradingRGBCurveTransform**
- [ ] **GradingToneTransform**
- [ ] **GroupTransform**
- [ ] **LogAffineTransform**
- [ ] **LogCameraTransform**
- [ ] **LogTransform**
- [ ] **LookTransform**
- [ ] **RangeTransform**

### 4.2 Missing Config Components

- [ ] **ViewTransform** (ViewTransform.cpp)
- [ ] **ViewingRules** (ViewingRules.cpp)
- [ ] **FileRules** (FileRules.cpp)
- [ ] **NamedTransform** (NamedTransform.cpp)
- [ ] **GPUProcessor** (GPUProcessor.cpp)
- [ ] **GpuShader** generation

### 4.3 Builtin Transforms Registry

- [ ] Validate builtin.rs covers all from:
  - [ ] transforms/builtins/ACES.cpp
  - [ ] transforms/builtins/ArriCameras.cpp
  - [ ] transforms/builtins/CanonCameras.cpp
  - [ ] transforms/builtins/PanasonicCameras.cpp
  - [ ] transforms/builtins/RedCameras.cpp
  - [ ] transforms/builtins/SonyCameras.cpp
  - [ ] transforms/builtins/AppleCameras.cpp
  - [ ] transforms/builtins/Displays.cpp

---

## Phase 5: SIMD Optimization

### 5.1 LUT1D SIMD

- [ ] Сравнить текущую реализацию с:
  - [ ] Lut1DOpCPU_SSE2.cpp
  - [ ] Lut1DOpCPU_AVX.cpp
  - [ ] Lut1DOpCPU_AVX2.cpp
  - [ ] Lut1DOpCPU_AVX512.cpp
- [ ] Реализовать через `wide` crate или `std::simd`
- [ ] Runtime detection (CPUInfo.cpp)

### 5.2 LUT3D SIMD

- [ ] Сравнить текущую реализацию с:
  - [ ] Lut3DOpCPU_SSE2.cpp
  - [ ] Lut3DOpCPU_AVX.cpp
  - [ ] Lut3DOpCPU_AVX2.cpp
  - [ ] Lut3DOpCPU_AVX512.cpp
- [ ] Tetrahedral interpolation SIMD

### 5.3 Matrix SIMD

- [ ] MatrixOpCPU.cpp SIMD paths
- [ ] 4x4 matrix * vec4 SIMD

### 5.4 Gamma/Transfer SIMD

- [ ] GammaOpCPU.cpp SIMD paths
- [ ] Batch processing

---

## Phase 6: Primaries & Adaptation (vfx-primaries, vfx-math)

### 6.1 Color Primaries

- [ ] Validate all primaries from ColorMatrixHelpers.cpp:
  - [ ] ACES AP0
  - [ ] ACES AP1
  - [ ] sRGB/Rec.709
  - [ ] Rec.2020
  - [ ] DCI-P3
  - [ ] Display P3
  - [ ] Adobe RGB
  - [ ] ProPhoto RGB
  - [ ] CIE RGB
  - [ ] AWG3 (ARRI Wide Gamut 3)
  - [ ] AWG4 (ARRI Wide Gamut 4)
  - [ ] REDWideGamutRGB
  - [ ] S-Gamut3
  - [ ] S-Gamut3.Cine
  - [ ] V-Gamut
  - [ ] Canon Cinema Gamut
  - [ ] и другие...

### 6.2 Chromatic Adaptation

- [ ] Validate adapt.rs:
  - [ ] Bradford matrix
  - [ ] Von Kries
  - [ ] CAT02
  - [ ] CAT16
  - [ ] XYZ Scaling

### 6.3 White Points

- [ ] D50, D55, D60, D65, D75
- [ ] DCI white
- [ ] ACES white (D60 approx)
- [ ] Illuminant A, C, E, F series

---

## Phase 7: Testing & Validation

### 7.1 Reference Test Data

- [ ] Создать тестовые данные из OCIO:
  - [ ] Transfer function reference values
  - [ ] LUT round-trip tests
  - [ ] Matrix precision tests
  - [ ] Grading ops reference

### 7.2 Cross-validation

- [ ] Сравнить результаты с:
  - [ ] ocioconvert output
  - [ ] ociolutimage output
  - [ ] Python PyOpenColorIO

### 7.3 Edge Cases

- [ ] Negative values handling
- [ ] NaN/Inf handling
- [ ] Out-of-range values
- [ ] Precision (f32 vs f64)

---

## Phase 8: Documentation

- [ ] API documentation для каждого crate
- [ ] Примеры использования
- [ ] Migration guide от OCIO C++
- [ ] Performance benchmarks

---

## Progress Tracking

| Phase | Total | Done | Progress |
|-------|-------|------|----------|
| 1. Transfer Functions | ~40 | 0 | 0% |
| 2. LUT Formats | ~25 | 0 | 0% |
| 3. Core Ops | ~50 | 0 | 0% |
| 4. OCIO Config | ~25 | 0 | 0% |
| 5. SIMD | ~15 | 0 | 0% |
| 6. Primaries | ~30 | 0 | 0% |
| 7. Testing | ~15 | 0 | 0% |
| 8. Documentation | ~10 | 0 | 0% |
| **TOTAL** | **~210** | **0** | **0%** |

---

## Current Focus

**Next task**: Phase 1.1 - ARRI LogC4 implementation

Starting point: `_ref/OpenColorIO/src/OpenColorIO/transforms/builtins/ArriCameras.cpp`
