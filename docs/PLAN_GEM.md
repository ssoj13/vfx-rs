# VFX-RS Bug Hunt & Refactoring Plan

This plan aims to unify the architecture of `vfx-rs`, achieve parity with OpenImageIO/OpenColorIO, and eliminate architectural redundancies.

## 1. Unified Core Types (Source of Truth)
- [ ] **Attrs & AttrValue Unification**: 
    - Move `vfx_io::attrs` to `vfx_core::attrs`.
    - Replace `vfx_core::spec::AttrValue` and `vfx_io::metadata::AttrValue` with the unified version.
    - Add support for all EXIF/VFX metadata types in the unified version.
- [ ] **ImageSpec Consolidation**:
    - Update `vfx_core::ImageSpec` to include the unified `Attrs`.
    - Add missing OIIO-like fields: `tile_width`, `tile_height`, `subimage`, `mipmap_level`.
    - Replace `vfx_io::Metadata` with `vfx_core::ImageSpec`.
- [ ] **Format Enum Unification**:
    - Unify `ChannelFormat` (core), `PixelFormat` (io), and `BitDepth` (ocio) into a single `PixelType` enum in `vfx-core`.

## 2. Integrated Processing Architecture
- [ ] **OCIO + Compute Integration**:
    - Update `vfx_ocio::Processor` to support a `vfx_compute` backend.
    - Implement shader generation in `vfx-ocio` that can be executed by `vfx-compute`.
- [ ] **SIMD & Parallelism**:
    - Implement `rayon`-based parallel processing in `vfx_ocio::Processor::apply`.
    - Use `wide` crate for SIMD optimizations in color math (CDL, Matrix, Transfer).
- [ ] **CDL Fix**:
    - Fix hardcoded Rec.709 coefficients in OCIO CDL Op. Use proper luminance coefficients from the active color space.

## 3. Image Container Refactoring
- [ ] **ImageData vs Image**:
    - Make `vfx_io::ImageData` a dynamic version of `vfx_core::Image`.
    - Ensure zero-copy conversion between them where possible.
    - Use `vfx_core::ImageSpec` as the common header.
- [ ] **Deep Data Support**:
    - Add `DeepImage` type to `vfx-core`.
    - Implement Deep data reading/writing in `vfx-io` (EXR).

## 4. OIIO Parity Improvements
- [ ] **ImageCache & TextureSystem**:
    - Refactor `vfx_io::cache` to support multi-channel tiles efficiently.
    - Improve filtering in `TextureSystem` (Anisotropic, proper Trilinear).
- [ ] **Streaming I/O**:
    - Implement true tiled reading for EXR and TIFF in `vfx-io`.

## 5. Cleanup & Standards
- [ ] **Remove Legacy Code**:
    - Delete `// === Legacy traits for backwards compatibility ===` sections.
    - Remove `ProcessingBackend` trait in favor of `TiledExecutor`.
- [ ] **Deduplicate Logic**:
    - Move shared color math between `vfx-ocio`, `vfx-color`, and `vfx-transfer` into a common location.

## 6. Verification
- [ ] Run full test suite: `cargo test`
- [ ] Benchmarking: `cargo bench`
- [ ] Parity validation against OIIO/OCIO CLI tools.
