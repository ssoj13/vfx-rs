# aces - ACES Workflow

Apply ACES (Academy Color Encoding System) transforms for film/TV production.

## Usage

```bash
vfx aces [OPTIONS] <INPUT> -o <OUTPUT>
```

## Options

| Option | Description |
|--------|-------------|
| `-t, --transform <T>` | Transform to apply |
| `--rrt <VARIANT>` | RRT variant (default, alt1, filmic) |

## Transforms

| Transform | Aliases | Description |
|-----------|---------|-------------|
| `idt` | `input`, `srgb-to-acescg` | sRGB → ACEScg (Input Device Transform) |
| `rrt` | `tonemap` | Reference Rendering Transform (tonemap) |
| `odt` | `output`, `acescg-to-srgb` | ACEScg → sRGB (Output Device Transform) |
| `rrt-odt` | `display`, `full` | Combined RRT+ODT for display |

## ACES Pipeline

```
Scene Linear → IDT → ACEScg → RRT → ODT → Display
```

## Examples

```bash
# sRGB input to ACEScg working space
vfx aces input.jpg -o working.exr -t idt

# Apply tonemapping only (already in ACEScg)
vfx aces acescg.exr -o tonemapped.exr -t rrt

# ACEScg to sRGB display (no tonemap)
vfx aces acescg.exr -o output.png -t odt

# Full display transform (RRT + ODT)
vfx aces render.exr -o display.png -t rrt-odt

# Alternative RRT curve (less contrast)
vfx aces render.exr -o alt.png -t rrt-odt --rrt alt1
```

## RRT Variants

| Variant | Description |
|---------|-------------|
| `default` | Standard ACES RRT |
| `alt1` | Reduced highlight rolloff |
| `filmic` | S-curve with toe/shoulder |

## Workflow Example

```bash
# Typical VFX pipeline
# 1. Import camera footage (sRGB → ACEScg)
vfx aces plate.jpg -o plate_acescg.exr -t idt

# 2. Composite in ACEScg (linear, wide gamut)
vfx composite fg.exr plate_acescg.exr -o comp.exr

# 3. Render to display
vfx aces comp.exr -o review.png -t rrt-odt
```
