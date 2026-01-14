# VFX-RS Bug Hunt & Code Quality Report (plan3)

**Date:** 2026-01-13
**Status:** Comprehensive audit completed
**Reviewer:** Claude Code
**Scope:** All 17 crates in workspace

---

## Executive Summary

The vfx-rs codebase is **production-quality** with excellent architecture and OCIO parity. This audit identified:

| Category | Count | Severity |
|----------|-------|----------|
| Critical Issues | 3 | Must fix before release |
| High Priority | 8 | Should fix soon |
| Medium Priority | 15 | Fix in next sprint |
| Low Priority | 25+ | Tech debt cleanup |
| Code Duplication | 6 patterns | Needs consolidation |

**Overall Code Health: 8/10** - Well-architected, well-tested, but with some technical debt.

---

## Critical Issues (P0)

### 1. PIZ Huffman Overflow Bug
**Location:** `crates/vfx-exr/src/compression/piz/huffman.rs:213-214`
```rust
// FIXME why does this happen??
code_bit_count -= short_code.len(); // FIXME may throw "attempted to subtract with overflow"
```
**Impact:** Potential panic on certain EXR files with PIZ compression.
**Fix:** Add bounds checking before subtraction.

### 2. Fake Streaming Implementation
**Location:** `crates/vfx-compute/src/streaming.rs:193-215`
```rust
/// Streaming source for EXR files.
/// Reads tiles/scanlines on demand without loading entire file.
pub struct ExrStreamingSource {
    // For now, we load the full image but could optimize later  <-- MISLEADING
    data: Vec<f32>,  // LOADS ENTIRE FILE INTO MEMORY
```
**Impact:** OOM on large files when users expect streaming behavior.
**Fix:** Either implement actual streaming or remove misleading documentation.

### 3. Thread Safety Issue in Cache
**Location:** `crates/vfx-compute/src/cache.rs`
```rust
pub fn get(&mut self, key: &CacheKey) -> Option<&[f32]> {
    entry.last_used = Instant::now();  // Mutation without synchronization
```
**Impact:** Potential data race in multi-threaded pipelines.
**Fix:** Use `RwLock` or `Mutex` for interior mutability.

---

## High Priority Issues (P1)

### 4. ACES Red Modifier NaN Risk
**Location:** `crates/vfx-ops/src/fixed_function.rs:418`
```rust
let discriminant = b * b - 4.0 * a * c;
rgb[0] = (-b - discriminant.sqrt()) / (2.0 * a);  // NaN if discriminant < 0
```
**Fix:** `discriminant.max(0.0).sqrt()`

### 5. fast_exp2 Floor Bug for Negatives
**Location:** `crates/vfx-color/src/sse_math.rs:80-84`
```rust
let floor_x = if x >= 0.0 {
    x as i32
} else {
    x as i32 - 1  // WRONG: floor(-1.0) should be -1, not -2
};
```
**Fix:** Use `x.floor() as i32` or `(x - 1.0).ceil() as i32`

### 6. 2-Channel Image Misinterpretation
**Location:** `crates/vfx-io/src/source.rs:131-144`
```rust
let g = data.get(idx + 1.min(channels - 1)).copied()...
let b = data.get(idx + 2.min(channels - 1)).copied()...
```
**Issue:** 2-channel (Y+A) images interpreted as R=Y, G=A, B=A instead of R=G=B=Y.
**Fix:** Add explicit handling for 2-channel images.

### 7. Deep Tile Empty Assertion Bug
**Location:** `crates/vfx-exr/src/block/chunk.rs:323-326`
```rust
debug_assert_ne!(
    self.compressed_sample_data_le.len(), 0,
    "empty blocks should not be put in the file bug"
);
```
**Issue:** Assertion is WRONG - empty sample data IS valid for deep tiles with 0 samples.
**Fix:** Remove incorrect assertion.

### 8. Unused CLI Argument
**Location:** `crates/vfx-cli/src/main.rs:287-288`
```rust
/// Quality (0-100, for JPEG)
#[arg(short = 'q', long)]
quality: Option<u8>,  // NEVER USED IN convert.rs
```
**Fix:** Wire up to JPEG encoder or remove.

### 9. V-Log Transform Returns Identity
**Location:** `crates/vfx-ocio/src/builtin_transforms.rs:295-301`
```rust
"vlogtoaces20651" | "vlogtoaces" => {
    // V-Gamut matrix needed
    Some(BuiltinDef::Identity)  // NO-OP!
}
```
**Fix:** Implement actual V-Log to ACES transform.

### 10. Trilinear Mip Blend Always 0.5
**Location:** `crates/vfx-io/src/texture.rs:229-251`
```rust
fn sample_trilinear(..., mip_f: u32, ...) {  // Should be f32!
    let blend = 0.5; // Simplified
```
**Fix:** Accept `mip_f: f32` and use fractional part for blend.

### 11. Division by Zero in Grading
**Location:** `crates/vfx-ops/src/grading_primary.rs:256`
```rust
let inv_contrast = [1.0 / contrast[0], ...];  // No zero check
```
**Fix:** Add MIN_CONTRAST check before division.

---

## Medium Priority Issues (P2)

### 12. CDL Struct Duplication
**Locations:**
- `vfx-color/src/cdl.rs:76` - Full implementation (canonical)
- `vfx-compute/src/color.rs:44` - Partial duplicate
- `vfx-lut/src/clf.rs:80` - Another duplicate
- `vfx-ocio/src/transform.rs:678` - Yet another
- `vfx-compute/src/shaders/mod.rs:38` - GPU version
- `vfx-compute/src/backend/wgpu_backend.rs:28` - Uniform version

**Fix:** Use `vfx_color::Cdl` everywhere with `From` trait conversions.

### 13. Rec.709 Luma Constants Scattered
**Found in 15+ files** with hardcoded `0.2126, 0.7152, 0.0722`.
**Fix:** Add to vfx-core:
```rust
pub const REC709_LUMA: [f32; 3] = [0.2126, 0.7152, 0.0722];
pub fn luminance(rgb: [f32; 3]) -> f32 {
    rgb[0] * REC709_LUMA[0] + rgb[1] * REC709_LUMA[1] + rgb[2] * REC709_LUMA[2]
}
```

### 14. sRGB to XYZ Matrix Duplication
**Canonical:** `vfx-primaries/lib.rs:481` - `SRGB_TO_XYZ`
**Duplicates:** 5+ other locations defining same matrix.
**Fix:** Import from vfx_primaries everywhere.

### 15. Memory Model Mismatch
**vfx-core:** `Image { data: Arc<Vec<T>> }` - zero-copy
**vfx-compute:** `ComputeImage { data: Vec<f32> }` - forces copy
**Fix:** Change ComputeImage to use `Arc<[f32]>`.

### 16. Dead wgpu Crop Pipeline
**Location:** `crates/vfx-compute/src/backend/wgpu_backend.rs:129-130`
```rust
#[allow(dead_code)]
crop: wgpu::ComputePipeline,  // Created but never used
```
**Fix:** Implement crop operation or remove pipeline.

### 17. SIMD Module Not Integrated
**Location:** `crates/vfx-ocio/src/simd.rs`
Entire module marked `#![allow(dead_code)]` - SIMD functions implemented but not called.
**Fix:** Integrate with processor for performance or remove.

### 18. Unused Config Fields
**Location:** `crates/vfx-ocio/src/config.rs:73-81`
```rust
#[allow(dead_code)]
inactive_colorspaces: Vec<String>,
#[allow(dead_code)]
strict_parsing: bool,
```
**Fix:** Wire up or remove.

### 19. RrtParams Unused Fields
**Location:** `crates/vfx-color/src/aces.rs`
```rust
pub struct RrtParams {
    f: f32,     // UNUSED in tonemap formula
    white: f32, // UNUSED
```
**Fix:** Use them or remove.

### 20. TransferStyle Enum Duplication
**Locations:**
- `vfx-ocio/src/builtin_transforms.rs:38-50`
- `vfx-ocio/src/processor.rs` (different enum)
**Fix:** Unify to single enum.

### 21. UDIM Pattern Regex Unused
**Location:** `crates/vfx-io/src/udim.rs:176`
```rust
let _pattern_regex = UDIM_MARKERS.iter()...  // Created but never used
```
**Fix:** Remove or use.

### 22. Deprecated metadata Module
**Location:** `crates/vfx-io/src/lib.rs:1176-1182`
```rust
#[deprecated(since = "0.2.0", note = "Use vfx_io::attrs::AttrValue instead")]
pub mod metadata { ... }
```
**Fix:** Remove in next major version.

### 23. contiguous() Always True
**Location:** `crates/vfx-io/src/imagebuf/mod.rs:693`
```rust
pub fn contiguous(&self) -> bool {
    // TODO: Check actual storage layout
    true  // ALWAYS returns true
}
```
**Fix:** Implement properly or document limitation.

### 24. Magic Bytes Buffer Too Small
**Location:** `crates/vfx-io/src/detect.rs:85-95`
```rust
let mut header = [0u8; 8];  // Only 8 bytes
```
**Issue:** HEIF/JP2 need 12 bytes for reliable detection.
**Fix:** Increase to 12 bytes.

### 25. LUT Invert Assumes Monotonic
**Location:** `crates/vfx-ocio/src/processor.rs:149-191`
```rust
// by binary search (assumes monotonic LUT)
```
**Issue:** Non-monotonic LUTs (S-curves) produce wrong inversions.
**Fix:** Add validation or document limitation.

### 26. logc3_params() Called But Discarded
**Location:** `crates/vfx-ocio/src/builtin_transforms.rs:256`
```rust
let _ = logc3_params(); // ACEScct uses Transfer style directly
```
**Fix:** Remove useless call.

---

## vfx-exr Technical Debt (~200 TODOs)

The vfx-exr crate has accumulated significant technical debt. Key categories:

| Category | Count | Examples |
|----------|-------|----------|
| Integer casting | ~15 | `as u32` without `try_from` |
| Optimization | ~25 | Cache level calculations |
| Cleanup | ~20 | Redundant clones |
| Documentation | ~15 | Missing safety docs |
| Tests | ~10 | Missing edge cases |
| Compression | ~50 | PIZ/B44 improvements |
| Deep data | ~20 | Validation, error handling |

**Recommendation:** Create dedicated sprint to address systematically.

---

## Code Duplication Summary

| Pattern | Locations | Fix |
|---------|-----------|-----|
| CDL struct | 6 crates | Use vfx_color::Cdl |
| Rec.709 luma | 15+ files | Add to vfx-core |
| sRGB→XYZ matrix | 6 locations | Use vfx_primaries |
| find_cusp functions | vfx-color/aces2 | Consolidate |
| saturation calc | cli, ops, compute | Extract to shared fn |
| color parsing | cli commands | Unify in mod.rs |

---

## Unfinished Features

### Confirmed Incomplete:
1. **maketx mipmaps** - Generated but not saved
2. **Batch mode operations** - Only 5 of 27 commands supported
3. **GPU shader backends** - HLSL/Metal return placeholders
4. **Baker shaper LUT** - Struct fields unused

### Potentially Abandoned:
1. **Channel "Z" as index 4** - Assumes fixed channel layout
2. **Deep tile empty handling** - Assertion contradicts valid case
3. **ExrReaderOptions** - Empty struct placeholder

---

## Test Coverage Gaps

| Crate | Unit | Integration | Parity | Notes |
|-------|------|-------------|--------|-------|
| vfx-cli | ~ | - | - | Limited, needs expansion |
| vfx-view | - | - | - | Manual testing only |
| vfx-compute | ✓ | ✓ | - | No OCIO parity tests |

### Missing Test Cases:
- Deep tiles with zero samples
- Subsampled channel handling
- Large sample counts (> 2^31)
- Non-monotonic LUT inversion
- 2-channel image handling
- Extreme exposure values

---

## Recommendations

### Immediate Actions (This Week):
1. [ ] Fix PIZ huffman overflow check
2. [ ] Fix deep tile empty assertion
3. [ ] Add discriminant check in ACES Red Modifier
4. [ ] Wire up or remove unused `quality` argument

### Short Term (Next Sprint):
5. [ ] Fix streaming implementation or documentation
6. [ ] Add thread safety to cache
7. [ ] Fix fast_exp2 floor calculation
8. [ ] Consolidate CDL implementations
9. [ ] Add REC709_LUMA constant to vfx-core
10. [ ] Fix 2-channel image handling

### Medium Term (Next Month):
11. [ ] Align ComputeImage with vfx-core memory model
12. [ ] Integrate SIMD module in vfx-ocio
13. [ ] Complete GPU shader backends
14. [ ] Expand CLI test coverage
15. [ ] Address vfx-exr TODOs systematically

### Long Term (Roadmap):
16. [ ] Implement actual streaming for large images
17. [ ] Add non-monotonic LUT handling
18. [ ] Complete batch mode operations
19. [ ] Remove deprecated modules

---

## Files Reference

### Critical Issues:
| Issue | File | Line |
|-------|------|------|
| PIZ overflow | vfx-exr/src/compression/piz/huffman.rs | 213-214 |
| Fake streaming | vfx-compute/src/streaming.rs | 193-215 |
| Cache race | vfx-compute/src/cache.rs | Various |

### High Priority:
| Issue | File | Line |
|-------|------|------|
| ACES NaN | vfx-ops/src/fixed_function.rs | 418 |
| fast_exp2 | vfx-color/src/sse_math.rs | 80-84 |
| 2-channel | vfx-io/src/source.rs | 131-144 |
| Deep tile | vfx-exr/src/block/chunk.rs | 323-326 |
| Quality arg | vfx-cli/src/main.rs | 287-288 |
| V-Log | vfx-ocio/src/builtin_transforms.rs | 295-301 |
| Trilinear | vfx-io/src/texture.rs | 229-251 |
| Division | vfx-ops/src/grading_primary.rs | 256 |

### Duplication:
| Pattern | Primary Location | To Consolidate |
|---------|-----------------|----------------|
| CDL | vfx-color/src/cdl.rs:76 | 5 other locations |
| Luma | (create new) | 15+ files |
| sRGB→XYZ | vfx-primaries/src/lib.rs:481 | 5 other locations |

---

## Approval Checklist

Before implementing fixes:

- [ ] Review this report for accuracy
- [ ] Prioritize based on current release goals
- [ ] Assign ownership for each fix
- [ ] Create tracking issues if using issue tracker
- [ ] Schedule vfx-exr tech debt sprint

---

## Appendix: Full TODO/FIXME Scan Results

See companion file: `C:\Users\joss1\.claude\projects\...\tool-results\toolu_013mDULvPyArR9bJZFVMeqoJ.txt`

Total markers found: ~200 in vfx-exr, ~15 in other crates

---

*Report generated by Claude Code Bug Hunt workflow*
