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
