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

## Full TODO/FIXME Inventory (auto)

<!-- BEGIN TODO-FIXME INVENTORY -->
- .\crates\vfx-compute\src\backend\executor.rs:193:#[allow(dead_code)]  // TODO: Integrate cache lookup in execute methods
- .\crates\vfx-exr\src\block\chunk.rs:279:        u64::write_le(self.compressed_sample_data_le.len() as u64, write)?; // TODO just guessed
- .\crates\vfx-exr\src\block\chunk.rs:331:        u64::write_le(self.compressed_sample_data_le.len() as u64, write)?; // TODO just guessed
- .\crates\vfx-exr\src\block\lines.rs:126:            .map(move |channel| block.pixel_size.0 * channel.sample_type.bytes_per_sample()) // FIXME is it fewer samples per tile or just fewer tiles for sampled images???
- .\crates\vfx-exr\src\block\lines.rs:15:    // TODO also store enum SampleType, as it would always be matched in every place it is used
- .\crates\vfx-exr\src\block\lines.rs:65:    // TODO be sure this cannot produce incorrect data, as this is not further checked but only handled with panics
- .\crates\vfx-exr\src\block\lines.rs:84:        // FIXME what about sub sampling??
- .\crates\vfx-exr\src\block\lines.rs:88:            // TODO size hint?
- .\crates\vfx-exr\src\block\mod.rs:160:        let expected_byte_size = header.channels.bytes_per_pixel * self.index.pixel_size.area(); // TODO sampling??
- .\crates\vfx-exr\src\block\mod.rs:170:            // FIXME this calculation should not be made here but elsewhere instead (in meta::header?)
- .\crates\vfx-exr\src\block\mod.rs:171:            tile_index: index.pixel_position / header.max_block_pixel_size(), // TODO sampling??
- .\crates\vfx-exr\src\block\mod.rs:209:                    // FIXME this calculation should not be made here but elsewhere instead (in meta::header?)
- .\crates\vfx-exr\src\block\mod.rs:211:                        + header.own_attributes.layer_position.y(), // TODO sampling??
- .\crates\vfx-exr\src\block\mod.rs:235:    /* TODO pub fn lines_mut<'s>(&'s mut self, header: &Header) -> impl 's + Iterator<Item=LineRefMut<'s>> {
- .\crates\vfx-exr\src\block\mod.rs:244:    // TODO from iterator??
- .\crates\vfx-exr\src\block\mod.rs:256:                // TODO subsampling
- .\crates\vfx-exr\src\block\reader.rs:104:        // TODO detect whether the filter actually would skip chunks, and aviod sorting etc when not filtering is applied
- .\crates\vfx-exr\src\block\reader.rs:18:    remaining_reader: PeekRead<Tracking<R>>, // TODO does R need to be Seek or is Tracking enough?
- .\crates\vfx-exr\src\block\reader.rs:255:    // FIXME try async + futures instead of rayon! Maybe even allows for external async decoding? (-> impl Stream<UncompressedBlock>)
- .\crates\vfx-exr\src\block\reader.rs:440:        // TODO remember last chunk index and then seek to index+size and check whether bytes are left?
- .\crates\vfx-exr\src\block\reader.rs:544:            // TODO print warning?
- .\crates\vfx-exr\src\block\reader.rs:550:        let (send, recv) = std::sync::mpsc::channel(); // TODO bounded channel simplifies logic?
- .\crates\vfx-exr\src\block\reader.rs:608:            ); // TODO not reliable
- .\crates\vfx-exr\src\block\reader.rs:83:    // TODO tile indices add no new information to block index??
- .\crates\vfx-exr\src\block\reader.rs:92:        // TODO regardless of pedantic, if invalid, read all chunks instead, and filter after reading each chunk?
- .\crates\vfx-exr\src\block\samples.rs:205:// TODO haven't i implemented this exact behaviour already somewhere else in this library...??
- .\crates\vfx-exr\src\block\writer.rs:109:        // TODO check block order if line order is not unspecified!
- .\crates\vfx-exr\src\block\writer.rs:114:        // TODO debug_assert_eq!(self.is_complete());
- .\crates\vfx-exr\src\block\writer.rs:133:        // TODO check block order if line order is not unspecified!
- .\crates\vfx-exr\src\block\writer.rs:138:        // TODO debug_assert_eq!(self.is_complete());
- .\crates\vfx-exr\src\block\writer.rs:193:        // TODO: use increasing line order where possible, but this requires us to know whether we want to be parallel right now
- .\crates\vfx-exr\src\block\writer.rs:44:    chunk_count: usize, // TODO compose?
- .\crates\vfx-exr\src\block\writer.rs:477:            // TODO print warning?
- .\crates\vfx-exr\src\block\writer.rs:486:        let (send, recv) = std::sync::mpsc::channel(); // TODO bounded channel simplifies logic?
- .\crates\vfx-exr\src\compression\b44\mod.rs:247:// TODO: Unsafe seems to be required to efficiently copy whole slice of u16 ot u8. For now, we use
- .\crates\vfx-exr\src\compression\b44\mod.rs:277:        "expected byte size does not match header" // TODO compute instead of passing argument?
- .\crates\vfx-exr\src\compression\b44\mod.rs:456:            // TODO do not convert endianness for f16-only images
- .\crates\vfx-exr\src\compression\b44\mod.rs:463:                // TODO simplify this and make it memcpy on little endian systems
- .\crates\vfx-exr\src\compression\b44\mod.rs:486:    // TODO do not convert endianness for f16-only images
- .\crates\vfx-exr\src\compression\b44\mod.rs:501:    // TODO do not convert endianness for f16-only images
- .\crates\vfx-exr\src\compression\b44\mod.rs:505:    let uncompressed_le = uncompressed_le.as_slice(); // TODO no alloc
- .\crates\vfx-exr\src\compression\b44\mod.rs:549:            // TODO do not convert endianness for f16-only images
- .\crates\vfx-exr\src\compression\b44\mod.rs:556:                // TODO simplify this and make it memcpy on little endian systems
- .\crates\vfx-exr\src\compression\b44\mod.rs:639:                        // TODO: Make [u8; 2] to u16 fast.
- .\crates\vfx-exr\src\compression\mod.rs:102:    B44, // TODO B44 { optimize_uniform_areas: bool }
- .\crates\vfx-exr\src\compression\mod.rs:118:    B44A, // TODO collapse with B44
- .\crates\vfx-exr\src\compression\mod.rs:123:    DWAA(Option<f32>), // TODO does this have a default value? make this non optional? default Compression Level setting is 45.0
- .\crates\vfx-exr\src\compression\mod.rs:130:    DWAB(Option<f32>), // TODO collapse with DWAA. default Compression Level setting is 45.0
- .\crates\vfx-exr\src\compression\mod.rs:273:        let expected_byte_size = pixel_section.size.area() * header.channels.bytes_per_pixel; // FIXME this needs to account for subsampling anywhere
- .\crates\vfx-exr\src\compression\mod.rs:43:    ZIP1, // TODO ZIP { individual_lines: bool, compression_level: Option<u8> }  // TODO specify zip compression level?
- .\crates\vfx-exr\src\compression\mod.rs:48:    ZIP16, // TODO collapse with ZIP1
- .\crates\vfx-exr\src\compression\mod.rs:78:    PXR24, // TODO specify zip compression level?
- .\crates\vfx-exr\src\compression\piz\huffman.rs:213:            }; // FIXME why does this happen??
- .\crates\vfx-exr\src\compression\piz\huffman.rs:214:            code_bit_count -= short_code.len(); // FIXME may throw "attempted to subtract with overflow"
- .\crates\vfx-exr\src\compression\piz\huffman.rs:22:    let _table_size = usize::try_from(u32::read_le(&mut remaining_compressed)?)?; // TODO check this and return Err?
- .\crates\vfx-exr\src\compression\piz\huffman.rs:301:    // TODO push() into encoding table instead of index stuff?
- .\crates\vfx-exr\src\compression\piz\huffman.rs:341:// TODO Use BitStreamReader for all the bit reads?!
- .\crates\vfx-exr\src\compression\piz\huffman.rs:425:            (*code_bits >> *code_bit_count) as u8, // TODO make sure never or always wraps?
- .\crates\vfx-exr\src\compression\piz\huffman.rs:552:        // TODO slice iteration?
- .\crates\vfx-exr\src\compression\piz\huffman.rs:641:        let mut code = 0_u64; // TODO use foldr?
- .\crates\vfx-exr\src\compression\piz\huffman.rs:652:    // l and the code in code[i]. // TODO iter + filter ?
- .\crates\vfx-exr\src\compression\piz\huffman.rs:739:        links[index] = index; // TODO for x in links.iter().enumerate()
- .\crates\vfx-exr\src\compression\piz\huffman.rs:823:        let mut index = high_position; // TODO fold()
- .\crates\vfx-exr\src\compression\piz\huffman.rs:838:        let mut index = low_position; // TODO fold()
- .\crates\vfx-exr\src\compression\piz\mod.rs:124:    // let out_buffer_size = (max_scan_line_size * scan_line_count) + 65536 + 8192; // TODO not use expected byte size?
- .\crates\vfx-exr\src\compression\piz\mod.rs:138:            // TODO do not convert endianness for f16-only images
- .\crates\vfx-exr\src\compression\piz\mod.rs:157:    // TODO optimize for when all channels are f16!
- .\crates\vfx-exr\src\compression\piz\mod.rs:172:    // TODO do not convert endianness for f16-only images twice
- .\crates\vfx-exr\src\compression\piz\mod.rs:176:    let uncompressed_le = uncompressed_le.as_slice(); // TODO no alloc
- .\crates\vfx-exr\src\compression\piz\mod.rs:219:            // TODO do not convert endianness for f16-only images
- .\crates\vfx-exr\src\compression\piz\mod.rs:32:    expected_byte_size: usize, // TODO remove expected byte size as it can be computed with `rectangle.size.area() * channels.bytes_per_pixel`
- .\crates\vfx-exr\src\compression\piz\mod.rs:48:    let mut bitmap = vec![0_u8; BITMAP_SIZE]; // FIXME use bit_vec!
- .\crates\vfx-exr\src\compression\piz\mod.rs:70:            // TODO length might be smaller than remaining??
- .\crates\vfx-exr\src\compression\piz\wavelet.rs:146:    let mut p: usize = 1; // TODO i32?
- .\crates\vfx-exr\src\compression\piz\wavelet.rs:147:    let mut p2: usize; // TODO i32?
- .\crates\vfx-exr\src\compression\piz\wavelet.rs:188:                buffer[position_x] = center; // TODO rustify
- .\crates\vfx-exr\src\compression\piz\wavelet.rs:245:    (m as u16, d as u16) // TODO explicitly wrap?
- .\crates\vfx-exr\src\compression\piz\wavelet.rs:256:    let a = ai as i16; // TODO explicitly wrap?
- .\crates\vfx-exr\src\compression\piz\wavelet.rs:257:    let b = (ai - hi) as i16; // TODO explicitly wrap?
- .\crates\vfx-exr\src\compression\piz\wavelet.rs:259:    (a as u16, b as u16) // TODO explicitly wrap?
- .\crates\vfx-exr\src\compression\piz\wavelet.rs:279:    (m as u16, d as u16) // TODO explicitly wrap?
- .\crates\vfx-exr\src\compression\piz\wavelet.rs:289:    (a as u16, b as u16) // TODO explicitly wrap?
- .\crates\vfx-exr\src\compression\piz\wavelet.rs:37:    let mut p: usize = 1; // TODO i32?
- .\crates\vfx-exr\src\compression\piz\wavelet.rs:38:    let mut p2: usize = 2; // TODO what is p??
- .\crates\vfx-exr\src\compression\piz\wavelet.rs:48:            // TODO: for py in (index..ey).nth(offset_2.0)
- .\crates\vfx-exr\src\compression\piz\wavelet.rs:75:                buffer[position_x] = center; // TODO rustify
- .\crates\vfx-exr\src\compression\pxr24.rs:173:        .map_err(|_| Error::invalid("zlib-compressed data malformed"))?; // TODO share code with zip?
- .\crates\vfx-exr\src\compression\pxr24.rs:43:    let mut remaining_bytes_ne = bytes_ne.as_slice(); // TODO less allocation
- .\crates\vfx-exr\src\compression\pxr24.rs:60:        // TODO this loop should be an iterator in the `IntegerBounds` class, as it is used in all compression methods
- .\crates\vfx-exr\src\error.rs:29:    Aborted, // FIXME remove?? is not used really?
- .\crates\vfx-exr\src\error.rs:68:// TODO use `usize::try_from(x)?` everywhere
- .\crates\vfx-exr\src\image\channel_groups.rs:116:        // TODO what about empty groups with NO channels??
- .\crates\vfx-exr\src\image\channel_groups.rs:129:        own_channels.sort_unstable(); // TODO only once at end
- .\crates\vfx-exr\src\image\channel_groups.rs:141:            .expect("empty channel groups (check failed)"); // TODO only happens for empty channels, right? panic maybe?
- .\crates\vfx-exr\src\image\channel_groups.rs:187:            let channel_reader = &mut blocks_per_channel[line.location.channel]; // TODO subsampling
- .\crates\vfx-exr\src\image\channel_groups.rs:222:            let mut channels_header = header.clone(); // TODO no clone?
- .\crates\vfx-exr\src\image\channel_groups.rs:227:            }).collect()); // FIXME does not comply to `header.chunk_count` and that stuff?? change ReadChannels fn signature?
- .\crates\vfx-exr\src\image\channel_groups.rs:231:            // FIXME this is not the original order indexed_channels.len() - 1
- .\crates\vfx-exr\src\image\crop.rs:165:// TODO make cropped view readable if you only need a specific section of the image?
- .\crates\vfx-exr\src\image\crop.rs:302:                            trimmed_lines.flatten().map(|x| *x).collect() // TODO does this use memcpy?
- .\crates\vfx-exr\src\image\crop.rs:414:    // TODO add 1px margin to avoid interpolation issues?
- .\crates\vfx-exr\src\image\mod.rs:1194:            // TODO dedup with above
- .\crates\vfx-exr\src\image\mod.rs:1205:                    allow_lossy: other.encoding.compression.may_loose_data(), // TODO check specific channels sample types
- .\crates\vfx-exr\src\image\mod.rs:1431:    /*TODO
- .\crates\vfx-exr\src\image\mod.rs:1438:        } // TODO no clone?
- .\crates\vfx-exr\src\image\mod.rs:156:    pub channels: ChannelsDescription, // TODO this is awkward. can this be not a type parameter please? maybe vec<option<chan_info>> ??
- .\crates\vfx-exr\src\image\mod.rs:159:    // TODO should also support `Levels<YourStorage>`, where levels are desired!
- .\crates\vfx-exr\src\image\mod.rs:160:    pub pixels: Pixels, // TODO rename to "pixels"?
- .\crates\vfx-exr\src\image\mod.rs:1729:            // TODO check out more nested behaviour!
- .\crates\vfx-exr\src\image\mod.rs:1748:                    line_order: Increasing, // FIXME unspecified may be optimized to increasing, which destroys test eq
- .\crates\vfx-exr\src\image\mod.rs:661:        list.sort_unstable_by_key(|channel| channel.name.clone()); // TODO no clone?
- .\crates\vfx-exr\src\image\mod.rs:666:// FIXME check content size of layer somewhere??? before writing?
- .\crates\vfx-exr\src\image\mod.rs:698:    // TODO storage order for RIP maps?
- .\crates\vfx-exr\src\image\mod.rs:745:    // TODO simplify working with levels in general! like level_size_by_index and such
- .\crates\vfx-exr\src\image\mod.rs:840:    // TODO test pls wtf
- .\crates\vfx-exr\src\image\read\any_samples.rs:347:            // TODO: Support multiple deep layers
- .\crates\vfx-exr\src\image\read\deep.rs:329:        // TODO: Optimize by caching block data
- .\crates\vfx-exr\src\image\read\image.rs:111:    // TODO Use Parallel<> Wrapper to only require sendable byte source where parallel decompression is required
- .\crates\vfx-exr\src\image\read\image.rs:125:    // TODO Use Parallel<> Wrapper to only require sendable byte source where parallel decompression is required
- .\crates\vfx-exr\src\image\read\image.rs:151:        // TODO propagate send requirement further upwards
- .\crates\vfx-exr\src\image\read\layers.rs:178:    layer_readers: SmallVec<[LayerReader<ChannelsReader>; 2]>, // TODO unpack struct?
- .\crates\vfx-exr\src\image\read\levels.rs:165:    // TODO specific channels for multiple resolution levels
- .\crates\vfx-exr\src\image\read\levels.rs:529:                    // TODO put this into Levels::new(..) ?
- .\crates\vfx-exr\src\image\read\levels.rs:56:// FIXME rgba levels???
- .\crates\vfx-exr\src\image\read\mod.rs:131:// FIXME Set and Create should not need to be static
- .\crates\vfx-exr\src\image\read\mod.rs:142:    Create: Fn(Vec2<usize>, &RgbaChannels) -> Pixels, // TODO type alias? CreateRgbaPixels<Pixels=Pixels>,
- .\crates\vfx-exr\src\image\read\mod.rs:164:// FIXME Set and Create should not need to be static
- .\crates\vfx-exr\src\image\read\mod.rs:175:    Create: Fn(Vec2<usize>, &RgbaChannels) -> Pixels, // TODO type alias? CreateRgbaPixels<Pixels=Pixels>,
- .\crates\vfx-exr\src\image\read\mod.rs:245:    // TODO
- .\crates\vfx-exr\src\image\read\samples.rs:138:            resolution, // TODO sampling
- .\crates\vfx-exr\src\image\read\samples.rs:23:    // TODO
- .\crates\vfx-exr\src\image\read\specific_channels.rs:176:        let channel_descriptions = pixel_reader.get_descriptions().into_non_recursive();// TODO not call this twice
- .\crates\vfx-exr\src\image\read\specific_channels.rs:214:    } // TODO all levels
- .\crates\vfx-exr\src\image\read\specific_channels.rs:217:        let mut pixels = vec![PxReader::RecursivePixel::default(); block.index.pixel_size.width()]; // TODO allocate once in self
- .\crates\vfx-exr\src\image\read\specific_channels.rs:229:            // TODO sampling
- .\crates\vfx-exr\src\image\read\specific_channels.rs:367:        let mut own_bytes_reader = &mut &bytes[start_index..start_index + byte_count]; // TODO check block size somewhere
- .\crates\vfx-exr\src\image\write\channels.rs:141:        vec.sort_unstable_by_key(|channel: &ChannelDescription| channel.name.clone()); // TODO no clone?
- .\crates\vfx-exr\src\image\write\channels.rs:155:        (LevelMode::Singular, RoundingMode::Down) // TODO
- .\crates\vfx-exr\src\image\write\channels.rs:182:    channels: &'channels SpecificChannels<Storage, Channels>, // TODO this need not be a reference?? impl writer for specific_channels directly?
- .\crates\vfx-exr\src\image\write\channels.rs:32:    fn extract_uncompressed_block(&self, header: &Header, block: BlockIndex) -> Vec<u8>; // TODO return uncompressed block?
- .\crates\vfx-exr\src\image\write\channels.rs:360:            // TODO does this boil down to a `memcpy` where the sample type equals the type parameter?
- .\crates\vfx-exr\src\image\write\channels.rs:409:    // TODO impl exact size iterator <item = Self::Pixel>
- .\crates\vfx-exr\src\image\write\layers.rs:113:        smallvec![header] // TODO no array-vs-first
- .\crates\vfx-exr\src\image\write\layers.rs:120:            .create_writer(headers.first().expect("inferred header error")); // TODO no array-vs-first
- .\crates\vfx-exr\src\image\write\layers.rs:133:        // TODO no array-vs-first
- .\crates\vfx-exr\src\image\write\layers.rs:144:        // TODO no array-vs-first
- .\crates\vfx-exr\src\image\write\layers.rs:166:        headers.push(self.value.infer_headers(image_attributes).remove(0)); // TODO no unwrap
- .\crates\vfx-exr\src\image\write\layers.rs:183:            ), // TODO no slice
- .\crates\vfx-exr\src\image\write\layers.rs:208:        // TODO no slice?
- .\crates\vfx-exr\src\image\write\layers.rs:64:        .collect() // TODO no array-vs-first
- .\crates\vfx-exr\src\image\write\layers.rs:74:            .zip(headers.chunks_exact(1)) // TODO no array-vs-first
- .\crates\vfx-exr\src\image\write\mod.rs:115:        // TODO this should perform all validity checks? and none after that?
- .\crates\vfx-exr\src\image\write\mod.rs:33:// TODO explain pixel tuple f32,f16,u32
- .\crates\vfx-exr\src\image\write\mod.rs:57:// TODO explain pixel tuple f32,f16,u32
- .\crates\vfx-exr\src\image\write\samples.rs:186:                        // TODO store level size in image??
- .\crates\vfx-exr\src\image\write\samples.rs:256:            .expect("invalid level index") // TODO compute level size from line index??
- .\crates\vfx-exr\src\io.rs:228:        let delta = target_position as i128 - self.position as i128; // FIXME  panicked at 'attempt to subtract with overflow'
- .\crates\vfx-exr\src\io.rs:232:            // TODO profile that this is indeed faster than a syscall! (should be because of bufread buffer discard)
- .\crates\vfx-exr\src\io.rs:45:            // FIXME deletes existing file if creation of new file fails?
- .\crates\vfx-exr\src\math.rs:170:// TODO does rust std not provide this?
- .\crates\vfx-exr\src\math.rs:174:    // TODO check if this unrolls properly?
- .\crates\vfx-exr\src\math.rs:185:// TODO does rust std not provide this?
- .\crates\vfx-exr\src\math.rs:190:    // TODO check if this unrolls properly
- .\crates\vfx-exr\src\math.rs:243:// TODO log2 tests
- .\crates\vfx-exr\src\math.rs:80:    // TODO use this!
- .\crates\vfx-exr\src\meta\attribute.rs:109:// TODO is this ascii? use a rust ascii crate?
- .\crates\vfx-exr\src\meta\attribute.rs:1104:    // FIXME this must be used everywhere
- .\crates\vfx-exr\src\meta\attribute.rs:1172:        self.name.validate(true, None)?; // TODO spec says this does not affect `requirements.long_names` but is that true?
- .\crates\vfx-exr\src\meta\attribute.rs:1201:            // TODO this must only be implemented in the crate::image module and child modules,
- .\crates\vfx-exr\src\meta\attribute.rs:1773:// TODO instead of pre calculating byte size, write to a tmp buffer whose length is inspected before actually writing?
- .\crates\vfx-exr\src\meta\attribute.rs:1993:        // TODO: don't read into an array at all, just read directly from the reader and optionally seek afterwards?
- .\crates\vfx-exr\src\meta\attribute.rs:234:    // FIXME this needs to account for subsampling anywhere?
- .\crates\vfx-exr\src\meta\attribute.rs:235:    pub bytes_per_pixel: usize, // FIXME only makes sense for flat images!
- .\crates\vfx-exr\src\meta\attribute.rs:354:// TODO is this linear?
- .\crates\vfx-exr\src\meta\attribute.rs:688:/* TODO (currently conflicts with From<&str>)
- .\crates\vfx-exr\src\meta\attribute.rs:984:        // TODO rename to "move" or "translate"?
- .\crates\vfx-exr\src\meta\header.rs:1048:            .map(|(name, val)| (name.as_slice(), val.clone())); // TODO no clone
- .\crates\vfx-exr\src\meta\header.rs:1062:        let max_string_len = if requirements.has_long_names { 256 } else { 32 }; // TODO DRY this information
- .\crates\vfx-exr\src\meta\header.rs:119:    // TODO same for all layers?
- .\crates\vfx-exr\src\meta\header.rs:122:    // TODO same for all layers?
- .\crates\vfx-exr\src\meta\header.rs:1236:            // FIXME dwa compression level gets lost if any other compression is used later in the process
- .\crates\vfx-exr\src\meta\header.rs:1463:        // debug.finish_non_exhaustive() TODO
- .\crates\vfx-exr\src\meta\header.rs:157:    // TODO parse!
- .\crates\vfx-exr\src\meta\header.rs:393:        // TODO without box?
- .\crates\vfx-exr\src\meta\header.rs:469:        vec.into_iter() // TODO without collect
- .\crates\vfx-exr\src\meta\header.rs:5:use crate::meta::attribute::*; // FIXME shouldn't this need some more imports????
- .\crates\vfx-exr\src\meta\header.rs:520:    // TODO reuse this function everywhere
- .\crates\vfx-exr\src\meta\header.rs:786:            return Err(Error::invalid("chunk count attribute")); // TODO this may be an expensive check?
- .\crates\vfx-exr\src\meta\header.rs:9:// TODO rename header to LayerDescription!
- .\crates\vfx-exr\src\meta\mod.rs:154:    // TODO check if exr before allocating BufRead
- .\crates\vfx-exr\src\meta\mod.rs:219:// TODO use this method everywhere instead of convoluted formulas
- .\crates\vfx-exr\src\meta\mod.rs:22:// TODO rename MetaData to ImageInfo?
- .\crates\vfx-exr\src\meta\mod.rs:238:// TODO this should be cached? log2 may be very expensive
- .\crates\vfx-exr\src\meta\mod.rs:244:// TODO this should be cached? log2 may be very expensive
- .\crates\vfx-exr\src\meta\mod.rs:255:// TODO cache these?
- .\crates\vfx-exr\src\meta\mod.rs:256:// TODO compute these directly instead of summing up an iterator?
- .\crates\vfx-exr\src\meta\mod.rs:262:        // TODO progressively divide instead??
- .\crates\vfx-exr\src\meta\mod.rs:271:// TODO cache all these level values when computing table offset size??
- .\crates\vfx-exr\src\meta\mod.rs:272:// TODO compute these directly instead of summing up an iterator?
- .\crates\vfx-exr\src\meta\mod.rs:278:        // TODO progressively divide instead??
- .\crates\vfx-exr\src\meta\mod.rs:321:        // TODO cache all these level values??
- .\crates\vfx-exr\src\meta\mod.rs:35:    // TODO rename to layer descriptions?
- .\crates\vfx-exr\src\meta\mod.rs:394:        // TODO check if supporting requirements 2 always implies supporting requirements 1
- .\crates\vfx-exr\src\meta\mod.rs:450:    // TODO use seek for large (probably all) tables!
- .\crates\vfx-exr\src\meta\mod.rs:453:        crate::io::skip_bytes(read, chunk_count * u64::BYTE_SIZE)?; // TODO this should seek for large tables
- .\crates\vfx-exr\src\meta\mod.rs:528:        // TODO validation fn!
- .\crates\vfx-exr\src\meta\mod.rs:617:            // TODO test if this correctly detects unsupported files
- .\crates\vfx-exr\src\meta\mod.rs:64:    // TODO write version 1 for simple images
- .\crates\vfx-exr\src\view\app.rs:243:                                    // TODO: need P.xyz channels, for now use depth as Y
- .\crates\vfx-io\src\imagebuf\mod.rs:520:        // TODO: Read from file if needed
- .\crates\vfx-io\src\imagebuf\mod.rs:526:        // TODO: Read from file if needed
- .\crates\vfx-io\src\imagebuf\mod.rs:678:        // TODO: Check actual storage layout
- .\crates\vfx-io\src\imagebufalgo\ocio.rs:510:    _unpremult: bool,  // TODO: implement unpremult support
- .\crates\vfx-rs-py\src\core.rs:1067:        let _ = native; // TODO: per-channel formats
- .\crates\vfx-rs-py\src\core.rs:1240:    #[allow(deprecated)]  // TODO: Migrate to IntoPyObject
- .\crates\vfx-rs-py\src\core.rs:925:// TODO: Migrate to IntoPyObject when pyo3 0.24 stabilizes
<!-- END TODO-FIXME INVENTORY -->







