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
# Generate visual difference (scaled 10x for visibility)
vfx diff a.exr b.exr -o diff.exr

# Note: difference image is `|A - B| * 10` clamped to 1.0
```

### Set Threshold

```bash
# Fail if max error exceeds 0.01
vfx diff expected.exr actual.exr -t 0.01

# Exit code 0 = pass, 1 = fail or error
```

### Warning Threshold

```bash
# Warn if max error exceeds 0.001, fail if > 0.01
vfx diff a.exr b.exr -t 0.01 -w 0.001

# Output:
# WARNING: Max difference 0.003 exceeds warning threshold 0.001
# PASS
```

**Note:** Warning threshold checks max difference, not pixel count.

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
- Per-channel difference scaled by 10x: `min(|A - B| * 10, 1.0)`
- Same channel count as input (compares common channels)
- Already amplified for visibility

```bash
# Generate difference image
vfx diff a.exr b.exr -o diff.exr
# Note: differences are pre-scaled 10x, clamped to 1.0
```

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Images match within threshold |
| 1 | Images differ beyond threshold, or error occurred |

**Note:** Both failures (threshold exceeded) and errors (file not found, dimension mismatch) return exit code 1.

## See Also

- [info](./info.md) - Image information
- [composite](./composite.md) - Blend images
