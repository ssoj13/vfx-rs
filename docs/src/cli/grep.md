# grep - Metadata Search

Search for patterns in image metadata (like OIIO's igrep).

## Synopsis

```bash
vfx grep <PATTERN> <INPUT>... [-i]
```

## Options

| Option | Description |
|--------|-------------|
| `<PATTERN>` | Search pattern (regex supported) |
| `<INPUT>` | Image file(s) to search |
| `-i, --ignore-case` | Case-insensitive search |

## Examples

### Basic Search

```bash
# Find images with specific camera
vfx grep "ARRI" *.exr

# Output:
# shot_001.exr: camera: ARRI Alexa Mini
# shot_005.exr: camera: ARRI Alexa 65
```

### Case-Insensitive

```bash
# Find color space info regardless of case
vfx grep -i "srgb" *.exr
```

### Regex Patterns

```bash
# Find images with resolution info
vfx grep "[0-9]+x[0-9]+" *.exr

# Find images with specific date format
vfx grep "2024-[0-9]{2}-[0-9]{2}" *.exr
```

### Multiple Files

```bash
# Search all EXR files in directory
vfx grep "ACEScg" shots/*.exr

# Search multiple formats
vfx grep "Adobe RGB" *.jpg *.tif *.exr
```

## Searched Metadata

The command searches through:

- **Standard attributes**: resolution, channels, bit depth
- **EXIF data**: camera, lens, exposure settings
- **EXR attributes**: chromaticities, compression, author
- **Custom attributes**: any embedded metadata

## Use Cases

### Find Camera Footage

```bash
# Find all ARRI footage
vfx grep -i "alexa" footage/*.dpx

# Find all Sony footage
vfx grep -i "venice\|fx9" footage/*.mxf
```

### Check Color Space

```bash
# Find images in specific color space
vfx grep "ACEScg" project/*.exr
vfx grep "sRGB" project/*.jpg
```

### Find by Date

```bash
# Find images from specific date
vfx grep "2024-01-15" *.exr
```

### Find by Compression

```bash
# Find PIZ compressed EXR files
vfx grep "piz" *.exr
```

## Output Format

```
<filename>: <matching_line>
```

If no match is found, file is not listed.

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Matches found |
| 1 | No matches found |
| 2 | Error (file not found, etc.) |

## Integration with Other Tools

```bash
# Find and process matching files
vfx grep "ACEScg" *.exr | cut -d: -f1 | while read f; do
    vfx aces "$f" -o "output/${f%.exr}_srgb.png" -t rrt-odt
done

# Count matches
vfx grep "ARRI" *.exr | wc -l
```

## See Also

- [info](./info.md) - View full metadata
- [batch](./batch.md) - Batch processing
