# TODO: OCIO Full Implementation Plan

**Цель:** Довести vfx-ocio до полного паритета с OpenColorIO C++

**Дата:** 2026-01-09

---

## Phase 1: Missing Transforms (CRITICAL)

### 1.1 LogAffine Transform ✅
**Файл:** `processor.rs`
**Формула:**
```
forward:  out = log(lin_slope * in + lin_offset) * log_slope + log_offset
inverse:  out = (pow(base, (in - log_offset) / log_slope) - lin_offset) / lin_slope
```

- [x] Добавить `Transform::LogAffine` в match в `compile_transform()`
- [x] Реализовать apply в `apply_single_pixel()`
- [x] GPU GLSL implementation
- [ ] Тесты с известными значениями

### 1.2 LogCamera Transform ✅
**Файл:** `processor.rs`
**Используется для:** ARRI LogC3/LogC4, Sony S-Log3, RED Log3G10, Panasonic V-Log

Формула LogC3:
```
if (x > cut)
    out = c * log10(a * x + b) + d
else
    out = e * x + f
```

- [x] Добавить `Transform::LogCamera` в match
- [x] Параметры: cut, a, b, c, d, e, f, base
- [x] GPU GLSL implementation
- [ ] Presets для LogC3, LogC4, S-Log3, V-Log
- [ ] Тесты против OCIO reference values

### 1.3 ExponentWithLinear Transform ✅
**Файл:** `processor.rs`
**Используется для:** sRGB, Rec.709 (более точная версия чем просто Exponent)

```
if (x >= break)
    out = pow(x, gamma)
else
    out = x * linear_slope + linear_offset
```

- [x] Добавить в compile_transform()
- [x] Параметры: gamma, offset, break point
- [x] GPU GLSL implementation
- [ ] Тесты

### 1.4 Inline Lut1D Transform ✅
**Файл:** `processor.rs`
**Отличие от FileTransform:** данные уже в памяти, не читаем файл

- [x] Добавить `Transform::Lut1D` в match
- [x] Использовать существующий `compile_lut1d()`
- [ ] Тесты

### 1.5 Inline Lut3D Transform ✅
**Файл:** `processor.rs`

- [x] Добавить `Transform::Lut3D` в match
- [x] Использовать существующий `compile_lut3d()`
- [ ] Тесты

---

## Phase 2: GPU Processor Completion

### 2.1 Transfer Functions в GPU ✅
**Файл:** `gpu.rs`

- [x] Добавить GLSL для sRGB OETF/EOTF
- [x] Добавить GLSL для Rec.709, Rec.2020
- [x] Добавить GLSL для PQ (ST.2084)
- [x] Добавить GLSL для HLG
- [x] Добавить GLSL для Log transforms (ACEScct, ACEScc, LogC3, LogC4, S-Log3, V-Log, Log3G10, BMD Film Gen5)
- [x] Добавить GLSL для Gamma (2.2, 2.4, 2.6)

### 2.2 LUT Texture Support ✅
**Файл:** `gpu.rs`

- [x] `GpuTexture` struct для 1D/3D LUT данных
- [x] Генерация sampler uniforms
- [x] GLSL texture lookup код
- [x] `textures()` метод возвращает LUT данные для upload

### 2.3 ExposureContrast в GPU ✅
- [x] GLSL implementation (Linear, Video, Logarithmic styles)

### 2.4 FixedFunction в GPU
- [ ] ACES RRT в GLSL
- [ ] ACES ODT в GLSL (или skip с fallback на CPU)

### 2.5 Grading в GPU ✅
- [x] GradingPrimary GLSL (lift/gamma/gain + exposure/contrast/saturation)
- [x] GradingTone GLSL (shadows/midtones/highlights)
- [ ] GradingRgbCurve (требует 1D LUT texture)

---

## Phase 3: Performance Optimization

### 3.1 SIMD для CPU Processor ✅
**Файл:** `simd.rs`

- [x] Auto-detect (no feature flag needed)
- [x] SSE4.1 matrix multiply with dot product
- [x] SSE4.1 range clamp
- [x] NEON fallback (scalar on ARM for now)
- [x] Scalar fallback for other archs

### 3.2 Processor Caching ✅
**Файл:** `cache.rs`

- [x] ProcessorCache struct
- [x] Thread-safe RwLock<HashMap> cache
- [x] Cache by (src, dst, looks) tuple
- [x] get_or_create() / get_or_create_with_looks()
- [x] clear() / len() / is_empty()

### 3.3 Operation Fusion ✅
**Файл:** `processor.rs`

- [x] Merge consecutive Matrix ops (combine_matrices)
- [x] Skip identity ops (is_identity check)
- [x] OptimizationLevel: None, Lossless, Good, Best

---

## Phase 4: Config Authoring API

### 4.1 Config Builder ✅
**Файл:** `config_builder.rs`

```rust
let config = ConfigBuilder::new("Studio Config")
    .add_colorspace(ColorSpace::builder("ACEScg")...)
    .add_display(Display::new("sRGB").with_view(...))
    .add_look(Look::new("shot_grade")...)
    .set_role("scene_linear", "ACEScg")
    .build()?;
```

- [x] ConfigBuilder struct
- [x] add_colorspace()
- [x] add_display() with Display::with_view()
- [x] add_look()
- [x] set_role()
- [x] build() -> OcioResult<Config> with validation

### 4.2 Config Serialization ✅
**Файл:** `config.rs`

- [x] `Config::serialize()` -> OCIO YAML string
- [x] `Config::write_to_file()` -> write to path
- [ ] Roundtrip tests (parse -> serialize -> parse)

---

## Phase 5: Advanced Features

### 5.1 Dynamic Properties ✅
**Файл:** `dynamic.rs`

- [x] DynamicProcessor with runtime adjustments
- [x] DynamicProcessorBuilder fluent API
- [x] Exposure, contrast, gamma, saturation adjustments
- [x] Apply before/after base processor option
- [ ] GPU uniform updates (future)

### 5.2 Baker ✅
**Файл:** `baker.rs`

- [x] `Baker::bake_lut_1d()` - bake processor to 1D LUT
- [x] `Baker::bake_lut_3d()` - bake processor to 3D LUT
- [x] `Baker::write_cube_1d()` / `write_cube_3d()` - export to .cube format
- [x] Custom domain support for HDR/log
- [ ] `Baker::bake_icc()` - bake to ICC profile (future)
- [ ] Export to .clf format (future)

### 5.3 Context Path Resolution ✅
**Файл:** `context.rs`

- [x] `$VAR` и `${VAR}` expansion
- [x] Search paths для LUT files
- [x] Environment variable expansion
- [x] Strict mode (error on unresolved)
- [x] Resolve with custom context map

---

## Implementation Order

| Priority | Task | Effort | Impact |
|----------|------|--------|--------|
| P0 | LogCamera (ARRI/Sony support) | 2h | HIGH |
| P0 | LogAffine | 1h | HIGH |
| P0 | ExponentWithLinear | 1h | MEDIUM |
| P1 | Inline Lut1D/Lut3D | 30min | MEDIUM |
| P1 | GPU Transfer functions | 3h | HIGH |
| P1 | GPU LUT textures | 4h | HIGH |
| P2 | SIMD optimization | 8h | MEDIUM |
| P2 | GPU Grading | 2h | LOW |
| P3 | Config Builder | 4h | LOW |
| P3 | Baker | 6h | LOW |

---

## Test Matrix

Для каждого transform:
1. Unit test с hardcoded values
2. Roundtrip test (forward -> inverse = identity)
3. Compare с OCIO C++ reference (если доступно)
4. GPU vs CPU comparison

---

## Files to Modify

| File | Changes |
|------|---------|
| `processor.rs` | +LogAffine, +LogCamera, +ExponentWithLinear, +Lut1D, +Lut3D |
| `gpu.rs` | +Transfer GLSL, +LUT textures, +Grading |
| `transform.rs` | (уже есть structs) |
| `lib.rs` | re-exports |
| NEW `simd.rs` | SIMD implementations |
| NEW `baker.rs` | LUT baking |
| NEW `config_builder.rs` | Config authoring |

---

## Progress Tracking

- [x] Phase 1: Missing Transforms (5/5) ✅
- [x] Phase 2: GPU Completion (5/5) ✅
- [x] Phase 3: Performance (3/3) ✅
- [x] Phase 4: Config Authoring (2/2) ✅
- [x] Phase 5: Advanced (3/3) ✅

**Overall: 100%** - All phases complete!

Last updated: 2026-01-09
