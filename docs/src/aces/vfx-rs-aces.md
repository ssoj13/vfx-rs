# ACES in vfx-rs

This section covers the specific implementation of ACES in the vfx-rs crates.

## Architecture

ACES support in vfx-rs is distributed across several crates:

```
┌─────────────────────────────────────────────────────────────┐
│                       vfx-cli                               │
│  ┌─────────────────────────────────────────────────────┐   │
│  │ vfx aces command                                     │   │
│  │ IDT/RRT/ODT pipeline execution                       │   │
│  └─────────────────────────────────────────────────────┘   │
└────────────────────────────┬────────────────────────────────┘
                             │
              ┌──────────────┼──────────────┐
              ▼              ▼              ▼
┌──────────────────┐ ┌──────────────┐ ┌──────────────────┐
│    vfx-color     │ │  vfx-ocio    │ │   vfx-transfer   │
│ ACES transforms  │ │ OCIO configs │ │ Transfer funcs   │
│ Matrix math      │ │ ColorSpace   │ │ Log curves       │
└──────────────────┘ └──────────────┘ └──────────────────┘
              │              │              │
              └──────────────┼──────────────┘
                             ▼
                   ┌──────────────────┐
                   │  vfx-primaries   │
                   │  AP0/AP1 matrices│
                   │  Chromatic adapt │
                   └──────────────────┘
```

## vfx-primaries Crate

Defines ACES color spaces and conversion matrices.

### Color Space Definitions

```rust
use vfx_primaries::{ACES_AP0, ACES_AP1, SRGB, REC709, REC2020, DCI_P3};

// ACES Primaries (module-level constants, not Primaries:: variants)
let ap0 = ACES_AP0;  // Archive space
let ap1 = ACES_AP1;  // Working space (ACEScg)

// Common display spaces
let srgb = SRGB;
let rec709 = REC709;
let rec2020 = REC2020;
let p3 = DCI_P3;
```

### Matrix Generation

```rust
use vfx_primaries::{rgb_to_xyz_matrix, xyz_to_rgb_matrix, rgb_to_rgb_matrix};
use vfx_primaries::{SRGB, ACES_AP1};

// Generate conversion matrices
let srgb_to_xyz = rgb_to_xyz_matrix(&SRGB);
let xyz_to_ap1 = xyz_to_rgb_matrix(&ACES_AP1);

// Direct RGB-to-RGB conversion
let srgb_to_acescg = rgb_to_rgb_matrix(&SRGB, &ACES_AP1);
```

### Available Primaries

| Constant | Description |
|----------|-------------|
| `SRGB` | sRGB / Rec.709 primaries |
| `REC709` | ITU-R BT.709 |
| `REC2020` | ITU-R BT.2020 (wide gamut) |
| `DCI_P3` | DCI-P3 (theatrical) |
| `DISPLAY_P3` | Display P3 (consumer) |
| `ACES_AP0` | ACES 2065-1 primaries |
| `ACES_AP1` | ACEScg primaries |
| `ADOBE_RGB` | Adobe RGB (1998) |
| `PROPHOTO_RGB` | ProPhoto RGB |
| `ARRI_WIDE_GAMUT_3` | ARRI Alexa gamut |
| `S_GAMUT3` | Sony S-Gamut3 |
| `V_GAMUT` | Panasonic V-Gamut |

## vfx-transfer Crate

Implements OETF/EOTF transfer functions.

### ACES Transfer Functions

```rust
use vfx_transfer::{acescct, acescc};

// ACEScct (with toe)
let linear = acescct::decode(0.4135);  // ~0.18 mid-gray
let cct = acescct::encode(0.18);       // ~0.4135

// ACEScc (pure log)
let linear = acescc::decode(0.4135);
let cc = acescc::encode(0.18);
```

### Camera Log Functions

```rust
use vfx_transfer::{log_c, s_log3, v_log};

// ARRI LogC3
let linear = log_c::decode(0.391);  // mid-gray

// Sony S-Log3
let linear = s_log3::decode(0.406);  // mid-gray

// Panasonic V-Log
let linear = v_log::decode(0.423);  // mid-gray
```

### Display Transfer Functions

```rust
use vfx_transfer::{srgb, pq, hlg};

// sRGB
let linear = srgb::eotf(0.5);
let encoded = srgb::oetf(0.5);

// PQ (HDR10)
let linear = pq::eotf(0.5);  // In nits: ~100
let encoded = pq::oetf(100.0);

// HLG
let linear = hlg::eotf(0.5);
let encoded = hlg::oetf(0.5);
```

## vfx-color Crate

High-level ACES workflow functions.

### ACES Transforms

```rust
use vfx_color::aces::{srgb_to_acescg, acescg_to_srgb, rrt_odt_srgb};

// sRGB to ACEScg (separate r, g, b arguments)
let (ar, ag, ab) = srgb_to_acescg(0.5, 0.3, 0.2);

// ACEScg to sRGB via RRT+ODT
let (sr, sg, sb) = rrt_odt_srgb(0.18, 0.15, 0.12);
```

### Processing Images

```rust
use vfx_color::aces::apply_rrt_odt_srgb;

// Process pixel data buffer (ACEScg → sRGB display)
let channels = 3;
let acescg_data: Vec<f32> = /* your ACEScg pixels */;
let display_data = apply_rrt_odt_srgb(&acescg_data, channels);
```

**Note:** vfx-color provides `apply_rrt_odt_srgb` for batch processing. There is no separate `apply_idt` function; use `srgb_to_acescg` per-pixel or matrix multiplication for bulk conversion.

## vfx-ocio Crate

Full OCIO integration for complex ACES workflows.

### Loading ACES Config

```rust
use vfx_ocio::{Config, builtin};

// Load ACES OCIO config from file
let config = Config::from_file("/path/to/config.ocio")?;
// Or use built-in ACES 1.3 config
let config = builtin::aces_1_3();

// Get color space names
for cs in config.colorspaces() {
    println!("{}", cs.name());
}
```

### Color Space Conversions

```rust
use vfx_ocio::Config;

let config = Config::from_file("aces_1.2/config.ocio")?;

// Create processor (method on Config, not Processor constructor)
let proc = config.processor("ACES - ACEScg", "Output - sRGB")?;

// Apply to image (apply_rgb for slices of RGB triplets)
proc.apply_rgb(&mut pixels);
```

### Display/View Transforms

```rust
use vfx_ocio::Config;

let config = Config::from_file("/path/to/config.ocio")?;

// Get display/view processor (method on Config)
let proc = config.display_processor(
    "ACES - ACEScg",  // Input
    "sRGB",           // Display
    "ACES 1.0 - SDR Video"  // View
)?;

proc.apply_rgb(&mut pixels);
```

## CLI Usage

The `vfx aces` command wraps these APIs:

```bash
# IDT: sRGB → ACEScg
vfx aces input.jpg -o working.exr -t idt

# RRT only (for custom ODT)
vfx aces working.exr -o rrt.exr -t rrt

# RRT + ODT: ACEScg → sRGB display
vfx aces working.exr -o final.png -t rrt-odt

# High contrast variant
vfx aces working.exr -o final.png -t rrt-odt --rrt-variant high-contrast
```

## Performance

ACES operations are optimized:

- **SIMD**: Matrix operations use packed SIMD where available
- **Parallel**: Image processing uses rayon for multi-threading
- **LUT cache**: 3D LUTs are cached for repeated operations

```rust
// GPU acceleration available via vfx-compute
use vfx_compute::Processor;

let proc = Processor::auto()?;  // GPU if available, CPU fallback
proc.apply_color_matrix(&mut image, &matrix)?;
```
