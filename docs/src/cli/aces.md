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
| `--rrt <VARIANT>` | RRT variant (default, high-contrast, filmic, alt1) |

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

# High contrast RRT curve
vfx aces render.exr -o punchy.png -t rrt-odt --rrt high-contrast

# Filmic look (softer highlights)
vfx aces render.exr -o filmic.png -t rrt-odt --rrt filmic
```

## RRT Variants

| Variant | Description |
|---------|-------------|
| `default` | Standard ACES RRT |
| `high-contrast` | Higher contrast curve with punchier midtones |
| `filmic` | Softer shoulder, more film-like rolloff |
| `alt1` | Neutral balanced response, smooth transitions |

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
