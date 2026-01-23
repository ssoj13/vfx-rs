# extract-layer - Layer Extraction

Extract a single layer from multi-layer EXR files.

**Alias:** `xl`

## Synopsis

```bash
vfx extract-layer <INPUT> -o <OUTPUT> [-l <LAYER>]
```

## Options

| Option | Description |
|--------|-------------|
| `-o, --output` | Output file path |
| `-l, --layer` | Layer name or index to extract |

## Examples

### Extract Named Layer

```bash
# Extract the beauty layer
vfx extract-layer render.exr -o beauty.exr --layer beauty
```

### Extract by Index

```bash
# Extract first layer (index 0)
vfx extract-layer render.exr -o first.exr --layer 0
```

### List Available Layers (no extraction)

```bash
# Without --layer, lists available layers and exits
vfx extract-layer render.exr -o main.exr
# No layer specified. Available layers:
#   [0] beauty
#   [1] diffuse
#   ...
```

**Note:** `--layer` is required. Without it, the command shows available layers but does not extract.

### Batch Extract

```bash
# Extract all layers to separate files
for layer in $(vfx layers render.exr --json | jq -r '.layers[].name'); do
    vfx extract-layer render.exr -o "${layer}.exr" --layer "$layer"
done
```

## Workflow

### List Available Layers

```bash
vfx layers render.exr

# Output:
# render.exr:
#   beauty (RGBA) - 1920x1080
#   diffuse (RGB) - 1920x1080
#   specular (RGB) - 1920x1080
#   depth (Z) - 1920x1080
```

### Extract Specific Passes

```bash
# Extract just the passes needed for comp
vfx extract-layer render.exr -o comp/beauty.exr --layer beauty
vfx extract-layer render.exr -o comp/diffuse.exr --layer diffuse
vfx extract-layer render.exr -o comp/spec.exr --layer specular
```

### Convert Layer to Different Format

```bash
# Extract and convert to PNG
vfx extract-layer render.exr -o beauty.exr --layer beauty
vfx convert beauty.exr -o beauty.png
```

## Common Layer Names

| Layer | Content |
|-------|---------|
| `beauty` | Final render |
| `diffuse` | Diffuse lighting |
| `specular` | Specular highlights |
| `reflection` | Reflection pass |
| `refraction` | Refraction pass |
| `emission` | Emissive objects |
| `sss` | Subsurface scattering |
| `depth` | Z-depth (single channel) |
| `normal` | World/camera normals |
| `position` | World position |
| `crypto_object` | Cryptomatte objects |
| `crypto_material` | Cryptomatte materials |

## Notes

- Output is standard single-layer EXR or other format
- Channel names are preserved
- Metadata from source is preserved
- Works with any multi-part EXR

## See Also

- [layers](./layers.md) - List layers
- [merge-layers](./merge-layers.md) - Combine layers
