# VFX-RS Bug Hunt Report & Plan (plan1)

**Date:** 2026-01-09

## Scope & Method

- Scanned workspace-level TODO/FIXME markers and cross-checked critical pipelines: OCIO, ImageBuf, EXR I/O, Compute.
- Verified OCIO view-transform semantics against OpenColorIO docs (see reference below).
- Added high-level dataflow/codepath diagrams to `AGENTS.md`.

**External reference:**
- OpenColorIO ViewTransform docs (reference-space semantics and direction usage):
  https://opencolorio.readthedocs.io/en/latest/api/viewtransform.html

## Confirmed Findings (ordered by severity)

### Critical
1) **Potential data loss on write failure**
   - `attempt_delete_file_on_write_error` removes the target path even if file creation failed, which can delete a valid pre-existing file when `File::create` errors (permissions, path issues).
   - Ref: `crates/vfx-exr/src/io.rs:45`
   - Fix: Track whether a new file was actually created before deleting; only delete if the file was created by this attempt.

### High
2) **OCIO ViewTransform dual-reference logic is incomplete**
   - `ViewTransform` stores scene/display reference transforms but does not track which reference space it represents.
   - `display_processor` selects the first available transform (from/to scene/display) without validating the reference space type or direction semantics.
   - This diverges from OCIO docs where `from_reference` is used when going out toward the display, and the reference space type must be honored.
   - Refs: `crates/vfx-ocio/src/display.rs:203`, `crates/vfx-ocio/src/display.rs:209`, `crates/vfx-ocio/src/config.rs:1048`, `crates/vfx-ocio/src/config.rs:1054`
   - Fix: Add reference space type to `ViewTransform` and update `display_processor` to follow OCIO v2 rules (scene->display vs display->display).

3) **OCIO named transform API promises behavior not implemented**
   - `ocionamedtransform` docs claim support for OCIO v2 named transforms, but the implementation only parses `X_to_Y` patterns and built-in aliases. It never resolves `Config::named_transform`.
   - Refs: `crates/vfx-io/src/imagebufalgo/ocio.rs:506`, `crates/vfx-io/src/imagebufalgo/ocio.rs:546`, `crates/vfx-ocio/src/config.rs:1999`
   - Fix: If a named transform exists in config, build a processor from it before falling back to `X_to_Y` parsing.

4) **Matrix inverse path has no singular-matrix handling**
   - `compile_transform` blindly inverts a 4x4 matrix; singular matrices will produce undefined results rather than a controlled error or fallback.
   - Ref: `crates/vfx-ocio/src/processor.rs:981`
   - Fix: Detect non-invertible matrices (e.g., determinant check in glam) and surface an error or skip with clear diagnostic.

### Medium
5) **ImageBuf metadata accessors are stubbed**
   - `nsubimages`/`nmiplevels` always return 1 regardless of input, which breaks expectations for multi-part or mipmapped sources.
   - `contiguous()` always returns true even for non-local or cached storage.
   - Refs: `crates/vfx-io/src/imagebuf/mod.rs:519`, `crates/vfx-io/src/imagebuf/mod.rs:525`, `crates/vfx-io/src/imagebuf/mod.rs:677`
   - Fix: Query from file/cache metadata when available; return false for non-local storage or unknown layout.

6) **OCIO named transform unpremult flag is ignored**
   - The `unpremult` flag is explicitly ignored, meaning alpha-premult workflows are silently incorrect when enabled.
   - Ref: `crates/vfx-io/src/imagebufalgo/ocio.rs:510`
   - Fix: Implement unpremultiply → transform → premultiply, or remove the flag until supported.

7) **ociofiletransform ignores ColorConfig**
   - The `_config` parameter is unused, so search paths, context vars, and OCIO path resolution are not honored.
   - Ref: `crates/vfx-io/src/imagebufalgo/ocio.rs:274`
   - Fix: Resolve file path via `ColorConfig` (search paths and context), then pass into `FileTransform`.

### Low
8) **Dead-code candidate: `Error::Aborted`**
   - Appears to be unused by library code; only referenced in tests.
   - Refs: `crates/vfx-exr/src/error.rs:29`, `crates/vfx-exr/tests/roundtrip.rs:180`
   - Action: Confirm public API usage or deprecate for removal.

## Deduplication / Single Source of Truth

1) **Transfer functions duplicated across crates**
   - OCIO processor hardcodes OETF/EOTF math while `vfx-transfer` provides equivalent functions.
   - Refs: `crates/vfx-ocio/src/processor.rs:24`, `crates/vfx-transfer/src/gamma.rs:32`
   - Recommendation: Use `vfx-transfer` as the canonical implementation and re-export / wrap in OCIO to reduce drift.

2) **CDL and image storage duplication in compute layer**
   - `vfx-compute` defines its own `Cdl` and `ComputeImage` instead of referencing `vfx-color` / `vfx-core` types.
   - Refs: `crates/vfx-compute/src/color.rs:44`, `crates/vfx-compute/src/image.rs:33`
   - Recommendation: Introduce a shared core trait or type alias to unify CPU/GPU pipelines without breaking APIs.

## Dataflow References

- Updated `AGENTS.md` with consolidated dataflow/codepath diagrams:
  - CLI/Batch pipeline
  - OCIO processor build/apply
  - EXR deep read
  - Viewer runtime loop

## GEM.md Recheck (Confirmed vs Not Confirmed)

Confirmed:
- CDL duplication in compute layer (`vfx-compute` defines its own `Cdl` instead of reusing `vfx-color`). Ref: `crates/vfx-compute/src/color.rs:44`
- ComputeImage uses `Vec<f32>` (not Arc/COW), so crossing boundaries implies deep copy. Ref: `crates/vfx-compute/src/image.rs:33`
- EXR seek overflow hazard is present as a FIXME. Ref: `crates/vfx-exr/src/io.rs:228`
- Known unwrap-risk comment exists in EXR writer. Ref: `crates/vfx-exr/src/image/write/layers.rs:166`

Not confirmed:
- The exact count (~240) of TODO/FIXME markers in `vfx-exr` (needs a full count scan if required).

## Proposed Execution Plan

- [ ] Fix `attempt_delete_file_on_write_error` to only delete files created by the current write attempt.
- [ ] Implement OCIO ViewTransform reference-space type and correct selection logic in `display_processor`.
- [ ] Wire OCIO named transforms into `ocionamedtransform` using `Config::named_transform`.
- [ ] Add singular-matrix detection in OCIO matrix inversion path and surface errors.
- [ ] Implement ImageBuf metadata accessors (`nsubimages`, `nmiplevels`, `contiguous`) with real data sources.
- [ ] Implement unpremult pipeline for OCIO named transforms or remove flag and update docs.
- [ ] Use `ColorConfig` for file LUT resolution in `ociofiletransform`.
- [ ] Audit `Error::Aborted` usage and decide on deprecation/removal.
- [ ] Unify compute-layer `Cdl` with `vfx-color::Cdl` to prevent drift.
- [ ] Align `ComputeImage` memory model with `vfx-core` (Arc-backed or shared view) to enable zero-copy transitions.
- [ ] Preserve and unify the universal compute engine (auto backend selection across CUDA/WGPU/CPU) while adding streaming and tiling as first-class paths.
- [ ] Fix overflow-prone seek delta in `vfx-exr` Tracking::seek_read_to.
- [ ] Replace or guard risky unwrap path in EXR layer header inference.

## Approval Checkpoint

Awaiting approval before applying code changes for the items above.
