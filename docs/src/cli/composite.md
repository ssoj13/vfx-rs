# composite - Image Compositing

Layer images using Porter-Duff operations and blend modes.

## Usage

```bash
vfx composite [OPTIONS] <FG> <BG> -o <OUTPUT>
```

## Options

| Option | Description |
|--------|-------------|
| `-m, --mode <MODE>` | Blend mode (default: over) |

## Blend Modes

| Mode | Description |
|------|-------------|
| `over` | Standard A over B (Porter-Duff) |
| `multiply` | Darken (A × B) |
| `screen` | Lighten (1 - (1-A)(1-B)) |
| `add` | Linear dodge (A + B) |

**Note:** `subtract`, `overlay`, `softlight`, `hardlight`, and `difference` modes are **not yet implemented**.

## Examples

```bash
# Standard over composite
vfx composite fg.exr bg.exr -o result.exr

# Multiply (shadows/dirt)
vfx composite dirt.exr clean.exr -o dirty.exr -m multiply

# Screen (glow/light)
vfx composite glow.exr base.exr -o lit.exr -m screen

# Add (lens flare)
vfx composite flare.exr plate.exr -o result.exr -m add
```

## Alpha Handling

- Both images must be RGBA (4 channels)
- Alpha is used for `over` mode blending
- Other modes typically ignore alpha

## Notes

- Processing is done on CPU
- Output is always float32
