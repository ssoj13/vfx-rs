# layers - EXR Layer Operations

Work with multi-layer OpenEXR files (AOVs, render passes).

## Commands

```bash
vfx layers list <INPUT>           # List layers
vfx layers extract <INPUT> -o <OUTPUT> --layer <NAME>
vfx layers merge <INPUTS>... -o <OUTPUT>
```

## List Layers

```bash
vfx layers list render.exr
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

## Extract Layer

```bash
# Extract specific layer to separate file
vfx layers extract render.exr -o depth.exr --layer depth

# Extract with verbose output
vfx layers extract -v render.exr -o diffuse.exr --layer diffuse
```

## Merge Layers

Combine multiple images into a multi-layer EXR:

```bash
# Merge separate passes
vfx layers merge beauty.exr diffuse.exr specular.exr -o combined.exr

# Layers are named from source filenames
# combined.exr will contain: beauty, diffuse, specular
```

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
