# batch - Batch Processing

Process multiple files in parallel with a single command.

## Usage

```bash
vfx batch -i <PATTERN> -o <OUTPUT_DIR> --op <OPERATION> [--args K=V...] [-f <FORMAT>]
```

## Options

| Option | Description |
|--------|-------------|
| `-i, --input` | Input pattern (glob), e.g., `"*.exr"` |
| `-o, --output-dir` | Output directory |
| `--op` | Operation to apply |
| `-a, --args` | Operation arguments (key=value) |
| `-f, --format` | Output format extension (default: same as input) |

## Supported Operations

| Operation | Arguments | Description |
|-----------|-----------|-------------|
| `convert` | (none) | Copy with format conversion |
| `resize` | `scale` | Resize by scale factor (Lanczos3 filter) |
| `blur` | `radius` | Box blur |
| `flip_h` | (none) | Flip horizontally |
| `flip_v` | (none) | Flip vertically |

**Note:** `color` operation is **not yet implemented**. The docs previously claimed exposure/gamma/saturation support, but these are not available in batch mode.

## Examples

### Convert EXR to PNG

```bash
vfx batch -i "frames/*.exr" -o pngs/ --op convert -f png
```

### Resize to 50%

```bash
vfx batch -i "renders/*.exr" -o proxies/ --op resize --args scale=0.5
```

### Box Blur

```bash
vfx batch -i "seq/*.exr" -o blurred/ --op blur --args radius=5
```

### Flip Images

```bash
vfx batch -i "*.exr" -o flipped/ --op flip_h
vfx batch -i "*.exr" -o flipped/ --op flip_v
```

## Parallelization

Batch uses Rayon for parallel processing. All CPU cores are utilized:

```bash
# Verbose shows per-file progress
vfx batch -v -i "*.exr" -o out/ --op resize --args scale=0.5
# Found 100 files matching '*.exr'
# Processing render.0001.exr -> out/render.0001.exr
# Processing render.0002.exr -> out/render.0002.exr
# ...
# Processed: 100 success, 0 failed
```

## Error Handling

Failed files don't stop the batch:

```bash
vfx batch -i "*.exr" -o out/ --op convert
# Processing file1.exr -> out/file1.png
# Error: file2.exr - corrupted data
# Processing file3.exr -> out/file3.png
# Processed: 99 success, 1 failed
```

## Output Naming

Output files keep the original name with new extension:
- `input/render.0001.exr` -> `output/render.0001.exr`
- With `-f png`: `output/render.0001.png`

## Limitations

The following features are **not yet implemented**:
- `color` operation (exposure, gamma, saturation)
- Resize by width/height (only scale factor)
- Resize filter selection (always Lanczos3)
- Blur type selection (always box blur)
- Convert depth/compression options
