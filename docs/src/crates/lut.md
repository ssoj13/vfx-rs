# vfx-lut

Look-Up Table types and operations for color pipelines.

## Purpose

LUTs (Look-Up Tables) are used for color grading, display calibration, and color space conversions. This crate provides data structures for 1D and 3D LUTs with file format support.

## LUT Types

### Lut1D - 1D Lookup Table

Per-channel curves, typically used for gamma/transfer functions:

```rust
use vfx_lut::{Lut1D, Interpolation};

// Create identity LUT
let mut lut = Lut1D::identity(1024);

// Create gamma curve
let gamma_lut = Lut1D::gamma(1024, 2.2);

// Apply to value (linear interpolation)
let output = lut.apply(0.5);

// Apply to RGB
let rgb_out = lut.apply_rgb([0.5, 0.3, 0.2]);
```

### Lut3D - 3D Lookup Table

Full RGB cube for complex color transforms with **OCIO-exact** implementation:

```rust
use vfx_lut::{Lut3D, Interpolation};

// Create identity 33x33x33 LUT
let lut = Lut3D::identity(33);

// Apply with trilinear interpolation
let rgb_out = lut.apply([0.5, 0.3, 0.2]);

// Apply with tetrahedral interpolation (more accurate)
let lut = lut.with_interpolation(Interpolation::Tetrahedral);
let rgb_out = lut.apply([0.5, 0.3, 0.2]);
```

#### OCIO Parity

| Property | Status |
|----------|--------|
| Indexing | Blue-major: `idx = B + dim*G + dim²*R` |
| Tetrahedral | All 6 tetrahedra conditions match OCIO |
| Trilinear | B→G→R interpolation order |
| Max diff vs OCIO | Tetrahedral: 1.19e-07, Trilinear: 0.0 |

## File Formats

### .cube (Adobe/Resolve)

Industry-standard format for 1D and 3D LUTs:

```rust
use vfx_lut::{read_cube_1d, read_cube_3d, write_cube_1d, write_cube_3d};
use std::path::Path;

// Read
let lut_1d = read_cube_1d(Path::new("gamma.cube"))?;
let lut_3d = read_cube_3d(Path::new("grade.cube"))?;

// Write
write_cube_1d(Path::new("out.cube"), &lut_1d)?;
write_cube_3d(Path::new("out.cube"), &lut_3d)?;
```

### .clf / .ctf (Academy CLF)

Academy Common LUT Format - XML-based, supports transform chains:

```rust
use vfx_lut::{read_clf, write_clf, ProcessList, ProcessNode};

// Read CLF/CTF
let process_list = read_clf(Path::new("transform.clf"))?;

// Access nodes
for node in &process_list.nodes {
    match node {
        ProcessNode::Matrix { values, .. } => println!("Matrix: {:?}", values),
        ProcessNode::Lut1D { lut, .. } => println!("1D LUT: {} entries", lut.size()),
        ProcessNode::Lut3D { lut, .. } => println!("3D LUT: {}^3", lut.size()),
        ProcessNode::Range(r) => println!("Range: {} - {}", r.min, r.max),
        _ => {}
    }
}

// Write
write_clf(Path::new("out.clf"), &process_list)?;
```

### .spi1d / .spi3d (Sony Pictures Imageworks)

```rust
use vfx_lut::{read_spi1d, read_spi3d, write_spi1d, write_spi3d};

let lut_1d = read_spi1d(Path::new("lut.spi1d"))?;
let lut_3d = read_spi3d(Path::new("lut.spi3d"))?;
```

### .3dl (Autodesk/Lustre)

```rust
use vfx_lut::{read_3dl, write_3dl};

let lut = read_3dl(Path::new("grade.3dl"))?;
write_3dl(Path::new("out.3dl"), &lut)?;
```

### Additional Formats

The crate also supports:

| Format | Extension | Description |
|--------|-----------|-------------|
| CSP | `.csp` | Rising Sun Research |
| HDL | `.hdl` | Houdini LUT |
| Truelight | `.cub` | FilmLight |
| Iridas | `.itx`, `.look` | Iridas/Adobe |
| Pandora | `.mga` | Pandora |
| Nuke | `.vf` | Foundry Nuke |
| SPI Matrix | `.spimtx` | SPI matrix |
| Discreet | `.lut` | Discreet 1D LUT |

## Interpolation Methods

### 1D LUT: Linear

Simple linear interpolation between adjacent entries:

```
index = value * (size - 1)
t = fract(index)
result = lerp(lut[floor(index)], lut[ceil(index)], t)
```

### 3D LUT: Trilinear

Interpolates between 8 corners of the containing cube:

```
Faster, slight color shifts in diagonals
```

### 3D LUT: Tetrahedral

Divides cube into 6 tetrahedra, interpolates within one:

```
More accurate, preserves neutrals better
Slightly slower
```

## Creating LUTs

### Procedural Generation

```rust
use vfx_lut::Lut1D;

// Build data array first
let size = 1024;
let gamma_data: Vec<f32> = (0..size)
    .map(|i| (i as f32 / (size - 1) as f32).powf(2.2))
    .collect();

// Create from data (single-channel, applied to R, G, B)
let lut = Lut1D::from_data(gamma_data, 0.0, 1.0)?;

// Or use gamma convenience constructor
let gamma_lut = Lut1D::gamma(1024, 2.2);

// Inverse gamma
let inv_gamma_lut = Lut1D::gamma(1024, 1.0 / 2.2);
```

### From Color Transform

```rust
use vfx_lut::Lut3D;
use vfx_color::aces::rrt_odt_srgb;

// Bake ACES RRT+ODT into a 3D LUT
let size = 65;
let mut data: Vec<[f32; 3]> = Vec::with_capacity(size * size * size);

// LUT3D uses blue-major indexing: B varies fastest
for r in 0..size {
    for g in 0..size {
        for b in 0..size {
            let rgb = [r as f32 / 64.0, g as f32 / 64.0, b as f32 / 64.0];
            let (out_r, out_g, out_b) = rrt_odt_srgb(rgb[0], rgb[1], rgb[2]);
            data.push([out_r, out_g, out_b]);
        }
    }
}

let lut = Lut3D::from_data(data, size)?;
```

## LUT Resolution

Common sizes:

| Size | Use Case | Memory |
|------|----------|--------|
| 17³ | Preview, fast | 19 KB |
| 33³ | Standard grading | 143 KB |
| 65³ | High quality | 1.1 MB |

Higher resolution = more accurate, but diminishing returns past 65³.

## Performance Tips

1. **Pre-bake transforms** - Convert complex pipelines to 3D LUT for real-time use
2. **Use tetrahedral** - For final output quality
3. **Use trilinear** - For interactive preview
4. **Cache LUTs** - Parse once, reuse

## Dependencies

- `vfx-core` - Core types
- `thiserror` - Error handling
- `quick-xml` - CLF/CTF parsing

## Used By

- `vfx-color` - Color transformations
- `vfx-ocio` - OCIO processing
- `vfx-view` - Display LUTs
