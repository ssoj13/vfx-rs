# ACEScg Working Space

ACEScg (ACES Computer Graphics) is the recommended working space for VFX and CGI operations.

## Why ACEScg?

### AP1 Primaries

Unlike AP0 (ACES2065-1), AP1 has practical primaries:

```
               ▲ y
               │
          G(AP1)●    G(AP0)●
              ╱│╲      ╱
             ╱ │ ╲    ╱
            ╱  │  ╲  ╱
           ╱   │   ╲╱
          ╱    │   /╲
         ╱     │  /  ╲
        ╱      │ /    ╲
    B(AP1)●────│╱──────●R(AP1)
               │    ●R(AP0)
       ●───────│
     B(AP0)    │
               └───────────────▶ x
```

**AP1 advantages:**
- All primaries inside visible spectrum
- No imaginary colors in working space
- Minimal negative RGB values

### Linear Encoding

ACEScg uses scene-linear encoding (no gamma):

```
Scene Light → ACEScg Value
    0.18    →    0.18 (mid-gray)
    0.36    →    0.36 (1 stop above)
    0.09    →    0.09 (1 stop below)
```

This ensures:
- Correct blending and compositing
- Accurate lighting calculations
- Physically-based rendering compatibility

## ACEScg Specifications

| Property | Value |
|----------|-------|
| **Primaries** | AP1 |
| **White Point** | D60 |
| **Transfer** | Linear (1.0 gamma) |
| **Dynamic Range** | -65504 to +65504 |
| **Recommended Bit Depth** | 16-bit half-float minimum |

### AP1 Chromaticity

| | Red | Green | Blue | White (D60) |
|--|-----|-------|------|-------------|
| x | 0.713 | 0.165 | 0.128 | 0.32168 |
| y | 0.293 | 0.830 | 0.044 | 0.33767 |

## Working in ACEScg

### Compositing

Linear math works correctly:

```rust
// Alpha over (correct in linear)
let result = fg * fg_alpha + bg * (1.0 - fg_alpha);
```

```bash
# vfx-rs compositing
vfx composite fg.exr bg.exr -o result.exr --mode over
```

### Color Corrections

Apply corrections before output transform:

```bash
# Exposure (stops)
vfx color input.exr -o output.exr --exposure 1.5

# Saturation
vfx color input.exr -o output.exr --saturation 1.2

# Both combined
vfx color input.exr -o output.exr --exposure 0.5 --saturation 1.1
```

### CG Integration

Most 3D renderers output linear RGB:

| Renderer | ACEScg Support |
|----------|---------------|
| Arnold | Native ACES workflow |
| V-Ray | ACES color space option |
| RenderMan | OCIO integration |
| Blender Cycles | ACES via OCIO |
| Unreal Engine | ACES post-process |

## vfx-rs ACEScg Operations

### Convert to ACEScg

```rust
use vfx_color::aces::srgb_to_acescg;
use vfx_primaries::{Primaries, rgb_to_rgb_matrix};

// Single pixel
let acescg = srgb_to_acescg([0.5, 0.3, 0.2]);

// Using matrix directly
let matrix = rgb_to_rgb_matrix(&Primaries::SRGB, &Primaries::ACES_AP1);
let acescg = [
    pixel[0] * matrix[0][0] + pixel[1] * matrix[0][1] + pixel[2] * matrix[0][2],
    pixel[0] * matrix[1][0] + pixel[1] * matrix[1][1] + pixel[2] * matrix[1][2],
    pixel[0] * matrix[2][0] + pixel[1] * matrix[2][1] + pixel[2] * matrix[2][2],
];
```

### CLI Operations

```bash
# sRGB to ACEScg (IDT)
vfx aces srgb_input.png -o acescg.exr -t idt

# Or via OCIO
vfx color input.exr -o output.exr --from sRGB --to ACEScg
```

## Best Practices

### File Storage

| Use Case | Format | Recommendation |
|----------|--------|----------------|
| Working files | EXR | 16-bit half float, PIZ compression |
| Archive | EXR | Convert to ACES2065-1 (AP0) |
| Delivery | Various | Apply RRT+ODT for final format |

### Viewing ACEScg

ACEScg is scene-linear and looks wrong on displays:

```
ACEScg (linear)          After RRT+ODT (display)
┌────────────────┐       ┌────────────────┐
│    Very dark   │       │    Normal      │
│    and flat    │  ───▶ │    contrast    │
│                │       │    and color   │
└────────────────┘       └────────────────┘
```

Always use a viewer with OCIO:
```bash
vfx view linear.exr --colorspace ACEScg
```

### EXR Metadata

Store color space in EXR metadata:

```bash
# vfx-rs writes chromaticities to EXR
vfx convert input.exr -o output.exr
# Metadata: chromaticities = AP1, whitePoint = D60
```

## Common Pitfalls

### Negative Values

Some ACEScg images have negative RGB values for very saturated colors:

```
Very saturated blue in sRGB: [0, 0, 1]
Same color in ACEScg: [0.13, -0.02, 0.94]  ← Negative green!
```

**Solution**: Preserve negatives, don't clamp until final output.

### Wrong Working Space

Common mistake: working in sRGB-linear instead of ACEScg

```
sRGB-linear uses sRGB primaries (same as Rec.709)
ACEScg uses AP1 primaries (much wider)

Mixing them causes color shifts!
```

### Premature Clamping

Don't clamp values in the pipeline:

```rust
// WRONG - loses highlight data
let value = value.max(0.0).min(1.0);

// CORRECT - preserve full range
let value = value;  // No clamping in working space
```
