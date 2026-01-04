# AGENTS.md

This document captures the current dataflow and codepaths in the VFX-RS workspace.
It is intended as a durable reference for future work.

## High-Level Dataflow

```
Files on disk
   |
   v
vfx-io::read  --> ImageData (format-agnostic container)
   |                  |
   |                  +-> LayeredImage (multi-layer EXR)
   |                           |
   +- metadata (colorspace, gamma)
   +- pixel data (U8/U16/F16/F32)
   |
   v
Color operations
   +- vfx-color (Pipeline + ColorProcessor + ACES)
   +- vfx-ocio (Config + Processor)
   +- vfx-ops (resize/composite/filter/warp/transform)
   +- vfx-compute (GPU-accelerated via wgpu)
   |
   v
vfx-io::write  --> Output files
```

## GPU Compute Architecture (vfx-compute)

```
Processor (unified API)
    +-- Backend selection (Auto/Cpu/Wgpu)
            |
            +-- CpuBackend (rayon parallelization)
            +-- WgpuBackend (Vulkan/Metal/DX12 compute shaders)
                    |
                    +-- WGSL shaders for each operation
                    +-- Automatic tiling for large images
```

### ProcessingBackend Trait

All backends implement:
- **Color ops**: apply_matrix, apply_cdl, apply_lut1d, apply_lut3d
- **Image ops**: resize, blur
- **Composite ops**: composite_over (Porter-Duff), blend (9 modes)
- **Transform ops**: crop, flip_h, flip_v, rotate_90

### BlendMode Enum

```rust
Normal, Multiply, Screen, Add, Subtract, Overlay, SoftLight, HardLight, Difference
```

### vfx-ops GPU Integration

Operations in vfx-ops automatically use GPU when available:
- `resize.rs` -> vfx-compute resize
- `composite.rs` -> vfx-compute composite_over, blend
- `transform.rs` -> vfx-compute crop, flip_h, flip_v, rotate_90

Fallback to CPU if GPU unavailable.

## OCIO Processing Flow

```
Config::from_file / from_yaml_str
   |
   +- parse roles, colorspaces, displays, looks, view_transforms
   +- parse file_rules (glob/regex/default)
   +- build internal structures
   |
   v
Config::processor(src, dst)
   |
   +- src colorspace -> to_reference transform
   +- dst colorspace -> from_reference transform
   +- group transforms -> Processor::from_transform
   |
   v
Processor::apply_rgb / apply_rgba
   |
   +- Op list (Matrix/LUT/CDL/Range/Transfer/FixedFunction/ExposureContrast/...)
```

## Display Pipeline

```
Config::display_processor(src, display, view)
   |
   +- resolve display + view
   +- apply view transform (OCIO v2)
   +- apply view look(s)
   +- convert to view colorspace
   |
   v
Processor::apply_rgb / apply_rgba
```

## CLI Commands

```
vfx (binary)
   |
   +- info           -> vfx-io::read -> metadata dump
   +- convert        -> vfx-io::read -> vfx-io::write
   +- resize         -> vfx-ops::resize (GPU) -> vfx-io::write
   +- crop           -> vfx-ops::transform::crop (GPU) -> vfx-io::write
   +- blur/sharpen   -> vfx-ops::filter -> vfx-io::write
   +- composite      -> vfx-ops::composite (GPU) -> vfx-io::write
   +- transform      -> vfx-ops::transform (flip/rotate90, GPU) -> vfx-io::write
   +- color          -> vfx-ocio / vfx-color -> vfx-io::write
   +- lut            -> vfx-lut -> vfx-io::write
   +- diff           -> compare images
   +- grep           -> search metadata
   +- batch          -> bulk processing
   +- maketx         -> texture generation
   |
   +- layers         -> list EXR layers/channels
   +- extract-layer  -> extract single layer from multi-layer EXR
   +- merge-layers   -> combine files into multi-layer EXR
   |
   +- channel-shuffle -> reorder channels (BGR, RRR, RGB1)
   +- channel-extract -> extract specific channels
   |
   +- paste          -> overlay image at position
   +- rotate         -> arbitrary angle rotation
   +- warp           -> distortion effects (barrel, twist, ripple, etc.)
   +- aces           -> ACES IDT/RRT/ODT transforms
```

## Crate Dependency Map

```
                        vfx-cli
                           |
        +------------------+-------------------+
        v                  v                   v
     vfx-io            vfx-ops             vfx-ocio
        |                  |                   |
        |     +------------+                   |
        |     v                                |
        |  vfx-compute  <----------------------+
        |     | (GPU/CPU backends)             |
        |     |                                |
        |     +------------+---------+         |
        |     v            v         v         |
        |  vfx-color    vfx-lut   (wgpu)       |
        |     |            |                   |
        |     +-- vfx-transfer --+             |
        |     +-- vfx-primaries  |             |
        |     +-- vfx-math ------+             |
        |              |                       |
        +--------------+-----------------------+
                       |
                   vfx-core
```

## Key Data Structures

### vfx-core
- `Image<C, T, N>`: typed image buffer with compile-time color space
- `ColorSpace`, `TransferFunction`, `Illuminant` enums

### vfx-io
- `ImageData`: format-agnostic container for I/O (single layer)
- `LayeredImage`: multi-layer container for EXR
- `ImageLayer`: single layer with named channels
- `ImageChannel`: individual channel with samples + metadata

### vfx-compute
- `Processor`: unified compute API (color + image + composite + transform)
- `ComputeImage`: GPU-friendly image container (f32 data)
- `Backend`: Auto/Cpu/Wgpu selection
- `ProcessingBackend` trait: interface for GPU/CPU implementations
- `BlendMode`: compositing blend modes
- `GpuLimits`: tiling parameters for large images

### vfx-ops
- `layer_ops`: operations on `ImageLayer` (resize, blur, crop, sharpen)
- `warp`: distortion effects (barrel, pincushion, fisheye, twist, wave, spherize, ripple)
- `transform`: geometric ops (crop, flip, rotate, paste, pad, tile) - GPU accelerated
- `composite`: over, blend operations - GPU accelerated
- `resize`: bilinear/bicubic/lanczos - GPU accelerated

### vfx-color
- `Pipeline`: explicit sequence of per-RGB ops
- `ColorProcessor`: applies pipeline to images
- `aces`: ACES RRT/ODT transforms with filmic tonemap

### vfx-ocio
- `Config`: parsed OCIO configuration (colorspaces, roles, displays, file_rules)
- `Processor`: compiled transform op list
- `builtin`: ACES 1.3 builtins

### vfx-lut
- `Lut1D`, `Lut3D`: lookup tables with domain support
- Formats: .cube, .clf, .spi1d, .spi3d, .cdl

## Supported Image Formats

| Format | Read | Write | Notes |
|--------|------|-------|-------|
| EXR | Y | Y | Multi-layer, F16/F32, ZIP/PIZ/ZIPS compression |
| PNG | Y | Y | 8/16-bit |
| JPEG | Y | Y | Quality control |
| TIFF | Y | Y | 8/16/32-bit |
| HDR | Y | Y | RGBE format |
| DPX | Y | Y | 10-bit log |
| HEIF/HEIC | Y | Y | HDR PQ/HLG, 8/10-bit, NCLX profiles (requires `heif` feature) |

## HEIF/HEIC Support (vfx-io)

Requires `heif` feature and system libheif >= 1.17.

### Setup

```bash
# Windows (vcpkg)
vcpkg install libheif:x64-windows
set VCPKG_ROOT=C:\vcpkg
set VCPKGRS_TRIPLET=x64-windows
set VCPKGRS_DYNAMIC=1

# Linux
apt install libheif-dev

# macOS
brew install libheif
```

### HDR Metadata

```rust
HdrMetadata {
    transfer: TransferCharacteristics,  // Pq, Hlg, Srgb, Linear, Bt709...
    primaries: ColorPrimaries,          // Bt2020, DisplayP3, DciP3, Bt709...
    matrix: MatrixCoefficients,         // Identity, Bt709, Bt2020Ncl...
    full_range: bool,
    bit_depth: u8,                      // 8, 10, 12
}
```

### Usage

```rust
use vfx_io::heif::{read_heif, write_heif, HdrMetadata, TransferCharacteristics, ColorPrimaries};

// Read with HDR metadata
let (image, hdr_meta) = read_heif("photo.heic")?;
if let Some(meta) = &hdr_meta {
    if meta.is_hdr() {
        println!("HDR: {:?} transfer, {:?} primaries", meta.transfer, meta.primaries);
    }
}

// Write with HDR metadata
let hdr = HdrMetadata {
    transfer: TransferCharacteristics::Pq,
    primaries: ColorPrimaries::Bt2020,
    bit_depth: 10,
    ..Default::default()
};
write_heif("output.heif", &image, Some(&hdr))?;
```

### Limitations

- libheif-rs only exposes `set_color_primaries()` for NCLX profiles
- Transfer characteristics and matrix coefficients use library defaults when writing
- Gain map extraction not yet implemented

## Transfer Functions (vfx-transfer)

- sRGB, Gamma 2.2/2.4, Rec.709 BT.1886
- PQ (ST 2084), HLG (BT.2100)
- ACEScc, ACEScct
- ARRI LogC, Sony S-Log3, Panasonic V-Log

## Color Primaries (vfx-primaries)

- sRGB/Rec.709, Rec.2020, DCI-P3, Display P3
- ACES AP0, ACES AP1 (ACEScg)
- Adobe RGB

## GPU Backend Details (vfx-compute)

### Supported Operations

| Operation | CPU | wgpu | Notes |
|-----------|-----|------|-------|
| Color matrix 4x4 | Y | Y | Exposure, contrast, etc. |
| CDL (slope/offset/power/sat) | Y | Y | ASC-CDL |
| 1D LUT | Y | Y | Per-channel |
| 3D LUT | Y | Y | Trilinear interpolation |
| Resize | Y | Y | Nearest/Bilinear/Bicubic/Lanczos |
| Gaussian blur | Y | Y | Separable, any radius |
| Composite Over | Y | Y | Porter-Duff |
| Blend (9 modes) | Y | Y | With opacity |
| Crop | Y | Y | Region extraction |
| Flip H/V | Y | Y | Mirror |
| Rotate 90 | Y | Y | 90/180/270 degrees |

### WGSL Shaders

Located in `vfx-compute/src/shaders/mod.rs`:
- COLOR_MATRIX, CDL, LUT1D, LUT3D
- RESIZE, BLUR_H, BLUR_V
- COMPOSITE_OVER, BLEND
- CROP, FLIP_H, FLIP_V, ROTATE_90

## Known Limitations / Future Work

### Limitations
- Deep EXR not supported
- Tiled EXR not supported (scanline only)
- No ImageCache / TextureSystem
- No UDIM support
- maketx generates base level only (no full mipchain)
- Limited LUT formats (.3DL, .CTF not supported)
- No video I/O

### Planned
- HEIF gain map extraction (iPhone HDR)
- More GPU operations (arbitrary rotate, perspective warp)
- Async GPU pipeline for batch processing
- OpenImageIO-style texture system
