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
- [ ] `bluenoise_image()` - Return cached blue noise texture for dithering

### 1.2 Utility Functions
- [ ] `copy()` - Copy with optional type conversion
- [ ] `text_size()` - Get text bounding box without rendering
- [ ] `scale()` - Scale image by factor (different from mul)

### 1.3 Statistics & Analysis
- [ ] `nonzero_region()` - Find ROI of non-zero pixels
- [ ] `computePixelHashSHA1()` - Compute SHA1 hash of pixel data
- [ ] `compare_Yee()` - Perceptual image comparison (Yee algorithm)

### 1.4 Repair & Cleanup
- [ ] `fixNonFinite()` - Replace NaN/Inf with valid values

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

### 3.2 Missing Formats (PRIORITY: MEDIUM)
- [ ] **PSD** - Photoshop format (layers, masks, blending)
- [ ] **DDS** - DirectDraw Surface (GPU textures, mipmaps, BC compression)
- [ ] **KTX/KTX2** - Khronos Texture (Vulkan/OpenGL, ASTC/ETC2/BC)

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
- [ ] `getNumColorSpaces()` - Wrapped but check API
- [ ] `getColorSpaceNameByIndex()` - Iteration support
- [ ] `getNumDisplays()` / `getDisplay()` - Display enumeration
- [ ] `getNumViews()` / `getView()` - View enumeration
- [ ] `getNumLooks()` / `getLookNameByIndex()` - Look enumeration
- [ ] Context variables support
- [ ] GPU processor support (for real-time preview)

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

### 5.2 Missing TextureSystem Features
- [ ] Texture lookup with filtering (trilinear, anisotropic)
- [ ] Texture tile caching
- [ ] Multi-resolution texture sampling
- [ ] Environment map sampling (latlong, cube)
- [ ] Shadow map sampling
- [ ] Texture statistics
- [ ] Texture handle API
- [ ] Batch texture operations

---

## 6. ImageCache

### 6.1 Current Status (vfx-io/cache.rs)
- [x] Basic tile caching
- [x] LRU eviction
- [x] Memory limit configuration

### 6.2 Missing ImageCache Features
- [ ] File handle management
- [ ] Tile-based reading for huge images
- [ ] Statistics and memory tracking
- [ ] Invalidation/refresh API
- [ ] Multi-threaded access optimization
- [ ] Prefetching hints
- [ ] Subimage/miplevel selection

---

## 7. Implementation Priority

### Phase 1: Complete ImageBufAlgo (CURRENT)
1. [ ] Add `bluenoise_image()`
2. [ ] Add `copy()` 
3. [ ] Add `text_size()`
4. [ ] Add `scale()`
5. [ ] Add `nonzero_region()`
6. [ ] Add `computePixelHashSHA1()` (or `pixel_hash_sha256()` for modern hash)
7. [ ] Add `compare_Yee()` (perceptual comparison)
8. [ ] Add `fixNonFinite()` (as `fix_non_finite()`)

### Phase 2: Formats
1. [ ] PSD read/write (layers, masks, blend modes)
2. [ ] DDS read/write (DXT1/3/5, BC1-7)
3. [ ] KTX2 read/write (ASTC, ETC2, BC)

### Phase 3: OCIO Enhancement
1. [ ] Complete config enumeration API
2. [ ] GPU processor support
3. [ ] Context variables

### Phase 4: TextureSystem
1. [ ] Texture lookup with filtering
2. [ ] Environment map support
3. [ ] Tile-based caching integration

### Phase 5: ImageCache
1. [ ] Full tile management
2. [ ] Statistics API
3. [ ] Prefetching

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

- [ ] Phase 1: ImageBufAlgo completion (0/8)
- [ ] Phase 2: Format support (0/3)
- [ ] Phase 3: OCIO enhancement (0/3)
- [ ] Phase 4: TextureSystem (0/3)
- [ ] Phase 5: ImageCache (0/3)

**Overall Progress: 0%**

Last updated: 2026-01-09
