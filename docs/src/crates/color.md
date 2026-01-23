# vfx-color

Unified color transformation API for VFX pipelines.

## Purpose

Combines all color-related functionality into a single, composable API. This is the main entry point for color management in vfx-rs.

## Architecture

```
            vfx-color
                |
    +-----------+-----------+
    |           |           |
vfx-transfer  vfx-primaries vfx-lut
    |           |           |
    +-----+-----+-----------+
          |
      vfx-math
          |
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

// Apply to single pixel
let result = pipeline.apply([0.5, 0.3, 0.2]);

// Apply to buffer (iterate manually)
for pixel in data.iter_mut() {
    *pixel = pipeline.apply(*pixel);
}
```

### Pipeline Methods

```rust
use vfx_color::Pipeline;

let pipeline = Pipeline::new()
    .transfer_in(f)          // Input transfer function (decode)
    .transfer_out(f)         // Output transfer function (encode)
    .matrix(m)               // 3x3 matrix transform
    .lut1d(lut)              // 1D LUT
    .lut3d(lut)              // 3D LUT (NOTE: lut3d, not lut_3d)
    .scale([sx, sy, sz])     // Per-channel scale
    .offset([ox, oy, oz])    // Per-channel offset
    .clamp(min, max)         // Clamp to range
    .clamp_01();             // Clamp to [0, 1]
```

## ColorProcessor

High-level processing with optional GPU acceleration:

```rust
use vfx_color::{ColorProcessor, Pipeline};
use vfx_color::prelude::*;

// Create processor
let mut proc = ColorProcessor::new();

// Or with GPU
let mut proc = ColorProcessor::with_gpu();

// Build a pipeline
let pipeline = Pipeline::new()
    .transfer_in(srgb::eotf)
    .matrix(rgb_to_xyz_matrix(&SRGB))
    .matrix(xyz_to_rgb_matrix(&REC2020))
    .transfer_out(pq::oetf);

// Apply to single pixel
let result = proc.apply(&pipeline, [0.5, 0.3, 0.2]);

// Apply to batch (returns new Vec)
let results = proc.apply_batch(&pipeline, &pixels);

// Apply in-place
proc.apply_in_place(&pipeline, &mut pixels);
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

// Full RRT+ODT: ACEScg -> sRGB display
let display = apply_rrt_odt_srgb(&linear_data, 3);

// Inverse ODT: sRGB -> ACEScg
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
use vfx_color::cdl::Cdl;

let cdl = Cdl::new()
    .with_slope([1.1, 1.0, 0.9])
    .with_offset([0.0, 0.0, 0.02])
    .with_power([1.0, 1.0, 1.0])
    .with_saturation(1.1);

// Apply to single pixel
let mut rgb = [0.5, 0.3, 0.2];
cdl.apply(&mut rgb);

// Apply to buffer
cdl.apply_buffer(&mut pixel_buffer);
```

### OCIO Parity

| Property | Status |
|----------|--------|
| Algorithm | ASC CDL v1.2 (Slope -> Offset -> Clamp -> Power -> Saturation) |
| Power function | `fast_pow` with OCIO-identical Chebyshev polynomials |
| Luma weights | Rec.709 (0.2126, 0.7152, 0.0722) |
| Max diff vs OCIO | ~3e-7 (8-22 ULP) |

See [OCIO Parity Audit](../OCIO_PARITY_AUDIT.md) for details.

## Color Space Conversion

### Direct Matrix

```rust
use vfx_color::convert::convert_rgb;
use vfx_color::prelude::*;

// Convert RGB between primaries
let rec2020 = convert_rgb([0.5, 0.3, 0.2], &SRGB, &REC2020);
```

### With Chromatic Adaptation

When white points differ:

```rust
use vfx_color::convert::convert_rgb;
use vfx_color::prelude::*;

// sRGB (D65) -> ACES (D60) with Bradford adaptation
let aces = convert_rgb_adapted(
    [0.5, 0.3, 0.2],
    &SRGB, &ACES_AP1,
    D65, D60,
    &BRADFORD
);
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
// - Pipeline, TransformOp
// - Transfer functions: srgb, gamma, pq, hlg
// - Primaries: SRGB, REC709, REC2020, ACES_AP0, ACES_AP1, etc.
// - Matrix functions: rgb_to_xyz_matrix, xyz_to_rgb_matrix
// - LUT types: Lut1D, Lut3D
// - Math: Vec3, Mat3, adapt_matrix
// - White points: D65, D60, D50
// - Adaptation: BRADFORD, CAT02, VON_KRIES
```

## Common Workflows

### HDR to SDR

```rust
use vfx_color::Pipeline;
use vfx_color::prelude::*;

let pipeline = Pipeline::new()
    .transfer_in(pq::eotf)           // PQ -> linear (nits)
    .scale([1.0/10000.0; 3])         // Normalize to 0-1
    .clamp_01()                      // Simple tonemap
    .matrix(rgb_to_rgb_matrix(&REC2020, &SRGB))
    .transfer_out(srgb::oetf);
```

### Camera Log to Display

```rust
let pipeline = Pipeline::new()
    .transfer_in(log_c::decode)      // LogC -> linear
    .matrix(rgb_to_rgb_matrix(&ARRI_WIDE_GAMUT_3, &REC709))
    .lut3d(display_lut)              // Creative grade (NOTE: lut3d, not lut_3d)
    .transfer_out(srgb::oetf);
```

### sRGB to ACEScg

```rust
use vfx_color::Pipeline;
use vfx_color::prelude::*;

let pipeline = Pipeline::new()
    .transfer_in(srgb::eotf)
    .matrix(rgb_to_xyz_matrix(&SRGB))
    // Chromatic adaptation D65 -> D60
    .matrix(adapt_matrix(D65, D60, &BRADFORD))
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
