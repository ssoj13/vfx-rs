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

### 2.1 Transfer Functions в GPU
**Файл:** `gpu.rs`

Сейчас `ProcessorOp::Transfer` возвращает `None` в GPU.

- [ ] Добавить GLSL для sRGB OETF/EOTF
- [ ] Добавить GLSL для Rec.709
- [ ] Добавить GLSL для PQ (ST.2084)
- [ ] Добавить GLSL для HLG
- [ ] Добавить GLSL для Log transforms (ACEScct, LogC, etc)

### 2.2 LUT Texture Support
**Файл:** `gpu.rs`

- [ ] `GpuTexture` struct для 1D/3D LUT данных
- [ ] Генерация sampler uniforms
- [ ] GLSL texture lookup код
- [ ] `textures()` метод возвращает LUT данные для upload

### 2.3 ExposureContrast в GPU
- [ ] GLSL implementation

### 2.4 FixedFunction в GPU
- [ ] ACES RRT в GLSL
- [ ] ACES ODT в GLSL (или skip с fallback на CPU)

### 2.5 Grading в GPU
- [ ] GradingPrimary GLSL
- [ ] GradingTone GLSL
- [ ] GradingRgbCurve (через 1D LUT texture)

---

## Phase 3: Performance Optimization

### 3.1 SIMD для CPU Processor
**Файл:** `processor.rs` или новый `simd.rs`

- [ ] Feature flag `simd`
- [ ] SSE4.2 для matrix multiply (4 pixels at once)
- [ ] AVX2 для 8 pixels at once
- [ ] NEON для ARM

### 3.2 Processor Caching
- [ ] Cache compiled processors by (src, dst) pair
- [ ] Lazy compilation
- [ ] Thread-safe cache

### 3.3 Operation Fusion
- [ ] Merge consecutive Matrix ops (уже есть частично)
- [ ] Merge Range + Matrix
- [ ] Skip identity ops

---

## Phase 4: Config Authoring API

### 4.1 Config Builder
**Файл:** новый `config_builder.rs`

```rust
let config = ConfigBuilder::new()
    .add_colorspace(ColorSpace::builder("ACEScg")...)
    .add_display("sRGB", vec![...])
    .add_look("shot_grade", ...)
    .build();
```

- [ ] ConfigBuilder struct
- [ ] add_colorspace()
- [ ] add_display()
- [ ] add_view()
- [ ] add_look()
- [ ] set_role()
- [ ] build() -> Config

### 4.2 Config Serialization
**Файл:** `config.rs`

- [ ] `Config::to_string()` -> OCIO YAML
- [ ] `Config::write_to_file()`
- [ ] Roundtrip tests (parse -> serialize -> parse)

---

## Phase 5: Advanced Features

### 5.1 Dynamic Properties
- [ ] DynamicProperty trait
- [ ] Runtime exposure/contrast adjustment
- [ ] GPU uniform updates

### 5.2 Baker
**Файл:** новый `baker.rs`

- [ ] `Baker::bake_lut_1d()` - bake processor to 1D LUT
- [ ] `Baker::bake_lut_3d()` - bake processor to 3D LUT
- [ ] `Baker::bake_icc()` - bake to ICC profile
- [ ] Export to .cube, .clf formats

### 5.3 Context Path Resolution
- [ ] `$SHOT`, `$SEQ` в FileTransform paths
- [ ] Search paths для LUT files
- [ ] Environment variable expansion

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
- [ ] Phase 2: GPU Completion (1/5) - LogAffine/LogCamera/ExponentWithLinear GLSL done
- [ ] Phase 3: Performance (0/3)
- [ ] Phase 4: Config Authoring (0/2)
- [ ] Phase 5: Advanced (0/3)

**Overall: ~25%**

Last updated: 2026-01-09
