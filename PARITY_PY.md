# Python API Parity Audit

**Цель**: Полное экспонирование Rust API в Python

**Статус**: В процессе
**Дата начала**: 2026-01-06

---

## Текущее состояние

### Экспортировано (25 элементов)
- [x] `Image` - базовый класс изображения
- [x] `Processor` - GPU/CPU обработка (exposure, saturation, contrast, cdl)
- [x] `BitDepth` - enum глубины цвета
- [x] `LayeredImage`, `ImageLayer`, `ImageChannel` - многослойные EXR
- [x] `ChannelKind`, `SampleType` - enums для каналов
- [x] `read()`, `write()`, `read_layered()` - top-level I/O
- [x] `io` submodule - read/write для 6 форматов
- [x] `lut` submodule - Lut1D, Lut3D, ProcessList

---

## ФАЗА 1: ImageBufAlgo Operations (~150 функций)

### 1.1 Подмодуль `vfx_rs.ops` - Основные операции

#### Patterns (7 функций)
- [ ] `zero(width, height, channels=4) -> Image`
- [ ] `fill(width, height, color, channels=4) -> Image`
- [ ] `checker(width, height, size, color1, color2) -> Image`
- [ ] `noise(width, height, type="gaussian", mean=0.0, stddev=1.0) -> Image`
- [ ] `bluenoise(width, height) -> Image`

#### Channels (8 функций)
- [ ] `channels(image, channel_order, fill_value=0.0) -> Image`
- [ ] `channel_append(a, b) -> Image`
- [ ] `channel_sum(image, weights=None) -> Image`
- [ ] `extract_channel(image, channel) -> Image`
- [ ] `channel_flatten(image) -> Image`
- [ ] `get_alpha(image) -> Image`

#### Geometry (18 функций)
- [ ] `crop(image, x, y, width, height) -> Image`
- [ ] `cut(image, x, y, width, height) -> Image`
- [ ] `flip(image) -> Image` - вертикальное отражение
- [ ] `flop(image) -> Image` - горизонтальное отражение
- [ ] `transpose(image) -> Image`
- [ ] `rotate90(image) -> Image`
- [ ] `rotate180(image) -> Image`
- [ ] `rotate270(image) -> Image`
- [ ] `resize(image, width, height, filter="lanczos3") -> Image`
- [ ] `resample(image, width, height) -> Image` - nearest neighbor
- [ ] `fit(image, width, height, filter="lanczos3") -> Image`
- [ ] `paste(dst, src, x, y) -> Image`
- [ ] `rotate(image, angle, center=None) -> Image`
- [ ] `circular_shift(image, x, y) -> Image`
- [ ] `warp(image, stmap) -> Image`
- [ ] `reorient(image, orientation) -> Image`

#### Arithmetic (16 функций)
- [ ] `add(a, b) -> Image` - a + b (image или scalar)
- [ ] `sub(a, b) -> Image` - a - b
- [ ] `mul(a, b) -> Image` - a * b
- [ ] `div(a, b) -> Image` - a / b
- [ ] `mad(a, b, c) -> Image` - a * b + c
- [ ] `abs(image) -> Image`
- [ ] `absdiff(a, b) -> Image`
- [ ] `pow(image, exponent) -> Image`
- [ ] `clamp(image, min=0.0, max=1.0) -> Image`
- [ ] `invert(image) -> Image`
- [ ] `max(a, b) -> Image`
- [ ] `min(a, b) -> Image`

#### Color Operations (14 функций)
- [ ] `premult(image) -> Image`
- [ ] `unpremult(image) -> Image`
- [ ] `repremult(image) -> Image`
- [ ] `saturate(image, factor) -> Image`
- [ ] `contrast_remap(image, black, white) -> Image`
- [ ] `color_map(image, map_name) -> Image`
- [ ] `colormatrixtransform(image, matrix) -> Image`
- [ ] `rangecompress(image, use_luma=False) -> Image`
- [ ] `rangeexpand(image, use_luma=False) -> Image`
- [ ] `srgb_to_linear(image) -> Image`
- [ ] `linear_to_srgb(image) -> Image`

#### Compositing (18 функций)
- [ ] `over(a, b) -> Image` - Porter-Duff over
- [ ] `under(a, b) -> Image`
- [ ] `in_op(a, b) -> Image` - "in" is reserved in Python
- [ ] `out(a, b) -> Image`
- [ ] `atop(a, b) -> Image`
- [ ] `xor(a, b) -> Image`
- [ ] `screen(a, b) -> Image`
- [ ] `multiply(a, b) -> Image`
- [ ] `overlay(a, b) -> Image`
- [ ] `hardlight(a, b) -> Image`
- [ ] `softlight(a, b) -> Image`
- [ ] `difference(a, b) -> Image`
- [ ] `exclusion(a, b) -> Image`
- [ ] `colordodge(a, b) -> Image`
- [ ] `colorburn(a, b) -> Image`
- [ ] `add_blend(a, b) -> Image`

#### Filters (15 функций)
- [ ] `blur(image, sigma) -> Image` - Gaussian blur
- [ ] `median(image, size=3) -> Image`
- [ ] `unsharp_mask(image, radius, strength, contrast=1.0) -> Image`
- [ ] `sharpen(image, amount=1.0) -> Image`
- [ ] `box_blur(image, size) -> Image`
- [ ] `convolve(image, kernel) -> Image`
- [ ] `make_kernel(type, width, height, normalize=True) -> Image`
- [ ] `dilate(image, size=3) -> Image`
- [ ] `erode(image, size=3) -> Image`
- [ ] `morph_open(image, size=3) -> Image`
- [ ] `morph_close(image, size=3) -> Image`
- [ ] `laplacian(image) -> Image`
- [ ] `sobel(image) -> Image`
- [ ] `flood_fill(image, x, y, color) -> Image`

#### FFT (8 функций)
- [ ] `fft(image) -> Image`
- [ ] `ifft(image) -> Image`
- [ ] `polar_to_complex(image) -> Image`
- [ ] `complex_to_polar(image) -> Image`
- [ ] `fft_shift(image) -> Image`
- [ ] `ifft_shift(image) -> Image`

#### Drawing (4 функции)
- [ ] `render_point(image, x, y, color) -> Image`
- [ ] `render_line(image, x1, y1, x2, y2, color) -> Image`
- [ ] `render_box(image, x1, y1, x2, y2, color, fill=False) -> Image`
- [ ] `render_text(image, text, x, y, fontsize, color) -> Image`

#### Screen Capture (4 функции)
- [ ] `capture_image(monitor=0) -> Image`
- [ ] `capture_primary() -> Image`
- [ ] `capture_all_monitors() -> list[Image]`
- [ ] `monitor_count() -> int`

---

## ФАЗА 2: Statistics & Analysis (~15 функций)

### 2.1 Подмодуль `vfx_rs.stats`

#### Statistics
- [ ] `compute_pixel_stats(image) -> PixelStats`
- [ ] `PixelStats` class (min, max, avg, stddev per channel)

#### Comparison
- [ ] `compare(a, b) -> CompareResults`
- [ ] `compare_relative(a, b) -> CompareResults`
- [ ] `compare_yee(a, b, gamma=2.2) -> float`
- [ ] `CompareResults` class (mean_error, max_error, rms_error, PSNR)

#### Analysis
- [ ] `histogram(image, channel=0, bins=256) -> Histogram`
- [ ] `Histogram` class (data, min, max)
- [ ] `is_constant_color(image, color=None) -> bool`
- [ ] `is_constant_channel(image, channel, value=None) -> bool`
- [ ] `is_monochrome(image) -> bool`
- [ ] `color_range_check(image, low, high) -> RangeCheckResult`
- [ ] `nonzero_region(image) -> tuple[int, int, int, int]`
- [ ] `color_count(image, color) -> int`
- [ ] `compute_pixel_hash(image) -> str`
- [ ] `maxchan(image) -> Image`
- [ ] `minchan(image) -> Image`

---

## ФАЗА 3: OCIO Integration (~100 функций)

### 3.1 Подмодуль `vfx_rs.ocio`

#### Config (главный класс)
- [ ] `Config` class
  - [ ] `Config()` - создание пустого
  - [ ] `Config.from_file(path)` - загрузка из файла
  - [ ] `Config.from_env()` - из $OCIO
  - [ ] `Config.aces_1_3()` - встроенный ACES 1.3
  - [ ] `Config.srgb_studio()` - встроенный sRGB
  - [ ] `config.name` property
  - [ ] `config.description` property
  - [ ] `config.version` property
  - [ ] `config.working_dir` property
  - [ ] `config.colorspaces` -> list[str]
  - [ ] `config.colorspace(name)` -> ColorSpace
  - [ ] `config.displays` -> list[str]
  - [ ] `config.views(display)` -> list[str]
  - [ ] `config.default_display` -> str
  - [ ] `config.default_view(display)` -> str
  - [ ] `config.looks` -> list[str]
  - [ ] `config.roles` -> dict[str, str]
  - [ ] `config.processor(src, dst)` -> Processor
  - [ ] `config.display_processor(input, display, view)` -> Processor
  - [ ] `config.processor_with_looks(src, dst, looks)` -> Processor
  - [ ] `config.validate()` -> list[str]
  - [ ] `config.serialize()` -> str
  - [ ] `config.write(path)`

#### ColorSpace
- [ ] `ColorSpace` class
  - [ ] `name` property
  - [ ] `aliases` property
  - [ ] `description` property
  - [ ] `family` property
  - [ ] `encoding` property
  - [ ] `is_data` property
  - [ ] `is_linear` property

#### Processor (OCIO)
- [ ] `OcioProcessor` class
  - [ ] `apply_rgb(pixels)` - apply to numpy array
  - [ ] `apply_rgba(pixels)`
  - [ ] `is_identity` property
  - [ ] `num_ops` property

#### Display / View
- [ ] `Display` class
  - [ ] `name` property
  - [ ] `views` -> list[str]
  - [ ] `default_view` -> str
- [ ] `View` class
  - [ ] `name` property
  - [ ] `colorspace` property
  - [ ] `looks` property

#### Look
- [ ] `Look` class
  - [ ] `name` property
  - [ ] `process_space` property
  - [ ] `description` property

#### Transform Types (24 типа)
- [ ] `MatrixTransform`
- [ ] `CdlTransform`
- [ ] `ExponentTransform`
- [ ] `LogTransform`
- [ ] `FileTransform`
- [ ] `RangeTransform`
- [ ] `ColorSpaceTransform`
- [ ] `LookTransform`
- [ ] `DisplayViewTransform`
- [ ] `GroupTransform`
- [ ] `BuiltinTransform`
- [ ] `Lut1DTransform`
- [ ] `Lut3DTransform`
- [ ] `GradingPrimaryTransform`
- [ ] `GradingRgbCurveTransform`
- [ ] `GradingToneTransform`
- [ ] `ExposureContrastTransform`
- [ ] `FixedFunctionTransform`
- [ ] `LogCameraTransform`
- [ ] и другие...

#### Baker
- [ ] `Baker` class
  - [ ] `Baker(processor)`
  - [ ] `baker.format` property (cube, spi3d, clf...)
  - [ ] `baker.cube_size` property
  - [ ] `baker.bake_to_file(path)`
  - [ ] `baker.generate_1d_lut()` -> numpy
  - [ ] `baker.generate_3d_lut()` -> numpy

#### Enums
- [ ] `Interpolation` enum (Nearest, Linear, Tetrahedral, Best)
- [ ] `TransformDirection` enum (Forward, Inverse)
- [ ] `LutFormat` enum (Cube1D, Cube3D, Spi1D, Spi3D, Clf, Ctf)
- [ ] `BitDepth` enum (для OCIO)

#### Convenience functions
- [ ] `colorconvert(image, from_space, to_space, config=None) -> Image`
- [ ] `ociodisplay(image, display, view, config=None) -> Image`
- [ ] `ociolook(image, looks, config=None) -> Image`
- [ ] `current_config()` -> Config
- [ ] `set_current_config(config)`

---

## ФАЗА 4: Deep Compositing (~60 методов)

### 4.1 Подмодуль `vfx_rs.deep`

#### DeepData class
- [ ] `DeepData` class
  - [ ] `DeepData()` - empty
  - [ ] `DeepData.from_spec(spec)`
  - [ ] `data.pixels` -> int
  - [ ] `data.channels` -> int
  - [ ] `data.samples(pixel)` -> int
  - [ ] `data.set_samples(pixel, count)`
  - [ ] `data.deep_value(pixel, channel, sample)` -> float
  - [ ] `data.set_deep_value(pixel, channel, sample, value)`
  - [ ] `data.channelname(index)` -> str
  - [ ] `data.channeltype(index)` -> TypeDesc
  - [ ] `data.sort(pixel)`
  - [ ] `data.merge_overlaps(pixel)`
  - [ ] `data.opaque_z(pixel)` -> float

#### Deep Operations
- [ ] `deep_flatten(deep, width, height) -> Image`
- [ ] `deepen(image, z_value) -> DeepData`
- [ ] `deepen_with_z(image, z_image) -> DeepData`
- [ ] `deep_merge(a, b) -> DeepData`
- [ ] `deep_holdout(deep, z) -> DeepData`
- [ ] `deep_holdout_matte(deep, holdout) -> DeepData`
- [ ] `deep_tidy(deep)`
- [ ] `deep_stats(deep) -> DeepStats`

#### DeepStats
- [ ] `DeepStats` class
  - [ ] `total_samples` property
  - [ ] `min_samples` property
  - [ ] `max_samples` property
  - [ ] `avg_samples` property

---

## ФАЗА 5: Core Types (OIIO Compatibility)

### 5.1 Подмодуль `vfx_rs.core` или top-level

#### TypeDesc (OIIO-совместимый)
- [ ] `TypeDesc` class
  - [ ] Constants: UINT8, INT8, UINT16, INT16, UINT32, INT32, HALF, FLOAT, DOUBLE, STRING
  - [ ] `TypeDesc(basetype, aggregate=SCALAR)`
  - [ ] `desc.basetype` property
  - [ ] `desc.aggregate` property
  - [ ] `desc.arraylen` property
  - [ ] `desc.size()` -> int bytes
  - [ ] `desc.basesize()` -> int
  - [ ] `desc.is_floating_point()` -> bool
  - [ ] `desc.is_array()` -> bool
  - [ ] `TypeDesc.color()` -> TypeDesc (vec3 float)
  - [ ] `TypeDesc.point()` -> TypeDesc
  - [ ] `TypeDesc.vector()` -> TypeDesc
  - [ ] `TypeDesc.matrix44()` -> TypeDesc

#### BaseType enum
- [ ] `BaseType.Unknown`, `UInt8`, `Int8`, `UInt16`, `Int16`, `UInt32`, `Int32`, `Half`, `Float`, `Double`, `String`

#### Aggregate enum
- [ ] `Aggregate.Scalar`, `Vec2`, `Vec3`, `Vec4`, `Matrix33`, `Matrix44`

#### ImageSpec
- [ ] `ImageSpec` class
  - [ ] `ImageSpec(width, height, channels, format)`
  - [ ] `ImageSpec.rgb(width, height)`
  - [ ] `ImageSpec.rgba(width, height)`
  - [ ] `spec.width`, `spec.height`, `spec.depth` properties
  - [ ] `spec.nchannels` property
  - [ ] `spec.format` property
  - [ ] `spec.channel_names` property
  - [ ] `spec.alpha_channel` property
  - [ ] `spec.z_channel` property
  - [ ] `spec.deep` property
  - [ ] `spec.tile_width`, `spec.tile_height`, `spec.tile_depth`
  - [ ] `spec.full_width`, `spec.full_height` (display window)
  - [ ] `spec.full_x`, `spec.full_y` (display window origin)
  - [ ] `spec.x`, `spec.y`, `spec.z` (data window origin)
  - [ ] `spec.roi()` -> Roi3D
  - [ ] `spec.roi_full()` -> Roi3D
  - [ ] `spec.set_format(format)`
  - [ ] `spec.bytes_per_pixel()` -> int
  - [ ] `spec.image_bytes()` -> int
  - [ ] `spec.scanline_bytes()` -> int
  - [ ] `spec.get_attr(key)` -> AttrValue
  - [ ] `spec.set_attr(key, value)`
  - [ ] `spec.get_string(key)` -> str
  - [ ] `spec.get_int(key)` -> int
  - [ ] `spec.get_float(key)` -> float
  - [ ] `spec.to_xml()` -> str
  - [ ] `ImageSpec.from_xml(xml)` -> ImageSpec

#### Roi3D
- [ ] `Roi3D` class
  - [ ] `Roi3D(xbegin, xend, ybegin, yend, zbegin=0, zend=1, chbegin=0, chend=-1)`
  - [ ] `Roi3D.all()` - unlimited ROI
  - [ ] `Roi3D.from_size(width, height)`
  - [ ] `roi.width`, `roi.height`, `roi.depth` properties
  - [ ] `roi.nchannels` property
  - [ ] `roi.xbegin`, `roi.xend`, `roi.ybegin`, `roi.yend` properties
  - [ ] `roi.zbegin`, `roi.zend`, `roi.chbegin`, `roi.chend` properties
  - [ ] `roi.npixels()` -> int
  - [ ] `roi.defined()` -> bool
  - [ ] `roi.contains(x, y, z=0)` -> bool
  - [ ] `roi.union(other)` -> Roi3D
  - [ ] `roi.intersection(other)` -> Roi3D | None

#### DataFormat enum
- [ ] `DataFormat.U8`, `U16`, `U32`, `F16`, `F32`

---

## ФАЗА 6: Additional I/O

### 6.1 Новые форматы в `vfx_rs.io`

#### WebP
- [ ] `read_webp(path) -> Image`
- [ ] `write_webp(path, image, quality=90, lossless=False)`

#### AVIF
- [ ] `read_avif(path) -> Image`
- [ ] `write_avif(path, image, quality=90, speed=6)`

#### JPEG 2000
- [ ] `read_jp2(path) -> Image`
- [ ] `write_jp2(path, image, quality=None)`

#### HEIF/HEIC
- [ ] `read_heif(path) -> Image`
- [ ] `write_heif(path, image, quality=90)`

### 6.2 ImageCache

- [ ] `ImageCache` class
  - [ ] `ImageCache()`
  - [ ] `ImageCache(max_bytes)`
  - [ ] `cache.get_image(path) -> Image`
  - [ ] `cache.invalidate(path)`
  - [ ] `cache.invalidate_all()`
  - [ ] `cache.stats()` -> CacheStats
  - [ ] `cache.max_size` property
  - [ ] `cache.used_size` property

### 6.3 TextureSystem

- [ ] `TextureSystem` class
  - [ ] `TextureSystem()`
  - [ ] `tex.get_texture(path) -> TextureHandle`
  - [ ] `tex.sample(handle, s, t)` -> tuple[float, ...]
  - [ ] `tex.sample3d(handle, s, t, r)` -> tuple[float, ...]
  - [ ] `tex.environment(handle, dir)` -> tuple[float, ...]
  - [ ] `tex.invalidate(path)`

### 6.4 Sequence

- [ ] `Sequence` class
  - [ ] `Sequence.scan(directory) -> list[Sequence]`
  - [ ] `seq.pattern` property
  - [ ] `seq.frames` -> list[int]
  - [ ] `seq.first_frame` property
  - [ ] `seq.last_frame` property
  - [ ] `seq.frame_count` property
  - [ ] `seq.path_for_frame(frame)` -> str

---

## ФАЗА 7: ColorConfig (OIIO-style)

### 7.1 `vfx_rs.ColorConfig` (альтернатива OCIO)

- [ ] `ColorConfig` class
  - [ ] `ColorConfig()` - default
  - [ ] `ColorConfig.from_file(path)`
  - [ ] `ColorConfig.aces_1_3()`
  - [ ] `ColorConfig.srgb()`
  - [ ] `cfg.valid` property
  - [ ] `cfg.error_message` property
  - [ ] `cfg.colorspaces` -> list[str]
  - [ ] `cfg.has_colorspace(name)` -> bool
  - [ ] `cfg.is_colorspace_linear(name)` -> bool
  - [ ] `cfg.displays` -> list[str]
  - [ ] `cfg.views(display)` -> list[str]
  - [ ] `cfg.default_display` -> str
  - [ ] `cfg.default_view(display)` -> str
  - [ ] `cfg.looks` -> list[str]
  - [ ] `cfg.roles` -> dict[str, str]
  - [ ] `cfg.scene_linear` -> str | None
  - [ ] `cfg.processor(from_space, to_space)` -> OcioProcessor
  - [ ] `cfg.display_processor(input, display, view)` -> OcioProcessor

---

## ФАЗА 8: Image class enhancements

### 8.1 Дополнительные методы Image

#### Properties
- [ ] `image.spec` -> ImageSpec
- [ ] `image.roi` -> Roi3D
- [ ] `image.dtype` -> str (numpy-style: 'float32', 'float16', 'uint16', 'uint8')
- [ ] `image.is_float` -> bool
- [ ] `image.has_alpha` -> bool

#### Методы как у PIL/OpenCV (опционально, chainable)
- [ ] `image.resize(width, height, filter="lanczos3") -> Image`
- [ ] `image.crop(x, y, width, height) -> Image`
- [ ] `image.rotate(angle) -> Image`
- [ ] `image.flip() -> Image`
- [ ] `image.flop() -> Image`
- [ ] `image.blur(sigma) -> Image`
- [ ] `image.sharpen(amount) -> Image`
- [ ] `image.premultiply() -> Image`
- [ ] `image.unpremultiply() -> Image`
- [ ] `image.to_linear() -> Image`
- [ ] `image.to_srgb() -> Image`
- [ ] `image.clamp(min, max) -> Image`
- [ ] `image.pow(exp) -> Image`

---

## ФАЗА 9: Stub Files (.pyi)

### 9.1 Генерация type stubs

- [ ] `vfx_rs/__init__.pyi`
- [ ] `vfx_rs/io.pyi`
- [ ] `vfx_rs/lut.pyi`
- [ ] `vfx_rs/ops.pyi`
- [ ] `vfx_rs/stats.pyi`
- [ ] `vfx_rs/ocio.pyi`
- [ ] `vfx_rs/deep.pyi`
- [ ] `vfx_rs/core.pyi`

---

## Прогресс

| Фаза | Описание | Элементов | Статус |
|------|----------|-----------|--------|
| 0 | Базовый API | 25 | ✅ Done |
| 1 | ImageBufAlgo Operations | ~150 | ⬜ TODO |
| 2 | Statistics & Analysis | ~15 | ⬜ TODO |
| 3 | OCIO Integration | ~100 | ⬜ TODO |
| 4 | Deep Compositing | ~60 | ⬜ TODO |
| 5 | Core Types (OIIO) | ~40 | ⬜ TODO |
| 6 | Additional I/O | ~30 | ⬜ TODO |
| 7 | ColorConfig | ~20 | ⬜ TODO |
| 8 | Image Enhancements | ~20 | ⬜ TODO |
| 9 | Stub Files | 8 | ⬜ TODO |

**Всего**: ~470 элементов API

---

## Порядок реализации

1. **Фаза 5: Core Types** - TypeDesc, ImageSpec, Roi3D (основа для всего)
2. **Фаза 1: ops** - операции ImageBufAlgo (основной функционал)
3. **Фаза 2: stats** - статистика и анализ
4. **Фаза 3: ocio** - цветовое управление
5. **Фаза 6: io** - дополнительные форматы
6. **Фаза 4: deep** - deep compositing
7. **Фаза 7: ColorConfig** - OIIO-style color
8. **Фаза 8: Image** - улучшения класса Image
9. **Фаза 9: stubs** - .pyi файлы

---

## Архитектурные решения

### Структура модуля
```
vfx_rs/
├── __init__.py (или .so)
│   ├── Image
│   ├── Processor
│   ├── BitDepth
│   ├── read(), write(), read_layered()
│   └── LayeredImage, ImageLayer, ImageChannel
│
├── io/
│   ├── read_exr(), write_exr()
│   ├── read_png(), write_png()
│   ├── ... (все форматы)
│   ├── ImageCache
│   ├── TextureSystem
│   └── Sequence
│
├── ops/
│   ├── # Patterns
│   ├── zero(), fill(), checker(), noise()
│   ├── # Channels
│   ├── channels(), channel_append(), extract_channel()
│   ├── # Geometry
│   ├── crop(), resize(), rotate(), flip(), warp()
│   ├── # Arithmetic
│   ├── add(), sub(), mul(), div(), pow(), clamp()
│   ├── # Color
│   ├── premult(), unpremult(), saturate(), srgb_to_linear()
│   ├── # Composite
│   ├── over(), under(), screen(), multiply(), overlay()
│   ├── # Filters
│   ├── blur(), median(), sharpen(), unsharp_mask()
│   ├── # FFT
│   ├── fft(), ifft()
│   └── # Drawing
│       └── render_line(), render_box(), render_text()
│
├── stats/
│   ├── compute_pixel_stats(), PixelStats
│   ├── compare(), CompareResults
│   ├── histogram(), Histogram
│   └── is_constant_color(), is_monochrome()
│
├── ocio/
│   ├── Config
│   ├── ColorSpace
│   ├── OcioProcessor
│   ├── Display, View
│   ├── Look
│   ├── Baker
│   ├── Transform types (24)
│   └── colorconvert(), ociodisplay()
│
├── deep/
│   ├── DeepData
│   ├── DeepStats
│   └── deep_flatten(), deepen(), deep_merge()
│
├── core/
│   ├── TypeDesc
│   ├── ImageSpec
│   ├── Roi3D
│   ├── BaseType, Aggregate, DataFormat
│   └── AttrValue
│
└── lut/
    ├── Lut1D, Lut3D
    ├── ProcessList
    └── read_cube(), read_clf()
```

### Naming conventions
- Функции: snake_case (`read_exr`, `channel_append`)
- Классы: PascalCase (`ImageSpec`, `DeepData`)
- Enums: PascalCase с members в PascalCase (`BitDepth.Bit10`)
- Properties: snake_case (`image.width`, `spec.nchannels`)

### Error handling
- Все I/O ошибки -> `IOError`
- Ошибки валидации -> `ValueError`
- Ошибки индексации -> `IndexError`
- Общие ошибки -> `RuntimeError`
