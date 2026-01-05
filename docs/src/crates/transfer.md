# vfx-transfer

Transfer functions (OETF/EOTF) for color encoding and decoding.

## Purpose

Transfer functions convert between linear light and encoded values. This crate implements all major standards used in VFX, broadcast, and HDR workflows.

## Terminology

- **OETF** (Opto-Electronic): Linear → Encoded (camera recording)
- **EOTF** (Electro-Optical): Encoded → Linear (display)
- **Gamma**: Power-law exponent in the transfer function

## Supported Functions

### Display-Referred

| Module | Use Case | Range |
|--------|----------|-------|
| `srgb` | Web, consumer displays | [0, 1] |
| `gamma` | CRT simulation | [0, 1] |
| `rec709` | HDTV broadcast | [0, 1] |

### HDR

| Module | Use Case | Range |
|--------|----------|-------|
| `pq` | HDR10, Dolby Vision | [0, 10000] cd/m² |
| `hlg` | HLG broadcast | [0, 1] |

### Camera Log (Scene-Referred)

| Module | Camera/System | Dynamic Range |
|--------|---------------|---------------|
| `log_c` | ARRI Alexa | ~14 stops |
| `s_log2` | Sony F65/F55 (legacy) | ~15 stops |
| `s_log3` | Sony Venice/FX | ~15 stops |
| `v_log` | Panasonic VariCam | ~14 stops |
| `red_log` | RED cameras | ~16+ stops |
| `bmd_film` | Blackmagic | ~13 stops |
| `acescc` | ACES grading | ~25 stops |
| `acescct` | ACES grading (toe) | ~25 stops |

## Usage

### Basic Encode/Decode

```rust
use vfx_transfer::{srgb, pq, log_c};

// sRGB: common display encoding
let linear = srgb::eotf(0.5);      // Decode: 0.5 → 0.214
let encoded = srgb::oetf(0.214);   // Encode: 0.214 → 0.5

// PQ: HDR absolute luminance
let nits = pq::eotf(0.5);          // ~100 cd/m²
let pq_code = pq::oetf(100.0);     // ~0.5

// ARRI LogC: camera log
let scene_linear = log_c::decode(0.5);
let log_value = log_c::encode(scene_linear);
```

### Gamma Functions

```rust
use vfx_transfer::gamma::{gamma_eotf, gamma_oetf};

// Pure power function
let linear = gamma_eotf(0.5, 2.2);   // v^2.2
let encoded = gamma_oetf(linear, 2.2); // v^(1/2.2)
```

### RED Log Curves

```rust
use vfx_transfer::red_log;

// REDLogFilm (original)
let linear = red_log::redlogfilm_decode(0.5);

// REDLog3G10 (modern)
let linear = red_log::log3g10_decode(0.5);
```

## Scene vs Display Referred

Understanding the difference is crucial:

**Display-referred** (sRGB, Rec.709):
- Values represent final display output
- Range: typically 0.0 - 1.0
- Middle gray ≈ 0.18 linear ≈ 0.46 encoded

**Scene-referred** (ACES, Log curves):
- Values represent scene light ratios
- Can represent very wide dynamic range (14+ stops)
- Middle gray = 0.18 linear, but encoded value varies

## Implementation Details

### sRGB Transfer

Not a pure gamma! Uses a linear segment near black:

```rust
// Simplified sRGB EOTF
fn eotf(v: f32) -> f32 {
    if v <= 0.04045 {
        v / 12.92           // Linear segment
    } else {
        ((v + 0.055) / 1.055).powf(2.4)  // Power curve
    }
}
```

### PQ (Perceptual Quantizer)

ST.2084 curve for HDR, maps absolute luminance:

```rust
// PQ constants
const M1: f32 = 0.1593017578125;
const M2: f32 = 78.84375;
const C1: f32 = 0.8359375;
const C2: f32 = 18.8515625;
const C3: f32 = 18.6875;
```

### Camera Log Curves

All camera log functions share similar structure:
- Cut point for linear toe
- Log curve for midtones/highlights
- Parametric constants specific to each camera

## Why These Specific Curves?

- **sRGB**: Matches legacy CRT behavior, web standard
- **PQ**: Optimized for human perception at HDR levels
- **HLG**: Backward-compatible with SDR displays
- **Camera logs**: Maximize sensor data utilization

## Dependencies

- `vfx-core` - Core types only

## Used By

- `vfx-color` - Full color pipeline
- `vfx-io` - Format-specific encoding
- `vfx-ocio` - Transform evaluation
