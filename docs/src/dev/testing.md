# Testing

vfx-rs uses a multi-tier testing strategy: unit tests in each crate, integration tests in `vfx-tests`, and visual comparison tests.

## Running Tests

```bash
# All tests
cargo test --workspace

# Specific crate
cargo test -p vfx-io

# Specific test
cargo test -p vfx-color test_srgb_roundtrip

# With features
cargo test -p vfx-io --features="exr,png"

# Show output
cargo test --workspace -- --nocapture
```

## Test Organization

### Unit Tests

Each crate has inline unit tests in `src/`:

```rust
// In vfx-transfer/src/srgb.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oetf_eotf_roundtrip() {
        for i in 0..=100 {
            let v = i as f32 / 100.0;
            let encoded = oetf(v);
            let decoded = eotf(encoded);
            assert!((v - decoded).abs() < 1e-6);
        }
    }
}
```

### Integration Tests

The `vfx-tests` crate contains integration tests:

```
crates/vfx-tests/
├── Cargo.toml
└── src/
    ├── lib.rs        # Integration tests (inline #[cfg(test)] mod)
    ├── golden.rs     # Golden parity tests (OCIO comparison)
    └── bin/          # Test binaries
```

### Test Assets

Test assets live in `test/` at workspace root:

```
test/
├── assets/
│   └── OpenColorIO-Config-ACES/   # ACES OCIO config
├── assets-exr/                     # EXR test files
└── *.exr, *.jpg                    # Various test images
```

## Writing Tests

### Basic Test

```rust
#[test]
fn test_resize_dimensions() {
    let data = vec![1.0f32; 100 * 100 * 3];
    let result = resize_f32(&data, 100, 100, 3, 50, 50, Filter::Bilinear)
        .unwrap();
    assert_eq!(result.len(), 50 * 50 * 3);
}
```

### Floating-Point Comparisons

Never use `==` for floats:

```rust
#[test]
fn test_gamma() {
    let result = apply_gamma(0.5, 2.2);
    let expected = 0.5f32.powf(2.2);
    assert!((result - expected).abs() < 1e-6, 
            "Expected {}, got {}", expected, result);
}
```

### Image Comparison

```rust
use vfx_tests::compare_images;

#[test]
fn test_blur_output() {
    let input = load_image("test/images/input.exr").unwrap();
    let result = apply_blur(&input, 5);
    
    let reference = load_image("test/images/blur_ref.exr").unwrap();
    let diff = compare_images(&result, &reference);
    
    assert!(diff.max_diff < 0.001, "Max pixel diff: {}", diff.max_diff);
    assert!(diff.rms < 0.0001, "RMS diff: {}", diff.rms);
}
```

### Property-Based Tests

Use `proptest` for edge cases:

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_clamp_bounds(v in -10.0f32..10.0) {
        let result = clamp(v, 0.0, 1.0);
        prop_assert!(result >= 0.0);
        prop_assert!(result <= 1.0);
    }
}
```

## Test Categories

### IO Tests

- Format read/write roundtrip
- Metadata preservation
- Multi-layer EXR handling
- Sequence detection

```rust
#[test]
fn test_exr_roundtrip() {
    let original = ImageData::new(256, 256, 4);
    // ... fill with test pattern
    
    save_image("test_output.exr", &original).unwrap();
    let loaded = load_image("test_output.exr").unwrap();
    
    assert_eq!(original.width, loaded.width);
    assert_eq!(original.height, loaded.height);
    // Compare pixel data...
}
```

### Color Tests

- Transfer function roundtrips
- Matrix accuracy
- ACES transform chains
- Primaries conversions

```rust
#[test]
fn test_acescg_to_srgb_roundtrip() {
    let rgb = [0.18, 0.18, 0.18]; // 18% gray
    
    let acescg = srgb_to_acescg(rgb[0], rgb[1], rgb[2]);
    let back = acescg_to_srgb(acescg.0, acescg.1, acescg.2);
    
    assert!((rgb[0] - back.0).abs() < 1e-5);
}
```

### Ops Tests

- Resize accuracy
- Filter kernels
- Composite modes
- Transform geometry

## Test Utilities

The `vfx-tests` crate provides helpers:

```rust
use vfx_tests::{
    test_image,        // Generate test patterns
    compare_images,    // Pixel-wise comparison
    approx_eq,         // Float comparison with epsilon
    temp_file,         // Temporary file path
};

#[test]
fn test_with_utilities() {
    let img = test_image::checkerboard(256, 256, 3);
    let path = temp_file("exr");
    
    save_image(&path, &img).unwrap();
    let loaded = load_image(&path).unwrap();
    
    assert!(compare_images(&img, &loaded).max_diff < 1e-6);
}
```

## CI Testing

GitHub Actions runs tests on every PR:

```yaml
# .github/workflows/test.yml
- name: Run tests
  run: cargo test --workspace --all-features
  
- name: Clippy
  run: cargo clippy --workspace -- -D warnings
```

## Debugging Test Failures

```bash
# Run single test with backtrace
RUST_BACKTRACE=1 cargo test -p vfx-io test_exr_load -- --nocapture

# Run ignored tests (slow tests)
cargo test --workspace -- --ignored

# Generate test coverage
cargo tarpaulin --workspace --out Html
```

## Test Data Generation

For generating reference images:

```bash
# Use CLI to create test data
vfx resize input.exr -w 256 -H 256 -o test/images/small.exr
vfx color input.exr --exposure 1.0 -o test/images/bright.exr
```

## Performance Regression Tests

Critical paths have benchmarks that can detect regressions:

```bash
# Run benchmarks
cargo bench -p vfx-bench

# Compare against baseline
cargo bench -p vfx-bench -- --baseline main
```
