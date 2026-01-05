# vfx-primaries

Color primaries, white points, and RGB-XYZ matrix generation.

## Purpose

Defines the chromaticity coordinates of color space primaries and provides functions to generate RGB↔XYZ conversion matrices.

## Key Concepts

### What are Color Primaries?

Color primaries define the **gamut** (range of colors) a color space can represent. Each primary is specified as CIE xy chromaticity coordinates.

```
         ^  y
       1 │
         │    ● Green
         │   /\
         │  /  \
         │ /    \
         │/______\
         ● Red    ● Blue
         └───────────────► x
                        1
```

### The Primaries Struct

```rust
use vfx_primaries::Primaries;

let custom = Primaries {
    r: (0.64, 0.33),    // Red primary xy
    g: (0.30, 0.60),    // Green primary xy
    b: (0.15, 0.06),    // Blue primary xy
    w: (0.3127, 0.329), // White point xy
    name: "Custom",
};
```

## Standard Color Spaces

### Consumer/Web

| Constant | Gamut Size | White Point | Use Case |
|----------|------------|-------------|----------|
| `SRGB` | Small | D65 | Web, consumer displays |
| `REC709` | Small | D65 | HDTV (same as sRGB) |

### Wide Gamut

| Constant | Gamut Size | White Point | Use Case |
|----------|------------|-------------|----------|
| `DCI_P3` | Medium | DCI | Digital cinema |
| `DISPLAY_P3` | Medium | D65 | Apple displays |
| `REC2020` | Large | D65 | UHDTV, HDR |

### ACES

| Constant | Gamut Size | White Point | Use Case |
|----------|------------|-------------|----------|
| `ACES_AP0` | Very Large | D60 | Archival (ACES 2065-1) |
| `ACES_AP1` | Large | D60 | Working space (ACEScg) |

### Professional

| Constant | Gamut Size | White Point | Use Case |
|----------|------------|-------------|----------|
| `ADOBE_RGB` | Medium | D65 | Print, photography |
| `PROPHOTO_RGB` | Very Large | D50 | Wide gamut photography |

### Camera Native

| Constant | Camera | White Point |
|----------|--------|-------------|
| `ARRI_WIDE_GAMUT_3` | ARRI Alexa | D65 |
| `S_GAMUT3` | Sony Venice | D65 |
| `V_GAMUT` | Panasonic VariCam | D65 |

## White Points

Standard illuminants:

```rust
use vfx_primaries::{D65_XY, D60_XY, D50_XY, DCI_XY};

// D65 - Daylight (~6500K), most common
let d65 = D65_XY;  // (0.31270, 0.32900)

// D60 - ACES standard (~6000K)
let d60 = D60_XY;  // (0.32168, 0.33767)

// D50 - Print/graphics (~5000K)
let d50 = D50_XY;  // (0.34567, 0.35850)

// DCI - Cinema projection
let dci = DCI_XY;  // (0.31400, 0.35100)
```

## Matrix Generation

### RGB to XYZ

```rust
use vfx_primaries::{SRGB, rgb_to_xyz_matrix};
use vfx_math::Vec3;

let m = rgb_to_xyz_matrix(&SRGB);

// Convert RGB to XYZ
let rgb = Vec3::new(1.0, 0.5, 0.25);
let xyz = m * rgb;
```

### XYZ to RGB

```rust
use vfx_primaries::{SRGB, xyz_to_rgb_matrix};

let m = xyz_to_rgb_matrix(&SRGB);
let rgb = m * xyz;
```

### RGB to RGB

Direct conversion between color spaces:

```rust
use vfx_primaries::{SRGB, REC2020, rgb_to_rgb_matrix};

let m = rgb_to_rgb_matrix(&SRGB, &REC2020);
let rec2020_rgb = m * srgb_rgb;
```

**Note**: This doesn't include chromatic adaptation. If white points differ, use `vfx_math::adapt_matrix`.

## Pre-computed Matrices

For performance, common matrices are pre-computed:

```rust
use vfx_primaries::{SRGB_TO_XYZ, XYZ_TO_SRGB};
use vfx_primaries::{ACES_AP0_TO_XYZ, XYZ_TO_ACES_AP0};
use vfx_primaries::{ACES_AP1_TO_XYZ, XYZ_TO_ACES_AP1};
```

## Algorithm

Matrix generation follows the standard method:

1. Convert xy chromaticities to XYZ (with Y=1)
2. Build matrix from primaries as columns
3. Solve for scaling factors so white maps correctly
4. Apply scaling to each column

```rust
// Pseudocode
let r_xyz = xy_to_xyz(primaries.r);
let g_xyz = xy_to_xyz(primaries.g);
let b_xyz = xy_to_xyz(primaries.b);
let w_xyz = xy_to_xyz(primaries.w);

let m = Mat3::from_cols(r_xyz, g_xyz, b_xyz);
let s = m.inverse() * w_xyz;  // Scaling factors

Mat3::from_cols(r_xyz * s.x, g_xyz * s.y, b_xyz * s.z)
```

## Gamut Comparison

Approximate coverage of visible spectrum:

```
sRGB/Rec.709:    ~35%
Display P3:      ~45%
Adobe RGB:       ~50%
Rec.2020:        ~75%
ACES AP1:        ~80%
ACES AP0:        >100% (imaginary colors)
```

## Dependencies

- `vfx-core` - Core types
- `vfx-math` - Matrix operations
- `glam` - SIMD math

## Used By

- `vfx-color` - Color space conversions
- `vfx-ocio` - Transform building
