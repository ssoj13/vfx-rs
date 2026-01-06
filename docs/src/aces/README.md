# ACES VFX Workflows

ACES (Academy Color Encoding System) is the industry-standard color management framework for motion pictures and high-end visual effects. This section provides both theoretical understanding and practical implementation in vfx-rs.

## What You'll Learn

- **Understanding ACES** - The theory behind the color pipeline
- **Working with ACES** - Practical transform application
- **ACES in vfx-rs** - Implementation details and API usage

## Quick Overview

ACES provides a unified color workflow:

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Camera    │ IDT │   ACES2065  │ RRT │   OCES      │ ODT │   Display   │
│   Input     │────▶│   (AP0)     │────▶│   Output    │────▶│   Output    │
└─────────────┘     └─────────────┘     └─────────────┘     └─────────────┘
                           │
                           ▼ (Working)
                    ┌─────────────┐
                    │   ACEScg    │
                    │   (AP1)     │
                    └─────────────┘
```

## Key Benefits

1. **Universal Interchange** - ACES2065-1 works across all facilities
2. **Future-Proof Archive** - Scene-referred, high dynamic range storage
3. **Consistent Look** - Same transforms yield identical results everywhere
4. **Wide Gamut** - Preserves all capturable colors

## Getting Started

```bash
# Convert camera footage to ACEScg
vfx aces camera.dpx -o working.exr -t idt

# Apply ACES output transform for sRGB viewing
vfx aces working.exr -o preview.png -t rrt-odt
```

## Chapters

1. [Understanding ACES](./understanding-aces.md) - Theory and concepts
2. [Working with ACES](./working-with-aces.md) - Practical transforms
3. [ACES in vfx-rs](./vfx-rs-aces.md) - Implementation details
