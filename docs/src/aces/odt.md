# Output Device Transforms (ODT)

ODTs convert the output of RRT (OCES) to specific display device encodings.

## Purpose

Different displays have different capabilities:
- **sRGB monitors**: ~100 nits, sRGB gamut
- **HDR TVs**: 1000+ nits, Rec.2020 gamut
- **Digital cinema**: 48 nits (14 fL), DCI-P3 gamut

The ODT maps OCES to each display's specific encoding.

## ODT Architecture

```
OCES (from RRT)
    │
    ▼
┌─────────────────┐
│ Tonescale limit │  Limit to display peak
└─────────────────┘
    │
    ▼
┌─────────────────┐
│ Gamut compress  │  Fit to display gamut
└─────────────────┘
    │
    ▼
┌─────────────────┐
│ Surround adjust │  Viewing environment
└─────────────────┘
    │
    ▼
┌─────────────────┐
│ Display EOTF    │  Apply gamma/PQ/HLG
└─────────────────┘
    │
    ▼
Display Signal
```

## Common ODTs

### SDR ODTs

| ODT | White Point | Gamut | Use Case |
|-----|-------------|-------|----------|
| sRGB | D65 | sRGB | Computer monitors |
| sRGB (D60 sim) | D60 | sRGB | ACES white on sRGB |
| Rec.709 | D65 | Rec.709 | HD broadcast |
| Rec.709 (D60 sim) | D60 | Rec.709 | ACES white HD |
| DCI-P3 | DCI white | P3 | Digital cinema |
| DCI-P3 D65 | D65 | P3 | Wide-gamut monitors |

### HDR ODTs

| ODT | Peak Nits | Gamut | Use Case |
|-----|-----------|-------|----------|
| Rec.2020 PQ 1000 | 1000 | Rec.2020 | HDR10 |
| Rec.2020 PQ 2000 | 2000 | Rec.2020 | Premium HDR |
| Rec.2020 PQ 4000 | 4000 | Rec.2020 | Professional HDR |
| P3-D65 PQ 1000 | 1000 | P3 | Dolby Vision |
| Rec.2020 HLG | 1000 | Rec.2020 | HLG broadcast |

## Using ODT in vfx-rs

**Note:** The vfx-rs library provides sRGB and Rec.709 ODT helpers. HDR ODTs (PQ, HLG, P3, Rec.2020) require OCIO configuration or custom implementation.

### Default sRGB Output (CLI)

```bash
# RRT + sRGB ODT combined (built-in)
vfx aces input.exr -o output.png -t rrt-odt
```

### Broadcast Rec.709 (via OCIO)

For Rec.709 and other display transforms, use OCIO:

```bash
# Via OCIO config
export OCIO=/path/to/aces_config.ocio
vfx ocio input.exr -o output.dpx \
    --src ACEScg \
    --dst "Rec.709 - Display"
```

### HDR Output (via OCIO)

HDR transforms require OCIO configuration:

```bash
# HDR10 (Rec.2020 PQ 1000 nits) via OCIO
export OCIO=/path/to/aces_config.ocio
vfx ocio input.exr -o output_hdr.exr \
    --src ACEScg \
    --dst "Rec.2020 - ST2084 (1000 nits)"
```

**Current Limitations:**
- CLI `vfx aces` only provides sRGB output
- `vfx color --from/--to` does gamut conversion, not full ODT
- HDR ODTs (PQ, HLG, Rec.2020) require OCIO or Rust API

## ODT Components

### 1. Inverse RRT Tonescale Limit

The ODT limits the tonescale to display capabilities:

```
RRT Output → ODT Limit
   100     →   1.0 (display white)
    50     →   0.9
    10     →   0.7
     1     →   0.18
   0.1     →   0.05
```

### 2. Gamut Mapping

Colors outside display gamut are compressed:

```
OCES Gamut (very wide)
     ●─────────────────────●
    /                       \
   /   Display Gamut         \
  /      ●─────────●          \
 /      /           \          \
●──────●─────────────●──────────●
       └─ Colors compressed to fit
```

### 3. Surround Compensation

Adjusts for viewing environment:
- **Dim surround**: Typical cinema
- **Dark surround**: Home theater
- **Average surround**: Office, daylight

### 4. Display EOTF

Applies the display's electro-optical transfer function:

| Display | EOTF |
|---------|------|
| sRGB | ~2.2 gamma (piecewise) |
| Rec.709 | 2.4 gamma |
| PQ (ST2084) | Perceptual quantizer |
| HLG | Hybrid log-gamma |

## White Point Considerations

### D65 vs D60

ACES uses D60 white point (slightly warmer than D65):

```
D65 (x=0.3127, y=0.3290) - Typical daylight
D60 (x=0.3217, y=0.3378) - ACES reference
```

**Options:**
1. **D60 simulation ODT**: Maintains ACES white on D65 display
2. **D65 ODT**: Chromatic adaptation to display white

### Which to Choose?

| Scenario | Recommendation |
|----------|----------------|
| Matching to ACES reference | D60 sim ODT |
| General viewing | D65 ODT |
| Mixing with D65 content | D65 ODT |

## Creating Multiple Deliverables

From a single ACEScg master:

```bash
# Web (sRGB)
vfx aces master.exr -o web.jpg -t rrt-odt

# Broadcast (Rec.709)
vfx color master.exr -o broadcast.dpx \
    --from ACEScg --to "Rec.709 - Display"

# Cinema (DCI-P3)
vfx color master.exr -o cinema.tiff \
    --from ACEScg --to "DCI-P3 - Display"

# HDR (Rec.2020 PQ)
vfx color master.exr -o hdr.exr \
    --from ACEScg --to "Rec.2020 - ST2084"
```

## Inverse ODT

For ingesting graded footage back to ACES:

```bash
# Inverse ODT (display-referred → ACEScg)
vfx color display_footage.dpx -o acescg.exr \
    --from "Rec.709 - Display" \
    --to ACEScg
```

**Caution**: Inverse transforms can introduce artifacts. Prefer working from original ACEScg when possible.
