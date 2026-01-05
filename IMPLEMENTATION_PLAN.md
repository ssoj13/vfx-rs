# VFX-RS Implementation Plan

**Target:** Parity with OpenColorIO & OpenImageIO (432 missing features)
**Branch:** dev2

---

## Phase 1: Core Types & ImageSpec (Priority: CRITICAL)

### 1.1 TypeDesc Extensions (vfx-core/src/format.rs)
- [ ] AGGREGATE enum (SCALAR, VEC2, VEC3, VEC4, MATRIX33, MATRIX44)
- [ ] VECSEMANTICS enum (NOXFORM, COLOR, POINT, VECTOR, NORMAL, etc.)
- [ ] Methods: elementtype(), elementsize(), basesize(), numelements()
- [ ] Methods: is_array(), is_floating_point(), is_signed(), size()

### 1.2 ImageSpec Extensions (vfx-core/src/spec.rs)
- [ ] Fields: x, y, z (origin coordinates)
- [ ] Fields: full_x, full_y, full_z (display window origin)
- [ ] Fields: full_width, full_height, full_depth
- [ ] Fields: tile_width, tile_height, tile_depth
- [ ] Fields: channelformats (per-channel formats)
- [ ] Fields: alpha_channel, z_channel indices
- [ ] Field: deep (bool)
- [ ] Methods: default_channel_names(), channel_bytes(), scanline_bytes()
- [ ] Methods: tile_pixels(), tile_bytes(), image_bytes(), image_pixels()
- [ ] Methods: auto_stride(), size_t_safe()
- [ ] Methods: erase_attribute(), getattributetype()
- [ ] Methods: get_int_attribute(), get_float_attribute(), get_string_attribute()
- [ ] Methods: serialize(), to_xml(), from_xml()
- [ ] Methods: valid_tile_range(), copy_dimensions(), set_colorspace()

### 1.3 ROI Extensions (vfx-core/src/rect.rs)
- [ ] Constructor with full parameters (xbegin, xend, ybegin, yend, zbegin, zend, chbegin, chend)
- [ ] npixels() method
- [ ] All() static method
- [ ] contains(x, y, z, ch) and contains(ROI) methods
- [ ] roi_union(), roi_intersection()

---

## Phase 2: ImageBuf Class (vfx-io/src/imagebuf/)

### 2.1 Core ImageBuf Structure
- [ ] IBStorage enum (UNINITIALIZED, LOCALBUFFER, APPBUFFER, IMAGECACHE)
- [ ] WrapMode enum (WrapDefault, WrapBlack, WrapClamp, WrapPeriodic, WrapMirror)
- [ ] Multiple constructor variants
- [ ] reset() methods (multiple overloads)
- [ ] make_writable()
- [ ] read() with subimage/miplevel support
- [ ] init_spec()
- [ ] write() with TypeDesc and options
- [ ] set_write_format(), set_write_tiles()

### 2.2 Metadata & Copying
- [ ] copy_metadata()
- [ ] copy_pixels()
- [ ] copy() returning new ImageBuf
- [ ] swap()

### 2.3 Pixel Access
- [ ] getchannel(x, y, z, c)
- [ ] getpixel(x, y, z, float*, n)
- [ ] interppixel(x, y, float*)
- [ ] interppixel_NDC(s, t, float*)
- [ ] interppixel_bicubic()
- [ ] setpixel(x, y, z, const float*)
- [ ] get_pixels(ROI, TypeDesc, void*, ...)
- [ ] set_pixels(ROI, TypeDesc, const void*, ...)

### 2.4 Buffer Properties
- [ ] storage()
- [ ] initialized()
- [ ] cachedpixels()
- [ ] imagecache()
- [ ] localpixels()
- [ ] pixel_stride(), scanline_stride(), z_stride()
- [ ] contiguous()

---

## Phase 3: DeepData Class (CRITICAL for compositing)

### 3.1 Core Structure (vfx-io/src/deep/)
- [ ] Constructor: DeepData(), DeepData(ImageSpec&)
- [ ] init(npix, nchan, channeltypes, ...)
- [ ] initialized(), allocated()
- [ ] pixels(), channels()
- [ ] Z_channel(), Zback_channel(), A_channel()
- [ ] AR_channel(), AG_channel(), AB_channel()
- [ ] channelname(c), channeltype(c), channelsize(c)
- [ ] samplesize()

### 3.2 Sample Management
- [ ] samples(pixel)
- [ ] set_samples(pixel, samps)
- [ ] set_all_samples()
- [ ] set_capacity(), capacity()
- [ ] insert_samples(), erase_samples()

### 3.3 Deep Values
- [ ] deep_value(pixel, chan, samp)
- [ ] deep_value_uint()
- [ ] set_deep_value()
- [ ] data_ptr()
- [ ] all_channeltypes(), all_samples(), all_data()
- [ ] get_pointers()

### 3.4 Deep Operations
- [ ] copy_deep_sample()
- [ ] copy_deep_pixel()
- [ ] split()
- [ ] sort()
- [ ] merge_overlaps()
- [ ] merge_deep_pixels()
- [ ] occlusion_cull()
- [ ] opaque_z()

---

## Phase 4: ImageBufAlgo - Core Operations (vfx-ops/)

### 4.1 Pattern Generation (vfx-ops/src/generate.rs)
- [ ] zero(ROI)
- [ ] fill(values, ROI)
- [ ] fill(top_values, bottom_values, ROI)
- [ ] fill(corners, ROI)
- [ ] checker()
- [ ] noise()
- [ ] bluenoise_image()

### 4.2 Channel Operations (vfx-ops/src/channels.rs)
- [ ] channels() - reorder/extract channels
- [ ] channel_append()
- [ ] flatten() - flatten deep to flat
- [ ] deepen() - convert flat to deep
- [ ] deep_merge()
- [ ] deep_holdout()

### 4.3 Cropping & Assembly (vfx-ops/src/crop.rs)
- [ ] crop(ROI)
- [ ] cut(ROI)
- [ ] paste()
- [ ] rotate90(), rotate180(), rotate270()
- [ ] flip(), flop()
- [ ] transpose()
- [ ] reorient()
- [ ] circular_shift()

### 4.4 Arithmetic Operations (vfx-ops/src/arithmetic.rs)
- [ ] add(), sub(), absdiff()
- [ ] abs()
- [ ] mul(), div()
- [ ] mad()
- [ ] invert()
- [ ] pow()
- [ ] channel_sum()
- [ ] max(), min()
- [ ] clamp()
- [ ] maxchan(), minchan()
- [ ] contrast_remap()
- [ ] saturate()

### 4.5 Geometric Transforms (vfx-ops/src/transform.rs - extend)
- [ ] rotate(angle, ...)
- [ ] resize(filter, ...)
- [ ] resample()
- [ ] fit()
- [ ] warp(M33f, ...)
- [ ] st_warp()

---

## Phase 5: ImageBufAlgo - Color & Compositing

### 5.1 Color Transforms (vfx-ops/src/color.rs)
- [ ] colorconvert(from, to, ...)
- [ ] colormatrixtransform(M44f)
- [ ] ociolook()
- [ ] ociodisplay()
- [ ] ociofiletransform()
- [ ] unpremult()
- [ ] premult()
- [ ] repremult()

### 5.2 Compositing (vfx-ops/src/composite.rs - extend)
- [ ] over() - Porter-Duff
- [ ] zover()

### 5.3 Convolution & Filters (vfx-ops/src/filter.rs - extend)
- [ ] convolve()
- [ ] laplacian()
- [ ] make_kernel()
- [ ] median_filter()
- [ ] unsharp_mask()
- [ ] dilate(), erode()

### 5.4 FFT (vfx-ops/src/fft.rs - extend)
- [ ] fft(), ifft()
- [ ] polar_to_complex()
- [ ] complex_to_polar()

---

## Phase 6: Statistics & Analysis (vfx-ops/src/stats.rs)

- [ ] computePixelStats() -> PixelStats
- [ ] compare(A, B, failthresh, warnthresh) -> CompareResults
- [ ] compare_Yee()
- [ ] isConstantColor()
- [ ] isConstantChannel()
- [ ] isMonochrome()
- [ ] color_count()
- [ ] color_range_check()
- [ ] histogram()
- [ ] computePixelHashSHA1()
- [ ] nonzero_region()

---

## Phase 7: ImageCache & TextureSystem (vfx-io/src/)

### 7.1 ImageCache (vfx-io/src/cache.rs - extend)
- [ ] create(shared)
- [ ] destroy()
- [ ] attribute(), getattribute()
- [ ] resolve_filename()
- [ ] get_image_info()
- [ ] get_imagespec(), imagespec()
- [ ] get_thumbnail()
- [ ] get_pixels()
- [ ] invalidate(), invalidate_all()
- [ ] close(), close_all()
- [ ] getstats(), reset_stats()
- [ ] Perthread, Tile structs

### 7.2 TextureSystem (vfx-io/src/texture.rs - extend)
- [ ] create(shared, imagecache)
- [ ] destroy()
- [ ] attribute(), getattribute()
- [ ] texture()
- [ ] texture3d()
- [ ] shadow()
- [ ] environment()
- [ ] TextureOpt struct
- [ ] resolve_filename()
- [ ] get_texture_info()
- [ ] get_imagespec(), imagespec()
- [ ] inventory_udim()
- [ ] is_udim(), resolve_udim()

---

## Phase 8: ColorConfig (OIIO color management)

### 8.1 ColorConfig Class (vfx-io/src/colorconfig.rs)
- [ ] Constructor with filename
- [ ] reset()
- [ ] error(), geterror()
- [ ] getNumColorSpaces(), getColorSpaceNameByIndex()
- [ ] getColorSpaceFamilyByName()
- [ ] getNumRoles(), getRoleByIndex()
- [ ] getColorSpaceFromFilepath()
- [ ] parseColorSpaceFromString()
- [ ] getNumLooks(), getLookNameByIndex()
- [ ] getNumDisplays(), getDisplayNameByIndex()
- [ ] getDefaultDisplayName()
- [ ] getNumViews(), getViewNameByIndex()
- [ ] getDefaultViewName()
- [ ] createColorProcessor()
- [ ] createLookTransform()
- [ ] createDisplayTransform()
- [ ] createFileTransform()
- [ ] createMatrixTransform()
- [ ] getColorSpaceDataType()
- [ ] equivalent()

---

## Phase 9: OpenColorIO Extensions (vfx-ocio/)

### 9.1 Config Extensions (vfx-ocio/src/config.rs)
- [ ] CreateRaw()
- [ ] getMajorVersion(), getMinorVersion()
- [ ] setMajorVersion(), setMinorVersion()
- [ ] upgradeToLatestVersion()
- [ ] validate()
- [ ] getName(), setName()
- [ ] getDescription(), setDescription()
- [ ] serialize()
- [ ] getFamilySeparator(), setFamilySeparator()
- [ ] getCacheID()

### 9.2 Environment & Context (vfx-ocio/src/context.rs - extend)
- [ ] addEnvironmentVar()
- [ ] getNumEnvironmentVars()
- [ ] getEnvironmentVarNameByIndex()
- [ ] getEnvironmentVarDefault()
- [ ] clearEnvironmentVars()
- [ ] setEnvironmentMode(), getEnvironmentMode()
- [ ] loadEnvironment()
- [ ] getWorkingDir(), setWorkingDir()

### 9.3 Search Paths
- [ ] getSearchPath() variants
- [ ] setSearchPath()
- [ ] getNumSearchPaths()
- [ ] clearSearchPaths()
- [ ] addSearchPath()

### 9.4 Color Spaces Advanced
- [ ] getColorSpaces(category) -> ColorSpaceSet
- [ ] getNumColorSpaces(SearchReferenceSpaceType)
- [ ] ColorSpaceVisibility enum
- [ ] getIndexForColorSpace()
- [ ] getCanonicalName()
- [ ] removeColorSpace()
- [ ] isColorSpaceUsed()
- [ ] clearColorSpaces()
- [ ] setInactiveColorSpaces(), getInactiveColorSpaces()
- [ ] isColorSpaceLinear()
- [ ] IdentifyBuiltinColorSpace()
- [ ] IdentifyInterchangeSpace()

### 9.5 Roles
- [ ] setRole()
- [ ] getNumRoles()
- [ ] hasRole()
- [ ] getRoleName()
- [ ] getRoleColorSpace()

### 9.6 Displays & Views Advanced
- [ ] addSharedView(), removeSharedView(), clearSharedViews()
- [ ] isViewShared()
- [ ] View transform support
- [ ] getDisplayViewTransformName()
- [ ] getDisplayViewColorSpaceName()
- [ ] getDisplayViewLooks()
- [ ] getDisplayViewRule()
- [ ] getDisplayViewDescription()
- [ ] hasView()
- [ ] getDefaultView(display, colorspaceName)
- [ ] getNumViews(display, colorspaceName)
- [ ] ViewingRules support

### 9.7 Virtual Display (ICC)
- [ ] addVirtualDisplayView()
- [ ] getVirtualDisplayNumViews()
- [ ] getVirtualDisplayView()
- [ ] removeVirtualDisplayView()
- [ ] clearVirtualDisplay()
- [ ] instantiateDisplayFromMonitorName()
- [ ] instantiateDisplayFromICCProfile()
- [ ] isDisplayTemporary(), setDisplayTemporary()

### 9.8 Active Displays/Views
- [ ] setActiveDisplays(), getActiveDisplays()
- [ ] getNumActiveDisplays(), getActiveDisplay()
- [ ] addActiveDisplay(), removeActiveDisplay()
- [ ] clearActiveDisplays()
- [ ] setActiveViews(), getActiveViews()
- [ ] getNumActiveViews(), getActiveView()

### 9.9 Looks
- [ ] getLook()
- [ ] getNumLooks()
- [ ] getLookNameByIndex()
- [ ] addLook()
- [ ] clearLooks()

### 9.10 View Transforms (OCIO v2)
- [ ] getNumViewTransforms()
- [ ] getViewTransform()
- [ ] getViewTransformNameByIndex()
- [ ] addViewTransform()
- [ ] clearViewTransforms()
- [ ] getDefaultSceneToDisplayViewTransform()
- [ ] getDefaultViewTransformName(), setDefaultViewTransformName()

### 9.11 Named Transforms (OCIO v2)
- [ ] getNumNamedTransforms(visibility)
- [ ] getNamedTransform()
- [ ] getNamedTransformNameByIndex()
- [ ] getIndexForNamedTransform()
- [ ] addNamedTransform()
- [ ] removeNamedTransform()
- [ ] clearNamedTransforms()

### 9.12 File Rules
- [ ] getFileRules(), setFileRules()
- [ ] getColorSpaceFromFilepath()
- [ ] filepathOnlyMatchesDefaultRule()
- [ ] isStrictParsingEnabled(), setStrictParsingEnabled()

### 9.13 Processors Advanced
- [ ] getProcessor(context, src, dst)
- [ ] getProcessor(src, display, view, dir)
- [ ] getProcessor(namedTransform, dir)
- [ ] getProcessor(transform)
- [ ] GetProcessorToBuiltinColorSpace()
- [ ] GetProcessorFromBuiltinColorSpace()
- [ ] GetProcessorFromConfigs()
- [ ] getProcessorCacheFlags(), setProcessorCacheFlags()
- [ ] setConfigIOProxy(), getConfigIOProxy()

### 9.14 Archiving
- [ ] isArchivable()
- [ ] archive()
- [ ] ExtractOCIOZArchive()

### 9.15 Transform Types (vfx-ocio/src/transform/)
- [ ] Lut1DTransform
- [ ] Lut3DTransform
- [ ] LogAffineTransform
- [ ] LogCameraTransform
- [ ] ExponentWithLinearTransform
- [ ] GradingHueCurveTransform

### 9.16 Dynamic Properties
- [ ] DynamicProperty base
- [ ] DynamicPropertyDouble
- [ ] DynamicPropertyGradingPrimary
- [ ] DynamicPropertyGradingRGBCurve
- [ ] DynamicPropertyGradingHueCurve
- [ ] DynamicPropertyGradingTone

### 9.17 Global Functions
- [ ] GetCurrentConfig(), SetCurrentConfig()
- [ ] Config::CreateFromBuiltinConfig()
- [ ] getDefaultLumaCoefs(), setDefaultLumaCoefs()

---

## Execution Order

### Week 1-2: Foundation
1. Phase 1 (TypeDesc, ImageSpec, ROI) - foundational types
2. Phase 2.1-2.2 (ImageBuf core) - basic buffer

### Week 3-4: Deep & Algorithms
3. Phase 3 (DeepData) - critical for VFX
4. Phase 4.1-4.3 (Pattern, Channels, Crop)

### Week 5-6: Operations
5. Phase 4.4-4.5 (Arithmetic, Geometric)
6. Phase 5 (Color, Compositing, Filters)

### Week 7-8: Caching & Texture
7. Phase 6 (Statistics)
8. Phase 7 (ImageCache, TextureSystem)

### Week 9-10: OCIO
9. Phase 8 (ColorConfig)
10. Phase 9.1-9.8 (OCIO Config extensions)

### Week 11-12: OCIO Advanced
11. Phase 9.9-9.14 (Looks, Transforms, Archiving)
12. Phase 9.15-9.17 (New transforms, Dynamic properties)

---

## Notes

- Reference implementations in `_ref/OpenImageIO/` and `_ref/OpenColorIO/`
- Run tests after each phase: `cargo test -p <crate>`
- Build check: `cargo build --all-features`
- Commits should be granular (per feature group)

---

*Generated: 2026-01-05*
