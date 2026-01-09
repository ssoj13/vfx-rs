# TODO8: OIIO Parity Checklist

Complete parity with OpenImageIO (OIIO) for vfx-rs.

## Summary

| Component | Our Count | OIIO Count | Status |
|-----------|-----------|------------|--------|
| ImageBufAlgo Functions | 233 | ~85 unique | **EXCEEDS** (we have more via Rust idioms) |
| Missing OIIO Functions | - | 12 | **NEED TO ADD** |
| Image Formats | 12 | 50+ | **75%** (plugins vs native) |
| OCIO Integration | Partial | Full | **80%** |
| TextureSystem | Partial | Full | **60%** |
| ImageCache | Basic | Full | **50%** |

---

## 1. ImageBufAlgo Missing Functions (PRIORITY: HIGH)

Functions present in OIIO but missing in vfx-io:

### 1.1 Patterns & Generation
- [x] `bluenoise_image()` - Blue noise texture for dithering

### 1.2 Utility Functions
- [x] `copy()` - Copy with optional type conversion
- [x] `text_size()` - Get text bounding box without rendering
- [x] `scale()` - Scale image by factor (different from mul)

### 1.3 Statistics & Analysis
- [x] `nonzero_region()` - Find ROI of non-zero pixels
- [x] `pixel_hash()` - FNV-1a hash of pixel data
- [x] `compare_yee()` - Perceptual image comparison (Yee algorithm)

### 1.4 Repair & Cleanup
- [x] `fix_non_finite()` - Replace NaN/Inf with valid values

### 1.5 Additional Filters
OIIO has kernel-based filtering we cover differently. Verify:
- [x] `median_filter` - We have `median()`
- [x] `unsharp_mask` - We have it
- [x] `dilate` / `erode` - We have them

---

## 2. Extra Functions We Have (Not in OIIO Core)

Our additions (keep them):
- `blur()` - Gaussian blur (OIIO uses make_kernel + convolve)
- `box_blur()` - Box blur
- `sharpen()` - Direct sharpen
- `sobel()` - Edge detection
- `bilateral()` - Edge-preserving filter
- `morph_open()`, `morph_close()` - Morphological ops
- `morph_gradient()`, `top_hat()`, `black_hat()` - Advanced morphology
- `dilate_n()`, `erode_n()` - Iterative morphology
- `srgb_to_linear()`, `linear_to_srgb()` - Direct color conversion
- `render_circle()`, `render_ellipse()`, `render_polygon()` - Extended drawing
- Extended blend modes: `screen`, `multiply`, `overlay`, `hardlight`, `softlight`, `difference`, `exclusion`, `colordodge`, `colorburn`, `add_blend`
- Deep image extensions: `deep_tidy()`, `deep_stats()`, `deep_holdout_matte()`
- `unique_color_count()` - Count unique colors
- Matrix utilities: `matrix_*` functions

---

## 3. Image Formats

### 3.1 Currently Supported (12 formats)
- [x] EXR (via vfx-exr, deep data support)
- [x] PNG
- [x] JPEG
- [x] TIFF
- [x] DPX
- [x] HDR/RGBE
- [x] WebP (via image crate)
- [x] AVIF (via image crate, write-only without dav1d)
- [x] JPEG2000 (via jpeg2k, requires OpenJPEG)
- [x] HEIF/HEIC (via libheif-rs)
- [x] ARRI RAW (metadata only, SDK required for decode)
- [x] RED REDCODE (metadata only, SDK required for decode)

### 3.2 GPU Texture Formats (DONE)
- [x] **PSD** - Photoshop format (layers, masks) - `psd` feature
- [x] **DDS** - DirectDraw Surface (BC1-BC7 decompression) - `dds` feature
- [x] **KTX2** - Khronos Texture (uncompressed, f16/f32) - `ktx` feature

### 3.3 Low Priority / OIIO Plugin Formats
OIIO supports these via plugins, we can add if needed:
- [ ] BMP - Simple, low priority
- [ ] GIF - Animation support needed
- [ ] ICO - Windows icons
- [ ] IFF - Interchange File Format
- [ ] OpenVDB - Volume data (separate crate recommended)
- [ ] PNM/PPM/PGM - Simple text formats
- [ ] PIC - Softimage
- [ ] RAW (various) - Use rawloader crate
- [ ] RLA/RPF - Wavefront
- [ ] SGI - Silicon Graphics
- [ ] TGA - Targa

---

## 4. OCIO Integration

### 4.1 Current Status (vfx-ocio crate)
- [x] `colorconvert()` - Color space conversion
- [x] `colorconvert_into()` - In-place conversion
- [x] `colorconvert_inplace()` - Modify buffer directly
- [x] `ociodisplay()` - Display transform
- [x] `ociolook()` - Look transform
- [x] `ociofiletransform()` - File-based transform (LUT)
- [x] `ocionamedtransform()` - Named transform
- [x] `colormatrixtransform()` - Matrix-based color transform
- [x] `equivalent_colorspace()` - Check if colorspaces are equivalent

### 4.2 Missing OCIO Functions
- [x] `getNumColorSpaces()` - via `num_colorspaces()` / `colorspaces()` iterator
- [x] `getColorSpaceNameByIndex()` - via `colorspace_by_index()`
- [x] `getNumDisplays()` / `getDisplay()` - via `num_displays()` / `display()`
- [x] `getNumViews()` / `getView()` - via `num_views()` / `views()`
- [x] `getNumLooks()` / `getLookNameByIndex()` - via `num_looks()` / `look()`
- [x] Context variables support - `Context` struct with variable resolution
- [x] GPU processor support - `GpuProcessor` with GLSL shader generation

### 4.3 OCIO Config Management
- [x] Load config from file/env
- [x] Default config
- [ ] Config creation/editing API
- [ ] Role management
- [ ] Family/alias support

---

## 5. TextureSystem

### 5.1 Current Status (vfx-io/texture.rs)
- [x] `make_texture()` - Generate mipmap chain
- [x] `make_mip_level()` - Single mip level
- [x] `mip_level_count()` - Calculate mip count
- [x] `mip_dimensions()` - Calculate mip dimensions
- [x] MipmapOptions struct

### 5.2 TextureSystem Features (DONE)
- [x] Texture lookup with filtering (Nearest, Bilinear, Trilinear, Anisotropic)
- [x] Texture tile caching - integrated with ImageCache
- [x] Multi-resolution texture sampling - MIP level support
- [x] Environment map sampling (LatLong, LightProbe, CubeMap)
- [x] Volume/3D texture sampling - `texture3d()`
- [x] Texture statistics - via CacheStats
- [x] Texture handle API - `TextureHandle` for efficient sampling
- [x] Derivatives support - `sample_d()` for proper MIP selection

---

## 6. ImageCache

### 6.1 Current Status (vfx-io/cache.rs)
- [x] Basic tile caching
- [x] LRU eviction
- [x] Memory limit configuration

### 6.2 ImageCache Features (DONE)
- [x] File handle management - via ImageStorage (Full/Streaming)
- [x] Tile-based reading for huge images - streaming threshold support
- [x] Statistics and memory tracking - CacheStats (hits, misses, evictions, peak_size, hit_rate)
- [x] Invalidation/refresh API - `invalidate()`, `clear()`
- [x] Multi-threaded access - RwLock-based thread-safe access
- [x] Prefetching hints - `prefetch()`, `prefetch_mip()`, `prefetch_region()`
- [x] Subimage/miplevel selection - TileKey with subimage and mip_level

---

## 7. Implementation Priority

### Phase 1: Complete ImageBufAlgo (DONE)
1. [x] Add `bluenoise_image()` - Blue noise texture generation
2. [x] Add `copy()` - Copy with type conversion
3. [x] Add `text_size()` - Text bounding box (requires `text` feature)
4. [x] Add `scale()` - Scale image by factor
5. [x] Add `nonzero_region()` - Find ROI of non-zero pixels
6. [x] Add `pixel_hash()` - FNV-1a hash of pixel data
7. [x] Add `compare_yee()` - Perceptual comparison (Yee algorithm)
8. [x] Add `fix_non_finite()` - Replace NaN/Inf with valid values

### Phase 2: Formats (DONE)
1. [x] PSD read (layers, masks) - `psd` feature
2. [x] DDS read (BC1-BC7 decompression) - `dds` feature
3. [x] KTX2 read (uncompressed, f16/f32) - `ktx` feature

### Phase 3: OCIO Enhancement (DONE)
1. [x] Complete config enumeration API - already implemented
2. [x] GPU processor support - `GpuProcessor` with GLSL generation
3. [x] Context variables - `Context` struct with resolve/set methods

### Phase 4: TextureSystem (DONE)
1. [x] Texture lookup with filtering - bilinear, trilinear, anisotropic
2. [x] Environment map support - LatLong, LightProbe, CubeMap
3. [x] Tile-based caching integration - uses ImageCache

### Phase 5: ImageCache (DONE)
1. [x] Full tile management - LRU eviction, streaming support
2. [x] Statistics API - hits, misses, evictions, hit_rate()
3. [x] Prefetching - prefetch(), prefetch_mip(), prefetch_region()

---

## 8. Testing Requirements

For each new function:
- [ ] Unit tests with known inputs/outputs
- [ ] Comparison with OIIO output where applicable
- [ ] Performance benchmarks
- [ ] Documentation with examples

---

## 9. Notes

### Naming Conventions
- OIIO: `camelCase` (isConstantColor, computePixelStats)
- vfx-rs: `snake_case` (is_constant_color, compute_pixel_stats)

### Thread Safety
All functions should be thread-safe. Use `Send + Sync` where needed.

### Error Handling
- Return `IoResult<T>` for fallible operations
- Use meaningful error variants in `IoError`

---

## Completion Tracking

- [x] Phase 1: ImageBufAlgo completion (8/8) - **DONE**
- [x] Phase 2: Format support (3/3) - **DONE**
- [x] Phase 3: OCIO enhancement (3/3) - **DONE**
- [x] Phase 4: TextureSystem (3/3) - **DONE**
- [x] Phase 5: ImageCache (3/3) - **DONE**

**Overall Progress: 100% (5/5 phases complete)**

Last updated: 2026-01-09
