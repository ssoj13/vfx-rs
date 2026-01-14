# VFX-RS Master Fix Plan

**Date:** 2026-01-13
**Status:** IN PROGRESS
**Goal:** Fix all identified bugs, eliminate duplication, improve code quality

---

## Phase 1: Critical Issues (P0) - MUST FIX

### 1.1 PIZ Huffman Overflow Bug
- [ ] **File:** `crates/vfx-exr/src/compression/piz/huffman.rs:213-214`
- **Problem:** `code_bit_count -= short_code.len()` can overflow
- **Fix:** Add saturating_sub or bounds check

### 1.2 Fake Streaming Implementation  
- [ ] **File:** `crates/vfx-compute/src/streaming.rs:193-215`
- **Problem:** Claims streaming but loads entire file
- **Fix:** Update documentation to reflect actual behavior (or implement real streaming)

### 1.3 Cache Thread Safety
- [ ] **File:** `crates/vfx-compute/src/cache.rs`
- **Problem:** `last_used` mutation without synchronization
- **Fix:** Use `RwLock` or `Mutex` for thread-safe access

---

## Phase 2: High Priority Issues (P1)

### 2.1 ACES Red Modifier NaN Risk
- [ ] **File:** `crates/vfx-ops/src/fixed_function.rs:418`
- **Problem:** `discriminant.sqrt()` without checking >= 0
- **Fix:** Add `.max(0.0)` before sqrt

### 2.2 fast_exp2 Floor Bug
- [ ] **File:** `crates/vfx-color/src/sse_math.rs:80-84`
- **Problem:** Wrong floor calculation for negatives
- **Fix:** Use `x.floor() as i32`

### 2.3 2-Channel Image Handling
- [ ] **File:** `crates/vfx-io/src/source.rs:131-144`
- **Problem:** Y+A images misinterpreted as R=Y, G=A, B=A
- **Fix:** Add explicit 2-channel handling (R=G=B=Y, A=A)

### 2.4 Deep Tile Empty Assertion
- [ ] **File:** `crates/vfx-exr/src/block/chunk.rs:323-326`
- **Problem:** Incorrect assertion - empty sample data IS valid
- **Fix:** Remove or fix debug_assert

### 2.5 Unused CLI Quality Argument
- [ ] **File:** `crates/vfx-cli/src/main.rs:287-288` and `commands/convert.rs`
- **Problem:** `quality` argument declared but never used
- **Fix:** Wire to JPEG encoder or remove

### 2.6 V-Log Returns Identity
- [ ] **File:** `crates/vfx-ocio/src/builtin_transforms.rs:295-301`
- **Problem:** V-Log transform returns Identity (no-op)
- **Fix:** Implement actual V-Log to ACES transform

### 2.7 Trilinear Mip Blend Always 0.5
- [ ] **File:** `crates/vfx-io/src/texture.rs:229-251`
- **Problem:** `mip_f: u32` should be `f32`, blend hardcoded to 0.5
- **Fix:** Accept f32 and use fractional part

### 2.8 Division by Zero in Grading
- [ ] **File:** `crates/vfx-ops/src/grading_primary.rs:256`
- **Problem:** No zero check before `1.0 / contrast[i]`
- **Fix:** Add MIN_CONTRAST constant and check

---

## Phase 3: Code Consolidation (Deduplication)

### 3.1 Add REC709_LUMA to vfx-core
- [ ] **File:** `crates/vfx-core/src/lib.rs` or new `constants.rs`
- **Add:**
  ```rust
  pub const REC709_LUMA: [f32; 3] = [0.2126, 0.7152, 0.0722];
  pub fn luminance(rgb: [f32; 3]) -> f32 {
      rgb[0] * REC709_LUMA[0] + rgb[1] * REC709_LUMA[1] + rgb[2] * REC709_LUMA[2]
  }
  ```

### 3.2 Replace Hardcoded Luma Constants
Replace in all files:
- [ ] `crates/vfx-color/src/cdl.rs` (lines 203-204, 247-248)
- [ ] `crates/vfx-view/src/handler.rs` (lines 31-32)
- [ ] `crates/vfx-cli/src/commands/color.rs` (line 86)
- [ ] `crates/vfx-cli/src/commands/grade.rs` (line 92)
- [ ] `crates/vfx-compute/src/shaders/mod.rs` (line 68)
- [ ] `crates/vfx-compute/src/backend/cpu_backend.rs` (line 122)
- [ ] `crates/vfx-compute/src/backend/cuda_backend.rs` (line 86)
- [ ] `crates/vfx-ops/src/grading_primary.rs` (lines 19-20)
- [ ] `crates/vfx-ops/src/fixed_function.rs` (lines 1468-1469)
- [ ] `crates/vfx-io/src/imagebufalgo/color.rs` (multiple)
- [ ] `crates/vfx-ocio/src/processor.rs` (multiple)
- [ ] `crates/vfx-ocio/src/dynamic.rs` (line 202)
- [ ] `crates/vfx-lut/src/cdl.rs` (line 95)
- [ ] `crates/vfx-lut/src/clf.rs` (line 138)

### 3.3 Consolidate sRGB to XYZ Matrix
- [ ] Use `vfx_primaries::SRGB_TO_XYZ` everywhere
- [ ] Remove duplicate in `vfx-ocio/src/builtin_transforms.rs:89`
- [ ] Remove duplicate helpers in `vfx-color/src/aces2/*.rs`

### 3.4 Consolidate CDL Structs
- [ ] Keep canonical: `vfx-color/src/cdl.rs:76`
- [ ] Add `From` traits for conversion
- [ ] Update `vfx-compute/src/color.rs:44` to use vfx_color::Cdl
- [ ] Update `vfx-lut/src/clf.rs:80` to use vfx_color::Cdl
- [ ] Update `vfx-ocio/src/transform.rs:678` to reuse

---

## Phase 4: Medium Priority Fixes (P2)

### 4.1 Memory Model Alignment
- [ ] **File:** `crates/vfx-compute/src/image.rs`
- **Problem:** `ComputeImage` uses `Vec<f32>` instead of `Arc`
- **Fix:** Change to `Arc<[f32]>` for zero-copy sharing

### 4.2 Remove Dead wgpu Crop Pipeline
- [ ] **File:** `crates/vfx-compute/src/backend/wgpu_backend.rs:129-130`
- **Fix:** Remove unused `crop` pipeline or implement

### 4.3 Integrate or Remove SIMD Module
- [ ] **File:** `crates/vfx-ocio/src/simd.rs`
- **Fix:** Either integrate with processor or remove dead code

### 4.4 Remove Unused Config Fields
- [ ] **File:** `crates/vfx-ocio/src/config.rs:73-81`
- **Fix:** Wire up `inactive_colorspaces` and `strict_parsing` or remove

### 4.5 Fix RrtParams Unused Fields
- [ ] **File:** `crates/vfx-color/src/aces.rs`
- **Fix:** Use `f` and `white` fields or remove

### 4.6 Unify TransferStyle Enums
- [ ] **Files:** `vfx-ocio/src/builtin_transforms.rs:38-50` and `processor.rs`
- **Fix:** Single enum definition

### 4.7 Remove UDIM Pattern Regex
- [ ] **File:** `crates/vfx-io/src/udim.rs:176`
- **Fix:** Remove unused `_pattern_regex`

### 4.8 Remove Deprecated metadata Module
- [ ] **File:** `crates/vfx-io/src/lib.rs:1176-1182`
- **Fix:** Remove deprecated module

### 4.9 Fix contiguous() Method
- [ ] **File:** `crates/vfx-io/src/imagebuf/mod.rs:693`
- **Fix:** Implement properly or document limitation

### 4.10 Increase Magic Bytes Buffer
- [ ] **File:** `crates/vfx-io/src/detect.rs:85-95`
- **Fix:** Increase from 8 to 12 bytes for HEIF/JP2

### 4.11 LUT Invert Validation
- [ ] **File:** `crates/vfx-ocio/src/processor.rs:149-191`
- **Fix:** Add warning or validation for non-monotonic LUTs

### 4.12 Remove logc3_params() Call
- [ ] **File:** `crates/vfx-ocio/src/builtin_transforms.rs:256`
- **Fix:** Remove useless `let _ = logc3_params();`

---

## Phase 5: Cleanup Tasks

### 5.1 Remove Unused Imports
- [ ] Run `cargo fix --allow-dirty` to remove unused imports

### 5.2 Fix Clippy Warnings
- [ ] Run `cargo clippy --workspace` and fix warnings

### 5.3 Add Safety Documentation
- [ ] **File:** `crates/vfx-color/src/sse_math.rs`
- **Fix:** Add `# Safety` docs for unsafe functions

### 5.4 Remove grep Metadata
- [ ] **File:** `crates/vfx-cli/src/commands/grep.rs:18`
- **Fix:** Remove unused `_metadata`

### 5.5 Fix info JSON Args
- [ ] **File:** `crates/vfx-cli/src/commands/info.rs:190`
- **Fix:** Use args in JSON output or remove

---

## Verification

After all fixes:
- [ ] `cargo build --workspace` - no errors
- [ ] `cargo test --workspace` - all tests pass
- [ ] `cargo clippy --workspace` - no warnings
- [ ] Manual testing of key workflows

---

## Progress Tracking

| Phase | Items | Done | Status |
|-------|-------|------|--------|
| P0 Critical | 3 | 0 | Not Started |
| P1 High | 8 | 0 | Not Started |
| P3 Dedup | 4 | 0 | Not Started |
| P2 Medium | 12 | 0 | Not Started |
| P5 Cleanup | 5 | 0 | Not Started |
| **Total** | **32** | **0** | **0%** |
