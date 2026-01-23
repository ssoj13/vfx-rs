# VFX-RS Documentation Fixes - Progress

## Current Status (2026-01-22)

**Total bugs in FINDINGS.md:** 491
**Fixed so far:** ~185 (bugs 1-31, 152-179, 438-442)

## Recently Fixed (this session)

### Color Space Appendix (bugs 161-164)
- [x] 161: Added missing primaries (S-Gamut3.Cine, Canon CGamut, DaVinci Wide Gamut, DJI D-Gamut)
- [x] 162: Fixed RED Wide Gamut values to match implementation
- [x] 163: Removed non-existent ACESproxy from transfer functions table
- [x] 164: Fixed usage example to use `srgb_eotf()`/`srgb_oetf()` instead of fake functions

### Feature Matrix (bugs 165-169)
- [x] 165: Removed fake RED Log3G12
- [x] 166: Removed fake CIE RGB primaries
- [x] 167: Removed fake PSD/TX format support
- [x] 168: Fixed AVIF (write-only) and JP2 (read-only) capabilities
- [x] 169: Already fixed in bug #27 (CubeFile supports 1D+3D)

### Architecture Docs (bugs 170-179)
- [x] 170: Already correct (says 17 crates)
- [x] 171: Fixed vfx-core description (ImageSpec, not Image<C,T,N>)
- [x] 172: Fixed OIIO mapping (ImageSpec → vfx_core::ImageSpec)
- [x] 173-174: Fixed external deps (vfx-exr, not exr)
- [x] 175: Already correct (PixelData enum)
- [x] 176: Fixed error types (IoError, OpsError)
- [x] 177: Fixed default features (added tiff/dpx/hdr)
- [x] 178-179: Fixed dependency hierarchy (vfx-math depends on vfx-core, vfx-io depends on vfx-ocio)

## Next to Fix

### Crate Documentation (bugs 180-200+)
- [ ] 180: vfx-core docs - fix Image<> generic syntax
- [ ] 181: vfx-core docs - fix srgb_to_linear reference
- [ ] 182: vfx-io EXR docs - fix API references
- [ ] ... continue with remaining bugs

## Files Modified This Session

1. `docs/src/appendix/color-spaces.md` - primaries table, usage example
2. `docs/src/appendix/feature-matrix.md` - transfer functions, primaries, formats
3. `docs/src/architecture/README.md` - OIIO mapping, crate descriptions
4. `docs/src/architecture/data-flow.md` - error types
5. `docs/src/architecture/decisions.md` - default features
6. `docs/src/crates/README.md` - dependency hierarchy
7. `FINDINGS.md` - marked bugs as FIXED

## Known Issues

- **VIEWER PERFORMANCE**: ✅ FIXED (2026-01-22)
  - Root cause: `query_pixel()` converted entire image on EVERY mouse move
  - Fix: Added `CachedPixels` struct to cache raw f32 data once in `regenerate_texture()`
  - Now uses O(1) array lookup instead of O(width*height) conversion
