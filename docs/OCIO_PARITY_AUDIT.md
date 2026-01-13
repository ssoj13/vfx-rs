# OCIO Parity Audit Results

## Summary

This document describes the findings from auditing vfx-rs transfer functions against OpenColorIO 2.5.1.

## Key Finding: LUT vs Analytical Implementation

**OCIO uses 1D LUT lookup tables** for most transfer functions when `OCIO_LUT_SUPPORT=1` (default). vfx-rs uses analytical (mathematical) formulas.

This architectural difference means **bit-exact hash matching is impossible** for LUT-based transforms, even though the algorithms are mathematically equivalent.

### OCIO LUT Usage

| Transform | OCIO Implementation | vfx-rs Implementation | Hash Match |
|-----------|--------------------|-----------------------|------------|
| Apple Log | HalfLut (fp16 LUT) | Analytical f64->f32 | No (max diff ~1.6e-5) |
| Canon C-Log2 | 4096-entry LUT | Analytical f64->f32 | No (max diff ~6e-5) |
| Canon C-Log3 | 4096-entry LUT | Analytical f64->f32 | No |
| ACEScct | HalfLut | Analytical f64->f32 | No |
| ACEScc | HalfLut | Analytical f64->f32 | No |
| PQ (ST-2084) | HalfLut | Analytical | No* |
| HLG | HalfLut | Analytical | No* |
| CDL | Analytical (SSE) | Analytical (SSE) | **Yes** |
| Matrices | Analytical | Analytical | **Yes** |

*PQ and HLG also have scaling differences (see below).

## Scaling Differences

### PQ (ST-2084)

- **OCIO**: Outputs nits/100 (max value = 100.0 for 10000 nits)
- **vfx-rs**: Outputs nits (max value = 10000 for 10000 nits)

This is a design choice. OCIO normalizes to 0-100 range, vfx-rs outputs physical nits.

### HLG

- **OCIO**: Uses E_MAX=3.0 scaling factor
- **vfx-rs**: Uses standard HLG formula without E_MAX scaling

## CDL Compatibility

CDL implementation **does match OCIO exactly** after integrating SSE-compatible math:

- Using `fast_pow()` with OCIO's Chebyshev polynomial approximations
- ASC CDL v1.2 order: Slope → Offset → Clamp [0,1] → Power → Saturation → Clamp [0,1]
- Rec.709 luma coefficients for saturation

## Recommendations

### For Production Use

1. **Transfer functions**: Use tolerance-based comparison (max error < 1e-4) instead of hash matching
2. **CDL**: Bit-exact compatibility is achievable and verified
3. **Matrices**: Bit-exact compatibility is achievable and verified

### For Testing

```rust
// Instead of:
assert_eq!(hash, golden_hash);

// Use:
let max_error = compute_max_error(&result, &expected);
assert!(max_error < 1e-4, "Max error {} exceeds tolerance", max_error);
```

### If Exact OCIO Match Required

To achieve bit-exact matching with OCIO for LUT-based transforms:

1. Extract OCIO LUT data and use the same interpolation
2. Or disable LUT support in OCIO (`OCIO_LUT_SUPPORT=0`) for comparison

## Verified Algorithms

The following algorithms are **mathematically correct** (match OCIO formulas):

- [x] Apple Log decode/encode
- [x] Canon C-Log2 decode/encode  
- [x] Canon C-Log3 decode/encode
- [x] ACEScct decode/encode
- [x] ACEScc decode/encode
- [x] PQ (ST-2084) decode/encode (different scaling)
- [x] HLG decode/encode (different E_MAX)
- [x] sRGB EOTF/OETF
- [x] CDL (ASC v1.2)
- [x] 3x3 color matrices

## Test Infrastructure

Golden hashes are generated from OCIO 2.5.1:
- `tests/golden/hashes.json` - SHA256 hashes of quantized outputs
- `tests/parity/generate_golden.py` - Python script to regenerate

For LUT-based transforms, use the debug scripts:
- `debug_apple_log.py` - Compare Apple Log
- `debug_clog2.py` - Compare Canon C-Log2
- `debug_cdl.py` - Compare CDL

## References

- OCIO source: `_ref/OpenColorIO/`
- OCIO CanonCameras.cpp - Canon Log implementations
- OCIO AppleCameras.cpp - Apple Log implementation
- OCIO Displays.cpp - PQ/HLG implementations
- OCIO OpHelpers.cpp - CreateLut/CreateHalfLut functions
