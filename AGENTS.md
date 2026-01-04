# AGENTS.md

This document captures the current dataflow and codepaths in the VFX-RS workspace.
It is intended as a durable reference for future work.

## High-Level Dataflow

```
Files on disk
   │
   ▼
vfx-io::read  ──► ImageData (format-agnostic container)
   │                  │
   │                  └─► LayeredImage (multi-layer EXR)
   │                           │
   ├─ metadata (colorspace, gamma)
   └─ pixel data (U8/U16/F16/F32)
   │
   ▼
Color operations
   ├─ vfx-color (Pipeline + ColorProcessor + ACES)
   ├─ vfx-ocio (Config + Processor)
   └─ vfx-ops (resize/composite/filter/warp/transform)
   │
   ▼
vfx-io::write  ──► Output files
```

## OCIO Processing Flow

```
Config::from_file / from_yaml_str
   │
   ├─ parse roles, colorspaces, displays, looks, view_transforms
   ├─ parse file_rules (glob/regex/default)
   └─ build internal structures
   │
   ▼
Config::processor(src, dst)
   │
   ├─ src colorspace → to_reference transform
   ├─ dst colorspace → from_reference transform
   └─ group transforms → Processor::from_transform
   │
   ▼
Processor::apply_rgb / apply_rgba
   │
   └─ Op list (Matrix/LUT/CDL/Range/Transfer/FixedFunction/ExposureContrast/...)
```

## Display Pipeline

```
Config::display_processor(src, display, view)
   │
   ├─ resolve display + view
   ├─ apply view transform (OCIO v2)
   ├─ apply view look(s)
   └─ convert to view colorspace
   │
   ▼
Processor::apply_rgb / apply_rgba
```

## CLI Commands

```
vfx (binary)
   │
   ├─ info           -> vfx-io::read -> metadata dump
   ├─ convert        -> vfx-io::read -> vfx-io::write
   ├─ resize         -> vfx-ops::resize -> vfx-io::write
   ├─ crop           -> vfx-ops::transform::crop -> vfx-io::write
   ├─ blur/sharpen   -> vfx-ops::filter -> vfx-io::write
   ├─ composite      -> vfx-ops::composite -> vfx-io::write
   ├─ transform      -> vfx-ops::transform (flip/rotate90) -> vfx-io::write
   ├─ color          -> vfx-ocio / vfx-color -> vfx-io::write
   ├─ lut            -> vfx-lut -> vfx-io::write
   ├─ diff           -> compare images
   ├─ grep           -> search metadata
   ├─ batch          -> bulk processing
   ├─ maketx         -> texture generation
   │
   ├─ layers         -> list EXR layers/channels
   ├─ extract-layer  -> extract single layer from multi-layer EXR
   ├─ merge-layers   -> combine files into multi-layer EXR
   │
   ├─ channel-shuffle -> reorder channels (BGR, RRR, RGB1)
   ├─ channel-extract -> extract specific channels
   │
   ├─ paste          -> overlay image at position
   ├─ rotate         -> arbitrary angle rotation
   ├─ warp           -> distortion effects (barrel, twist, ripple, etc.)
   └─ aces           -> ACES IDT/RRT/ODT transforms
```

## Crate Dependency Map

```
                        vfx-cli
                           │
        ┌──────────────────┼───────────────────┐
        ▼                  ▼                   ▼
     vfx-io            vfx-ops             vfx-ocio
        │                  │                   │
        │     ┌────────────┴─────────┐         │
        │     ▼                      ▼         │
        │  vfx-color              vfx-lut      │
        │     │                      │         │
        │     ├─── vfx-transfer ─────┤         │
        │     ├─── vfx-primaries     │         │
        │     └─── vfx-math ─────────┘         │
        │              │                       │
        └──────────────┴───────────────────────┘
                       │
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

### vfx-ops
- `layer_ops`: operations on `ImageLayer` (resize, blur, crop, sharpen)
- `warp`: distortion effects (barrel, pincushion, fisheye, twist, wave, spherize, ripple)
- `transform`: geometric ops (crop, flip, rotate, paste, pad, tile)

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
| EXR | ✓ | ✓ | Multi-layer, F16/F32, ZIP/PIZ/ZIPS compression |
| PNG | ✓ | ✓ | 8/16-bit |
| JPEG | ✓ | ✓ | Quality control |
| TIFF | ✓ | ✓ | 8/16/32-bit |
| HDR | ✓ | ✓ | RGBE format |
| DPX | ✓ | ✓ | 10-bit log |

## Transfer Functions (vfx-transfer)

- sRGB, Gamma 2.2/2.4, Rec.709 BT.1886
- PQ (ST 2084), HLG (BT.2100)
- ACEScc, ACEScct
- ARRI LogC, Sony S-Log3, Panasonic V-Log

## Color Primaries (vfx-primaries)

- sRGB/Rec.709, Rec.2020, DCI-P3, Display P3
- ACES AP0, ACES AP1 (ACEScg)
- Adobe RGB

## Known Limitations / Future Work

- Deep EXR not supported
- Tiled EXR not supported (scanline only)
- No GPU processing / shader generation
- No ImageCache / TextureSystem
- No UDIM support
- maketx generates base level only (no full mipchain)
- Limited LUT formats (.3DL, .CTF not supported)
- No video I/O
