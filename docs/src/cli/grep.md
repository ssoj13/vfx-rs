# grep - Simple Image Search

Search for patterns in image filenames and basic properties.

**Note:** This is a simplified search tool. It does NOT support regex or metadata search (EXIF, EXR attributes).

## Synopsis

```bash
vfx grep <PATTERN> <INPUT>... [-i]
```

## Options

| Option | Description |
|--------|-------------|
| `<PATTERN>` | Search pattern (substring match, NOT regex) |
| `<INPUT>` | Image file(s) to search |
| `-i, --ignore-case` | Case-insensitive search |

## What Gets Searched

Only three things are checked:
1. **Filename** - the input filename
2. **Dimensions** - format `WIDTHxHEIGHT CHANNELSch`
3. **Format** - format enum like `Exr`, `Jpeg`, `Png`

**NOT searched:** EXIF, EXR attributes, camera info, color space, compression, or any other metadata.

## Examples

### Search by Filename

```bash
# Find files with "shot" in name
vfx grep "shot" *.exr
```

### Search by Dimensions

```bash
# Find 1920 wide images
vfx grep "1920" *.exr
```

### Case-Insensitive

```bash
vfx grep -i "jpeg" *.jpg
```

### Multiple Files

```bash
vfx grep "alpha" shots/*.exr
```

## Limitations

The following features are **not implemented**:
- Regex patterns
- EXIF metadata search
- EXR attribute search
- Custom metadata search
- Camera or lens info search
- Color space search

For detailed metadata inspection, use `vfx info` instead.

## See Also

- [info](./info.md) - View full metadata
- [batch](./batch.md) - Batch processing
