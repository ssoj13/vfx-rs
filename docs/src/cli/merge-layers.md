# merge-layers - Layer Merging

Merge multiple images into a single multi-layer EXR file.

**Alias:** `ml`

## Synopsis

```bash
vfx merge-layers <INPUT>... -o <OUTPUT> [-n <NAMES>]
```

## Options

| Option | Description |
|--------|-------------|
| `<INPUT>` | Input files (each becomes a layer) |
| `-o, --output` | Output multi-layer EXR file |
| `-n, --names` | Custom layer names (comma-separated) |

## Examples

### Basic Merge

```bash
# Merge three images as layers
vfx merge-layers beauty.exr diffuse.exr specular.exr -o combined.exr
```

### With Custom Names

```bash
# Specify layer names
vfx merge-layers a.exr b.exr c.exr -o combined.exr \
    --names beauty,diffuse,specular
```

### From Render Passes

```bash
# Merge all render passes
vfx merge-layers \
    render_beauty.exr \
    render_diffuse.exr \
    render_specular.exr \
    render_depth.exr \
    -o render_combined.exr \
    --names beauty,diffuse,specular,depth
```

## Workflow

### Create Multi-Layer from Separate Files

```bash
# Standard VFX workflow
vfx merge-layers \
    passes/beauty.exr \
    passes/diffuse.exr \
    passes/spec.exr \
    passes/emission.exr \
    passes/depth.exr \
    -o shot_v001.exr \
    --names beauty,diffuse,specular,emission,depth
```

### Add Layer to Existing File

```bash
# Extract existing layers, add new one, merge back
vfx extract-layer existing.exr -o temp_beauty.exr --layer beauty
vfx extract-layer existing.exr -o temp_diff.exr --layer diffuse
vfx merge-layers temp_beauty.exr temp_diff.exr new_layer.exr \
    -o updated.exr --names beauty,diffuse,new_layer
```

### Combine Different Formats

```bash
# Convert and merge
vfx convert beauty.png -o temp_beauty.exr
vfx convert diffuse.jpg -o temp_diffuse.exr
vfx merge-layers temp_beauty.exr temp_diffuse.exr -o combined.exr
```

## Layer Naming

### Default Names

Without `--names`, layers are named by input filename:

```bash
vfx merge-layers beauty.exr diffuse.exr -o combined.exr
# Layers: beauty, diffuse
```

### Custom Names

```bash
vfx merge-layers a.exr b.exr -o combined.exr --names main,secondary
# Layers: main, secondary
```

## Requirements

- All inputs must have same resolution
- All inputs must have compatible bit depths
- Output is always multi-part EXR

## Use Cases

### Render Layer Management

```bash
# Combine separated render passes for Nuke/Fusion
vfx merge-layers *.exr -o combined.exr
```

### Archive Organization

```bash
# Combine related plates into single file
vfx merge-layers \
    plate_clean.exr \
    plate_matte.exr \
    plate_depth.exr \
    -o plate_combined.exr
```

### Delivery Package

```bash
# Create delivery file with all passes
vfx merge-layers \
    final_rgb.exr \
    final_alpha.exr \
    final_depth.exr \
    -o delivery.exr
```

## See Also

- [layers](./layers.md) - List layers
- [extract-layer](./extract-layer.md) - Extract single layer
