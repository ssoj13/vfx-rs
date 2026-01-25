# DONE - Bug Fixes Progress

## Bug #1: FileTransform supports only small subset of formats
**Status**: FIXED

**Problem**: FileTransform in processor.rs only handles: cube, spi1d, spi3d, clf/ctf
**OCIO supports**: 3DL, CC, CCC, CDL, CLF, CSP, Discreet1DL, HDL, ICC, IridasCube, IridasItx, IridasLook, Pandora, ResolveCube, Spi1D, Spi3D, SpiMtx, Truelight, VF

**Good news**: vfx-lut already has parsers for most formats:
- threedl.rs (3DL)
- cdl.rs (CC, CCC, CDL)
- csp.rs (CSP)
- discreet1dl.rs (Discreet 1DL)
- hdl.rs (HDL)
- iridas_itx.rs (Iridas ITX)
- iridas_look.rs (Iridas Look)
- pandora.rs (Pandora MGA)
- spi_mtx.rs (SpiMtx)
- truelight.rs (Truelight CUB)
- nuke_vf.rs (Nuke VF)

**Missing**: ICC (needs vfx-icc integration)

**Fix**: Add match arms to processor.rs FileTransform for all formats

---

## Bug #2: Config parser does not load several OCIO transform types
**Status**: FIXED

**Problem**: Missing parsers for Lut1DTransform, Lut3DTransform, ExponentWithLinearTransform, DisplayViewTransform

**Fix**: Added parse handlers in config.rs for:
- ExponentWithLinearTransform (gamma, offset, negativeStyle)
- DisplayViewTransform (src, display, view)
- Lut1DTransform (length, halfDomain, rawHalfs, interpolation, values)
- Lut3DTransform (gridSize, interpolation)

Also added helper functions: parse_rgba, yaml_int, yaml_f32_list

---

## Bug #3: ExponentWithLinearTransform negative handling diverges from OCIO
**Status**: FIXED

**Problem**: OCIO documentation says "Negative values are never clamped" for ExponentWithLinearTransform, but vfx-ocio defaulted to NegativeStyle::Clamp.

**Fix**:
- Added NegativeStyle::Linear variant to transform.rs
- Changed ExponentWithLinearTransform default to NegativeStyle::Linear
- Added Linear handling in apply_channel() in transform.rs
- Added Linear handling in processor.rs apply_pixel() for Exponent and ExponentWithLinear ops
- Added Linear handling in gpu.rs for GLSL shader generation

---

## Bug #4: BuiltinTransform coverage is minimal compared to OCIO registry
**Status**: FIXED

**Problem**: OCIO has 66 builtin styles, vfx-ocio had ~15.

**Fix**:
- Added new color matrices: CANON_CGAMUT_TO_AP0, REC2020_TO_AP0, XYZ_D65_TO_REC709, XYZ_D65_TO_REC2020, XYZ_D65_TO_P3_D65
- Added new TransferStyle variants: AppleLog, CanonCLog2, CanonCLog3, Pq, Hlg, Rec1886, Gamma26
- Added builtin entries for: Apple Log, Canon C-Log2/3, Display transforms, PQ/HLG curves
- Added transfer function implementations in processor.rs apply_transfer()
- Added GLSL implementations in gpu.rs generate_transfer_glsl()

---

## Bug #5: FileTransform ccc_id is unused
**Status**: FIXED (as part of Bug #1)

**Fix**: CCC format handler now uses ft.ccc_id to select specific correction by ID

---

