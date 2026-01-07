# Understanding ACES

This section provides the theoretical foundation for understanding ACES color management.

## Core Concepts

### Scene-Referred vs Display-Referred

**Scene-Referred** (ACES working spaces):
- Values represent actual scene light ratios
- 0.18 = 18% gray (middle gray)
- Values can exceed 1.0 (highlights, lights)
- No clipping, no black crush

**Display-Referred** (after RRT/ODT):
- Values represent display output
- 0.0 = display black, 1.0 = display white
- Encoded for specific display technology

### Why This Matters

```
Scene-Referred:                  Display-Referred:
Sun at 10000 nits               Display white at 100 nits
     ▼                                  ▼
     ●───────────────────▶             ●
     │    Preserve all               /  ╲
     │    information              /    ╲  Compressed
     │                           /      ╲  to display
     ○ Mid-gray 18%            ●        ●  range
     │                          ╲      /
     │                           ╲    /
     ▼                            ╲  /
Deep shadows                       ●
                              Display black
```

### Dynamic Range

ACES preserves the full dynamic range of the scene:

| Content | Stops Above Mid-Gray |
|---------|---------------------|
| Mid-gray (18%) | 0 |
| White paper | +2.5 |
| Bright sky | +6 |
| Sun | +16 |
| Specular highlight | +20 or more |

All of this is preserved in ACES and compressed appropriately by RRT/ODT.

## The Math Behind ACES

### Linear Light

In ACES working spaces, values are linear:
- Doubling the value = doubling the light
- 0.36 is twice as bright as 0.18
- This makes compositing mathematically correct

```rust
// Correct linear blend (in ACEScg)
let blended = fg * alpha + bg * (1.0 - alpha);

// Would be wrong in gamma-encoded space!
```

### Matrix Transforms

Converting between color spaces uses 3x3 matrices:

```
┌Rout┐   ┌m00 m01 m02┐   ┌Rin┐
│Gout│ = │m10 m11 m12│ × │Gin│
└Bout┘   └m20 m21 m22┘   └Bin┘
```

Example: sRGB to ACEScg matrix:
```
┌ 0.6131  0.3395  0.0474┐
│ 0.0702  0.9164  0.0134│
└ 0.0206  0.1096  0.8698┘
```

### Chromatic Adaptation

ACES uses D60 white point, different from sRGB (D65).
Bradford chromatic adaptation converts between them:

```
XYZ_D60 = Bradford × XYZ_D65
```

This ensures white remains white across different illuminants.

## Key Terminology

| Term | Definition |
|------|------------|
| **AP0** | ACES Primaries 0 - ultra-wide gamut for archival |
| **AP1** | ACES Primaries 1 - wide gamut for working |
| **OCES** | Output Color Encoding Specification (after RRT) |
| **CTL** | Color Transformation Language (transform scripts) |
| **AMF** | ACES Metadata File (project color settings) |
| **CLF** | Common LUT Format (ACES-compatible LUT format) |

## Common Misconceptions

### "ACES is just a LUT"
**False.** ACES is a complete color management system with defined transforms, not just a look.

### "ACES makes everything orange and teal"
**False.** The default ACES look is neutral. The "ACES look" people describe is just one possible output.

### "ACES clips highlights"
**False.** ACES preserves all highlights in the working space. The RRT does compress them for display, but the data is never lost in ACEScg.

### "I need ACES for a professional workflow"
**Partially true.** ACES is an excellent choice for complex productions, but simpler workflows can succeed with proper color management using other methods.

## When to Use ACES

**Good fit:**
- Multi-camera productions
- VFX-heavy projects
- Long-term archival needs
- HDR delivery requirements
- Facility-to-facility handoffs

**May not need:**
- Single-camera simple shoots
- Quick turnaround projects
- Legacy equipment constraints
- Team unfamiliar with ACES
