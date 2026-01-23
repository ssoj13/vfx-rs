# channel-shuffle - Channel Reordering

Shuffle, swap, or rearrange image channels.

**Alias:** `cs`

## Synopsis

```bash
vfx channel-shuffle <INPUT> -o <OUTPUT> -p <PATTERN>
```

## Options

| Option | Description |
|--------|-------------|
| `-o, --output` | Output file path |
| `-p, --pattern` | Channel shuffle pattern |

## Pattern Syntax

| Character | Meaning |
|-----------|---------|
| `R` | Red channel (index 0) |
| `G` | Green channel (index 1) |
| `B` | Blue channel (index 2) |
| `A` | Alpha channel (index 3) |
| `2-9` | Numeric channel index |
| `0` | Black (0.0) |
| `1` | White (1.0) |

**Note:** `0` and `1` are interpreted as constants (black/white), not channel indices. Use `R`, `G`, `B`, `A` for channels 0-3, and digits 2-9 for channels 4+.

Pattern length determines output channels.

## Examples

### Swap Channels

```bash
# RGB → BGR
vfx channel-shuffle input.exr -o bgr.exr -p BGR
```

### Extract Single Channel

```bash
# Red channel to grayscale
vfx channel-shuffle input.exr -o red.exr -p RRR

# Alpha to grayscale
vfx channel-shuffle input.exr -o alpha_vis.exr -p AAA
```

### Add/Remove Alpha

```bash
# RGB → RGBA (opaque alpha)
vfx channel-shuffle rgb.exr -o rgba.exr -p RGB1

# RGBA → RGB (drop alpha)
vfx channel-shuffle rgba.exr -o rgb.exr -p RGB
```

### Create Mask

```bash
# Black image with white alpha
vfx channel-shuffle input.exr -o mask.exr -p 0001

# Solid color
vfx channel-shuffle input.exr -o white.exr -p 1111
```

### Channel to Alpha

```bash
# Red channel to alpha
vfx channel-shuffle input.exr -o output.exr -p RGBR
```

## Common Patterns

| Pattern | Result |
|---------|--------|
| `RGB` | Remove alpha |
| `RGB1` | Add opaque alpha |
| `BGR` | Swap red/blue |
| `GGG` | Green to grayscale |
| `RRR1` | Red to grayscale with alpha |
| `AAAA` | Alpha to all channels |
| `0001` | Black with alpha |
| `111A` | White with original alpha |

## Use Cases

### Fix Channel Order

```bash
# Some software uses BGR internally
vfx channel-shuffle render.exr -o fixed.exr -p BGR
```

### Create Alpha Matte

```bash
# Use luminance as alpha
vfx color input.exr -o temp.exr --saturation 0
vfx channel-shuffle temp.exr -o matte.exr -p RRRR
```

### Prep for Compositing

```bash
# Ensure proper alpha
vfx channel-shuffle plate.exr -o plate_rgba.exr -p RGB1
```

### Debug Channels

```bash
# Visualize each channel
vfx channel-shuffle render.exr -o red.exr -p RRR
vfx channel-shuffle render.exr -o green.exr -p GGG
vfx channel-shuffle render.exr -o blue.exr -p BBB
```

## Notes

- Works on any number of input channels
- Missing channels default to 0, except **alpha (A) defaults to 1** (opaque)
- Output is always float32 (bit depth is not preserved)
- Fast operation (no pixel processing)

## See Also

- [channel-extract](./channel-extract.md) - Extract named channels
- [layers](./layers.md) - Work with EXR layers
