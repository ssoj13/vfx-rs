# CLI Reference

Complete command reference for `vfx` CLI.

## Global Options

```
-v, --verbose          Increase verbosity (repeat for more: -vv, -vvv)
-l, --log <LEVEL>      Set log level (trace, debug, info, warn, error)
-j, --threads <N>      Number of processing threads (0 = auto)
--allow-non-color      Process non-color data (depth, normals, etc.)
-h, --help             Show help
-V, --version          Show version
```

## Commands

### info

Display image metadata.

```bash
vfx info <INPUT>...

Options:
  -s, --stats     Show pixel statistics (min/max/avg)
  -a, --all       Show all available metadata
  --json          Output as JSON
```

**Examples**:
```bash
vfx info image.exr
vfx info image.exr --all
vfx info *.exr --json
```

**Note:** Use `vfx layers` to list EXR layers.

---

### convert

Convert between image formats.

```bash
vfx convert <INPUT> -o <OUTPUT>

Options:
  -o, --output <PATH>    Output image
  -d, --depth <TYPE>     Output bit depth (u8, u16, f16, f32)
  -c, --compression <TYPE> EXR compression
  -q, --quality <0-100>  JPEG quality (default: 95)
```

**Examples**:
```bash
vfx convert input.exr -o output.png
vfx convert input.exr -o preview.jpg -q 90
```

**Note:** Use `vfx extract-layer` to convert specific EXR layers.

---

### resize

Scale images using various filters.

```bash
vfx resize <INPUT> -o <OUTPUT> [SIZE_OPTIONS]

Size options (one required):
  -w, --width <N>        Target width (height auto)
  -H, --height <N>       Target height (width auto)
  -s, --scale <FACTOR>   Scale factor (e.g., 0.5)

Options:
  -f, --filter <TYPE>    Resampling filter (default: lanczos)
  --fit <MODE>           Fit mode: exact, contain, cover, fill
  --layer <NAME>         Process specific EXR layer

Filters:
  nearest                Nearest neighbor (fastest)
  bilinear               Bilinear interpolation
  bicubic                Bicubic interpolation
  lanczos                Lanczos3 (default, best quality)
```

**Examples**:
```bash
vfx resize input.exr -o half.exr -s 0.5
vfx resize input.exr -o thumb.png -w 256
vfx resize input.exr -o out.exr -w 1920 -H 1080 -f bicubic
```

---

### color

Apply color adjustments.

```bash
vfx color <INPUT> -o <OUTPUT> [ADJUSTMENTS]

Adjustments:
  --exposure <STOPS>          Exposure adjustment
  --gamma <VALUE>             Gamma correction
  --saturation <VALUE>        Saturation (0=gray, 1=normal, 2=saturated)
  --transfer <TYPE>           Transfer function
  --from <COLORSPACE>         Source color space
  --to <COLORSPACE>           Target color space

Transfer types:
  srgb                        sRGB to linear
  rec709                      Rec.709
  log                         Log
  pq                          PQ (HDR)

Options:
  --layer <NAME>              Process specific EXR layer
```

**Examples**:
```bash
vfx color input.exr -o output.exr --exposure 1.5
vfx color input.exr -o output.exr --gamma 2.2 --saturation 1.2
vfx color linear.exr -o srgb.png --from ACEScg --to sRGB
```

---

### blur

Apply blur effects.

```bash
vfx blur <INPUT> -o <OUTPUT> -r <RADIUS>

Options:
  -r, --radius <N>       Blur radius in pixels (default: 3)
  -t, --blur-type <TYPE> Blur type: box, gaussian (default: gaussian)
  --layer <NAME>         Process specific EXR layer
```

**Examples**:
```bash
vfx blur input.exr -o blurred.exr -r 5
vfx blur input.exr -o soft.exr -r 10 -t box
```

---

### aces

Apply ACES color transforms.

```bash
vfx aces <INPUT> -o <OUTPUT> -t <TYPE>

Transforms:
  idt, input, srgb-to-acescg    Input: sRGB → ACEScg
  rrt, tonemap                  RRT tonemap only
  odt, output, acescg-to-srgb   Output: ACEScg → sRGB
  rrt-odt, display, full        Full: ACEScg → sRGB display (default)

Options:
  -t, --transform <TYPE>        Transform type (default: rrt-odt)
  --rrt-variant <TYPE>          RRT variant: default, high-contrast
```

**Examples**:
```bash
vfx aces srgb.png -o acescg.exr -t idt
vfx aces render.exr -o display.png -t rrt-odt
vfx aces render.exr -o display.png -t display --rrt-variant high-contrast
```

---

### lut

Apply LUT (Look-Up Table).

```bash
vfx lut <INPUT> -o <OUTPUT> -l <LUT_FILE>

Options:
  -l, --lut <PATH>       LUT file (.cube, .clf)
  --invert               Invert LUT
```

**Examples**:
```bash
vfx lut input.exr -o graded.exr -l film_look.cube
vfx lut input.exr -o output.exr -l transform.clf --invert
```

---

### composite

Composite images together.

```bash
vfx composite <FG> <BG> -o <OUTPUT> -m <MODE>

Arguments:
  <FG>                   Foreground image
  <BG>                   Background image

Options:
  -o, --output <PATH>    Output image
  -m, --mode <TYPE>      Blend mode (default: over)
  --opacity <0-1>        Foreground opacity (default: 1.0)

Blend modes:
  over                   Alpha composite (default)
  add                    Additive
  multiply               Multiply
  screen                 Screen
```

**Examples**:
```bash
vfx composite fg.exr bg.exr -o comp.exr -m over
vfx composite layer.exr base.exr -o out.exr -m multiply --opacity 0.5
```

---

### transform

Apply geometric transforms (flip/rotate 90°/transpose).

```bash
vfx transform <INPUT> -o <OUTPUT> [OPERATIONS]

Operations:
  -r, --rotate <DEG>     Rotate by 90, 180, or 270 degrees
  --flip-h               Flip horizontal
  --flip-v               Flip vertical
  --transpose            Transpose (swap X/Y axes)
```

**Note:** For arbitrary-angle rotation, use `vfx rotate` instead.

**Examples**:
```bash
vfx transform input.exr -o rotated.exr -r 90
vfx transform input.exr -o flipped.exr --flip-h
vfx transform input.exr -o transposed.exr --transpose
```

---

### view

Interactive image viewer.

```bash
vfx view [INPUT]

Options:
  --ocio <PATH>          OCIO config file (overrides $OCIO)
  --display <NAME>       Display name (e.g., "sRGB")
  --view <NAME>          View transform name
  --cs <NAME>            Input color space (overrides metadata)

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

### rotate

Rotate image by arbitrary angle.

```bash
vfx rotate <INPUT> -o <OUTPUT> -a <ANGLE>

Options:
  -a, --angle <DEG>      Rotation angle in degrees (counter-clockwise)
  --bg-color <R,G,B>     Background color (default: 0,0,0)
```

**Examples**:
```bash
vfx rotate input.exr -o rotated.exr -a 45
vfx rotate input.exr -o rotated.exr -a 15 --bg-color 0,0,0
```

---

### warp

Apply lens distortion and warp effects.

```bash
vfx warp <INPUT> -o <OUTPUT> -t <TYPE>

Options:
  -t, --type <TYPE>      Warp type
  -k, --k1 <VALUE>       Primary parameter (default: 0.2)
  --k2 <VALUE>           Secondary parameter (default: 0.0)
  -r, --radius <VALUE>   Effect radius (default: 0.5)

Warp types:
  barrel                 Barrel distortion
  pincushion             Pincushion distortion
  fisheye                Fisheye effect
  twist                  Spiral/twirl effect
  wave                   Sinusoidal wave
  spherize               Spherical bulge
  ripple                 Concentric ripples
```

**Examples**:
```bash
vfx warp input.exr -o distorted.exr -t barrel -k 0.3
vfx warp input.exr -o twisted.exr -t twist -k 0.5 -r 0.8
```

---

### crop

Crop a region from an image.

```bash
vfx crop <INPUT> -o <OUTPUT> -x <X> -y <Y> -w <WIDTH> -H <HEIGHT>

Options:
  -x <N>                 X offset
  -y <N>                 Y offset
  -w <N>                 Width
  -H <N>                 Height
  --layer <NAME>         Process specific EXR layer
```

**Examples**:
```bash
vfx crop input.exr -o cropped.exr -x 100 -y 50 -w 800 -H 600
```

---

### diff

Compare two images.

```bash
vfx diff <A> <B> [-o <OUTPUT>]

Options:
  -o, --output <PATH>    Output difference image
  -t, --threshold <N>    Fail threshold (max allowed difference, default: 0.0)
  -w, --warn <N>         Per-pixel warning threshold
```

**Examples**:
```bash
vfx diff a.exr b.exr
vfx diff a.exr b.exr -o diff.exr -t 0.01
```

---

### sharpen

Apply sharpening filter.

```bash
vfx sharpen <INPUT> -o <OUTPUT> -a <AMOUNT>

Options:
  -a, --amount <N>       Sharpen amount (0.0-10.0, default: 1.0)
  --layer <NAME>         Process specific EXR layer
```

**Examples**:
```bash
vfx sharpen input.exr -o sharp.exr -a 1.5
```

---

### maketx

Create tiled/mipmapped texture.

```bash
vfx maketx <INPUT> -o <OUTPUT> [-m] [-t <SIZE>]

Options:
  -m, --mipmap           Generate mipmaps
  -t, --tile <N>         Tile size (default: 64)
  -f, --filter <TYPE>    Mipmap filter (default: lanczos)
  -w, --wrap <MODE>      Wrap mode: black, clamp, periodic (default: black)
```

**Examples**:
```bash
vfx maketx input.exr -o texture.exr -m -t 64
vfx maketx input.png -o texture.exr -m -f lanczos
```

---

### grep

Search for pattern in image filenames and properties.

```bash
vfx grep <PATTERN> <INPUT>...

Options:
  -i, --ignore-case      Case-insensitive search
```

**Note:** Searches filename, dimensions, and format only. Not EXIF/EXR metadata.

**Examples**:
```bash
vfx grep "1920" *.exr
vfx grep -i "shot" images/*.exr
```

---

### batch

Batch process multiple images.

```bash
vfx batch -i <PATTERN> -o <DIR> --op <OP> [--args <KEY=VALUE>...]

Options:
  -i, --input <GLOB>     Input glob pattern
  -o, --output-dir <DIR> Output directory
  --op <NAME>            Operation: convert, resize, blur, flip_h, flip_v
  -a, --args <K=V>       Operation arguments
  -f, --format <EXT>     Output format extension
```

**Examples**:
```bash
vfx batch -i "*.exr" -o output/ --op convert -f png
vfx batch -i "shots/*.exr" -o resized/ --op resize -a width=1920
```

---

### layers

List layers and channels in multi-layer EXR files.

```bash
vfx layers <INPUT>...

Options:
  --json                 Output as JSON
```

**Examples**:
```bash
vfx layers render.exr
vfx layers *.exr --json
```

---

### extract-layer

Extract a single layer from multi-layer EXR.

```bash
vfx extract-layer <INPUT> -o <OUTPUT> -l <LAYER>

Options:
  -l, --layer <NAME>     Layer name or index to extract
```

**Examples**:
```bash
vfx extract-layer render.exr -o beauty.exr -l beauty
vfx extract-layer render.exr -o diffuse.png -l "diffuse.R,diffuse.G,diffuse.B"
```

---

### merge-layers

Merge multiple images into one multi-layer EXR.

```bash
vfx merge-layers <INPUT>... -o <OUTPUT> [-n <NAME>...]

Options:
  -n, --names <NAME>     Custom layer names (one per input)
```

**Examples**:
```bash
vfx merge-layers beauty.exr diffuse.exr specular.exr -o combined.exr
vfx merge-layers a.exr b.exr -o out.exr -n layer_a -n layer_b
```

---

### channel-shuffle

Shuffle/rearrange image channels.

```bash
vfx channel-shuffle <INPUT> -o <OUTPUT> -p <PATTERN>

Options:
  -p, --pattern <PAT>    Channel pattern (e.g., BGR, RGBA, RRR, RGB1)
                         R/G/B/A = copy channel, 0 = black, 1 = white
```

**Examples**:
```bash
vfx channel-shuffle input.exr -o bgr.exr -p BGR
vfx channel-shuffle input.exr -o gray.exr -p RRR
vfx channel-shuffle rgb.exr -o rgba.exr -p RGBA
```

---

### channel-extract

Extract specific channels to new image.

```bash
vfx channel-extract <INPUT> -o <OUTPUT> -c <CHANNELS>...

Options:
  -c, --channels <CH>    Channels to extract (R/G/B/A/Z or index 0/1/2/...)
```

**Examples**:
```bash
vfx channel-extract input.exr -o red.exr -c R
vfx channel-extract input.exr -o rg.exr -c R -c G
vfx channel-extract depth.exr -o z.exr -c Z
```

---

### paste

Paste/overlay one image onto another.

```bash
vfx paste <BACKGROUND> <FOREGROUND> -o <OUTPUT>

Options:
  -x <N>                 X offset (default: 0, can be negative)
  -y <N>                 Y offset (default: 0, can be negative)
  -b, --blend            Use alpha blending
```

**Examples**:
```bash
vfx paste bg.exr overlay.exr -o result.exr -x 100 -y 50
vfx paste bg.exr fg.exr -o result.exr -b
```

---

### udim

UDIM texture set operations.

```bash
vfx udim <SUBCOMMAND>

Subcommands:
  info <PATTERN>         Show info about UDIM tiles
  convert <PATTERN>      Convert UDIM tiles
```

**Examples**:
```bash
vfx udim info "texture.<UDIM>.exr"
vfx udim convert "input.<UDIM>.exr" -o "output.<UDIM>.png"
```

---

### grade

Apply CDL grading (slope/offset/power/saturation).

```bash
vfx grade <INPUT> -o <OUTPUT>

Options:
  --slope <R,G,B>        Slope values (default: 1,1,1)
  --offset <R,G,B>       Offset values (default: 0,0,0)
  --power <R,G,B>        Power values (default: 1,1,1)
  --saturation <N>       Saturation (default: 1.0)
  --layer <NAME>         Process specific EXR layer
```

**Examples**:
```bash
vfx grade input.exr -o graded.exr --slope 1.1,1.0,0.9 --offset 0.01,0,0
vfx grade input.exr -o graded.exr --saturation 1.2
```

---

### clamp

Clamp pixel values to range.

```bash
vfx clamp <INPUT> -o <OUTPUT>

Options:
  --min <N>              Minimum value (default: 0.0)
  --max <N>              Maximum value (default: 1.0)
  --negatives            Clamp only negative values to 0
  --fireflies            Clamp only values > 1.0
  --layer <NAME>         Process specific EXR layer
```

**Examples**:
```bash
vfx clamp input.exr -o clamped.exr
vfx clamp input.exr -o clamped.exr --min 0.0 --max 2.0
vfx clamp render.exr -o clean.exr --negatives
vfx clamp hdr.exr -o ldr.exr --fireflies
```

---

### premult

Control alpha premultiplication.

```bash
vfx premult <INPUT> -o <OUTPUT> <--premultiply|--unpremultiply>

Options:
  --premultiply          Premultiply RGB by alpha
  --unpremultiply        Unpremultiply RGB (divide by alpha)
  --layer <NAME>         Process specific EXR layer
```

**Note:** One of `--premultiply` or `--unpremultiply` is required.

**Examples**:
```bash
vfx premult straight.exr -o premult.exr --premultiply
vfx premult premult.exr -o straight.exr --unpremultiply
```

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Error (any type) |

**Note:** All errors return code 1. Check stderr for details.

## Environment Variables

| Variable | Description |
|----------|-------------|
| `OCIO` | Default OCIO config path |
| `RUST_LOG` | Log level for tracing (trace, debug, info, warn, error) |

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
vfx convert "input.####.exr" "output.####.png"
```
