# The ACES Pipeline

The ACES pipeline is a series of transforms that convert image data from camera input to display output while preserving maximum quality.

## Pipeline Overview

```
┌────────────────────────────────────────────────────────────────────────┐
│                         ACES PIPELINE                                  │
├────────────────────────────────────────────────────────────────────────┤
│                                                                        │
│  ┌──────────┐    ┌──────────┐    ┌──────────┐    ┌──────────┐         │
│  │  Camera  │    │  ACES    │    │  Working │    │  Archive │         │
│  │  Native  │───▶│  2065-1  │───▶│  (ACEScg)│───▶│  Master  │         │
│  │          │ IDT│  (AP0)   │    │  (AP1)   │    │  (AP0)   │         │
│  └──────────┘    └──────────┘    └──────────┘    └──────────┘         │
│       ▲                               │                                │
│       │                               │ LMT (optional)                 │
│       │                               ▼                                │
│  ┌──────────┐    ┌──────────┐    ┌──────────┐    ┌──────────┐         │
│  │  Display │◀───│   ODT    │◀───│   RRT    │◀───│  Graded  │         │
│  │  Output  │    │ (to sRGB)│    │ (tonemap)│    │  Image   │         │
│  └──────────┘    └──────────┘    └──────────┘    └──────────┘         │
│                                                                        │
└────────────────────────────────────────────────────────────────────────┘
```

## Stage 1: Input Device Transform (IDT)

The IDT converts camera-native encoding to ACES.

### What IDT Does
1. **Decode log/gamma** - Linearize the camera encoding
2. **Apply color matrix** - Convert camera primaries to AP0
3. **White balance** - Adjust to ACES D60 white point

### IDT Examples
| Camera | IDT Name |
|--------|----------|
| ARRI Alexa | ARRI LogC4 to ACES |
| RED V-Raptor | RED Log3G10 to ACES |
| Sony Venice | S-Log3/S-Gamut3 to ACES |
| Blackmagic | BMD Film Gen5 to ACES |
| sRGB (graphics) | sRGB to ACES |

### vfx-rs IDT Example
```bash
# Convert sRGB image to ACEScg
vfx aces input.jpg -o working.exr -t idt
```

## Stage 2: Working Space Processing

All VFX work happens in ACEScg (AP1 linear).

### Why ACEScg?
- **Linear math** - Compositing works correctly
- **Wide gamut** - No clipping of saturated colors
- **Practical primaries** - Minimal negative RGB values

### Operations in ACEScg
```bash
# These all work in linear ACEScg
vfx composite fg.exr bg.exr -o result.exr
vfx blur input.exr -o output.exr -r 5
vfx color input.exr -o output.exr --exposure 1.5
```

## Stage 3: Look Modification Transform (LMT)

Optional creative looks applied before output.

### LMT Use Cases
- Film emulation (Kodak 2383, Fuji 3510)
- Show-specific looks
- Day-for-night effects
- Desaturation / stylization

### LMT in vfx-rs
```bash
# Apply LUT as creative look
vfx lut input.exr -o styled.exr -l film_look.cube
```

## Stage 4: Reference Rendering Transform (RRT)

The RRT converts scene-referred ACES to display-referred OCES.

### What RRT Does
1. **Tone mapping** - Compress high dynamic range
2. **Gamut mapping** - Fit colors to displayable range
3. **Filmic response** - Highlight rolloff, shadow toe

### RRT Characteristics
- **Input**: Scene-linear (0 to ∞)
- **Output**: Display-referred (0 to ~16 nits)
- **Fixed algorithm**: Same everywhere

### The ACES Tone Curve
```
Output │                          ___________
       │                      ___/
       │                  ___/
       │              ___/      ← Shoulder (highlight rolloff)
       │          ___/
       │      ___/
       │  ___/                  ← Linear mid-section
       │_/                      ← Toe (shadow lift)
       └─────────────────────────────────────── Input
       0                                        ∞
```

### vfx-rs RRT Example
```bash
# Apply only RRT (for custom ODT later)
vfx aces linear.exr -o tonemapped.exr -t rrt
```

## Stage 5: Output Device Transform (ODT)

The ODT converts OCES to specific display encoding.

### Common ODTs
| ODT | Target Display |
|-----|----------------|
| sRGB | Computer monitors |
| Rec.709 | HD broadcast |
| Rec.2020 ST2084 (PQ) | HDR10 TVs |
| DCI-P3 D65 | Digital cinema |
| P3-D65 ST2084 | HDR Dolby Vision |

### vfx-rs RRT+ODT Example
```bash
# Full output transform to sRGB
vfx aces working.exr -o final.png -t rrt-odt
```

## Complete Pipeline Example

```bash
# 1. Camera footage to ACEScg (IDT)
vfx aces camera_log.dpx -o working.exr -t idt

# 2. Process in linear (compositing, grading)
vfx composite fg.exr working.exr -o comp.exr --mode over
vfx color comp.exr -o graded.exr --exposure 0.5 --saturation 1.1

# 3. Apply creative LUT (LMT)
vfx lut graded.exr -o look.exr -l film_emulation.cube

# 4. Output transform (RRT + ODT)
vfx aces look.exr -o final_srgb.png -t rrt-odt

# For HDR delivery, use different ODT:
# vfx aces look.exr -o final_hdr.exr -t rrt-odt --odt rec2020_pq
```

## Pipeline Diagram with Color Spaces

```
Camera        IDT         Working        LMT          RRT           ODT         Display
┌─────┐    ┌─────┐     ┌─────────┐    ┌─────┐     ┌─────────┐    ┌─────┐     ┌─────────┐
│LogC │───▶│ARRI │────▶│ ACEScg  │───▶│Look │────▶│Tonemap  │───▶│sRGB │────▶│ Monitor │
│S-Log│───▶│Sony │     │  (AP1)  │    │(opt)│     │  (OCES) │    │Rec709    │ Display │
│RED  │───▶│RED  │     │ Linear  │    │     │     │         │    │HDR10│     │         │
└─────┘    └─────┘     └─────────┘    └─────┘     └─────────┘    └─────┘     └─────────┘
                            │
                            ▼
                       ┌─────────┐
                       │ Archive │
                       │ACES2065 │
                       │  (AP0)  │
                       └─────────┘
```
