# Feature Flags

vfx-rs uses Cargo features to enable/disable format support and optional functionality.

## vfx-io Features

| Feature | Default | Description |
|---------|---------|-------------|
| `exr` | Yes | OpenEXR support (via `vfx-exr` crate) |
| `png` | Yes | PNG support |
| `jpeg` | Yes | JPEG read/write |
| `tiff` | Yes | TIFF with LZW compression |
| `dpx` | Yes | DPX (10/12/16-bit log) |
| `hdr` | Yes | Radiance RGBE |
| `heif` | No | HEIF/HEIC (requires libheif) |
| `webp` | No | WebP (via image crate) |
| `avif` | No | AVIF (write-only) |
| `jp2` | No | JPEG2000 (requires OpenJPEG) |
| `psd` | No | Photoshop PSD/PSB |
| `dds` | No | DirectDraw Surface textures |
| `ktx` | No | Khronos KTX2 format |
| `text` | No | Text rendering (cosmic-text) |
| `rayon` | No | Parallel processing |

## vfx-cli Features

| Feature | Default | Description |
|---------|---------|-------------|
| `viewer` | Yes | Built-in image viewer (vfx-view) |

**Note:** Format features (exr, png, etc.) are in `vfx-io`, not `vfx-cli`. The CLI includes all default vfx-io formats via its workspace dependency.

## Examples

```bash
# Build CLI with viewer disabled
cargo build -p vfx-cli --release --no-default-features

# Build vfx-io with minimal formats
cargo build -p vfx-io --no-default-features --features exr,png

# Build workspace with specific vfx-io features
cargo build --release --no-default-features -F vfx-io/exr,vfx-io/png

# Build with HEIF support (requires libheif)
cargo build --release -F vfx-io/heif

# Build with all optional formats
cargo build --release --all-features
```

## Feature Detection at Runtime

```rust
use vfx_io::Format;

// Detect format from extension
let format = Format::from_extension("heic");
match format {
    Some(Format::Heif) => println!("HEIF format recognized"),
    None => println!("Unknown extension"),
    _ => {}
}

// Check if format is available at compile time
#[cfg(feature = "heif")]
fn read_heif(path: &str) -> Result<ImageBuf> {
    vfx_io::read(path)
}
```
