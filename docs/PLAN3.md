# VFX-RS Bug Hunt Report - January 2026

## Executive Summary

Comprehensive analysis of vfx-rs codebase reveals significant code duplication that should be addressed for maintainability. The codebase is production-quality but has accumulated redundant type definitions during development.

## 1. TODO/FIXME Items (8 total)

| Location | Description | Priority |
|----------|-------------|----------|
| `vfx-rs-py/src/io.rs:54` | EXR compression options not implemented | P2 |
| `vfx-rs-py/src/io.rs:77` | PNG compression level not implemented | P2 |
| `vfx-io/Cargo.toml:69` | dav1d pkg-config not working with vcpkg | P3 |
| `vfx-io/src/heif.rs:349` | Extract HDR metadata from MDCV/CLLI boxes | P2 |
| `vfx-io/src/streaming/exr.rs:109` | True tile-only reading for memory efficiency | P1 |
| `vfx-compute/src/layer.rs:305` | Apply spatial ops to color result | P2 |
| `vfx-compute/src/pipeline.rs:560` | True tiled processing with TileWorkflow | P1 |
| `vfx-compute/src/pipeline.rs:928` | Header-only probing for efficiency | P2 |

## 2. Critical Code Duplication

### 2.1 BitDepth Enum (6 definitions!)

| File | Line | Variants |
|------|------|----------|
| `vfx-ocio/colorspace.rs` | 133 | Uint8, Uint10, Uint12, Uint16 |
| `vfx-ocio/processor.rs` | 581 | Unknown, Uint8, Uint10, Uint12, Uint16, Uint32, Float16, Float32 |
| `vfx-lut/clf.rs` | 72 | Uint8, Uint10, Uint12, Uint16, Float16, Float32 |
| `vfx-io/dpx.rs` | 82 | Bit8, Bit10, Bit12, Bit16 |
| `vfx-io/png.rs` | 68 | Eight, Sixteen |
| `vfx-io/tiff.rs` | 70 | Eight, Sixteen, ThirtyTwoFloat |

**Recommendation**: Unify into single `vfx-core::BitDepth` with all variants:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BitDepth {
    #[default]
    Unknown,
    U8,
    U10,
    U12,
    U16,
    U32,
    F16,
    F32,
}
```

### 2.2 AttrValue Enum (3 definitions!)

| File | Line | Purpose |
|------|------|---------|
| `vfx-io/metadata.rs` | 9 | Simple: Bool, Str, Int, UInt, Float, Double, Bytes, List, Map |
| `vfx-io/attrs/value.rs` | 58 | Full EXIF: + Rational, URational, Group |
| `vfx-core/spec.rs` | 169 | Minimal: Int, Float, String, IntArray, FloatArray, Matrix3, Matrix4 |

**Recommendation**: Keep `vfx-io/attrs/value.rs` as the canonical version, deprecate others.

### 2.3 PixelFormat vs ChannelFormat

| File | Enum | Variants |
|------|------|----------|
| `vfx-io/lib.rs:482` | PixelFormat | U8, U16, F16, F32, U32 |
| `vfx-core/spec.rs:89` | ChannelFormat | U8, U16, F16, F32 |

**Recommendation**: Merge into `vfx-core::PixelFormat`, add U32.

## 3. Dead Code (20 `#[allow(dead_code)]` markers)

### vfx-ocio (4 instances)
- `config.rs:67` - inactive_colorspaces field
- `config.rs:74` - strict_parsing field
- `processor.rs:245` - constant G in Rec709 luma
- `processor.rs:575` - has_dynamic field

### vfx-io (11 instances)
- `dpx.rs:191,219,251,291,559` - DPX header structs and options
- `exr.rs:207` - ExrReader options field
- `hdr.rs:160` - HdrReader options field
- `jpeg.rs:176` - JpegReader options field
- `png.rs:211` - PngReader options field
- `tiff.rs:212` - TiffReader options field
- `traits.rs:263,274` - ImageReader/ImageWriter traits

### vfx-compute (3 instances)
- `processor.rs:418` - log_strategy method
- `cuda_backend.rs:749,752` - CUDA context/module fields

**Analysis**: Most dead code is for planned features. Reader options fields are ready for per-format configuration. Consider implementing or removing.

## 4. API Compatibility Issues

### 4.1 Python Bindings (vfx-rs-py)
Fixed during implementation:
- `Processor::wgpu()` -> `Processor::new(Backend::Wgpu)`
- `apply_contrast(img, factor, pivot)` -> `apply_contrast(img, factor)` (no pivot)
- `backend()` field -> `backend_name()` method
- `Lut3D.size()` method -> `Lut3D.size` field
- `ProcessList.processes` -> `ProcessList.nodes`
- `PixelFormat::U32` variant was missing in match

### 4.2 FormatWriter Pattern
All format writers use consistent pattern:
```rust
Writer::with_options(opts).write(&path, &image)?
```

## 5. Architecture Recommendations

### 5.1 Immediate (P0)
- [x] Python bindings (vfx-rs-py) - DONE
- [ ] Unify BitDepth in vfx-core
- [ ] Unify AttrValue - use vfx-io/attrs/value.rs

### 5.2 Short-term (P1)
- [ ] True tiled EXR reading (`vfx-io/src/streaming/exr.rs:109`)
- [ ] True tiled processing (`vfx-compute/src/pipeline.rs:560`)
- [ ] Merge ChannelFormat + PixelFormat

### 5.3 Medium-term (P2)
- [ ] HDR metadata extraction for HEIF
- [ ] EXR/PNG compression options in Python API
- [ ] Header-only format probing
- [ ] Apply spatial ops in layer processing

### 5.4 Low Priority (P3)
- [ ] dav1d AVIF support via vcpkg
- [ ] Remove dead code or implement planned features

## 6. Crate Dependency Flow

```
vfx-core (types, specs)
    |
    +-- vfx-math (matrix, vector)
    +-- vfx-transfer (EOTF/OETF)
    +-- vfx-primaries (RGB primaries)
    +-- vfx-lut (1D/3D LUTs)
    |
    v
vfx-color (transforms)
    |
    +-- vfx-io (file I/O)
    +-- vfx-ops (image ops)
    |
    v
vfx-compute (GPU/CPU)
    |
    +-- vfx-ocio (OCIO compat)
    +-- vfx-icc (ICC profiles)
    |
    v
vfx-rs-py (Python API)
vfx-cli (CLI tools)
```

## 7. Proposed Unification Plan

### Step 1: Create `vfx-core::format`
```rust
// vfx-core/src/format.rs
pub enum BitDepth { U8, U10, U12, U16, U32, F16, F32 }
pub enum PixelFormat { U8, U16, U32, F16, F32 }

impl From<BitDepth> for PixelFormat { ... }
impl TryFrom<PixelFormat> for BitDepth { ... }
```

### Step 2: Update crates
1. `vfx-io` - use `vfx_core::format::*`
2. `vfx-ocio` - use `vfx_core::format::BitDepth`
3. `vfx-lut` - use `vfx_core::format::BitDepth`

### Step 3: Unify AttrValue
1. Move `vfx-io/attrs` to `vfx-core/attrs`
2. Re-export from vfx-io for backwards compat
3. Remove `vfx-io/metadata.rs` AttrValue
4. Remove `vfx-core/spec.rs` AttrValue

## 8. Test Coverage

Current test files:
- `vfx-rs-py/src/test.py` - visual quality tests (10 tests)
- Individual crate tests via `cargo test`

Recommended additions:
- [ ] Integration tests for format roundtrips
- [ ] Benchmark suite for compute backends
- [ ] Comparison tests vs OIIO/OCIO output

## 9. Conclusion

The vfx-rs codebase is well-architected but has accumulated 6 copies of BitDepth and 3 copies of AttrValue during development. Unifying these into vfx-core will:
- Reduce maintenance burden
- Ensure consistent behavior
- Simplify cross-crate type conversions

Priority order:
1. Fix TODOs for tiled processing (P1)
2. Unify BitDepth enum
3. Unify AttrValue enum
4. Merge PixelFormat/ChannelFormat
5. Clean up dead code
