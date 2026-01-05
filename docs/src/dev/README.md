# Development Guide

This section covers development workflows for contributing to vfx-rs.

## Contents

- [Testing](./testing.md) - Running and writing tests
- [Benchmarks](./benchmarks.md) - Performance measurement
- [Adding Formats](./adding-formats.md) - Extending image format support
- [Adding Operations](./adding-ops.md) - Creating new image operations

## Development Setup

```bash
# Clone with full history (needed for some tests)
git clone https://github.com/yourname/vfx-rs.git
cd vfx-rs

# Build all crates
cargo build --workspace

# Run tests
cargo test --workspace

# Build with all features
cargo build --workspace --all-features
```

## Workspace Structure

```
vfx-rs/
├── crates/           # All library crates
│   ├── vfx-core/
│   ├── vfx-io/
│   └── ...
├── test/             # Integration tests + test assets
│   ├── images/       # Test images (EXR, PNG, etc.)
│   └── luts/         # Test LUTs
├── docs/             # This documentation (mdbook)
└── Cargo.toml        # Workspace manifest
```

## Code Style

- **Formatting**: `cargo fmt` (rustfmt defaults)
- **Linting**: `cargo clippy --workspace --all-features`
- **Documentation**: All public items must have doc comments
- **Naming**: snake_case for functions, CamelCase for types
- **Errors**: Use `anyhow::Result` for applications, `thiserror` for libraries

## Feature Flags

Many crates have feature flags to control optional dependencies:

```bash
# Build with specific features
cargo build -p vfx-io --features="exr,png"

# Build with all features
cargo build -p vfx-color --all-features
```

## Commit Messages

Follow conventional commits:

```
feat(io): add TIFF support
fix(color): correct sRGB EOTF threshold
docs(cli): document --layer flag
perf(ops): SIMD resize implementation
```

## Pull Request Workflow

1. Fork and create feature branch
2. Write tests for new functionality
3. Ensure `cargo test --workspace` passes
4. Run `cargo clippy` and fix warnings
5. Update documentation if needed
6. Submit PR with clear description
