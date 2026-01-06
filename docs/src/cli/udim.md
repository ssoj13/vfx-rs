# udim - UDIM Texture Operations

Manage UDIM (U-DIMension) texture sets for 3D assets.

## Synopsis

```bash
vfx udim <COMMAND> [OPTIONS]
```

## Subcommands

| Command | Description |
|---------|-------------|
| `info` | Show UDIM texture set information |
| `convert` | Convert all tiles to another format |
| `atlas` | Create atlas from UDIM tiles |
| `split` | Split single image into UDIM tiles |

---

## udim info

Display information about a UDIM texture set.

```bash
vfx udim info <PATTERN>
```

### Examples

```bash
# Info on UDIM set
vfx udim info textures/diffuse.<UDIM>.exr

# Or detect from a single tile
vfx udim info textures/diffuse.1001.exr
```

### Output

```
UDIM Set: textures/diffuse.<UDIM>.exr
Tiles found: 8

  1001: 4096x4096, 4 channels, half float
  1002: 4096x4096, 4 channels, half float
  1003: 4096x4096, 4 channels, half float
  1004: 4096x4096, 4 channels, half float
  1011: 4096x4096, 4 channels, half float
  1012: 4096x4096, 4 channels, half float
  1013: 4096x4096, 4 channels, half float
  1014: 4096x4096, 4 channels, half float

Total size: 2.1 GB
```

---

## udim convert

Convert all tiles in a UDIM set to another format or settings.

```bash
vfx udim convert <INPUT> <OUTPUT> [-c <COMPRESSION>]
```

### Options

| Option | Description |
|--------|-------------|
| `-c, --compression` | Compression type (for EXR) |

### Examples

```bash
# Convert to different compression
vfx udim convert \
    textures/diffuse.<UDIM>.exr \
    output/diffuse.<UDIM>.exr \
    -c dwaa

# Convert to different format
vfx udim convert \
    textures/diffuse.<UDIM>.exr \
    output/diffuse.<UDIM>.tx
```

---

## udim atlas

Combine UDIM tiles into a single atlas image.

```bash
vfx udim atlas <INPUT> <OUTPUT> [-t <TILE_SIZE>]
```

### Options

| Option | Description |
|--------|-------------|
| `-t, --tile-size` | Tile resolution (all tiles scaled to this, default: 1024) |

### Examples

```bash
# Create atlas from UDIMs
vfx udim atlas \
    textures/diffuse.<UDIM>.exr \
    atlas_diffuse.exr \
    --tile-size 1024
```

### Output Layout

```
Atlas for 2x2 UDIMs:
┌──────────┬──────────┐
│   1001   │   1002   │
│          │          │
├──────────┼──────────┤
│   1011   │   1012   │
│          │          │
└──────────┴──────────┘
```

---

## udim split

Split a single atlas image into UDIM tiles.

```bash
vfx udim split <INPUT> <OUTPUT> [-t <TILE_SIZE>]
```

### Options

| Option | Description |
|--------|-------------|
| `-t, --tile-size` | Tile size in pixels (default: 1024) |

### Examples

```bash
# Split 4K atlas into 1K tiles
vfx udim split \
    atlas.exr \
    tiles/diffuse.<UDIM>.exr \
    --tile-size 1024
```

---

## UDIM Naming Convention

UDIM numbers follow the Mari convention:

```
UDIM = 1001 + (U tile) + (V tile * 10)

U=0, V=0 → 1001
U=1, V=0 → 1002
U=0, V=1 → 1011
U=1, V=1 → 1012
```

### Layout

```
V
↑
│  1021  1022  1023  1024
│  1011  1012  1013  1014
│  1001  1002  1003  1004
└────────────────────────→ U
```

## Use Cases

### Texture Pipeline

```bash
# Prepare textures for rendering
vfx udim info assets/car_diffuse.<UDIM>.exr

# Convert to TX format
vfx udim convert \
    assets/car_diffuse.<UDIM>.exr \
    textures/car_diffuse.<UDIM>.tx

# Convert to ACES
for tile in assets/car_diffuse.*.exr; do
    vfx aces "$tile" -o "aces/${tile##*/}" -t idt
done
```

### Preview Generation

```bash
# Create atlas for quick preview
vfx udim atlas \
    assets/character.<UDIM>.exr \
    preview/character_atlas.jpg \
    --tile-size 512
```

### Asset Migration

```bash
# Convert from tiled to atlas
vfx udim atlas input.<UDIM>.exr output_atlas.exr -t 1024

# Convert from atlas to tiled
vfx udim split atlas.exr output.<UDIM>.exr -t 2048
```

## See Also

- [convert](./convert.md) - Single file conversion
- [maketx](./maketx.md) - Texture creation
