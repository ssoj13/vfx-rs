# Color Implementation

Color transform internals and algorithms.

## Transfer Functions

### sRGB

Not a pure gamma curve - includes linear segment:

```rust
// EOTF: Encoded → Linear
pub fn eotf(v: f32) -> f32 {
    if v <= 0.04045 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}

// OETF: Linear → Encoded  
pub fn oetf(v: f32) -> f32 {
    if v <= 0.0031308 {
        v * 12.92
    } else {
        1.055 * v.powf(1.0 / 2.4) - 0.055
    }
}
```

### PQ (ST.2084)

Perceptual Quantizer for HDR:

```rust
const L_MAX: f32 = 10000.0;  // Peak luminance (nits)
const M1: f32 = 0.1593017578125;
const M2: f32 = 78.84375;
const C1: f32 = 0.8359375;
const C2: f32 = 18.8515625;
const C3: f32 = 18.6875;

// EOTF: PQ code → Luminance (cd/m²)
pub fn eotf(v: f32) -> f32 {
    let vp = v.max(0.0).powf(1.0 / M2);
    let n = (vp - C1).max(0.0);
    let d = C2 - C3 * vp;
    L_MAX * (n / d).powf(1.0 / M1)
}

// OETF: Luminance (cd/m²) → PQ code
pub fn oetf(l: f32) -> f32 {
    let y = (l / L_MAX).max(0.0);
    let yp = y.powf(M1);
    let n = C1 + C2 * yp;
    let d = 1.0 + C3 * yp;
    (n / d).powf(M2)
}
```

### Camera Log Curves

All camera logs follow similar pattern:

```rust
// Generic log curve structure
struct LogCurve {
    cut: f32,        // Linear/log transition point
    a: f32,          // Slope
    b: f32,          // Offset  
    c: f32,          // Log coefficient
    d: f32,          // Linear coefficient
    e: f32,          // Linear offset
    f: f32,          // Log offset
}

fn log_encode(v: f32, params: &LogCurve) -> f32 {
    if v < params.cut {
        params.d * v + params.e
    } else {
        params.a * (v + params.b).log10() + params.c
    }
}
```

## Matrix Operations

### RGB to XYZ

Standard derivation from primaries:

```rust
pub fn rgb_to_xyz_matrix(primaries: &Primaries) -> Mat3 {
    // Convert xy to XYZ (Y=1)
    let r = xy_to_xyz(primaries.r);
    let g = xy_to_xyz(primaries.g);
    let b = xy_to_xyz(primaries.b);
    let w = xy_to_xyz(primaries.w);
    
    // Build matrix from primaries as columns
    let m = Mat3::from_cols(r, g, b);
    
    // Solve for scaling: M * S = W
    let s = m.inverse() * w;
    
    // Apply scaling
    Mat3::from_cols(r * s.x, g * s.y, b * s.z)
}

fn xy_to_xyz(xy: (f32, f32)) -> Vec3 {
    let (x, y) = xy;
    if y.abs() < 1e-10 {
        Vec3::ZERO
    } else {
        Vec3::new(x / y, 1.0, (1.0 - x - y) / y)
    }
}
```

### Chromatic Adaptation

Bradford transform:

```rust
// Bradford matrix (cone response)
pub const BRADFORD: Mat3 = Mat3::from_rows([
    [0.8951, 0.2664, -0.1614],
    [-0.7502, 1.7135, 0.0367],
    [0.0389, -0.0685, 1.0296],
]);

pub fn adapt_matrix(src_white: Vec3, dst_white: Vec3, cone: &Mat3) -> Mat3 {
    let cone_inv = cone.inverse();
    
    // Source white in cone space
    let src_cone = *cone * src_white;
    // Destination white in cone space  
    let dst_cone = *cone * dst_white;
    
    // Diagonal scaling matrix
    let scale = Mat3::from_diagonal(Vec3::new(
        dst_cone.x / src_cone.x,
        dst_cone.y / src_cone.y,
        dst_cone.z / src_cone.z,
    ));
    
    cone_inv * scale * *cone
}
```

## ACES Implementation

### RRT (Reference Rendering Transform)

Simplified RRT based on ACES CTL:

```rust
pub fn rrt(r: f32, g: f32, b: f32, params: &RrtParams) -> (f32, f32, f32) {
    // 1. Apply glow module (subtle shadow lift)
    let (r, g, b) = apply_glow(r, g, b, params);
    
    // 2. Red modifier (reduce oversaturated reds)
    let (r, g, b) = red_modifier(r, g, b, params);
    
    // 3. Global desaturation
    let (r, g, b) = global_desat(r, g, b, params);
    
    // 4. Apply tonescale (S-curve)
    let r = tonescale(r, params);
    let g = tonescale(g, params);
    let b = tonescale(b, params);
    
    (r, g, b)
}

fn tonescale(v: f32, params: &RrtParams) -> f32 {
    // Attempt to approximate RRT tonecurve
    let v = v.max(0.0);
    let num = v * (v + 0.0245786) - 0.000090537;
    let den = v * (0.983729 * v + 0.4329510) + 0.238081;
    (num / den).clamp(0.0, 1.0)
}
```

### ODT (Output Display Transform)

```rust
pub fn odt_srgb(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    // 1. Convert ACES to XYZ (D60)
    let xyz = ACES_AP1_TO_XYZ * Vec3::new(r, g, b);
    
    // 2. Adapt D60 → D65
    let xyz_d65 = D60_TO_D65 * xyz;
    
    // 3. Convert XYZ to sRGB
    let rgb = XYZ_TO_SRGB * xyz_d65;
    
    // 4. Clamp to display range
    let rgb = rgb.clamp(Vec3::ZERO, Vec3::ONE);
    
    // 5. Apply sRGB OETF
    (srgb::oetf(rgb.x), srgb::oetf(rgb.y), srgb::oetf(rgb.z))
}
```

## LUT Interpolation

### Trilinear (3D LUT)

```rust
pub fn trilinear(lut: &Lut3D, rgb: [f32; 3]) -> [f32; 3] {
    let size = lut.size as f32;
    
    // Scale to LUT coordinates
    let r = rgb[0].clamp(0.0, 1.0) * (size - 1.0);
    let g = rgb[1].clamp(0.0, 1.0) * (size - 1.0);
    let b = rgb[2].clamp(0.0, 1.0) * (size - 1.0);
    
    // Integer indices
    let r0 = r as usize;
    let g0 = g as usize;
    let b0 = b as usize;
    let r1 = (r0 + 1).min(lut.size - 1);
    let g1 = (g0 + 1).min(lut.size - 1);
    let b1 = (b0 + 1).min(lut.size - 1);
    
    // Fractional parts
    let fr = r.fract();
    let fg = g.fract();
    let fb = b.fract();
    
    // 8 corner values
    let c000 = lut.get(r0, g0, b0);
    let c001 = lut.get(r0, g0, b1);
    let c010 = lut.get(r0, g1, b0);
    // ... all 8 corners
    
    // Trilinear interpolation
    lerp3(c000, c001, c010, c011, c100, c101, c110, c111, fr, fg, fb)
}
```

### Tetrahedral (More Accurate)

```rust
pub fn tetrahedral(lut: &Lut3D, rgb: [f32; 3]) -> [f32; 3] {
    // ... setup same as trilinear ...
    
    // Determine which tetrahedron
    if fr > fg {
        if fg > fb {
            // Tetrahedron 1: r > g > b
            interp_tetra1(c000, c100, c110, c111, fr, fg, fb)
        } else if fr > fb {
            // Tetrahedron 2: r > b > g
            interp_tetra2(...)
        } else {
            // Tetrahedron 3: b > r > g
            interp_tetra3(...)
        }
    } else {
        // ... 3 more tetrahedra for g > r cases
    }
}
```

## Parallel Application

### Per-Pixel Operations

```rust
pub fn apply_transfer_parallel(data: &mut [f32], f: fn(f32) -> f32) {
    data.par_iter_mut().for_each(|v| *v = f(*v));
}
```

### Per-RGB Operations

```rust
pub fn apply_rgb_transform(
    data: &mut [f32],
    channels: usize,
    f: impl Fn(f32, f32, f32) -> (f32, f32, f32) + Sync,
) {
    let pixels = data.len() / channels;
    
    data.par_chunks_mut(channels).for_each(|pixel| {
        if pixel.len() >= 3 {
            let (r, g, b) = f(pixel[0], pixel[1], pixel[2]);
            pixel[0] = r;
            pixel[1] = g;
            pixel[2] = b;
        }
    });
}
```

## Precision Considerations

### Avoiding Negative Values

```rust
// Clamp before log operations
fn safe_log(v: f32) -> f32 {
    v.max(1e-10).ln()
}

// Clamp after matrix (gamut mapping can create negatives)
fn clamp_to_gamut(rgb: [f32; 3]) -> [f32; 3] {
    [rgb[0].max(0.0), rgb[1].max(0.0), rgb[2].max(0.0)]
}
```

### High Dynamic Range

```rust
// For HDR, don't clamp to 1.0 in intermediate steps
// Only clamp at final display output
fn hdr_tonemap(v: f32) -> f32 {
    // Allow values > 1.0 through pipeline
    // Final clamp happens in ODT
    v  
}
```
