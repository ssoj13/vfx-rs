# channel-extract - Channel Extraction

Extract specific channels from an image to create a new image.

**Alias:** `cx`

## Synopsis

```bash
vfx channel-extract <INPUT> -o <OUTPUT> -c <CHANNELS>
```

## Options

| Option | Description |
|--------|-------------|
| `-o, --output` | Output file path |
| `-c, --channels` | Channels to extract (by name or index) |

## Channel Specification

Channels can be specified by:
- **Name**: `R`, `G`, `B`, `A`, `Z` (standard names only)
- **Index**: `0`, `1`, `2`, `3`, ...

Use `-c` once per channel (not comma-separated).

**Note:** Custom/arbitrary channel names (like `N.x`, `P.y`, `beauty.R`) are **not yet supported**. Use numeric indices for non-standard channels.

## Examples

### Extract by Name

```bash
# Extract RGB channels only
vfx channel-extract input.exr -o rgb.exr -c R -c G -c B

# Extract just alpha
vfx channel-extract input.exr -o alpha.exr -c A
```

### Extract by Index

```bash
# First three channels
vfx channel-extract input.exr -o output.exr -c 0 -c 1 -c 2

# Fourth channel (often alpha or Z)
vfx channel-extract input.exr -o fourth.exr -c 3
```

### Extract Depth Channel

```bash
# Extract depth channel
vfx channel-extract render.exr -o depth.exr -c Z

# For non-standard channels, use numeric indices
vfx channel-extract render.exr -o ch5.exr -c 5
```

### Extract Subset

```bash
# Just red and green
vfx channel-extract input.exr -o rg.exr -c R -c G
```

## Supported Channel Names

Currently supported names:
- `R`, `G`, `B`, `A` - Main RGBA (indices 0-3)
- `Z` - Depth (index 4)

For other channels (normals, position, layer channels), use numeric indices.

## Use Cases

### Separate Depth Pass

```bash
# Extract Z channel for depth compositing
vfx channel-extract render.exr -o depth.exr -c Z
```

### Create Grayscale

```bash
# Extract single channel as grayscale
vfx channel-extract color.exr -o gray.exr -c R
```

### Prepare for Specific Software

```bash
# Some software expects specific channel configs
vfx channel-extract input.exr -o output.exr -c R -c G -c B
```

### Debug Channels

```bash
# Check what's in each channel
vfx channel-extract input.exr -o ch0.exr -c 0
vfx channel-extract input.exr -o ch1.exr -c 1
vfx channel-extract input.exr -o ch2.exr -c 2
vfx channel-extract input.exr -o ch3.exr -c 3
```

## Difference from channel-shuffle

| Tool | Use Case |
|------|----------|
| `channel-extract` | Select specific channels by name |
| `channel-shuffle` | Reorder/copy channels by pattern |

```bash
# Both can extract RGB:
vfx channel-extract input.exr -o rgb.exr -c R -c G -c B
vfx channel-shuffle input.exr -o rgb.exr -p RGB
```

## Notes

- Output has exactly the number of extracted channels
- Channel order matches specification order
- Only standard names supported (use indices for custom channels)
- Fast operation (direct channel copy)

## See Also

- [channel-shuffle](./channel-shuffle.md) - Reorder channels
- [layers](./layers.md) - Work with EXR layers
