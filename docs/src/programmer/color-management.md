# Color Management

Color space handling, transfer functions, and ACES workflows in vfx-rs.

## Overview

Color management in vfx-rs involves:

1. **Color Primaries** - RGB to XYZ mapping (gamut)
2. **Transfer Functions** - Linear to encoded (gamma, log)
3. **Chromatic Adaptation** - White point conversion
4. **ACES Transforms** - Scene-referred workflow

## Color Primaries (vfx-primaries)

### Available Primaries

```rust
use vfx_primaries::Primaries;

// Standard gamuts
let srgb = Primaries::SRGB;
let rec709 = Primaries::REC709;
let rec2020 = Primaries::REC2020;
let dci_p3 = Primaries::DCI_P3;
let display_p3 = Primaries::DISPLAY_P3;

// ACES
let ap0 = Primaries::ACES_AP0;
let ap1 = Primaries::ACES_AP1;

// Camera specific
let arri_wg3 = Primaries::ARRI_WIDE_GAMUT_3;
let s_gamut3 = Primaries::S_GAMUT3;
let v_gamut = Primaries::V_GAMUT;
```

### Matrix Generation

```rust
use vfx_primaries::{rgb_to_xyz_matrix, xyz_to_rgb_matrix, rgb_to_rgb_matrix};

// RGB to XYZ
let srgb_to_xyz = rgb_to_xyz_matrix(&Primaries::SRGB);

// XYZ to RGB
let xyz_to_acescg = xyz_to_rgb_matrix(&Primaries::ACES_AP1);

// Direct RGB to RGB
let srgb_to_acescg = rgb_to_rgb_matrix(
    &Primaries::SRGB,
    &Primaries::ACES_AP1
);
```

### Apply Matrix

```rust
// Apply to single pixel
fn apply_matrix(pixel: [f32; 3], matrix: &[[f32; 3]; 3]) -> [f32; 3] {
    [
        pixel[0] * matrix[0][0] + pixel[1] * matrix[0][1] + pixel[2] * matrix[0][2],
        pixel[0] * matrix[1][0] + pixel[1] * matrix[1][1] + pixel[2] * matrix[1][2],
        pixel[0] * matrix[2][0] + pixel[1] * matrix[2][1] + pixel[2] * matrix[2][2],
    ]
}
```

## Transfer Functions (vfx-transfer)

### Encode/Decode

```rust
use vfx_transfer::*;

// sRGB
let encoded = linear_to_srgb(0.18);   // 0.18 → ~0.46
let linear = srgb_to_linear(0.46);    // ~0.46 → 0.18

// Gamma
let encoded = linear_to_gamma(0.18, 2.2);
let linear = gamma_to_linear(0.46, 2.2);

// Rec.709
let encoded = linear_to_rec709(0.18);
let linear = rec709_to_linear(0.409);
```

### HDR Transfer Functions

```rust
// PQ (ST.2084)
let pq = linear_to_pq(100.0);     // 100 nits → ~0.51
let nits = pq_to_linear(0.51);    // ~0.51 → 100 nits

// HLG
let hlg = linear_to_hlg(0.18);
let linear = hlg_to_linear(0.5);
```

### Camera Log

```rust
// ARRI LogC
let logc = linear_to_logc3(0.18);  // 0.18 → ~0.39
let linear = logc3_to_linear(0.39);

// Sony S-Log3
let slog = linear_to_slog3(0.18);  // 0.18 → ~0.41
let linear = slog3_to_linear(0.41);

// RED Log3G10
let redlog = linear_to_redlog(0.18);
let linear = redlog_to_linear(0.33);

// Blackmagic Film
let bmd = linear_to_bmdfilm(0.18);
let linear = bmdfilm_to_linear(0.38);
```

### ACES Log

```rust
// ACEScct (with toe)
let cct = linear_to_acescct(0.18);  // ~0.41
let linear = acescct_to_linear(0.41);

// ACEScc (pure log)
let cc = linear_to_acescc(0.18);
let linear = acescc_to_linear(0.41);
```

## ACES Workflow (vfx-color)

### Convert to ACEScg

```rust
use vfx_color::aces::{srgb_to_acescg, apply_idt};

// Single pixel
let acescg = srgb_to_acescg([0.5, 0.3, 0.2]);

// Full image
let mut image = vfx_io::read("input.jpg")?;
apply_idt(&mut image)?;  // sRGB → ACEScg
```

### Output Transform

```rust
use vfx_color::aces::{acescg_to_srgb, apply_rrt_odt};

// Single pixel (RRT + ODT)
let srgb = acescg_to_srgb([0.18, 0.15, 0.12]);

// Full image
apply_rrt_odt(&mut image)?;  // ACEScg → sRGB display
```

### Complete Pipeline

```rust
use vfx_color::aces::{apply_idt, apply_rrt_odt};

// 1. Load and convert to ACEScg
let mut image = vfx_io::read("camera.dpx")?;
apply_idt(&mut image)?;

// 2. Process in ACEScg
vfx_ops::exposure(&mut image, 0.5)?;
vfx_ops::saturation(&mut image, 1.1)?;

// 3. Output transform
apply_rrt_odt(&mut image)?;

vfx_io::write("output.png", &image)?;
```

## LUT Application (vfx-lut)

### Load and Apply

```rust
use vfx_lut::{Lut3D, apply_lut};

// Load 3D LUT
let lut = Lut3D::from_file("film_look.cube")?;

// Apply to image
apply_lut(&mut image, &lut)?;
```

### Supported Formats

```rust
use vfx_lut::Lut;

// Auto-detect format
let lut = Lut::from_file("transform.cube")?;  // Resolve/Adobe
let lut = Lut::from_file("transform.clf")?;   // ACES CLF
let lut = Lut::from_file("transform.spi3d")?; // Sony
let lut = Lut::from_file("transform.3dl")?;   // Autodesk
```

## ICC Profiles (vfx-icc)

### Profile Loading

```rust
use vfx_icc::{Profile, Transform, Intent};

// Load from file
let camera = Profile::from_file("camera.icc")?;

// Built-in profiles
let srgb = Profile::srgb();
let adobe_rgb = Profile::adobe_rgb();
let aces_ap0 = Profile::aces_ap0();
```

### Transforms

```rust
// Create transform
let transform = Transform::new(
    &camera,
    &Profile::aces_ap0(),
    Intent::Perceptual
)?;

// Apply to pixels
transform.apply(&mut pixels)?;
```

## Example: Full Color Pipeline

```rust
use vfx_io::{read, write};
use vfx_primaries::{Primaries, rgb_to_rgb_matrix};
use vfx_transfer::{srgb_to_linear, linear_to_srgb};
use vfx_color::aces::{apply_rrt_odt};

fn main() -> anyhow::Result<()> {
    // Read sRGB image
    let mut image = read("photo.jpg")?;

    // 1. Linearize (remove sRGB gamma)
    for pixel in image.pixels_mut() {
        pixel[0] = srgb_to_linear(pixel[0]);
        pixel[1] = srgb_to_linear(pixel[1]);
        pixel[2] = srgb_to_linear(pixel[2]);
    }

    // 2. Convert to ACEScg
    let matrix = rgb_to_rgb_matrix(&Primaries::SRGB, &Primaries::ACES_AP1);
    for pixel in image.pixels_mut() {
        let r = pixel[0] * matrix[0][0] + pixel[1] * matrix[0][1] + pixel[2] * matrix[0][2];
        let g = pixel[0] * matrix[1][0] + pixel[1] * matrix[1][1] + pixel[2] * matrix[1][2];
        let b = pixel[0] * matrix[2][0] + pixel[1] * matrix[2][1] + pixel[2] * matrix[2][2];
        pixel[0] = r; pixel[1] = g; pixel[2] = b;
    }

    // 3. Process in ACEScg...

    // 4. Output transform
    apply_rrt_odt(&mut image)?;

    write("output.png", &image)?;
    Ok(())
}
```
