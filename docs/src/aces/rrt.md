# Reference Rendering Transform (RRT)

The RRT is the heart of ACES output, converting scene-referred imagery to display-referred.

## Purpose

The RRT performs the critical mapping from the infinite dynamic range of scene light to the limited range of display devices.

```
Scene Light (0 → ∞)     RRT      Display Output (0 → 1)

       ∞  ───────────────────────────── 1.0 (peak white)
          ╲
           ╲  Highlight compression
            ╲
             ╲
   1.0  ──────●────────────────────── 0.8 (bright)
              │
              │ Linear-ish region
              │
  0.18  ──────●────────────────────── 0.18 (mid-gray preserved)
              │
              │
              │ Shadow region
       0  ────●────────────────────── 0.0 (display black)
```

## What RRT Does

### 1. Global Desaturation

Extreme luminance values are desaturated to prevent color skew:

```
Very bright red → Less saturated bright
Very dark blue → Less saturated dark
```

This mimics how film and human vision handle extreme values.

### 2. Tone Mapping (S-Curve)

The characteristic filmic S-curve:

| Region | Behavior |
|--------|----------|
| **Toe** | Lifts shadows, adds density |
| **Linear** | Near 1:1 around mid-gray |
| **Shoulder** | Compresses highlights gently |

### 3. Highlight Rolloff

Unlike digital clipping, highlights roll off gradually:

```
Digital Clip:    Filmic Rolloff (RRT):
    │ ████████       │     ____
    │ ████████       │    /
    │ ████████       │   /
    │ ████████       │  /
    │ ███            │ /
    │ ██             │/
```

### 4. Color Appearance

The RRT includes subtle color adjustments:
- Blue hue shifts (similar to film)
- Red desaturation in highlights
- Shadow color preservation

## RRT Versions

### ACES 1.0 RRT (2015)
- Original reference transform
- Some issues with very saturated colors
- Still widely used

### ACES 1.1 RRT (2019)
- Improved highlight handling
- Better saturated color behavior
- Default for new projects

### ACES 2.0 (Coming)
- Completely redesigned tone curve
- Better skin tone handling
- Improved gamut mapping

## Using RRT in vfx-rs

### Apply RRT Only

```bash
# RRT without ODT (produces OCES)
vfx aces linear.exr -o rrt_output.exr -t rrt
```

### Apply RRT + ODT

```bash
# Combined output transform
vfx aces linear.exr -o final.png -t rrt-odt
```

### High Contrast Variant

```bash
# RRT with increased contrast
vfx aces linear.exr -o final.png -t rrt-odt --rrt-variant high-contrast
```

## RRT Parameters

The RRT is parameterized but typically used with defaults:

| Parameter | Default | Description |
|-----------|---------|-------------|
| `mid_gray` | 0.18 | Input mid-gray point |
| `min_exposure` | -6.5 | Stops below mid-gray |
| `max_exposure` | +6.5 | Stops above mid-gray |

## Behind the Scenes

The RRT is implemented as a series of transforms:

```
Input (ACES)
    │
    ▼
┌─────────────────┐
│ glow_module     │  Localized flare simulation
└─────────────────┘
    │
    ▼
┌─────────────────┐
│ red_modifier    │  Red hue rotation
└─────────────────┘
    │
    ▼
┌─────────────────┐
│ global_desat    │  Desaturate extremes
└─────────────────┘
    │
    ▼
┌─────────────────┐
│ rgb_to_rrt      │  Convert to RRT primaries
└─────────────────┘
    │
    ▼
┌─────────────────┐
│ tone_scale      │  S-curve per channel
└─────────────────┘
    │
    ▼
┌─────────────────┐
│ rrt_to_oces     │  Convert to OCES
└─────────────────┘
    │
    ▼
Output (OCES)
```

## Common Issues

### "RRT looks too contrasty"
The RRT has a designed contrast level. For less contrast:
- Grade in ACEScg before RRT
- Use LMT to adjust

### "Highlights clip too early"
Check your input values:
- Ensure scene-linear input
- Values should exceed 1.0 for highlights
- If clipped before RRT, data is lost

### "Colors look different than expected"
RRT intentionally modifies colors for filmic look:
- Blue shadows shift slightly
- Saturated highlights desaturate
- This is working as designed

## Comparison: With and Without RRT

```
Without RRT (linear to display):
- Harsh highlight clipping
- Crushed shadows
- Linear, "digital" look
- Oversaturated colors

With RRT:
- Smooth highlight rolloff
- Open shadows with detail
- Filmic, "cinematic" look
- Natural color response
```
