# Plan 3: Full OCIO/OIIO Parity Track (CPU-First)

This plan sequences work to reach OCIO v2 and OIIO-like parity within the current scope: existing formats + HDR, EXR deep/multipart later, unified CLI, and metadata integration.

## Scope and Principles
- OCIO v2 only (no v1 compatibility).
- CPU parity first; design an engine trait for future backends.
- Formats: keep current set, add HDR/RGBE; deep/multipart EXR later.
- Metadata: integrate from exiftool-rs and map into vfx-io metadata.
- CLI: one smart tool with consistent command/option system; not 1:1 with OIIO/OCIO, but better UX.

## Status Update
- Completed: OCIO strict parsing + file rules + view/looks + LUT domain + .cube FileTransform.
- Completed: HDR format support (RGBE read/write, RLE scanlines) and format detection.
- In progress: Metadata integration (Attrs/AttrValue added; EXR/HDR metadata mapped; remaining formats pending).

## Phase 1: OCIO v2 Correctness and Rules
1. Strict parsing policy
   - Enforce `strictparsing` on all transform parse errors.
   - Collect warnings in non-strict mode.
2. File rules
   - Implement basic rules (glob + extension), regex rules, default rule enforcement.
   - Normalize paths and apply first-match semantics.
3. Display pipeline
   - Apply view transforms and view looks in display processing.
   - Respect display-reference vs scene-reference paths.

## Phase 2: LUT and Transform Parity
1. LUT domain correctness
   - Preserve 1D/3D LUT domains through compile and apply.
2. FileTransform formats
   - Add .cube support and unify LUT loaders.
3. Transfer functions
   - Deduplicate OCIO transfer math by using vfx-transfer.
   - Ensure constants and edge behaviors match reference specs.

## Phase 3: Metadata Integration
1. Metadata ingestion
   - Integrate exiftool-rs parsing into vfx-io.
   - Map extracted tags into ImageData::Metadata.
2. Metadata preservation
   - Pipe metadata through read/write for supported formats (EXR/PNG/JPEG/TIFF/HDR as available).

## Phase 4: HDR Format Support
1. Implement RGBE/HDR reader/writer in vfx-io.
2. Wire into format detection (magic bytes + extension).
3. Tests with reference fixtures.

## Phase 5: Unified CLI
1. Define command system and high-level UX
   - Single binary with subcommands and shared options.
2. Implement conversion pipelines
   - I/O + OCIO transforms + ops + metadata preservation.
3. Add profiling/logging and clear error diagnostics.

## Phase 6: Engine Abstraction
1. Add engine trait (CPU implementation now).
2. Move Processor application behind engine interface.
3. Prepare for future GPU backend.

## Phase 7: Test Parity
1. Add reference tests for OCIO transforms (vs OCIO/CTL reference).
2. Golden image tests for I/O and ops.
3. Performance sanity checks (non-regression).

## Deliverables
- OCIO v2 correctness (rules + view/looks + LUT domain correctness).
- HDR format support.
- Metadata integration.
- Unified CLI.
- Engine abstraction + tests.

## Open Decisions (Need Confirmation)
- Metadata integration: path dependency vs vendoring exiftool-rs.
- HDR format: HDR/RGBE only or include .pic.
- Metadata schema: keep ImageData::Metadata or adopt richer struct.
