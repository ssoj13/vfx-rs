# Design Decisions

This page documents key architectural decisions and their rationale.

## Why Pure Rust (Mostly)?

**Decision**: Write core functionality in pure Rust, binding to C libraries only when necessary.

**Rationale**:
- **Memory safety** - VFX pipelines process untrusted files (artist uploads, third-party assets). Rust eliminates entire classes of vulnerabilities.
- **Concurrency** - Parallel image processing is trivial with Rayon. No data races, no undefined behavior.
- **Packaging** - Single binary distribution. No runtime dependencies to manage.
- **Cross-compilation** - Build for Linux, Windows, macOS, ARM from any platform.

**Exceptions**:
- `lcms2` for ICC profiles - implementing ICC correctly is a multi-year effort
- `wgpu` backends use native graphics APIs (Vulkan, Metal, DX12)
- Optional system libraries for exotic formats (libheif, OpenJPEG)

## Why Not Wrap OIIO/OCIO?

**Decision**: Native Rust implementation rather than FFI bindings.

**Rationale**:
- **Compile times** - C++ binding generation is slow and fragile
- **Binary size** - OIIO/OCIO pull in Boost, OpenEXR C++, etc.
- **Maintenance** - Tracking upstream API changes across major versions
- **Customization** - Can optimize for common VFX workflows
- **Learning** - Educational value of implementing color science from scratch

**Tradeoffs**:
- More initial development effort
- Must maintain format compatibility ourselves
- May lag behind OIIO/OCIO for new features

## f32 as Working Format

**Decision**: Convert all images to `f32` for processing.

**Rationale**:
- **HDR support** - f32 handles values > 1.0 naturally
- **Precision** - No banding in gradients
- **Simplicity** - One code path for all operations
- **GPU compatibility** - GPUs work best with f32

**Memory cost**: 4x compared to u8. Acceptable for single-image processing; batch operations can stream.

## Layered Crate Architecture

**Decision**: Split functionality across many small crates.

**Rationale**:
- **Compile times** - Change one crate, rebuild only dependents
- **Feature selection** - Use only what you need
- **Clear APIs** - Forced to think about interfaces
- **Testing** - Each crate is independently testable

**Example**: Need only image I/O? Use `vfx-io` alone (no color management overhead).

## Optional Format Support

**Decision**: Each image format is a feature flag.

```toml
[features]
default = ["exr", "png", "jpeg", "tiff", "dpx", "hdr"]
heif = ["dep:libheif-rs"]  # Requires system library
```

**Rationale**:
- **Build times** - Don't compile unused format code
- **Binary size** - Smaller when features disabled
- **System deps** - HEIF/JP2K need external libraries

**Tradeoff**: Users must know to enable features.

## Explicit vs Implicit Color Management

**Decision**: Color transforms are explicit function calls, not automatic.

```rust
// Explicit - user controls the pipeline
let linear = vfx_transfer::srgb::eotf(srgb_value);
let display = vfx_color::aces::apply_rrt_odt_srgb(&data, c);

// NOT implicit (like some frameworks)
// image.set_colorspace("sRGB");  // No magic
```

**Rationale**:
- **Predictability** - No surprise transforms
- **Performance** - No redundant conversions
- **Debugging** - Easy to trace color pipeline
- **Composability** - Build custom pipelines

## Rayon for Parallelism

**Decision**: Use Rayon for CPU parallelism.

**Rationale**:
- **Work stealing** - Automatically balances load
- **Simple API** - `.par_iter()` instead of `.iter()`
- **Zero overhead** - Falls back to sequential when threading isn't worth it
- **Ecosystem** - Standard choice in Rust graphics

**Example**:
```rust
// Sequential
data.chunks_mut(channels).for_each(|px| { ... });

// Parallel (one word change)
data.par_chunks_mut(channels).for_each(|px| { ... });
```

## wgpu for GPU Compute

**Decision**: Use wgpu for cross-platform GPU compute.

**Rationale**:
- **Cross-platform** - Vulkan (Linux/Windows), Metal (macOS), DX12 (Windows)
- **WebGPU standard** - Future web support possible
- **Safe API** - Validation layers catch errors
- **Active development** - Strong community

**Alternative considered**: Vulkano (too low-level), CUDA-only (not portable).

## Error Handling Strategy

**Decision**: `thiserror` for libraries, `anyhow` for applications.

```rust
// Library crate - typed errors
#[derive(thiserror::Error)]
pub enum IoError {
    #[error("unsupported format: {0}")]
    UnsupportedFormat(String),
}

// Application crate - erased errors
fn main() -> anyhow::Result<()> {
    let image = vfx_io::read(&path)?;
    Ok(())
}
```

**Rationale**:
- Libraries: Callers need to pattern-match errors
- Applications: Just need to display errors to users

## Logging with tracing

**Decision**: Use `tracing` crate for structured logging.

**Rationale**:
- **Structured** - Key-value pairs, not just strings
- **Hierarchical** - Spans show call context
- **Async-ready** - Works with async code
- **Ecosystem** - Integrates with many backends

```rust
trace!(input = %path, "loading image");
info!(width = w, height = h, "resize complete");
```

## LUT-Based Color Space Conversions

**Decision**: Gamut mapping uses 3D LUT interpolation.

**Rationale**:
- **Accuracy** - Pre-computed tables avoid numerical errors
- **Performance** - Lookup + interpolation faster than matrix chains
- **GPU-friendly** - 3D textures are hardware-optimized

**Tradeoff**: Memory usage for LUT storage (typically 32-128 KB per LUT).

## No Global Config State

**Decision**: No global OCIO config or context.

```rust
// NOT like OCIO
// OCIO::SetCurrentConfig(config);

// Explicit config passing
let config = vfx_ocio::Config::from_file("config.ocio")?;
let processor = config.processor("ACEScg", "sRGB - Display")?;
```

**Rationale**:
- **Thread safety** - No global mutable state
- **Testing** - Easy to mock configs
- **Multiple configs** - Can use different configs in same process

## Test Image Assets

**Decision**: Include test images in repository.

```
test/
├── images/
│   ├── tiny.exr (8x8 reference)
│   ├── gradient.png
│   └── ...
└── luts/
    └── test.cube
```

**Rationale**:
- **Reproducibility** - Tests work offline
- **Known values** - Pixel-exact verification
- **Fast CI** - No network fetches

**Tradeoff**: Repo size. Mitigated by using tiny test images.
