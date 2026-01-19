# Bug Hunt Report - vfx-rs

## Scope
- vfx-io EXR read/write path, deep EXR integration
- vfx-exr deep read/write capabilities

## Findings

### 1) Deep EXR API in vfx-io is stubbed despite local deep support
**Severity:** High

**Evidence**
- `vfx-io` exposes `read_deep`/`write_deep*` but they always return `UnsupportedFeature`: `crates/vfx-io/src/exr.rs:888`, `crates/vfx-io/src/exr.rs:915`, `crates/vfx-io/src/exr.rs:925`.
- `vfx-io` also has a deep helper module that can read deep data, but its write helper is stubbed: `crates/vfx-io/src/exr_deep.rs:311`, `crates/vfx-io/src/exr_deep.rs:354`.
- `vfx-exr` in this repo already provides deep read/write implementations: `crates/vfx-exr/src/image/read/deep.rs:131`, `crates/vfx-exr/src/image/write/deep.rs:110`, `crates/vfx-exr/src/image/write/deep.rs:165`.
- Tests lock in the stub behavior: `crates/vfx-io/src/exr.rs:1225`.

**Impact**
- Any consumer calling `vfx_io::exr::read_deep` or `write_deep*` will always fail, even though the repo contains working deep EXR I/O. This contradicts the public deep-support claims and blocks deep pipelines.

**Recommended fix**
- Wire `vfx_io::exr::read_deep` to `vfx_io::exr_deep::read_deep_exr` (and/or directly to `vfx_exr::image::read::deep`), with conversion into `DeepData` if required.
- Replace `vfx_io::exr::write_deep*` and `vfx_io::exr_deep::write_deep_exr` stubs with calls to `vfx_exr::image::write::deep` helpers.
- Update/remove `test_deep_exr_stubs` to assert real behavior (feature gating if necessary).

## Open questions / assumptions
- Confirm desired public API: should `vfx_io::exr::read_deep` return `DeepData` (AoS) or expose `DeepSamples` (SoA) directly? Current deep module supports SoA; conversion code may be needed.

## Suggested next steps
- Implement deep I/O wiring and add tests that round-trip a small deep image.
- Update README/feature docs if the public API changes.
