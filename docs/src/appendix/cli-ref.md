# CLI Reference

Complete command reference for `vfx` CLI.

## Global Options

```
-v, --verbose     Increase verbosity (repeat for more: -vv, -vvv)
-q, --quiet       Suppress output
-h, --help        Show help
-V, --version     Show version
```

## Commands

### info

Display image metadata.

```bash
vfx info <INPUT>

Options:
  --layers        List EXR layers
  --channels      Show channel details
  --json          Output as JSON
```

**Examples**:
```bash
vfx info image.exr
vfx info image.exr --layers
vfx info sequence.%04d.exr
```

---

### convert

Convert between image formats.

```bash
vfx convert -i <INPUT> -o <OUTPUT>

Options:
  -i, --input <PATH>     Input image
  -o, --output <PATH>    Output image
  --layer <NAME>         Process specific EXR layer
  --quality <0-100>      JPEG quality (default: 90)
```

**Examples**:
```bash
vfx convert -i input.exr -o output.png
vfx convert -i render.exr -o preview.jpg --layer diffuse
```

---

### resize

Scale images using various filters.

```bash
vfx resize -i <INPUT> -o <OUTPUT> [SIZE_OPTIONS]

Size options (one required):
  -w, --width <N>        Target width (height auto)
  -h, --height <N>       Target height (width auto)
  -s, --scale <FACTOR>   Scale factor (e.g., 0.5)

Options:
  -f, --filter <TYPE>    Resampling filter
  --layer <NAME>         Process specific EXR layer

Filters:
  nearest                Nearest neighbor (fastest)
  bilinear, linear       Bilinear interpolation
  bicubic, cubic         Bicubic interpolation
  lanczos, lanczos3      Lanczos3 (default, best quality)
```

**Examples**:
```bash
vfx resize -i input.exr -o half.exr --scale 0.5
vfx resize -i input.exr -o thumb.png -w 256
vfx resize -i input.exr -o out.exr -w 1920 -h 1080 -f bicubic
```

---

### color

Apply color adjustments.

```bash
vfx color -i <INPUT> -o <OUTPUT> [ADJUSTMENTS]

Adjustments:
  -e, --exposure <STOPS>      Exposure adjustment
  -g, --gamma <VALUE>         Gamma correction
  -s, --saturation <VALUE>    Saturation (0=gray, 1=normal, 2=saturated)
  -t, --transfer <TYPE>       Transfer function

Transfer types:
  srgb, srgb_to_linear        sRGB to linear
  linear_to_srgb              Linear to sRGB
  rec709                      Rec.709 to linear

Options:
  --layer <NAME>              Process specific EXR layer
```

**Examples**:
```bash
vfx color -i input.exr -o output.exr --exposure 1.5
vfx color -i input.exr -o output.exr -g 2.2 -s 1.2
vfx color -i linear.exr -o srgb.png --transfer linear_to_srgb
```

---

### blur

Apply blur effects.

```bash
vfx blur -i <INPUT> -o <OUTPUT> -r <RADIUS>

Options:
  -r, --radius <N>       Blur radius in pixels
  -t, --type <TYPE>      Blur type: box, gaussian (default: box)
  --layer <NAME>         Process specific EXR layer
```

**Examples**:
```bash
vfx blur -i input.exr -o blurred.exr -r 5
vfx blur -i input.exr -o soft.exr -r 10 -t gaussian
```

---

### aces

Apply ACES color transforms.

```bash
vfx aces -i <INPUT> -o <OUTPUT> --transform <TYPE>

Transforms:
  idt, input, srgb-to-acescg    Input: sRGB → ACEScg
  rrt, tonemap                  RRT tonemap only
  odt, output, acescg-to-srgb   Output: ACEScg → sRGB
  rrt-odt, display, full        Full: ACEScg → sRGB display

Options:
  --rrt-variant <TYPE>          RRT variant: default, high-contrast
```

**Examples**:
```bash
vfx aces -i srgb.png -o acescg.exr --transform idt
vfx aces -i render.exr -o display.png --transform rrt-odt
vfx aces -i render.exr -o display.png --transform display --rrt-variant high-contrast
```

---

### lut

Apply LUT (Look-Up Table).

```bash
vfx lut -i <INPUT> -o <OUTPUT> --lut <LUT_FILE>

Options:
  --lut <PATH>           LUT file (.cube, .clf, .spi1d, .spi3d)
  --interpolation <TYPE> Interpolation: trilinear, tetrahedral (3D)
  --layer <NAME>         Process specific EXR layer
```

**Examples**:
```bash
vfx lut -i input.exr -o graded.exr --lut film_look.cube
vfx lut -i input.exr -o output.exr --lut transform.clf --interpolation tetrahedral
```

---

### composite

Composite images together.

```bash
vfx composite -a <FG> -b <BG> -o <OUTPUT> --mode <MODE>

Options:
  -a, --fg <PATH>        Foreground image
  -b, --bg <PATH>        Background image
  -o, --output <PATH>    Output image
  --mode <TYPE>          Blend mode
  --opacity <0-1>        Foreground opacity (default: 1.0)

Blend modes:
  over                   Alpha composite (default)
  add                    Additive
  multiply               Multiply
  screen                 Screen
  overlay                Overlay
```

**Examples**:
```bash
vfx composite -a fg.exr -b bg.exr -o comp.exr --mode over
vfx composite -a layer.exr -b base.exr -o out.exr --mode multiply --opacity 0.5
```

---

### transform

Apply geometric transforms.

```bash
vfx transform -i <INPUT> -o <OUTPUT> [OPERATIONS]

Operations:
  --rotate <DEG>         Rotate by degrees
  --flip-h               Flip horizontal
  --flip-v               Flip vertical
  --translate <X,Y>      Translate in pixels
  --layer <NAME>         Process specific EXR layer
```

**Examples**:
```bash
vfx transform -i input.exr -o rotated.exr --rotate 90
vfx transform -i input.exr -o flipped.exr --flip-h
```

---

### view

Interactive image viewer.

```bash
vfx view <INPUT>

Keyboard shortcuts:
  1-4          View channels (R, G, B, A)
  0            View all channels
  E            Exposure up
  Shift+E      Exposure down
  G            Toggle sRGB gamma
  F            Fit to window
  Space        Next image in sequence
  Q, Escape    Quit
```

---

### icc

Apply ICC profile transforms.

```bash
vfx icc -i <INPUT> -o <OUTPUT> --profile <ICC_FILE>

Options:
  --profile <PATH>       ICC profile file
  --intent <TYPE>        Rendering intent

Intents:
  perceptual            Perceptual (default)
  relative              Relative colorimetric
  saturation            Saturation
  absolute              Absolute colorimetric
```

---

### ocio

Apply OCIO color transforms.

```bash
vfx ocio -i <INPUT> -o <OUTPUT> --src <COLORSPACE> --dst <COLORSPACE>

Options:
  --config <PATH>        OCIO config file (or use $OCIO)
  --src <COLORSPACE>     Source colorspace
  --dst <COLORSPACE>     Destination colorspace
  --look <NAME>          Apply look
  --display <NAME>       Display name
  --view <NAME>          View transform
```

**Examples**:
```bash
vfx ocio -i render.exr -o display.png --src "ACES - ACEScg" --dst "Output - sRGB"
vfx ocio -i input.exr -o output.exr --config studio.ocio --src linear --dst "sRGB - Texture"
```

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error |
| 2 | Invalid arguments |
| 3 | File not found |
| 4 | Format error |
| 5 | Processing error |

## Environment Variables

| Variable | Description |
|----------|-------------|
| `OCIO` | Default OCIO config path |
| `VFX_LOG` | Log level (trace, debug, info, warn, error) |
| `VFX_THREADS` | Number of processing threads |

## Sequences

Supported sequence patterns:

```bash
# Printf-style
image.%04d.exr          # image.0001.exr, image.0002.exr, ...
render.%d.exr           # render.1.exr, render.2.exr, ...

# Hash notation
image.####.exr          # image.0001.exr, image.0002.exr, ...
```

**Examples**:
```bash
vfx info "sequence.%04d.exr"
vfx convert -i "input.####.exr" -o "output.####.png"
```
