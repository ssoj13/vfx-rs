# Internals

This section documents the internal implementation details of vfx-rs. Understanding these internals is useful for:

- Contributing to vfx-rs
- Debugging issues
- Extending functionality
- Performance optimization

## Topics

- [Processing Pipeline](pipeline.md) - How images flow through the system
- [EXR Implementation](exr.md) - OpenEXR reading and writing
- [Color Implementation](color.md) - Color transform internals
- [GPU Compute](gpu.md) - wgpu compute shader architecture

## Code Organization

Each crate follows a consistent structure:

```
crates/vfx-xxx/
├── Cargo.toml
├── src/
│   ├── lib.rs          # Public API, re-exports
│   ├── error.rs        # Error types
│   └── [modules]       # Implementation
└── tests/              # Unit tests
```

## Naming Conventions

| Pattern | Usage |
|---------|-------|
| `apply_*` | In-place mutation |
| `to_*` | Conversion returning new value |
| `from_*` | Construction from another type |
| `*_f32` | f32-specific variant |
| `*_rgb` | RGB-specific variant |

## Error Handling Pattern

Library crates use `thiserror`:

```rust
#[derive(thiserror::Error, Debug)]
pub enum MyError {
    #[error("invalid dimension: {0}")]
    InvalidDimension(String),
    
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

pub type MyResult<T> = Result<T, MyError>;
```

Application code uses `anyhow`:

```rust
fn main() -> anyhow::Result<()> {
    let img = vfx_io::read(&path)?;
    // Errors automatically converted
    Ok(())
}
```

## Testing Strategy

1. **Unit tests** - In each module (`#[cfg(test)]`)
2. **Integration tests** - In `vfx-tests` crate
3. **Benchmarks** - In `vfx-bench` crate
4. **Doc tests** - In documentation comments

## Performance Considerations

### Parallelism

Operations use Rayon when beneficial:

```rust
// Parallel iteration over rows
(0..height).into_par_iter().for_each(|y| {
    // Process row
});

// Parallel chunks
data.par_chunks_mut(chunk_size).for_each(|chunk| {
    // Process chunk
});
```

### SIMD

Where applicable, via `glam` and `wide`:

```rust
use wide::f32x8;

// Process 8 pixels at once
for chunk in data.chunks_exact_mut(8) {
    let v = f32x8::from(chunk);
    let result = v * scale;
    chunk.copy_from_slice(&result.to_array());
}
```

### Memory

- Prefer in-place operations
- Use `Vec::with_capacity` when size known
- Avoid unnecessary allocations in hot paths

## Feature Flags

Used for:
- Optional format support
- GPU backends
- Expensive dependencies

```toml
[features]
default = ["exr", "png"]
gpu = ["wgpu"]
all-formats = ["exr", "png", "jpeg", "tiff", "dpx", "heif", "webp", "avif"]
```
