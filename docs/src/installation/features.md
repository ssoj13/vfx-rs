# Feature Flags

vfx-rs uses Cargo features to enable/disable format support and optional functionality.

## vfx-io Features

| Feature | Default | Description |
|---------|---------|-------------|
| `exr` | ✅ | OpenEXR support (via `exr` crate) |
| `png` | ✅ | PNG support |
| `jpeg` | ✅ | JPEG read/write |
| `tiff` | ✅ | TIFF with LZW compression |
| `dpx` | ✅ | DPX (10/12/16-bit log) |
| `hdr` | ✅ | Radiance RGBE |
| `heif` | ❌ | HEIF/HEIC (requires libheif) |
| `webp` | ❌ | WebP (via image crate) |
| `avif` | ❌ | AVIF (write-only) |
| `jp2` | ❌ | JPEG2000 (requires OpenJPEG) |

## vfx-ops Features

| Feature | Default | Description |
|---------|---------|-------------|
| `parallel` | ✅ | Rayon parallelization |
| `fft` | ❌ | FFT-based operations (rustfft) |

## vfx-cli Features

| Feature | Default | Description |
|---------|---------|-------------|
| `viewer` | ✅ | Built-in image viewer |

## Examples

```bash
# Minimal build
cargo build -p vfx-cli --no-default-features --features exr,png

# With HEIF support
cargo build -p vfx-cli --features heif

# Without viewer (headless server)
cargo build -p vfx-cli --no-default-features \
  --features exr,png,jpeg,tiff,dpx,hdr

# All formats
cargo build -p vfx-cli --all-features
```

## Feature Detection at Runtime

```rust
// Check if format is supported
use vfx_io::Format;

let format = Format::detect("image.heic");
match format {
    Ok(Format::Heif) => println!("HEIF supported"),
    Err(_) => println!("HEIF not compiled in"),
}
```
