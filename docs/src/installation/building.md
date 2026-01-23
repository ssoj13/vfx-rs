# Building from Source

## Clone and Build

```bash
git clone https://github.com/vfx-rs/vfx-rs
cd vfx-rs

# Debug build (faster compile, slower runtime)
cargo build

# Release build (slower compile, optimized)
cargo build --release

# Run tests
cargo test --workspace
```

## Build Specific Crates

```bash
# CLI only
cargo build -p vfx-cli --release

# Library only (no CLI)
cargo build -p vfx-io -p vfx-ops --release

# Python bindings
cd crates/vfx-rs-py
maturin build --release
```

## Build with All Features

```bash
cargo build --release --all-features
```

## Build with Specific Format Features

Format features (exr, png, jpeg, etc.) are controlled in `vfx-io`, not `vfx-cli`.
The CLI includes all default formats via its dependency on vfx-io.

```bash
# Build vfx-io with specific formats only
cargo build -p vfx-io --no-default-features --features exr,png,jpeg

# Build workspace with minimal formats
cargo build --release --no-default-features -F vfx-io/exr,vfx-io/png

# Build CLI with viewer disabled
cargo build -p vfx-cli --release --no-default-features
```

### Available vfx-io Features

| Feature | Description |
|---------|-------------|
| `exr` | OpenEXR format (default) |
| `png` | PNG format (default) |
| `jpeg` | JPEG read/write (default) |
| `tiff` | TIFF format (default) |
| `dpx` | DPX format (default) |
| `hdr` | Radiance HDR (default) |
| `webp` | WebP format |
| `avif` | AVIF format |
| `jp2` | JPEG2000 (requires OpenJPEG) |
| `psd` | Photoshop PSD/PSB |
| `dds` | DirectDraw Surface |
| `ktx` | Khronos KTX2 |
| `heif` | HEIF/HEIC (requires libheif) |
| `text` | Text rendering |
| `rayon` | Parallel processing |

### vfx-cli Features

| Feature | Description |
|---------|-------------|
| `viewer` | Interactive image viewer (default) |

## Cross-Compilation

```bash
# Add target
rustup target add x86_64-unknown-linux-gnu

# Build for Linux from Windows/macOS
cargo build --release --target x86_64-unknown-linux-gnu
```

## Profile Settings

The workspace uses optimized profiles:

```toml
[profile.release]
lto = "thin"        # Link-time optimization
codegen-units = 1   # Better optimization

[profile.dev]
opt-level = 1       # Faster dev builds with some optimization
```
