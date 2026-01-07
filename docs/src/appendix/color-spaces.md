# Color Space Reference

Complete reference of color spaces supported by vfx-rs.

## Color Primaries

### Display Standards

| Name | Red (x,y) | Green (x,y) | Blue (x,y) | White |
|------|-----------|-------------|------------|-------|
| sRGB | (0.64, 0.33) | (0.30, 0.60) | (0.15, 0.06) | D65 |
| Rec.709 | (0.64, 0.33) | (0.30, 0.60) | (0.15, 0.06) | D65 |
| Rec.2020 | (0.708, 0.292) | (0.170, 0.797) | (0.131, 0.046) | D65 |
| DCI-P3 | (0.680, 0.320) | (0.265, 0.690) | (0.150, 0.060) | DCI |
| Display P3 | (0.680, 0.320) | (0.265, 0.690) | (0.150, 0.060) | D65 |

### ACES

| Name | Red (x,y) | Green (x,y) | Blue (x,y) | White |
|------|-----------|-------------|------------|-------|
| ACES AP0 | (0.7347, 0.2653) | (0.0000, 1.0000) | (0.0001, -0.0770) | D60 |
| ACES AP1 | (0.7130, 0.2930) | (0.1650, 0.8300) | (0.1280, 0.0440) | D60 |

### Camera

| Name | Red (x,y) | Green (x,y) | Blue (x,y) | White |
|------|-----------|-------------|------------|-------|
| ARRI Wide Gamut 3 | (0.6840, 0.3130) | (0.2210, 0.8480) | (0.0861, -0.1020) | D65 |
| ARRI Wide Gamut 4 | (0.7347, 0.2653) | (0.1424, 0.8576) | (0.0991, -0.0308) | D65 |
| Sony S-Gamut3 | (0.7300, 0.2800) | (0.1400, 0.8550) | (0.1000, -0.0500) | D65 |
| RED Wide Gamut | (0.7800, 0.3040) | (0.1210, 0.8340) | (0.0950, -0.0290) | D65 |
| Panasonic V-Gamut | (0.7300, 0.2800) | (0.1650, 0.8400) | (0.1000, -0.0300) | D65 |

### Photography

| Name | Red (x,y) | Green (x,y) | Blue (x,y) | White |
|------|-----------|-------------|------------|-------|
| Adobe RGB | (0.64, 0.33) | (0.21, 0.71) | (0.15, 0.06) | D65 |
| ProPhoto RGB | (0.7347, 0.2653) | (0.1596, 0.8404) | (0.0366, 0.0001) | D50 |

## Transfer Functions

### Display

| Function | Mid-Gray In | Mid-Gray Out | Formula |
|----------|-------------|--------------|---------|
| sRGB | 0.18 | 0.461 | 12.92x (x≤0.0031) or 1.055x^(1/2.4)-0.055 |
| Rec.709 | 0.18 | 0.409 | 4.5x (x≤0.018) or 1.099x^0.45-0.099 |
| Gamma 2.2 | 0.18 | 0.461 | x^(1/2.2) |
| Gamma 2.4 | 0.18 | 0.435 | x^(1/2.4) |

### HDR

| Function | Range | Mid-Gray | Description |
|----------|-------|----------|-------------|
| PQ (ST.2084) | 0-10000 nits | 100 nits @ 0.51 | Perceptual quantizer |
| HLG | System gamma | Variable | Hybrid log-gamma |

### Camera Log

| Function | Mid-Gray | Dynamic Range | Notes |
|----------|----------|---------------|-------|
| ARRI LogC3 | 0.391 | 14+ stops | Alexa classic |
| ARRI LogC4 | 0.278 | 17+ stops | Alexa 35 |
| Sony S-Log2 | 0.339 | 15+ stops | |
| Sony S-Log3 | 0.406 | 15+ stops | |
| RED Log3G10 | 0.333 | 16+ stops | |
| Panasonic V-Log | 0.423 | 14+ stops | |
| Blackmagic Film | 0.38 | 13+ stops | Gen5 |
| Canon C-Log2 | 0.392 | 15+ stops | |
| Canon C-Log3 | 0.343 | 16+ stops | |

### ACES

| Function | Mid-Gray | Range | Use Case |
|----------|----------|-------|----------|
| ACEScct | 0.414 | 0 to 1 | Color grading (with toe) |
| ACEScc | 0.414 | -∞ to 1.5 | Color grading (pure log) |
| ACESproxy | 0.426 | 0 to 1 | On-set monitoring |

## White Points

| Name | x | y | CCT | Usage |
|------|---|---|-----|-------|
| D50 | 0.3457 | 0.3585 | 5003K | Print, ProPhoto |
| D55 | 0.3324 | 0.3474 | 5503K | Daylight film |
| D60 | 0.3217 | 0.3378 | 6004K | ACES |
| D65 | 0.3127 | 0.3290 | 6504K | Video, sRGB |
| DCI | 0.3140 | 0.3510 | 6300K | Digital cinema |

## Gamut Comparison

```
                  ▲ y
                 1.0
                  │
              G(AP0)●
                 /│\
                / │ \
            G(AP1)●  \
              /   │   \
     G(Rec2020)●  │    \
           /      │     \
      G(P3)●      │      \
         /        │       \
    G(sRGB)●──────┼────────● R(sRGB)
              ────┼──── R(P3)
                  │ R(Rec2020)
                  │    R(AP1)
    B(sRGB)●      │         R(AP0)
          \       │
    B(P3)  ●      │
            \     │
   B(Rec2020)●    │
               \  │
            B(AP1)●
                  │
             B(AP0)●
                  └─────────────────▶ x
                0.0              0.8
```

## Conversion Matrices

### sRGB to ACEScg

```
┌─────────────────────────────────────┐
│  0.6131  0.3395  0.0474  │
│  0.0702  0.9164  0.0134  │
│  0.0206  0.1096  0.8698  │
└─────────────────────────────────────┘
```

### ACEScg to sRGB

```
┌─────────────────────────────────────┐
│  1.7051 -0.6218 -0.0833  │
│ -0.1302  1.1408 -0.0106  │
│ -0.0240 -0.1289  1.1529  │
└─────────────────────────────────────┘
```

### Rec.709 to Rec.2020

```
┌─────────────────────────────────────┐
│  0.6274  0.3293  0.0433  │
│  0.0691  0.9195  0.0114  │
│  0.0164  0.0880  0.8956  │
└─────────────────────────────────────┘
```

## Usage in vfx-rs

```rust
use vfx_primaries::Primaries;
use vfx_transfer::*;

// Get primaries
let acescg = Primaries::ACES_AP1;

// Apply transfer
let linear = srgb_to_linear(0.5);
let encoded = linear_to_srgb(0.18);
```
