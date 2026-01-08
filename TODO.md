# VFX-RS TODO / Audit Results

## Completed Phases

### Phase 1: Transfer Functions - COMPLETE
- [x] Canon Log 2/3 - constants match OCIO CanonCameras.cpp
- [x] ARRI LogC4 - constants match OCIO ArriCameras.cpp (base 2, linSideSlope=2231.82...)
- [x] Apple Log - constants match OCIO AppleCameras.cpp (R_0, R_t, c, beta, gamma, delta)
- [x] S-Log3 - matches OCIO SonyCameras.cpp
- [x] V-Log - matches OCIO PanasonicCameras.cpp
- [x] RED Log3G10 / REDLogFilm
- [x] BMD Film Gen5
- [x] ACEScc / ACEScct
- [x] sRGB, Rec.709, PQ, HLG

### Phase 2: LUT Formats - COMPLETE
- [x] CDL (ASC-CDL with SOP, saturation)
- [x] CSP format
- [x] 1D/3D LUT with tetrahedral interpolation

### Phase 3: ACES2 & Grading - COMPLETE
- [x] ACES2 Output Transform (32 tests pass)
- [x] ExposureContrast (Linear, Video, Logarithmic styles)
- [x] GradingPrimary (lift/gamma/gain, brightness/contrast)

### Phase 4: OCIO Parity - COMPLETE
Camera primaries added:
- [x] Canon CGamut: (0.74, 0.27), (0.17, 1.14), (0.08, -0.10), D65
- [x] ARRI Wide Gamut 4: (0.7347, 0.2653), (0.1424, 0.8576), (0.0991, -0.0308), D65
- [x] RED Wide Gamut: (0.780308, 0.304253), (0.121595, 1.493994), (0.095612, -0.084589), D65
- [x] S-Gamut3.Cine: (0.766, 0.275), (0.225, 0.800), (0.089, -0.087), D65

Mac build support:
- [x] .cargo/config.toml with -undefined dynamic_lookup for pyo3

### Phase 5: SIMD Optimization - COMPLETE
- [x] vfx-math::simd - f32x4/f32x8 via `wide` crate
- [x] vfx-compute - rayon parallel processing
- [x] Auto-vectorizable code patterns
- [x] LTO enabled in release profile

### Phase 6: Chromatic Adaptation - COMPLETE
vfx-math::adapt matches OCIO:
- [x] BRADFORD matrix: 0.8951, 0.2664, -0.1614 / -0.7502, 1.7135, 0.0367 / 0.0389, -0.0685, 1.0296
- [x] CAT02 matrix: 0.7328, 0.4296, -0.1624 / -0.7036, 1.6975, 0.0061 / 0.0030, 0.0136, 0.9834
- [x] VON_KRIES, XYZ_SCALING
- [x] Standard illuminants: D65, D50, D55, D60, A, E, ACES_WHITE, DCI_WHITE
- [x] Pre-computed D65<->D50, D65<->D60 Bradford matrices

## Test Results
- 1200+ tests passing
- All transfer function roundtrips < 1e-9 precision
- All OCIO constants verified against reference

## Architecture Summary

```
vfx-core       - Core types (Pixel, ColorSpaceId)
vfx-math       - Mat3, Vec3, SIMD, chromatic adaptation
vfx-primaries  - RGB primaries, rgb_to_xyz matrices
vfx-transfer   - All transfer functions (encode/decode)
vfx-lut        - 1D/3D LUT, CDL
vfx-color      - Color conversions
vfx-ops        - Image operations (CDL, grading, composite, FFT)
vfx-io         - Image I/O (EXR, PNG, JPEG, TIFF, DPX, HDR, ...)
vfx-compute    - GPU/CPU compute (rayon, wgpu ready)
vfx-ocio       - OCIO config parser, transforms, processor
vfx-rs-py      - Python bindings (pyo3)
```

## Future Work
- [ ] GPU compute shaders (wgpu backend)
- [ ] CUDA backend via cudarc
- [ ] Additional camera gamuts as needed
- [ ] ICC profile support via lcms2
