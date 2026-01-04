# Plan TODO: VFX-RS Implementation Status vs ULTRA.md

This document is a code-verified status map and an actionable checklist.
It is intended as a detailed guide to close gaps against ULTRA.md.

Legend:
- [x] Implemented (confirmed in code)
- [~] Partial / basic support
- [ ] Missing

---

## 0. Source of truth references (code paths)

- EXR IO: `crates/vfx-io/src/exr.rs`
- IO registry + supported formats: `crates/vfx-io/src/lib.rs`
- Core pixel alpha ops: `crates/vfx-core/src/pixel.rs`
- Ops: `crates/vfx-ops/src/*`
- OCIO config / display pipeline / file rules: `crates/vfx-ocio/src/config.rs`
- OCIO processor ops: `crates/vfx-ocio/src/processor.rs`
- OCIO builtins (ACES 1.3 minimal): `crates/vfx-ocio/src/builtin.rs`
- LUT formats + domain: `crates/vfx-lut/src/*`
- Transfer curves: `crates/vfx-transfer/src/*`
- CLI: `crates/vfx-cli/src/main.rs`, `crates/vfx-cli/src/commands/*`

---

## 1. Image I/O

### 1.1 Formats (read/write)

Implemented (in `vfx-io`):
- [x] EXR (basic) - `crates/vfx-io/src/exr.rs`
- [x] HDR/RGBE - `crates/vfx-io/src/hdr.rs`
- [x] PNG - `crates/vfx-io/src/png.rs`
- [x] JPEG - `crates/vfx-io/src/jpeg.rs`
- [x] TIFF - `crates/vfx-io/src/tiff.rs`
- [x] DPX - `crates/vfx-io/src/dpx.rs`

Missing:
- [ ] JPEG2000 (JP2)
- [ ] WebP
- [ ] AVIF
- [ ] GIF (animated)
- [ ] PSD
- [ ] RAW (libraw or equivalent)
- [ ] Video I/O (FFmpeg)

Action steps:
- [ ] Decide external crates/backends per format (e.g., `image`, `libwebp`, `libavif`, `ffmpeg-sys`).
- [ ] Add format detection to `crates/vfx-io/src/detect.rs`.
- [ ] Add new Format variants in `crates/vfx-io/src/detect.rs` + `crates/vfx-io/src/lib.rs` dispatch.
- [ ] Implement `FormatReader`/`FormatWriter` for each format.
- [ ] Add CLI entry points if needed (`vfx-cli`).
- [ ] Add tests in `crates/vfx-tests` or format-specific unit tests.

### 1.2 EXR advanced features

Confirmed current behavior:
- [~] Multi-layer: reads **first RGBA layer only** (`read().first_valid_layer()`), writer supports **single layer** with `layer_name` metadata only. (`crates/vfx-io/src/exr.rs`)
- [ ] Deep data: explicitly disabled (`.no_deep_data()`)
- [ ] Tiled images
- [ ] Mipmap levels

Action steps:
- [ ] Extend EXR reader to enumerate all layers and expose to `ImageData` (or new multi-layer container).
- [ ] Add deep data reading/writing; define deep data structure.
- [ ] Add tiled EXR read/write (tile size metadata + storage).
- [ ] Add multi-resolution mipmaps (proper EXR levels).
- [ ] Update `vfx-io::ImageData` or introduce `ImageLayers` to hold multi-layer data.

### 1.3 Metadata

Confirmed current behavior:
- [~] EXIF: basic metadata sizes present (JPEG/PNG) but not full tag parsing.
- [~] XMP: size captured (JPEG), not parsed.
- [ ] IPTC: not present.
- [x] Custom attrs container exists (`crates/vfx-io/src/attrs/*`).

Action steps:
- [ ] Add proper EXIF tag parsing (e.g., via `exif` crate) and map to `Attrs`.
- [ ] Add XMP parsing (XML) into `Attrs` structured fields.
- [ ] Add IPTC parsing or skip explicitly with a doc note.
- [ ] Normalize metadata key naming to a documented scheme.

---

## 2. Image Processing (vfx-ops)

Implemented modules:
- [x] resize (with 4 filters) - `crates/vfx-ops/src/resize.rs`
- [x] composite + blend modes - `crates/vfx-ops/src/composite.rs`
- [x] filter / convolution (blur/sharpen/edge kernels) - `crates/vfx-ops/src/filter.rs`
- [x] transform: crop/flip/rotate 90/180/pad - `crates/vfx-ops/src/transform.rs`

Missing critical ops (ULTRA P0):
- [ ] paste
- [x] premult/unpremult (present in `vfx-ops/src/composite.rs` and `vfx-core/src/pixel.rs`)
- [ ] warp/distortion
- [ ] rotate arbitrary angle
- [ ] deep operations

Missing important ops (ULTRA P1):
- [ ] FFT/IFFT
- [ ] dilate/erode
- [ ] median filter
- [ ] text rendering
- [ ] noise generation
- [ ] demosaic
- [ ] transpose
- [ ] reorient (EXIF)

Action steps:
- [ ] Add `paste` op (copy sub-rect into destination with bounds + alpha). Add tests.
- [ ] Add `rotate_arbitrary` with interpolation (nearest/bilinear/bicubic) and update CLI.
- [ ] Add `warp` framework (grid-based mapping or UV mapping) + bilinear sampling.
- [ ] Add `dilate/erode` (morphology) with kernel control.
- [ ] Add `median` filter for denoise.
- [ ] Add `noise` (Perlin/value/simple) generator.
- [ ] Add `text` rendering (font rasterization; consider `rusttype`/`fontdue`).
- [ ] Add `demosaic` for Bayer patterns.
- [ ] Add `transpose`/`reorient` ops and EXIF orientation integration.
- [ ] Decide how to handle deep images before adding deep ops.

---

## 3. Color Management (vfx-ocio / vfx-color)

### 3.1 OCIO config parsing

Implemented:
- [x] Colorspaces, roles, displays, looks, view transforms parsing - `crates/vfx-ocio/src/config.rs`
- [x] File rules parsing + validation (default last) - `crates/vfx-ocio/src/config.rs`

Partial:
- [~] Transform parsing is limited to: Matrix/File/Exponent/Log/CDL/ColorSpace/Builtin/Range (`crates/vfx-ocio/src/config.rs`).
- [ ] FixedFunction, ExposureContrast, Look, DisplayView in config parsing.

Action steps:
- [ ] Extend `parse_raw_transform_def` in `crates/vfx-ocio/src/config.rs` to handle:
  - FixedFunctionTransform
  - ExposureContrastTransform
  - LookTransform
  - DisplayViewTransform
- [ ] Ensure schema in raw structs is defined for these transforms.

### 3.2 OCIO processor ops

Implemented:
- [x] Matrix
- [x] LUT 1D/3D
- [x] CDL
- [x] Exponent
- [x] Log (basic)
- [x] Range
- [x] Group
- [x] FileTransform (LUT load) - partial
- [x] BuiltinTransform (ACES) - partial
- [x] FixedFunction (RGB<->HSV, ACES Gamut Comp) - `crates/vfx-ocio/src/processor.rs`
- [x] ExposureContrast - `crates/vfx-ocio/src/processor.rs`

Missing / partial:
- [~] FileTransform: only LUT types supported; non-LUT file types not handled.
- [~] BuiltinTransform: limited set.
- [ ] GPU shader generation.
- [ ] GPU processing.
- [ ] Real-time preview pipeline.

Action steps:
- [ ] Extend file transform handler for .ctf/.3dl/.icc (if desired).
- [ ] Expand BuiltinTransform library coverage.
- [ ] Define GPU pipeline design (format + shader generation targets).

### 3.3 Display pipeline

Implemented:
- [x] display_processor uses view + looks + view_transform + colorspace (`crates/vfx-ocio/src/config.rs`).

Action steps:
- [ ] Add tests covering OCIO v2 display/view semantics (scene vs display reference).

---

## 4. LUT formats (vfx-lut)

Implemented:
- [x] .cube
- [x] .clf (basic)
- [x] .spi1d/.spi3d
- [x] .cdl
- [x] LUT domain support (LUT1D/LUT3D domains + OCIO ops use domain)

Missing:
- [ ] .3DL
- [ ] .CTF

Action steps:
- [ ] Add parsers/writers for .3DL and .CTF.
- [ ] Ensure LUT domain handling is correctly mapped for all LUT types.

---

## 5. ACES support

Implemented:
- [x] ACES color spaces AP0/AP1 basics in builtin `aces_1_3`.
- [x] ACEScc/ACEScct transfer functions (`crates/vfx-transfer`).

Partial / missing:
- [~] ACES config: minimal; no IDT/RRT/ODT transforms.
- [ ] ACES IDTs (camera input transforms).
- [ ] Full RRT/ODT chain (currently simplified matrices only).

Action steps:
- [ ] Add ACES 1.3 config data with IDT/RRT/ODT transforms.
- [ ] Decide if to ship ACES transforms as builtins or external config files.
- [ ] Add validation tests that compare to known OCIO outputs.

---

## 6. Transfer functions

Implemented (confirmed in `crates/vfx-transfer`):
- [x] sRGB
- [x] Gamma 2.2/2.4
- [x] Rec.709 BT.1886
- [x] PQ (ST 2084)
- [x] HLG (BT.2100)
- [x] ACEScct
- [x] ACEScc
- [x] LogC (ARRI) (basic)
- [x] S-Log3 (basic)
- [x] V-Log (basic)

Missing:
- [ ] S-Log2
- [ ] REDLog
- [ ] BMDFilm

Action steps:
- [ ] Implement missing curves.
- [ ] Add tests against published curve specs.

---

## 7. CLI commands

Implemented (based on `crates/vfx-cli/src/main.rs` and commands):
- [x] info, convert, resize, crop, diff, composite, blur, sharpen, color, lut, transform, maketx, grep, batch

Partial:
- [~] maketx: generates mipmaps but only saves base image; no tiled texture output.

Action steps:
- [ ] Add CLI for new ops (paste/warp/rotate/etc) when implemented.
- [ ] Upgrade `maketx` to write tiled + full mip chain format.

---

## 8. Caching / Texture / UDIM

Missing:
- [ ] ImageCache
- [ ] TextureSystem
- [ ] UDIM support

Action steps:
- [ ] Define cache architecture (LRU, disk cache policy).
- [ ] Add UDIM resolver in `vfx-io::sequence` or new module.

---

## 9. Plugin / Extensibility

Current:
- [~] Static compile-time formats/ops only.

Missing:
- [ ] Dynamic loading for formats/ops.

Action steps:
- [ ] Decide plugin API (C ABI, Rust dynamic libs, feature flags).

---

## 10. Immediate corrections vs ULTRA.md (based on code)

ULTRA.md items that are outdated and should be updated:
- [x] premult/unpremult are actually implemented.
- [x] display pipeline is wired (basic).
- [x] file rules are implemented.
- [x] LUT domain handling exists in LUT + OCIO processing.
- [x] FixedFunction and ExposureContrast ops exist in processor (but not config parsing).

Action steps:
- [ ] Update `ULTRA.md` to reflect the above.

---

## 11. Suggested execution order (practical roadmap)

P0 (critical for production viability):
- [ ] Multi-layer EXR read/write
- [ ] Deep EXR data model + read/write
- [ ] paste op
- [ ] rotate arbitrary angle + warp/distortion
- [ ] ACES IDT + RRT/ODT pipeline

P1 (important):
- [ ] Extended IO formats (WebP/AVIF/JP2)
- [ ] More camera log curves
- [ ] FFT/median/morphology
- [ ] .3DL/.CTF LUT support

P2 (nice-to-have):
- [ ] ImageCache/TextureSystem
- [ ] GPU shader generation
- [ ] Video I/O

---

## 12. Next concrete steps (if starting now)

1) EXR multi-layer support
- [ ] Define a `MultiLayerImage` structure (or extend `ImageData`).
- [ ] Update `exr::read_impl` to iterate all layers (not just `first_valid_layer`).
- [ ] Update `exr::write` to write multiple layers.
- [ ] Add tests with multi-layer fixtures.

2) paste + rotate arbitrary
- [ ] Implement `paste` in `vfx-ops`.
- [ ] Implement `rotate_arbitrary` with interpolation.
- [ ] Add CLI commands and tests.

3) ACES pipeline
- [ ] Add reference ACES config or embed transform data.
- [ ] Implement IDT/RRT/ODT transforms in OCIO pipeline.
- [ ] Add test images + expected outputs.

---

## 13. Status summary (short)

- Core architecture is stable; IO and color core are in place.
- The biggest production blockers are EXR advanced features and missing ops.
- OCIO pipeline is functional but incomplete in config parsing and builtins.
- CLI covers basic operations but not advanced workflows.
