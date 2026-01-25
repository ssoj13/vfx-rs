# vfx-tests

Integration tests for vfx-rs.

## Purpose

Cross-crate integration tests that verify the complete vfx-rs stack works correctly together. Tests real-world workflows rather than isolated unit functionality.

## Structure

```
crates/vfx-tests/
├── Cargo.toml
└── src/
    ├── lib.rs        # Integration tests (inline #[cfg(test)] mod)
    ├── golden.rs     # Golden parity tests (OCIO comparison)
    └── bin/          # Test binaries
```

## Running Tests

```bash
# Run all integration tests
cargo test -p vfx-tests

# Run golden parity tests only
cargo test -p vfx-tests golden

# Run with output
cargo test -p vfx-tests -- --nocapture

# Run ignored (slow) tests
cargo test -p vfx-tests -- --ignored
```

## Test Categories

### I/O Roundtrip

Verify format codecs preserve data. Tests are inline in `src/lib.rs`:

```rust
#[test]
fn test_io_roundtrip_exr() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("test.exr");

    let image = ImageData::from_f32(64, 64, 4, data.clone());
    vfx_io::write(&path, &image).expect("Failed to write EXR");
    let loaded = vfx_io::read(&path).expect("Failed to read EXR");

    // Verify dimensions and data match
    assert_eq!(loaded.width, 64);
    for (orig, load) in data.iter().zip(loaded.to_f32().iter()) {
        assert!((orig - load).abs() < 1e-5);
    }
}
```

### Golden Parity Tests

The `golden` module verifies vfx-rs output matches PyOpenColorIO exactly:

```bash
# Generate golden reference data
python tests/parity/generate_golden.py

# Run golden tests
cargo test --package vfx-tests golden
```

## Test Assets

Located in `test/` at workspace root:

```
test/
├── assets/
│   └── OpenColorIO-Config-ACES/   # ACES OCIO config
├── assets-exr/                     # EXR test files
└── *.exr, *.jpg                    # Various test images
```

## Writing Tests

### Test Image Creation

```rust
use vfx_io::ImageData;

fn create_gradient(w: u32, h: u32) -> ImageData {
    let mut data = vec![0.0f32; (w * h * 3) as usize];
    for y in 0..h {
        for x in 0..w {
            let idx = ((y * w + x) * 3) as usize;
            data[idx] = x as f32 / w as f32;     // R
            data[idx + 1] = y as f32 / h as f32; // G
            data[idx + 2] = 0.5;                  // B
        }
    }
    ImageData::from_f32(w, h, 3, data)
}
```

### Temporary Files

```rust
use tempfile::TempDir;

#[test]
fn test_with_temp_dir() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.exr");
    
    // Test...
    
    // TempDir auto-cleans on drop
}
```

## Slow Tests

Mark expensive tests as ignored:

```rust
#[test]
#[ignore]  // Run with: cargo test -- --ignored
fn large_image_processing() {
    let img = create_test_image(4096, 4096, 4);
    // Expensive operation...
}
```

## Dependencies

```toml
[dependencies]
vfx-core = { workspace = true }
vfx-math = { workspace = true }
vfx-lut = { workspace = true }
vfx-transfer = { workspace = true }
vfx-primaries = { workspace = true }
vfx-color = { workspace = true }
vfx-io = { workspace = true }
vfx-ops = { workspace = true }
tempfile = { workspace = true }
serde = { workspace = true }
serde_json = "1.0"
sha2 = "0.10"
```

**Note:** All dependencies are under `[dependencies]`, not `[dev-dependencies]`, as this is a test crate.
