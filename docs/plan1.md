# VFX-RS Audit Report and Plan (Plan 1)

Date: 2025-01

## Executive Summary
This audit focused on the OCIO pipeline, I/O layer, CLI, and the color/ops stack. The core types are coherent, but the OCIO config parsing and processor pipeline silently drop important transforms, and several features are implemented twice (with diverging behavior). The most urgent issues are: (1) OCIO config parsing ignores look/view transforms and file rule semantics, (2) file transforms silently skip unsupported LUT formats, and (3) LUT domain handling is lost for 3D LUTs in the OCIO processor. These lead to incorrect output without visible errors.

## Critical Findings
1) OCIO config parsing silently ignores transform errors and skips transforms
- Files: `crates/vfx-ocio/src/config.rs`
- Problem: `parse_colorspace` and `parse_raw_transform` ignore errors (`if let Ok(t) ...`) and `filter_map` drops failing transforms. `strict_parsing` is stored but never applied. This can produce a config that looks valid yet has missing transforms.
- Impact: Incorrect color conversions without any diagnostics.

2) OCIO looks and view transforms are parsed but never applied
- Files: `crates/vfx-ocio/src/config.rs`, `crates/vfx-ocio/src/display.rs`, `crates/vfx-ocio/src/processor.rs`
- Problem: `RawLook.transform`, `RawLook.inverse_transform`, and `RawView.view_transform` are parsed in the YAML schema but never wired into the `Look` or display pipeline. `display_processor` ignores view transforms and view looks entirely.
- Impact: Display pipelines and look grades do nothing in OCIO configs.

3) OCIO file rules implementation does not match OCIO rule semantics
- Files: `crates/vfx-ocio/src/config.rs`
- Problem: `colorspace_from_filepath` uses substring matching, but OCIO rules require glob/regex (basic/regex rules) and a mandatory default rule. Current implementation can return wrong colorspace or none at all.
- Reference: OCIO rules API docs (basic rule uses glob patterns, regex rule uses regex, default rule required).
- Impact: Automatic colorspace assignment is unreliable.

4) 3D LUT domain is discarded in OCIO processor
- Files: `crates/vfx-ocio/src/processor.rs`, `crates/vfx-lut/src/lut3d.rs`
- Problem: `compile_lut3d` flattens LUT data but drops `domain_min`/`domain_max`. The runtime LUT operation assumes input is [0,1].
- Impact: Any LUT with non-default domain produces wrong results.

## Major Findings
1) FileTransform silently skips unsupported LUT formats
- Files: `crates/vfx-ocio/src/processor.rs`
- Problem: Unsupported extensions are ignored with no error, and `.cube` is not supported even though vfx-lut can parse it.
- Impact: Transforms are dropped without warning; configs behave incorrectly.

2) Look transforms are not stored or applied
- Files: `crates/vfx-ocio/src/config.rs`, `crates/vfx-ocio/src/look.rs`
- Problem: `RawLook.transform` and `RawLook.inverse_transform` are ignored when building `Look`.
- Impact: `processor_with_looks` cannot ever apply the intended transform chain.

3) OCIO context variables are never applied to file paths
- Files: `crates/vfx-ocio/src/config.rs`, `crates/vfx-ocio/src/context.rs`
- Problem: `Context` exists but `FileTransform` paths are built without variable substitution or search-path resolution.
- Impact: Configs using `$VAR` paths fail silently or load wrong paths.

4) RangeTransform style is ignored
- Files: `crates/vfx-ocio/src/config.rs`, `crates/vfx-ocio/src/processor.rs`
- Problem: Parsing forces `RangeStyle::Clamp` and apply path always clamps; `NoClamp` is not respected.
- Impact: Configs using `RangeTransform` with no clamping behave incorrectly.

## Medium Findings
1) Duplicate color pipelines and LUT parsing
- Files: `crates/vfx-color/*`, `crates/vfx-ocio/*`, `crates/vfx-cli/src/commands/lut.rs`
- Problem: Transfer functions and LUT parsing are duplicated (vfx-transfer vs vfx-ocio apply_transfer; vfx-lut vs CLI cube parser). This risks inconsistent math and behavior.
- Impact: Divergent outputs for the same operations depending on code path.

2) Two different image types in the same stack
- Files: `crates/vfx-core/*`, `crates/vfx-io/src/lib.rs`
- Problem: `vfx-core::Image` and `vfx-io::ImageData` are parallel universes. There is no canonical conversion layer.
- Impact: The pipeline is fragmented; dataflow has extra conversions or dead-ends.

3) PixelFormat::F16 stores data as f32 in I/O container
- Files: `crates/vfx-io/src/lib.rs`
- Problem: `PixelFormat::F16` uses `PixelData::F32` storage, but `bytes_per_channel` reports 2 bytes. This is inconsistent for any raw or size-sensitive use.
- Impact: Size calculations and format expectations can be wrong.

## Minor / TODO Findings
- `vfx-cli` convert depth option is declared but not implemented (`crates/vfx-cli/src/commands/convert.rs`).
- LUT inversion is TODO in CLI and OCIO processor (`crates/vfx-cli/src/commands/lut.rs`, `crates/vfx-ocio/src/processor.rs`).

## Recommended Fixes (High-Level)
1) Make OCIO parsing strict by default
- Honor `strict_parsing` and surface errors instead of silently skipping transforms.
- When not strict, still collect and report warnings.

2) Implement OCIO rule types correctly
- Support basic rules with glob matching and regex rules.
- Require a default rule and apply OCIO v1 path search rule behavior.

3) Wire looks and view transforms into the display pipeline
- Parse `RawLook.transform`/`inverse_transform` and store them.
- Apply view-level looks and view transforms in `display_processor`.

4) Restore LUT domain handling
- For 3D LUTs, store domain and use it during lookup (same normalization as vfx-lut).

5) Unify transfer functions and LUT parsing
- Use `vfx-transfer` in `vfx-ocio::processor` to prevent drift.
- Replace CLI `CubeLut` parser with `vfx-lut` loaders.

6) Define a single canonical image data path
- Provide conversion glue between `vfx-io::ImageData` and `vfx-core::Image`.
- Choose one type as the primary dataflow container for ops and color.

## Plan (Next Steps)
- [ ] Implement strict parsing and error reporting for OCIO config transforms.
- [ ] Implement OCIO file rules per spec (glob/regex/default rule).
- [ ] Apply look transforms and view transforms in display processors.
- [ ] Add LUT domain support to OCIO 3D LUT ops and `.cube` support in FileTransform.
- [ ] Deduplicate transfer/LUT parsing (reuse vfx-transfer and vfx-lut).
- [ ] Add explicit conversion adapters between `ImageData` and `Image`.
- [ ] Implement CLI bit-depth conversion and LUT inversion (if inversion stays supported).

## References Consulted
- OCIO Rules API: https://opencolorio.readthedocs.io/en/latest/api/rules.html
