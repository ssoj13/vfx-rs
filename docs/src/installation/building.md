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

## Build without Optional Features

```bash
# Minimal build (no HEIF, WebP, JP2)
cargo build --release -p vfx-cli --no-default-features \
  --features exr,png,jpeg,tiff,dpx,hdr
```

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
