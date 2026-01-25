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
// Signature: adapt_matrix(method, src_white, dst_white)
let adapt = adapt_matrix(BRADFORD, D65, D60);

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
- `DCI_WHITE` - Cinema projection

## Interpolation

```rust
use vfx_math::{lerp, smoothstep, saturate};

// Linear interpolation
let mid = lerp(0.0, 1.0, 0.5);  // 0.5

// Smooth step (ease in/out)
let smooth = smoothstep(0.0, 1.0, 0.5);

// Clamp to 0-1 (saturate)
let clamped = saturate(1.5);  // 1.0
```

## SIMD Operations

SIMD-accelerated operations via the `wide` crate:

```rust
use vfx_math::simd::{batch_mul_add, batch_pow, batch_clamp01, batch_rgb_to_luma};

// Batch multiply-add: out = in * slope + offset
let result = batch_mul_add(&values, 2.0, 0.1);

// Batch power function
let result = batch_pow(&values, 2.2);

// Batch clamp to [0, 1]
let result = batch_clamp01(&values);

// RGB to grayscale (Rec.709)
let luma = batch_rgb_to_luma(&pixels);
```

## Color Math Functions

Optimized implementations for common operations:

```rust
use vfx_math::{saturate, lerp};
use vfx_math::simd::batch_rgb_to_luma;

// Clamp to 0-1
let clamped = saturate(value);

// Rec.709 luminance (via simd module)
let pixels = vec![[0.5, 0.3, 0.2]];
let luma = batch_rgb_to_luma(&pixels);
```

**Note:** For sRGB linearization, use `vfx-transfer::srgb::eotf()` instead.

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
