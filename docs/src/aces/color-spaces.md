# ACES Color Encoding Spaces

ACES defines several color encoding spaces, each optimized for specific tasks in the pipeline.

## ACES2065-1 (AP0)

The **interchange and archival** color space.

### Primaries (AP0)
```
         Red         Green       Blue
x        0.7347      0.0000      0.0001
y        0.2653      1.0000     -0.0770
```

### Characteristics
- **Gamut**: Covers all visible colors
- **White Point**: D60 (x=0.32168, y=0.33767)
- **Transfer**: Linear (scene-referred)
- **Bit Depth**: 16-bit half-float minimum

### When to Use
- File interchange between facilities
- Long-term archival
- Color grading handoffs

### Example
```rust
use vfx_primaries::ACES_AP0;

let ap0 = ACES_AP0;  // Module-level constant
// Primaries encompass the entire visible spectrum
```

## ACEScg (AP1)

The **working space** for CGI and compositing.

### Primaries (AP1)
```
         Red         Green       Blue
x        0.713       0.165       0.128
y        0.293       0.830       0.044
```

### Characteristics
- **Gamut**: Very wide but practical
- **White Point**: D60 (same as AP0)
- **Transfer**: Linear (scene-referred)
- **Bit Depth**: 16-bit half-float minimum

### Why AP1 for CGI?
- No imaginary colors (all primaries visible)
- Minimizes negative RGB values in normal images
- Math-friendly for compositing operations

### Example
```rust
use vfx_primaries::ACES_AP1;

let acescg = ACES_AP1;  // Module-level constant
// Practical working space for VFX
```

## ACEScc

**Logarithmic encoding** for color grading.

### Transfer Function
```
ACEScc = (log2(x) + 9.72) / 17.52    for x >= 2^-15
       = (log2(2^-16 + x*0.5) + 9.72) / 17.52    for x < 2^-15
```

### Characteristics
- **Range**: [0, 1] for practical values
- **Primaries**: AP1
- **Mid-gray (0.18)**: ~0.4135

### When to Use
- Color grading in DaVinci, Baselight
- Perceptually uniform adjustments
- Lift/Gamma/Gain operations

## ACEScct

**Log encoding with toe** for color grading.

### Difference from ACEScc
ACEScct adds a linear portion below 0.0078125:

```
ACEScct = 10.5402377416545 * x + 0.0729055341958355    for x <= 0.0078125
        = (log2(x) + 9.72) / 17.52                      for x > 0.0078125
```

### Why the Toe?
- Prevents values going to negative infinity in shadows
- More similar to traditional film density
- Preferred by many colorists

## ACESproxy

**Integer encoding** for on-set monitoring.

### Characteristics
- 10-bit or 12-bit integer encoding
- Real-time viewable on standard monitors
- Not for archival or delivery

### Use Case
- Camera monitor display
- Live grading preview
- On-set look application

## Comparison Chart

| Space | Primaries | Transfer | Use Case |
|-------|-----------|----------|----------|
| ACES2065-1 | AP0 | Linear | Archive, Interchange |
| ACEScg | AP1 | Linear | CGI, Compositing |
| ACEScc | AP1 | Log | Color Grading |
| ACEScct | AP1 | Log+Toe | Color Grading |
| ACESproxy | AP1 | Log | Monitoring |

## Gamut Visualization

```
                    ▲ y
                    │
                 Green (AP0)
                    ●
                   /│\
                  / │ \
            Green(AP1)
                ●   │   \
               /    │    \
              /  ───┼───  \
    Blue(AP0)● ─────●─────● Red(AP0)
              ╲  Visible  /
               ╲ Gamut   /
                ╲       /
                 ●─────●
              Blue(AP1) Red(AP1)
                    │
                    └───────────▶ x
```

AP0 (dashed) encompasses all visible colors.
AP1 (solid) is smaller but has no imaginary primaries.
