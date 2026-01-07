# What is ACES?

ACES (Academy Color Encoding System) is a free, open, device-independent color management and image interchange framework developed by the Academy of Motion Picture Arts and Sciences (AMPAS).

## The Problem ACES Solves

Before ACES, every camera manufacturer, post-production facility, and display had its own color encoding. This created significant problems:

- **Camera A → Facility B → Display C** required custom color pipelines
- Different vendors produced different results from the same source
- Archiving was problematic as display technology evolved
- Matching footage from multiple cameras was difficult

## The ACES Solution

ACES provides a standardized color pipeline:

### Scene-Referred Encoding

ACES stores colors relative to the original scene, not the display:

```
Scene Light → ACES Value
0.18 (18% gray) → 0.18 in ACES
1.0 (white paper) → ~1.0 in ACES
100+ (bright lights) → 100+ in ACES (no clipping!)
```

This means:
- **No clipping** - Bright lights, specular highlights preserved
- **No black crush** - Deep shadows maintain detail
- **No gamut limits** - All visible colors can be represented

### Standard Transforms

Instead of custom conversions, ACES defines standard transforms:

| Transform | Purpose |
|-----------|---------|
| **IDT** (Input Device Transform) | Camera → ACES |
| **LMT** (Look Modification Transform) | Creative look |
| **RRT** (Reference Rendering Transform) | Tonemap for display |
| **ODT** (Output Device Transform) | ACES → Display |

### Wide Gamut Storage

ACES uses two main color spaces:

**ACES2065-1 (AP0)**
- Interchange and archival format
- Covers all visible colors (and more)
- Primaries outside the visible spectrum

**ACEScg (AP1)**
- Working space for CGI
- Practical gamut with no negative RGB values
- Optimized for compositing math

## ACES vs Traditional Workflows

### Traditional Workflow
```
Camera RAW → Camera LUT → Rec.709 → Display
                ↓
         (Information lost)
```

### ACES Workflow
```
Camera RAW → IDT → ACES (full dynamic range) → RRT → ODT → Display
                          ↓
                   (Archives everything)
```

## Real-World Benefits

1. **Multi-Camera Productions**
   - Mix ARRI, RED, Sony footage seamlessly
   - Each camera has a manufacturer-provided IDT

2. **VFX Integration**
   - CG renders in ACEScg match live-action
   - Consistent color math across applications

3. **HDR Delivery**
   - Archive once in ACES
   - Deliver to SDR, HDR10, Dolby Vision from same master

4. **Long-Term Archive**
   - ACES is display-independent
   - Future displays get new ODTs, archive unchanged
