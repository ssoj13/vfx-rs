# layers - List EXR Layers

List layers and channels in multi-layer OpenEXR files.

**Alias:** `l`

**Note:** This is a separate command from `extract-layer` and `merge-layers`. There are no subcommands.

## Synopsis

```bash
vfx layers <INPUT> [--json]
```

## Options

| Option | Description |
|--------|-------------|
| `--json` | Output in JSON format |

## Examples

### List Layers

```bash
vfx layers render.exr
# Output:
# render.exr
#   Layers: 5
#     [0] beauty (1920x1080, 4 channels)
#         R, G, B, A
#     [1] diffuse (1920x1080, 3 channels)
#         diffuse.R, diffuse.G, diffuse.B
#     [2] specular (1920x1080, 3 channels)
#     [3] depth (1920x1080, 1 channel)
#     [4] normals (1920x1080, 3 channels)
```

### JSON Output

```bash
vfx layers render.exr --json
# Outputs structured JSON for scripting
```

## Related Commands

| Command | Purpose |
|---------|---------|
| `vfx layers` | List layers in EXR |
| `vfx extract-layer` | Extract single layer to file |
| `vfx merge-layers` | Combine images into multi-layer EXR |

**Note:** Unlike the documentation previously suggested, these are separate top-level commands, not subcommands of `layers`.

## Use with Other Commands

Process specific layers in other commands:

```bash
# Resize only the beauty layer
vfx resize render.exr -o resized.exr --layer beauty -s 0.5

# Color correct diffuse pass
vfx color render.exr -o corrected.exr --layer diffuse -e 0.5

# Blur depth for DOF
vfx blur render.exr -o blurred.exr --layer depth -r 5
```

## EXR Layer Naming

vfx-rs follows OpenEXR conventions:
- Default layer: `R`, `G`, `B`, `A`
- Named layers: `layername.R`, `layername.G`, `layername.B`
- Arbitrary channels: `layername.channelname`

## See Also

- [extract-layer](./extract-layer.md) - Extract single layer
- [merge-layers](./merge-layers.md) - Combine into multi-layer
