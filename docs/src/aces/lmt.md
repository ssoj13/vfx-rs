# Look Modification Transforms (LMT)

LMTs are creative transforms applied in ACES space before the RRT/ODT output chain.

## Purpose

LMTs allow creative looks while preserving ACES color management:

```
     ACEScg          LMT           ACEScg          RRT+ODT        Display
   (Working)    (Creative)       (Styled)        (Output)        (Final)
┌───────────┐   ┌─────────┐   ┌───────────┐   ┌───────────┐   ┌───────────┐
│           │──▶│  Film   │──▶│           │──▶│           │──▶│           │
│   Linear  │   │ Look LUT│   │  Linear   │   │  Tonemap  │   │  Encoded  │
│           │   │         │   │  Styled   │   │           │   │           │
└───────────┘   └─────────┘   └───────────┘   └───────────┘   └───────────┘
```

## LMT Types

### Film Emulation LMTs

Emulate film print stocks:

| LMT | Emulates |
|-----|----------|
| Kodak 2383 | Standard print film |
| Fuji 3510 | Japanese film stock |
| Kodak 2393 | Intermediate film |

### Creative LMTs

Stylistic modifications:
- **Day for Night**: Blue push, underexposure
- **Bleach Bypass**: Desaturated, high contrast
- **Cross Process**: Color shifts

### Show-Specific LMTs

Custom looks for productions:
- Defined by cinematographer/colorist
- Consistent across all shots
- Shared via CLF or OCIO config

## Applying LMTs in vfx-rs

### Using LUT Files

```bash
# Apply .cube LUT as LMT
vfx lut input.exr -o styled.exr -l film_look.cube

# Apply CLF (Common LUT Format)
vfx lut input.exr -o styled.exr -l look_transform.clf
```

### Using OCIO Looks

```bash
export OCIO=/path/to/aces_config.ocio

# Apply named look from config
vfx color input.exr -o styled.exr \
    --from ACEScg \
    --to ACEScg \
    --look "Film Emulation"
```

### In the Full Pipeline

```bash
# 1. Work in ACEScg
vfx composite fg.exr bg.exr -o comp.exr --mode over

# 2. Apply creative LMT
vfx lut comp.exr -o styled.exr -l show_lmt.cube

# 3. Output with RRT+ODT
vfx aces styled.exr -o final.png -t rrt-odt
```

## Creating LMTs

### LMT Requirements

1. **Input**: ACEScg (AP1, linear)
2. **Output**: ACEScg (AP1, linear)
3. **Preserves**: Dynamic range, gamut
4. **Applies**: Creative color modifications

### Basic LMT Structure

```
Input (ACEScg)
    │
    ▼
┌─────────────────┐
│ Color Matrix    │  Primary adjustments
└─────────────────┘
    │
    ▼
┌─────────────────┐
│ 3D LUT          │  Non-linear color
└─────────────────┘
    │
    ▼
┌─────────────────┐
│ Curves          │  Tonal adjustments
└─────────────────┘
    │
    ▼
Output (ACEScg)
```

### CLF Format Example

```xml
<?xml version="1.0" encoding="UTF-8"?>
<ProcessList id="FilmLook_LMT" compCLFversion="3.0">
  <Description>Film emulation LMT for show X</Description>

  <!-- Primary color adjustments -->
  <Matrix>
    <Array dim="3 3">
      1.1 -0.05 -0.05
      -0.02 1.05 -0.03
      0.0 -0.1 1.1
    </Array>
  </Matrix>

  <!-- Saturation curve -->
  <Range inBitDepth="32f" outBitDepth="32f">
    <minInValue>0</minInValue>
    <maxInValue>65504</maxInValue>
  </Range>

  <!-- Creative 3D LUT -->
  <LUT3D interpolation="tetrahedral">
    <Array dim="17 17 17 3">
      <!-- LUT data -->
    </Array>
  </LUT3D>
</ProcessList>
```

## LMT Best Practices

### Do:
- Apply LMT in ACEScg (before RRT)
- Keep originals without LMT applied
- Document LMT settings for archive
- Use 3D LUTs for complex color changes
- Test LMT across wide range of content

### Don't:
- Apply LMT to display-referred footage
- Destructively bake LMT into master files
- Use LMT to fix technical color problems
- Assume LMT works the same on all monitors

## LMT in Color Pipeline

```
┌──────────────────────────────────────────────────────────────────────┐
│                        ACES Production Pipeline                      │
├──────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  Camera ──IDT──▶ ACEScg ──────────────▶ Archive (ACES2065-1)        │
│                    │                                                 │
│                    │                                                 │
│                    ▼                                                 │
│               ┌─────────┐                                           │
│               │   LMT   │ ◀── Film Look / Creative Style            │
│               └────┬────┘                                           │
│                    │                                                 │
│                    ▼                                                 │
│               ┌─────────┐                                           │
│               │   RRT   │                                           │
│               └────┬────┘                                           │
│                    │                                                 │
│          ┌─────────┼─────────┐                                      │
│          ▼         ▼         ▼                                      │
│      ┌──────┐  ┌──────┐  ┌──────┐                                  │
│      │ ODT  │  │ ODT  │  │ ODT  │                                  │
│      │ sRGB │  │ Rec709│ │ HDR  │                                  │
│      └──────┘  └──────┘  └──────┘                                  │
│          │         │         │                                      │
│          ▼         ▼         ▼                                      │
│        Web     Broadcast   HDR10                                    │
│                                                                      │
└──────────────────────────────────────────────────────────────────────┘
```

The LMT ensures all outputs share the same creative look.
