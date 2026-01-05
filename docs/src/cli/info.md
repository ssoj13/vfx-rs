# info - Image Information

Display image metadata, dimensions, and format details. Equivalent to OIIO's `iinfo`.

## Usage

```bash
vfx info [OPTIONS] <INPUT>...
```

## Options

| Option | Description |
|--------|-------------|
| `--stats` | Compute min/max/avg pixel values |
| `--all`, `-a` | Show all metadata and layer details |
| `--json` | Output as JSON |

## Examples

```bash
# Basic info
vfx info render.exr
# Output:
# render.exr
#   Resolution: 1920x1080
#   Channels:   4
#   Pixels:     2073600
#   File size:  12.5 MB

# With statistics
vfx info --stats render.exr
# Output:
#   Min value:  0.000000
#   Max value:  12.456789
#   Avg value:  0.523456

# Full metadata (EXR attributes, EXIF, etc.)
vfx info --all photo.exr

# JSON output for scripting
vfx info --json render.exr | jq '.width, .height'

# Multiple files
vfx info *.exr

# Verbose mode shows format detection
vfx info -vv render.exr
```

## EXR Layer Information

For multi-layer EXR files, `--all` or `-v` shows layer details:

```bash
vfx info -v multilayer.exr
# Output:
# multilayer.exr
#   Resolution: 1920x1080
#   Channels:   4
#   Layers:     3
#   Layer details:
#     [0] "rgba" (1920x1080) R, G, B, A
#     [1] "depth" (1920x1080) Z
#     [2] "normals" (1920x1080) N.X, N.Y, N.Z
```
