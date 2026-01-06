# diff - Image Comparison

Compare two images and optionally output a difference image.

**Alias:** `d`

## Synopsis

```bash
vfx diff <A> <B> [-o <OUTPUT>] [-t <THRESHOLD>] [-w <WARN>]
```

## Options

| Option | Description |
|--------|-------------|
| `<A>` | First image |
| `<B>` | Second image |
| `-o, --output` | Output difference image |
| `-t, --threshold` | Fail threshold (max allowed difference), default: 0.0 |
| `-w, --warn` | Per-pixel warning threshold |

## Output Statistics

The command outputs:
- Mean error
- Max error
- RMS error
- Number of pixels exceeding thresholds
- Pass/fail status

## Examples

### Basic Comparison

```bash
# Compare two images
vfx diff original.exr processed.exr

# Output:
# Comparing original.exr vs processed.exr
# Mean error: 0.0012
# Max error: 0.0523
# RMS error: 0.0034
# PASS: Images match within threshold
```

### Save Difference Image

```bash
# Generate visual difference
vfx diff a.exr b.exr -o diff.exr

# The difference image shows absolute per-pixel error
```

### Set Threshold

```bash
# Fail if max error exceeds 0.01
vfx diff expected.exr actual.exr -t 0.01

# Exit code 0 = pass, 1 = fail
```

### Warning Threshold

```bash
# Warn on pixels with error > 0.001, fail on > 0.01
vfx diff a.exr b.exr -t 0.01 -w 0.001

# Output:
# WARNING: 127 pixels exceed warning threshold (0.001)
# PASS: Max error 0.008 within fail threshold
```

## Use Cases

### CI/CD Testing

```bash
# Test render matches reference
vfx diff reference.exr test_output.exr -t 0.001 || exit 1
```

### Quality Control

```bash
# Check for compression artifacts
vfx diff original.exr compressed.jpg -o artifacts.exr
vfx info artifacts.exr --stats
```

### Debugging

```bash
# Visualize differences in compositing
vfx diff comp_v1.exr comp_v2.exr -o changes.exr
```

## Difference Image

The output difference image contains:
- Absolute difference per channel: `|A - B|`
- Alpha channel: max difference across RGB
- Can be visualized with gain for subtle differences

```bash
# Boost difference visibility
vfx diff a.exr b.exr -o diff.exr
vfx color diff.exr -o diff_visible.exr --exposure 5.0
```

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Images match within threshold |
| 1 | Images differ beyond threshold |
| 2 | Error (file not found, size mismatch, etc.) |

## See Also

- [info](./info.md) - Image information
- [composite](./composite.md) - Blend images
