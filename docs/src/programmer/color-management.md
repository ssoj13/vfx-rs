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
use vfx_primaries::{
    SRGB, REC709, REC2020, DCI_P3, DISPLAY_P3,
    ACES_AP0, ACES_AP1,
    ARRI_WIDE_GAMUT_3, S_GAMUT3, V_GAMUT,
    Primaries,
};

// Standard gamuts (module-level constants)
let srgb = SRGB;
let rec709 = REC709;  // Same as SRGB
let rec2020 = REC2020;
let dci_p3 = DCI_P3;
let display_p3 = DISPLAY_P3;

// ACES
let ap0 = ACES_AP0;
let ap1 = ACES_AP1;

// Camera specific
let arri_wg3 = ARRI_WIDE_GAMUT_3;
let s_gamut3 = S_GAMUT3;
let v_gamut = V_GAMUT;
```

### Matrix Generation

```rust
use vfx_primaries::{rgb_to_xyz_matrix, xyz_to_rgb_matrix, rgb_to_rgb_matrix, SRGB, ACES_AP1};

// RGB to XYZ
let srgb_to_xyz = rgb_to_xyz_matrix(&SRGB);

// XYZ to RGB
let xyz_to_acescg = xyz_to_rgb_matrix(&ACES_AP1);

// Direct RGB to RGB
let srgb_to_acescg = rgb_to_rgb_matrix(&SRGB, &ACES_AP1);
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

Transfer functions use EOTF (electro-optical) and OETF (opto-electronic) naming:

- **EOTF** (decode): Encoded signal -> Linear light
- **OETF** (encode): Linear light -> Encoded signal

```rust
use vfx_transfer::srgb;

// sRGB
let linear = srgb::eotf(0.46);     // ~0.46 encoded -> 0.18 linear
let encoded = srgb::oetf(0.18);    // 0.18 linear -> ~0.46 encoded

// RGB arrays
let linear_rgb = srgb::eotf_rgb([0.46, 0.46, 0.46]);
let encoded_rgb = srgb::oetf_rgb([0.18, 0.18, 0.18]);

// Gamma
use vfx_transfer::gamma;
let linear = gamma::gamma_eotf(0.46, 2.2);   // Gamma 2.2 decode
let encoded = gamma::gamma_oetf(0.18, 2.2);  // Gamma 2.2 encode

// Rec.709
use vfx_transfer::rec709;
let linear = rec709::eotf(0.409);
let encoded = rec709::oetf(0.18);
```

### HDR Transfer Functions

```rust
// PQ (ST.2084) - values in nits
use vfx_transfer::pq;
let nits = pq::eotf(0.51);         // ~0.51 PQ -> 100 nits
let pq_val = pq::oetf(100.0);      // 100 nits -> ~0.51 PQ

// HLG
use vfx_transfer::hlg;
let linear = hlg::eotf(0.5);
let hlg_val = hlg::oetf(0.18);
```

### Camera Log

```rust
// ARRI LogC3
use vfx_transfer::log_c;
let logc = log_c::encode(0.18);     // 0.18 linear -> ~0.39 LogC
let linear = log_c::decode(0.39);   // ~0.39 LogC -> 0.18 linear

// ARRI LogC4
use vfx_transfer::log_c4;
let logc4 = log_c4::encode(0.18);
let linear = log_c4::decode(0.39);

// Sony S-Log3
use vfx_transfer::s_log3;
let slog = s_log3::encode(0.18);    // 0.18 -> ~0.41 S-Log3
let linear = s_log3::decode(0.41);

// RED Log3G10
use vfx_transfer::red_log;
let redlog = red_log::log3g10_encode(0.18);
let linear = red_log::log3g10_decode(0.33);

// Blackmagic Film Gen5
use vfx_transfer::bmd_film;
let bmd = bmd_film::bmd_film_gen5_encode(0.18);
let linear = bmd_film::bmd_film_gen5_decode(0.38);
```

### ACES Log

```rust
// ACEScct (with toe for shadows)
use vfx_transfer::acescct;
let cct = acescct::encode(0.18);    // ~0.41
let linear = acescct::decode(0.41);

// ACEScc (pure log, no toe)
use vfx_transfer::acescc;
let cc = acescc::encode(0.18);
let linear = acescc::decode(0.41);
```

## ACES Workflow (vfx-color)

### Convert to ACEScg

```rust
use vfx_color::aces;

// Single pixel - takes 3 separate args, returns tuple
let (r, g, b) = aces::srgb_to_acescg(0.5, 0.3, 0.2);

// Reverse: ACEScg to sRGB linear
let (r, g, b) = aces::acescg_to_srgb(0.18, 0.15, 0.12);
```

### Output Transform (RRT + ODT)

```rust
use vfx_color::aces;

// Single pixel: apply RRT + ODT for sRGB display
let (r, g, b) = aces::rrt_odt_srgb(0.18, 0.15, 0.12);

// For Rec.709 display
let (r, g, b) = aces::rrt_odt_rec709(0.18, 0.15, 0.12);
```

### Buffer Processing

```rust
use vfx_color::aces;

// apply_rrt_odt_srgb returns a NEW Vec (does not modify in-place)
let linear_data: Vec<f32> = vec![0.18, 0.15, 0.12, /* ... */];
let channels = 3;

// Returns new buffer with RRT+ODT applied
let display_data = aces::apply_rrt_odt_srgb(&linear_data, channels);
```

### Complete Pipeline

```rust
use vfx_io;
use vfx_transfer::srgb;
use vfx_color::aces;

fn process_image() -> anyhow::Result<()> {
    // 1. Load image
    let image = vfx_io::read("photo.jpg")?;
    let mut data = image.to_f32();
    let channels = image.channels as usize;

    // 2. Linearize (decode sRGB transfer function)
    for pixel in data.chunks_exact_mut(channels.min(3)) {
        let rgb = srgb::eotf_rgb([pixel[0], pixel[1], pixel[2]]);
        pixel[0] = rgb[0];
        pixel[1] = rgb[1];
        pixel[2] = rgb[2];
    }

    // 3. Convert to ACEScg for processing
    for pixel in data.chunks_exact_mut(channels.min(3)) {
        let (r, g, b) = aces::srgb_to_acescg(pixel[0], pixel[1], pixel[2]);
        pixel[0] = r;
        pixel[1] = g;
        pixel[2] = b;
    }

    // 4. Process in ACEScg...
    // (exposure, grade, compositing, etc.)

    // 5. Apply RRT+ODT for display (returns new Vec)
    let display = aces::apply_rrt_odt_srgb(&data, channels);

    // 6. Save result
    let output = vfx_io::ImageData::from_f32(
        image.width, image.height, image.channels, display
    )?;
    vfx_io::write("output.png", &output)?;

    Ok(())
}
```

## LUT Application (vfx-lut)

### Load and Apply 3D LUT

```rust
use vfx_lut::cube;

// Load 3D LUT from .cube file
let lut = cube::read_3d("film_look.cube")?;

// Apply to single pixel (returns new value)
let result = lut.apply([0.5, 0.3, 0.2]);

// Apply to buffer
for pixel in data.chunks_exact_mut(3) {
    let rgb = lut.apply([pixel[0], pixel[1], pixel[2]]);
    pixel[0] = rgb[0];
    pixel[1] = rgb[1];
    pixel[2] = rgb[2];
}
```

### Load and Apply 1D LUT

```rust
use vfx_lut::cube;

let lut1d = cube::read_1d("gamma.cube")?;

// Single value
let out = lut1d.apply(0.5);

// RGB array
let rgb = lut1d.apply_rgb([0.5, 0.3, 0.2]);
```

### Combined 1D+3D (Resolve .cube)

```rust
use vfx_lut::cube;

// Some .cube files contain both 1D shaper and 3D LUT
let cube_file = cube::read_cube("resolve_look.cube")?;

// Check what's in the file
if let Some(ref shaper) = cube_file.lut1d {
    // Apply 1D shaper first
    let rgb = shaper.apply_rgb([r, g, b]);
}

if let Some(ref lut3d) = cube_file.lut3d {
    // Then apply 3D LUT
    let result = lut3d.apply([r, g, b]);
}
```

### Other LUT Formats

```rust
use vfx_lut::{clf, csp, hdl, iridas_itx};

// CLF/CTF (ACES Common LUT Format)
let process_list = clf::read_clf("transform.clf")?;
let mut rgb = [0.5, 0.3, 0.2];
process_list.apply(&mut rgb);

// Cinespace (.csp)
let csp_file = csp::read_csp("transform.csp")?;

// Houdini HDL
let hdl_file = hdl::read_hdl("transform.hdl")?;

// Iridas/Autodesk .itx
let lut = iridas_itx::read_itx("transform.itx")?;
```

## Example: Full Color Pipeline

```rust
use vfx_io;
use vfx_primaries::{rgb_to_rgb_matrix, SRGB, ACES_AP1};
use vfx_transfer::srgb;
use vfx_color::aces;

fn main() -> anyhow::Result<()> {
    // Read sRGB image
    let image = vfx_io::read("photo.jpg")?;
    let mut data = image.to_f32();
    let channels = image.channels as usize;

    // 1. Linearize (decode sRGB transfer function)
    for pixel in data.chunks_exact_mut(channels.min(3)) {
        let rgb = srgb::eotf_rgb([pixel[0], pixel[1], pixel[2]]);
        pixel[0] = rgb[0];
        pixel[1] = rgb[1];
        pixel[2] = rgb[2];
    }

    // 2. Convert to ACEScg
    let matrix = rgb_to_rgb_matrix(&SRGB, &ACES_AP1);
    for pixel in data.chunks_exact_mut(channels.min(3)) {
        let r = pixel[0] * matrix[0][0] + pixel[1] * matrix[0][1] + pixel[2] * matrix[0][2];
        let g = pixel[0] * matrix[1][0] + pixel[1] * matrix[1][1] + pixel[2] * matrix[1][2];
        let b = pixel[0] * matrix[2][0] + pixel[1] * matrix[2][1] + pixel[2] * matrix[2][2];
        pixel[0] = r;
        pixel[1] = g;
        pixel[2] = b;
    }

    // 3. Process in ACEScg...

    // 4. Output transform (returns new Vec)
    let display = aces::apply_rrt_odt_srgb(&data, channels);

    // 5. Save
    let output = vfx_io::ImageData::from_f32(
        image.width, image.height, image.channels, display
    )?;
    vfx_io::write("output.png", &output)?;

    Ok(())
}
```

## See Also

- [vfx-primaries](../crates/vfx-primaries.md) - Color primary definitions
- [vfx-transfer](../crates/vfx-transfer.md) - Transfer function implementations
- [vfx-color](../crates/vfx-color.md) - Color processing utilities
- [vfx-lut](../crates/vfx-lut.md) - LUT loading and application
