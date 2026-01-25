# VFX-RS Documentation Fixes - Progress

## Current Status (2026-01-24)

**Total bugs in FINDINGS.md:** 491
**Fixed so far:** ~448 (bugs 1-31, 152-448, duplicates)
**Remaining:** ~43 (bugs 32-151, 449-491 - mostly implementation TODOs, not doc bugs)

### Batch 12: Bugs 412-437 (CLI docs completion)
- [x] 412-413: tests.md structure - already fixed
- [x] 414: dependency diagram - already fixed
- [x] 415: vfx_ops::resize removed
- [x] 416: PSD layers documented
- [x] 417: CLI flags corrected
- [x] 418: TX format removed
- [x] 419-420: channel specs and alpha defaults fixed
- [x] 421: blend modes noted as not implemented
- [x] 422: --warn description corrected
- [x] 423: grep limitations documented
- [x] 424: LUT formats corrected
- [x] 425: maketx mipmaps implemented
- [x] 426: rayon claim removed from rotate docs
- [x] 427: rotation direction documented correctly
- [x] 428: layers subcommands corrected
- [x] 429-430: merge-layers and batch docs fixed
- [x] 431: resize -H flag documented
- [x] 433: ACES variants supported
- [x] 434: viewer feature documented
- [x] 435: sharpen algorithm clarified
- [x] 436: warp formulas corrected
- [x] 437: extract-layer behavior documented

### Batch 13: Bugs 443-448 (appendix docs)
- [x] 443: CLI sequence patterns clarified (not supported at CLI level)
- [x] 444: .tx removed from udim.md
- [x] 445: JPEG CMYK documented correctly
- [x] 446: CSP write no longer marked unsupported
- [x] 447: PNG 16-bit claim removed
- [x] 448: srgb_to_linear -> srgb::eotf

### Bugs 449-491: Implementation TODOs (not doc bugs)
These are code-level issues like:
- ImageBuf::contiguous() always returns true (TODO)
- Streaming sources reading entire file (TODO)
- GPU cache not integrated (TODO)
- Feature-matrix claims not matching implementation

## Session Progress (2026-01-24)

### Performance Fix
- [x] **VIEWER PERFORMANCE**: Fixed query_pixel() converting entire image on every mouse move
  - Added CachedPixels struct to cache raw f32 data once in regenerate_texture()
  - Now uses O(1) array lookup instead of O(width*height) conversion

### Documentation Fixes (Batch 1: bugs 180-201)

#### vfx-core crate docs (bugs 180-181)
- [x] 180: Fixed Image<> to include channel count const generic (Image<Srgb, f32, 3>)
- [x] 180: Added Roi argument to view()/view_mut() calls
- [x] 181: Changed srgb_to_linear to use vfx_transfer::srgb::eotf()

#### vfx-io crate docs (bugs 182-190)
- [x] 182-190: Fixed EXR, HEIF, AVIF, sequence, UDIM APIs and formats

#### vfx-color crate docs (bugs 191-194)
- [x] 191-194: Already correct in current docs

#### vfx-compute crate docs (bugs 195-201)
- [x] 195-201: Fixed into_vec, convert module, matrix API, GpuLimits

### Documentation Fixes (Batch 2: bugs 202-236)

#### vfx-compute (bugs 202-203)
- [ ] 202: LayerProcessor API not documented (needs investigation)
- [ ] 203: Processor::auto() description (needs investigation)

#### vfx-cli crate docs (bugs 204-211)
- [x] 204-207: Already correct (flags, commands, output paths)
- [x] 208: INVALID - docs correct (nearest/bilinear/bicubic/lanczos)
- [x] 209: Fixed --inverse to --invert
- [x] 210-211: Already correct (batch, viewer feature)

#### vfx-ops crate docs (bugs 212-219)
- [x] 212: Already correct (width/height args)
- [x] 213: Fixed BlendMode::Over to BlendMode::Normal
- [x] 214-218: Already correct (premultiply, rotate, warp, layer_ops, guard)
- [x] 219: Added vfx-color to dependencies

#### vfx-lut crate docs (bugs 220-223)
- [x] 220: Fixed apply_tetrahedral to with_interpolation + apply
- [x] 221: Fixed ProcessNode struct-style matching and size()
- [x] 222: Fixed apply_pixel to apply_rrt_odt_srgb buffer style
- [x] 223: Added additional formats table

#### vfx-transfer crate docs (bug 224)
- [x] 224: Added d_log and davinci_intermediate

#### vfx-ocio crate docs (bugs 225-228)
- [x] 225: Fixed from_str to from_yaml_str with working_dir
- [x] 226: Fixed processor_opt to processor_with_opts
- [x] 227: Fixed processor_with_look to processor_with_looks
- [x] 228: Removed broken link to OCIO_PARITY_AUDIT.md

#### vfx-icc crate docs (bugs 229-233)
- [x] 229: Fixed from_bytes to from_icc
- [x] 230: Fixed lab_d50 to lab()
- [x] 231: Fixed StandardProfile::SRgb to Srgb, removed ?
- [x] 232: Fixed convert_rgb argument order
- [x] 233: Fixed IccError variants

#### vfx-view crate docs (bug 234)
- [x] 234: Removed non-existent layer field from ViewerConfig

#### vfx-rs-py crate docs (bugs 235-236)
- [x] 235: Already correct (numpy, Image constructor)
- [x] 236: Added u32 to format list

### Documentation Fixes (Batch 3: bugs 237-243)

#### vfx-rs-py remaining issues (bugs 237-243)
- [x] 237: Write quality/compression params (docs were already correct)
- [x] 238: Replaced vfx_rs.color with Image methods and ops functions
- [x] 239: Fixed lut.apply_3d/apply_1d to Lut3D.apply()/Lut1D.apply()
- [x] 240: Fixed ops resize/blur/blend signatures
- [x] 241: Fixed layer_names() to property, updated Multi-Layer EXR section
- [x] 242: Fixed IoError/FormatError to standard IOError/ValueError
- [x] 243: Clarified numpy() always copies data

### Documentation Fixes (Batch 4: bugs 244-263)

#### vfx-tests docs (bugs 244-247)
- [x] 244: Structure was already correct
- [x] 245: Commands were already correct
- [x] 246: Test assets structure was already correct
- [x] 247: Updated dependencies (added vfx-math, vfx-lut, vfx-transfer, vfx-primaries, serde, sha2)

#### vfx-bench docs (bugs 248-252)
- [x] 248: Updated benchmark commands to actual groups (transfer, lut3d)
- [x] 249: Rewrote Benchmark Categories with actual groups
- [x] 250: Removed non-existent GPU benchmarks section
- [x] 251: Removed invalid #[bench] Memory Benchmarks section
- [x] 252: Updated dependencies and single vfx_bench target

#### vfx-math docs (bugs 253-257)
- [x] 253: Fixed adapt_matrix(BRADFORD, D65, D60) parameter order
- [x] 254: Changed DCI to DCI_WHITE
- [x] 255: Removed catmull_rom, replaced with saturate()
- [x] 256: Replaced SIMD functions with actual: batch_mul_add, batch_pow, etc.
- [x] 257: Fixed rgb_to_luminance/linearize_srgb to batch_rgb_to_luma

#### Introduction docs (bugs 258-260)
- [x] 258: Fixed vfx-core description (PixelFormat, Roi not ImageData)
- [x] 259: Fixed vfx-math description (Vec3, Mat3 not Mat4)
- [x] 260: Replaced broken plan3.md link

#### Feature flags, Quick start (bugs 261-263)
- [x] 261: Features were already correct in current docs
- [x] 262: Changed git clone URL to vfx-rs/vfx-rs
- [x] 263: Changed -h to -H for height

### Documentation Fixes (Batch 5: bugs 264-271)

#### docs/src/python.md (bugs 264-265)
- [x] 264: Added u32 to format list
- [x] 265: Clarified numpy() always copies data

#### dev/benchmarks.md (bugs 268-270)
- [x] 268: Updated benchmark commands to actual groups
- [x] 269: Replaced IO/resize examples with actual transfer/lut benchmarks
- [x] 270: Fixed Lut3D API to use identity() and apply()

#### dev/testing.md (bug 271)
- [x] 271: Fixed test structure and asset paths

### Documentation Fixes (Batch 6: bugs 272-292)

#### dev/testing.md (bug 272)
- [x] 272: Changed `vfx convert --resize` to `vfx resize -w -H`

#### dev/adding-formats.md (bug 273)
- [x] 273: Updated default features to show all 6

#### internals/README.md (bugs 274-275)
- [x] 274: Already fixed (tests are inline #[cfg(test)])
- [x] 275: Already fixed (correct feature list)

#### internals/pipeline.md (bug 276)
- [x] 276: Fixed ExrWriter::with_options pattern

#### internals/color.md (bugs 277-278)
- [x] 277: Fixed Mat3::from_col_vecs and diagonal()
- [x] 278: Fixed adapt_matrix signature

#### internals/exr.md (bug 279)
- [x] 279: Fixed ExrWriter compression example

#### aces/vfx-rs-aces.md (bug 280)
- [x] 280: Fixed ACES transforms API, module-level primaries

#### aces/idt.md (bug 281)
- [x] 281: Changed linearize_srgb to srgb::eotf

#### aces/acescg.md (bug 282)
- [x] 282: Fixed srgb_to_acescg args and primaries constants

#### aces/transfer-functions.md (bug 283)
- [x] 283: Fixed all transfer function names to *_encode/*_decode/*_eotf/*_oetf

#### aces/examples.md (bugs 284-285)
- [x] 284: Already fixed (uses shell loops, not batch --op aces)
- [x] 285: Already fixed (extract-layer before aces)

#### aces/lmt.md (bug 286)
- [x] 286: Replaced --look CLI with Rust API example

#### aces/pipeline.md (bug 287)
- [x] 287: Removed --odt flag, added note about sRGB-only CLI

#### aces/color-spaces.md (bug 288)
- [x] 288: Fixed Primaries:: to module-level constants

#### installation.md (bug 289)
- [x] 289: Changed `icc` feature to vfx-icc crate

#### programmer/README.md (bugs 290-292)
- [x] 290: Already correct (vfx_io::ImageData)
- [x] 291: Already correct (no ChannelType mentioned)
- [x] 292: Fixed resize to resize_f32 with full example

### Documentation Fixes (Batch 7: bugs 293-310)

#### internals/gpu.md (bugs 293-296)
- [x] 293: Fixed GpuPrimitives trait: src/dst pattern, no exec_exposure, limits() returns &GpuLimits
- [x] 294: Fixed ImageHandle to size_bytes(), renamed *Handle to *Image
- [x] 295: Fixed TiledExecutor to execute_color()
- [x] 296: Fixed apply_batch to apply_color_ops

#### aces/odt.md (bugs 297-298)
- [x] 297: Changed vfx color --from/--to to vfx ocio --src/--dst, added limitations note
- [x] 298: Added note about sRGB/Rec.709 only, HDR ODTs require OCIO

#### appendix/formats.md (bugs 299-303, 305)
- [x] 299: Fixed JPEG CMYK to "✓ (read, auto-converted to RGB)"
- [x] 300: Fixed PNG 16-bit note to clarify defaults to 8-bit
- [x] 301: Fixed PSD Layers to "✓ (via read_layers())"
- [x] 302: Fixed PSD bit depth to "8/16-bit input | ✓ (output always 8-bit RGBA)"
- [x] 303: Fixed JPEG Progressive to "Read only (write uses baseline)"
- [x] 305: Clarified TIFF multi-page to "partial (read-only, no page selection API)"

#### appendix/feature-matrix.md (bug 304)
- [x] 304: Removed TIFF "tiles" claim

#### CODE_BUG (bug 306)
- [ ] 306: convert.rs Format::detect bug (not documentation, is actual code bug)

#### crates/io.md (bug 307)
- [x] 307: Already correct (uses `if let Some(ref hdr)` and `hdr_info.as_ref()`)

#### cli/convert.md (bugs 308-309)
- [x] 308: Fixed DPX-to-EXR comment to "preserves log encoding, no color transform"
- [x] 309: Added `-q, --quality` option to table

#### cli/layers.md (bug 310)
- [x] 310: Already correct (shows separate top-level commands)

### Documentation Fixes (Batch 8: bugs 311-330)

#### cli/extract-layer.md, merge-layers.md (bugs 311-312)
- [x] 311: Already correct (docs say --layer required)
- [x] 312: Already correct (docs show `-n beauty -n diffuse` syntax)

#### cli/channel-extract.md (bugs 313-314)
- [x] 313: Fixed comma-separated to `-c R -c G -c B` syntax
- [x] 314: Added note about custom channel names not yet supported

#### cli/udim.md (bugs 315-316)
- [x] 315: Added compression limitation note
- [x] 316: Added verbose flag note for format details

#### cli/transform.md (bug 317)
- [x] 317: Added "Multi-layer EXR: first layer only" note

#### cli/composite.md (bugs 318-319)
- [x] 318: Listed only available modes, added note about unimplemented
- [x] 319: Changed to "Processing is done on CPU"

#### cli/warp.md (bugs 320-322)
- [x] 320: Documented k1=frequency, k2=amplitude
- [x] 321: Added clamping note "(clamped to >= 1.0)"
- [x] 322: FALSE ALARM - warp DOES use rayon (par_chunks_mut)

#### cli/README.md (bugs 323-326)
- [x] 323: Removed non-existent --threads option
- [x] 324-326: FALSE ALARM - aliases DO exist (visible_alias)

#### cli/resize.md (bugs 327-328)
- [x] 327: FALSE ALARM - docs correct, code supports all documented filters
- [x] 328: Changed to "Processing is done on CPU"

#### cli/blur.md (bugs 329-330)
- [x] 329: Changed "Parallel processing via rayon" to "Single-threaded processing"
- [x] 330: Changed to "Uses 2D convolution (not separable, O(n²) per pixel)"

### Documentation Fixes (Batch 9: bugs 331-350)

#### cli/paste.md (bug 331)
- [x] 331: Changed "Preserves format" to "Output is always float32"

#### programmer/imagebufalgo/README.md (bug 332)
- [x] 332: Already correct (docs show correct `blur`, `blur_into` patterns)

#### Internal implementation bugs (bugs 333-337)
- [ ] 333-337: Internal code issues, not documentation

#### Already fixed (bugs 338-341)
- [x] 338-341: Already marked FIXED in FINDINGS.md

#### Format/pipeline docs (bugs 342-346)
- [ ] 342-346: Internal implementation limitations, not documentation

#### programmer/color-management.md (bugs 347-349)
- [x] 347: Fixed Primaries imports to module-level constants
- [x] 348: Already correct (uses eotf/oetf)
- [x] 349: Already correct (uses cube::read_3d)

#### programmer/core-api.md (bug 350)
- [x] 350: Already correct (notes ImageData is in vfx_io)

### Documentation Fixes (Batch 10: bugs 351-370)

#### crates/ocio.md (bugs 352-356)
- [x] 352-354: Already correct (from_yaml_str, processor_with_opts, processor_with_looks)
- [x] 355: Fixed FileRuleKind match pattern
- [x] 356: Fixed Option handling (if let Some)

#### crates/compute.md (bugs 357-363)
- [x] 357-363: All already correct in current docs

#### crates/cli.md (bugs 365-369)
- [x] 365-369: All already correct in current docs
- [x] Removed --threads from Global Options

#### core-api.md, color-management.md (bugs 347, 350, 370)
- [x] All already correct in current docs

### Documentation Fixes (Batch 11: bugs 371-400)

#### programmer/color-management.md (bug 371)
- [x] 371: Already correct (uses apply_rrt_odt_srgb)

#### crates/io.md (bug 372)
- [x] 372: Fixed read_layered to read_layers

#### programmer/imagebufalgo/README.md (bug 373)
- [x] 373: Already correct (uses blur, blur_into, ResizeFilter::Lanczos)

#### programmer/ocio-integration.md (bug 374)
- [x] 374: Already correct (uses bake_lut_1d/bake_lut_3d)

#### programmer/imagebufalgo/filters.md (bug 375)
- [x] 375: Already correct (uses roi: Option<Roi3D>)

#### crates/bench.md (bug 376)
- [x] 376: Already correct (lists transfer/lut3d/cdl/simd groups)

#### aces/vfx-rs-aces.md (bug 377)
- [x] 377: Major rewrite - fixed transfer functions to module::encode/decode, OCIO API to config methods

#### aces/transfer-functions.md (bug 378)
- [x] 378: Fixed rec709::oetf/eotf, pq::oetf/eotf, hlg::oetf/eotf

#### crates/color.md (bugs 379-381)
- [x] 379-381: Already correct (Pipeline with apply(), lut3d())

#### crates/core.md (bugs 382-384)
- [x] 382: Fixed Rgb<Srgb, f32> and Rgba<AcesCg, f32> with T parameter
- [x] 383-384: Already correct (Image<C, T, N>, srgb::eotf)

#### crates/icc.md (bugs 385-388)
- [x] 385-388: Already correct (from_icc, lab()?, from_standard, convert_rgb params)

#### crates/lut.md (bugs 389-392)
- [x] 389-390: Already correct (with_interpolation, struct-like match)
- [x] 391: Fixed from_fn -> from_data and Lut1D::gamma
- [x] 392: Fixed Lut3D baking example (from_data instead of set)

#### crates/math.md (bugs 393-395)
- [x] 393-395: Already correct (lerp/smoothstep/saturate, batch_* functions)

#### crates/ops.md (bugs 396-400)
- [x] 396-400: Already correct (composite with w/h, premultiply_inplace, rotate_90_cw/ccw, barrel/pincushion, layer_ops)

## Next to Fix

### Bugs 401-420 (more Python bindings, format docs)

## Files Modified This Session (continued)

29. `docs/src/internals/gpu.md` - GpuPrimitives trait, ImageHandle, TiledExecutor, apply_color_ops
30. `docs/src/aces/odt.md` - OCIO examples, limitation notes
31. `docs/src/appendix/formats.md` - JPEG CMYK/Progressive, PNG 16-bit, PSD layers/bit depth, TIFF multi-page
32. `docs/src/appendix/feature-matrix.md` - Removed TIFF tiles
33. `docs/src/cli/convert.md` - DPX comment, quality option
34. `FINDINGS.md` - Marked bugs 293-310 as FIXED

14. `docs/src/dev/testing.md` - Fixed vfx resize command
15. `docs/src/dev/adding-formats.md` - Updated default features
16. `docs/src/internals/pipeline.md` - ExrWriter pattern
17. `docs/src/internals/color.md` - Mat3 methods, adapt_matrix signature
18. `docs/src/internals/exr.md` - ExrWriter compression
19. `docs/src/aces/vfx-rs-aces.md` - Primaries constants, ACES API
20. `docs/src/aces/idt.md` - srgb::eotf
21. `docs/src/aces/acescg.md` - Module-level constants
22. `docs/src/aces/transfer-functions.md` - All function names
23. `docs/src/aces/lmt.md` - Rust API for looks
24. `docs/src/aces/pipeline.md` - Removed --odt
25. `docs/src/aces/color-spaces.md` - Module-level primaries
26. `docs/src/installation.md` - vfx-icc crate note
27. `docs/src/programmer/README.md` - resize_f32 example
28. `FINDINGS.md` - Marked bugs 272-292 as FIXED
