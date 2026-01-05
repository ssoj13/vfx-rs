# batch - Batch Processing

Process multiple files in parallel with a single command.

## Usage

```bash
vfx batch [OPTIONS] <PATTERN> -o <OUTPUT_DIR> --op <OPERATION>
```

## Options

| Option | Description |
|--------|-------------|
| `--op <OP>` | Operation: resize, convert, color, blur |
| `--args <K=V>...` | Operation arguments |
| `--format <FMT>` | Output format (default: same as input) |

## Examples

```bash
# Resize all EXRs to 1920 width
vfx batch "renders/*.exr" -o proxies/ --op resize --args width=1920

# Convert EXR to PNG
vfx batch "frames/*.exr" -o pngs/ --op convert --format png

# Apply exposure adjustment
vfx batch "plates/*.exr" -o graded/ --op color --args exposure=0.5

# Blur sequence
vfx batch "seq/*.exr" -o blurred/ --op blur --args radius=3
```

## Parallelization

Batch uses Rayon for parallel processing. All CPU cores are utilized:

```bash
# Verbose shows per-file progress
vfx batch -v "*.exr" -o out/ --op resize --args width=1920
# Found 100 files matching '*.exr'
# Processing render.0001.exr -> out/render.0001.exr
# Processing render.0002.exr -> out/render.0002.exr
# ...
# Processed: 100 success, 0 failed
```

## Error Handling

Failed files don't stop the batch:

```bash
vfx batch "*.exr" -o out/ --op convert
# Processing file1.exr -> out/file1.png
# Error: file2.exr - corrupted data
# Processing file3.exr -> out/file3.png
# Processed: 99 success, 1 failed
```

## Output Naming

Output files keep the original name with new extension:
- `input/render.0001.exr` → `output/render.0001.exr`
- With `--format png`: `output/render.0001.png`

## Available Operations

| Operation | Arguments |
|-----------|-----------|
| `resize` | `width`, `height`, `scale`, `filter` |
| `convert` | `depth`, `compression` |
| `color` | `exposure`, `gamma`, `saturation` |
| `blur` | `radius`, `type` |
