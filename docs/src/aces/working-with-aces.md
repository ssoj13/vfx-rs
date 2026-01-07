# Working with ACES

Practical guide to using ACES transforms in your VFX workflow.

## Transform Overview

| Transform | Input | Output | Purpose |
|-----------|-------|--------|---------|
| **IDT** | Camera native | ACES2065/ACEScg | Input conversion |
| **RRT** | ACES (scene) | OCES (display) | Tonemapping |
| **ODT** | OCES | Display encoding | Output conversion |
| **LMT** | ACES | ACES | Creative look |

## The Standard Workflow

### Step 1: Ingest (IDT)

Convert camera footage to ACEScg:

```bash
# sRGB input (CG renders, graphics)
vfx aces render.png -o render_acescg.exr -t idt

# The IDT linearizes and converts to ACEScg
# Input: sRGB gamma-encoded → Output: ACEScg linear
```

### Step 2: Process

All work happens in linear ACEScg:

```bash
# Compositing
vfx composite fg.exr bg.exr -o comp.exr --mode over

# Color corrections
vfx color comp.exr -o graded.exr --exposure 0.5

# Filters
vfx blur input.exr -o soft.exr -r 3
```

### Step 3: Output (RRT + ODT)

Convert to display-ready format:

```bash
# Full output transform to sRGB
vfx aces graded.exr -o final.png -t rrt-odt

# This applies RRT (tonemap) + ODT (sRGB encoding)
```

## Understanding Each Transform

### IDT (Input Device Transform)

**Purpose**: Convert any input to ACES working space

**What it does**:
1. Linearize (undo gamma/log encoding)
2. Convert color primaries to AP1/AP0
3. Adapt white point to D60

**Common IDTs**:
```
sRGB           → ACEScg
Rec.709        → ACEScg
ARRI LogC      → ACEScg
Sony S-Log3    → ACEScg
RED Log3G10    → ACEScg
```

### RRT (Reference Rendering Transform)

**Purpose**: Map scene-referred to display-referred

**What it does**:
1. Global desaturation of extreme values
2. Filmic S-curve tonemapping
3. Highlight rolloff (no hard clip)
4. Shadow toe (lifted blacks)

**Characteristics**:
- Fixed algorithm, same everywhere
- No user parameters
- Produces OCES (Output Color Encoding Specification)

### ODT (Output Device Transform)

**Purpose**: Convert OCES to specific display

**What it does**:
1. Gamut mapping to display gamut
2. Apply display gamma/EOTF
3. Scale to display peak luminance

**Common ODTs**:
| ODT | Use Case |
|-----|----------|
| sRGB | Computer monitors, web |
| Rec.709 | HD broadcast TV |
| Rec.709 (D60 sim) | HD with ACES white |
| DCI-P3 | Digital cinema |
| Rec.2020 PQ 1000 nits | HDR10 delivery |

## Practical Examples

### Web Delivery
```bash
# Input: ACEScg EXR
# Output: sRGB JPEG for web
vfx aces hero_shot.exr -o web_hero.jpg -t rrt-odt
vfx convert web_hero.jpg -o web_hero.jpg -q 85
```

### Broadcast Delivery
```bash
# Input: ACEScg EXR
# Output: Rec.709 DPX for broadcast
vfx aces master.exr -o broadcast.dpx -t rrt-odt
```

### HDR Delivery
```bash
# Input: ACEScg EXR
# Output: Rec.2020 PQ for HDR10
# (requires OCIO config with HDR ODT)
vfx color master.exr -o hdr.exr --from ACEScg --to "Rec.2020 - PQ"
```

### Archive
```bash
# Input: ACEScg working files
# Output: ACES2065-1 for archive
vfx color final_acescg.exr -o archive.exr --from ACEScg --to ACES2065-1
```

## Handling Different Sources

### CG Renders
Most renderers output linear (already suitable for ACES):
```bash
# If render is in sRGB primaries (linear)
vfx aces render.exr -o render_acescg.exr -t idt

# If render is already ACEScg, no conversion needed
```

### Stock Footage
Usually sRGB or Rec.709 encoded:
```bash
# sRGB stock footage
vfx aces stock.jpg -o stock_acescg.exr -t idt
```

### Camera RAW
Use camera-specific IDT via OCIO:
```bash
# ARRI footage with proper OCIO config
export OCIO=/path/to/aces_config.ocio
vfx color arri.dpx -o acescg.exr --from "ARRI LogC" --to ACEScg
```

## Common Issues and Solutions

### Issue: Image looks too dark after RRT
**Cause**: Input already display-referred
**Solution**: Don't apply IDT to pre-graded footage

### Issue: Colors look desaturated
**Cause**: RRT naturally desaturates for filmic look
**Solution**: This is intentional; grade in ACEScg before RRT

### Issue: Clipped highlights
**Cause**: Input already clipped before ACES
**Solution**: Use higher bit-depth source, or work in log space

### Issue: Skin tones shifted
**Cause**: White point adaptation (D65→D60)
**Solution**: Use appropriate IDT; this is expected behavior
