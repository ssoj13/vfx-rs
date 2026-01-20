# Findings

## OpenColorIO parity gaps

1) FileTransform supports only a small subset of OCIO file formats.
   - Evidence (OCIO formats registry): `_ref/OpenColorIO/src/OpenColorIO/transforms/FileTransform.cpp:333`
   - Evidence (vfx-ocio FileTransform extension match): `crates/vfx-ocio/src/processor.rs:963`
   - Impact: FileTransform with 3DL/CC/CCC/CDL/CSP/Discreet1DL/HDL/ICC/Iridas/Resolve/Pandora/SpiMtx/Truelight/VF will be ignored (no-op).

2) Config parser does not load several OCIO transform types.
   - Missing tags in parser: `Lut1DTransform`, `Lut3DTransform`, `ExponentWithLinearTransform`, `DisplayViewTransform`, `GradingHueCurveTransform`.
   - Evidence (parser has no tags): `crates/vfx-ocio/src/config.rs`
   - Evidence (types exist in Rust): `crates/vfx-ocio/src/transform.rs`
   - Evidence (OCIO TransformType list): `_ref/OpenColorIO/include/OpenColorIO/OpenColorTypes.h:361`

3) ExponentWithLinearTransform negative handling diverges from OCIO.
   - OCIO: “Negative values are never clamped.” `_ref/OpenColorIO/include/OpenColorIO/OpenColorTransforms.h:900`
   - vfx-ocio default clamps negatives via NegativeStyle::Clamp.
   - Evidence (transform defaults): `crates/vfx-ocio/src/transform.rs:534`
   - Evidence (processor behavior): `crates/vfx-ocio/src/processor.rs:2096`

4) BuiltinTransform coverage is minimal compared to OCIO registry.
   - vfx-ocio supports only a small subset (ACES core, a few camera log->ACES, sRGB->XYZ).
   - Evidence (vfx builtin map): `crates/vfx-ocio/src/builtin_transforms.rs`
   - Evidence (OCIO builtins registry across cameras/displays): `_ref/OpenColorIO/src/OpenColorIO/transforms/builtins/*.cpp`
   - Impact: unknown builtin styles become no-op in processor.

5) FileTransform ccc_id is unused.
   - vfx-ocio FileTransform has `ccc_id` but processor does not use it.
   - Evidence (ccc_id field): `crates/vfx-ocio/src/transform.rs`
   - Evidence (processor FileTransform match): `crates/vfx-ocio/src/processor.rs:963`
   - Impact: cannot select specific CC/CCC/CDL entries by ID.

## OpenImageIO parity gaps (initial)

6) vfx-io format coverage is far smaller than OpenImageIO plugins.
   - OIIO has many imageio plugins (bmp, cineon, dicom, ffmpeg, fits, gif, ico, iff, jpeg2000, jpegxl, openvdb, pnm, ptex, raw, r3d, rla, sgi, softimage, targa, term, zfile, etc.).
   - Evidence (plugin dirs): `_ref/OpenImageIO/src/*.imageio`
   - vfx-io Format enum/detect only includes exr/png/jpeg/tiff/dpx/hdr/heif/webp/avif/jp2/arri/redcode.
   - Evidence (format detection): `crates/vfx-io/src/detect.rs`

7) Several vfx-io formats are feature-gated or stubbed.
   - arriraw/redcode return UnsupportedFeature; heif/webp/jp2 gated by features; avif read requires dav1d; jp2 write not supported.
   - Evidence (dispatch): `crates/vfx-io/src/lib.rs`

8) ImageBuf spec reading is incomplete (assumes RGBA, ignores full metadata for most formats).
   - vfx-io ImageBuf `ensure_spec_read` uses probe_dimensions and hard-codes `nchannels=4` and `full_*` for all formats, only tries EXR headers for subimages.
   - Evidence (vfx-io behavior): `crates/vfx-io/src/imagebuf/mod.rs:1480`
   - OIIO ImageBuf reads full ImageSpec via ImageInput and does not assume RGBA.
   - Evidence (OIIO ImageBuf design): `_ref/OpenImageIO/src/include/OpenImageIO/imagebuf.h:66`

9) vfx-io read/write APIs lack OIIO-style subimage/miplevel/scanline/tile and deep access.
   - vfx-io FormatReader/Writer only expose whole-image read/write from path or memory.
   - Evidence (traits): `crates/vfx-io/src/traits.rs`
   - OIIO ImageInput supports subimage/miplevel selection, scanline/tile reads, and deep reads.
   - Evidence (OIIO API): `_ref/OpenImageIO/src/include/OpenImageIO/imageio.h:1166`

10) ImageBuf `read()` ignores subimage/miplevel parameters.
   - Method accepts subimage/miplevel, but `ensure_pixels_read` always calls `crate::read(&name)` with no subimage/miplevel selection.
   - Evidence (ImageBuf read path): `crates/vfx-io/src/imagebuf/mod.rs:760`

11) TextureSystem sampling falls back for trilinear/anisotropic in `sample()`.
   - `sample()` uses bilinear when filter is Trilinear/Anisotropic due to missing derivatives.
   - Evidence (implementation): `crates/vfx-io/src/texture.rs:110`
   - OIIO TextureSystem uses derivative-based LOD and anisotropy in texture() API.
   - Evidence (OIIO API): `_ref/OpenImageIO/src/include/OpenImageIO/texture.h:897`

12) No capability query API (supports/features) in vfx-io registry/traits.
   - OIIO ImageInput/ImageOutput expose `supports("...")` for metadata, multiimage, mipmap, ioproxy, thumbnails, etc.
   - Evidence (OIIO supports API): `_ref/OpenImageIO/src/include/OpenImageIO/imageio.h:1131` and `_ref/OpenImageIO/src/include/OpenImageIO/imageio.h:2545`
   - vfx-io FormatReader/FormatWriter/FormatInfo expose only format name, extensions, can_read, read/write paths.
   - Evidence (traits): `crates/vfx-io/src/traits.rs:128` and `crates/vfx-io/src/traits.rs:218`
   - Evidence (registry info): `crates/vfx-io/src/registry.rs:44`
   - Impact: callers cannot detect format capabilities (multiimage, mipmap, ioproxy, thumbnails, metadata limits).

13) Deep read API is not part of the unified vfx-io interfaces.
   - OIIO ImageInput exposes deep read entry points and reports `"deepdata"` support.
   - Evidence (deep reads): `_ref/OpenImageIO/src/include/OpenImageIO/imageio.h:1605`
   - Evidence (supports "deepdata"): `_ref/OpenImageIO/src/include/OpenImageIO/imageio.h:2496`
   - vfx-io deep is only exposed via EXR-specific functions, not in `FormatReader`/`FormatRegistry`.
   - Evidence (EXR deep entry points): `crates/vfx-io/src/exr.rs:888`
   - Impact: generic deep workflows cannot be implemented via `vfx-io::read`/registry; deep is format-specific only.

14) ImageCache ignores subimage/multiimage data despite exposing subimage in API.
   - `get_tile()` takes `subimage`, but `load_tile()` ignores it and full-loads via `crate::read(path)`.
   - Evidence (unused subimage parameter): `crates/vfx-io/src/cache.rs:405`
   - Evidence (full read path): `crates/vfx-io/src/cache.rs:435`
   - CachedImageInfo hard-codes `subimages: 1` even for files with multiple subimages.
   - Evidence (subimages fixed to 1): `crates/vfx-io/src/cache.rs:333`
   - Impact: multiimage/multipart files cannot be addressed correctly; cache API is misleading.

15) ImageCache streaming mode does not support mip levels.
   - `load_tile()` returns an UnsupportedOperation error if `mip_level > 0` in streaming mode.
   - Evidence (mip restriction): `crates/vfx-io/src/cache.rs:460`
   - Impact: `get_tile()` cannot serve mip tiles for large images that trigger streaming; callers must handle errors or get incorrect LOD behavior.

16) ImageBuf `contiguous()` always returns true (TODO left unresolved).
   - The method returns `true` unconditionally and has a TODO to check real storage layout.
   - Evidence: `crates/vfx-io/src/imagebuf/mod.rs:692`
   - Impact: callers may assume contiguous layout and perform invalid memcpy/stride math on non-contiguous buffers.

17) Streaming API reports source channel count, but Region data is always RGBA.
   - `Region` is defined as RGBA f32-only; `RGBA_CHANNELS` is fixed to 4.
   - Evidence (Region contract): `crates/vfx-io/src/streaming/traits.rs:46`
   - StreamingSource implementations return original channel count (e.g., EXR/Memory/TIFF).
   - Evidence (EXR channels): `crates/vfx-io/src/streaming/exr.rs:272`
   - Evidence (MemorySource channels): `crates/vfx-io/src/streaming/source.rs:235`
   - Impact: callers can misinterpret Region layout or allocate incorrect buffers based on `channels()`.

18) ImageCache streaming path can panic for images with >4 channels.
   - Streaming Region is RGBA, but `load_tile()` uses `channels` to index `rgba[c]`.
   - Evidence (loop indexes rgba by channels): `crates/vfx-io/src/cache.rs:455`
   - Evidence (Region is RGBA): `crates/vfx-io/src/streaming/traits.rs:46`
   - Impact: if `channels > 4` (AOVs, extra EXR channels), indexing past RGBA causes panic.

19) ExponentTransform ignores OCIO negativeStyle setting in config parsing.
   - OCIO allows setting NegativeStyle for ExponentTransform in configs > v1.
   - Evidence (OCIO API): `_ref/OpenColorIO/include/OpenColorIO/OpenColorTransforms.h:900`
   - vfx-ocio parser hard-codes `negative_style: NegativeStyle::Clamp` and never reads a YAML field.
   - Evidence (parser behavior): `crates/vfx-ocio/src/config.rs:664`
   - Impact: configs that rely on pass-through/mirror negative handling are parsed incorrectly.

20) ImageBuf read-only paths do not load pixels; const APIs can return zeroed data silently.
   - `ensure_pixels_read_ref()` always returns false and never loads pixels for read-only/cache-backed buffers.
   - Evidence (no-op read ref): `crates/vfx-io/src/imagebuf/mod.rs:1605`
   - `to_image_data()` ignores the boolean result and reads from `PixelStorage`, which is `Empty` by default and yields zeros.
   - Evidence (to_image_data ignores load): `crates/vfx-io/src/imagebuf/mod.rs:1407`
   - Evidence (Empty returns 0.0): `crates/vfx-io/src/imagebuf/storage.rs:198`
   - Impact: calling `write()` or `to_image_data()` on an ImageBuf that hasn't been mutably read yields blank output without error.

21) GradingRgbCurveTransform ignores direction in processor.
   - The transform has a direction field, but compile step always bakes forward curves.
   - Evidence (transform has direction): `crates/vfx-ocio/src/transform.rs:948`
   - Evidence (processor ignores direction): `crates/vfx-ocio/src/processor.rs:1136`
   - Impact: inverse grading curves are treated as forward, producing incorrect results.

22) LogCameraTransform linear slope can divide by zero without checks.
   - The linear slope formula divides by `ln(base) * (lin_side_break * lin_side_slope + lin_side_offset)`.
   - Evidence (no zero guard): `crates/vfx-ocio/src/processor.rs:1209`
   - Impact: configs with zero/near-zero denominator yield inf/NaN and break processing.

23) vfx-lut Lut1D cannot represent per-channel domain min/max from .cube.
   - .cube supports `DOMAIN_MIN`/`DOMAIN_MAX` with 3 values (per-channel), but Lut1D stores scalar `domain_min`/`domain_max`.
   - Evidence (scalar domain): `crates/vfx-lut/src/lut1d.rs:42`
   - Evidence (parser drops G/B): `crates/vfx-lut/src/cube.rs:96`
   - Impact: LUTs with non-uniform domain scaling are parsed incorrectly.

24) FileTransform uses CLF parser for .ctf files.
   - Processor treats `"ctf"` the same as `"clf"` and always calls `read_clf`.
   - Evidence (FileTransform branch): `crates/vfx-ocio/src/processor.rs:987`
   - vfx-lut provides a separate CTF parser (`read_ctf`).
   - Evidence (CTF API): `crates/vfx-lut/src/lib.rs:75`
   - Impact: valid .ctf files can fail to parse or be interpreted incorrectly.

25) ImageBuf write ignores `fileformat` hint and per-buffer write settings.
   - `write()` ignores `_fileformat` and does not apply `write_format`/`write_tiles`; it always converts to ImageData and calls `crate::write`.
   - Evidence (unused args and path): `crates/vfx-io/src/imagebuf/mod.rs:807`
   - Impact: callers cannot force output format or tiling through ImageBuf API despite setter methods.

26) .cube INPUT_RANGE directives are ignored.
   - Resolve-style .cube uses `LUT_1D_INPUT_RANGE` / `LUT_3D_INPUT_RANGE` to define domain.
   - Evidence (reference file): `_ref/OpenColorIO/tests/data/files/resolve_1d3d.cube:1`
   - vfx-lut .cube parser only handles `DOMAIN_MIN` / `DOMAIN_MAX` and does not parse INPUT_RANGE.
   - Evidence (parser keywords): `crates/vfx-lut/src/cube.rs:69`
   - Impact: input domain defaults to 0..1 even when file defines a different range.

27) .cube files containing both 1D and 3D LUTs are not supported.
   - Reference file includes both `LUT_1D_SIZE` and `LUT_3D_SIZE` in one file.
   - Evidence (reference file): `_ref/OpenColorIO/tests/data/files/resolve_1d3d.cube:1`
   - vfx-lut parse_3d errors if data length != size^3 (will include 1D lines) and parse_1d errors if `LUT_3D_SIZE` is present.
   - Evidence (errors): `crates/vfx-lut/src/cube.rs:74` and `crates/vfx-lut/src/cube.rs:84`
   - Impact: valid Resolve .cube files cannot be read via vfx-ocio FileTransform.

28) TextureSystem assumes fixed tile size 64 even if cache tile size is changed.
   - Texture sampling uses `DEFAULT_TILE_SIZE` to compute tile indices and local offsets.
   - Evidence (fixed tile size): `crates/vfx-io/src/texture.rs:274`
   - ImageCache allows changing tile size via `set_tile_size`, and uses it for tiling.
   - Evidence (configurable tile size): `crates/vfx-io/src/cache.rs:274`
   - Impact: if cache tile size != 64, TextureSystem will fetch wrong tiles/pixels.

29) Environment light-probe mapping can divide by zero.
   - LightProbe projection computes `r = sqrt(2 * (1 + z))` and divides by `r` with no zero guard.
   - Evidence (no guard): `crates/vfx-io/src/texture.rs:571`
   - Impact: direction (0,0,-1) yields r=0, causing inf/NaN texture coordinates.

30) vfx-lut .cube parser does not implement Resolve-style files with both 1D/3D and INPUT_RANGE.
   - OCIO Resolve cube format supports LUT_1D_SIZE/LUT_3D_SIZE in the same file and parses LUT_1D_INPUT_RANGE/LUT_3D_INPUT_RANGE.
   - Evidence (OCIO parser behavior): `_ref/OpenColorIO/src/OpenColorIO/fileformats/FileFormatResolveCube.cpp:339`
   - vfx-lut only supports DOMAIN_MIN/MAX and does not split 1D/3D blocks or parse INPUT_RANGE.
   - Evidence (vfx-lut keywords/logic): `crates/vfx-lut/src/cube.rs:69` and `crates/vfx-lut/src/cube.rs:124`
   - Impact: Resolve `.cube` files accepted by OCIO will be rejected or misinterpreted.

31) ImageBuf ignores config spec hints on open.
   - `from_file_opts` accepts a config `ImageSpec` but the parameter is unused; all reads ignore config hints.
   - Evidence (unused _config): `crates/vfx-io/src/imagebuf/mod.rs:355`
   - Impact: callers cannot pass per-format read hints that OIIO supports via ImageSpec config.

32) CDLTransform style/negative handling is ignored; clamping is always applied.
   - OCIO CDL style controls negative handling (default NO_CLAMP), and supports style selection.
   - Evidence (OCIO CDLStyle): `_ref/OpenColorIO/include/OpenColorIO/OpenColorTransforms.h:277`
   - vfx-ocio parser hard-codes `style: CdlStyle::AscCdl` and does not parse `style`.
   - Evidence (parser behavior): `crates/vfx-ocio/src/config.rs:671`
   - Processor clamps negatives via `max(0.0)` regardless of style.
   - Evidence (processor clamp): `crates/vfx-ocio/src/processor.rs:1490`
   - Impact: configs expecting NO_CLAMP or other CDL styles produce incorrect results.

33) processor_with_context does not resolve $VAR in FileTransform paths.
   - Method comment notes full implementation would resolve `$VAR`, but it only sets Processor context.
   - Evidence (comment + behavior): `crates/vfx-ocio/src/config.rs:1167`
   - Processor stores context but does not use it when applying ops.
   - Evidence (context unused): `crates/vfx-ocio/src/processor.rs:666`
   - Impact: context variables do not affect FileTransform path resolution after processor creation.

34) Viewing rules are parsed but never applied to filter views.
   - OCIO implements ViewingRules and uses them to filter views (see tests).
   - Evidence (OCIO viewing rules tests): `_ref/OpenColorIO/tests/cpu/ViewingRules_tests.cpp:286`
   - vfx-ocio only stores viewing_rules and exposes accessors; no filtering logic is used in display processor.
   - Evidence (only accessors, no usage): `crates/vfx-ocio/src/config.rs:2055`
   - Impact: configs relying on viewing_rules to select views behave differently.

35) Discreet1DL parser does not skip comments and is case-sensitive for the LUT header.
   - Parser only skips empty lines and checks `starts_with("LUT:")`, so `#` comments and `lut:` headers are treated as data.
   - Evidence (no comment handling): `crates/vfx-lut/src/discreet1dl.rs:158`
   - Evidence (case-sensitive header): `crates/vfx-lut/src/discreet1dl.rs:163`
   - OCIO skips comments and matches `lut:` case-insensitively.
   - Evidence (comment skipping): `_ref/OpenColorIO/src/OpenColorIO/fileformats/FileFormatDiscreet1DL.cpp:341`
   - Evidence (lower-case header check): `_ref/OpenColorIO/src/OpenColorIO/fileformats/FileFormatDiscreet1DL.cpp:420`
   - Impact: valid .1dl/.lut files with comments or lowercase headers fail to parse.

36) Discreet1DL dstDepth parsing does not accept Smoke's `65536f` token and ignores filename-based depth hints.
   - Parser only accepts `8/10/12/16/16f/32f` tokens; `65536f` is rejected.
   - Evidence (supported tokens): `crates/vfx-lut/src/discreet1dl.rs:81`
   - OCIO explicitly notes `65536f` usage for 16f output.
   - Evidence (65536f note): `_ref/OpenColorIO/src/OpenColorIO/fileformats/FileFormatDiscreet1DL.cpp:443`
   - OCIO also infers target depth from filename; vfx-lut does not.
   - Evidence (OCIO filename depth): `_ref/OpenColorIO/src/OpenColorIO/fileformats/FileFormatDiscreet1DL.cpp:493`
   - Impact: certain 16f exports and name-based depth hints are not honored.

37) Pandora parser ignores the declared `values:` order and does not validate `in:` against the data size.
   - `values:` only toggles LUT parsing and does not verify `red green blue`.
   - Evidence (no validation): `crates/vfx-lut/src/pandora.rs:84`
   - `in:` is parsed but not used to validate cube size; size is derived from data count.
   - Evidence (in tag parsed): `crates/vfx-lut/src/pandora.rs:58`
   - Evidence (size from data): `crates/vfx-lut/src/pandora.rs:124`
   - OCIO enforces `values: red green blue` and uses `in` to compute edge length.
   - Evidence (values check): `_ref/OpenColorIO/src/OpenColorIO/fileformats/FileFormatPandora.cpp:188`
   - Evidence (edge len from in): `_ref/OpenColorIO/src/OpenColorIO/fileformats/FileFormatPandora.cpp:160`
   - Impact: files with mismatched `values` order or inconsistent `in` can be accepted and misinterpreted.

38) Nuke VF parser applies global_transform before knowing grid_size, making parsing order-dependent.
   - Matrix unscale multiplies by size values at parse time; if `global_transform` appears before `grid_size`, size is 0.
   - Evidence (unscale uses size immediately): `crates/vfx-lut/src/nuke_vf.rs:115`
   - OCIO unscales after parsing and uses the final grid size, so order does not matter.
   - Evidence (OCIO unscale stage): `_ref/OpenColorIO/src/OpenColorIO/fileformats/FileFormatVF.cpp:222`
   - Impact: valid .vf files with `global_transform` before `grid_size` yield a zeroed matrix.

39) SpiMtx parser silently drops non-float tokens instead of erroring.
   - Parsing uses `filter_map(|s| s.parse().ok())`, so invalid tokens are skipped.
   - Evidence (silent drop): `crates/vfx-lut/src/spi_mtx.rs:246`
   - OCIO treats any non-float token as a parse error.
   - Evidence (strict float conversion): `_ref/OpenColorIO/src/OpenColorIO/fileformats/FileFormatSpiMtx.cpp:101`
   - Impact: malformed files can be partially parsed and yield incorrect matrices.

40) JPEG CMYK conversion assumes non-inverted CMYK and yields wrong RGB for typical JPEG CMYK files.
   - CMYK is converted using `(1 - C) * (1 - K)` style, which assumes C/M/Y/K are not inverted.
   - Evidence (conversion math): `crates/vfx-io/src/jpeg.rs:209`
   - OIIO notes JPEG CMYK is stored as 1-x and uses raw values directly (R = C*K), implying inverted storage.
   - Evidence (OIIO CMYK note + math): `_ref/OpenImageIO/src/jpeg.imageio/jpeginput.cpp:542`
   - Impact: CMYK JPEGs (common Adobe/print pipeline) are decoded with incorrect colors.

41) TIFF writer advertises 32-bit float but writes 16-bit integers instead.
   - `write_f32` explicitly converts floats to u16 as a fallback.
   - Evidence (fallback): `crates/vfx-io/src/tiff.rs:563`
   - Docs claim 32-bit float output support.
   - Evidence (doc claim): `crates/vfx-io/src/tiff.rs:6`
   - Impact: HDR/linear data is quantized and written as 16-bit, not 32f.

42) TIFF CMYK support is documented but not implemented for read/write paths.
   - Docs claim CMYK support.
   - Evidence (doc claim): `crates/vfx-io/src/tiff.rs:10`
   - Reader only handles Gray/RGB/RGBA and errors otherwise.
   - Evidence (read match): `crates/vfx-io/src/tiff.rs:252`
   - Writer only accepts channel counts 1/3/4 (no CMYK).
   - Evidence (write_u8/write_u16 channel match): `crates/vfx-io/src/tiff.rs:455`
   - Impact: CMYK TIFFs fail to decode or cannot be written despite stated support.

43) WebP writer options are ignored; encoding is always lossless.
   - Writer constructs `WebPEncoder::new_lossless` and then discards the provided options.
   - Evidence (lossless encoder + unused options): `crates/vfx-io/src/webp.rs:70`
   - Module docs claim lossy and lossless support with quality control.
   - Evidence (doc claim): `crates/vfx-io/src/webp.rs:5`
   - Impact: callers cannot produce lossy WebP or control quality despite API options.

44) DPX reader ignores RGBA/ABGR descriptors and always reads 3 channels.
   - Header maps descriptor 51/52 to 4 channels, but all read paths decode 3 samples per pixel.
   - Evidence (descriptor->channels): `crates/vfx-io/src/dpx.rs:532`
   - Evidence (read_8bit uses pixel_count * 3): `crates/vfx-io/src/dpx.rs:1088`
   - Evidence (read_10bit/12bit/16bit read 3 samples per pixel): `crates/vfx-io/src/dpx.rs:1094`
   - Impact: RGBA/ABGR DPX files are decoded with missing/shifted channels.

45) DPX 10-bit packing method B (filled/LSB) is not implemented.
   - Code comments state packing 2 is “filled method B (LSB justified)”, but the implementation treats packing 1 and 2 identically.
   - Evidence (packing comment + match): `crates/vfx-io/src/dpx.rs:1094`
   - Impact: files using packing method 2 will decode incorrectly.

46) DPX writer drops alpha even when input has 4 channels.
   - Writer enforces `channels >= 3` and always writes RGB from the first three components.
   - Evidence (channel check + RGB-only write path): `crates/vfx-io/src/dpx.rs:804`
   - Impact: alpha is silently discarded for RGBA images.

47) KTX2 module claims BC-compressed decode and metadata parsing but does not implement either.
   - Docs list BC1-BC7 decode support via image_dds, but `read_from_memory` returns UnsupportedFeature for BC formats.
   - Evidence (doc claim): `crates/vfx-io/src/ktx.rs:12`
   - Evidence (BC formats unsupported): `crates/vfx-io/src/ktx.rs:325`
   - KTX2 metadata parsing is marked “not yet implemented” and always returns empty metadata.
   - Evidence (metadata stub): `crates/vfx-io/src/ktx.rs:284`
   - Impact: stated capabilities do not match runtime behavior; metadata is unavailable.

48) HEIF docs mention Gain Map support, but implementation only extracts NCLX metadata.
   - Documentation lists “Gain Map” as supported HDR feature.
   - Evidence (doc claim): `crates/vfx-io/src/heif.rs:38`
   - Implementation only parses NCLX color profile; no gain map handling exists.
   - Evidence (NCLX-only path): `crates/vfx-io/src/heif.rs:222`
   - Impact: Gain Map HDR workflows are not actually supported despite documentation.

49) vfx-icc `Profile::lab()` returns an XYZ profile instead of Lab.
   - The function claims to create a Lab profile but uses `LcmsProfile::new_xyz()`.
   - Evidence (implementation): `crates/vfx-icc/src/profile.rs:173`
   - Impact: callers requesting Lab get XYZ, causing incorrect color conversions.

50) ACES2 OutputTransform ignores HDR display primaries and always uses sRGB matrices.
   - DisplayType::Hdr* is documented as Rec.2020 + PQ, but initialization uses sRGB matrices unconditionally.
   - Evidence (uses sRGB matrix for limit_jmh and output): `crates/vfx-color/src/aces2/transform.rs:63`
   - Evidence (ap1_to_srgb matrix for all display types): `crates/vfx-color/src/aces2/transform.rs:101`
   - Impact: HDR outputs use sRGB primaries instead of Rec.2020, producing wrong gamut for HDR displays.

51) BitDepth::is_integer returns true for Unknown despite documentation.
   - Doc comment says “Returns false for Unknown.”
   - Evidence (doc): `crates/vfx-core/src/format.rs:74`
   - Implementation returns `!self.is_float()`, so Unknown => true.
   - Evidence (implementation): `crates/vfx-core/src/format.rs:89`
   - Impact: callers treating Unknown as non-integer may get incorrect branching.

52) OCIO config validation claims to check missing LUT files but does not inspect FileTransform paths.
   - Module docs list “Missing LUT files,” yet `check_files` only verifies search paths exist.
   - Evidence (doc claim): `crates/vfx-ocio/src/validate.rs:6`
   - Evidence (implementation comment/behavior): `crates/vfx-ocio/src/validate.rs:214`
   - Impact: configs with missing LUTs will pass validation, masking runtime errors.

53) vfx-primaries silently falls back to identity/zero on invalid primaries instead of surfacing an error.
   - `xy_to_xyz` returns `Vec3::ZERO` when `y` is near zero.
   - Evidence (silent zero): `crates/vfx-primaries/src/lib.rs:364`
   - `rgb_to_xyz_matrix` and `xyz_to_rgb_matrix` use `unwrap_or(Mat3::IDENTITY)` on failed inversion.
   - Evidence (identity fallback): `crates/vfx-primaries/src/lib.rs:408`
   - Evidence (identity fallback): `crates/vfx-primaries/src/lib.rs:433`
   - Impact: invalid or degenerate primaries produce plausible-but-wrong matrices without diagnostics.

54) vfx-view applies exposure scaling to alpha, altering transparency.
   - Exposure multiplier is applied to every channel before conversion, including alpha.
   - Evidence (exposure loop): `crates/vfx-view/src/handler.rs:430`
   - Alpha is later read from the same pixel buffer for display.
   - Evidence (alpha read): `crates/vfx-view/src/handler.rs:492`
   - Impact: transparency changes when adjusting exposure, which is incorrect for premultiplied or straight alpha workflows.

55) vfx-compute streaming APIs claim out-of-core I/O, but EXR streaming loads the full file into memory.
   - Module docs and backend README describe streaming for images larger than RAM.
   - Evidence (streaming doc): `crates/vfx-compute/src/backend/streaming.rs:1`
   - Evidence (README claim): `crates/vfx-compute/src/backend/README.md:3`
   - EXR streaming source uses `vfx_io::read` and stores full `Vec<f32>`, with TODO noting true streaming is unimplemented.
   - Evidence (vfx_io::read + TODO): `crates/vfx-compute/src/backend/streaming.rs:196`
   - Evidence (full read): `crates/vfx-compute/src/backend/streaming.rs:206`
   - Impact: “streaming” paths can still OOM on large EXRs; docs overstate capability.

56) vfx-cli color/grade/premult operations apply exposure/gamma/saturation to alpha channel.
   - CLI `color` multiplies all channels for exposure and gamma; `apply_saturation` leaves alpha untouched but exposure/gamma do not.
   - Evidence (exposure/gamma over full data): `crates/vfx-cli/src/commands/color.rs:43`
   - `grade` applies slope/offset/power and saturation to the first three channels only, but iterates over full pixel chunks and leaves alpha unchanged; ok.
   - `premult` modifies RGB by alpha, ok.
   - Impact: `color` exposure/gamma alter alpha, which is incorrect for straight or premultiplied alpha workflows.

57) vfx-cli `color` uses misleading transfer labels: `rec709` path decodes to linear, but there is no explicit encode path (nor BT.1886 EOTF).
   - `transfer=rec709` invokes `rec709_to_linear` only.
   - Evidence: `crates/vfx-cli/src/commands/color.rs:70`
   - Impact: users expecting a symmetric encode/decode or display EOTF get a decode-only transform.

58) vfx-rs-py README advertises zero-copy numpy interop, but implementation always copies.
   - README claims `arr = img.numpy()` is zero-copy.
   - Evidence (README): `crates/vfx-rs-py/README.md:28`
   - Implementation uses `to_vec()` for both “copy” and non-copy paths, so always allocates.
   - Evidence (implementation comment + to_vec): `crates/vfx-rs-py/src/image.rs:59`
   - Impact: performance/memory expectations are incorrect; large images will copy.

59) vfx-rs-py `Image` constructor docs claim multiple dtypes, but signature only accepts float32 arrays.
   - Docstring lists float16/uint16/uint8 support, but `new` takes `PyArray3<f32>`.
   - Evidence (doc claim): `crates/vfx-rs-py/src/image.rs:36`
   - Evidence (signature): `crates/vfx-rs-py/src/image.rs:46`
   - Impact: non-f32 numpy arrays fail to construct despite documentation.

60) vfx-rs-py `read()` doc claims AVIF support, but vfx-io rejects AVIF reads.
   - `read` docstring lists AVIF among supported formats.
   - Evidence (doc claim): `crates/vfx-rs-py/src/lib.rs:20`
   - vfx-io read path returns UnsupportedFormat for AVIF (write-only).
   - Evidence (read behavior): `crates/vfx-io/src/lib.rs:244`
   - Impact: Python users get errors when reading AVIF despite documentation.

61) CLI docs list transfer functions that are not implemented in `vfx color`.
   - Docs claim `srgb-inv`, `pq`, `pq-inv`, `hlg`, `hlg-inv`, `log`, `log-inv` are supported.
   - Evidence (doc claim): `docs/src/cli/color.md:22`
   - Implementation only handles `srgb`, `linear_to_srgb`, and `rec709` (decode only).
   - Evidence (implementation): `crates/vfx-cli/src/commands/color.rs:70`
   - Impact: documented CLI options silently do nothing or are unavailable.

62) CLI LUT docs claim .clf/.spi1d/.spi3d/.3dl support, but CLI only loads .cube.
   - Docs list multiple LUT formats.
   - Evidence (doc claim): `docs/src/cli/lut.md:15`
   - Implementation only branches on `.cube` and errors otherwise.
   - Evidence (implementation): `crates/vfx-cli/src/commands/lut.rs:15`
   - Impact: users will get “Unsupported LUT format” for documented formats.

63) User guide lists formats (PFM, TX, BMP, TGA, PSD) not supported by format detection/IO.
   - Quick Start table claims read/write support for PFM/TX/BMP/TGA and PSD read.
   - Evidence (doc claim): `docs/src/user-guide/quick-start.md:73`
   - `Format` enum and extension detection only include EXR/PNG/JPEG/TIFF/DPX/HDR/HEIF/WebP/AVIF/JP2/ARRI/RED.
   - Evidence (format list): `crates/vfx-io/src/detect.rs:9`
   - Impact: documented formats will fail `vfx_io::read`/`vfx_io::write` and CLI commands.

64) User guide shows `vfx color --from/--to` color space conversion, but CLI color implementation ignores these flags.
   - Docs show `--from ACEScg --to sRGB` usage.
   - Evidence (doc claim): `docs/src/user-guide/quick-start.md:33`
   - `color` command does not reference `args.from` or `args.to`.
   - Evidence (implementation): `crates/vfx-cli/src/commands/color.rs:13`
   - Impact: advertised color space conversion is a no-op.

65) CLI ACES docs list RRT variants `alt1`/`filmic`, but implementation only supports `default` and `high-contrast`.
   - Docs list `alt1` and `filmic` variants.
   - Evidence (doc claim): `docs/src/cli/aces.md:38`
   - Implementation maps only `high-contrast` and defaults otherwise.
   - Evidence (implementation): `crates/vfx-cli/src/commands/aces.rs:97`
   - Impact: documented variants are ignored and fall back to default.

66) CLI maketx docs claim `.tx` output and embedded mipmaps, but implementation only writes the original image.
   - Docs describe `.tx` tiled EXR with mipmap chain.
   - Evidence (doc claim): `docs/src/cli/maketx.md:1`
   - Implementation generates mipmaps in memory but then writes the original `image` only.
   - Evidence (implementation): `crates/vfx-cli/src/commands/maketx.rs:63`
   - Impact: users don’t get `.tx` outputs or embedded mipmaps as documented.

67) CLI maketx ignores `--tile` and `--wrap` options.
   - Arguments are parsed and logged but never used to affect output.
   - Evidence (options only used for printing): `crates/vfx-cli/src/commands/maketx.rs:18`
   - Impact: user-specified tiling/wrap behavior has no effect.

68) Python docs use non-existent API names (`Image.from_numpy`, `Image.to_numpy`).
   - Docs show `Image.from_numpy` and `to_numpy()` usage.
   - Evidence (doc claim): `docs/src/crates/python.md:186`
   - Bindings expose `Image.__new__` and `Image.numpy()` instead.
   - Evidence (implementation): `crates/vfx-rs-py/src/image.rs:45`
   - Impact: docs are misleading; copy/paste examples will fail.

69) Python docs mention `read_layers`/`write_layers` functions that are not in the bindings.
   - Docs show `vfx_rs.read_layers` and `vfx_rs.write_layers` examples.
   - Evidence (doc claim): `docs/src/crates/python.md:214`
   - Bindings expose `read_layered` and no `write_layers` function.
   - Evidence (implementation): `crates/vfx-rs-py/src/lib.rs:34`
   - Impact: documented API does not exist, breaking examples.

70) CLI batch docs list operations/arguments and CLI shape not implemented in the command.
   - Docs show positional `<PATTERN>` instead of required `-i/--input` flag.
   - Evidence (doc claim): `docs/src/cli/batch.md:7`
   - Implementation requires `--input` (no positional pattern).
   - Evidence (implementation): `crates/vfx-cli/src/main.rs:556`
   - Docs also claim `--op color` and `--args width/height/filter` for resize, plus convert depth/compression.
   - Evidence (doc claim): `docs/src/cli/batch.md:15`
   - Implementation supports `convert/resize/blur/flip_h/flip_v` only, and `resize` uses only `scale` with fixed Lanczos3; `color` is not handled.
   - Evidence (implementation): `crates/vfx-cli/src/commands/batch.rs:66`
   - Impact: documented batch syntax/operations/arguments are ignored or error.

71) CLI blur docs claim alpha preservation and separable gaussian, but implementation blurs all channels and uses full 2D convolution.
   - Docs: “Preserves alpha channel” and “Gaussian blur uses separable implementation”.
   - Evidence (doc claim): `docs/src/cli/blur.md:83`
   - Implementation passes all channels to `box_blur`/`convolve` and uses 2D `Kernel::gaussian` with `convolve`.
   - Evidence (implementation): `crates/vfx-cli/src/commands/blur.rs:33`
   - Evidence (kernel/convolve): `crates/vfx-ops/src/filter.rs:90`
   - Impact: alpha gets blurred; performance/behavior differs from docs.

72) CLI channel-extract docs claim arbitrary named channels (e.g., `N.x`), but implementation only supports R/G/B/A/Z and numeric indices.
   - Evidence (doc claim): `docs/src/cli/channel-extract.md:36`
   - Evidence (implementation): `crates/vfx-cli/src/commands/channels.rs:178`
   - Impact: documented channel names fail with “Unknown channel” errors.

73) CLI channel-shuffle docs state missing alpha defaults to 1 and bit depth is preserved, but implementation fills missing channels with 0 and converts to f32.
   - Evidence (doc claim): `docs/src/cli/channel-shuffle.md:108`
   - Evidence (implementation): `crates/vfx-cli/src/commands/channels.rs:94`
   - Impact: alpha may be zeroed and output precision may change.

74) CLI composite docs list many blend modes and GPU acceleration, but CLI only supports over/add/multiply/screen on CPU.
   - Evidence (doc claim): `docs/src/cli/composite.md:16`
   - Evidence (implementation): `crates/vfx-cli/src/commands/composite.rs:31`
   - Impact: documented modes (overlay, softlight, etc.) are unavailable; GPU claim is false for CLI.

75) CLI diff docs describe thresholded pixel counts and diff image semantics that don’t match implementation.
   - Docs say diff image is absolute per-channel error and alpha is max RGB; warn/fail counts are thresholded.
   - Evidence (doc claim): `docs/src/cli/diff.md:86`
   - Implementation scales diffs by 10 and clamps to 1.0, never writes an alpha max channel, and counts “pixels differ” using a fixed 1e-6 epsilon.
   - Evidence (implementation): `crates/vfx-cli/src/commands/diff.rs:64`
   - Impact: diff images and statistics differ from documented behavior.

76) CLI extract-layer docs claim default extraction of first layer, but implementation lists layers and exits when `--layer` is missing.
   - Evidence (doc claim): `docs/src/cli/extract-layer.md:35`
   - Evidence (implementation): `crates/vfx-cli/src/commands/layers.rs:118`
   - Impact: documented default behavior does not happen.

77) CLI layers docs describe `vfx layers list/extract/merge` subcommands, but CLI exposes separate top-level commands.
   - Evidence (doc claim): `docs/src/cli/layers.md:7`
   - Evidence (implementation): `crates/vfx-cli/src/main.rs:136`
   - Impact: documented command syntax fails.

78) CLI merge-layers docs claim `--names` is comma-separated, but CLI expects repeated `--names` values.
   - Evidence (doc claim): `docs/src/cli/merge-layers.md:16`
   - Evidence (implementation): `crates/vfx-cli/src/main.rs:621`
   - Impact: users supplying comma-separated names get a single layer name with commas.

79) CLI resize docs claim GPU acceleration, but implementation always uses CPU resampling.
   - Evidence (doc claim): `docs/src/cli/resize.md:42`
   - Evidence (implementation): `crates/vfx-cli/src/commands/resize.rs:50`
   - Impact: performance expectations are overstated; no GPU path in CLI.

80) CLI sharpen docs claim unsharp mask, but implementation applies a single convolution kernel.
   - Evidence (doc claim): `docs/src/cli/sharpen.md:45`
   - Evidence (implementation): `crates/vfx-cli/src/commands/sharpen.rs:21`
   - Impact: actual effect differs from documented unsharp-mask behavior.

81) CLI transform docs describe 90° rotation as counter-clockwise and “all EXR layers” support, but code rotates clockwise and operates on a single layer.
   - Evidence (doc claim): `docs/src/cli/transform.md:28`
   - Evidence (implementation): `crates/vfx-cli/src/commands/transform.rs:33`
   - Impact: rotation direction is inverted; multi-layer EXR handling is not as documented.

82) CLI warp docs show wave/ripple `k2` values below 1.0, but implementation clamps `k2` to >= 1.0.
   - Evidence (doc claim): `docs/src/cli/warp.md:68`
   - Evidence (implementation): `crates/vfx-cli/src/commands/warp.rs:33`
   - Impact: documented low-amplitude waves/ripples are impossible in CLI.

83) TODO (requested): investigate true streaming for scanline EXR (if feasible) instead of caching full image.
   - Current scanline path loads full image into `cached_image`.
   - Evidence (current behavior): `crates/vfx-io/src/streaming/exr.rs:170`
   - Impact: large scanline EXR still requires full RAM; streaming claim is limited.

84) TODO (requested): consider reusing distortion/warp implementations from `C:\projects\projects.rust\_done\stool-rs` as a reference/source if needed.

85) CLI grep docs claim regex and full metadata search, but implementation only does substring checks on filename, dimensions, and format.
   - Docs advertise regex and EXIF/EXR/custom metadata search.
   - Evidence (doc claim): `docs/src/cli/grep.md:10`
   - Implementation lowercases and `contains()` pattern for filename, size string, and format; no metadata or regex.
   - Evidence (implementation): `crates/vfx-cli/src/commands/grep.rs:11`
   - Impact: documented search behavior is not available.

86) vfx-cli crate docs list global options that do not exist and omit actual ones.
   - Docs include `-q/--quiet` and `--log <FILE>`, but CLI uses `-l/--log [PATH]`, no quiet flag, and includes `-j/--threads` + `--allow-non-color`.
   - Evidence (doc claim): `docs/src/crates/cli.md:32`
   - Evidence (implementation): `crates/vfx-cli/src/main.rs:135`
   - Impact: documentation shows invalid flags and misses real ones.

87) vfx-cli crate docs describe `layers` subcommand flags and `batch` templated output/--jobs that CLI does not implement.
   - Docs show `vfx layers ... --list/--extract/--merge` and `vfx batch "*.exr" --output "./{name}.png" --jobs 8`.
   - Evidence (doc claim): `docs/src/crates/cli.md:108`
   - Implementation exposes separate top-level `layers`, `extract-layer`, `merge-layers` commands and `batch` only supports `--input/--output-dir/--op/--args/--format` (no templating, no jobs).
   - Evidence (implementation): `crates/vfx-cli/src/main.rs:136`
   - Impact: documented CLI syntax fails.

88) vfx-cli crate docs mention `info --layers` and `view --layer`, but those flags do not exist in CLI args.
   - Evidence (doc claim): `docs/src/crates/cli.md:56`
   - Evidence (implementation): `crates/vfx-cli/src/main.rs:200`
   - Impact: examples fail with unknown flag.

89) vfx-io crate docs show `StreamReader/StreamWriter` APIs and claim true EXR streaming, but the API surface uses `StreamingSource` and scanline EXR falls back to full-image cache.
   - Docs show `StreamReader::open`/`StreamWriter` usage and streaming support table with EXR true streaming.
   - Evidence (doc claim): `docs/src/crates/io.md:184`
   - Actual module exports `StreamingSource`/`open_streaming`, and scanline EXR uses cached full image.
   - Evidence (implementation): `crates/vfx-io/src/streaming/mod.rs:170`
   - Evidence (scanline fallback): `crates/vfx-io/src/streaming/exr.rs:165`
   - Impact: crate docs do not match API/behavior; EXR streaming is limited.

90) vfx-compute docs mention builder APIs and workflow helpers that do not exist.
   - Docs show `ProcessorBuilder::prefer_gpu(true)`; API uses `backend(Backend::Wgpu)` and has no `prefer_gpu` method.
   - Evidence (doc claim): `docs/src/crates/compute.md:113`
   - Evidence (implementation): `crates/vfx-compute/src/processor.rs:312`
   - Docs show `ComputePipeline::builder().add(...).build()`; builder only configures processor/strategy, no `add`/op list.
   - Evidence (doc claim): `docs/src/crates/compute.md:150`
   - Evidence (implementation): `crates/vfx-compute/src/pipeline.rs:870`
   - Docs show `TileWorkflow::new(proc, 1024)` and `workflow.process(...)`; `TileWorkflow` is an enum, no constructor or `process` method.
   - Evidence (doc claim): `docs/src/crates/compute.md:171`
   - Evidence (implementation): `crates/vfx-compute/src/backend/tiling.rs:165`
   - Impact: documentation examples are not usable as written.

91) vfx-compute docs claim `Processor::auto()` uses image-size heuristics, but it only selects backend.
   - Evidence (doc claim): `docs/src/crates/compute.md:238`
   - Evidence (implementation): `crates/vfx-compute/src/processor.rs:430`
   - Impact: docs overstate behavior; size-based strategy is in `ComputePipeline`, not `Processor`.

92) vfx-ops docs reference APIs that don't exist or are named differently.
   - `Kernel::from_data` is documented, but the API is `Kernel::new`.
   - Evidence (doc claim): `docs/src/crates/ops.md:69`
   - Evidence (implementation): `crates/vfx-ops/src/filter.rs:33`
   - `rotate_90`/`rotate_270` are documented, but exposed functions are `rotate_90_cw`/`rotate_90_ccw` and `rotate_180` only.
   - Evidence (doc claim): `docs/src/crates/ops.md:116`
   - Evidence (implementation): `crates/vfx-ops/src/transform.rs:176`
   - `warp::barrel_distort`/`pincushion_distort` and `warp::st_map` are documented but not present; actual functions are `warp::barrel`/`warp::pincushion`.
   - Evidence (doc claim): `docs/src/crates/ops.md:144`
   - Evidence (implementation): `crates/vfx-ops/src/warp.rs:52`
   - Impact: docs include non-existent functions or wrong names.

93) vfx-color docs show APIs that don't exist or are named differently.
   - `ColorProcessor::srgb_to_linear` and `apply_srgb_to_linear` are referenced, but ColorProcessor exposes `apply`/`apply_batch` and Pipeline usage instead.
   - Evidence (doc claim): `docs/src/crates/color.md:43`
   - Evidence (implementation): `crates/vfx-color/src/processor.rs:127`
   - `Pipeline::apply_buffer` is documented, but Pipeline has only `apply`; buffered processing is on `ColorProcessor::apply_buffer`.
   - Evidence (doc claim): `docs/src/crates/color.md:79`
   - Evidence (implementation): `crates/vfx-color/src/pipeline.rs:228`
   - `Pipeline::lut_3d` is documented; actual method is `lut3d`.
   - Evidence (doc claim): `docs/src/crates/color.md:165`
   - Evidence (implementation): `crates/vfx-color/src/pipeline.rs:142`
   - Impact: docs/examples are not usable as written.

94) Dev guide for adding formats describes outdated module/trait structure in vfx-io.
   - Docs refer to `vfx-io/src/formats/*` and `ImageReader`/`ImageWriter` traits.
   - Evidence (doc claim): `docs/src/dev/adding-formats.md:20`
   - Actual crate uses `FormatReader`/`FormatWriter` in `vfx-io/src/traits.rs` and per-format modules at crate root (e.g., `tiff.rs`).
   - Evidence (implementation): `crates/vfx-io/src/traits.rs:1`
   - Impact: contributors following the guide will edit non-existent paths and traits.

95) Dev guide for adding ops describes module layout and APIs that don't exist.
   - Guide suggests creating `vfx-ops/src/sharpen.rs` with `unsharp_mask` and referencing `vfx_ops::sharpen`, but the crate implements sharpening via `filter::Kernel::sharpen` (no `sharpen` module).
   - Evidence (doc claim): `docs/src/dev/adding-ops.md:12`
   - Evidence (implementation): `crates/vfx-ops/src/filter.rs:124`
   - Example uses `Kernel::new(3, 3, data)` but actual signature is `Kernel::new(data, width, height)`.
   - Evidence (doc claim): `docs/src/dev/adding-ops.md:74`
   - Evidence (implementation): `crates/vfx-ops/src/filter.rs:33`
   - Impact: guide code won't compile; wrong module path/signature.

96) Architecture doc states workspace has 16 crates, but workspace lists 17 members.
   - Evidence (doc claim): `docs/src/architecture/README.md:1`
   - Evidence (implementation): `Cargo.toml:4`
   - Impact: architecture overview is outdated.

97) Crate graph omits an actual dependency: vfx-io depends on vfx-ocio.
   - Evidence (doc claim): `docs/src/architecture/crate-graph.md:72`
   - Evidence (implementation): `crates/vfx-io/Cargo.toml:66`
   - Impact: dependency diagram is inaccurate.

98) Data-flow doc describes `ImageBuffer` and `ImageData` fields that do not exist.
   - Evidence (doc claim): `docs/src/architecture/data-flow.md:10`
   - Actual `ImageData` uses `PixelData` and `PixelFormat` fields, not `ImageBuffer`.
   - Evidence (implementation): `crates/vfx-io/src/lib.rs:538`
   - Impact: structural description is outdated.

99) Data-flow doc claims `save_image_layer` preserves other layers, but CLI writes a single-layer output.
   - Evidence (doc claim): `docs/src/architecture/data-flow.md:142`
   - Evidence (implementation): `crates/vfx-cli/src/commands/mod.rs:74`
   - Impact: documented layer-preserving behavior does not happen.

100) Dev testing docs list vfx-tests folder layout and asset paths that don't exist in repo.
   - Docs show `crates/vfx-tests/tests/*` and `test/images/*`, `test/luts/*` layout.
   - Evidence (doc claim): `docs/src/crates/tests.md:12`
   - Actual vfx-tests has `src/` only, and `test/` contains different files (no images/ or luts/ subdirs).
   - Evidence (implementation): `crates/vfx-tests/src/lib.rs:1`
   - Impact: contributors following docs will target missing paths.

101) Dev benchmarks doc uses APIs that don't exist (`apply_srgb_to_linear`, `Lut3D::load`, public `apply_trilinear/apply_tetrahedral`).
   - Evidence (doc claim): `docs/src/dev/benchmarks.md:58`
   - Evidence (implementation): `crates/vfx-lut/src/lut3d.rs:175`
   - Evidence (implementation): `crates/vfx-color/src/processor.rs:127`
   - Impact: benchmark examples won't compile.

102) Architecture doc misattributes `ImageData` to vfx-core.
   - Docs claim vfx-core provides `ImageData` in the foundation layer.
   - Evidence (doc claim): `docs/src/architecture/README.md:44`
   - Actual `ImageData` is defined in vfx-io.
   - Evidence (implementation): `crates/vfx-io/src/lib.rs:537`
   - Impact: crate responsibilities are misstated.

103) Crate graph omits dependency of vfx-ops on vfx-color.
   - Evidence (doc claim): `docs/src/architecture/crate-graph.md:121`
   - Evidence (implementation): `crates/vfx-ops/Cargo.toml:16`
   - Impact: dependency diagram is incomplete.

104) Dev README describes test asset layout that does not exist.
   - Docs show `test/images` and `test/luts` directories.
   - Evidence (doc claim): `docs/src/dev/README.md:24`
   - Actual `test/` contains flat files and `assets/` directories, not `images/` or `luts/`.
   - Evidence (implementation): `test` directory layout
   - Impact: contributors looking for assets follow wrong paths.

105) Internals README claims each crate has a `tests/` directory and an `error.rs` file, which is not true for many crates.
   - Evidence (doc claim): `docs/src/internals/README.md:18`
   - Evidence (implementation): workspace crates like `crates/vfx-tests` and `crates/vfx-cli` have no `tests/` directory.
   - Impact: internal code organization guidance is inaccurate.

106) Internals README lists a fictional `all-formats` feature flag.
   - Evidence (doc claim): `docs/src/internals/README.md:109`
   - Evidence (implementation): `crates/vfx-io/Cargo.toml:9`
   - Impact: developers may enable non-existent features.

107) Programmer core API docs describe a non-existent `vfx_core::ImageData` API and channel classification types.
   - Docs reference `vfx_core::ImageData`, `ImageData::new/constant/from_f32`, `get_pixel`, `set_pixel`, `as_f32_slice`, plus `ChannelType` and `classify_channel`.
   - Evidence (doc claim): `docs/src/programmer/core-api.md:9`
   - Actual `ImageData` is in vfx-io, and vfx-core has no `ChannelType` or `classify_channel` symbols.
   - Evidence (implementation): `crates/vfx-io/src/lib.rs:537`
   - Impact: programmer guide examples are invalid.

108) Internals pipeline doc uses non-existent helper functions (`apply_srgb_eotf`, `apply_srgb_oetf`).
   - Evidence (doc claim): `docs/src/internals/pipeline.md:70`
   - Evidence (implementation): no matches in `crates/` for those symbols.
   - Impact: internal pipeline examples are misleading.

109) Programmer color-management docs use non-existent transfer and LUT APIs, and ACES helpers that aren't present.
   - Transfer examples use `linear_to_srgb`, `srgb_to_linear`, `linear_to_rec709`, `rec709_to_linear`, etc., but vfx-transfer exposes `srgb::oetf/eotf` and `rec709::oetf/eotf` (plus re-exports like `srgb_oetf`).
   - Evidence (doc claim): `docs/src/programmer/color-management.md:60`
   - Evidence (implementation): `crates/vfx-transfer/src/lib.rs:88`
   - ACES examples reference `apply_idt`/`apply_rrt_odt` and `srgb_to_acescg` returning `[f32;3]`, but vfx-color only exposes `apply_rrt_odt_srgb` and `srgb_to_acescg(r,g,b)` returning tuple.
   - Evidence (doc claim): `docs/src/programmer/color-management.md:109`
   - Evidence (implementation): `crates/vfx-color/src/aces.rs:203`
   - LUT examples use `Lut3D::from_file`, `Lut::from_file`, and `apply_lut`, which don't exist in vfx-lut.
   - Evidence (doc claim): `docs/src/programmer/color-management.md:152`
   - Evidence (implementation): `crates/vfx-lut/src/lib.rs:65`
   - Impact: programmer guide examples are not executable as written.

110) Programmer GPU compute docs use non-existent APIs and wrong filter names.
   - Docs show `Processor::new(Backend::Auto)` and `ComputeImage::data()` access, but API uses `Processor::auto()` and `ComputeImage::to_vec()`.
   - Evidence (doc claim): `docs/src/programmer/gpu-compute.md:20`
   - Evidence (implementation): `crates/vfx-compute/src/processor.rs:424`
   - Docs use `ResizeFilter::Lanczos` in `vfx_compute::ResizeFilter`, but enum variants are `Lanczos` in compute (ok) while doc also references `Lanczos` and method `resize` returns `ComputeImage`; check OK.
   - Docs show `apply_matrix` with 4x4 and describe as color matrix, but compute expects `[f32;16]` (matches). No issue.
   - Impact: examples that call `Processor::new(Backend::Auto)` and `img.data()` will not compile.

111) Programmer README uses non-existent vfx-core APIs for ImageData, ChannelType, and ImageSpec metadata setters.
   - Docs show `use vfx_core::ImageData;` and a ChannelType enum example, plus `ImageSpec::set_attribute`.
   - Evidence (doc claim): `docs/src/programmer/README.md:68`, `docs/src/programmer/README.md:95`, `docs/src/programmer/README.md:86`
   - Actual `ImageData` lives in vfx-io, and vfx-core has no `ChannelType` or `ImageSpec::set_attribute`.
   - Evidence (implementation): `crates/vfx-io/src/lib.rs:537`, `crates/vfx-core/src/spec.rs:358`
   - Impact: README examples will not compile.

112) OCIO integration docs reference Config loading APIs and built-in configs that do not exist in Rust.
   - Docs show `Config::from_env`, `Config::from_string`, and `builtin::aces_1_2()`.
   - Evidence (doc claim): `docs/src/programmer/ocio-integration.md:32`, `docs/src/programmer/ocio-integration.md:38`, `docs/src/programmer/ocio-integration.md:50`
   - Rust API only exposes `Config::from_file`, and builtin config list includes `aces_1_3()` and `srgb_studio()` but not `aces_1_2()`.
   - Evidence (implementation): `crates/vfx-ocio/src/config.rs:202`, `crates/vfx-ocio/src/builtin.rs:31`
   - Impact: Rust quick-start examples do not compile.

113) OCIO integration docs use processor apply APIs with wrong signatures and a non-existent batch helper.
   - Docs apply to `Vec<f32>` and call `apply_rgb_batch`.
   - Evidence (doc claim): `docs/src/programmer/ocio-integration.md:119`, `docs/src/programmer/ocio-integration.md:127`
   - Actual API expects `&mut [[f32; 3]]` / `&mut [[f32; 4]]` and has no `apply_rgb_batch`.
   - Evidence (implementation): `crates/vfx-ocio/src/processor.rs:1467`, `crates/vfx-ocio/src/processor.rs:1474`
   - Impact: examples do not compile and show incorrect usage.

114) OCIO dynamic processor builder example has incorrect `build` signature and pixel buffer type.
   - Docs use `build(&processor)?` and call `apply_rgba` on `Vec<f32>`.
   - Evidence (doc claim): `docs/src/programmer/ocio-integration.md:175`, `docs/src/programmer/ocio-integration.md:182`
   - Actual builder signature is `build(self, base: Processor) -> DynamicProcessor`, and `apply_rgba` expects `&mut [[f32; 4]]`.
   - Evidence (implementation): `crates/vfx-ocio/src/dynamic.rs:339`, `crates/vfx-ocio/src/dynamic.rs:233`
   - Impact: documented dynamic pipeline does not compile.

115) OCIO baker example uses non-existent methods and wrong write API.
   - Docs call `bake_1d`/`bake_3d` and `lut.write_cube(...)`.
   - Evidence (doc claim): `docs/src/programmer/ocio-integration.md:200`
   - Actual API uses `bake_lut_1d`/`bake_lut_3d` and `Baker::write_cube_1d/3d`.
   - Evidence (implementation): `crates/vfx-ocio/src/baker.rs:101`, `crates/vfx-ocio/src/baker.rs:217`
   - Impact: LUT export examples do not compile.

116) OCIO processor cache example uses a non-existent constructor and method name.
   - Docs show `ProcessorCache::new(config)` and `cache.get(...)`.
   - Evidence (doc claim): `docs/src/programmer/ocio-integration.md:215`, `docs/src/programmer/ocio-integration.md:218`
   - Actual API uses `ProcessorCache::new()` and `get_or_create(&config, ...)`.
   - Evidence (implementation): `crates/vfx-ocio/src/cache.rs:50`, `crates/vfx-ocio/src/cache.rs:59`
   - Impact: cache examples do not compile.

117) OCIO builtin transform styles list contains names that are not recognized by the builtin registry.
   - Docs list `ACES-AP0_to_XYZ-D65` and `ACES-AP1_to_XYZ-D65`.
   - Evidence (doc claim): `docs/src/programmer/ocio-integration.md:264`
   - Builtin registry only matches styles with the `...XYZ-D65-BFD` suffix (`acesap0toxyzd65bfd`, `acesap1toxyzd65bfd`).
   - Evidence (implementation): `crates/vfx-ocio/src/builtin_transforms.rs:253`
   - Impact: using documented style strings returns `None`.

118) OCIO transform support table overstates GPU support for FixedFunction and GradingRGBCurve.
   - Docs mark GPU support as "Partial" for both.
   - Evidence (doc claim): `docs/src/programmer/ocio-integration.md:246`, `docs/src/programmer/ocio-integration.md:249`
   - GPU backend returns `None` for these ops (not supported).
   - Evidence (implementation): `crates/vfx-ocio/src/gpu.rs:434`, `crates/vfx-ocio/src/gpu.rs:456`
   - Impact: GPU capability matrix is inaccurate.

119) ImageBufAlgo README examples use non-existent functions and incorrect signatures.
   - Docs call `add_constant`, `blur_inplace`, `resize(&mut ...)`, `flip_horizontal`, `computePixelStats`, and `isConstantColor` which do not exist; they also use incorrect `fill/checker/noise/crop/rotate/resize` signatures and string filter names.
   - Evidence (doc claim): `docs/src/programmer/imagebufalgo/README.md:33`, `docs/src/programmer/imagebufalgo/README.md:48`, `docs/src/programmer/imagebufalgo/README.md:63`, `docs/src/programmer/imagebufalgo/README.md:77`, `docs/src/programmer/imagebufalgo/README.md:83`, `docs/src/programmer/imagebufalgo/README.md:86`, `docs/src/programmer/imagebufalgo/README.md:132`
   - Actual APIs use `add(a, b, roi)`, `blur(src, sigma, roi)`, `resize(src, w, h, ResizeFilter, roi)`, `flip(src, roi)`, `compute_pixel_stats`, `is_constant_color`, `fill(values, roi)`, `checker(check_w, check_h, check_d, color1, color2, offset, roi)`, and `noise(NoiseType, a, b, mono, seed, roi)`.
   - Evidence (implementation): `crates/vfx-io/src/imagebufalgo/arithmetic.rs:86`, `crates/vfx-io/src/imagebufalgo/filters.rs:111`, `crates/vfx-io/src/imagebufalgo/geometry.rs:315`, `crates/vfx-io/src/imagebufalgo/geometry.rs:83`, `crates/vfx-io/src/imagebufalgo/stats.rs:89`, `crates/vfx-io/src/imagebufalgo/patterns.rs:59`, `crates/vfx-io/src/imagebufalgo/patterns.rs:216`
   - Docs label "Add blend" but call `imagebufalgo::add` (arithmetic add), while the compositing blend is `add_blend`.
   - Evidence (doc claim): `docs/src/programmer/imagebufalgo/README.md:108`
   - Evidence (implementation): `crates/vfx-io/src/imagebufalgo/composite.rs:323`
   - Impact: README gives multiple non-compiling examples and misrepresents API names.

120) Deep ImageBufAlgo docs reference missing functions and wrong deep I/O types.
   - Docs use `deep_flatten`, `deep_sample_count`, `deep_trim`, and `deep_holdout(&deep, &holdout)` plus `vfx_io::read` for deep data.
   - Evidence (doc claim): `docs/src/programmer/imagebufalgo/deep.md:38`, `docs/src/programmer/imagebufalgo/deep.md:97`, `docs/src/programmer/imagebufalgo/deep.md:106`, `docs/src/programmer/imagebufalgo/deep.md:57`
   - Actual APIs expose `flatten_deep(deep, width, height)` and `deep_holdout(deep, holdout_z)` and do not define `deep_sample_count` or `deep_trim`; deep reads use `exr::read_deep` to return `DeepData`.
   - Evidence (implementation): `crates/vfx-io/src/imagebufalgo/deep.rs:55`, `crates/vfx-io/src/imagebufalgo/deep.rs:365`, `crates/vfx-io/src/exr.rs:888`
   - Impact: deep workflow examples are not executable as written.

121) Filters docs use non-existent helper functions and wrong call signatures.
   - Docs call `blur_xy`, omit ROI arguments across filters, assume `Result` returns, and use wrong helper calls like `imagebufalgo::add(&image, &edges, 0.3)` and in-place `clamp(&mut bright, 0.8, 1000.0)`.
   - Evidence (doc claim): `docs/src/programmer/imagebufalgo/filters.md:16`, `docs/src/programmer/imagebufalgo/filters.md:148`, `docs/src/programmer/imagebufalgo/filters.md:171`
   - Actual API has `blur(src, sigma, roi)` with ROI, no `blur_xy`, `add` expects `(a, b, roi)`, and `clamp` is `clamp(src, min_vals, max_vals, roi)` returning `ImageBuf`.
   - Evidence (implementation): `crates/vfx-io/src/imagebufalgo/filters.rs:111`, `crates/vfx-io/src/imagebufalgo/arithmetic.rs:86`, `crates/vfx-io/src/imagebufalgo/arithmetic.rs:425`
   - Impact: filters documentation is not aligned with current API signatures.

122) Installation/build docs suggest passing format features to vfx-cli, but vfx-cli exposes only a `viewer` feature.
   - Docs show `cargo build -p vfx-cli --no-default-features --features exr,png,...`.
   - Evidence (doc claim): `docs/src/installation/building.md:44`
   - vfx-cli only defines the `viewer` feature; format features live in vfx-io and are not re-exposed by vfx-cli.
   - Evidence (implementation): `crates/vfx-cli/Cargo.toml:24`
   - Impact: documented build commands fail with “unknown feature”.

123) Feature flags doc misattributes EXR support to the `exr` crate and repeats invalid vfx-cli feature usage.
   - Docs say `exr` feature uses the `exr` crate and show `vfx-cli` builds with format features.
   - Evidence (doc claim): `docs/src/installation/features.md:9`, `docs/src/installation/features.md:37`
   - Actual EXR support depends on `vfx-exr`, and vfx-cli does not expose format features.
   - Evidence (implementation): `crates/vfx-io/Cargo.toml:15`, `crates/vfx-cli/Cargo.toml:24`
   - Impact: feature docs are misleading for build configuration.

124) Resize CLI docs claim GPU acceleration, but implementation is CPU-only.
   - Docs say resize uses GPU via wgpu with fallback to CPU.
   - Evidence (doc claim): `docs/src/cli/resize.md:49`
   - Implementation uses vfx-ops CPU resize only.
   - Evidence (implementation): `crates/vfx-cli/src/commands/resize.rs:11`
   - Impact: users expect GPU acceleration that does not occur.

125) Diff CLI docs misdescribe difference image and exit codes.
   - Docs say diff image shows absolute per-pixel error and list exit code 2 for errors.
   - Evidence (doc claim): `docs/src/cli/diff.md:54`, `docs/src/cli/diff.md:114`
   - Implementation scales diff image by 10.0 and does not assign a special error exit code (errors use standard failure).
   - Evidence (implementation): `crates/vfx-cli/src/commands/diff.rs:116`, `crates/vfx-cli/src/commands/diff.rs:58`
   - Impact: automated tooling relying on doc behavior gets incorrect output/exit codes.

126) Composite CLI docs list unsupported blend modes and omit that `--opacity` is ignored.
   - Docs list subtract/overlay/softlight/hardlight/difference modes.
   - Evidence (doc claim): `docs/src/cli/composite.md:25`
   - Implementation supports only over/add/multiply/screen.
   - Evidence (implementation): `crates/vfx-cli/src/commands/composite.rs:32`
   - `CompositeArgs` has `opacity`, but it is not used in compositing logic.
   - Evidence (implementation): `crates/vfx-cli/src/main.rs:394`
   - Impact: documented modes fail at runtime; opacity flag is misleading.

127) Sharpen CLI docs claim unsharp masking, but implementation uses a simple sharpen kernel.
   - Docs describe unsharp masking formula and algorithm.
   - Evidence (doc claim): `docs/src/cli/sharpen.md:3`
   - Implementation uses `Kernel::sharpen` + convolution (no unsharp mask step).
   - Evidence (implementation): `crates/vfx-cli/src/commands/sharpen.rs:25`
   - Impact: expected behavior differs from actual output.

128) Color CLI docs list unsupported transfer functions and short flags; also `--from/--to` are unused.
   - Docs show short flags `-e/-g/-s/-t` and transfer list including pq/hlg/log and srgb-inv.
   - Evidence (doc claim): `docs/src/cli/color.md:15`, `docs/src/cli/color.md:28`
   - Actual code only implements `srgb` (to linear), `linear_to_srgb`, and `rec709`; no pq/hlg/log handling.
   - Evidence (implementation): `crates/vfx-cli/src/commands/color.rs:90`
   - Args include `from`/`to`, but they are never read in the implementation.
   - Evidence (implementation): `crates/vfx-cli/src/main.rs:442`, `crates/vfx-cli/src/commands/color.rs:18`
   - Impact: documented transfer functions and flags do not work; color space conversion is not applied.

129) LUT CLI docs claim support for CLF/SPI/3DL, but the command only accepts .cube.
   - Docs list `.clf`, `.spi1d`, `.spi3d`, `.3dl` support.
   - Evidence (doc claim): `docs/src/cli/lut.md:24`
   - Implementation only handles `.cube` and rejects others.
   - Evidence (implementation): `crates/vfx-cli/src/commands/lut.rs:24`
   - Impact: advertised LUT formats fail.

130) maketx CLI docs claim `.tx` output and embedded mipmaps, but implementation saves the original image only.
   - Docs describe `.tx` tiled EXR output and embedded mipmaps.
   - Evidence (doc claim): `docs/src/cli/maketx.md:17`, `docs/src/cli/maketx.md:55`
   - Implementation generates mipmaps but writes only the original image and notes TX embedding is not implemented.
   - Evidence (implementation): `crates/vfx-cli/src/commands/maketx.rs:73`, `crates/vfx-cli/src/commands/maketx.rs:77`
   - Impact: maketx does not produce .tx or embedded mip chains as documented.

131) grep CLI docs claim regex and metadata search with exit codes, but implementation only does substring checks on filename/size/format.
   - Docs describe regex support, EXIF/EXR metadata search, and exit codes.
   - Evidence (doc claim): `docs/src/cli/grep.md:14`, `docs/src/cli/grep.md:73`
   - Implementation only checks filename, dimensions, and format strings; no regex or metadata; no exit code changes for “no matches”.
   - Evidence (implementation): `crates/vfx-cli/src/commands/grep.rs:31`, `crates/vfx-cli/src/commands/grep.rs:36`
   - Impact: grep is far more limited than documented.

132) batch CLI docs describe positional pattern and operations not implemented.
   - Docs use positional `<PATTERN>` and list ops resize/convert/color/blur with width/height/filter and color args.
   - Evidence (doc claim): `docs/src/cli/batch.md:8`, `docs/src/cli/batch.md:15`
   - Actual CLI requires `--input`, and supports only convert/resize/blur/flip_h/flip_v; resize uses `scale` only and blur ignores type.
   - Evidence (implementation): `crates/vfx-cli/src/main.rs:563`, `crates/vfx-cli/src/commands/batch.rs:117`
   - Impact: documented batch usage fails or silently ignores args.

133) layers CLI docs describe subcommands that do not exist.
   - Docs use `vfx layers list/extract/merge` subcommands.
   - Evidence (doc claim): `docs/src/cli/layers.md:8`
   - Implementation exposes separate commands `layers`, `extract-layer`, `merge-layers`.
   - Evidence (implementation): `crates/vfx-cli/src/main.rs:203`
   - Impact: documented commands fail.

134) merge-layers docs say `--names` is comma-separated and requires matching bit depths, but implementation expects repeated flags and only validates dimensions.
   - Docs: `--names beauty,diffuse` and “compatible bit depths”.
   - Evidence (doc claim): `docs/src/cli/merge-layers.md:16`, `docs/src/cli/merge-layers.md:107`
   - Implementation uses `Vec<String>` for names (repeat flag) and checks only width/height.
   - Evidence (implementation): `crates/vfx-cli/src/main.rs:622`, `crates/vfx-cli/src/commands/layers.rs:243`
   - Impact: docs mislead about naming and validation.

135) channel-extract docs claim comma-separated lists and custom channel names like `N.x`, but implementation only accepts R/G/B/A/Z or numeric indices.
   - Docs show comma-separated input and custom names.
   - Evidence (doc claim): `docs/src/cli/channel-extract.md:26`, `docs/src/cli/channel-extract.md:57`
   - Implementation parses each argument as a single spec and only maps R/G/B/A/Z/DEPTH or numeric indices.
   - Evidence (implementation): `crates/vfx-cli/src/commands/channels.rs:160`
   - Impact: documented channel specs are rejected.

136) channel-shuffle docs describe default alpha behavior and omit numeric channel selectors.
   - Docs say missing channels default to 0 except A defaults to 1.
   - Evidence (doc claim): `docs/src/cli/channel-shuffle.md:128`
   - Implementation defaults all missing channels (including A) to 0 and supports numeric channel indices in patterns.
   - Evidence (implementation): `crates/vfx-cli/src/commands/channels.rs:99`
   - Impact: doc behavior differs from actual output and available syntax.

137) view CLI docs require `<INPUT>`, but CLI accepts input as optional.
   - Evidence (doc claim): `docs/src/cli/view.md:8`
   - Evidence (implementation): `crates/vfx-cli/src/main.rs:756`
   - Impact: usage line is incorrect.

138) aces CLI docs reference `--rrt` and variants `alt1/filmic`, but CLI flag is `--rrt-variant` and only supports default/high-contrast.
   - Evidence (doc claim): `docs/src/cli/aces.md:16`, `docs/src/cli/aces.md:57`
   - Evidence (implementation): `crates/vfx-cli/src/main.rs:744`, `crates/vfx-cli/src/commands/aces.rs:78`
   - Impact: documented flags/variants fail.

139) Logging docs show GPU resize logs that do not occur in the current CLI implementation.
   - Docs show `vfx_ops::resize` GPU messages and backend selection in debug output.
   - Evidence (doc claim): `docs/src/logging.md:25`
   - Resize command uses CPU vfx-ops without GPU path.
   - Evidence (implementation): `crates/vfx-cli/src/commands/resize.rs:11`
   - Impact: debug logs in docs do not match actual output.

140) ACEScg guide suggests OCIO conversion via `vfx color --from/--to`, but color command ignores these options.
   - Evidence (doc claim): `docs/src/aces/acescg.md:137`
   - Evidence (implementation): `crates/vfx-cli/src/main.rs:442`, `crates/vfx-cli/src/commands/color.rs:18`
   - Impact: documented ACEScg conversion via CLI does nothing.

141) ACES examples rely on `vfx color --from/--to` conversions that are not implemented.
   - Evidence (doc claim): `docs/src/aces/examples.md:34`, `docs/src/aces/examples.md:56`
   - Evidence (implementation): `crates/vfx-cli/src/commands/color.rs:18`
   - Impact: example pipelines fail to apply OCIO conversions.

142) ACES examples use `vfx batch --op aces`, but batch supports only convert/resize/blur/flip operations.
   - Evidence (doc claim): `docs/src/aces/examples.md:152`
   - Evidence (implementation): `crates/vfx-cli/src/commands/batch.rs:117`
   - Impact: batch ACES examples fail.

143) ACES examples pass `--layer` to `vfx aces`, but the aces command has no layer option.
   - Evidence (doc claim): `docs/src/aces/examples.md:199`
   - Evidence (implementation): `crates/vfx-cli/src/main.rs:731`
   - Impact: example command fails.

144) Appendix format table claims EXR deep data unsupported, but deep read/write APIs exist.
   - Evidence (doc claim): `docs/src/appendix/formats.md:16`
   - Evidence (implementation): `crates/vfx-io/src/exr.rs:883`
   - Impact: documentation understates EXR deep capabilities.

145) Appendix EXR CLI examples use unsupported flags (`info --layers`, `convert --layer`).
   - Evidence (doc claim): `docs/src/appendix/formats.md:29`, `docs/src/appendix/formats.md:30`
   - Info command has only `--stats/--all/--json`, and convert has no `--layer`.
   - Evidence (implementation): `crates/vfx-cli/src/main.rs:252`, `crates/vfx-cli/src/main.rs:273`
   - Impact: CLI examples do not work.

146) Appendix lists `ocio` and vfx-lut feature flags that do not exist.
   - Docs list feature `ocio` and vfx-lut features (`cube`, `clf`, `spi`).
   - Evidence (doc claim): `docs/src/appendix/formats.md:206`, `docs/src/appendix/formats.md:251`
   - vfx-io has no `ocio` feature; vfx-lut has no feature flags.
   - Evidence (implementation): `crates/vfx-io/Cargo.toml:10`, `crates/vfx-lut/Cargo.toml:1`
   - Impact: feature guidance is incorrect.

147) Appendix “Format Detection” table lists .psd and LUT extensions as detectable formats, but format detection only handles image formats.
   - Evidence (doc claim): `docs/src/appendix/formats.md:230`
   - `Format::from_extension` does not include psd/cube/clf/spi extensions.
   - Evidence (implementation): `crates/vfx-io/src/detect.rs:52`
   - Impact: users will expect detection that does not exist.

148) CLI reference documents a global `-q/--quiet` flag that is not implemented.
   - Evidence (doc claim): `docs/src/appendix/cli-ref.md:9`
   - CLI global options only include verbose/log/threads/allow-non-color.
   - Evidence (implementation): `crates/vfx-cli/src/main.rs:130`, `crates/vfx-cli/src/main.rs:136`, `crates/vfx-cli/src/main.rs:140`, `crates/vfx-cli/src/main.rs:144`, `crates/vfx-cli/src/main.rs:148`
   - Impact: documented flag fails.

149) CLI reference lists `vfx info --layers/--channels`, but the info command only supports stats/all/json.
   - Evidence (doc claim): `docs/src/appendix/cli-ref.md:24`, `docs/src/appendix/cli-ref.md:25`
   - Evidence (implementation): `crates/vfx-cli/src/main.rs:252`, `crates/vfx-cli/src/main.rs:259`, `crates/vfx-cli/src/main.rs:263`, `crates/vfx-cli/src/main.rs:267`
   - Impact: documented options are rejected.

150) CLI reference shows `vfx convert -i/-o` and `--layer`, but convert uses positional input/output and has no layer option.
   - Evidence (doc claim): `docs/src/appendix/cli-ref.md:43`, `docs/src/appendix/cli-ref.md:48`, `docs/src/appendix/cli-ref.md:55`
   - Evidence (implementation): `crates/vfx-cli/src/main.rs:271`
   - Impact: documented flags and examples fail.

151) CLI reference for resize uses `-h` for height and lists bicubic/lanczos3 filters, but CLI uses `-H` for height and only supports box/bilinear/lanczos/mitchell.
   - Evidence (doc claim): `docs/src/appendix/cli-ref.md:69`, `docs/src/appendix/cli-ref.md:79`, `docs/src/appendix/cli-ref.md:80`, `docs/src/appendix/cli-ref.md:87`
   - Evidence (implementation): `crates/vfx-cli/src/main.rs:292`, `crates/vfx-cli/src/main.rs:305`, `crates/vfx-cli/src/main.rs:312`
   - Impact: documented flags/filters are wrong.

152) CLI reference for color uses short flags (`-e/-g/-s/-t`) and `-i/--input`, but CLI defines only long flags and positional input.
   - Evidence (doc claim): `docs/src/appendix/cli-ref.md:100`, `docs/src/appendix/cli-ref.md:101`, `docs/src/appendix/cli-ref.md:102`
   - Evidence (implementation): `crates/vfx-cli/src/main.rs:438`, `crates/vfx-cli/src/main.rs:456`, `crates/vfx-cli/src/main.rs:460`, `crates/vfx-cli/src/main.rs:464`, `crates/vfx-cli/src/main.rs:468`
   - Impact: documented flags fail.

153) CLI reference says blur default is box, but CLI default blur type is gaussian.
   - Evidence (doc claim): `docs/src/appendix/cli-ref.md:132`
   - Evidence (implementation): `crates/vfx-cli/src/main.rs:411`
   - Impact: users get different results than documented.

154) CLI reference claims LUT supports `.spi1d/.spi3d` and `--interpolation`, but CLI only accepts `.cube/.clf` and has no interpolation or layer options.
   - Evidence (doc claim): `docs/src/appendix/cli-ref.md:178`, `docs/src/appendix/cli-ref.md:179`, `docs/src/appendix/cli-ref.md:186`
   - Evidence (implementation): `crates/vfx-cli/src/main.rs:484`
   - Impact: documented formats/options fail.

155) CLI reference includes `overlay` composite mode, but CLI supports only over/add/multiply/screen.
   - Evidence (doc claim): `docs/src/appendix/cli-ref.md:210`
   - Evidence (implementation): `crates/vfx-cli/src/main.rs:388`
   - Impact: documented mode fails.

156) CLI reference for transform includes `--translate` and implies arbitrary rotation degrees, but transform only supports flip/rotate 90/180/270/transpose and has no translate.
   - Evidence (doc claim): `docs/src/appendix/cli-ref.md:229`, `docs/src/appendix/cli-ref.md:232`
   - Evidence (implementation): `crates/vfx-cli/src/main.rs:494`, `crates/vfx-cli/src/main.rs:516`, `crates/vfx-cli/src/commands/transform.rs:55`
   - Impact: documented operations fail.

157) CLI reference requires an input for `vfx view`, but CLI allows it to be omitted.
   - Evidence (doc claim): `docs/src/appendix/cli-ref.md:249`
   - Evidence (implementation): `crates/vfx-cli/src/main.rs:756`
   - Impact: usage line is incorrect.

158) CLI reference documents `icc` and `ocio` commands that do not exist in the CLI.
   - Evidence (doc claim): `docs/src/appendix/cli-ref.md:264`, `docs/src/appendix/cli-ref.md:284`
   - Evidence (implementation): `crates/vfx-cli/src/main.rs:152`
   - Impact: commands fail.

159) CLI reference omits many implemented commands (crop, diff, sharpen, maketx, grep, batch, layers, extract-layer, merge-layers, channel-shuffle, channel-extract, paste, rotate, warp, udim, grade, clamp, premult).
   - Evidence (implementation): `crates/vfx-cli/src/main.rs:152`
   - Impact: documentation is incomplete for existing CLI surface.

160) CLI reference publishes exit codes (0-5) and env vars `VFX_LOG`/`VFX_THREADS`, but CLI only defines flags and does not implement those env vars or a general exit code mapping.
   - Evidence (doc claim): `docs/src/appendix/cli-ref.md:306`, `docs/src/appendix/cli-ref.md:322`, `docs/src/appendix/cli-ref.md:323`
   - Evidence (implementation): `crates/vfx-cli/src/main.rs:140`, `crates/vfx-cli/src/main.rs:144`, `crates/vfx-cli/src/commands/view.rs:22`
   - Impact: users rely on behaviors that are not implemented.

161) Color space appendix claims a complete reference but omits several implemented primaries (S-Gamut3.Cine, Canon CGamut, DaVinci Wide Gamut, DJI D-Gamut).
   - Evidence (doc claim): `docs/src/appendix/color-spaces.md:3`
   - Evidence (implementation): `crates/vfx-primaries/src/lib.rs:291`, `crates/vfx-primaries/src/lib.rs:301`, `crates/vfx-primaries/src/lib.rs:340`, `crates/vfx-primaries/src/lib.rs:350`
   - Impact: users miss supported color spaces.

162) Color space appendix lists RED Wide Gamut primaries that do not match the implemented values.
   - Evidence (doc claim): `docs/src/appendix/color-spaces.md:31`
   - Evidence (implementation): `crates/vfx-primaries/src/lib.rs:311`
   - Impact: reference values are inconsistent with runtime transforms.

163) Color space appendix lists ACESproxy, but vfx-transfer does not implement it.
   - Evidence (doc claim): `docs/src/appendix/color-spaces.md:79`
   - Evidence (implementation): `crates/vfx-transfer/src/lib.rs:80`, `crates/vfx-transfer/src/lib.rs:81`
   - Impact: documented transfer function cannot be used.

164) Color space appendix usage snippet calls `srgb_to_linear`/`linear_to_srgb`, but vfx-transfer only exposes `srgb_eotf`/`srgb_oetf` (or `srgb::eotf/oetf`).
   - Evidence (doc claim): `docs/src/appendix/color-spaces.md:165`, `docs/src/appendix/color-spaces.md:166`
   - Evidence (implementation): `crates/vfx-transfer/src/lib.rs:88`
   - Impact: example code does not compile.

165) Feature matrix lists RED Log3G12 as implemented, but vfx-transfer only implements REDLogFilm and REDLog3G10.
   - Evidence (doc claim): `docs/src/appendix/feature-matrix.md:36`
   - Evidence (implementation): `crates/vfx-transfer/src/red_log.rs:3`, `crates/vfx-transfer/src/red_log.rs:9`
   - Impact: feature matrix overstates transfer support.

166) Feature matrix lists CIE RGB primaries as implemented, but no CIE RGB primaries exist in vfx-primaries.
   - Evidence (doc claim): `docs/src/appendix/feature-matrix.md:63`
   - Evidence (implementation): `crates/vfx-primaries/src/lib.rs:594`
   - Impact: feature matrix overstates primaries coverage.

167) Feature matrix claims PSD read support and TX read/write, but vfx-io format detection has no PSD/TX format entries.
   - Evidence (doc claim): `docs/src/appendix/feature-matrix.md:173`, `docs/src/appendix/feature-matrix.md:174`
   - Evidence (implementation): `crates/vfx-io/src/detect.rs:11`
   - Impact: documented formats are not available via format detection/registry.

168) Feature matrix marks AVIF and JPEG 2000 as read/write, but AVIF is write-only and JP2 is read-only.
   - Evidence (doc claim): `docs/src/appendix/feature-matrix.md:182`, `docs/src/appendix/feature-matrix.md:183`
   - Evidence (implementation): `crates/vfx-io/src/avif.rs:1`, `crates/vfx-io/src/jp2.rs:1`
   - Impact: feature matrix overstates I/O capabilities.

169) Feature matrix claims `.cube` supports combined 1D+3D LUTs, but the parser rejects mixed 1D/3D headers.
   - Evidence (doc claim): `docs/src/appendix/feature-matrix.md:120`
   - Evidence (implementation): `crates/vfx-lut/src/cube.rs:78`, `crates/vfx-lut/src/cube.rs:132`
   - Impact: combined LUT files fail to parse.

170) Architecture README claims the workspace has 16 crates, but the workspace members list includes 17 entries.
   - Evidence (doc claim): `docs/src/architecture/README.md:3`
   - Evidence (implementation): `Cargo.toml:4`, `Cargo.toml:21`
   - Impact: documentation understates the workspace surface.

171) Architecture README says `vfx-core` defines `ImageData`, but `ImageData` is defined in vfx-io.
   - Evidence (doc claim): `docs/src/architecture/README.md:59`
   - Evidence (implementation): `crates/vfx-io/src/lib.rs:537`
   - Impact: crate ownership is misdocumented.

172) Architecture README maps ImageSpec to `vfx_io::ImageInfo`, but vfx-io does not define an `ImageInfo` type.
   - Evidence (doc claim): `docs/src/architecture/README.md:43`
   - Evidence (implementation): `crates/vfx-io/src/cache.rs:103`
   - Impact: API mapping guidance is incorrect.

173) Crate graph documentation says vfx-io depends only on vfx-core and uses the `exr` crate, but vfx-io depends on `vfx-ocio` and uses `vfx-exr`.
   - Evidence (doc claim): `docs/src/architecture/crate-graph.md:90`, `docs/src/architecture/crate-graph.md:98`
   - Evidence (implementation): `crates/vfx-io/Cargo.toml:60`, `crates/vfx-io/Cargo.toml:66`
   - Impact: dependency graph and external dependency list are inaccurate.

174) Crate graph external dependency table lists `exr`, but the workspace uses `vfx-exr`.
   - Evidence (doc claim): `docs/src/architecture/crate-graph.md:187`
   - Evidence (implementation): `crates/vfx-io/Cargo.toml:66`
   - Impact: users looking for the `exr` crate integration will be misled.

175) Data-flow doc shows `ImageData` with an `ImageBuffer` enum, but the actual struct uses `PixelFormat`, `PixelData`, and `Metadata` and there is no `ImageBuffer` type.
   - Evidence (doc claim): `docs/src/architecture/data-flow.md:10`, `docs/src/architecture/data-flow.md:21`
   - Evidence (implementation): `crates/vfx-io/src/lib.rs:537`, `crates/vfx-io/src/lib.rs:697`
   - Impact: documentation misrepresents the core data type.

176) Data-flow doc describes `FormatError`/format-specific errors, but vfx-io exposes a unified `IoError`.
   - Evidence (doc claim): `docs/src/architecture/data-flow.md:217`
   - Evidence (implementation): `crates/vfx-io/src/error.rs:10`
   - Impact: error handling guidance is inaccurate.

177) Architecture decisions doc shows vfx-io default features as exr/png/jpeg only, but vfx-io defaults also include tiff/dpx/hdr.
   - Evidence (doc claim): `docs/src/architecture/decisions.md:66`
   - Evidence (implementation): `crates/vfx-io/Cargo.toml:12`
   - Impact: feature guidance is incorrect.

178) Crates README dependency hierarchy inverts the vfx-core/vfx-math relationship (shows vfx-core depending on vfx-math).
   - Evidence (doc claim): `docs/src/crates/README.md:78`, `docs/src/crates/README.md:79`
   - Evidence (implementation): `crates/vfx-math/Cargo.toml:10`
   - Impact: dependency guidance is wrong.

179) Crates README dependency hierarchy omits the vfx-io dependency on vfx-ocio.
   - Evidence (doc claim): `docs/src/crates/README.md:78`
   - Evidence (implementation): `crates/vfx-io/Cargo.toml:60`
   - Impact: dependency graph is incomplete.

180) vfx-core crate docs show `Image<Srgb, f32>` without channel count const generic and use `img.view()`/`view_mut()` without a region argument.
   - Evidence (doc claim): `docs/src/crates/core.md:23`, `docs/src/crates/core.md:40`, `docs/src/crates/core.md:46`
   - Evidence (implementation): `crates/vfx-core/src/image.rs:392`
   - Impact: example code does not compile.

181) vfx-core crate docs use `srgb_to_linear` in the design philosophy snippet, but vfx-core does not define that function.
   - Evidence (doc claim): `docs/src/crates/core.md:79`
   - Evidence (implementation): `crates/vfx-core/src/lib.rs:1`
   - Impact: example code does not compile.

182) vfx-io EXR docs reference `read_layer` and `write_with_options`, but the EXR API exposes `read_layers` and `ExrWriter::with_options` instead.
   - Evidence (doc claim): `docs/src/crates/io.md:48`, `docs/src/crates/io.md:55`
   - Evidence (implementation): `crates/vfx-io/src/exr.rs:819`, `crates/vfx-io/src/exr.rs:550`
   - Impact: example code does not compile.

183) vfx-io docs show HEIF write call `write_heif(..., Some(&hdr_info))` where `hdr_info` is an `Option`, which does not match the API signature.
   - Evidence (doc claim): `docs/src/crates/io.md:84`, `docs/src/crates/io.md:89`
   - Evidence (implementation): `crates/vfx-io/src/heif.rs:392`
   - Impact: example code does not compile.

184) vfx-io docs list AVIF write bit depths 8/10, but the AVIF writer converts via `to_u8()` and does not write 10-bit.
   - Evidence (doc claim): `docs/src/crates/io.md:27`
   - Evidence (implementation): `crates/vfx-io/src/avif.rs:70`
   - Impact: format capabilities are overstated.

185) vfx-io docs omit supported optional formats (PSD, DDS, KTX) from the supported formats table.
   - Evidence (doc claim): `docs/src/crates/io.md:20`
   - Evidence (implementation): `crates/vfx-io/Cargo.toml:30`, `crates/vfx-io/Cargo.toml:33`, `crates/vfx-io/Cargo.toml:36`
   - Impact: documentation understates available formats.

186) vfx-io docs show sequence API `find_sequences` and `Sequence::from_pattern(pattern, start, end)` plus iterating paths, but actual API uses `scan_dir`, `Sequence::from_pattern(pattern)` and `frame_path()/paths()`.
   - Evidence (doc claim): `docs/src/crates/io.md:120`, `docs/src/crates/io.md:127`
   - Evidence (implementation): `crates/vfx-io/src/sequence.rs:390`, `crates/vfx-io/src/sequence.rs:580`, `crates/vfx-io/src/sequence.rs:502`
   - Impact: example code does not compile and describes non-existent APIs.

187) vfx-io docs show UDIM API `UdimSet` and `udim_pattern`, but the module exposes `UdimResolver` and `UdimTile`.
   - Evidence (doc claim): `docs/src/crates/io.md:136`
   - Evidence (implementation): `crates/vfx-io/src/udim.rs:53`, `crates/vfx-io/src/udim.rs:56`
   - Impact: example code does not compile.

188) vfx-io docs show streaming API `StreamReader/StreamWriter`, but the streaming module exposes `open_streaming` and `StreamingSource`.
   - Evidence (doc claim): `docs/src/crates/io.md:152`
   - Evidence (implementation): `crates/vfx-io/src/streaming/mod.rs:184`, `crates/vfx-io/src/streaming/traits.rs:30`
   - Impact: example code does not compile.

189) vfx-io docs claim format detection checks extension first then magic bytes, but the implementation checks magic bytes first and falls back to extension.
   - Evidence (doc claim): `docs/src/crates/io.md:214`
   - Evidence (implementation): `crates/vfx-io/src/detect.rs:28`
   - Impact: behavior description is inverted.

190) vfx-io docs list `exr` crate as a dependency, but vfx-io uses `vfx-exr`.
   - Evidence (doc claim): `docs/src/crates/io.md:183`
   - Evidence (implementation): `crates/vfx-io/Cargo.toml:66`
   - Impact: dependency list is inaccurate.

191) vfx-color docs use `ColorProcessor::srgb_to_linear` and `apply_srgb_to_linear`, but those methods do not exist.
   - Evidence (doc claim): `docs/src/crates/color.md:53`, `docs/src/crates/color.md:57`
   - Evidence (implementation): `crates/vfx-color/src/processor.rs:93`
   - Impact: example code does not compile.

192) vfx-color pipeline docs call `.apply_buffer(&mut data)` and `.lut_3d(&display_lut)`/`.tonemap_reinhard()`/`.adapt(...)`, but the API only provides `apply()` and `ColorProcessor::apply_buffer(...)`, plus `lut3d` (no underscore) and no tonemap/adapt helpers.
   - Evidence (doc claim): `docs/src/crates/color.md:83`, `docs/src/crates/color.md:217`, `docs/src/crates/color.md:206`, `docs/src/crates/color.md:227`
   - Evidence (implementation): `crates/vfx-color/src/pipeline.rs:173`, `crates/vfx-color/src/processor.rs:264`
   - Impact: examples reference non-existent methods.

193) vfx-color docs describe chromatic adaptation via `Pipeline::adapt`, but adaptation only exists on conversion helpers (RgbConvert) and `adapt_matrix`.
   - Evidence (doc claim): `docs/src/crates/color.md:227`
   - Evidence (implementation): `crates/vfx-color/src/convert.rs:148`
   - Impact: suggested pipeline usage is not supported.

194) vfx-color docs show `ColorProcessor::apply_srgb_to_linear` and `Pipeline::apply_buffer` working on `Vec<[f32; 3]>`, but processor buffer APIs operate on flat `[f32]` with width/height.
   - Evidence (doc claim): `docs/src/crates/color.md:56`, `docs/src/crates/color.md:83`
   - Evidence (implementation): `crates/vfx-color/src/processor.rs:264`
   - Impact: buffer examples do not compile and mismatch data layout.

195) vfx-compute docs use `ComputeImage::to_vec`, but the API only exposes `into_vec()` (consuming) and `data()`.
   - Evidence (doc claim): `docs/src/crates/compute.md:34`, `docs/src/crates/compute.md:87`
   - Evidence (implementation): `crates/vfx-compute/src/image.rs:139`
   - Impact: example code does not compile.

196) vfx-compute docs show `ComputeImage::from_image_data` and `ComputeImage::to_image_data`, but conversion is via free functions `from_image_data`/`to_image_data` (io feature), not methods.
   - Evidence (doc claim): `docs/src/crates/compute.md:84`, `docs/src/crates/compute.md:90`
   - Evidence (implementation): `crates/vfx-compute/src/convert.rs:272`, `crates/vfx-compute/src/convert.rs:289`
   - Impact: example code does not compile.

197) vfx-compute docs build a `Mat3` and pass it to `ComputeOp::Matrix`, but `ComputeOp::Matrix` and `apply_matrix` require a 4x4 `[f32; 16]` matrix.
   - Evidence (doc claim): `docs/src/crates/compute.md:129`, `docs/src/crates/compute.md:131`, `docs/src/crates/compute.md:162`
   - Evidence (implementation): `crates/vfx-compute/src/pipeline.rs:256`, `crates/vfx-compute/src/processor.rs:560`
   - Impact: example code does not compile and uses the wrong matrix size.

198) vfx-compute docs show `ComputePipeline::builder().add(...).build()` and `pipeline.apply(...)`, but the builder only configures backend/limits and the pipeline API processes via `process(input, output, ops)`.
   - Evidence (doc claim): `docs/src/crates/compute.md:158`, `docs/src/crates/compute.md:165`
   - Evidence (implementation): `crates/vfx-compute/src/pipeline.rs:414`, `crates/vfx-compute/src/pipeline.rs:870`, `crates/vfx-compute/src/pipeline.rs:513`
   - Impact: example code does not compile and describes a non-existent API.

199) vfx-compute docs use `ProcessorBuilder::prefer_gpu(true)`, but `ProcessorBuilder` exposes `backend()` and has no `prefer_gpu` method.
   - Evidence (doc claim): `docs/src/crates/compute.md:176`
   - Evidence (implementation): `crates/vfx-compute/src/processor.rs:316`
   - Impact: example code does not compile.

200) vfx-compute docs show `TileWorkflow::new` and `workflow.process(...)`, but `TileWorkflow` is just an enum for tile-size selection with no constructor or processing API.
   - Evidence (doc claim): `docs/src/crates/compute.md:189`
   - Evidence (implementation): `crates/vfx-compute/src/backend/tiling.rs:165`
   - Impact: example code does not compile and overstates capabilities.

201) vfx-compute docs call `proc.limits()?` and reference `max_texture_dimension`/`max_buffer_size`, but `limits()` returns `&GpuLimits` and fields are `max_tile_dim` and `max_buffer_bytes`.
   - Evidence (doc claim): `docs/src/crates/compute.md:205`, `docs/src/crates/compute.md:206`, `docs/src/crates/compute.md:207`
   - Evidence (implementation): `crates/vfx-compute/src/processor.rs:482`, `crates/vfx-compute/src/backend/tiling.rs:29`
   - Impact: example code does not compile.

202) vfx-compute docs use `LayerProcessor::new(&proc)`, `process_groups`, and `ChannelGroup::Color/Depth/Id`, but the API provides `LayerProcessor::new(processor)`, `process_layer`, and `ChannelGroup` is a struct paired with `ChannelClassification`.
   - Evidence (doc claim): `docs/src/crates/compute.md:218`, `docs/src/crates/compute.md:220`
   - Evidence (implementation): `crates/vfx-compute/src/layer.rs:241`, `crates/vfx-compute/src/layer.rs:272`, `crates/vfx-compute/src/layer.rs:58`
   - Impact: example code does not compile and misrepresents the layer API.

203) vfx-compute docs claim `Processor::auto()` considers image size, but auto selection only picks the best available backend and does not take image dimensions.
   - Evidence (doc claim): `docs/src/crates/compute.md:255`
   - Evidence (implementation): `crates/vfx-compute/src/backend/mod.rs:441`
   - Impact: behavior description is inaccurate.

204) vfx-cli docs list `-q/--quiet` and describe `--log <FILE>`, but the CLI defines no quiet flag, uses `-l/--log` with optional path, and also provides `-j/--threads` and `--allow-non-color` which are not documented.
   - Evidence (doc claim): `docs/src/crates/cli.md:45`, `docs/src/crates/cli.md:46`
   - Evidence (implementation): `crates/vfx-cli/src/main.rs:132`, `crates/vfx-cli/src/main.rs:138`, `crates/vfx-cli/src/main.rs:141`, `crates/vfx-cli/src/main.rs:145`
   - Impact: global CLI flags are misdocumented.

205) vfx-cli docs command overview omits many available subcommands (crop/diff/blur/sharpen/transform/maketx/grep/extract-layer/merge-layers/channel-shuffle/channel-extract/paste/rotate/warp/udim/grade/clamp/premult) and does not note the additional layer/channel commands.
   - Evidence (doc claim): `docs/src/crates/cli.md:25`
   - Evidence (implementation): `crates/vfx-cli/src/main.rs:152`, `crates/vfx-cli/src/main.rs:176`
   - Impact: capability overview is incomplete and misleading.

206) vfx-cli docs show `vfx info ... --layers`, but the info command only supports `--stats`, `--all`, and `--json`; layer listing is a separate `layers` command.
   - Evidence (doc claim): `docs/src/crates/cli.md:72`
   - Evidence (implementation): `crates/vfx-cli/src/main.rs:252`, `crates/vfx-cli/src/main.rs:584`
   - Impact: example command does not work.

207) vfx-cli docs use positional output paths for `resize`, `color`, `aces`, `composite`, and `lut`, but these commands require `-o/--output` flags.
   - Evidence (doc claim): `docs/src/crates/cli.md:103`, `docs/src/crates/cli.md:123`, `docs/src/crates/cli.md:144`, `docs/src/crates/cli.md:162`, `docs/src/crates/cli.md:194`
   - Evidence (implementation): `crates/vfx-cli/src/main.rs:292`, `crates/vfx-cli/src/main.rs:438`, `crates/vfx-cli/src/main.rs:730`, `crates/vfx-cli/src/main.rs:377`, `crates/vfx-cli/src/main.rs:476`
   - Impact: example commands do not match actual CLI usage.

208) vfx-cli docs list resize filters `nearest`, `bilinear`, `bicubic`, `lanczos`, but the CLI accepts `box`, `bilinear`, `lanczos`, `mitchell`.
   - Evidence (doc claim): `docs/src/crates/cli.md:115`
   - Evidence (implementation): `crates/vfx-cli/src/main.rs:312`
   - Impact: filter documentation is incorrect.

209) vfx-cli docs use `--inverse` for LUT, but the CLI flag is `--invert`.
   - Evidence (doc claim): `docs/src/crates/cli.md:200`
   - Evidence (implementation): `crates/vfx-cli/src/main.rs:490`
   - Impact: example command does not work.

210) vfx-cli docs describe `batch` with `--output` templates and `--jobs`, but the CLI accepts `--output-dir`, `--args`, and optional `--format`, with no job control or templated output variables.
   - Evidence (doc claim): `docs/src/crates/cli.md:209`, `docs/src/crates/cli.md:216`
   - Evidence (implementation): `crates/vfx-cli/src/main.rs:560`
   - Impact: batch usage guidance is incorrect.

211) vfx-cli docs show `vfx view ... --layer`, but view args do not include a layer option and the view command exists only when built with the `viewer` feature.
   - Evidence (doc claim): `docs/src/crates/cli.md:240`
   - Evidence (implementation): `crates/vfx-cli/src/main.rs:203`, `crates/vfx-cli/src/main.rs:754`
   - Impact: example command does not work and feature gating is undocumented.

212) vfx-ops docs show `over(&foreground, &background)` and `blend(&a, &b, ...)` without width/height arguments, but the APIs require width and height.
   - Evidence (doc claim): `docs/src/crates/ops.md:111`, `docs/src/crates/ops.md:114`
   - Evidence (implementation): `crates/vfx-ops/src/composite.rs:250`, `crates/vfx-ops/src/composite.rs:296`
   - Impact: example code does not compile.

213) vfx-ops docs list `BlendMode::Over`, but the enum uses `BlendMode::Normal` for standard over.
   - Evidence (doc claim): `docs/src/crates/ops.md:122`
   - Evidence (implementation): `crates/vfx-ops/src/composite.rs:40`, `crates/vfx-ops/src/composite.rs:43`
   - Impact: enum variant name is wrong.

214) vfx-ops docs call `premultiply(&mut rgba_data)` and `unpremultiply(&mut rgba_data)`, but those functions operate on a single `[f32; 4]`; in-place buffer helpers are `premultiply_inplace`/`unpremultiply_inplace`.
   - Evidence (doc claim): `docs/src/crates/ops.md:138`, `docs/src/crates/ops.md:141`
   - Evidence (implementation): `crates/vfx-ops/src/composite.rs:345`, `crates/vfx-ops/src/composite.rs:451`
   - Impact: example code does not compile.

215) vfx-ops docs reference `rotate_90`, `rotate_180`, `rotate_270`, but the API provides `rotate_90_cw`/`rotate_90_ccw` and `rotate_180`, and no `rotate_270` helper.
   - Evidence (doc claim): `docs/src/crates/ops.md:152`, `docs/src/crates/ops.md:155`
   - Evidence (implementation): `crates/vfx-ops/src/transform.rs:176`, `crates/vfx-ops/src/transform.rs:215`, `crates/vfx-ops/src/transform.rs:253`
   - Impact: example code does not compile and function names are inaccurate.

216) vfx-ops docs use `warp::barrel_distort`/`pincushion_distort` and `st_map`, but the module exposes `barrel`/`pincushion` and has no `st_map`.
   - Evidence (doc claim): `docs/src/crates/ops.md:180`, `docs/src/crates/ops.md:194`
   - Evidence (implementation): `crates/vfx-ops/src/warp.rs:78`, `crates/vfx-ops/src/warp.rs:103`
   - Impact: example code does not compile.

217) vfx-ops docs reference `layer_ops::{apply_to_layer, LayerMask}`, but those APIs do not exist; layer ops provide helpers like `resize_layer` and `blur_layer`.
   - Evidence (doc claim): `docs/src/crates/ops.md:205`
   - Evidence (implementation): `crates/vfx-ops/src/layer_ops.rs:22`
   - Impact: example code does not compile.

218) vfx-ops docs reference `guard::ensure_color_processing`, but the API provides `ensure_color_channels`/`ensure_color_channels_layer`.
   - Evidence (doc claim): `docs/src/crates/ops.md:243`
   - Evidence (implementation): `crates/vfx-ops/src/guard.rs:18`, `crates/vfx-ops/src/guard.rs:68`
   - Impact: example code does not compile.

219) vfx-ops docs dependencies omit `vfx-color`, but the crate depends on it.
   - Evidence (doc claim): `docs/src/crates/ops.md:271`
   - Evidence (implementation): `crates/vfx-ops/Cargo.toml:18`
   - Impact: dependency list is incomplete.

220) vfx-lut docs call `Lut3D::apply_tetrahedral`, but tetrahedral interpolation is a private method; public usage is `with_interpolation(Interpolation::Tetrahedral)` plus `apply`.
   - Evidence (doc claim): `docs/src/crates/lut.md:45`
   - Evidence (implementation): `crates/vfx-lut/src/lut3d.rs:118`, `crates/vfx-lut/src/lut3d.rs:225`
   - Impact: example code does not compile.

221) vfx-lut CLF example matches `ProcessNode::Matrix(m)`/`ProcessNode::Lut1D(lut)` and uses `lut.size`, but variants are struct-style and `Lut1D` exposes `size()` (no size field).
   - Evidence (doc claim): `docs/src/crates/lut.md:89`, `docs/src/crates/lut.md:90`
   - Evidence (implementation): `crates/vfx-lut/src/clf.rs:382`, `crates/vfx-lut/src/clf.rs:389`, `crates/vfx-lut/src/lut1d.rs:150`
   - Impact: example code does not compile.

222) vfx-lut docs import `apply_rrt_odt_srgb` but call `apply_pixel`, which does not exist; `apply_rrt_odt_srgb` operates on a buffer, not a single RGB tuple.
   - Evidence (doc claim): `docs/src/crates/lut.md:180`
   - Evidence (implementation): `crates/vfx-color/src/aces.rs:224`
   - Impact: example code does not compile.

223) vfx-lut docs list only `.cube`, `.clf/.ctf`, `.spi1d/.spi3d`, and `.3dl`, but the crate supports additional formats (csp, hdl, truelight, iridas itx/look, pandora mga, nuke vf, spi_mtx, discreet1dl, cdl).
   - Evidence (doc claim): `docs/src/crates/lut.md:59`, `docs/src/crates/lut.md:110`
   - Evidence (implementation): `crates/vfx-lut/src/lib.rs:52`
   - Impact: format coverage is understated.

224) vfx-transfer docs omit `d_log` and `davinci_intermediate` from supported functions, but both modules and re-exports exist.
   - Evidence (doc claim): `docs/src/crates/transfer.md:15`
   - Evidence (implementation): `crates/vfx-transfer/src/lib.rs:84`, `crates/vfx-transfer/src/lib.rs:85`
   - Impact: supported transfer functions list is incomplete.

225) vfx-ocio docs use `Config::from_str`, but the API exposes `Config::from_yaml_str` (and requires a working_dir), not `from_str`.
   - Evidence (doc claim): `docs/src/crates/ocio.md:48`
   - Evidence (implementation): `crates/vfx-ocio/src/config.rs:220`
   - Impact: example code does not compile.

226) vfx-ocio docs call `config.processor_opt`, but the API provides `processor_with_opts`.
   - Evidence (doc claim): `docs/src/crates/ocio.md:127`
   - Evidence (implementation): `crates/vfx-ocio/src/config.rs:983`, `crates/vfx-ocio/src/config.rs:991`
   - Impact: example code does not compile.

227) vfx-ocio docs call `config.processor_with_look`, but the API provides `processor_with_looks`.
   - Evidence (doc claim): `docs/src/crates/ocio.md:198`
   - Evidence (implementation): `crates/vfx-ocio/src/config.rs:1118`
   - Impact: example code does not compile.

228) vfx-ocio docs reference `../OCIO_PARITY_AUDIT.md`, but the file is not present in the repository.
   - Evidence (doc claim): `docs/src/crates/ocio.md:342`
   - Evidence (implementation): `OCIO_PARITY_AUDIT.md` (not found)
   - Impact: documentation link is broken.

229) vfx-icc docs use `Profile::from_bytes`, but the API does not expose a `from_bytes` constructor.
   - Evidence (doc claim): `docs/src/crates/icc.md:41`, `docs/src/crates/icc.md:200`
   - Evidence (implementation): `crates/vfx-icc/src/profile.rs:52`
   - Impact: example code does not compile.

230) vfx-icc docs reference `Profile::lab_d50`, but the API provides `Profile::lab()`.
   - Evidence (doc claim): `docs/src/crates/icc.md:61`
   - Evidence (implementation): `crates/vfx-icc/src/profile.rs:193`
   - Impact: example code does not compile.

231) vfx-icc docs call `Profile::from_standard(StandardProfile::SRgb)?`, but the variant is `StandardProfile::Srgb` and `from_standard` returns a `Profile` (no `Result`).
   - Evidence (doc claim): `docs/src/crates/icc.md:71`
   - Evidence (implementation): `crates/vfx-icc/src/standard.rs:12`, `crates/vfx-icc/src/profile.rs:108`
   - Impact: example code does not compile.

232) vfx-icc docs call `convert_rgb(&srgb, &aces, Intent::..., &mut pixels)`, but the function signature is `convert_rgb(pixels, source, dest, intent)`.
   - Evidence (doc claim): `docs/src/crates/icc.md:135`
   - Evidence (implementation): `crates/vfx-icc/src/transform.rs:226`
   - Impact: example code does not compile.

233) vfx-icc docs mention `IccError::FileNotFound` and `IccError::TransformError`, but actual variants are `LoadFailed`, `TransformFailed`, `InvalidProfile`, and `Io`.
   - Evidence (doc claim): `docs/src/crates/icc.md:182`, `docs/src/crates/icc.md:183`
   - Evidence (implementation): `crates/vfx-icc/src/error.rs:11`, `crates/vfx-icc/src/error.rs:20`
   - Impact: error-handling examples are incorrect.

234) vfx-view docs show `ViewerConfig` with a `layer` field, but `ViewerConfig` only includes ocio/display/view/colorspace/verbose.
   - Evidence (doc claim): `docs/src/crates/view.md:129`, `docs/src/crates/view.md:130`
   - Evidence (implementation): `crates/vfx-view/src/app.rs:44`
   - Impact: example code does not compile.

235) vfx-rs-py docs use `Image.to_numpy`, `Image.from_numpy`, and `img.data()`, but bindings expose `Image.numpy(copy=...)` and the constructor `Image(array)`; there is no `data()`.
   - Evidence (doc claim): `docs/src/crates/python.md:42`, `docs/src/crates/python.md:48`, `docs/src/crates/python.md:71`
   - Evidence (implementation): `crates/vfx-rs-py/src/image.rs:24`, `crates/vfx-rs-py/src/image.rs:56`, `crates/vfx-rs-py/src/image.rs:94`
   - Impact: examples do not work as written.

236) vfx-rs-py docs list `img.format` values without `u32`, but the binding can return `u32`.
   - Evidence (doc claim): `docs/src/crates/python.md:64`
   - Evidence (implementation): `crates/vfx-rs-py/src/image.rs:79`
   - Impact: format list is incomplete.

237) vfx-rs-py docs show `vfx_rs.write(..., quality=..., compression=...)`, but top-level `write` only accepts `(path, image)`; format options are under `vfx_rs.io.*` functions.
   - Evidence (doc claim): `docs/src/crates/python.md:88`, `docs/src/crates/python.md:89`
   - Evidence (implementation): `crates/vfx-rs-py/src/lib.rs:73`, `crates/vfx-rs-py/src/io.rs:40`
   - Impact: examples do not work as written.

238) vfx-rs-py docs reference `vfx_rs.color` and color functions (apply_srgb_eotf, apply_rrt_odt_srgb), but no `color` submodule is exported.
   - Evidence (doc claim): `docs/src/crates/python.md:95`, `docs/src/crates/python.md:103`, `docs/src/crates/python.md:264`
   - Evidence (implementation): `crates/vfx-rs-py/src/lib.rs:73`
   - Impact: examples do not work as written.

239) vfx-rs-py LUT examples call `lut.apply_3d`/`apply_1d`, but bindings expose `Lut1D`/`Lut3D` classes with `apply` methods only.
   - Evidence (doc claim): `docs/src/crates/python.md:135`, `docs/src/crates/python.md:136`
   - Evidence (implementation): `crates/vfx-rs-py/src/lut.rs:24`, `crates/vfx-rs-py/src/lut.rs:49`
   - Impact: examples do not work as written.

240) vfx-rs-py ops examples use `resize(..., scale=...)`, `resize(..., filter="lanczos")`, `blur(radius=..., type=...)`, and `blend(mode=...)`, but bindings require width/height with `ResizeFilter` enum, expose `blur(sigma=...)`, and have no `blend` function.
   - Evidence (doc claim): `docs/src/crates/python.md:145`, `docs/src/crates/python.md:149`, `docs/src/crates/python.md:153`
   - Evidence (implementation): `crates/vfx-rs-py/src/ops.rs:418`, `crates/vfx-rs-py/src/ops.rs:941`, `crates/vfx-rs-py/src/ops.rs:1560`
   - Impact: examples do not work as written.

241) vfx-rs-py docs show `read_layers`, `read_layer`, and `write_layers`, but bindings only provide `read_layered` returning a `LayeredImage` object.
   - Evidence (doc claim): `docs/src/crates/python.md:227`, `docs/src/crates/python.md:232`
   - Evidence (implementation): `crates/vfx-rs-py/src/lib.rs:58`, `crates/vfx-rs-py/src/layered.rs:1`
   - Impact: examples do not work as written.

242) vfx-rs-py docs reference `vfx_rs.IoError` and `vfx_rs.FormatError`, but bindings raise Python `IOError`/`ValueError` and do not define those exception types.
   - Evidence (doc claim): `docs/src/crates/python.md:249`, `docs/src/crates/python.md:251`
   - Evidence (implementation): `crates/vfx-rs-py/src/lib.rs:30`
   - Impact: error handling examples are incorrect.

243) vfx-rs-py docs claim `to_numpy()` returns a zero-copy view when possible, but `Image.numpy()` always allocates a new `Vec<f32>` via `to_f32`.
   - Evidence (doc claim): `docs/src/crates/python.md:213`, `docs/src/crates/python.md:214`
   - Evidence (implementation): `crates/vfx-rs-py/src/image.rs:96`
   - Impact: performance guidance is inaccurate.

244) vfx-tests docs describe a `tests/` directory with specific files, but the crate uses `crates/vfx-tests/src/lib.rs` and `crates/vfx-tests/src/golden.rs` with no `tests/` folder.
   - Evidence (doc claim): `docs/src/crates/tests.md:12`
   - Evidence (implementation): `crates/vfx-tests/src/lib.rs:1`, `crates/vfx-tests/src/golden.rs:1`
   - Impact: test layout documentation is incorrect.

245) vfx-tests docs suggest running `cargo test -p vfx-tests --test io_roundtrip`, but there are no integration test targets under `crates/vfx-tests/tests`.
   - Evidence (doc claim): `docs/src/crates/tests.md:29`
   - Evidence (implementation): `crates/vfx-tests/src/lib.rs:1`
   - Impact: the command will fail.

246) vfx-tests docs reference `test/images/scene_linear.exr` and a `test/images` layout, but no such file or directory exists; assets live under `test/` with different structure.
   - Evidence (doc claim): `docs/src/crates/tests.md:84`
   - Evidence (implementation): `test/` (no `test/images`), `scene_linear.exr` not found
   - Impact: example paths are invalid.

247) vfx-tests docs dependency list includes `vfx-ocio` and a `[dev-dependencies]` section, but `crates/vfx-tests/Cargo.toml` has neither and lists additional crates instead.
   - Evidence (doc claim): `docs/src/crates/tests.md:214`, `docs/src/crates/tests.md:221`
   - Evidence (implementation): `crates/vfx-tests/Cargo.toml:1`
   - Impact: dependency guidance is incorrect.

248) vfx-bench docs show `cargo bench -p vfx-bench -- resize`, but the only bench target is `vfx_bench` and no resize benchmark exists.
   - Evidence (doc claim): `docs/src/crates/bench.md:16`
   - Evidence (implementation): `crates/vfx-bench/Cargo.toml:20`, `crates/vfx-bench/Cargo.toml:21`, `crates/vfx-bench/benches/vfx_bench.rs:15`
   - Impact: command does not select any matching benchmark group.

249) vfx-bench docs list I/O/resize/color/LUT benchmark outputs, but the actual groups are `transfer`, `lut1d`, `lut3d`, `cdl`, `simd`, and `pixels`.
   - Evidence (doc claim): `docs/src/crates/bench.md:27`, `docs/src/crates/bench.md:36`, `docs/src/crates/bench.md:44`, `docs/src/crates/bench.md:52`
   - Evidence (implementation): `crates/vfx-bench/benches/vfx_bench.rs:15`, `crates/vfx-bench/benches/vfx_bench.rs:53`, `crates/vfx-bench/benches/vfx_bench.rs:85`, `crates/vfx-bench/benches/vfx_bench.rs:130`, `crates/vfx-bench/benches/vfx_bench.rs:166`, `crates/vfx-bench/benches/vfx_bench.rs:217`
   - Impact: category names and sample outputs do not reflect actual benchmarks.

250) vfx-bench docs describe GPU benchmarks and `--features gpu`, but vfx-bench declares no `gpu` feature and contains no GPU benchmark groups.
   - Evidence (doc claim): `docs/src/crates/bench.md:113`, `docs/src/crates/bench.md:115`, `docs/src/crates/bench.md:122`
   - Evidence (implementation): `crates/vfx-bench/Cargo.toml:1`, `crates/vfx-bench/benches/vfx_bench.rs:15`
   - Impact: GPU instructions do not work.

251) vfx-bench docs include a `memory_exr_read` `#[bench]` example, but vfx-bench uses Criterion and has no such benchmark.
   - Evidence (doc claim): `docs/src/crates/bench.md:134`
   - Evidence (implementation): `crates/vfx-bench/benches/vfx_bench.rs:1`
   - Impact: example cannot be run as-is.

252) vfx-bench docs list `vfx-io`/`vfx-ops` dependencies and `io_bench`/`resize_bench` targets, but the crate depends on `vfx-core`/`vfx-math`/`vfx-lut`/`vfx-transfer`/`vfx-color` and only defines `vfx_bench`.
   - Evidence (doc claim): `docs/src/crates/bench.md:204`, `docs/src/crates/bench.md:205`, `docs/src/crates/bench.md:211`, `docs/src/crates/bench.md:215`
   - Evidence (implementation): `crates/vfx-bench/Cargo.toml:11`, `crates/vfx-bench/Cargo.toml:12`, `crates/vfx-bench/Cargo.toml:13`, `crates/vfx-bench/Cargo.toml:14`, `crates/vfx-bench/Cargo.toml:15`, `crates/vfx-bench/Cargo.toml:21`
   - Impact: dependency and bench configuration guidance is incorrect.

253) vfx-math docs call `adapt_matrix(D65, D60, &BRADFORD)`, but the function signature is `adapt_matrix(method, src_white, dst_white)` and takes `Mat3` by value.
   - Evidence (doc claim): `docs/src/crates/math.md:55`, `docs/src/crates/math.md:58`
   - Evidence (implementation): `crates/vfx-math/src/adapt.rs:175`
   - Impact: example code does not compile and misstates parameter order.

254) vfx-math docs list a `DCI` white point constant, but the constant is named `DCI_WHITE`.
   - Evidence (doc claim): `docs/src/crates/math.md:73`
   - Evidence (implementation): `crates/vfx-math/src/adapt.rs:76`, `crates/vfx-math/src/adapt.rs:77`
   - Impact: example code does not compile.

255) vfx-math docs reference `catmull_rom`, but no such function exists in the interpolation module.
   - Evidence (doc claim): `docs/src/crates/math.md:78`, `docs/src/crates/math.md:87`
   - Evidence (implementation): `crates/vfx-math/src/interp.rs` (no `catmull_rom` symbol)
   - Impact: example code does not compile.

256) vfx-math docs reference `process_rgba_f32x8` and `apply_matrix_simd`, but those functions are not exposed in `vfx_math::simd`.
   - Evidence (doc claim): `docs/src/crates/math.md:95`, `docs/src/crates/math.md:98`
   - Evidence (implementation): `crates/vfx-math/src/simd.rs` (no such symbols)
   - Impact: example code does not compile.

257) vfx-math docs reference `rgb_to_luminance` and `linearize_srgb`, but `vfx-math` does not export these functions.
   - Evidence (doc claim): `docs/src/crates/math.md:109`, `docs/src/crates/math.md:112`
   - Evidence (implementation): `crates/vfx-math/src/lib.rs:50`
   - Impact: example code does not compile and points users to the wrong crate.

258) Introduction project structure claims `vfx-core` provides `ImageData`, but `ImageData` is defined in `vfx-io`, not `vfx-core`.
   - Evidence (doc claim): `docs/src/introduction.md:60`
   - Evidence (implementation): `crates/vfx-core/src/lib.rs:1`, `crates/vfx-io/src/lib.rs:181`
   - Impact: users look for `ImageData` in the wrong crate.

259) Introduction project structure lists `Mat4` in `vfx-math`, but there is no `Mat4` type in that crate.
   - Evidence (doc claim): `docs/src/introduction.md:61`
   - Evidence (implementation): `crates/vfx-math/src/lib.rs:27`
   - Impact: example expectations do not match the available math types.

260) Introduction links to `plan3.md` as the Bug Hunt report, but the file is not present in the repo.
   - Evidence (doc claim): `docs/src/introduction.md:98`
   - Evidence (implementation): `plan3.md` (not found)
   - Impact: broken documentation link.

261) Feature flags doc omits several `vfx-io` features (`text`, `rayon`, `psd`, `dds`, `ktx`) that exist in the crate.
   - Evidence (doc claim): `docs/src/installation/features.md:5`
   - Evidence (implementation): `crates/vfx-io/Cargo.toml:14`, `crates/vfx-io/Cargo.toml:23`, `crates/vfx-io/Cargo.toml:33`, `crates/vfx-io/Cargo.toml:36`, `crates/vfx-io/Cargo.toml:39`
   - Impact: documentation understates available build options.

262) Quick Start uses an outdated clone URL (`philipc/vfx-rs`), but the workspace repository points to `vfx-rs/vfx-rs`.
   - Evidence (doc claim): `docs/src/user-guide/quick-start.md:16`
   - Evidence (implementation): `Cargo.toml:28`
   - Impact: users may clone the wrong repository.

263) Quick Start resize example uses `-h` for height, but the CLI defines height as `-H` (uppercase).
   - Evidence (doc claim): `docs/src/user-guide/quick-start.md:55`
   - Evidence (implementation): `crates/vfx-cli/src/main.rs:305`
   - Impact: the example fails because `-h` is reserved for help.

264) Python docs list `img.format` as `f32|f16|u16|u8`, but bindings can return `u32`.
   - Evidence (doc claim): `docs/src/python.md:56`
   - Evidence (implementation): `crates/vfx-rs-py/src/image.rs:83`
   - Impact: format list is incomplete.

265) Python docs claim `img.numpy()` returns a view when possible, but bindings always allocate via `to_f32` regardless of `copy`.
   - Evidence (doc claim): `docs/src/python.md:946`
   - Evidence (implementation): `crates/vfx-rs-py/src/image.rs:95`
   - Impact: performance guidance is inaccurate; copies always occur.

266) Python docs import OCIO helpers (`ColorConfig`, `Context`, `colorconvert`, `ociodisplay`, `ociolook`, `ociofiletransform`) from the top-level `vfx_rs`, but they are only exposed under the `vfx_rs.ocio` submodule.
   - Evidence (doc claim): `docs/src/python.md:488`, `docs/src/python.md:532`, `docs/src/python.md:535`
   - Evidence (implementation): `crates/vfx-rs-py/src/lib.rs:124`, `crates/vfx-rs-py/src/ocio.rs:864`
   - Impact: imports fail as written.

267) Python docs reference `GpuProcessor`/`GpuLanguage` from top-level `vfx_rs`, but the advanced OCIO module is not registered, so these classes are not exposed.
   - Evidence (doc claim): `docs/src/python.md:513`
   - Evidence (implementation): `crates/vfx-rs-py/src/lib.rs:19`, `crates/vfx-rs-py/src/lib.rs:124`
   - Impact: GPU shader examples cannot be executed from the published API.

268) Dev benchmarks doc suggests `cargo bench -p vfx-bench -- resize` and `cargo flamegraph --bench vfx-bench -- resize`, but the only bench target is `vfx_bench`.
   - Evidence (doc claim): `docs/src/dev/benchmarks.md:12`, `docs/src/dev/benchmarks.md:200`
   - Evidence (implementation): `crates/vfx-bench/Cargo.toml:20`, `crates/vfx-bench/Cargo.toml:21`
   - Impact: commands do not select any benchmark.

269) Dev benchmarks doc includes IO/resize/color examples (`bench_exr_load`, `resize_f32`, `apply_srgb_to_linear`) but vfx-bench has no vfx-io/vfx-ops deps and `apply_srgb_to_linear` is not defined anywhere.
   - Evidence (doc claim): `docs/src/dev/benchmarks.md:28`, `docs/src/dev/benchmarks.md:53`, `docs/src/dev/benchmarks.md:75`
   - Evidence (implementation): `crates/vfx-bench/Cargo.toml:11`, `crates/vfx-bench/Cargo.toml:12`, `crates/vfx-bench/Cargo.toml:13`
   - Impact: example code does not compile in the bench crate.

270) Dev benchmarks doc uses `Lut3D::load` and `apply_trilinear`/`apply_tetrahedral`, but these methods are not public APIs.
   - Evidence (doc claim): `docs/src/dev/benchmarks.md:85`, `docs/src/dev/benchmarks.md:91`, `docs/src/dev/benchmarks.md:95`
   - Evidence (implementation): `crates/vfx-lut/src/lut3d.rs:175`, `crates/vfx-lut/src/lut3d.rs:225`
   - Impact: LUT benchmark example does not compile.

271) Dev testing doc describes a `crates/vfx-tests/tests` layout and `test/images` asset paths that do not exist.
   - Evidence (doc claim): `docs/src/dev/testing.md:53`, `docs/src/dev/testing.md:112`
   - Evidence (implementation): `crates/vfx-tests/src/lib.rs:1`, `test/` (no `test/images`)
   - Impact: guidance points to non-existent test structure and assets.

272) Dev testing doc uses `vfx convert --resize`, but `convert` has no `--resize` option (resize is a separate command).
   - Evidence (doc claim): `docs/src/dev/testing.md:246`
   - Evidence (implementation): `crates/vfx-cli/src/main.rs:271`
   - Impact: example command fails.

273) Adding-formats guide pins `tiff = "0.9"` and sets default features to `exr,png`, but the crate uses `tiff = "0.10"` and defaults include `jpeg,tiff,dpx,hdr`.
   - Evidence (doc claim): `docs/src/dev/adding-formats.md:30`, `docs/src/dev/adding-formats.md:33`
   - Evidence (implementation): `crates/vfx-io/Cargo.toml:11`, `crates/vfx-io/Cargo.toml:76`
   - Impact: instructions drift from actual dependency and default feature set.

274) Internals README claims each crate has a `tests/` directory, but at least `vfx-math` has none.
   - Evidence (doc claim): `docs/src/internals/README.md:28`
   - Evidence (implementation): `crates/vfx-math/` (no `tests` dir)
   - Impact: internal structure description is inaccurate.

275) Internals README shows feature flags `default = ["exr", "png"]`, `gpu`, and `all-formats`, but these features are not defined for vfx-io.
   - Evidence (doc claim): `docs/src/internals/README.md:123`, `docs/src/internals/README.md:124`, `docs/src/internals/README.md:125`
   - Evidence (implementation): `crates/vfx-io/Cargo.toml:11`
   - Impact: feature flag guidance is incorrect.

276) Pipeline internals doc uses `exr::write_with_options` and `ExrWriteOptions`, but the API exposes `ExrWriter` with `ExrWriterOptions` instead.
   - Evidence (doc claim): `docs/src/internals/pipeline.md:149`
   - Evidence (implementation): `crates/vfx-io/src/exr.rs:40`, `crates/vfx-io/src/exr.rs:157`
   - Impact: example code does not compile.

277) Color internals doc uses `Mat3::from_cols(r, g, b)` and `Mat3::from_diagonal`, but the API provides `Mat3::from_col_vecs` and `Mat3::diagonal`.
   - Evidence (doc claim): `docs/src/internals/color.md:101`, `docs/src/internals/color.md:141`
   - Evidence (implementation): `crates/vfx-math/src/mat3.rs:112`, `crates/vfx-math/src/mat3.rs:126`
   - Impact: matrix construction examples do not compile.

278) Color internals doc defines `adapt_matrix(src_white, dst_white, cone: &Mat3)`, but the API signature is `adapt_matrix(method, src_white, dst_white)`.
   - Evidence (doc claim): `docs/src/internals/color.md:132`
   - Evidence (implementation): `crates/vfx-math/src/adapt.rs:175`
   - Impact: example code does not compile and parameter order is wrong.

279) EXR internals doc uses `write_with_options`/`ExrWriteOptions` and describes the `exr` crate directly, but the API exposes `ExrWriter` + `ExrWriterOptions` in vfx-io.
   - Evidence (doc claim): `docs/src/internals/exr.md:7`, `docs/src/internals/exr.md:116`
   - Evidence (implementation): `crates/vfx-io/src/exr.rs:40`, `crates/vfx-io/src/exr.rs:157`
   - Impact: examples and dependency description are out of date.

280) ACES vfx-rs guide references `apply_aces_idt`, `apply_aces_rrt_odt`, `apply_idt`, and `apply_rrt_odt`, but the ACES API only exposes `apply_rrt_odt_srgb` and related helpers.
   - Evidence (doc claim): `docs/src/aces/vfx-rs-aces.md:147`, `docs/src/aces/vfx-rs-aces.md:159`, `docs/src/aces/vfx-rs-aces.md:165`, `docs/src/aces/vfx-rs-aces.md:170`
   - Evidence (implementation): `crates/vfx-color/src/aces.rs:224`
   - Impact: example code does not compile.

281) ACES IDT doc uses `linearize_srgb`, but no such function exists in vfx-color.
   - Evidence (doc claim): `docs/src/aces/idt.md:28`, `docs/src/aces/idt.md:31`
   - Evidence (implementation): `crates/vfx-color/src/aces.rs:203`
   - Impact: example code does not compile.

282) ACEScg doc uses `srgb_to_acescg([r,g,b])` and `Primaries::SRGB`/`Primaries::ACES_AP1`, but the API expects `srgb_to_acescg(r, g, b)` and primaries are module-level constants (`SRGB`, `ACES_AP1`).
   - Evidence (doc claim): `docs/src/aces/acescg.md:115`, `docs/src/aces/acescg.md:119`, `docs/src/aces/acescg.md:124`
   - Evidence (implementation): `crates/vfx-color/src/aces.rs:203`, `crates/vfx-primaries/src/lib.rs:119`
   - Impact: example code does not compile.

283) ACES transfer-functions doc uses `linear_to_*` / `*_to_linear` helpers that do not exist; vfx-transfer exports `*_encode`/`*_decode` and `*_eotf`/`*_oetf`.
   - Evidence (doc claim): `docs/src/aces/transfer-functions.md:63`, `docs/src/aces/transfer-functions.md:94`, `docs/src/aces/transfer-functions.md:116`, `docs/src/aces/transfer-functions.md:170`
   - Evidence (implementation): `crates/vfx-transfer/src/lib.rs:62`, `crates/vfx-transfer/src/lib.rs:68`, `crates/vfx-transfer/src/lib.rs:80`, `crates/vfx-transfer/src/lib.rs:88`
   - Impact: sample code does not compile; users see wrong function names.

284) ACES examples use `vfx batch --op aces` and pass `transform=...`, but batch only supports convert/resize/blur/flip_h/flip_v.
   - Evidence (doc claim): `docs/src/aces/examples.md:151`, `docs/src/aces/examples.md:159`
   - Evidence (implementation): `crates/vfx-cli/src/commands/batch.rs:78`
   - Impact: examples fail.

285) ACES examples use `vfx aces ... --layer`, but the `aces` command has no `--layer` argument.
   - Evidence (doc claim): `docs/src/aces/examples.md:199`
   - Evidence (implementation): `crates/vfx-cli/src/main.rs:730`
   - Impact: example fails with unknown flag.

286) LMT doc uses `vfx color ... --look`, but the CLI has no `--look` option on the color command.
   - Evidence (doc claim): `docs/src/aces/lmt.md:63`
   - Evidence (implementation): `crates/vfx-cli/src/main.rs:438`
   - Impact: example fails; look application is not available via CLI flag.

287) ACES pipeline doc suggests `vfx aces ... --odt`, but the `aces` command has no `--odt` option.
   - Evidence (doc claim): `docs/src/aces/pipeline.md:110`
   - Evidence (implementation): `crates/vfx-cli/src/main.rs:730`
   - Impact: example fails with unknown flag.

288) ACES color-spaces doc uses `Primaries::ACES_AP0` and `Primaries::ACES_AP1`, but primaries are module-level constants (`ACES_AP0`, `ACES_AP1`).
   - Evidence (doc claim): `docs/src/aces/color-spaces.md:29`, `docs/src/aces/color-spaces.md:59`
   - Evidence (implementation): `crates/vfx-primaries/src/lib.rs:189`, `crates/vfx-primaries/src/lib.rs:201`
   - Impact: example code does not compile.

289) Installation doc lists an `icc` feature with `lcms2`, but there is no `icc` feature flag in the workspace or vfx-io feature list.
   - Evidence (doc claim): `docs/src/installation.md:14`
   - Evidence (implementation): `crates/vfx-io/Cargo.toml:10`, `crates/vfx-io/Cargo.toml:32`
   - Impact: users look for a non-existent feature flag.

290) Programmer guide uses `vfx_core::ImageData`, but `ImageData` is defined in vfx-io, not vfx-core.
   - Evidence (doc claim): `docs/src/programmer/README.md:68`, `docs/src/programmer/README.md:70`, `docs/src/programmer/README.md:112`
   - Evidence (implementation): `crates/vfx-io/src/lib.rs:537`
   - Impact: example code fails to compile.

291) Programmer guide references `ChannelType`, but no such type exists; the closest type is `ChannelKind` in vfx-io.
   - Evidence (doc claim): `docs/src/programmer/README.md:95`, `docs/src/programmer/README.md:112`
   - Evidence (implementation): `crates/vfx-io/src/lib.rs:553`
   - Impact: example code fails to compile and misleads API consumers.

292) Programmer guide shows `use vfx_ops::resize;` and calls `resize(&image, ...)`, but there is no `resize` function taking image objects; the available API is `resize_f32`/layer-level helpers.
   - Evidence (doc claim): `docs/src/programmer/README.md:45`, `docs/src/programmer/README.md:52`
   - Evidence (implementation): `crates/vfx-ops/src/resize.rs:142`
   - Impact: example code fails to compile.

293) Internals GPU doc describes a `GpuPrimitives` trait with in-place `exec_exposure`/`exec_matrix` and `limits() -> GpuLimits`, but the actual trait has no `exec_exposure`, uses src/dst handles for `exec_matrix`/`exec_cdl`/`exec_lut*`, and returns `&GpuLimits`.
   - Evidence (doc claim): `docs/src/internals/gpu.md:47`, `docs/src/internals/gpu.md:48`, `docs/src/internals/gpu.md:43`
   - Evidence (implementation): `crates/vfx-compute/src/backend/gpu_primitives.rs:54`, `crates/vfx-compute/src/backend/gpu_primitives.rs:66`, `crates/vfx-compute/src/backend/gpu_primitives.rs:111`
   - Impact: internal API documentation is out of sync; code samples won’t compile.

294) Internals GPU doc uses `byte_size()` on `ImageHandle` and names `CpuHandle`/`WgpuHandle`/`CudaHandle`, but actual API uses `size_bytes()` and handle structs are `CpuImage`, `WgpuImage`, `CudaImage`.
   - Evidence (doc claim): `docs/src/internals/gpu.md:75`, `docs/src/internals/gpu.md:83`, `docs/src/internals/gpu.md:91`, `docs/src/internals/gpu.md:100`
   - Evidence (implementation): `crates/vfx-compute/src/backend/gpu_primitives.rs:15`, `crates/vfx-compute/src/backend/cpu_backend.rs:12`, `crates/vfx-compute/src/backend/wgpu_backend.rs:88`, `crates/vfx-compute/src/backend/cuda_backend.rs:621`
   - Impact: examples and type names don’t match code.

295) Internals GPU doc shows `TiledExecutor` with `tile_size` and an `execute_tiled` API, but the actual executor uses config/planner/cache and does not expose `execute_tiled`.
   - Evidence (doc claim): `docs/src/internals/gpu.md:113`, `docs/src/internals/gpu.md:130`
   - Evidence (implementation): `crates/vfx-compute/src/backend/executor.rs:211`, `crates/vfx-compute/src/backend/executor.rs:237`
   - Impact: internal flow description misleads about available APIs.

296) Internals GPU doc suggests `processor.apply_batch(&mut img, &batch)`, but compute processor exposes `apply_color_ops`.
   - Evidence (doc claim): `docs/src/internals/gpu.md:605`
   - Evidence (implementation): `crates/vfx-compute/src/processor.rs:633`
   - Impact: example code fails to compile.

297) ODT doc instructs using `vfx color --from/--to` for broadcast/HDR conversions, but the color command ignores these options.
   - Evidence (doc claim): `docs/src/aces/odt.md:80`, `docs/src/aces/odt.md:90`, `docs/src/aces/odt.md:175`, `docs/src/aces/odt.md:183`, `docs/src/aces/odt.md:193`
   - Evidence (implementation): `crates/vfx-cli/src/main.rs:438`, `crates/vfx-cli/src/commands/color.rs:18`
   - Impact: documented ODT workflows do nothing.

298) ODT doc lists many HDR/SDR ODTs (Rec.2020 PQ/HLG, P3-D65, D60 sim), but the code only provides sRGB and Rec.709 ODT helpers, and CLI uses sRGB-only ODT.
   - Evidence (doc claim): `docs/src/aces/odt.md:58`, `docs/src/aces/odt.md:60`, `docs/src/aces/odt.md:62`, `docs/src/aces/odt.md:63`, `docs/src/aces/odt.md:64`
   - Evidence (implementation): `crates/vfx-color/src/aces.rs:161`, `crates/vfx-color/src/aces.rs:179`, `crates/vfx-cli/src/commands/aces.rs:12`
   - Impact: docs overstate available ODT options.

299) Appendix formats table marks JPEG CMYK as unsupported, but the JPEG reader accepts CMYK and converts to RGB.
   - Evidence (doc claim): `docs/src/appendix/formats.md:61`
   - Evidence (implementation): `crates/vfx-io/src/jpeg.rs:206`
   - Impact: documentation understates actual CMYK input handling.

300) Appendix formats notes “16-bit output when source is float” for PNG, but PNG writer defaults to 8-bit unless options explicitly set.
   - Evidence (doc claim): `docs/src/appendix/formats.md:49`
   - Evidence (implementation): `crates/vfx-io/src/png.rs:168`, `crates/vfx-io/src/png.rs:498`
   - Impact: users may expect automatic 16-bit output that doesn’t occur.

301) Appendix formats table marks PSD layers as unsupported, but the PSD module exposes `read_layers` and layer metadata.
   - Evidence (doc claim): `docs/src/appendix/formats.md:102`
   - Evidence (implementation): `crates/vfx-io/src/psd.rs:109`
   - Impact: documentation understates PSD layer capabilities.

302) Appendix formats table claims PSD supports 8/16-bit, but the PSD implementation only exposes 8-bit RGBA output (u8 conversion).
   - Evidence (doc claim): `docs/src/appendix/formats.md:103`
   - Evidence (implementation): `crates/vfx-io/src/psd.rs:8`, `crates/vfx-io/src/psd.rs:86`
   - Impact: 16-bit PSD files are not represented at full precision.

303) Appendix formats table marks JPEG progressive as supported, but the JPEG writer always uses baseline encoding (no progressive option exposed).
   - Evidence (doc claim): `docs/src/appendix/formats.md:60`
   - Evidence (implementation): `crates/vfx-io/src/jpeg.rs:562`
   - Impact: users expect progressive JPEG output that is not implemented.

304) Feature matrix claims TIFF tile support, but the TIFF writer exposes only bit depth/compression options and has no tile path.
   - Evidence (doc claim): `docs/src/appendix/feature-matrix.md:170`
   - Evidence (implementation): `crates/vfx-io/src/tiff.rs:170`
   - Impact: tiled TIFF output is not available despite documentation.

305) Appendix formats table marks TIFF multi-page support as “partial”, but the reader has no page selection and reserves page support for future use.
   - Evidence (doc claim): `docs/src/appendix/formats.md:78`
   - Evidence (implementation): `crates/vfx-io/src/tiff.rs:145`
   - Impact: multi-page TIFFs are effectively treated as single-page without an API to access other pages.

306) `vfx convert` detects output format via `Format::detect` on the output path; if the file does not exist yet, detection fails and EXR-to-EXR layered preservation is skipped.
   - Evidence (implementation): `crates/vfx-cli/src/commands/convert.rs:19`, `crates/vfx-cli/src/commands/convert.rs:26`, `crates/vfx-cli/src/commands/convert.rs:43`
   - Impact: converting EXR to EXR loses layers unless the output file already exists.

307) HEIF docs pass `Some(&hdr_info)` where the API expects `Option<&HdrMetadata>`; example also moves `hdr_info` in `if let` making the later use invalid.
   - Evidence (doc claim): `docs/src/crates/io.md:99`, `docs/src/crates/io.md:101`, `docs/src/crates/io.md:107`
   - Evidence (implementation): `crates/vfx-io/src/heif.rs:392`
   - Impact: example code does not compile as written.

308) `vfx convert` docs say DPX-to-EXR converts 10-bit log to linear, but the DPX reader only labels colorspace as log and does not apply a log-to-linear transform.
   - Evidence (doc claim): `docs/src/cli/convert.md:30`
   - Evidence (implementation): `crates/vfx-io/src/dpx.rs:600`
   - Impact: conversion output remains log-encoded unless the user applies an explicit transform.

309) `vfx convert` docs omit the `-q/--quality` option even though the CLI exposes JPEG quality control.
   - Evidence (doc claim): `docs/src/cli/convert.md:10`
   - Evidence (implementation): `crates/vfx-cli/src/main.rs:284`
   - Impact: users can’t discover the quality flag from docs.

310) `layers` CLI docs describe subcommands (`vfx layers list/extract/merge`), but the CLI exposes separate commands (`layers`, `extract-layer`, `merge-layers`).
   - Evidence (doc claim): `docs/src/cli/layers.md:7`
   - Evidence (implementation): `crates/vfx-cli/src/main.rs:201`, `crates/vfx-cli/src/main.rs:205`, `crates/vfx-cli/src/main.rs:209`
   - Impact: users will run non-existent subcommands.

311) `extract-layer` docs claim that omitting `--layer` extracts the first layer, but the implementation only prints available layers and exits without writing output.
   - Evidence (doc claim): `docs/src/cli/extract-layer.md:37`
   - Evidence (implementation): `crates/vfx-cli/src/commands/layers.rs:128`
   - Impact: documented default behavior does not occur; scripts expecting output will fail.

312) `merge-layers` docs say `--names` is comma-separated, but the CLI expects repeated `-n/--names` values (Vec<String>) and does not split commas.
   - Evidence (doc claim): `docs/src/cli/merge-layers.md:16`
   - Evidence (implementation): `crates/vfx-cli/src/main.rs:620`, `crates/vfx-cli/src/commands/layers.rs:200`
   - Impact: comma-separated names are treated as a single layer name.

313) `channel-extract` docs instruct comma-separated channel lists, but the CLI treats each `-c/--channels` value as a separate entry and does not split on commas.
   - Evidence (doc claim): `docs/src/cli/channel-extract.md:21`
   - Evidence (implementation): `crates/vfx-cli/src/commands/channels.rs:131`
   - Impact: `-c R,G,B` is interpreted as a single unknown channel spec.

314) `channel-extract` docs claim support for custom channel names (e.g., `N.x`, `beauty.R`), but the parser only accepts R/G/B/A/Z or numeric indices.
   - Evidence (doc claim): `docs/src/cli/channel-extract.md:17`, `docs/src/cli/channel-extract.md:42`
   - Evidence (implementation): `crates/vfx-cli/src/commands/channels.rs:152`
   - Impact: users cannot extract arbitrary EXR channel names as documented.

315) `udim convert` docs suggest EXR compression selection via `-c`, but the implementation only stores a `compression` metadata attr and does not route it into the EXR writer options.
   - Evidence (doc claim): `docs/src/cli/udim.md:63`
   - Evidence (implementation): `crates/vfx-cli/src/commands/udim.rs:78`
   - Impact: requested compression is ignored; output uses default writer settings.

316) `udim info` docs show per-tile bit depth/format details ("half float") but the implementation only prints dimensions/channels and path in verbose mode.
   - Evidence (doc claim): `docs/src/cli/udim.md:33`
   - Evidence (implementation): `crates/vfx-cli/src/commands/udim.rs:47`
   - Impact: docs overstate available UDIM info output.

317) `transform` docs claim EXR layers are transformed together, but the command loads a single image (not layered) and writes a flat output.
   - Evidence (doc claim): `docs/src/cli/transform.md:116`
   - Evidence (implementation): `crates/vfx-cli/src/commands/transform.rs:10`
   - Impact: multi-layer EXR data is not preserved as documented.

318) `composite` docs list many blend modes (overlay/softlight/hardlight/difference/etc.), but the CLI only supports over/add/multiply/screen.
   - Evidence (doc claim): `docs/src/cli/composite.md:20`
   - Evidence (implementation): `crates/vfx-cli/src/commands/composite.rs:30`
   - Impact: documented blend modes are unavailable.

319) `composite` docs claim GPU acceleration, but the CLI path uses CPU-only vfx-ops composite functions.
   - Evidence (doc claim): `docs/src/cli/composite.md:48`
   - Evidence (implementation): `crates/vfx-cli/src/commands/composite.rs:6`
   - Impact: users expect GPU usage that does not occur.

320) `warp` docs describe wave/ripple parameters as `k1=frequency` and `k2=amplitude`, but the implementation passes `k1` as amplitude and `k2` as frequency.
   - Evidence (doc claim): `docs/src/cli/warp.md:66`, `docs/src/cli/warp.md:78`
   - Evidence (implementation): `crates/vfx-cli/src/commands/warp.rs:45`, `crates/vfx-ops/src/warp.rs:140`
   - Impact: CLI parameter semantics are inverted relative to docs.

321) `warp` docs say k2 default is 0.0, but the CLI clamps k2 to at least 1.0 for wave/ripple.
   - Evidence (doc claim): `docs/src/cli/warp.md:16`
   - Evidence (implementation): `crates/vfx-cli/src/commands/warp.rs:45`
   - Impact: wave/ripple never run with k2 < 1.0 despite doc default.

322) `warp` docs claim parallel processing via rayon, but warp implementation is a single-threaded loop with no rayon usage.
   - Evidence (doc claim): `docs/src/cli/warp.md:121`
   - Evidence (implementation): `crates/vfx-ops/src/warp.rs:36`
   - Impact: performance expectations are overstated.

323) CLI README claims `--threads` global flag, but the CLI has no threads option.
   - Evidence (doc claim): `docs/src/cli/README.md:9`
   - Evidence (implementation): `crates/vfx-cli/src/main.rs:32`
   - Impact: documented global option does not exist.

324) CLI README lists alias `c` for `convert`, but the CLI defines no alias for convert.
   - Evidence (doc claim): `docs/src/cli/README.md:27`
   - Evidence (implementation): `crates/vfx-cli/src/main.rs:155`
   - Impact: users will try `vfx c` and get an unknown command.

325) CLI README lists alias `r` for `resize`, but the CLI defines no alias for resize.
   - Evidence (doc claim): `docs/src/cli/README.md:36`
   - Evidence (implementation): `crates/vfx-cli/src/main.rs:166`
   - Impact: `vfx r` does not work.

326) CLI README lists alias `v` for `view`, but the CLI defines no alias for view.
   - Evidence (doc claim): `docs/src/cli/README.md:68`
   - Evidence (implementation): `crates/vfx-cli/src/main.rs:237`
   - Impact: `vfx v` does not work.

327) `resize` docs list filters `nearest`/`bilinear`/`bicubic`/`lanczos`, but CLI `--filter` expects `box`/`bilinear`/`lanczos`/`mitchell`.
   - Evidence (doc claim): `docs/src/cli/resize.md:14`
   - Evidence (implementation): `crates/vfx-cli/src/main.rs:300`
   - Impact: documented filter names don’t match CLI help.

328) `resize` docs claim GPU acceleration, but the CLI uses CPU-only `vfx_ops::resize`.
   - Evidence (doc claim): `docs/src/cli/resize.md:41`
   - Evidence (implementation): `crates/vfx-cli/src/commands/resize.rs:9`
   - Impact: users expect GPU acceleration that does not occur.

329) `blur` docs claim blur uses rayon, but the CLI calls CPU blur without any parallelism in this path.
   - Evidence (doc claim): `docs/src/cli/blur.md:92`
   - Evidence (implementation): `crates/vfx-cli/src/commands/blur.rs:8`
   - Impact: performance expectations are overstated.

330) `blur` docs say gaussian uses separable implementation, but the CLI uses full 2D convolution with a generated kernel.
   - Evidence (doc claim): `docs/src/cli/blur.md:87`
   - Evidence (implementation): `crates/vfx-cli/src/commands/blur.rs:28`
   - Impact: performance characteristics differ from docs.

331) `paste` docs imply any pixel format and background color space preservation, but the CLI always converts to f32 and returns an f32 image.
   - Evidence (doc claim): `docs/src/cli/paste.md:128`
   - Evidence (implementation): `crates/vfx-cli/src/commands/paste.rs:37`
   - Impact: bit depth/format is not preserved as documented.

332) ImageBufAlgo README claims full OIIO API coverage and shows in-place functions (`add_constant`, `resize(&mut ...)`, `blur_inplace`), but the module exports different APIs (e.g., `add`, `resize`, `blur_into`) and does not expose those in-place names.
   - Evidence (doc claim): `docs/src/programmer/imagebufalgo/README.md:3`, `docs/src/programmer/imagebufalgo/README.md:23`, `docs/src/programmer/imagebufalgo/README.md:35`
   - Evidence (implementation): `crates/vfx-io/src/imagebufalgo/mod.rs:48`
   - Impact: examples do not compile and coverage is overstated.

333) `InitializePixels::No` claims uninitialized pixel memory, but `PixelStorage::allocate` always zero-initializes buffers regardless of the flag.
   - Evidence (doc claim): `crates/vfx-io/src/imagebuf/mod.rs:36`
   - Evidence (implementation): `crates/vfx-io/src/imagebuf/storage.rs:40`
   - Impact: callers cannot get uninitialized buffers as documented.

334) `ScanlineIterator` claims to iterate over the ROI, but when advancing z it resets `y` to 0 instead of `roi.ybegin`, breaking non-zero ROI origins.
   - Evidence (doc claim): `crates/vfx-io/src/imagebuf/iterators.rs:63`
   - Evidence (implementation): `crates/vfx-io/src/imagebuf/iterators.rs:117`
   - Impact: scanline iteration yields rows outside the ROI for z>0.

335) Streaming format docs reference `native_bpp` and `should_use_streaming`, but the module exposes `bytes_per_pixel` and `should_stream` instead.
   - Evidence (doc claim): `crates/vfx-io/src/streaming/format.rs:8`, `crates/vfx-io/src/streaming/format.rs:10`
   - Evidence (implementation): `crates/vfx-io/src/streaming/format.rs:139`, `crates/vfx-io/src/streaming/format.rs:170`
   - Impact: doc examples do not compile and APIs are misnamed.

336) `estimate_memory` claims to read only header bytes, but it loads the entire file into memory via `std::fs::read`.
   - Evidence (doc claim): `crates/vfx-io/src/streaming/format.rs:264`
   - Evidence (implementation): `crates/vfx-io/src/streaming/format.rs:283`
   - Impact: large files can be fully loaded despite “header-only” intent.

337) Streaming module docs claim EXR true streaming, but `ExrStreamingSource` only supports random access for tiled EXR; scanline EXR falls back to full-image cache.
   - Evidence (doc claim): `crates/vfx-io/src/streaming/mod.rs:77`
   - Evidence (implementation): `crates/vfx-io/src/streaming/exr.rs:188`
   - Impact: documentation overstates EXR streaming capability.

338) `TiffStreamingSource::read_region` clamps `x`/`y` to the last pixel, so a region fully outside bounds returns edge pixels instead of transparent black.
   - Evidence (contract): `crates/vfx-io/src/streaming/traits.rs:217`
   - Evidence (implementation): `crates/vfx-io/src/streaming/tiff.rs:352`
   - Impact: out-of-bounds reads violate `StreamingSource` contract and can leak edge pixels.

339) DeepData capacity management diverges from the OIIO reference: once allocated, `set_capacity` and `set_samples` do not reallocate or move data, and `insert_samples` silently returns if capacity is insufficient. This can leave `nsamples` larger than allocated storage and break split/merge paths.
   - Evidence (reference behavior): `_ref/OpenImageIO/src/libOpenImageIO/deepdata.cpp:506`, `_ref/OpenImageIO/src/libOpenImageIO/deepdata.cpp:536`, `_ref/OpenImageIO/src/libOpenImageIO/deepdata.cpp:591`
   - Evidence (implementation): `crates/vfx-io/src/deepdata.rs:337`, `crates/vfx-io/src/deepdata.rs:363`, `crates/vfx-io/src/deepdata.rs:379`, `crates/vfx-io/src/deepdata.rs:409`
   - Impact: sample insertion/merge can become no-ops or lead to out-of-bounds access in subsequent writes.

340) DDS module docs claim support for cube maps/texture arrays, but `read()`/`read_all_mips()` decode a flat surface and only return the first width*height layer, dropping additional faces/layers.
   - Evidence (doc claim): `crates/vfx-io/src/dds.rs:9`
   - Evidence (implementation): `crates/vfx-io/src/dds.rs:199`, `crates/vfx-io/src/dds.rs:223`
   - Impact: cubemap/array DDS reads return incomplete data without warning.

341) `probe_dimensions` docs say TIFF reads only IFD tags, but implementation does a full TIFF decode via `tiff::read`.
   - Evidence (doc claim): `crates/vfx-io/src/lib.rs:368`
   - Evidence (implementation): `crates/vfx-io/src/lib.rs:495`
   - Impact: dimension probing loads full TIFF image data, negating performance guarantees for large files.

342) Feature matrix marks CinemaDNG as "Done", but implementation is a thin wrapper over generic TIFF decoding; no DNG-specific tags/RAW/CFA handling is present.
   - Evidence (doc claim): `docs/src/appendix/feature-matrix.md:189`
   - Evidence (implementation): `crates/vfx-io/src/cinema_dng.rs:106`, `crates/vfx-io/src/cinema_dng.rs:256`
   - Impact: CinemaDNG support is limited to whatever the TIFF reader returns, with DNG metadata and raw semantics ignored.

343) Texture module claims anisotropic filtering, but `FilterMode::Anisotropic` is implemented as trilinear without any anisotropic footprint handling.
   - Evidence (doc claim): `crates/vfx-io/src/texture.rs:4`
   - Evidence (implementation): `crates/vfx-io/src/texture.rs:120`
   - Impact: anisotropic sampling is not actually implemented; quality expectations are not met.

344) `TextureSystem::sample` accepts `FilterMode::Trilinear`, but without derivatives it always falls back to bilinear at mip 0.
   - Evidence (doc claim): `crates/vfx-io/src/texture.rs:4`
   - Evidence (implementation): `crates/vfx-io/src/texture.rs:116`
   - Impact: callers selecting trilinear in `sample` do not get mip blending.

345) Feature matrix claims Gaussian blur is separable, but the only Gaussian implementation is a full 2D kernel used with `convolve`.
   - Evidence (doc claim): `docs/src/appendix/feature-matrix.md:293`
   - Evidence (implementation): `crates/vfx-ops/src/filter.rs:65`, `crates/vfx-ops/src/filter.rs:178`
   - Impact: performance characteristics are overstated; Gaussian blur is not separable in vfx-ops.

346) vfx-color `ColorProcessor` docs обещают кэширование и предвычисление LUT/transfer, но в реализации нет ни кэша, ни LUT-precompute — только линейный проход и опциональная оптимизация матриц/scale/offset.
   - Evidence (doc claim): `crates/vfx-color/src/processor.rs:1`, `crates/vfx-color/src/processor.rs:4`, `crates/vfx-color/src/processor.rs:41`, `crates/vfx-color/src/processor.rs:49`
   - Evidence (implementation): `crates/vfx-color/src/processor.rs:137`, `crates/vfx-color/src/processor.rs:363`
   - Impact: ожидания производительности/поведения расходятся с фактом; API не предоставляет заявленных оптимизаций.

347) Документация по цветовым примариям использует `Primaries::SRGB`/`Primaries::ACES_AP1` и т.п., но в API это свободные константы `vfx_primaries::SRGB`, `vfx_primaries::ACES_AP1` и т.д.; ассоциированных констант в `Primaries` нет.
   - Evidence (doc claim): `docs/src/programmer/color-management.md:22`, `docs/src/programmer/color-management.md:44`
   - Evidence (implementation): `crates/vfx-primaries/src/lib.rs:184`, `crates/vfx-primaries/src/lib.rs:245`
   - Impact: примеры из документации не компилируются.

348) Документация по transfer functions использует имена `linear_to_srgb`, `srgb_to_linear`, `linear_to_pq`, и т.п., которых нет в API; экспортируются `*_eotf`/`*_oetf` и encode/decode функции.
   - Evidence (doc claim): `docs/src/programmer/color-management.md:77`, `docs/src/programmer/color-management.md:93`, `docs/src/programmer/color-management.md:105`
   - Evidence (implementation): `crates/vfx-transfer/src/lib.rs:88`, `crates/vfx-transfer/src/lib.rs:91`
   - Impact: примеры из документации не компилируются, вводят в заблуждение по API именам.

349) Документация по LUT применению ссылается на `Lut3D::from_file`, `apply_lut`, и тип `Lut`, которых в `vfx-lut` нет.
   - Evidence (doc claim): `docs/src/programmer/color-management.md:181`, `docs/src/programmer/color-management.md:191`
   - Evidence (implementation): `crates/vfx-lut/src/lib.rs:60`
   - Impact: примеры из документации не компилируются; пользователям нужно использовать явные парсеры (cube/clf/spi/etc.).

350) Документация `core-api.md` описывает `vfx_core::ImageData`, `ChannelType`, `classify_channel` и `CoreError/CoreResult`, но в `vfx-core` таких типов/функций нет.
   - Evidence (doc claim): `docs/src/programmer/core-api.md:9`, `docs/src/programmer/core-api.md:141`, `docs/src/programmer/core-api.md:176`
   - Evidence (implementation): `crates/vfx-core/src/lib.rs:37`, `crates/vfx-core/src/error.rs:21`
   - Impact: раздел Core API не соответствует фактическому API и вводит в заблуждение.

351) В обзоре модулей и программистском README `vfx-core` описан как владелец `ImageData`/`ChannelType`, но фактический `ImageData` живет в `vfx-io`, а `ChannelType` отсутствует.
   - Evidence (doc claim): `docs/src/introduction.md:60`, `docs/src/programmer/README.md:112`
   - Evidence (implementation): `crates/vfx-io/src/lib.rs:537`, `crates/vfx-core/src/lib.rs:37`
   - Impact: навигация по крейтам вводит в заблуждение; пользователи ищут типы не там.

352) В `crates/ocio.md` приведен `Config::from_str`, которого нет; реальная функция — `Config::from_yaml_str`.
   - Evidence (doc claim): `docs/src/crates/ocio.md:48`
   - Evidence (implementation): `crates/vfx-ocio/src/config.rs:202`, `crates/vfx-ocio/src/config.rs:220`
   - Impact: пример не компилируется.

353) В `crates/ocio.md` используется `config.processor_opt`, но в API есть `processor_with_opts`.
   - Evidence (doc claim): `docs/src/crates/ocio.md:127`
   - Evidence (implementation): `crates/vfx-ocio/src/config.rs:987`
   - Impact: пример не компилируется.

354) В `crates/ocio.md` используется `config.processor_with_look` (единственное число), но в API есть `processor_with_looks`.
   - Evidence (doc claim): `docs/src/crates/ocio.md:198`
   - Evidence (implementation): `crates/vfx-ocio/src/config.rs:1118`
   - Impact: пример не компилируется.

355) В `crates/ocio.md` в примере `file_rules` используется `rule.pattern`, но поле `pattern` спрятано внутри `FileRuleKind::Basic`/`Regex`; напрямую доступно только `name`, `colorspace`, `kind`.
   - Evidence (doc claim): `docs/src/crates/ocio.md:285`
   - Evidence (implementation): `crates/vfx-ocio/src/config.rs:96`, `crates/vfx-ocio/src/config.rs:157`
   - Impact: пример не компилируется и неверно показывает API.

356) `colorspace_from_filepath` возвращает `Option<&str>`, но в документации используется `?` как будто это `Result`.
   - Evidence (doc claim): `docs/src/crates/ocio.md:289`
   - Evidence (implementation): `crates/vfx-ocio/src/config.rs:1253`
   - Impact: пример не компилируется; требуется явная обработка `Option`.

357) В `crates/compute.md` используется `ComputeImage::from_image_data` и `img.to_image_data()` как методы, но в API это свободные функции `from_image_data`/`to_image_data` (feature `io`), а методов нет.
   - Evidence (doc claim): `docs/src/crates/compute.md:41`, `docs/src/crates/compute.md:47`
   - Evidence (implementation): `crates/vfx-compute/src/convert.rs:272`, `crates/vfx-compute/src/convert.rs:289`
   - Impact: пример не компилируется, сигнатуры неверны.

358) В `crates/compute.md` используется `img.to_vec()`, но у `ComputeImage` есть `into_vec()`; `to_vec` отсутствует.
   - Evidence (doc claim): `docs/src/crates/compute.md:31`, `docs/src/crates/compute.md:45`
   - Evidence (implementation): `crates/vfx-compute/src/image.rs:139`
   - Impact: пример не компилируется.

359) В `crates/compute.md` показан `apply_matrix` с `Mat3`, но API принимает 4x4 `[f32;16]`.
   - Evidence (doc claim): `docs/src/crates/compute.md:86`
   - Evidence (implementation): `crates/vfx-compute/src/color.rs:118`
   - Impact: пример не компилируется и вводит в заблуждение по формату матрицы.

360) В `crates/compute.md` используется `ResizeFilter::Lanczos3`, но в API вариант называется `Lanczos`.
   - Evidence (doc claim): `docs/src/crates/compute.md:102`
   - Evidence (implementation): `crates/vfx-compute/src/ops.rs:56`
   - Impact: пример не компилируется.

361) В `crates/compute.md` показан `ProcessorBuilder::prefer_gpu(true)`, но такого метода нет; выбор backend делается через `backend(...)`.
   - Evidence (doc claim): `docs/src/crates/compute.md:142`
   - Evidence (implementation): `crates/vfx-compute/src/processor.rs:334`, `crates/vfx-compute/src/processor.rs:370`
   - Impact: пример не компилируется.

362) В `crates/compute.md` показан `TileWorkflow::new(proc, 1024)` и `workflow.process(...)`, но `TileWorkflow` — это enum без конструктора и без метода `process`.
   - Evidence (doc claim): `docs/src/crates/compute.md:156`, `docs/src/crates/compute.md:158`
   - Evidence (implementation): `crates/vfx-compute/src/backend/tiling.rs:165`
   - Impact: пример не компилируется; API неверно описан.

363) В `crates/compute.md` используется `proc.limits()?`, но `limits()` возвращает ссылку без `Result`.
   - Evidence (doc claim): `docs/src/crates/compute.md:169`
   - Evidence (implementation): `crates/vfx-compute/src/processor.rs:482`
   - Impact: пример не компилируется.

364) В `crates/io.md` упомянуты `StreamReader`/`StreamWriter` в `vfx_io::streaming`, но таких типов нет; актуальный API использует `StreamingSource`/`StreamingOutput`/`StreamingPipeline`.
   - Evidence (doc claim): `docs/src/crates/io.md:176`
   - Evidence (implementation): `crates/vfx-io/src/streaming/mod.rs:111`, `crates/vfx-io/src/streaming/traits.rs:203`
   - Impact: пример не компилируется, вводит в заблуждение по API стриминга.

365) В `crates/cli.md` указан флаг `vfx info --layers`, но у команды `info` нет флага `--layers` (есть `--stats`, `--all`, `--json`).
   - Evidence (doc claim): `docs/src/crates/cli.md:72`
   - Evidence (implementation): `crates/vfx-cli/src/main.rs:252`
   - Impact: пример не работает.

366) В `crates/cli.md` описан `vfx layers` с флагами `--list/--extract/--merge/--rename`, но в CLI это отдельные подкоманды `layers`, `extract-layer`, `merge-layers`; флага `--rename` нет.
   - Evidence (doc claim): `docs/src/crates/cli.md:175`, `docs/src/crates/cli.md:179`, `docs/src/crates/cli.md:182`, `docs/src/crates/cli.md:185`
   - Evidence (implementation): `crates/vfx-cli/src/main.rs:195`, `crates/vfx-cli/src/main.rs:206`, `crates/vfx-cli/src/main.rs:210`
   - Impact: примеры не соответствуют реальному CLI интерфейсу.

367) В `crates/cli.md` для `resize` указан фильтр `bicubic`/`nearest`, но CLI принимает `box`, `bilinear`, `lanczos`, `mitchell`.
   - Evidence (doc claim): `docs/src/crates/cli.md:101`, `docs/src/crates/cli.md:106`
   - Evidence (implementation): `crates/vfx-cli/src/main.rs:300`
   - Impact: часть фильтров из документации не распознается.

368) В `crates/cli.md` для `batch` используется `--output` с шаблоном и `--jobs`, но CLI ожидает `--output-dir`, `--op`, `--args`, `--format` и не поддерживает `--jobs`.
   - Evidence (doc claim): `docs/src/crates/cli.md:209`, `docs/src/crates/cli.md:216`
   - Evidence (implementation): `crates/vfx-cli/src/main.rs:562`, `crates/vfx-cli/src/commands/batch.rs:34`
   - Impact: примеры не работают.

369) В `crates/cli.md` указано `vfx view ... --layer`, но у команды `view` нет флага `--layer`.
   - Evidence (doc claim): `docs/src/crates/cli.md:240`
   - Evidence (implementation): `crates/vfx-cli/src/main.rs:754`
   - Impact: пример не работает.

370) В `core-api.md`/`programmer/README.md` примеры используют методы `ImageData::constant`, `get_pixel`, `set_pixel`, `width()/height()/channels()`, но у `vfx-io::ImageData` таких методов нет (есть только публичные поля и `from_f32`/`to_f32` и т.п.).
   - Evidence (doc claim): `docs/src/programmer/core-api.md:18`, `docs/src/programmer/core-api.md:30`, `docs/src/programmer/README.md:68`
   - Evidence (implementation): `crates/vfx-io/src/lib.rs:723`
   - Impact: примеры не компилируются и вводят в заблуждение по API `ImageData`.

371) В `color-management.md` используются `apply_idt`/`apply_rrt_odt`, но в `vfx_color::aces` таких функций нет (есть `apply_rrt_odt_srgb` и `apply_inverse_odt_srgb`).
   - Evidence (doc claim): `docs/src/programmer/color-management.md:146`, `docs/src/programmer/color-management.md:164`
   - Evidence (implementation): `crates/vfx-color/src/aces.rs:224`, `crates/vfx-color/src/aces.rs:243`
   - Impact: примеры не компилируются и неверно описывают ACES API.

372) В `crates/io.md` используется `exr::read_layer` и `exr::read_layered`, но в API есть только `exr::read_layers`.
   - Evidence (doc claim): `docs/src/crates/io.md:68`, `docs/src/crates/io.md:119`
   - Evidence (implementation): `crates/vfx-io/src/exr.rs:819`
   - Impact: примеры не компилируются.

373) В `programmer/imagebufalgo/README.md` примеры не соответствуют API: используется `add_constant`, `blur_inplace`, и `resize(..., "lanczos")` с строковым фильтром, но таких функций/сигнатур нет (есть `add`, `blur_into`, `resize` с `ResizeFilter`).
   - Evidence (doc claim): `docs/src/programmer/imagebufalgo/README.md:22`, `docs/src/programmer/imagebufalgo/README.md:44`, `docs/src/programmer/imagebufalgo/README.md:73`
   - Evidence (implementation): `crates/vfx-io/src/imagebufalgo/mod.rs:66`, `crates/vfx-io/src/imagebufalgo/filters.rs:119`, `crates/vfx-io/src/imagebufalgo/geometry.rs:315`
   - Impact: примеры не компилируются, вводят в заблуждение по API imagebufalgo.

374) В `programmer/ocio-integration.md` используются `baker.bake_1d`/`baker.bake_3d`, но реальные методы — `bake_lut_1d`/`bake_lut_3d`.
   - Evidence (doc claim): `docs/src/programmer/ocio-integration.md:200`, `docs/src/programmer/ocio-integration.md:204`
   - Evidence (implementation): `crates/vfx-ocio/src/baker.rs:84`, `crates/vfx-ocio/src/baker.rs:148`
   - Impact: пример не компилируется.

375) В `programmer/imagebufalgo/filters.md` примеры вызывают `blur(&image, 5.0)?`, `blur_xy`, и функции без `roi`, но API требует `roi: Option<Roi3D>`, не возвращает `Result`, и `blur_xy` отсутствует.
   - Evidence (doc claim): `docs/src/programmer/imagebufalgo/filters.md:10`, `docs/src/programmer/imagebufalgo/filters.md:13`
   - Evidence (implementation): `crates/vfx-io/src/imagebufalgo/filters.rs:111`, `crates/vfx-io/src/imagebufalgo/filters.rs:119`
   - Impact: примеры не компилируются и не отражают фактические сигнатуры.

376) В `crates/bench.md` описаны I/O и resize бенчмарки (io/*, resize/*), которых нет в `vfx-bench` — там только transfer/lut/cdl/simd.
   - Evidence (doc claim): `docs/src/crates/bench.md:23`, `docs/src/crates/bench.md:31`
   - Evidence (implementation): `crates/vfx-bench/benches/vfx_bench.rs:9`
   - Impact: документация вводит в заблуждение по покрытию бенчмарков.

377) `docs/src/aces/vfx-rs-aces.md` использует ряд несуществующих API: `Primaries::ACES_AP0`/`Primaries::ACES_AP1`, `linear_to_*`/`*_to_linear` из `vfx_transfer`, `apply_aces_idt`/`apply_aces_rrt_odt`/`apply_idt`/`apply_rrt_odt` из `vfx_color::aces`, `Config::from_env`, `config.color_spaces()`, `Processor::new`, `DisplayViewProcessor::new`.
   - Evidence (doc claim): `docs/src/aces/vfx-rs-aces.md:45`, `docs/src/aces/vfx-rs-aces.md:92`, `docs/src/aces/vfx-rs-aces.md:147`, `docs/src/aces/vfx-rs-aces.md:183`, `docs/src/aces/vfx-rs-aces.md:187`, `docs/src/aces/vfx-rs-aces.md:200`, `docs/src/aces/vfx-rs-aces.md:214`
   - Evidence (implementation): `crates/vfx-primaries/src/lib.rs:184`, `crates/vfx-transfer/src/lib.rs:88`, `crates/vfx-color/src/aces.rs:224`, `crates/vfx-ocio/src/config.rs:202`, `crates/vfx-ocio/src/processor.rs:642`
   - Impact: документация по ACES не компилируется и вводит в заблуждение.

378) В `docs/src/aces/color-spaces.md` и `docs/src/aces/transfer-functions.md` используются `Primaries::ACES_AP0/AP1` и `linear_to_*`/`*_to_linear` из `vfx_transfer`, которых нет в API.
   - Evidence (doc claim): `docs/src/aces/color-spaces.md:31`, `docs/src/aces/color-spaces.md:61`, `docs/src/aces/transfer-functions.md:63`, `docs/src/aces/transfer-functions.md:170`
   - Evidence (implementation): `crates/vfx-primaries/src/lib.rs:184`, `crates/vfx-transfer/src/lib.rs:88`
   - Impact: ACES доп. документация не компилируется.

379) В `crates/color.md` используются методы `ColorProcessor::srgb_to_linear` и `ColorProcessor::apply_srgb_to_linear`, которых нет в API.
   - Evidence (doc claim): `docs/src/crates/color.md:53`, `docs/src/crates/color.md:57`
   - Evidence (implementation): `crates/vfx-color/src/processor.rs:181`, `crates/vfx-color/src/processor.rs:210`
   - Impact: пример не компилируется; нужно использовать `Pipeline` + `apply`/`apply_batch`.

380) В `crates/color.md` указан `pipeline.apply_buffer(&mut data)`, но у `Pipeline` нет `apply_buffer`; есть `ColorProcessor::apply_buffer` для плоского буфера и размерности.
   - Evidence (doc claim): `docs/src/crates/color.md:82`
   - Evidence (implementation): `crates/vfx-color/src/pipeline.rs:129`, `crates/vfx-color/src/processor.rs:264`
   - Impact: пример не компилируется и вводит в заблуждение по API буферной обработки.

381) В `crates/color.md` используются `Pipeline::tonemap_reinhard()` и `Pipeline::lut_3d(...)`, но в API нет `tonemap_reinhard`, а метод называется `lut3d`.
   - Evidence (doc claim): `docs/src/crates/color.md:206`, `docs/src/crates/color.md:217`
   - Evidence (implementation): `crates/vfx-color/src/pipeline.rs:129`, `crates/vfx-color/src/pipeline.rs:173`
   - Impact: примеры не компилируются и неверно описывают API пайплайна.

382) В `crates/core.md` примеры используют `Rgb<Srgb>` и `Rgba<AcesCg>` без параметра типа пикселя, но в API требуется `Rgb<C, T>`/`Rgba<C, T>`.
   - Evidence (doc claim): `docs/src/crates/core.md:19`, `docs/src/crates/core.md:20`
   - Evidence (implementation): `crates/vfx-core/src/pixel.rs:340`, `crates/vfx-core/src/pixel.rs:507`
   - Impact: примеры не компилируются, сигнатуры неверны.

383) В `crates/core.md` типы `Image`, `ImageView`, `ImageViewMut` используются без параметра `const N`, но в API они определены как `Image<C, T, N>` и `ImageView<'a, C, T, N>`.
   - Evidence (doc claim): `docs/src/crates/core.md:40`, `docs/src/crates/core.md:43`, `docs/src/crates/core.md:46`
   - Evidence (implementation): `crates/vfx-core/src/image.rs:115`, `crates/vfx-core/src/image.rs:537`, `crates/vfx-core/src/image.rs:639`
   - Impact: примеры не компилируются.

384) В `crates/core.md` показан вызов `srgb_to_linear(...)`, но такой функции в `vfx-core` нет.
   - Evidence (doc claim): `docs/src/crates/core.md:97`
   - Evidence (implementation): `crates/vfx-core/src/lib.rs:53`
   - Impact: пример не компилируется; требуется использовать функцию из `vfx-transfer` либо другой слой.

385) В `crates/icc.md` используется `Profile::from_bytes`, но в API есть `Profile::from_icc`.
   - Evidence (doc claim): `docs/src/crates/icc.md:41`, `docs/src/crates/icc.md:200`
   - Evidence (implementation): `crates/vfx-icc/src/profile.rs:69`
   - Impact: примеры не компилируются.

386) В `crates/icc.md` указан `Profile::lab_d50()`, но в API есть `Profile::lab()`.
   - Evidence (doc claim): `docs/src/crates/icc.md:61`
   - Evidence (implementation): `crates/vfx-icc/src/profile.rs:193`
   - Impact: пример не компилируется.

387) В `crates/icc.md` используется `Profile::from_standard(...) ?`, но функция возвращает `Self` без `Result`.
   - Evidence (doc claim): `docs/src/crates/icc.md:71`
   - Evidence (implementation): `crates/vfx-icc/src/profile.rs:105`
   - Impact: пример не компилируется и вводит в заблуждение по сигнатуре.

388) В `crates/icc.md` пример `convert_rgb` использует порядок аргументов `(src, dst, intent, pixels)`, но API ожидает `(pixels, src, dst, intent)`.
   - Evidence (doc claim): `docs/src/crates/icc.md:135`
   - Evidence (implementation): `crates/vfx-icc/src/transform.rs:186`
   - Impact: пример не компилируется.

389) В `crates/lut.md` используется `Lut3D::apply_tetrahedral`, но этот метод не публичный; публичный API использует `apply` + `Interpolation::Tetrahedral`.
   - Evidence (doc claim): `docs/src/crates/lut.md:45`
   - Evidence (implementation): `crates/vfx-lut/src/lut3d.rs:224`
   - Impact: пример не компилируется и предлагает недоступный API.

390) В `crates/lut.md` матчинг `ProcessNode::Matrix(m)`/`Lut1D(lut)`/`Lut3D(lut)` не соответствует реальным вариантам enum (они struct-like с полями).
   - Evidence (doc claim): `docs/src/crates/lut.md:89`, `docs/src/crates/lut.md:90`, `docs/src/crates/lut.md:91`, `docs/src/crates/lut.md:92`
   - Evidence (implementation): `crates/vfx-lut/src/clf.rs:334`
   - Impact: пример не компилируется, структура `ProcessNode` описана неверно.

391) В `crates/lut.md` используется `Lut1D::from_fn`, но в API такого конструктора нет (есть `from_data`/`from_rgb`).
   - Evidence (doc claim): `docs/src/crates/lut.md:156`, `docs/src/crates/lut.md:159`, `docs/src/crates/lut.md:162`
   - Evidence (implementation): `crates/vfx-lut/src/lut1d.rs:112`
   - Impact: пример не компилируется.

392) В `crates/lut.md` пример использует `apply_pixel` и `Lut3D::set`, которых нет в API.
   - Evidence (doc claim): `docs/src/crates/lut.md:180`, `docs/src/crates/lut.md:181`
   - Evidence (implementation): `crates/vfx-lut/src/lut3d.rs:49`
   - Impact: пример не компилируется.

393) В `crates/math.md` указан `catmull_rom`, но такой функции в `vfx-math` нет.
   - Evidence (doc claim): `docs/src/crates/math.md:78`, `docs/src/crates/math.md:87`
   - Evidence (implementation): `crates/vfx-math/src/interp.rs:42`
   - Impact: пример не компилируется.

394) В `crates/math.md` используются `simd::process_rgba_f32x8` и `simd::apply_matrix_simd`, но таких функций в модуле `simd` нет.
   - Evidence (doc claim): `docs/src/crates/math.md:95`, `docs/src/crates/math.md:98`
   - Evidence (implementation): `crates/vfx-math/src/simd.rs:1`
   - Impact: пример не компилируется.

395) В `crates/math.md` указаны `rgb_to_luminance` и `linearize_srgb`, но этих функций в `vfx-math` нет.
   - Evidence (doc claim): `docs/src/crates/math.md:109`, `docs/src/crates/math.md:112`
   - Evidence (implementation): `crates/vfx-math/src/lib.rs:29`
   - Impact: пример не компилируется и вводит в заблуждение по набору утилит.

396) В `crates/ops.md` примеры `over`/`blend` не передают `width/height`, но API требует размеры изображения.
   - Evidence (doc claim): `docs/src/crates/ops.md:111`, `docs/src/crates/ops.md:114`
   - Evidence (implementation): `crates/vfx-ops/src/composite.rs:250`, `crates/vfx-ops/src/composite.rs:296`
   - Impact: примеры не компилируются.

397) В `crates/ops.md` `premultiply(&mut rgba_data)`/`unpremultiply(&mut rgba_data)` не соответствует API: `premultiply` работает с одним пикселем, а для буфера есть `premultiply_inplace`/`unpremultiply_inplace`.
   - Evidence (doc claim): `docs/src/crates/ops.md:141`, `docs/src/crates/ops.md:144`
   - Evidence (implementation): `crates/vfx-ops/src/composite.rs:345`, `crates/vfx-ops/src/composite.rs:451`
   - Impact: пример не компилируется/использует неверную функцию.

398) В `crates/ops.md` используются `rotate_90` и `rotate_270`, но в API есть `rotate_90_cw`/`rotate_90_ccw` и `rotate_180`; `rotate_270` отсутствует.
   - Evidence (doc claim): `docs/src/crates/ops.md:152`, `docs/src/crates/ops.md:155`
   - Evidence (implementation): `crates/vfx-ops/src/transform.rs:176`, `crates/vfx-ops/src/transform.rs:215`, `crates/vfx-ops/src/transform.rs:253`
   - Impact: пример не компилируется.

399) В `crates/ops.md` указаны `barrel_distort`, `pincushion_distort` и `st_map`, но в API есть `barrel`, `pincushion`; `st_map` отсутствует.
   - Evidence (doc claim): `docs/src/crates/ops.md:180`, `docs/src/crates/ops.md:194`, `docs/src/crates/ops.md:197`
   - Evidence (implementation): `crates/vfx-ops/src/warp.rs:69`, `crates/vfx-ops/src/warp.rs:99`
   - Impact: примеры не компилируются.

400) В `crates/ops.md` упомянуты `layer_ops::apply_to_layer` и `LayerMask`, но в `layer_ops` их нет (есть `resize_layer`, `blur_layer`, `crop_layer` и т.п.).
   - Evidence (doc claim): `docs/src/crates/ops.md:205`, `docs/src/crates/ops.md:208`
   - Evidence (implementation): `crates/vfx-ops/src/layer_ops.rs:1`
   - Impact: пример не компилируется и описывает несуществующий API.

401) В `crates/python.md` используются `Image.to_numpy()`/`Image.from_numpy()` и `img.data()`, но в биндингах есть `Image.numpy()` и конструктор `Image(array)`; `data()` отсутствует.
   - Evidence (doc claim): `docs/src/crates/python.md:42`, `docs/src/crates/python.md:48`, `docs/src/crates/python.md:67`, `docs/src/crates/python.md:68`, `docs/src/crates/python.md:71`
   - Evidence (implementation): `crates/vfx-rs-py/src/image.rs:48`, `crates/vfx-rs-py/src/image.rs:105`
   - Impact: примеры не работают, API описан неверно.

402) В `crates/python.md` показан `vfx_rs.write(..., quality=..., compression=...)`, но топ-уровневый `write` принимает только `(path, image)`.
   - Evidence (doc claim): `docs/src/crates/python.md:88`, `docs/src/crates/python.md:89`
   - Evidence (implementation): `crates/vfx-rs-py/src/lib.rs:75`
   - Impact: примеры не работают; форматные опции доступны только в `vfx_rs.io`.

403) В `crates/python.md` используется модуль `vfx_rs.color` и функции `apply_srgb_eotf`/`rgb_to_rgb_matrix`, но такого подмодуля в биндингах нет.
   - Evidence (doc claim): `docs/src/crates/python.md:95`, `docs/src/crates/python.md:103`, `docs/src/crates/python.md:117`
   - Evidence (implementation): `crates/vfx-rs-py/src/lib.rs:98`
   - Impact: примеры не работают.

404) В `crates/python.md` используются `lut.apply_3d`/`lut.apply_1d`, но в модуле `lut` нет таких функций (есть классы `Lut1D`/`Lut3D` с `apply`).
   - Evidence (doc claim): `docs/src/crates/python.md:135`, `docs/src/crates/python.md:136`
   - Evidence (implementation): `crates/vfx-rs-py/src/lut.rs:7`
   - Impact: примеры не работают.

405) В `crates/python.md` `ops.resize` использует `filter="lanczos"` и `scale=...`, но Python API ожидает `ResizeFilter` enum и параметры `width/height`; `scale` не поддерживается.
   - Evidence (doc claim): `docs/src/crates/python.md:145`, `docs/src/crates/python.md:146`
   - Evidence (implementation): `crates/vfx-rs-py/src/ops.rs:383`, `crates/vfx-rs-py/src/ops.rs:418`
   - Impact: примеры не работают.

406) В `crates/python.md` `ops.blur` принимает `radius` и `type`, но реальная сигнатура — `blur(image, sigma, roi=None)`.
   - Evidence (doc claim): `docs/src/crates/python.md:149`
   - Evidence (implementation): `crates/vfx-rs-py/src/ops.rs:941`
   - Impact: пример не работает.

407) В `crates/python.md` указан `ops.blend(..., mode=...)`, но в Python-модуле `ops` нет функции `blend` (есть отдельные `*_blend`).
   - Evidence (doc claim): `docs/src/crates/python.md:153`
   - Evidence (implementation): `crates/vfx-rs-py/src/ops.rs:1676`
   - Impact: пример не работает.

408) В `crates/python.md` используется `ocio.Config`/`builtin_aces_1_3` и методы `config.processor`/`display_processor`, но в биндингах класс называется `ColorConfig`, метод `aces_1_3`, а `display_processor` отсутствует.
   - Evidence (doc claim): `docs/src/crates/python.md:162`, `docs/src/crates/python.md:165`, `docs/src/crates/python.md:175`
   - Evidence (implementation): `crates/vfx-rs-py/src/ocio.rs:178`, `crates/vfx-rs-py/src/ocio.rs:240`
   - Impact: примеры не работают.

409) В `crates/python.md` показаны `read_layers`/`read_layer`/`write_layers`, но в биндингах есть только `read_layered` (LayeredImage); указанных функций нет.
   - Evidence (doc claim): `docs/src/crates/python.md:227`, `docs/src/crates/python.md:232`, `docs/src/crates/python.md:235`
   - Evidence (implementation): `crates/vfx-rs-py/src/lib.rs:58`
   - Impact: примеры не работают.

410) В `crates/python.md` используются исключения `vfx_rs.IoError`/`vfx_rs.FormatError`, но такие типы не экспортируются в Python.
   - Evidence (doc claim): `docs/src/crates/python.md:249`, `docs/src/crates/python.md:251`
   - Evidence (implementation): `crates/vfx-rs-py/src/lib.rs:84`
   - Impact: обработчик исключений не сработает как описано.

411) В `crates/python.md` заявлено, что `to_numpy()` может возвращать zero-copy view, но реализация `numpy()` всегда делает копию через `to_f32()`.
   - Evidence (doc claim): `docs/src/crates/python.md:213`, `docs/src/crates/python.md:214`
   - Evidence (implementation): `crates/vfx-rs-py/src/image.rs:105`
   - Impact: ожидания по производительности/памяти не соответствуют факту.

412) В `crates/tests.md` описана структура `crates/vfx-tests/tests/*.rs`, но в реальности тесты находятся в `crates/vfx-tests/src/lib.rs` и `crates/vfx-tests/src/golden.rs`; директории `tests/` нет.
   - Evidence (doc claim): `docs/src/crates/tests.md:12`, `docs/src/crates/tests.md:14`, `docs/src/crates/tests.md:15`, `docs/src/crates/tests.md:19`
   - Evidence (implementation): `crates/vfx-tests/src/lib.rs:1`
   - Impact: документация вводит в заблуждение по структуре тестов.

413) В `crates/tests.md` предлагается `cargo test -p vfx-tests --test io_roundtrip`, но такой integration test-таргет отсутствует.
   - Evidence (doc claim): `docs/src/crates/tests.md:29`
   - Evidence (implementation): `crates/vfx-tests/src/lib.rs:21`
   - Impact: команда из документации не работает.

414) В `crates/README.md` диаграмма зависимостей показывает `vfx-core → vfx-math`, но на самом деле `vfx-math` зависит от `vfx-core`.
   - Evidence (doc claim): `docs/src/crates/README.md:78`, `docs/src/crates/README.md:79`
   - Evidence (implementation): `crates/vfx-math/Cargo.toml:10`
   - Impact: неверная архитектурная схема вводит в заблуждение.

415) В `programmer/README.md` используется `vfx_ops::resize` и вызов `resize(&image, 960, 540)`, но такого API в `vfx-ops` нет; есть `resize::resize_f32` для raw-буфера.
   - Evidence (doc claim): `docs/src/programmer/README.md:45`, `docs/src/programmer/README.md:52`
   - Evidence (implementation): `crates/vfx-ops/src/resize.rs:120`
   - Impact: пример не компилируется.

416) В `appendix/formats.md` указано, что PSD поддерживает только flattened read и не поддерживает layers, но в `vfx-io::psd` есть чтение слоев (`read_layers`, `read_layer_by_*`).
   - Evidence (doc claim): `docs/src/appendix/formats.md:101`, `docs/src/appendix/formats.md:103`
   - Evidence (implementation): `crates/vfx-io/src/psd.rs:153`, `crates/vfx-io/src/psd.rs:190`
   - Impact: документация занижает фактическую поддержку PSD.

417) В `appendix/formats.md` используются CLI флаги `vfx info ... --layers` и `vfx convert ... --layer`, но таких флагов в CLI нет.
   - Evidence (doc claim): `docs/src/appendix/formats.md:29`, `docs/src/appendix/formats.md:30`
   - Evidence (implementation): `crates/vfx-cli/src/main.rs:252`, `crates/vfx-cli/src/main.rs:195`
   - Impact: примеры не работают.

418) В `appendix/feature-matrix.md` заявлен формат `TX (tiled)` как поддерживаемый, но в `vfx-io` нет модуля/формата `tx` (есть `ktx` под фичей `ktx`).
   - Evidence (doc claim): `docs/src/appendix/feature-matrix.md:174`
   - Evidence (implementation): `crates/vfx-io/src/lib.rs:152`, `crates/vfx-io/src/lib.rs:153`
   - Impact: матрица возможностей вводит в заблуждение по поддержке форматов.

419) В `cli/channel-extract.md` сказано, что каналы можно задавать через запятые и через произвольные имена (`N.x`, `beauty.R`), но CLI принимает список значений `-c` и поддерживает только `R/G/B/A/Z` (или индекс); произвольные имена не парсятся.
   - Evidence (doc claim): `docs/src/cli/channel-extract.md:26`, `docs/src/cli/channel-extract.md:57`, `docs/src/cli/channel-extract.md:72`, `docs/src/cli/channel-extract.md:118`
   - Evidence (implementation): `crates/vfx-cli/src/main.rs:653`, `crates/vfx-cli/src/commands/channels.rs:160`, `crates/vfx-cli/src/commands/channels.rs:175`, `crates/vfx-cli/src/commands/channels.rs:176`
   - Impact: примеры с `-c R,G,B` и `N.x` не работают.

420) В `cli/channel-shuffle.md` утверждается, что отсутствующие каналы по умолчанию = 1 для `A` и что операция «без пиксельной обработки», но реализация для отсутствующего канала пишет 0.0 и итерирует все пиксели.
   - Evidence (doc claim): `docs/src/cli/channel-shuffle.md:128`, `docs/src/cli/channel-shuffle.md:130`
   - Evidence (implementation): `crates/vfx-cli/src/commands/channels.rs:96`, `crates/vfx-cli/src/commands/channels.rs:117`, `crates/vfx-cli/src/commands/channels.rs:120`
   - Impact: описанное поведение по альфе/производительности не соответствует факту.

421) В `cli/composite.md` перечислены режимы `subtract/overlay/softlight/hardlight/difference`, но CLI принимает только `over/add/multiply/screen`.
   - Evidence (doc claim): `docs/src/cli/composite.md:25`, `docs/src/cli/composite.md:26`, `docs/src/cli/composite.md:27`, `docs/src/cli/composite.md:28`, `docs/src/cli/composite.md:29`
   - Evidence (implementation): `crates/vfx-cli/src/commands/composite.rs:32`, `crates/vfx-cli/src/commands/composite.rs:33`, `crates/vfx-cli/src/commands/composite.rs:34`, `crates/vfx-cli/src/commands/composite.rs:35`, `crates/vfx-cli/src/commands/composite.rs:36`
   - Impact: документированные режимы приводят к ошибке «Unknown blend mode».

422) В `cli/diff.md` заявлены «per-pixel warn», спец-формат diff-изображения и отдельные exit-коды, но реализация сравнивает/варнит только по `max_diff`, применяет порог только если `threshold > 0`, и diff-изображение — просто `|A-B| * 10` по общим каналам.
   - Evidence (doc claim): `docs/src/cli/diff.md:21`, `docs/src/cli/diff.md:73`, `docs/src/cli/diff.md:103`, `docs/src/cli/diff.md:114`
   - Evidence (implementation): `crates/vfx-cli/src/commands/diff.rs:57`, `crates/vfx-cli/src/commands/diff.rs:62`, `crates/vfx-cli/src/commands/diff.rs:63`, `crates/vfx-cli/src/commands/diff.rs:116`
   - Impact: поведение предупреждений/порогов и формат diff-изображения не соответствует документации.

423) В `cli/grep.md` заявлены regex и поиск по метаданным/EXIF, но команда ищет только подстроки в имени файла, строке `WxH Nch` и названии формата.
   - Evidence (doc claim): `docs/src/cli/grep.md:3`, `docs/src/cli/grep.md:15`, `docs/src/cli/grep.md:64`, `docs/src/cli/grep.md:66`
   - Evidence (implementation): `crates/vfx-cli/src/commands/grep.rs:19`, `crates/vfx-cli/src/commands/grep.rs:30`, `crates/vfx-cli/src/commands/grep.rs:36`, `crates/vfx-cli/src/commands/grep.rs:52`
   - Impact: ожидания по функционалу не совпадают; примеры с regex/metadata не работают.

424) В `cli/lut.md` указана поддержка `.clf/.spi1d/.spi3d/.3dl`, но CLI принимает только `.cube`.
   - Evidence (doc claim): `docs/src/cli/lut.md:16`, `docs/src/cli/lut.md:24`, `docs/src/cli/lut.md:25`, `docs/src/cli/lut.md:26`, `docs/src/cli/lut.md:27`, `docs/src/cli/lut.md:49`
   - Evidence (implementation): `crates/vfx-cli/src/commands/lut.rs:25`, `crates/vfx-cli/src/commands/lut.rs:26`
   - Impact: все примеры с `.clf/.spi/.3dl` завершаются ошибкой.

425) В `cli/maketx.md` заявлены `.tx`, тайлинг, wrap и встроенные mipmap-цепочки, но реализация сохраняет только исходное изображение и не использует `tile/wrap`; встроенные mipmaps не пишутся.
   - Evidence (doc claim): `docs/src/cli/maketx.md:17`, `docs/src/cli/maketx.md:55`, `docs/src/cli/maketx.md:136`, `docs/src/cli/maketx.md:137`
   - Evidence (implementation): `crates/vfx-cli/src/commands/maketx.rs:18`, `crates/vfx-cli/src/commands/maketx.rs:21`, `crates/vfx-cli/src/commands/maketx.rs:72`, `crates/vfx-cli/src/commands/maketx.rs:73`
   - Impact: пользователи ожидают `.tx`/mipmaps/тайлинг, но получают обычный файл без mip-цепочки.

426) В `cli/rotate.md` заявлена параллельная обработка через rayon, но реализация — один проход с обычными циклами без parallel.
   - Evidence (doc claim): `docs/src/cli/rotate.md:107`
   - Evidence (implementation): `crates/vfx-ops/src/transform.rs:520`, `crates/vfx-ops/src/transform.rs:545`
   - Impact: ожидания по производительности не соответствуют факту.

427) В `cli/transform.md` сказано, что `-r 90` — CCW, «все EXR-слои трансформируются» и «метаданные сохраняются», но код использует `rotate_90_cw`, читает один слой и создаёт `ImageData::from_f32` с `Metadata::default()`.
   - Evidence (doc claim): `docs/src/cli/transform.md:40`, `docs/src/cli/transform.md:126`, `docs/src/cli/transform.md:128`
   - Evidence (implementation): `crates/vfx-cli/src/commands/transform.rs:8`, `crates/vfx-cli/src/commands/transform.rs:11`, `crates/vfx-cli/src/commands/transform.rs:37`, `crates/vfx-cli/src/commands/transform.rs:65`, `crates/vfx-io/src/lib.rs:747`, `crates/vfx-io/src/lib.rs:754`
   - Impact: поворот идёт в другую сторону; слои и метаданные теряются при записи.

428) В `cli/layers.md` описаны подкоманды `layers list/extract/merge`, но в CLI есть отдельные команды `layers`, `extract-layer`, `merge-layers`.
   - Evidence (doc claim): `docs/src/cli/layers.md:7`, `docs/src/cli/layers.md:8`, `docs/src/cli/layers.md:9`
   - Evidence (implementation): `crates/vfx-cli/src/main.rs:200`, `crates/vfx-cli/src/main.rs:204`, `crates/vfx-cli/src/main.rs:208`
   - Impact: команды из документации не существуют.

429) В `cli/merge-layers.md` указано `--names` как «comma-separated», но CLI принимает `Vec<String>` без разделителя; нужно передавать `-n` несколько раз или перечислять значениями.
   - Evidence (doc claim): `docs/src/cli/merge-layers.md:15`, `docs/src/cli/merge-layers.md:26`, `docs/src/cli/merge-layers.md:34`
   - Evidence (implementation): `crates/vfx-cli/src/main.rs:620`
   - Impact: примеры с `--names beauty,diffuse,...` не работают.

430) В `cli/batch.md` синтаксис и доступные операции не соответствуют реализации: в доке нет `-i/--input`, заявлены `color` и `width/height/filter`, а реально поддерживаются только `convert/resize(scale)/blur(box)/flip_h/flip_v`.
   - Evidence (doc claim): `docs/src/cli/batch.md:8`, `docs/src/cli/batch.md:15`, `docs/src/cli/batch.md:23`, `docs/src/cli/batch.md:29`, `docs/src/cli/batch.md:71`, `docs/src/cli/batch.md:73`, `docs/src/cli/batch.md:74`
   - Evidence (implementation): `crates/vfx-cli/src/main.rs:560`, `crates/vfx-cli/src/main.rs:563`, `crates/vfx-cli/src/commands/batch.rs:117`, `crates/vfx-cli/src/commands/batch.rs:121`, `crates/vfx-cli/src/commands/batch.rs:122`, `crates/vfx-cli/src/commands/batch.rs:134`, `crates/vfx-cli/src/commands/batch.rs:143`, `crates/vfx-cli/src/commands/batch.rs:147`
   - Impact: большая часть примеров и аргументов в batch-доках не работает.

431) В `cli/resize.md` для высоты указан `-h`, но в CLI используется `-H` (так как `-h` зарезервирован под help).
   - Evidence (doc claim): `docs/src/cli/resize.md:16`
   - Evidence (implementation): `crates/vfx-cli/src/main.rs:305`
   - Impact: пример `-h` не работает; CLI ожидает `-H`.

432) В `cli/color.md` используются короткие флаги `-e/-g/-s/-t` и заявлены `pq/hlg/log/srgb-inv`, но CLI поддерживает только длинные флаги и фактически обрабатывает только `srgb/srgb_to_linear/linear_to_srgb/rec709`.
   - Evidence (doc claim): `docs/src/cli/color.md:15`, `docs/src/cli/color.md:16`, `docs/src/cli/color.md:17`, `docs/src/cli/color.md:18`, `docs/src/cli/color.md:28`, `docs/src/cli/color.md:29`, `docs/src/cli/color.md:30`, `docs/src/cli/color.md:31`, `docs/src/cli/color.md:32`, `docs/src/cli/color.md:33`, `docs/src/cli/color.md:51`
   - Evidence (implementation): `crates/vfx-cli/src/main.rs:448`, `crates/vfx-cli/src/main.rs:456`, `crates/vfx-cli/src/main.rs:460`, `crates/vfx-cli/src/main.rs:464`, `crates/vfx-cli/src/main.rs:468`, `crates/vfx-cli/src/commands/color.rs:74`, `crates/vfx-cli/src/commands/color.rs:115`, `crates/vfx-cli/src/commands/color.rs:125`
   - Impact: короткие флаги и заявленные transfer-функции не работают.

433) В `cli/aces.md` используется `--rrt` и варианты `alt1/filmic`, но CLI имеет только `--rrt-variant` (long) и поддерживает `default` или `high-contrast`.
   - Evidence (doc claim): `docs/src/cli/aces.md:16`, `docs/src/cli/aces.md:49`, `docs/src/cli/aces.md:57`, `docs/src/cli/aces.md:58`
   - Evidence (implementation): `crates/vfx-cli/src/main.rs:739`, `crates/vfx-cli/src/commands/aces.rs:92`, `crates/vfx-cli/src/commands/aces.rs:93`
   - Impact: примеры с `--rrt alt1/filmic` не работают.

434) В `cli/view.md` команда описана как доступная всегда, но в CLI она включается только при фиче `viewer`.
   - Evidence (doc claim): `docs/src/cli/view.md:1`, `docs/src/cli/view.md:5`
   - Evidence (implementation): `crates/vfx-cli/src/main.rs:237`, `crates/vfx-cli/src/main.rs:752`
   - Impact: без сборки с `viewer` команда отсутствует, документация вводит в заблуждение.

435) В `cli/sharpen.md` заявлено unsharp masking, но реализация использует свёртку с 3x3 sharpen kernel без отдельного blur-pass.
   - Evidence (doc claim): `docs/src/cli/sharpen.md:1`, `docs/src/cli/sharpen.md:55`
   - Evidence (implementation): `crates/vfx-cli/src/commands/sharpen.rs:21`, `crates/vfx-ops/src/filter.rs:124`
   - Impact: описание алгоритма не соответствует реальному поведению.

436) В `cli/warp.md` приведены формулы и интерпретации параметров (например `wave` по X), но реализация для `wave` смещает X в зависимости от `y` и использует иные формулы для `twist`.
   - Evidence (doc claim): `docs/src/cli/warp.md:90`, `docs/src/cli/warp.md:98`
   - Evidence (implementation): `crates/vfx-ops/src/warp.rs:120`, `crates/vfx-ops/src/warp.rs:160`
   - Impact: пользователи получают иное искажение, чем ожидают по документации.

437) В `cli/extract-layer.md` говорится, что без `--layer` извлекается первый слой и «метаданные сохраняются», но реализация без `--layer` лишь выводит список и выходит, а `to_image_data` создаёт `ImageData` с `Metadata::default()`.
   - Evidence (doc claim): `docs/src/cli/extract-layer.md:39`, `docs/src/cli/extract-layer.md:105`
   - Evidence (implementation): `crates/vfx-cli/src/commands/layers.rs:155`, `crates/vfx-cli/src/commands/layers.rs:156`, `crates/vfx-io/src/lib.rs:1042`, `crates/vfx-io/src/lib.rs:1068`
   - Impact: команда без `--layer` ничего не извлекает; метаданные теряются.

438) В `appendix/cli-ref.md` указан глобальный флаг `-q/--quiet`, которого нет в CLI (доступны только `-v`, `-l`, `-j`, `--allow-non-color`).
   - Evidence (doc claim): `docs/src/appendix/cli-ref.md:9`
   - Evidence (implementation): `crates/vfx-cli/src/main.rs:131`, `crates/vfx-cli/src/main.rs:135`, `crates/vfx-cli/src/main.rs:139`, `crates/vfx-cli/src/main.rs:143`
   - Impact: пользователи получают ошибку «unexpected argument --quiet».

439) В `appendix/cli-ref.md` для `info` описаны `--layers/--channels`, но в CLI есть только `--stats/--all/--json`.
   - Evidence (doc claim): `docs/src/appendix/cli-ref.md:24`, `docs/src/appendix/cli-ref.md:25`
   - Evidence (implementation): `crates/vfx-cli/src/main.rs:259`, `crates/vfx-cli/src/main.rs:263`, `crates/vfx-cli/src/main.rs:267`
   - Impact: опции из справки не работают.

440) В `appendix/cli-ref.md` синтаксис и параметры нескольких команд не совпадают с реальными аргументами CLI (например `-i/--input` для `convert/resize/color/blur`, `--layer` для `convert`, `--interpolation`/`--layer` для `lut`, `-a/-b` для `composite`, `--translate` для `transform`).
   - Evidence (doc claim): `docs/src/appendix/cli-ref.md:46`, `docs/src/appendix/cli-ref.md:48`, `docs/src/appendix/cli-ref.md:63`, `docs/src/appendix/cli-ref.md:67`, `docs/src/appendix/cli-ref.md:90`, `docs/src/appendix/cli-ref.md:111`, `docs/src/appendix/cli-ref.md:153`, `docs/src/appendix/cli-ref.md:179`, `docs/src/appendix/cli-ref.md:180`, `docs/src/appendix/cli-ref.md:207`, `docs/src/appendix/cli-ref.md:232`
   - Evidence (implementation): `crates/vfx-cli/src/main.rs:271`, `crates/vfx-cli/src/main.rs:294`, `crates/vfx-cli/src/main.rs:440`, `crates/vfx-cli/src/main.rs:400`, `crates/vfx-cli/src/main.rs:478`, `crates/vfx-cli/src/main.rs:428`, `crates/vfx-cli/src/main.rs:317`, `crates/vfx-cli/src/main.rs:496`
   - Impact: справка вводит в заблуждение; многие команды не принимают заявленные флаги.

441) В `appendix/cli-ref.md` есть разделы `icc` и `ocio`, но таких команд в CLI нет.
   - Evidence (doc claim): `docs/src/appendix/cli-ref.md:264`, `docs/src/appendix/cli-ref.md:284`
   - Evidence (implementation): `crates/vfx-cli/src/main.rs:150`, `crates/vfx-cli/src/main.rs:220`
   - Impact: команды из справки отсутствуют.

442) В `appendix/cli-ref.md` описаны exit-коды и переменные окружения `VFX_LOG`/`VFX_THREADS`, но в CLI нет явного маппинга exit-кодов, а конфигурация логирования берётся через стандартный `EnvFilter::try_from_default_env()` (без `VFX_LOG`/`VFX_THREADS`).
   - Evidence (doc claim): `docs/src/appendix/cli-ref.md:306`, `docs/src/appendix/cli-ref.md:317`, `docs/src/appendix/cli-ref.md:320`
   - Evidence (implementation): `crates/vfx-cli/src/main.rs:84`, `crates/vfx-cli/src/main.rs:823`
   - Impact: автоматизация на основе документации даёт неверные ожидания.

443) В `appendix/cli-ref.md` заявлена поддержка последовательностей `%04d`/`####` в CLI, но команды принимают `PathBuf` и передают путь напрямую в `load_image` без разборa последовательностей.
   - Evidence (doc claim): `docs/src/appendix/cli-ref.md:325`, `docs/src/appendix/cli-ref.md:331`, `docs/src/appendix/cli-ref.md:335`, `docs/src/appendix/cli-ref.md:340`
   - Evidence (implementation): `crates/vfx-cli/src/commands/info.rs:20`, `crates/vfx-cli/src/commands/info.rs:33`
   - Impact: примеры с шаблонами последовательностей не работают в CLI.

444) В `cli/udim.md` примеры конвертации используют выход `.tx`, но формат `tx` не поддерживается в `vfx-io`.
   - Evidence (doc claim): `docs/src/cli/udim.md:86`, `docs/src/cli/udim.md:191`
   - Evidence (implementation): `crates/vfx-io/src/lib.rs:152`, `crates/vfx-io/src/lib.rs:153`
   - Impact: примеры с `.tx` завершаются ошибкой записи/формата.

445) В `appendix/formats.md` для JPEG указано `CMYK ✗`, но JPEG-ридер поддерживает CMYK вход и конвертирует в RGB.
   - Evidence (doc claim): `docs/src/appendix/formats.md:61`
   - Evidence (implementation): `crates/vfx-io/src/jpeg.rs:155`, `crates/vfx-io/src/jpeg.rs:209`
   - Impact: документация занижает реальную поддержку входных JPEG.

446) В `appendix/formats.md` для CSP (`.csp`) написано `Write ✗`, но в `vfx-lut` экспортируются `write_csp_1d/3d`.
   - Evidence (doc claim): `docs/src/appendix/formats.md:180`
   - Evidence (implementation): `crates/vfx-lut/src/lib.rs:90`
   - Impact: документация занижает реальную поддержку записи CSP.

447) В `appendix/formats.md` для PNG указано «16-bit output when source is float», но `png::write` всегда пишет 8-bit по умолчанию (если явно не задано 16-bit в `PngWriterOptions`).
   - Evidence (doc claim): `docs/src/appendix/formats.md:49`
   - Evidence (implementation): `crates/vfx-io/src/png.rs:679`, `crates/vfx-io/src/png.rs:690`
   - Impact: ожидания по глубине цвета при конвертации не соответствуют факту.

448) В `appendix/color-spaces.md` пример использует `vfx_transfer::srgb_to_linear`/`linear_to_srgb`, но в `vfx-transfer` таких функций нет (есть `srgb::eotf/oetf` и `srgb_eotf/srgb_oetf`).
   - Evidence (doc claim): `docs/src/appendix/color-spaces.md:150`, `docs/src/appendix/color-spaces.md:151`
   - Evidence (implementation): `crates/vfx-transfer/src/lib.rs:70`, `crates/vfx-transfer/src/lib.rs:86`
   - Impact: пример из документации не компилируется.

449) `ImageBuf::contiguous()` всегда возвращает `true`, хотя реальная раскладка данных не проверяется (есть TODO).
   - Evidence (implementation): `crates/vfx-io/src/imagebuf/mod.rs:682`
   - Impact: пользователи могут ошибочно считать буфер непрерывным.

450) В Python-обёртке `pixel_bytes(native)` игнорирует `native` и не учитывает возможные per-channel форматы (TODO), что нарушает заявленную совместимость с OIIO.
   - Evidence (implementation): `crates/vfx-rs-py/src/core.rs:1067`
   - Impact: размер пикселя может быть неверным для non-uniform каналов.

451) В `vfx-compute` заявлены стриминг-источники, но `ExrStreamingSource` всё равно читает весь файл в память; есть TODO на истинный стриминг по тайлам/сканлайнам.
   - Evidence (implementation): `crates/vfx-compute/src/backend/streaming.rs:156`, `crates/vfx-compute/src/backend/streaming.rs:196`
   - Impact: поведение не соответствует ожиданиям «streaming», риск OOM на больших файлах.

452) В `vfx-compute` кэширование GPU-хэндлов объявлено, но фактически не интегрировано (TODO на подключение кэша в execute).
   - Evidence (implementation): `crates/vfx-compute/src/backend/executor.rs:193`
   - Impact: режим viewer заявляет кэширование, но эффект не реализован.

453) Конверсия `vfx_core::Image -> ComputeImage` всегда копирует данные (TODO на zero-copy), что противоречит ожиданию нулевой копии из комментария.
   - Evidence (implementation): `crates/vfx-compute/src/image.rs:224`
   - Impact: лишние аллокации и копии при переходе между слоями API.

454) В `appendix/feature-matrix.md` для TIFF указано «tiles», но в `TiffWriterOptions` нет поддержки тайлов (только bit_depth/ compression), т.е. тайлинг не реализован.
   - Evidence (doc claim): `docs/src/appendix/feature-matrix.md:170`
   - Evidence (implementation): `crates/vfx-io/src/tiff.rs:172`
   - Impact: матрица возможностей завышает поддержку TIFF.

455) В `appendix/feature-matrix.md` заявлено, что JPEG2000 поддерживает запись, но `vfx-io` помечает JP2 как read-only и явно отказывает в write.
   - Evidence (doc claim): `docs/src/appendix/feature-matrix.md:183`
   - Evidence (implementation): `crates/vfx-io/src/jp2.rs:1`, `crates/vfx-io/src/lib.rs:333`
   - Impact: матрица возможностей вводит в заблуждение по поддержке записи JP2.

456) В `appendix/feature-matrix.md` указано, что RED Log3G12 реализован, но в `vfx-transfer` нет функций для Log3G12 (есть только REDLogFilm и Log3G10).
   - Evidence (doc claim): `docs/src/appendix/feature-matrix.md:36`
   - Evidence (implementation): `crates/vfx-transfer/src/red_log.rs:1`, `crates/vfx-transfer/src/red_log.rs:112`
   - Impact: матрица возможностей завышает покрытие log-кривых.

457) В `streaming::ExrStreamingSource` для scanline-файлов идёт lazy-load всего изображения в память (кэш), поэтому стриминг по региону не реализован.
   - Evidence (implementation): `crates/vfx-io/src/streaming/exr.rs:188`, `crates/vfx-io/src/streaming/exr.rs:191`
   - Impact: риск OOM на больших scanline-файлах; отсутствует ожидаемый региональный стриминг.
