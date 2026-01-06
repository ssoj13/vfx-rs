# Input Transforms (IDT)

Input Device Transforms convert camera-specific color encoding to ACES.

## What IDT Does

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Camera    │     │  Linearize  │     │   Convert   │
│  Encoding   │────▶│  (OETF^-1)  │────▶│  to AP0/AP1 │
└─────────────┘     └─────────────┘     └─────────────┘
     LogC              Linear             ACEScg
     S-Log3           (Camera)            (ACES)
     REDlog
```

## vfx-rs IDT Implementation

The `vfx aces` command with `-t idt` converts sRGB to ACEScg:

```bash
vfx aces input.jpg -o output.exr -t idt
```

### Under the Hood

```rust
use vfx_color::aces::{srgb_to_acescg, linearize_srgb};

// 1. Linearize sRGB
let linear = linearize_srgb(pixel);

// 2. Matrix transform to ACEScg
let acescg = srgb_to_acescg(linear);
```

## Common Camera IDTs

### ARRI (LogC3/LogC4)

LogC encoding characteristics:
- **Log base**: Log-C (not log10 or ln)
- **Mid-gray**: 0.391 (LogC3), 0.278 (LogC4)
- **Primaries**: ARRI Wide Gamut 3/4

```
LogC3 → Linear:
x = (10^((LogC - 0.385537) / 0.247189) - 0.052272) / 5.555556
    for LogC > 0.1496...
```

### Sony (S-Log2/S-Log3)

S-Log3/S-Gamut3 encoding:
- **Mid-gray**: 0.406 (S-Log3)
- **Primaries**: S-Gamut3, S-Gamut3.Cine

```
S-Log3 → Linear:
x = (10^((S-Log3 - 0.4105571850) / 0.255620723) - 0.0526315789) / 5.26315789
    for S-Log3 >= 0.1673609920
```

### RED (Log3G10)

REDWideGamutRGB Log3G10:
- **Mid-gray**: 0.333 (Log3G10)
- **Primaries**: REDWideGamutRGB

```
Log3G10 → Linear:
x = (10^(Log3G10 / 0.224282) - 1) / 155.975327
    for Log3G10 > 0
```

### Blackmagic (BMD Film Gen5)

Blackmagic Design Film:
- **Mid-gray**: 0.38
- **Primaries**: Blackmagic Wide Gamut

## vfx-rs Transfer Functions

The `vfx-transfer` crate implements these camera curves:

```rust
use vfx_transfer::{TransferFunction, logc3_to_linear, slog3_to_linear};

// ARRI LogC3
let linear = logc3_to_linear(logc_value);

// Sony S-Log3
let linear = slog3_to_linear(slog_value);
```

### Supported Transfer Functions

| Function | Encode | Decode | Description |
|----------|--------|--------|-------------|
| `srgb` | Yes | Yes | sRGB (IEC 61966-2-1) |
| `gamma` | Yes | Yes | Pure gamma (configurable) |
| `rec709` | Yes | Yes | Rec.709 OETF/EOTF |
| `pq` | Yes | Yes | SMPTE ST 2084 (HDR10) |
| `hlg` | Yes | Yes | Hybrid Log-Gamma (HLG) |
| `logc3` | Yes | Yes | ARRI LogC3 |
| `logc4` | Yes | Yes | ARRI LogC4 |
| `slog2` | Yes | Yes | Sony S-Log2 |
| `slog3` | Yes | Yes | Sony S-Log3 |
| `vlog` | Yes | Yes | Panasonic V-Log |
| `acescct` | Yes | Yes | ACEScct (log with toe) |
| `acescc` | Yes | Yes | ACEScc (pure log) |
| `redlog` | Yes | Yes | RED Log3G10 |
| `bmdfilm` | Yes | Yes | Blackmagic Film Gen5 |

## Creating Custom IDT

For cameras not in the standard library, use OCIO:

```bash
# Set OCIO config with camera IDT
export OCIO=/path/to/studio_config.ocio

# Use color space conversion
vfx color camera.dpx -o acescg.exr \
    --from "Camera - LogC4" \
    --to "ACES - ACEScg"
```

Or create a CLF (Common LUT Format) transform:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<ProcessList id="MyCamera_to_ACES">
  <Log inBitDepth="32f" outBitDepth="32f" style="cameraLogToLin">
    <LogParams base="10" ... />
  </Log>
  <Matrix inBitDepth="32f" outBitDepth="32f">
    <Array dim="3 3 3">
      0.7006 0.1488 0.1506
      0.0455 0.8602 0.0943
      -0.0392 0.0102 1.0290
    </Array>
  </Matrix>
</ProcessList>
```

## Best Practices

1. **Use manufacturer IDTs** - Camera manufacturers provide tested transforms
2. **Match camera settings** - Different ISO/WB may need different IDTs
3. **Document your pipeline** - Record which IDT version used
4. **Test with gray card** - Verify 18% gray maps to 0.18 in ACEScg
5. **Preserve original** - Keep camera RAW, apply IDT non-destructively
