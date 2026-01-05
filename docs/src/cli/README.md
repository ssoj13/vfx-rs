# CLI Reference

The `vfx` command provides OIIO-like functionality for image processing.

## Global Options

```
-v, --verbose    Increase verbosity (-v info, -vv debug, -vvv trace)
-l, --log [PATH] Write log to file (default: vfx.log)
--allow-non-color  Allow operations on non-color data (IDs, normals)
```

## Commands Overview

| Command | Alias | Description | OIIO Equivalent |
|---------|-------|-------------|-----------------|
| `info` | `i` | Display image metadata | `iinfo` |
| `convert` | `c` | Convert format/depth | `iconvert` |
| `resize` | `r` | Scale image | `oiiotool --resize` |
| `crop` | | Extract region | `oiiotool --crop` |
| `diff` | `d` | Compare images | `idiff` |
| `composite` | | Layer compositing | `oiiotool --over` |
| `color` | | Color transforms | OCIO |
| `aces` | | ACES transforms | OCIO ACES config |
| `lut` | | Apply LUT | `ociobakelut` |
| `layers` | | EXR layer operations | `oiiotool --ch` |
| `channels` | | Channel shuffle/extract | `oiiotool --ch` |
| `batch` | | Batch processing | `oiiotool` scripts |
| `transform` | | Flip/rotate/transpose | `oiiotool --flip` |
| `udim` | | UDIM texture operations | `maketx` |
| `view` | | Interactive viewer | `iv` |

## Common Patterns

```bash
# Pipeline: EXR -> resize -> color -> PNG
vfx convert input.exr temp.exr && \
vfx resize temp.exr -s 0.5 -o half.exr && \
vfx color half.exr -o output.png --from ACEScg --to sRGB

# Batch with verbose logging
vfx batch "shots/*.exr" -o proxies/ --op resize \
  --args width=1920 -vv --log=batch.log

# Debug mode for troubleshooting
vfx info -vvv problem.exr 2>&1 | tee debug.log
```
