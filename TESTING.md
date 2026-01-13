# VFX-RS Testing Strategy

## Overview

VFX-RS uses a **hybrid testing approach** to ensure mathematical correctness and OCIO parity:

1. **Unit Tests** - Rust tests for individual functions
2. **Roundtrip Tests** - Encode/decode verification
3. **Python Parity Tests** - Bit-exact comparison with PyOpenColorIO
4. **Golden Hash Tests** - Fast Rust tests using pre-computed reference hashes

## Test Structure

```
vfx-rs/
├── crates/*/src/**/*.rs     # Unit tests (#[test] in modules)
├── crates/*/tests/*.rs      # Integration tests
├── tests/
│   ├── parity/              # Python parity test suite
│   │   ├── conftest.py      # Pytest fixtures, OCIO setup
│   │   ├── test_transfer_parity.py
│   │   ├── test_matrix_parity.py
│   │   ├── test_lut_parity.py
│   │   ├── test_ops_parity.py
│   │   ├── test_processor_parity.py
│   │   └── generate_golden.py
│   └── golden/              # Reference data
│       ├── hashes.json      # Pixel hashes for Rust tests
│       ├── reference_images/
│       └── expected_outputs/
└── crates/vfx-tests/        # Rust golden tests
```

## Running Tests

### Quick: Rust Unit Tests

```bash
# All tests
cargo test --workspace

# Specific crate
cargo test -p vfx-transfer
cargo test -p vfx-lut
cargo test -p vfx-ocio

# With output
cargo test --workspace -- --nocapture
```

### Full: Python Parity Tests

**Prerequisites:**
```bash
pip install pytest numpy PyOpenColorIO
pip install maturin  # for building vfx-rs-py
```

**Build and test:**
```bash
# Build Python bindings
cd crates/vfx-rs-py
maturin develop --release

# Run parity tests
cd ../../tests/parity
pytest -v

# Run specific test category
pytest test_transfer_parity.py -v
pytest test_matrix_parity.py -v
```

### Regenerate Golden Hashes

When OCIO reference changes or new tests added:

```bash
cd tests/parity
python generate_golden.py
```

This updates `tests/golden/hashes.json` used by Rust tests.

## Test Categories

### 1. Transfer Functions (22 total)

| Category | Functions | Tests |
|----------|-----------|-------|
| Display | sRGB, Gamma, Rec.709 | roundtrip, reference values |
| HDR | PQ, HLG | roundtrip, range checks |
| Camera Log | LogC3/4, S-Log2/3, V-Log, Canon Log 1/2/3, RED Log, BMD Film | roundtrip, OCIO parity |
| ACES | ACEScc, ACEScct | roundtrip, AMPAS spec values |

**Tolerance:** 1e-4 relative error

### 2. Color Space Matrices

| Transform | Reference |
|-----------|-----------|
| sRGB <-> XYZ | IEC 61966-2-1 |
| Rec.2020 <-> XYZ | ITU-R BT.2020 |
| ACES AP0/AP1 <-> XYZ | AMPAS spec |
| Camera gamuts | OCIO BuiltinTransforms |
| Chromatic adaptation | OCIO CAT matrices |

**Tolerance:** 1e-5 absolute error (matrices are precise)

### 3. LUT Operations

| Test | Description |
|------|-------------|
| 1D LUT linear interp | Compare vs OCIO Lut1DOp |
| 3D LUT trilinear | Compare vs OCIO Lut3DOp |
| 3D LUT tetrahedral | Compare vs OCIO tetrahedral |
| Shaper + 3D combo | Full FileTransform chain |

**Tolerance:** 1e-4 (interpolation can vary slightly)

### 4. Grading Operations

| Op | OCIO Reference |
|----|----------------|
| CDL (SOP + Sat) | CDLOpData.cpp |
| ExposureContrast | ExposureContrastOpData.cpp |
| GradingPrimary | GradingPrimaryOpData.cpp |
| GradingTone | GradingToneOpData.cpp |
| GradingRGBCurve | GradingRGBCurveOpData.cpp |
| Range | RangeOpData.cpp |

**Tolerance:** 1e-4

### 5. Fixed Functions

| Function | OCIO Reference |
|----------|----------------|
| ACES_RED_MOD_03/10 | FixedFunctionOpCPU.cpp |
| ACES_GLOW_03/10 | FixedFunctionOpCPU.cpp |
| ACES_DARK_TO_DIM_10 | FixedFunctionOpCPU.cpp |
| ACES_GAMUT_COMP_13 | FixedFunctionOpCPU.cpp |
| REC2100_SURROUND | FixedFunctionOpCPU.cpp |
| RGB_TO_HSV/HSL | Standard formulas |
| XYZ_TO_xyY/uvY/Luv | CIE formulas |

**Tolerance:** 1e-5 (fixed point formulas)

### 6. Full Processor Chains

Test complete OCIO processor pipelines:

```python
# Example: camera to display
processor = config.getProcessor("ARRI LogC3", "sRGB")
vfx_result = vfx.processor(config_path, "ARRI LogC3", "sRGB").apply(image)
ocio_result = processor.applyRGB(image)
assert_close(vfx_result, ocio_result, rtol=1e-4)
```

## Reference Images

### Standard Test Images

| Image | Size | Purpose |
|-------|------|---------|
| `gray_ramp_16x16.exr` | 16x16 | Linear ramp 0-1, basic transform test |
| `gray_ramp_hdr.exr` | 16x16 | Extended range 0-100, HDR tests |
| `color_checker_24.exr` | 64x64 | Macbeth chart, color accuracy |
| `negative_values.exr` | 8x8 | Negative linear values |
| `extreme_values.exr` | 8x8 | Inf, NaN, very large values |

### Golden Output Format

```json
{
  "version": "1.0",
  "tolerance": 1e-4,
  "tests": {
    "srgb_to_linear": {
      "input": "gray_ramp_16x16.exr",
      "transform": "sRGB OETF inverse",
      "hash": "sha256:abc123...",
      "max_value": 1.0,
      "min_value": 0.0
    }
  }
}
```

## Writing New Tests

### Adding a Transfer Function Test

```python
# tests/parity/test_transfer_parity.py

def test_new_camera_log(ocio_config, test_image):
    """Test NewCameraLog vs OCIO."""
    import vfx
    import PyOpenColorIO as ocio
    
    # OCIO reference
    processor = ocio_config.getProcessor(
        ocio.BuiltinTransform("NEW_CAMERA_LOG_to_LINEAR")
    )
    ocio_result = apply_ocio(processor, test_image)
    
    # vfx-rs
    vfx_result = vfx.transfer.new_camera_log_decode(test_image)
    
    # Compare
    assert_allclose(vfx_result, ocio_result, rtol=1e-4)
```

### Adding a Golden Hash Test

```rust
// crates/vfx-tests/src/golden_tests.rs

#[test]
fn test_new_transform_golden() {
    let input = load_test_image("gray_ramp_16x16.exr");
    let output = apply_new_transform(&input);
    let hash = hash_pixels(&output);
    
    let expected = load_golden_hash("new_transform");
    assert_eq!(hash, expected, "Golden hash mismatch");
}
```

## Debugging Test Failures

### Parity test fails

1. Check tolerance - might need adjustment for edge cases
2. Check input range - some transforms have domain restrictions
3. Check OCIO version - we target OCIO 2.3+
4. Generate diff image: `pytest --diff-images`

### Golden hash fails

1. Regenerate hashes: `python generate_golden.py`
2. Check if OCIO reference changed
3. Check platform differences (float precision)

### Common Issues

| Issue | Cause | Fix |
|-------|-------|-----|
| Small diffs at 0 | Log functions undefined at 0 | Use clamped input |
| Large diffs in shadows | Linear toe vs log | Check break point |
| Platform differences | Float rounding | Use looser tolerance |
| NaN in output | Negative input to log | Check input domain |

## CI Integration (Future)

Currently tests are local-only. Future CI setup:

```yaml
# .github/workflows/parity.yml (not active)
jobs:
  parity:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-python@v5
        with:
          python-version: '3.11'
      - run: pip install pytest numpy PyOpenColorIO maturin
      - run: cd crates/vfx-rs-py && maturin develop --release
      - run: cd tests/parity && pytest -v
```

## Test Coverage Goals

| Category | Current | Target |
|----------|---------|--------|
| Transfer Functions | ~60% | 100% |
| Matrix Transforms | ~40% | 100% |
| LUT Operations | ~50% | 100% |
| Grading Ops | ~30% | 100% |
| Fixed Functions | ~70% | 100% |
| Processor Chains | ~20% | 80% |

## References

- OpenColorIO source: `_ref/OpenColorIO/`
- OpenImageIO source: `_ref/OpenImageIO/`
- ACES specifications: https://github.com/ampas/aces-dev
- OCIO BuiltinTransforms: `src/OpenColorIO/transforms/builtins/`
