# VFX-RS Dataflow & Architecture Guide

## Purpose

This document provides visual dataflow diagrams for understanding the vfx-rs architecture. Use this as a reference for navigating the codebase.

## 1. Crate Dependency Graph

```
                    +------------+
                    | vfx-core   |
                    | (types)    |
                    +-----+------+
                          |
      +--------+----------+----------+--------+
      |        |          |          |        |
      v        v          v          v        v
+--------+ +--------+ +--------+ +--------+ +--------+
|vfx-math| |vfx-xfer| |vfx-prim| |vfx-lut | |vfx-icc |
|(matrix)| |(EOTF)  | |(gamut) | |(1D/3D) | |(ICC)   |
+---+----+ +---+----+ +---+----+ +---+----+ +--------+
    |          |          |          |
    +----------+----------+----------+
                    |
                    v
              +-----------+
              | vfx-color |
              |(transform)|
              +-----+-----+
                    |
        +-----------+-----------+
        |           |           |
        v           v           v
   +--------+  +--------+  +----------+
   | vfx-io |  |vfx-ops |  | vfx-ocio |
   |(file)  |  |(ops)   |  |(compat)  |
   +---+----+  +---+----+  +----------+
       |           |
       +-----------+
             |
             v
       +-----------+
       |vfx-compute|
       | (GPU/CPU) |
       +-----+-----+
             |
     +-------+-------+
     |               |
     v               v
+----------+   +----------+
| vfx-cli  |   |vfx-rs-py |
|(cmdline) |   | (Python) |
+----------+   +----------+
```

## 2. Image Processing Pipeline

### 2.1 Read -> Process -> Write

```
[File on Disk]
      |
      v
+------------------+
| Format Detection |
| (extension/magic)|
+--------+---------+
         |
   +-----+-----+-----+-----+-----+-----+
   |     |     |     |     |     |     |
   v     v     v     v     v     v     v
 EXR   PNG  JPEG  TIFF  DPX   HDR  HEIF
   |     |     |     |     |     |     |
   +-----+-----+-----+-----+-----+-----+
         |
         v
+------------------+
| ImageData        |
| {width, height,  |
|  channels, data} |
+--------+---------+
         |
         v
+------------------+
| Processor        |
| (CPU or GPU)     |
+--------+---------+
         |
   +-----+-----+-----+
   |           |     |
   v           v     v
exposure  saturation CDL
   |           |     |
   +-----+-----+-----+
         |
         v
+------------------+
| Output Format    |
+--------+---------+
         |
         v
[File on Disk]
```

### 2.2 Color Transform Pipeline

```
[Input RGB (unknown space)]
         |
         v
+-------------------+
| Identify Source   |
| (EXIF/ICC/OCIO)   |
+--------+----------+
         |
         v
+-------------------+
| Linearization     |
| (EOTF inverse)    |
| sRGB: x^2.4       |
| PQ: ST.2084^-1    |
+--------+----------+
         |
         v
+-------------------+
| RGB -> XYZ        |
| (3x3 matrix)      |
+--------+----------+
         |
         v
+-------------------+
| Chromatic Adapt   |
| (Bradford/CAT02)  |
| D65 <-> DCI-P3    |
+--------+----------+
         |
         v
+-------------------+
| XYZ -> RGB        |
| (target primaries)|
+--------+----------+
         |
         v
+-------------------+
| Apply Grading     |
| (CDL/LUT/etc)     |
+--------+----------+
         |
         v
+-------------------+
| Output Transfer   |
| (OETF)            |
+--------+----------+
         |
         v
[Output RGB (target space)]
```

## 3. Compute Backend Selection

```
           +------------------+
           | Processor::auto()|
           +--------+---------+
                    |
                    v
           +------------------+
           | Check GPU        |
           | availability     |
           +--------+---------+
                    |
            +-------+-------+
            |               |
         [GPU OK]       [No GPU]
            |               |
            v               v
     +-----------+   +-----------+
     |   wgpu    |   |    CPU    |
     | (compute) |   |  (rayon)  |
     +-----+-----+   +-----+-----+
           |               |
           +-------+-------+
                   |
                   v
           +------------------+
           | Tile Scheduler   |
           | (64-512px tiles) |
           +--------+---------+
                    |
            +-------+-------+
            |       |       |
            v       v       v
         Tile0   Tile1   TileN
            |       |       |
            +-------+-------+
                    |
                    v
           +------------------+
           | Merge Results    |
           +------------------+
```

## 4. Format-Specific I/O

### 4.1 EXR Pipeline

```
[.exr file]
     |
     v
+------------------+
| OpenEXR crate    |
| (exr = "1.7")    |
+--------+---------+
     |
     +-- Read header (channels, compression)
     |
     +-- Decode tiles/scanlines
     |
     +-- Convert to f32 (or keep f16)
     |
     v
+------------------+
| ImageData        |
| format: F32/F16  |
| channels: 3-4    |
+------------------+
```

### 4.2 DPX Pipeline

```
[.dpx file]
     |
     v
+------------------+
| Parse Headers    |
| (file/image/     |
|  orient/film/tv) |
+--------+---------+
     |
     +-- BitDepth: 8/10/12/16
     |
     +-- Packing: Filled/Packed
     |
     +-- Encoding: Linear/Log
     |
     v
+------------------+
| Unpack bits      |
| (handle endian)  |
+--------+---------+
     |
     v
+------------------+
| Normalize to f32 |
| 10-bit: /1023.0  |
+------------------+
```

## 5. Python API (vfx-rs-py)

### 5.1 Module Structure

```
vfx_rs (PyModule)
   |
   +-- Image (PyClass)
   |      +-- width, height, channels, format
   |      +-- numpy() -> np.ndarray
   |      +-- copy() -> Image
   |
   +-- Processor (PyClass)
   |      +-- backend (cpu/wgpu)
   |      +-- exposure(img, stops)
   |      +-- saturation(img, factor)
   |      +-- contrast(img, factor)
   |      +-- cdl(img, slope, offset, power, sat)
   |
   +-- read(path) -> Image
   +-- write(path, image)
   |
   +-- io (SubModule)
   |      +-- read_exr, write_exr
   |      +-- read_png, write_png
   |      +-- read_jpeg, write_jpeg
   |      +-- read_tiff, write_tiff
   |      +-- read_dpx, write_dpx
   |      +-- read_hdr, write_hdr
   |
   +-- lut (SubModule)
          +-- Lut1D, Lut3D
          +-- ProcessList (CLF)
          +-- read_cube, read_cube_1d, read_cube_3d
          +-- read_clf
```

### 5.2 Typical Usage

```python
import vfx_rs
import numpy as np

# Read
img = vfx_rs.read("input.exr")

# Process
proc = vfx_rs.Processor()  # auto GPU/CPU
proc.exposure(img, 0.5)    # +0.5 stops
proc.saturation(img, 1.2)  # boost saturation
proc.cdl(img, 
    slope=[1.1, 1.0, 0.9],   # warm
    offset=[0.01, 0.0, -0.01],
    power=[1.0, 1.0, 1.0],
    saturation=1.0
)

# Numpy access
arr = img.numpy()  # (H, W, C) float32
arr = arr * 2.0    # direct manipulation
img = vfx_rs.Image(arr)

# Write
vfx_rs.write("output.exr", img)
```

## 6. Key Type Locations

| Type | Location | Purpose |
|------|----------|---------|
| `ImageData` | `vfx-io/src/lib.rs` | Runtime image buffer |
| `Image<C,T,N>` | `vfx-core/src/image.rs` | Compile-time typed image |
| `ImageSpec` | `vfx-core/src/spec.rs` | Image metadata spec |
| `PixelFormat` | `vfx-io/src/lib.rs` | Runtime pixel format |
| `ChannelFormat` | `vfx-core/src/spec.rs` | Compile-time format |
| `Processor` | `vfx-compute/src/processor.rs` | GPU/CPU processor |
| `Pipeline` | `vfx-compute/src/pipeline.rs` | Multi-stage pipeline |
| `ColorSpace` | `vfx-ocio/src/colorspace.rs` | OCIO-compatible space |

## 7. Critical Files for Refactoring

### Type Unification Targets

1. **BitDepth** - unify 6 definitions into `vfx-core/src/format.rs`
   - `vfx-ocio/colorspace.rs:133`
   - `vfx-ocio/processor.rs:581`
   - `vfx-lut/clf.rs:72`
   - `vfx-io/dpx.rs:82`
   - `vfx-io/png.rs:68`
   - `vfx-io/tiff.rs:70`

2. **AttrValue** - canonical at `vfx-io/attrs/value.rs:58`
   - Remove `vfx-io/metadata.rs:9`
   - Remove `vfx-core/spec.rs:169`

3. **PixelFormat/ChannelFormat** - merge in `vfx-core`
   - `vfx-io/lib.rs:482` (PixelFormat)
   - `vfx-core/spec.rs:89` (ChannelFormat)

## 8. Testing Strategy

```
cargo test                    # All unit tests
cargo test -p vfx-io          # Single crate
cargo test --features all     # All features
python test.py                # Visual quality tests
cargo bench                   # Performance benchmarks
```

Output directory: `C:\projects\projects.rust\_vfx-rs\test\out\`
