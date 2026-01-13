# vfx-color

Unified color transformation API for VFX pipelines.

## Purpose

Combines all color-related functionality into a single, composable API. This is the main entry point for color management in vfx-rs.

## Architecture

```
            vfx-color
                │
    ┌───────────┼───────────┐
    │           │           │
vfx-transfer  vfx-primaries vfx-lut
    │           │           │
    └─────┬─────┴───────────┘
          │
      vfx-math
          │
      vfx-core
```

## Quick Start

```rust
use vfx_color::prelude::*;

// Transfer functions
let linear = srgb::eotf(0.5);
let encoded = srgb::oetf(linear);

// Color space matrices
let m = rgb_to_xyz_matrix(&SRGB);
let xyz = m * Vec3::new(1.0, 0.5, 0.25);

// Chromatic adaptation
let adapt = adapt_matrix(D65, D60, &BRADFORD);
```

## ColorProcessor

The main processing API:

```rust
use vfx_color::ColorProcessor;

let mut proc = ColorProcessor::new();

// Single pixel
let rgb = [0.5, 0.3, 0.2];
let result = proc.srgb_to_linear(rgb);

// Image buffer
let mut data = vec![[0.5f32, 0.3, 0.2]; 1000];
proc.apply_srgb_to_linear(&mut data);
```

## Pipeline

Build transform chains:

```rust
use vfx_color::{Pipeline, TransformOp};
use vfx_color::prelude::*;

let pipeline = Pipeline::new()
    // Step 1: Decode sRGB
    .transfer_in(srgb::eotf)
    // Step 2: sRGB to XYZ
    .matrix(rgb_to_xyz_matrix(&SRGB))
    // Step 3: XYZ to Rec.2020
    .matrix(xyz_to_rgb_matrix(&REC2020))
    // Step 4: Encode PQ
    .transfer_out(pq::oetf);

// Apply to pixel
let result = pipeline.apply([0.5, 0.3, 0.2]);

// Apply to buffer
pipeline.apply_buffer(&mut data);
```

## ACES Workflow

Full ACES support via the `aces` module:

```rust
use vfx_color::aces::{
    apply_rrt_odt_srgb,
    apply_inverse_odt_srgb,
    rrt,
    RrtParams,
};

// Full RRT+ODT: ACEScg → sRGB display
let display = apply_rrt_odt_srgb(&linear_data, 3);

// Inverse ODT: sRGB → ACEScg
let linear = apply_inverse_odt_srgb(&srgb_data, 3);

// RRT only with custom params
let params = RrtParams::aces_high_contrast();
let (r, g, b) = rrt(0.18, 0.18, 0.18, &params);
```

### ACES Transforms

| Transform | Input | Output |
|-----------|-------|--------|
| IDT | sRGB gamma | ACEScg linear |
| RRT | ACEScg linear | Tonemapped linear |
| ODT | Tonemapped | Display (sRGB) |
| RRT+ODT | ACEScg linear | Display (sRGB) |

## CDL (Color Decision List)

ASC CDL standard (slope/offset/power/saturation) with **OCIO-exact** implementation:

```rust
use vfx_color::cdl::{Cdl, apply_cdl};

let cdl = Cdl {
    slope: [1.1, 1.0, 0.9],
    offset: [0.0, 0.0, 0.02],
    power: [1.0, 1.0, 1.0],
    saturation: 1.1,
};

// Apply: out = (in * slope + offset) ^ power, then saturation
apply_cdl(&mut data, channels, &cdl);
```

### OCIO Parity

| Property | Status |
|----------|--------|
| Algorithm | ASC CDL v1.2 (Slope → Offset → Clamp → Power → Saturation) |
| Power function | `fast_pow` with OCIO-identical Chebyshev polynomials |
| Luma weights | Rec.709 (0.2126, 0.7152, 0.0722) |
| Max diff vs OCIO | ~3e-7 (8-22 ULP) |

See [OCIO Parity Audit](../OCIO_PARITY_AUDIT.md) for details.

## Color Space Conversion

### Direct Matrix

```rust
use vfx_color::convert::RgbConvert;

let converter = RgbConvert::new(&SRGB, &REC2020);
let rec2020 = converter.convert([0.5, 0.3, 0.2]);
```

### With Chromatic Adaptation

When white points differ:

```rust
use vfx_color::convert::RgbConvert;

// sRGB (D65) → ACES (D60) with Bradford adaptation
let converter = RgbConvert::with_adaptation(&SRGB, &ACES_AP1, &BRADFORD);
let aces = converter.convert([0.5, 0.3, 0.2]);
```

## Re-exported Modules

For convenience, sub-crates are re-exported:

```rust
use vfx_color::transfer;    // vfx-transfer
use vfx_color::primaries;   // vfx-primaries
use vfx_color::lut;         // vfx-lut
use vfx_color::math;        // vfx-math
```

## Prelude

Common imports in one line:

```rust
use vfx_color::prelude::*;

// Includes:
// - ColorProcessor, Pipeline, TransformOp
// - Transfer functions: srgb, gamma, pq, hlg
// - Primaries: SRGB, REC709, REC2020, ACES_AP0, ACES_AP1, etc.
// - Matrix functions: rgb_to_xyz_matrix, xyz_to_rgb_matrix
// - LUT types: Lut1D, Lut3D, Interpolation
// - Math: Vec3, Mat3, adapt_matrix
// - White points: D65, D60, D50
// - Adaptation: BRADFORD, CAT02, VON_KRIES
```

## Common Workflows

### HDR to SDR

```rust
let pipeline = Pipeline::new()
    .transfer_in(pq::eotf)           // PQ → linear (nits)
    .scale(1.0 / 10000.0)            // Normalize to 0-1
    .tonemap_reinhard()              // Simple tonemap
    .matrix(rgb_to_rgb_matrix(&REC2020, &SRGB))
    .transfer_out(srgb::oetf);
```

### Camera Log to Display

```rust
let pipeline = Pipeline::new()
    .transfer_in(log_c::decode)      // LogC → linear
    .matrix(rgb_to_rgb_matrix(&ARRI_WIDE_GAMUT_3, &REC709))
    .lut_3d(&display_lut)            // Creative grade
    .transfer_out(srgb::oetf);
```

### sRGB to ACEScg

```rust
let pipeline = Pipeline::new()
    .transfer_in(srgb::eotf)
    .matrix(rgb_to_xyz_matrix(&SRGB))
    .adapt(D65, D60, &BRADFORD)
    .matrix(xyz_to_rgb_matrix(&ACES_AP1));
```

## Feature Flags

```toml
[dependencies]
vfx-color = { version = "0.1", features = ["gpu"] }
```

| Feature | Description |
|---------|-------------|
| `gpu` | GPU-accelerated transforms via vfx-compute |

## Dependencies

- `vfx-core` - Core types
- `vfx-math` - Matrix/vector math
- `vfx-transfer` - Transfer functions
- `vfx-primaries` - Color primaries
- `vfx-lut` - LUT types
- `vfx-compute` - GPU acceleration (optional)
