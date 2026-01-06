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
use vfx_primaries::Primaries;

// ACES Primaries
let ap0 = Primaries::ACES_AP0;  // Archive space
let ap1 = Primaries::ACES_AP1;  // Working space (ACEScg)

// Common display spaces
let srgb = Primaries::SRGB;
let rec709 = Primaries::REC709;
let rec2020 = Primaries::REC2020;
let p3 = Primaries::DCI_P3;
```

### Matrix Generation

```rust
use vfx_primaries::{rgb_to_xyz_matrix, xyz_to_rgb_matrix, rgb_to_rgb_matrix};

// Generate conversion matrices
let srgb_to_xyz = rgb_to_xyz_matrix(&Primaries::SRGB);
let xyz_to_ap1 = xyz_to_rgb_matrix(&Primaries::ACES_AP1);

// Direct RGB-to-RGB conversion
let srgb_to_acescg = rgb_to_rgb_matrix(&Primaries::SRGB, &Primaries::ACES_AP1);
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
use vfx_transfer::{acescct_to_linear, linear_to_acescct};
use vfx_transfer::{acescc_to_linear, linear_to_acescc};

// ACEScct (with toe)
let linear = acescct_to_linear(0.4135);  // ~0.18 mid-gray
let cct = linear_to_acescct(0.18);       // ~0.4135

// ACEScc (pure log)
let linear = acescc_to_linear(0.4135);
let cc = linear_to_acescc(0.18);
```

### Camera Log Functions

```rust
use vfx_transfer::{logc3_to_linear, slog3_to_linear, vlog_to_linear};

// ARRI LogC3
let linear = logc3_to_linear(0.391);  // mid-gray

// Sony S-Log3
let linear = slog3_to_linear(0.406);  // mid-gray

// Panasonic V-Log
let linear = vlog_to_linear(0.423);  // mid-gray
```

### Display Transfer Functions

```rust
use vfx_transfer::{srgb_to_linear, linear_to_srgb};
use vfx_transfer::{pq_to_linear, linear_to_pq};
use vfx_transfer::{hlg_to_linear, linear_to_hlg};

// sRGB
let linear = srgb_to_linear(0.5);
let encoded = linear_to_srgb(0.5);

// PQ (HDR10)
let linear = pq_to_linear(0.5);  // In nits: ~100
let encoded = linear_to_pq(100.0);

// HLG
let linear = hlg_to_linear(0.5);
let encoded = linear_to_hlg(0.5);
```

## vfx-color Crate

High-level ACES workflow functions.

### ACES Transforms

```rust
use vfx_color::aces::{srgb_to_acescg, acescg_to_srgb};
use vfx_color::aces::{apply_aces_idt, apply_aces_rrt_odt};

// sRGB to ACEScg (IDT)
let acescg = srgb_to_acescg([0.5, 0.3, 0.2]);

// ACEScg to sRGB (RRT+ODT)
let srgb = acescg_to_srgb([0.18, 0.15, 0.12]);
```

### Processing Images

```rust
use vfx_color::aces::{apply_idt, apply_rrt_odt};
use vfx_io::ImageData;

let mut image: ImageData = ...;

// Apply IDT (sRGB → ACEScg)
apply_idt(&mut image)?;

// ... do compositing work ...

// Apply output transform (RRT + ODT)
apply_rrt_odt(&mut image)?;
```

## vfx-ocio Crate

Full OCIO integration for complex ACES workflows.

### Loading ACES Config

```rust
use vfx_ocio::{Config, Processor};

// Load ACES OCIO config
let config = Config::from_env()?;  // Uses $OCIO
// Or: Config::from_file("/path/to/config.ocio")?

// Get color space names
for cs in config.color_spaces() {
    println!("{}", cs);
}
```

### Color Space Conversions

```rust
use vfx_ocio::{Config, Processor};

let config = Config::from_file("aces_1.2/config.ocio")?;

// Create processor
let proc = Processor::new(&config, "ACES - ACEScg", "Output - sRGB")?;

// Apply to image
proc.apply(&mut pixels)?;
```

### Display/View Transforms

```rust
use vfx_ocio::{Config, DisplayViewProcessor};

let config = Config::from_env()?;

// Get display/view processor
let proc = DisplayViewProcessor::new(
    &config,
    "ACES - ACEScg",  // Input
    "sRGB",           // Display
    "ACES 1.0 - SDR Video"  // View
)?;

proc.apply(&mut pixels)?;
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
