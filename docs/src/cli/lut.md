# lut - LUT Application

Apply 1D or 3D lookup tables to images.

## Synopsis

```bash
vfx lut <INPUT> -o <OUTPUT> -l <LUT> [--invert]
```

## Options

| Option | Description |
|--------|-------------|
| `-o, --output` | Output file path |
| `-l, --lut` | LUT file path (.cube, .clf) |
| `--invert` | Invert the LUT (if supported) |

## Supported Formats

| Format | Extension | Type |
|--------|-----------|------|
| Resolve/Adobe Cube | `.cube` | 1D/3D |
| Common LUT Format | `.clf` | 1D/3D + operations |
| Sony Spi1d | `.spi1d` | 1D |
| Sony Spi3d | `.spi3d` | 3D |
| Autodesk 3DL | `.3dl` | 3D |

## Examples

### Apply 3D LUT

```bash
# Apply color grade LUT
vfx lut input.exr -o graded.exr -l film_look.cube
```

### Apply 1D LUT

```bash
# Apply gamma curve
vfx lut input.exr -o adjusted.exr -l gamma_22.cube
```

### Apply CLF

```bash
# Apply ACES-compatible CLF
vfx lut input.exr -o output.exr -l transform.clf
```

### Invert LUT

```bash
# Undo a LUT transformation
vfx lut graded.exr -o original.exr -l film_look.cube --invert
```

## LUT File Examples

### Cube Format (1D)

```
TITLE "Gamma 2.2"
LUT_1D_SIZE 256
0.0 0.0 0.0
0.003906 0.003906 0.003906
0.007813 0.007813 0.007813
...
```

### Cube Format (3D)

```
TITLE "Film Look"
LUT_3D_SIZE 33
0.0 0.0 0.0
0.03125 0.0 0.0
0.0625 0.0 0.0
...
```

### CLF Format

```xml
<?xml version="1.0" encoding="UTF-8"?>
<ProcessList id="FilmLook">
  <LUT3D interpolation="tetrahedral">
    <Array dim="17 17 17 3">
      <!-- LUT data -->
    </Array>
  </LUT3D>
</ProcessList>
```

## Use Cases

### Color Grading

```bash
# Apply colorist-provided LUT
vfx lut raw_grade.exr -o final.exr -l colorist_v3.cube
```

### Film Emulation

```bash
# Emulate film stock
vfx lut digital.exr -o film_look.exr -l kodak_2383.cube
```

### Camera LUT

```bash
# Apply manufacturer camera LUT
vfx lut camera.dpx -o converted.exr -l arri_to_rec709.cube
```

### Creative Look

```bash
# Apply creative grade
vfx lut footage.exr -o styled.exr -l cinematic_teal_orange.cube
```

## Workflow Tips

1. **Apply in correct space** - Most LUTs expect specific input (log, linear, etc.)
2. **Check LUT size** - Larger LUTs (33³, 65³) are more accurate
3. **CLF for ACES** - Use CLF for ACES-compatible workflows
4. **Test extremes** - Check highlights and shadows

## Technical Notes

- 3D LUTs use tetrahedral interpolation
- 1D LUTs use linear interpolation
- Values outside LUT range are clamped
- Supports half-float and full-float precision

## See Also

- [color](./color.md) - Color transforms
- [aces](./aces.md) - ACES workflow
