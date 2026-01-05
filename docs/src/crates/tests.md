# vfx-tests

Integration tests for vfx-rs.

## Purpose

Cross-crate integration tests that verify the complete vfx-rs stack works correctly together. Tests real-world workflows rather than isolated unit functionality.

## Structure

```
crates/vfx-tests/
├── Cargo.toml
└── tests/
    ├── io_roundtrip.rs      # Format read/write cycles
    ├── color_pipeline.rs    # End-to-end color transforms
    ├── aces_workflow.rs     # ACES IDT→RRT→ODT chain
    ├── ocio_config.rs       # Config loading and processing
    └── cli_commands.rs      # CLI integration tests
```

## Running Tests

```bash
# Run all integration tests
cargo test -p vfx-tests

# Run specific test file
cargo test -p vfx-tests --test io_roundtrip

# Run with output
cargo test -p vfx-tests -- --nocapture

# Run ignored (slow) tests
cargo test -p vfx-tests -- --ignored
```

## Test Categories

### I/O Roundtrip

Verify format codecs preserve data:

```rust
#[test]
fn exr_roundtrip() {
    let original = create_test_image(256, 256, 4);
    
    let tmp = tempfile::NamedTempFile::new().unwrap();
    vfx_io::write(tmp.path(), &original).unwrap();
    let loaded = vfx_io::read(tmp.path()).unwrap();
    
    assert_images_equal(&original, &loaded, 1e-5);
}
```

### Color Pipeline

End-to-end color transforms:

```rust
#[test]
fn srgb_to_acescg_roundtrip() {
    let srgb = [0.5f32, 0.3, 0.2];
    
    // Forward: sRGB → ACEScg
    let aces = pipeline_srgb_to_acescg(srgb);
    
    // Inverse: ACEScg → sRGB
    let back = pipeline_acescg_to_srgb(aces);
    
    assert_rgb_close(srgb, back, 1e-4);
}
```

### ACES Workflow

Complete ACES pipeline:

```rust
#[test]
fn aces_display_pipeline() {
    // Load scene-referred EXR
    let scene = vfx_io::read("test/images/scene_linear.exr").unwrap();
    
    // Apply RRT+ODT
    let data = scene.to_f32();
    let display = vfx_color::aces::apply_rrt_odt_srgb(&data, 3);
    
    // Verify output is display-referred (0-1 range)
    for v in &display {
        assert!(*v >= 0.0 && *v <= 1.0);
    }
}
```

### OCIO Config

Config loading and validation:

```rust
#[test]
fn load_aces_config() {
    let config = vfx_ocio::builtin::aces_1_3();
    
    // Verify expected color spaces exist
    assert!(config.colorspace("ACEScg").is_some());
    assert!(config.colorspace("sRGB").is_some());
    
    // Verify role resolution
    let linear = config.colorspace("scene_linear").unwrap();
    assert_eq!(linear.name(), "ACEScg");
}
```

## Test Assets

Located in `test/` at workspace root:

```
test/
├── images/
│   ├── tiny.exr         # 8x8 reference (f16)
│   ├── gradient.png     # 256x256 gradient
│   ├── checker.exr      # Pattern for resize tests
│   └── multilayer.exr   # Multi-layer test
├── luts/
│   ├── identity.cube    # 33^3 identity
│   ├── gamma22.cube     # 1D gamma 2.2
│   └── test.clf         # CLF format test
└── configs/
    └── test.ocio        # Minimal OCIO config
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

### Assertions

```rust
fn assert_images_equal(a: &ImageData, b: &ImageData, tolerance: f32) {
    assert_eq!(a.width, b.width);
    assert_eq!(a.height, b.height);
    assert_eq!(a.channels, b.channels);
    
    let a_data = a.to_f32();
    let b_data = b.to_f32();
    
    for (i, (va, vb)) in a_data.iter().zip(b_data.iter()).enumerate() {
        assert!(
            (va - vb).abs() < tolerance,
            "Pixel {} differs: {} vs {}", i, va, vb
        );
    }
}

fn assert_rgb_close(a: [f32; 3], b: [f32; 3], tol: f32) {
    for i in 0..3 {
        assert!((a[i] - b[i]).abs() < tol);
    }
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
vfx-io = { workspace = true }
vfx-ops = { workspace = true }
vfx-color = { workspace = true }
vfx-ocio = { workspace = true }

[dev-dependencies]
tempfile = { workspace = true }
approx = { workspace = true }
```
