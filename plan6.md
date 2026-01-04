# Plan TODO: VFX-RS Implementation Status vs ULTRA.md

This document is a code-verified status map and an actionable checklist.
Last updated: 2025-01-03

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
- Layer ops: `crates/vfx-ops/src/layer_ops.rs`
- Warp/distortion: `crates/vfx-ops/src/warp.rs`
- ACES transforms: `crates/vfx-color/src/aces.rs`
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
- [x] EXR (multi-layer) - `crates/vfx-io/src/exr.rs`
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

### 1.2 EXR advanced features

Current status:
- [x] Multi-layer read/write - `ExrReader::read_layers`, `ExrWriter::write_layers`
- [x] Layer extraction/merging - CLI commands `extract-layer`, `merge-layers`
- [ ] Deep data: explicitly disabled (`.no_deep_data()`)
- [ ] Tiled images (scanline only)
- [ ] Mipmap levels

### 1.3 Metadata

- [~] EXIF: basic metadata sizes present (JPEG/PNG) but not full tag parsing.
- [~] XMP: size captured (JPEG), not parsed.
- [ ] IPTC: not present.
- [x] Custom attrs container exists (`crates/vfx-io/src/attrs/*`).

---

## 2. Image Processing (vfx-ops)

### Implemented:
- [x] resize (Nearest, Bilinear, Lanczos3, Mitchell) - `resize.rs`
- [x] composite + blend modes (over, add, multiply, screen) - `composite.rs`
- [x] filter / convolution (blur/sharpen/edge kernels) - `filter.rs`
- [x] transform: crop/flip/rotate90/180/pad/tile - `transform.rs`
- [x] paste (with alpha blending) - `transform.rs`
- [x] rotate arbitrary angle (bilinear interpolation) - `transform.rs`
- [x] warp/distortion (barrel, pincushion, fisheye, twist, wave, spherize, ripple) - `warp.rs`
- [x] premult/unpremult - `composite.rs` and `vfx-core/src/pixel.rs`
- [x] layer_ops (resize, blur, crop, sharpen for ImageLayer) - `layer_ops.rs`
- [x] channel shuffle/extract - CLI commands

### Missing:
- [ ] FFT/IFFT
- [ ] dilate/erode (morphology)
- [ ] median filter
- [ ] text rendering
- [ ] noise generation
- [ ] demosaic
- [ ] transpose
- [ ] reorient (EXIF)
- [ ] deep operations

---

## 3. Color Management (vfx-ocio / vfx-color)

### 3.1 OCIO config parsing

Implemented:
- [x] Colorspaces, roles, displays, looks, view transforms parsing
- [x] File rules parsing + validation (default last)

Partial:
- [~] Transform parsing: Matrix/File/Exponent/Log/CDL/ColorSpace/Builtin/Range
- [ ] FixedFunction in config parsing (implemented in processor, not config)
- [ ] ExposureContrast in config parsing (implemented in processor, not config)
- [ ] LookTransform, DisplayViewTransform in config parsing

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
- [x] FixedFunction (RGB<->HSV, ACES Gamut Comp)
- [x] ExposureContrast

Missing:
- [ ] GPU shader generation
- [ ] GPU processing
- [ ] Real-time preview pipeline

### 3.3 Display pipeline

- [x] display_processor uses view + looks + view_transform + colorspace

---

## 4. LUT formats (vfx-lut)

Implemented:
- [x] .cube
- [x] .clf (basic)
- [x] .spi1d/.spi3d
- [x] .cdl
- [x] LUT domain support

Missing:
- [ ] .3DL
- [ ] .CTF

---

## 5. ACES support

Implemented:
- [x] ACES color spaces AP0/AP1 - `vfx-primaries`
- [x] ACEScc/ACEScct transfer functions - `vfx-transfer`
- [x] ACES RRT (filmic tonemap) - `vfx-color/src/aces.rs`
- [x] ACES ODT (ACEScg -> sRGB/Rec.709) - `vfx-color/src/aces.rs`
- [x] ACES IDT (sRGB -> ACEScg) - `vfx-color/src/aces.rs`
- [x] CLI `aces` command with idt/rrt/odt/rrt-odt transforms

Partial:
- [~] ACES config: simplified RRT, no full CTL

---

## 6. Transfer functions

Implemented (in `vfx-transfer`):
- [x] sRGB
- [x] Gamma 2.2/2.4
- [x] Rec.709 BT.1886
- [x] PQ (ST 2084)
- [x] HLG (BT.2100)
- [x] ACEScct
- [x] ACEScc
- [x] LogC (ARRI)
- [x] S-Log3
- [x] V-Log

Missing:
- [ ] S-Log2
- [ ] REDLog
- [ ] BMDFilm

---

## 7. CLI commands

Implemented:
- [x] info, convert, resize, crop, diff, composite
- [x] blur, sharpen, color, lut, transform, maketx, grep, batch
- [x] layers, extract-layer, merge-layers
- [x] channel-shuffle, channel-extract
- [x] paste, rotate, warp, aces

Partial:
- [~] maketx: generates mipmaps but only saves base image; no tiled texture output

---

## 8. Caching / Texture / UDIM

Missing:
- [ ] ImageCache
- [ ] TextureSystem
- [ ] UDIM support

---

## 9. Plugin / Extensibility

- [~] Static compile-time formats/ops only
- [ ] Dynamic loading for formats/ops

---

## 10. Priority Roadmap

### P0 (critical) - DONE:
- [x] Multi-layer EXR read/write
- [x] paste op
- [x] rotate arbitrary angle
- [x] warp/distortion
- [x] ACES IDT + RRT/ODT pipeline

### P1 (important) - TODO:
- [ ] Deep EXR data model + read/write
- [ ] Extended IO formats (WebP/AVIF/JP2)
- [ ] More camera log curves (S-Log2, REDLog, BMD)
- [ ] FFT/median/morphology
- [ ] .3DL/.CTF LUT support
- [ ] OCIO config parsing for FixedFunction/ExposureContrast

### P2 (nice-to-have) - TODO:
- [ ] ImageCache/TextureSystem
- [ ] GPU shader generation
- [ ] Video I/O
- [ ] Text rendering
- [ ] UDIM support
- [ ] Tiled EXR / full mipchain maketx

---

## 11. Comparison with OpenImageIO/OpenColorIO

### vs oiiotool (missing):
| Feature | Status |
|---------|--------|
| Deep images | ❌ |
| Tiled textures | ❌ |
| FFT/IFFT | ❌ |
| Morphology (dilate/erode) | ❌ |
| Median filter | ❌ |
| Text overlay | ❌ |
| Noise generation | ❌ |
| Demosaic | ❌ |
| ImageCache | ❌ |
| TextureSystem | ❌ |
| UDIM | ❌ |
| PSD/RAW/Video | ❌ |

### vs OCIO (missing):
| Feature | Status |
|---------|--------|
| GPU processing | ❌ |
| Shader generation | ❌ |
| .3DL/.CTF LUTs | ❌ |
| Full config transform parsing | Partial |
| Dynamic config | ❌ |
