# vfx-math

Math utilities for VFX color and image processing.

## Purpose

Provides matrix and vector math for color space conversions, built on `glam` with VFX-specific additions.

## Key Types

### Mat3 - 3x3 Matrix

```rust
use vfx_math::{Mat3, Vec3};

// Create from rows
let m = Mat3::from_rows([
    [0.412, 0.358, 0.180],
    [0.213, 0.715, 0.072],
    [0.019, 0.119, 0.950],
]);

// Matrix-vector multiplication
let rgb = Vec3::new(1.0, 0.5, 0.25);
let xyz = m * rgb;

// Matrix-matrix multiplication
let combined = m2 * m1;  // Apply m1, then m2

// Inversion
let m_inv = m.inverse().unwrap();
```

### Vec3 - 3D Vector

```rust
use vfx_math::Vec3;

let v = Vec3::new(0.5, 0.3, 0.2);

// Component access
println!("x: {}, y: {}, z: {}", v.x, v.y, v.z);

// Common operations
let scaled = v * 2.0;
let sum = v + Vec3::ONE;
let dot = v.dot(Vec3::ONE);
```

## Chromatic Adaptation

Adapt colors between different white points:

```rust
use vfx_math::{adapt_matrix, BRADFORD, D65, D60};

// Create D65 → D60 adaptation matrix
let adapt = adapt_matrix(D65, D60, &BRADFORD);

// Apply to XYZ color
let xyz_d60 = adapt * xyz_d65;
```

Available adaptation matrices:
- `BRADFORD` - Most accurate for natural scenes
- `CAT02` - CIE CAT02 standard
- `VON_KRIES` - Legacy, cone response

Standard white points:
- `D65` - Daylight (~6500K)
- `D60` - ACES (~6000K)
- `D50` - Print (~5000K)
- `DCI` - Cinema projection

## Interpolation

```rust
use vfx_math::{lerp, smoothstep, catmull_rom};

// Linear interpolation
let mid = lerp(0.0, 1.0, 0.5);  // 0.5

// Smooth step (ease in/out)
let smooth = smoothstep(0.0, 1.0, 0.5);

// Catmull-Rom spline
let spline = catmull_rom(p0, p1, p2, p3, t);
```

## SIMD Operations

SIMD-accelerated operations via the `wide` crate:

```rust
use vfx_math::simd::{process_rgba_f32x8, apply_matrix_simd};

// Process 8 pixels at once
process_rgba_f32x8(&mut data, |r, g, b, a| {
    // Transform operates on f32x8 vectors
    (r * 1.1, g, b * 0.9, a)
});
```

## Color Math Functions

Optimized implementations for common operations:

```rust
use vfx_math::{rgb_to_luminance, saturate, linearize_srgb};

// Rec.709 luminance
let lum = rgb_to_luminance(r, g, b);

// Clamp to 0-1
let clamped = saturate(value);
```

## Dependencies

- `vfx-core` - Core types
- `glam` - SIMD math library
- `wide` - Explicit SIMD types

## Design Decisions

### Row-Major Storage

Matrices use row-major storage with column vectors:

```rust
// Transform: result = matrix * vector
let result = m * v;

// Matrix chain: apply m1, then m2
let combined = m2 * m1;
```

### glam Wrapper

`Mat3` and `Vec3` wrap `glam` types, adding:
- `from_rows()` / `from_cols()` constructors
- Color-specific methods
- Display formatting for debugging

## Used By

- `vfx-primaries` - RGB/XYZ matrix generation
- `vfx-color` - Color space conversions
- `vfx-ocio` - Transform processing
