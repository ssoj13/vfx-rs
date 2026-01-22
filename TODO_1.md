# TODO_1: Bugs 1-25

## Status Legend
- [ ] Not started
- [x] Fixed
- [~] Partial / In progress
- [!] Won't fix / By design

---

## 1) FileTransform supports only a small subset of OCIO file formats
- [ ] **Status:** Not started
- **Evidence (OCIO formats registry):** `_ref/OpenColorIO/src/OpenColorIO/transforms/FileTransform.cpp:333`
- **Evidence (vfx-ocio FileTransform extension match):** `crates/vfx-ocio/src/processor.rs:963`
- **Impact:** FileTransform with 3DL/CC/CCC/CDL/CSP/Discreet1DL/HDL/ICC/Iridas/Resolve/Pandora/SpiMtx/Truelight/VF will be ignored (no-op).
- **Notes:**

---

## 2) Config parser does not load several OCIO transform types
- [ ] **Status:** Not started
- **Missing tags in parser:** `Lut1DTransform`, `Lut3DTransform`, `ExponentWithLinearTransform`, `DisplayViewTransform`, `GradingHueCurveTransform`.
- **Evidence (parser has no tags):** `crates/vfx-ocio/src/config.rs`
- **Evidence (types exist in Rust):** `crates/vfx-ocio/src/transform.rs`
- **Evidence (OCIO TransformType list):** `_ref/OpenColorIO/include/OpenColorIO/OpenColorTypes.h:361`
- **Notes:**

---

## 3) ExponentWithLinearTransform negative handling diverges from OCIO
- [ ] **Status:** Not started
- **OCIO:** "Negative values are never clamped." `_ref/OpenColorIO/include/OpenColorIO/OpenColorTransforms.h:900`
- **vfx-ocio default clamps negatives via NegativeStyle::Clamp.**
- **Evidence (transform defaults):** `crates/vfx-ocio/src/transform.rs:534`
- **Evidence (processor behavior):** `crates/vfx-ocio/src/processor.rs:2096`
- **Notes:**

---

## 4) BuiltinTransform coverage is minimal compared to OCIO registry
- [ ] **Status:** Not started
- **vfx-ocio supports only a small subset (ACES core, a few camera log->ACES, sRGB->XYZ).**
- **Evidence (vfx builtin map):** `crates/vfx-ocio/src/builtin_transforms.rs`
- **Evidence (OCIO builtins registry across cameras/displays):** `_ref/OpenColorIO/src/OpenColorIO/transforms/builtins/*.cpp`
- **Impact:** unknown builtin styles become no-op in processor.
- **Notes:**

---

## 5) FileTransform ccc_id is unused
- [ ] **Status:** Not started
- **vfx-ocio FileTransform has `ccc_id` but processor does not use it.**
- **Evidence (ccc_id field):** `crates/vfx-ocio/src/transform.rs`
- **Evidence (processor FileTransform match):** `crates/vfx-ocio/src/processor.rs:963`
- **Impact:** cannot select specific CC/CCC/CDL entries by ID.
- **Notes:**

---

## 6) vfx-io format coverage is far smaller than OpenImageIO plugins
- [ ] **Status:** Not started
- **OIIO has many imageio plugins (bmp, cineon, dicom, ffmpeg, fits, gif, ico, iff, jpeg2000, jpegxl, openvdb, pnm, ptex, raw, r3d, rla, sgi, softimage, targa, term, zfile, etc.).**
- **Evidence (plugin dirs):** `_ref/OpenImageIO/src/*.imageio`
- **vfx-io Format enum/detect only includes exr/png/jpeg/tiff/dpx/hdr/heif/webp/avif/jp2/arri/redcode.**
- **Evidence (format detection):** `crates/vfx-io/src/detect.rs`
- **Notes:**

---

## 7) Several vfx-io formats are feature-gated or stubbed
- [ ] **Status:** Not started
- **arriraw/redcode return UnsupportedFeature; heif/webp/jp2 gated by features; avif read requires dav1d; jp2 write not supported.**
- **Evidence (dispatch):** `crates/vfx-io/src/lib.rs`
- **Notes:**

---

## 8) ImageBuf spec reading is incomplete (assumes RGBA, ignores full metadata for most formats)
- [ ] **Status:** Not started
- **vfx-io ImageBuf `ensure_spec_read` uses probe_dimensions and hard-codes `nchannels=4` and `full_*` for all formats, only tries EXR headers for subimages.**
- **Evidence (vfx-io behavior):** `crates/vfx-io/src/imagebuf/mod.rs:1480`
- **OIIO ImageBuf reads full ImageSpec via ImageInput and does not assume RGBA.**
- **Evidence (OIIO ImageBuf design):** `_ref/OpenImageIO/src/include/OpenImageIO/imagebuf.h:66`
- **Notes:**

---

## 9) vfx-io read/write APIs lack OIIO-style subimage/miplevel/scanline/tile and deep access
- [ ] **Status:** Not started
- **vfx-io FormatReader/Writer only expose whole-image read/write from path or memory.**
- **Evidence (traits):** `crates/vfx-io/src/traits.rs`
- **OIIO ImageInput supports subimage/miplevel selection, scanline/tile reads, and deep reads.**
- **Evidence (OIIO API):** `_ref/OpenImageIO/src/include/OpenImageIO/imageio.h:1166`
- **Notes:**

---

## 10) ImageBuf `read()` ignores subimage/miplevel parameters
- [ ] **Status:** Not started
- **Method accepts subimage/miplevel, but `ensure_pixels_read` always calls `crate::read(&name)` with no subimage/miplevel selection.**
- **Evidence (ImageBuf read path):** `crates/vfx-io/src/imagebuf/mod.rs:760`
- **Notes:**

---

## 11) TextureSystem sampling falls back for trilinear/anisotropic in `sample()`
- [ ] **Status:** Not started
- **`sample()` uses bilinear when filter is Trilinear/Anisotropic due to missing derivatives.**
- **Evidence (implementation):** `crates/vfx-io/src/texture.rs:110`
- **OIIO TextureSystem uses derivative-based LOD and anisotropy in texture() API.**
- **Evidence (OIIO API):** `_ref/OpenImageIO/src/include/OpenImageIO/texture.h:897`
- **Notes:**

---

## 12) No capability query API (supports/features) in vfx-io registry/traits
- [ ] **Status:** Not started
- **OIIO ImageInput/ImageOutput expose `supports("...")` for metadata, multiimage, mipmap, ioproxy, thumbnails, etc.**
- **Evidence (OIIO supports API):** `_ref/OpenImageIO/src/include/OpenImageIO/imageio.h:1131` and `_ref/OpenImageIO/src/include/OpenImageIO/imageio.h:2545`
- **vfx-io FormatReader/FormatWriter/FormatInfo expose only format name, extensions, can_read, read/write paths.**
- **Evidence (traits):** `crates/vfx-io/src/traits.rs:128` and `crates/vfx-io/src/traits.rs:218`
- **Evidence (registry info):** `crates/vfx-io/src/registry.rs:44`
- **Impact:** callers cannot detect format capabilities (multiimage, mipmap, ioproxy, thumbnails, metadata limits).
- **Notes:**

---

## 13) Deep read API is not part of the unified vfx-io interfaces
- [ ] **Status:** Not started
- **OIIO ImageInput exposes deep read entry points and reports `"deepdata"` support.**
- **Evidence (deep reads):** `_ref/OpenImageIO/src/include/OpenImageIO/imageio.h:1605`
- **Evidence (supports "deepdata"):** `_ref/OpenImageIO/src/include/OpenImageIO/imageio.h:2496`
- **vfx-io deep is only exposed via EXR-specific functions, not in `FormatReader`/`FormatRegistry`.**
- **Evidence (EXR deep entry points):** `crates/vfx-io/src/exr.rs:888`
- **Impact:** generic deep workflows cannot be implemented via `vfx-io::read`/registry; deep is format-specific only.
- **Notes:**

---

## 14) ImageCache ignores subimage/multiimage data despite exposing subimage in API
- [ ] **Status:** Not started
- **`get_tile()` takes `subimage`, but `load_tile()` ignores it and full-loads via `crate::read(path)`.**
- **Evidence (unused subimage parameter):** `crates/vfx-io/src/cache.rs:405`
- **Evidence (full read path):** `crates/vfx-io/src/cache.rs:435`
- **CachedImageInfo hard-codes `subimages: 1` even for files with multiple subimages.**
- **Evidence (subimages fixed to 1):** `crates/vfx-io/src/cache.rs:333`
- **Impact:** multiimage/multipart files cannot be addressed correctly; cache API is misleading.
- **Notes:**

---

## 15) ImageCache streaming mode does not support mip levels
- [ ] **Status:** Not started
- **`load_tile()` returns an UnsupportedOperation error if `mip_level > 0` in streaming mode.**
- **Evidence (mip restriction):** `crates/vfx-io/src/cache.rs:460`
- **Impact:** `get_tile()` cannot serve mip tiles for large images that trigger streaming; callers must handle errors or get incorrect LOD behavior.
- **Notes:**

---

## 16) ImageBuf `contiguous()` always returns true (TODO left unresolved)
- [ ] **Status:** Not started
- **The method returns `true` unconditionally and has a TODO to check real storage layout.**
- **Evidence:** `crates/vfx-io/src/imagebuf/mod.rs:692`
- **Impact:** callers may assume contiguous layout and perform invalid memcpy/stride math on non-contiguous buffers.
- **Notes:**

---

## 17) Streaming API reports source channel count, but Region data is always RGBA
- [ ] **Status:** Not started
- **`Region` is defined as RGBA f32-only; `RGBA_CHANNELS` is fixed to 4.**
- **Evidence (Region contract):** `crates/vfx-io/src/streaming/traits.rs:46`
- **StreamingSource implementations return original channel count (e.g., EXR/Memory/TIFF).**
- **Evidence (EXR channels):** `crates/vfx-io/src/streaming/exr.rs:272`
- **Evidence (MemorySource channels):** `crates/vfx-io/src/streaming/source.rs:235`
- **Impact:** callers can misinterpret Region layout or allocate incorrect buffers based on `channels()`.
- **Notes:**

---

## 18) ImageCache streaming path can panic for images with >4 channels
- [ ] **Status:** Not started
- **Streaming Region is RGBA, but `load_tile()` uses `channels` to index `rgba[c]`.**
- **Evidence (loop indexes rgba by channels):** `crates/vfx-io/src/cache.rs:455`
- **Evidence (Region is RGBA):** `crates/vfx-io/src/streaming/traits.rs:46`
- **Impact:** if `channels > 4` (AOVs, extra EXR channels), indexing past RGBA causes panic.
- **Notes:**

---

## 19) ExponentTransform ignores OCIO negativeStyle setting in config parsing
- [ ] **Status:** Not started
- **OCIO allows setting NegativeStyle for ExponentTransform in configs > v1.**
- **Evidence (OCIO API):** `_ref/OpenColorIO/include/OpenColorIO/OpenColorTransforms.h:900`
- **vfx-ocio parser hard-codes `negative_style: NegativeStyle::Clamp` and never reads a YAML field.**
- **Evidence (parser behavior):** `crates/vfx-ocio/src/config.rs:664`
- **Impact:** configs that rely on pass-through/mirror negative handling are parsed incorrectly.
- **Notes:**

---

## 20) ImageBuf read-only paths do not load pixels; const APIs can return zeroed data silently
- [ ] **Status:** Not started
- **`ensure_pixels_read_ref()` always returns false and never loads pixels for read-only/cache-backed buffers.**
- **Evidence (no-op read ref):** `crates/vfx-io/src/imagebuf/mod.rs:1605`
- **`to_image_data()` ignores the boolean result and reads from `PixelStorage`, which is `Empty` by default and yields zeros.**
- **Evidence (to_image_data ignores load):** `crates/vfx-io/src/imagebuf/mod.rs:1407`
- **Evidence (Empty returns 0.0):** `crates/vfx-io/src/imagebuf/storage.rs:198`
- **Impact:** calling `write()` or `to_image_data()` on an ImageBuf that hasn't been mutably read yields blank output without error.
- **Notes:**

---

## 21) GradingRgbCurveTransform ignores direction in processor
- [ ] **Status:** Not started
- **The transform has a direction field, but compile step always bakes forward curves.**
- **Evidence (transform has direction):** `crates/vfx-ocio/src/transform.rs:948`
- **Evidence (processor ignores direction):** `crates/vfx-ocio/src/processor.rs:1136`
- **Impact:** inverse grading curves are treated as forward, producing incorrect results.
- **Notes:**

---

## 22) LogCameraTransform linear slope can divide by zero without checks
- [ ] **Status:** Not started
- **The linear slope formula divides by `ln(base) * (lin_side_break * lin_side_slope + lin_side_offset)`.**
- **Evidence (no zero guard):** `crates/vfx-ocio/src/processor.rs:1209`
- **Impact:** configs with zero/near-zero denominator yield inf/NaN and break processing.
- **Notes:**

---

## 23) vfx-lut Lut1D cannot represent per-channel domain min/max from .cube
- [ ] **Status:** Not started
- **.cube supports `DOMAIN_MIN`/`DOMAIN_MAX` with 3 values (per-channel), but Lut1D stores scalar `domain_min`/`domain_max`.**
- **Evidence (scalar domain):** `crates/vfx-lut/src/lut1d.rs:42`
- **Evidence (parser drops G/B):** `crates/vfx-lut/src/cube.rs:96`
- **Impact:** LUTs with non-uniform domain scaling are parsed incorrectly.
- **Notes:**

---

## 24) FileTransform uses CLF parser for .ctf files
- [ ] **Status:** Not started
- **Processor treats `"ctf"` the same as `"clf"` and always calls `read_clf`.**
- **Evidence (FileTransform branch):** `crates/vfx-ocio/src/processor.rs:987`
- **vfx-lut provides a separate CTF parser (`read_ctf`).**
- **Evidence (CTF API):** `crates/vfx-lut/src/lib.rs:75`
- **Impact:** valid .ctf files can fail to parse or be interpreted incorrectly.
- **Notes:**

---

## 25) ImageBuf write ignores `fileformat` hint and per-buffer write settings
- [ ] **Status:** Not started
- **`write()` ignores `_fileformat` and does not apply `write_format`/`write_tiles`; it always converts to ImageData and calls `crate::write`.**
- **Evidence (unused args and path):** `crates/vfx-io/src/imagebuf/mod.rs:807`
- **Impact:** callers cannot force output format or tiling through ImageBuf API despite setter methods.
- **Notes:**

---
