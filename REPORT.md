# VFX-RS: Exhaustive Missing Features

**Date:** 2026-01-05  
**Scope:** Complete parity analysis vs OpenColorIO & OpenImageIO

---

# PART 2: OpenImageIO Missing Features (200+ items)

## 1. ImageSpec Fields

| # | Field | OIIO Type | File | Status | Priority |
|---|-------|-----------|------|--------|----------|
| 1 | x, y, z | int | imageio.h | MISSING | HIGH |
| 2 | full_x, full_y, full_z | int | imageio.h | MISSING | HIGH |
| 3 | full_width, full_height, full_depth | int | imageio.h | MISSING | HIGH |
| 4 | tile_width, tile_height, tile_depth | int | imageio.h | MISSING | HIGH |
| 5 | nchannels | int | imageio.h | EXISTS | - |
| 6 | format | TypeDesc | imageio.h | PARTIAL | HIGH |
| 7 | channelformats | vector<TypeDesc> | imageio.h | MISSING | HIGH |
| 8 | channelnames | vector<string> | imageio.h | PARTIAL | MEDIUM |
| 9 | alpha_channel | int | imageio.h | MISSING | HIGH |
| 10 | z_channel | int | imageio.h | MISSING | MEDIUM |
| 11 | deep | bool | imageio.h | MISSING | CRITICAL |
| 12 | extra_attribs | ParamValueList | imageio.h | PARTIAL | HIGH |

### ImageSpec Methods

| # | Method | Signature | Status | Priority |
|---|--------|-----------|--------|----------|
| 13 | default_channel_names | `void default_channel_names()` | MISSING | MEDIUM |
| 14 | channel_bytes | `size_t channel_bytes(int chan, bool native)` | MISSING | HIGH |
| 15 | pixel_bytes | `size_t pixel_bytes(bool native)` | PARTIAL | HIGH |
| 16 | scanline_bytes | `size_t scanline_bytes(bool native)` | MISSING | HIGH |
| 17 | tile_pixels | `imagesize_t tile_pixels()` | MISSING | HIGH |
| 18 | tile_bytes | `size_t tile_bytes(bool native)` | MISSING | HIGH |
| 19 | image_bytes | `imagesize_t image_bytes(bool native)` | MISSING | HIGH |
| 20 | image_pixels | `imagesize_t image_pixels()` | MISSING | HIGH |
| 21 | size_t_safe | `bool size_t_safe()` | MISSING | MEDIUM |
| 22 | auto_stride | `static void auto_stride(...)` | MISSING | HIGH |
| 23 | attribute | `void attribute(string_view, TypeDesc, void*)` | PARTIAL | HIGH |
| 24 | erase_attribute | `void erase_attribute(string_view, TypeDesc)` | MISSING | MEDIUM |
| 25 | getattribute | `ParamValue* find_attribute(...)` | PARTIAL | HIGH |
| 26 | getattributetype | `TypeDesc getattributetype(string_view)` | MISSING | MEDIUM |
| 27 | get_int_attribute | `int get_int_attribute(string_view, int def)` | MISSING | HIGH |
| 28 | get_float_attribute | `float get_float_attribute(...)` | MISSING | HIGH |
| 29 | get_string_attribute | `string_view get_string_attribute(...)` | MISSING | HIGH |
| 30 | metadata_val | `std::string metadata_val(ParamValue&, bool)` | MISSING | LOW |
| 31 | serialize | `std::string serialize(SerialFormat, SerialVerbose)` | MISSING | MEDIUM |
| 32 | to_xml | `std::string to_xml()` | MISSING | LOW |
| 33 | from_xml | `void from_xml(const char*)` | MISSING | LOW |
| 34 | valid_tile_range | `bool valid_tile_range(...)` | MISSING | HIGH |
| 35 | copy_dimensions | `void copy_dimensions(const ImageSpec&)` | MISSING | MEDIUM |
| 36 | set_format | `void set_format(TypeDesc)` | PARTIAL | HIGH |
| 37 | set_colorspace | `void set_colorspace(string_view)` | MISSING | HIGH |
| 38 | undefined | `static bool undefined(const ImageSpec&)` | MISSING | MEDIUM |

## 2. ImageBuf Class

| # | Method | Signature | Status | Priority |
|---|--------|-----------|--------|----------|
| 39 | IBStorage enum | UNINITIALIZED, LOCALBUFFER, APPBUFFER, IMAGECACHE | MISSING | HIGH |
| 40 | Constructor variants | 8 different constructors | PARTIAL | HIGH |
| 41 | reset | `void reset(...)` (multiple overloads) | MISSING | HIGH |
| 42 | make_writable | `bool make_writable(bool keep_cache_type)` | MISSING | HIGH |
| 43 | read | `bool read(int subimage, int miplevel, ...)` | PARTIAL | HIGH |
| 44 | init_spec | `bool init_spec(string_view, int, int)` | MISSING | HIGH |
| 45 | write | `bool write(string_view, TypeDesc, ...)` | PARTIAL | HIGH |
| 46 | set_write_format | `void set_write_format(TypeDesc)` | MISSING | MEDIUM |
| 47 | set_write_tiles | `void set_write_tiles(int, int, int)` | MISSING | HIGH |
| 48 | copy_metadata | `void copy_metadata(const ImageBuf&)` | MISSING | MEDIUM |
| 49 | copy_pixels | `bool copy_pixels(const ImageBuf&)` | MISSING | HIGH |
| 50 | copy | `ImageBuf copy(TypeDesc)` | MISSING | HIGH |
| 51 | swap | `void swap(ImageBuf&)` | MISSING | LOW |
| 52 | getchannel | `float getchannel(int x, int y, int z, int c)` | MISSING | HIGH |
| 53 | getpixel | `void getpixel(int x, int y, int z, float*, int)` | MISSING | HIGH |
| 54 | interppixel | `void interppixel(float x, float y, float*)` | MISSING | HIGH |
| 55 | interppixel_NDC | `void interppixel_NDC(float, float, float*)` | MISSING | HIGH |
| 56 | interppixel_bicubic | `void interppixel_bicubic(...)` | MISSING | MEDIUM |
| 57 | setpixel | `void setpixel(int x, int y, int z, const float*)` | MISSING | HIGH |
| 58 | get_pixels | `bool get_pixels(ROI, TypeDesc, void*, ...)` | PARTIAL | HIGH |
| 59 | set_pixels | `bool set_pixels(ROI, TypeDesc, const void*, ...)` | PARTIAL | HIGH |
| 60 | storage | `IBStorage storage()` | MISSING | MEDIUM |
| 61 | initialized | `bool initialized()` | PARTIAL | - |
| 62 | cachedpixels | `bool cachedpixels()` | MISSING | MEDIUM |
| 63 | imagecache | `ImageCache* imagecache()` | MISSING | HIGH |
| 64 | localpixels | `void* localpixels()` | PARTIAL | HIGH |
| 65 | pixel_stride | `stride_t pixel_stride()` | MISSING | HIGH |
| 66 | scanline_stride | `stride_t scanline_stride()` | MISSING | HIGH |
| 67 | z_stride | `stride_t z_stride()` | MISSING | MEDIUM |
| 68 | contiguous | `bool contiguous()` | MISSING | MEDIUM |
| 69 | deep | `bool deep()` | MISSING | CRITICAL |
| 70 | deep_samples | `int deep_samples(int x, int y, int z)` | MISSING | CRITICAL |
| 71 | deepdata | `DeepData* deepdata()` | MISSING | CRITICAL |
| 72 | set_deep_samples | `void set_deep_samples(...)` | MISSING | CRITICAL |
| 73 | deep_value | `float deep_value(int x, int y, int z, int c, int s)` | MISSING | CRITICAL |
| 74 | set_deep_value | `void set_deep_value(...)` | MISSING | CRITICAL |
| 75 | WrapMode enum | WrapDefault, WrapBlack, WrapClamp, WrapPeriodic, WrapMirror | MISSING | HIGH |

## 3. ImageBufAlgo - Pattern Generation

| # | Function | Signature | Status | Priority |
|---|----------|-----------|--------|----------|
| 76 | zero | `ImageBuf zero(ROI, int nthreads)` | MISSING | HIGH |
| 77 | fill | `ImageBuf fill(cspan<float>, ROI, int)` | MISSING | HIGH |
| 78 | fill (2-color) | `ImageBuf fill(cspan<float>, cspan<float>, ROI)` | MISSING | MEDIUM |
| 79 | fill (4-corner) | `ImageBuf fill(top, bottom, left, right, ROI)` | MISSING | MEDIUM |
| 80 | checker | `ImageBuf checker(int, int, int, cspan, cspan, ...)` | MISSING | MEDIUM |
| 81 | noise | `ImageBuf noise(string_view type, float A, float B, ...)` | MISSING | MEDIUM |
| 82 | bluenoise_image | `ImageBuf bluenoise_image()` | MISSING | LOW |
| 83 | point | `ImageBuf point(const ImageBuf&, int x, int y, cspan)` | MISSING | LOW |
| 84 | lines | `ImageBuf lines(const ImageBuf&, cspan<int>, cspan)` | MISSING | LOW |
| 85 | box | `ImageBuf box(const ImageBuf&, int, int, int, int, cspan)` | MISSING | MEDIUM |
| 86 | text | `ImageBuf text(const ImageBuf&, int, int, string_view, ...)` | MISSING | MEDIUM |

## 4. ImageBufAlgo - Channel Operations

| # | Function | Signature | Status | Priority |
|---|----------|-----------|--------|----------|
| 87 | channels | `ImageBuf channels(src, int nchans, cspan<int>, ...)` | MISSING | CRITICAL |
| 88 | channel_append | `ImageBuf channel_append(A, B, ROI, int)` | MISSING | HIGH |
| 89 | flatten | `ImageBuf flatten(src, ROI, int)` | MISSING | HIGH |
| 90 | deepen | `ImageBuf deepen(src, float zvalue, ROI, int)` | MISSING | HIGH |
| 91 | deep_merge | `ImageBuf deep_merge(A, B, bool, ROI, int)` | MISSING | CRITICAL |
| 92 | deep_holdout | `ImageBuf deep_holdout(src, holdout, ROI, int)` | MISSING | HIGH |

## 5. ImageBufAlgo - Cropping & Assembly

| # | Function | Signature | Status | Priority |
|---|----------|-----------|--------|----------|
| 93 | copy | `ImageBuf copy(src, TypeDesc, ROI, int)` | MISSING | HIGH |
| 94 | crop | `ImageBuf crop(src, ROI, int)` | MISSING | CRITICAL |
| 95 | cut | `ImageBuf cut(src, ROI, int)` | MISSING | HIGH |
| 96 | paste | `bool paste(dst, int, int, int, int, src, ROI, int)` | MISSING | CRITICAL |
| 97 | rotate90 | `ImageBuf rotate90(src, ROI, int)` | MISSING | HIGH |
| 98 | rotate180 | `ImageBuf rotate180(src, ROI, int)` | MISSING | HIGH |
| 99 | rotate270 | `ImageBuf rotate270(src, ROI, int)` | MISSING | HIGH |
| 100 | flip | `ImageBuf flip(src, ROI, int)` | MISSING | HIGH |
| 101 | flop | `ImageBuf flop(src, ROI, int)` | MISSING | HIGH |
| 102 | transpose | `ImageBuf transpose(src, ROI, int)` | MISSING | HIGH |
| 103 | reorient | `ImageBuf reorient(src, int)` | MISSING | MEDIUM |
| 104 | circular_shift | `ImageBuf circular_shift(src, int, int, int, ROI, int)` | MISSING | MEDIUM |

## 6. ImageBufAlgo - Geometric Transforms

| # | Function | Signature | Status | Priority |
|---|----------|-----------|--------|----------|
| 105 | rotate | `ImageBuf rotate(src, float angle, ...)` | MISSING | CRITICAL |
| 106 | resize | `ImageBuf resize(src, string_view filter, float, ROI, int)` | MISSING | CRITICAL |
| 107 | resample | `ImageBuf resample(src, bool interpolate, ROI, int)` | MISSING | HIGH |
| 108 | fit | `ImageBuf fit(src, string_view, float, string_view, ROI, int)` | MISSING | HIGH |
| 109 | warp | `ImageBuf warp(src, M33f, string_view, float, ...)` | MISSING | HIGH |
| 110 | st_warp | `ImageBuf st_warp(src, stbuf, string_view, int, ...)` | MISSING | MEDIUM |

## 7. ImageBufAlgo - Arithmetic Operations

| # | Function | Signature | Status | Priority |
|---|----------|-----------|--------|----------|
| 111 | add | `ImageBuf add(A, B, ROI, int)` | MISSING | CRITICAL |
| 112 | add (scalar) | `ImageBuf add(A, cspan<float>, ROI, int)` | MISSING | HIGH |
| 113 | sub | `ImageBuf sub(A, B, ROI, int)` | MISSING | CRITICAL |
| 114 | absdiff | `ImageBuf absdiff(A, B, ROI, int)` | MISSING | HIGH |
| 115 | abs | `ImageBuf abs(src, ROI, int)` | MISSING | HIGH |
| 116 | mul | `ImageBuf mul(A, B, ROI, int)` | MISSING | CRITICAL |
| 117 | mul (scalar) | `ImageBuf mul(A, cspan<float>, ROI, int)` | MISSING | HIGH |
| 118 | div | `ImageBuf div(A, B, ROI, int)` | MISSING | HIGH |
| 119 | mad | `ImageBuf mad(A, B, C, ROI, int)` | MISSING | MEDIUM |
| 120 | invert | `ImageBuf invert(src, ROI, int)` | MISSING | HIGH |
| 121 | pow | `ImageBuf pow(src, cspan<float>, ROI, int)` | MISSING | HIGH |
| 122 | channel_sum | `ImageBuf channel_sum(src, cspan<float>, ROI, int)` | MISSING | MEDIUM |
| 123 | max | `ImageBuf max(A, B, ROI, int)` | MISSING | HIGH |
| 124 | min | `ImageBuf min(A, B, ROI, int)` | MISSING | HIGH |
| 125 | clamp | `ImageBuf clamp(src, cspan min, cspan max, ...)` | MISSING | HIGH |
| 126 | maxchan | `ImageBuf maxchan(src, ROI, int)` | MISSING | MEDIUM |
| 127 | minchan | `ImageBuf minchan(src, ROI, int)` | MISSING | MEDIUM |
| 128 | contrast_remap | `ImageBuf contrast_remap(src, black, white, ...)` | MISSING | HIGH |
| 129 | saturate | `ImageBuf saturate(src, float scale, int, ROI, int)` | MISSING | HIGH |

## 8. ImageBufAlgo - Color Transforms

| # | Function | Signature | Status | Priority |
|---|----------|-----------|--------|----------|
| 130 | colorconvert | `ImageBuf colorconvert(src, from, to, ...)` | MISSING | CRITICAL |
| 131 | colormatrixtransform | `ImageBuf colormatrixtransform(src, M44f, ...)` | MISSING | HIGH |
| 132 | ociolook | `ImageBuf ociolook(src, looks, from, to, ...)` | MISSING | HIGH |
| 133 | ociodisplay | `ImageBuf ociodisplay(src, display, view, ...)` | MISSING | HIGH |
| 134 | ociofiletransform | `ImageBuf ociofiletransform(src, name, ...)` | MISSING | HIGH |
| 135 | unpremult | `ImageBuf unpremult(src, ROI, int)` | MISSING | CRITICAL |
| 136 | premult | `ImageBuf premult(src, ROI, int)` | MISSING | CRITICAL |
| 137 | repremult | `ImageBuf repremult(src, ROI, int)` | MISSING | HIGH |

## 9. ImageBufAlgo - Compositing & Blend

| # | Function | Signature | Status | Priority |
|---|----------|-----------|--------|----------|
| 138 | over | `ImageBuf over(A, B, ROI, int)` | MISSING | CRITICAL |
| 139 | zover | `ImageBuf zover(A, B, bool, ROI, int)` | MISSING | HIGH |

## 10. ImageBufAlgo - Convolution & Filters

| # | Function | Signature | Status | Priority |
|---|----------|-----------|--------|----------|
| 140 | convolve | `ImageBuf convolve(src, kernel, bool, ROI, int)` | MISSING | HIGH |
| 141 | laplacian | `ImageBuf laplacian(src, ROI, int)` | MISSING | MEDIUM |
| 142 | fft | `ImageBuf fft(src, ROI, int)` | MISSING | MEDIUM |
| 143 | ifft | `ImageBuf ifft(src, ROI, int)` | MISSING | MEDIUM |
| 144 | polar_to_complex | `ImageBuf polar_to_complex(src, ROI, int)` | MISSING | LOW |
| 145 | complex_to_polar | `ImageBuf complex_to_polar(src, ROI, int)` | MISSING | LOW |
| 146 | make_kernel | `ImageBuf make_kernel(name, w, h, d, bool)` | MISSING | HIGH |
| 147 | median_filter | `ImageBuf median_filter(src, w, h, ROI, int)` | MISSING | HIGH |
| 148 | unsharp_mask | `ImageBuf unsharp_mask(src, kernel, w, contrast, ...)` | MISSING | HIGH |
| 149 | dilate | `ImageBuf dilate(src, w, h, ROI, int)` | MISSING | HIGH |
| 150 | erode | `ImageBuf erode(src, w, h, ROI, int)` | MISSING | HIGH |

## 11. ImageBufAlgo - Statistics & Comparison

| # | Function | Signature | Status | Priority |
|---|----------|-----------|--------|----------|
| 151 | computePixelStats | `PixelStats computePixelStats(src, ROI, int)` | MISSING | HIGH |
| 152 | compare | `CompareResults compare(A, B, float, float, ROI, int)` | MISSING | HIGH |
| 153 | compare_Yee | `int compare_Yee(A, B, result, ...)` | MISSING | MEDIUM |
| 154 | isConstantColor | `bool isConstantColor(src, float, cspan, ROI, int)` | MISSING | MEDIUM |
| 155 | isConstantChannel | `bool isConstantChannel(src, int, float, float, ROI, int)` | MISSING | MEDIUM |
| 156 | isMonochrome | `bool isMonochrome(src, float, ROI, int)` | MISSING | MEDIUM |
| 157 | color_count | `bool color_count(src, imagesize_t*, int, ...)` | MISSING | LOW |
| 158 | color_range_check | `bool color_range_check(src, low, high, ...)` | MISSING | LOW |
| 159 | histogram | `std::vector<imagesize_t> histogram(src, int, ...)` | MISSING | HIGH |
| 160 | computePixelHashSHA1 | `std::string computePixelHashSHA1(src, ...)` | MISSING | MEDIUM |
| 161 | ROI_union | `ROI roi_union(A, B)` | MISSING | HIGH |
| 162 | ROI_intersection | `ROI roi_intersection(A, B)` | MISSING | HIGH |
| 163 | nonzero_region | `ROI nonzero_region(src, ROI, int)` | MISSING | MEDIUM |

## 12. ImageBufAlgo - Morphological

| # | Function | Signature | Status | Priority |
|---|----------|-----------|--------|----------|
| 164 | flood_fill | `bool flood_fill(dst, x, y, cspan<float>, ...)` | MISSING | MEDIUM |
| 165 | capture_image | `ImageBuf capture_image(int cameranum, TypeDesc)` | MISSING | LOW |

## 13. ImageBufAlgo - Misc

| # | Function | Signature | Status | Priority |
|---|----------|-----------|--------|----------|
| 166 | render_point | `bool render_point(dst, x, y, cspan<float>)` | MISSING | LOW |
| 167 | render_line | `bool render_line(dst, x1, y1, x2, y2, cspan, ...)` | MISSING | LOW |
| 168 | render_box | `bool render_box(dst, x1, y1, x2, y2, cspan, ...)` | MISSING | MEDIUM |
| 169 | render_text | `bool render_text(dst, x, y, text, fontsize, ...)` | MISSING | MEDIUM |

## 14. DeepData Class (CRITICAL - Compositing)

| # | Method | Signature | Status | Priority |
|---|--------|-----------|--------|----------|
| 170 | Constructor | `DeepData()`, `DeepData(ImageSpec&)` | MISSING | CRITICAL |
| 171 | init | `void init(int npix, int nchan, cspan<TypeDesc>, ...)` | MISSING | CRITICAL |
| 172 | initialized | `bool initialized()` | MISSING | HIGH |
| 173 | allocated | `bool allocated()` | MISSING | HIGH |
| 174 | pixels | `int pixels()` | MISSING | HIGH |
| 175 | channels | `int channels()` | MISSING | HIGH |
| 176 | Z_channel | `int Z_channel()` | MISSING | HIGH |
| 177 | Zback_channel | `int Zback_channel()` | MISSING | HIGH |
| 178 | A_channel | `int A_channel()` | MISSING | HIGH |
| 179 | AR_channel | `int AR_channel()` | MISSING | MEDIUM |
| 180 | AG_channel | `int AG_channel()` | MISSING | MEDIUM |
| 181 | AB_channel | `int AB_channel()` | MISSING | MEDIUM |
| 182 | channelname | `string_view channelname(int c)` | MISSING | HIGH |
| 183 | channeltype | `TypeDesc channeltype(int c)` | MISSING | HIGH |
| 184 | channelsize | `size_t channelsize(int c)` | MISSING | HIGH |
| 185 | samplesize | `size_t samplesize()` | MISSING | HIGH |
| 186 | samples | `int samples(int pixel)` | MISSING | CRITICAL |
| 187 | set_samples | `void set_samples(int pixel, int samps)` | MISSING | CRITICAL |
| 188 | set_all_samples | `void set_all_samples(cspan<uint32_t>)` | MISSING | HIGH |
| 189 | set_capacity | `void set_capacity(int samps)` | MISSING | HIGH |
| 190 | capacity | `int capacity()` | MISSING | MEDIUM |
| 191 | insert_samples | `void insert_samples(int pixel, int idx, int n)` | MISSING | HIGH |
| 192 | erase_samples | `void erase_samples(int pixel, int idx, int n)` | MISSING | HIGH |
| 193 | deep_value | `float deep_value(int pixel, int chan, int samp)` | MISSING | CRITICAL |
| 194 | deep_value_uint | `uint32_t deep_value_uint(...)` | MISSING | HIGH |
| 195 | set_deep_value | `void set_deep_value(int pixel, int chan, int samp, float)` | MISSING | CRITICAL |
| 196 | data_ptr | `void* data_ptr(int pixel, int chan, int samp)` | MISSING | HIGH |
| 197 | all_channeltypes | `cspan<TypeDesc> all_channeltypes()` | MISSING | MEDIUM |
| 198 | all_samples | `cspan<uint32_t> all_samples()` | MISSING | MEDIUM |
| 199 | all_data | `cspan<char> all_data()` | MISSING | MEDIUM |
| 200 | get_pointers | `void get_pointers(vector<void*>&)` | MISSING | MEDIUM |
| 201 | copy_deep_sample | `bool copy_deep_sample(...)` | MISSING | HIGH |
| 202 | copy_deep_pixel | `bool copy_deep_pixel(...)` | MISSING | HIGH |
| 203 | split | `bool split(int pixel, float depth)` | MISSING | MEDIUM |
| 204 | sort | `void sort(int pixel)` | MISSING | HIGH |
| 205 | merge_overlaps | `void merge_overlaps(int pixel)` | MISSING | HIGH |
| 206 | merge_deep_pixels | `void merge_deep_pixels(int pixel, DeepData&, int)` | MISSING | HIGH |
| 207 | occlusion_cull | `float occlusion_cull(int pixel)` | MISSING | HIGH |
| 208 | opaque_z | `float opaque_z(int pixel)` | MISSING | HIGH |

## 15. ImageCache Class

| # | Method | Signature | Status | Priority |
|---|--------|-----------|--------|----------|
| 209 | create | `static ImageCache* create(bool shared)` | MISSING | HIGH |
| 210 | destroy | `static void destroy(ImageCache*, bool)` | MISSING | HIGH |
| 211 | attribute | `bool attribute(string_view, TypeDesc, void*)` | MISSING | HIGH |
| 212 | getattribute | `bool getattribute(string_view, TypeDesc, void*)` | MISSING | HIGH |
| 213 | resolve_filename | `std::string resolve_filename(string_view)` | MISSING | HIGH |
| 214 | get_image_info | `bool get_image_info(ustring, int, int, ...)` | MISSING | HIGH |
| 215 | get_imagespec | `bool get_imagespec(ustring, ImageSpec&, int, int, ...)` | MISSING | HIGH |
| 216 | imagespec | `const ImageSpec* imagespec(ustring, int, int, ...)` | MISSING | HIGH |
| 217 | get_thumbnail | `bool get_thumbnail(ustring, ImageBuf&, int)` | MISSING | MEDIUM |
| 218 | get_pixels | `bool get_pixels(ustring, int, int, ...)` | MISSING | HIGH |
| 219 | invalidate | `void invalidate(ustring, bool)` | MISSING | HIGH |
| 220 | invalidate_all | `void invalidate_all(bool)` | MISSING | HIGH |
| 221 | close | `void close(ustring)` | MISSING | MEDIUM |
| 222 | close_all | `void close_all()` | MISSING | MEDIUM |
| 223 | getstats | `std::string getstats(int level)` | MISSING | MEDIUM |
| 224 | reset_stats | `void reset_stats()` | MISSING | LOW |
| 225 | Perthread | Struct for thread-specific data | MISSING | HIGH |
| 226 | Tile | Struct for tile data | MISSING | HIGH |

## 16. TextureSystem Class

| # | Method | Signature | Status | Priority |
|---|--------|-----------|--------|----------|
| 227 | create | `static TextureSystem* create(bool shared, ImageCache*)` | MISSING | HIGH |
| 228 | destroy | `static void destroy(TextureSystem*)` | MISSING | HIGH |
| 229 | attribute | `bool attribute(string_view, TypeDesc, void*)` | MISSING | HIGH |
| 230 | getattribute | `bool getattribute(string_view, TypeDesc, void*)` | MISSING | HIGH |
| 231 | texture | `bool texture(ustring, TextureOpt&, ...)` | MISSING | CRITICAL |
| 232 | texture3d | `bool texture3d(ustring, TextureOpt&, V3f, ...)` | MISSING | HIGH |
| 233 | shadow | `bool shadow(ustring, TextureOpt&, ...)` | MISSING | HIGH |
| 234 | environment | `bool environment(ustring, TextureOpt&, V3f, ...)` | MISSING | HIGH |
| 235 | TextureOpt | Struct for texture options | MISSING | HIGH |
| 236 | resolve_filename | `std::string resolve_filename(string_view)` | MISSING | HIGH |
| 237 | get_texture_info | `bool get_texture_info(ustring, int, ...)` | MISSING | HIGH |
| 238 | get_imagespec | `bool get_imagespec(ustring, int, ImageSpec&)` | MISSING | HIGH |
| 239 | imagespec | `const ImageSpec* imagespec(ustring, int)` | MISSING | HIGH |
| 240 | inventory_udim | `bool inventory_udim(ustring, vector<ustring>&, int&, int&)` | MISSING | HIGH |
| 241 | is_udim | `bool is_udim(ustring)` | MISSING | HIGH |
| 242 | resolve_udim | `ustring resolve_udim(ustring, float, float)` | MISSING | HIGH |

## 17. Color Management (ColorConfig)

| # | Method | Signature | Status | Priority |
|---|--------|-----------|--------|----------|
| 243 | Constructor | `ColorConfig(string_view filename)` | MISSING | CRITICAL |
| 244 | reset | `bool reset(string_view filename)` | MISSING | HIGH |
| 245 | error | `bool error()` | MISSING | HIGH |
| 246 | geterror | `std::string geterror(bool clear)` | MISSING | HIGH |
| 247 | getNumColorSpaces | `int getNumColorSpaces()` | MISSING | HIGH |
| 248 | getColorSpaceNameByIndex | `const char* getColorSpaceNameByIndex(int)` | MISSING | HIGH |
| 249 | getColorSpaceFamilyByName | `const char* getColorSpaceFamilyByName(...)` | MISSING | MEDIUM |
| 250 | getNumRoles | `int getNumRoles()` | MISSING | MEDIUM |
| 251 | getRoleByIndex | `const char* getRoleByIndex(int)` | MISSING | MEDIUM |
| 252 | getColorSpaceFromFilepath | `const char* getColorSpaceFromFilepath(...)` | MISSING | HIGH |
| 253 | parseColorSpaceFromString | `std::string parseColorSpaceFromString(...)` | MISSING | HIGH |
| 254 | getNumLooks | `int getNumLooks()` | MISSING | MEDIUM |
| 255 | getLookNameByIndex | `const char* getLookNameByIndex(int)` | MISSING | MEDIUM |
| 256 | getNumDisplays | `int getNumDisplays()` | MISSING | MEDIUM |
| 257 | getDisplayNameByIndex | `const char* getDisplayNameByIndex(int)` | MISSING | MEDIUM |
| 258 | getDefaultDisplayName | `const char* getDefaultDisplayName()` | MISSING | MEDIUM |
| 259 | getNumViews | `int getNumViews(string_view display)` | MISSING | MEDIUM |
| 260 | getViewNameByIndex | `const char* getViewNameByIndex(...)` | MISSING | MEDIUM |
| 261 | getDefaultViewName | `const char* getDefaultViewName(...)` | MISSING | MEDIUM |
| 262 | createColorProcessor | `ColorProcessorHandle createColorProcessor(...)` | MISSING | CRITICAL |
| 263 | createLookTransform | `ColorProcessorHandle createLookTransform(...)` | MISSING | HIGH |
| 264 | createDisplayTransform | `ColorProcessorHandle createDisplayTransform(...)` | MISSING | HIGH |
| 265 | createFileTransform | `ColorProcessorHandle createFileTransform(...)` | MISSING | HIGH |
| 266 | createMatrixTransform | `ColorProcessorHandle createMatrixTransform(...)` | MISSING | HIGH |
| 267 | getColorSpaceDataType | `std::string getColorSpaceDataType(...)` | MISSING | MEDIUM |
| 268 | equivalent | `bool equivalent(ColorProcessorHandle, ColorProcessorHandle)` | MISSING | LOW |

## 18. TypeDesc Extensions

| # | Feature | Description | Status | Priority |
|---|---------|-------------|--------|----------|
| 269 | BASETYPE enum | UNKNOWN, NONE, UINT8...DOUBLE, STRING, PTR, etc. | PARTIAL | HIGH |
| 270 | AGGREGATE enum | SCALAR, VEC2, VEC3, VEC4, MATRIX33, MATRIX44 | MISSING | HIGH |
| 271 | VECSEMANTICS enum | NOXFORM, NOSEMANTICS, COLOR, POINT, VECTOR, NORMAL, TIMECODE, KEYCODE, RATIONAL | MISSING | MEDIUM |
| 272 | elementtype | `TypeDesc elementtype()` | MISSING | HIGH |
| 273 | elementsize | `size_t elementsize()` | MISSING | HIGH |
| 274 | basesize | `size_t basesize()` | MISSING | HIGH |
| 275 | numelements | `size_t numelements()` | MISSING | HIGH |
| 276 | is_array | `bool is_array()` | MISSING | HIGH |
| 277 | is_unsized_array | `bool is_unsized_array()` | MISSING | MEDIUM |
| 278 | is_sized_array | `bool is_sized_array()` | MISSING | MEDIUM |
| 279 | is_floating_point | `bool is_floating_point()` | MISSING | HIGH |
| 280 | is_signed | `bool is_signed()` | MISSING | HIGH |
| 281 | size | `size_t size()` | MISSING | HIGH |
| 282 | scalartype | `BASETYPE scalartype()` | MISSING | HIGH |
| 283 | unarray | `TypeDesc unarray()` | MISSING | MEDIUM |

## 19. ROI (Region of Interest)

| # | Method | Signature | Status | Priority |
|---|--------|-----------|--------|----------|
| 284 | Constructor | `ROI(int, int, int, int, int, int, int, int)` | PARTIAL | HIGH |
| 285 | defined | `bool defined()` | PARTIAL | - |
| 286 | width | `int width()` | PARTIAL | - |
| 287 | height | `int height()` | PARTIAL | - |
| 288 | depth | `int depth()` | PARTIAL | - |
| 289 | nchannels | `int nchannels()` | PARTIAL | - |
| 290 | npixels | `imagesize_t npixels()` | MISSING | HIGH |
| 291 | All | `static ROI All()` | MISSING | HIGH |
| 292 | contains | `bool contains(int, int, int, int)` | MISSING | HIGH |
| 293 | contains_roi | `bool contains(ROI)` | MISSING | MEDIUM |

## 20. Plugin System

| # | Feature | Description | Status | Priority |
|---|---------|-------------|--------|----------|
| 294 | ImageInput::create | `static ImageInput* create(string_view, ...)` | PARTIAL | HIGH |
| 295 | ImageInput::open | `static ImageInput* open(string_view, ...)` | PARTIAL | HIGH |
| 296 | ImageOutput::create | `static ImageOutput* create(string_view, ...)` | PARTIAL | HIGH |
| 297 | Plugin loading | DSO/DLL dynamic loading | MISSING | LOW |
| 298 | Plugin search paths | Custom plugin directories | MISSING | LOW |
| 299 | declare_imageio_format | Registration macro | MISSING | LOW |

---

# SUMMARY

## OpenColorIO (133 items)

| Priority | Count | Categories |
|----------|-------|------------|
| CRITICAL | 34 | Transforms, Processors, FileRules, ViewingRules, Named/View transforms |
| HIGH | 58 | Config methods, Displays, Roles, Context, Cache |
| MEDIUM | 38 | Metadata, Utilities, Advanced features |
| LOW | 3 | Deprecated, Specialized |

## OpenImageIO (299 items)

| Priority | Count | Categories |
|----------|-------|------------|
| CRITICAL | 25 | DeepData, ImageBufAlgo core (crop, resize, over, colorconvert), TextureSystem |
| HIGH | 150 | ImageSpec, ImageBuf, ImageBufAlgo, ImageCache, ColorConfig |
| MEDIUM | 80 | Statistics, Morphological, Misc operations |
| LOW | 44 | Capture, Text rendering, Legacy |

## Total Missing Features: 432

---

*Report generated by Claude Bug Hunt Agent*
# PART 1: OpenColorIO Missing Features (133 items)

## 1. CONFIG PARSING & MANAGEMENT

| # | Feature | OCIO Method | File | Status | Priority |
|---|---------|-------------|------|--------|----------|
| 1 | Raw config fallback | `Config::CreateRaw()` | OpenColorIO.h:313 | MISSING | HIGH |
| 2 | Version getters | `getMajorVersion()`, `getMinorVersion()` | OpenColorIO.h:321-340 | MISSING | HIGH |
| 3 | Version setters | `setMajorVersion()`, `setMinorVersion()`, `setVersion()` | OpenColorIO.h:321-340 | MISSING | HIGH |
| 4 | Upgrade version | `upgradeToLatestVersion()` | OpenColorIO.h:340 | MISSING | MEDIUM |
| 5 | Config validation | `Config::validate()` | OpenColorIO.h:342 | MISSING | CRITICAL |
| 6 | Config name | `getName()`, `setName()` | OpenColorIO.h:344-346 | MISSING | MEDIUM |
| 7 | Config description | `getDescription()`, `setDescription()` | OpenColorIO.h:347-349 | MISSING | MEDIUM |
| 8 | Config serialization | `serialize(std::ostream&)` | OpenColorIO.h:351 | MISSING | CRITICAL |
| 9 | Family separator | `getFamilySeparator()`, `setFamilySeparator()` | OpenColorIO.h:355-362 | MISSING | MEDIUM |
| 10 | Cache ID | `getCacheID(ConstContextRcPtr&)` | OpenColorIO.h:368-372 | MISSING | HIGH |

## 2. ENVIRONMENT VARIABLES & CONTEXT

| # | Feature | OCIO Method | File | Status | Priority |
|---|---------|-------------|------|--------|----------|
| 11 | Add env var | `addEnvironmentVar()` | OpenColorIO.h:378 | MISSING | HIGH |
| 12 | Env var count | `getNumEnvironmentVars()` | OpenColorIO.h:380 | MISSING | HIGH |
| 13 | Env var by index | `getEnvironmentVarNameByIndex()` | OpenColorIO.h:382 | MISSING | HIGH |
| 14 | Env var default | `getEnvironmentVarDefault()` | OpenColorIO.h:384 | MISSING | HIGH |
| 15 | Clear env vars | `clearEnvironmentVars()` | OpenColorIO.h:387 | MISSING | MEDIUM |
| 16 | Environment mode | `setEnvironmentMode()`, `getEnvironmentMode()` | OpenColorIO.h:391-394 | MISSING | HIGH |
| 17 | Load environment | `loadEnvironment()` | OpenColorIO.h:396 | MISSING | HIGH |
| 18 | Working directory | `getWorkingDir()`, `setWorkingDir()` | OpenColorIO.h:405-411 | MISSING | MEDIUM |

## 3. SEARCH PATHS

| # | Feature | OCIO Method | File | Status | Priority |
|---|---------|-------------|------|--------|----------|
| 19 | Get search path | `getSearchPath()` (all variants) | OpenColorIO.h:399-410 | PARTIAL | HIGH |
| 20 | Set search path | `setSearchPath()` | OpenColorIO.h:412 | PARTIAL | HIGH |
| 21 | Search path count | `getNumSearchPaths()` | OpenColorIO.h:414 | MISSING | HIGH |
| 22 | Clear search paths | `clearSearchPaths()` | OpenColorIO.h:420 | MISSING | MEDIUM |
| 23 | Add search path | `addSearchPath()` | OpenColorIO.h:424 | MISSING | HIGH |

## 4. COLOR SPACES - ADVANCED

| # | Feature | OCIO Method | File | Status | Priority |
|---|---------|-------------|------|--------|----------|
| 24 | ColorSpaceSet by category | `getColorSpaces(category)` | OpenColorIO.h:430-442 | MISSING | HIGH |
| 25 | Search by reference type | `getNumColorSpaces(SearchReferenceSpaceType)` | OpenColorIO.h:444-455 | MISSING | HIGH |
| 26 | ColorSpace visibility | `ColorSpaceVisibility` enum | OpenColorIO.h | MISSING | HIGH |
| 27 | Index for colorspace | `getIndexForColorSpace(name)` | OpenColorIO.h:462-469 | MISSING | MEDIUM |
| 28 | Canonical name | `getCanonicalName(name)` | OpenColorIO.h:471-475 | MISSING | HIGH |
| 29 | Remove color space | `removeColorSpace(name)` | OpenColorIO.h:491-499 | MISSING | MEDIUM |
| 30 | Check usage | `isColorSpaceUsed(name)` | OpenColorIO.h:501-505 | MISSING | MEDIUM |
| 31 | Clear all spaces | `clearColorSpaces()` | OpenColorIO.h:507-512 | MISSING | MEDIUM |
| 32 | Inactive spaces list | `setInactiveColorSpaces()`, `getInactiveColorSpaces()` | OpenColorIO.h:514-528 | MISSING | HIGH |
| 33 | Is linear heuristics | `isColorSpaceLinear(name, refType)` | OpenColorIO.h:530-574 | MISSING | HIGH |
| 34 | Identify builtin | `IdentifyBuiltinColorSpace()` | OpenColorIO.h:576-589 | MISSING | MEDIUM |
| 35 | Identify interchange | `IdentifyInterchangeSpace()` | OpenColorIO.h:591-607 | MISSING | MEDIUM |

## 5. ROLES (SEMANTIC ACCESS)

| # | Feature | OCIO Method | File | Status | Priority |
|---|---------|-------------|------|--------|----------|
| 36 | Set role | `setRole()` | OpenColorIO.h:609 | MISSING | HIGH |
| 37 | Role count | `getNumRoles()` | OpenColorIO.h:615 | MISSING | HIGH |
| 38 | Has role | `hasRole()` | OpenColorIO.h:620 | MISSING | HIGH |
| 39 | Role name by index | `getRoleName(idx)` | OpenColorIO.h:625 | MISSING | HIGH |
| 40 | Role colorspace | `getRoleColorSpace()` | OpenColorIO.h:630-635 | MISSING | HIGH |

## 6. DISPLAYS & VIEWS - ADVANCED

| # | Feature | OCIO Method | File | Status | Priority |
|---|---------|-------------|------|--------|----------|
| 41 | Add shared view | `addSharedView()` | OpenColorIO.h:651 | MISSING | CRITICAL |
| 42 | Remove shared view | `removeSharedView()` | OpenColorIO.h:655 | MISSING | HIGH |
| 43 | Clear shared views | `clearSharedViews()` | OpenColorIO.h:660 | MISSING | MEDIUM |
| 44 | Is view shared | `isViewShared()` | OpenColorIO.h:669 | MISSING | MEDIUM |
| 45 | View transform support | Views with viewTransformName | OpenColorIO.h | MISSING | CRITICAL |
| 46 | USE_DISPLAY_NAME token | Special token for dynamic colorspace | OpenColorIO.h | MISSING | HIGH |
| 47 | Views comparison | `AreViewsEqual()` | OpenColorIO.h:671-685 | MISSING | MEDIUM |
| 48 | View transform name | `getDisplayViewTransformName()` | OpenColorIO.h:687 | MISSING | HIGH |
| 49 | View colorspace name | `getDisplayViewColorSpaceName()` | OpenColorIO.h:690 | MISSING | HIGH |
| 50 | View looks | `getDisplayViewLooks()` | OpenColorIO.h:693 | MISSING | HIGH |
| 51 | View rule | `getDisplayViewRule()` | OpenColorIO.h:696 | MISSING | HIGH |
| 52 | View description | `getDisplayViewDescription()` | OpenColorIO.h:700 | MISSING | MEDIUM |
| 53 | Has view | `hasView(display, view)` | OpenColorIO.h:702-712 | MISSING | MEDIUM |
| 54 | Default view by space | `getDefaultView(display, colorspaceName)` | OpenColorIO.h:447-451 | MISSING | HIGH |
| 55 | Views by colorspace | `getNumViews(display, colorspaceName)` | OpenColorIO.h:453-455 | MISSING | MEDIUM |
| 56 | Viewing rules | `getViewingRules()`, `setViewingRules()` | OpenColorIO.h:743-752 | MISSING | CRITICAL |

## 7. VIRTUAL DISPLAY (ICC PROFILES)

| # | Feature | OCIO Method | File | Status | Priority |
|---|---------|-------------|------|--------|----------|
| 57 | Add virtual view | `addVirtualDisplayView()` | OpenColorIO.h:758 | MISSING | HIGH |
| 58 | Virtual view count | `getVirtualDisplayNumViews()` | OpenColorIO.h:765 | MISSING | HIGH |
| 59 | Get virtual view | `getVirtualDisplayView()` | OpenColorIO.h:768 | MISSING | HIGH |
| 60 | Remove virtual view | `removeVirtualDisplayView()` | OpenColorIO.h:805 | MISSING | MEDIUM |
| 61 | Clear virtual display | `clearVirtualDisplay()` | OpenColorIO.h:816 | MISSING | MEDIUM |
| 62 | Virtual shared views | `addVirtualDisplaySharedView()` | OpenColorIO.h:810 | MISSING | MEDIUM |
| 63 | Has virtual view | `hasVirtualView()` | OpenColorIO.h:770 | MISSING | MEDIUM |
| 64 | Virtual view shared | `isVirtualViewShared()` | OpenColorIO.h:775 | MISSING | LOW |
| 65 | Virtual views equal | `AreVirtualViewsEqual()` | OpenColorIO.h:779-791 | MISSING | LOW |
| 66 | Virtual view attrs | `getVirtualDisplayViewTransformName()` etc. | OpenColorIO.h:793-801 | MISSING | HIGH |
| 67 | Display from monitor | `instantiateDisplayFromMonitorName()` | OpenColorIO.h:803-823 | MISSING | HIGH |
| 68 | Display from ICC | `instantiateDisplayFromICCProfile()` | OpenColorIO.h:825-836 | MISSING | HIGH |
| 69 | Display temporary | `isDisplayTemporary()`, `setDisplayTemporary()` | OpenColorIO.h:857-865 | MISSING | MEDIUM |
| 70 | Views by type | `getNumViews(ViewType, display)` | OpenColorIO.h:872-879 | MISSING | MEDIUM |
| 71 | All displays | `getNumDisplaysAll()`, `getDisplayAll()` | OpenColorIO.h:849-856 | MISSING | MEDIUM |

## 8. ACTIVE DISPLAYS/VIEWS FILTERING

| # | Feature | OCIO Method | File | Status | Priority |
|---|---------|-------------|------|--------|----------|
| 72 | Set active displays | `setActiveDisplays()` | OpenColorIO.h:881 | MISSING | HIGH |
| 73 | Get active displays | `getActiveDisplays()` | OpenColorIO.h:891 | MISSING | HIGH |
| 74 | Active display count | `getNumActiveDisplays()` | OpenColorIO.h:893 | MISSING | HIGH |
| 75 | Get active display | `getActiveDisplay(idx)` | OpenColorIO.h:895 | MISSING | HIGH |
| 76 | Add active display | `addActiveDisplay()` | OpenColorIO.h:897 | MISSING | HIGH |
| 77 | Remove active display | `removeActiveDisplay()` | OpenColorIO.h:900 | MISSING | MEDIUM |
| 78 | Clear active displays | `clearActiveDisplays()` | OpenColorIO.h:903 | MISSING | MEDIUM |
| 79 | Set active views | `setActiveViews()` | OpenColorIO.h:905 | MISSING | HIGH |
| 80 | Get active views | `getActiveViews()` | OpenColorIO.h:920 | MISSING | HIGH |
| 81 | Active view methods | `getNumActiveViews()`, `getActiveView()` etc. | OpenColorIO.h:922-932 | MISSING | HIGH |

## 9. LUMA COEFFICIENTS

| # | Feature | OCIO Method | File | Status | Priority |
|---|---------|-------------|------|--------|----------|
| 82 | Default luma coefs | `getDefaultLumaCoefs()`, `setDefaultLumaCoefs()` | OpenColorIO.h:750-763 | MISSING | MEDIUM |

## 10. LOOKS

| # | Feature | OCIO Method | File | Status | Priority |
|---|---------|-------------|------|--------|----------|
| 83 | Get look | `getLook()` | OpenColorIO.h:765 | PARTIAL | CRITICAL |
| 84 | Look count | `getNumLooks()` | OpenColorIO.h:770 | PARTIAL | HIGH |
| 85 | Look by index | `getLookNameByIndex()` | OpenColorIO.h:775 | PARTIAL | HIGH |
| 86 | Add look | `addLook()` | OpenColorIO.h:780 | MISSING | CRITICAL |
| 87 | Clear looks | `clearLooks()` | OpenColorIO.h:785 | MISSING | MEDIUM |

## 11. VIEW TRANSFORMS (OCIO v2)

| # | Feature | OCIO Method | File | Status | Priority |
|---|---------|-------------|------|--------|----------|
| 88 | View transform count | `getNumViewTransforms()` | OpenColorIO.h:787 | MISSING | CRITICAL |
| 89 | Get view transform | `getViewTransform()` | OpenColorIO.h:790 | MISSING | CRITICAL |
| 90 | View transform by idx | `getViewTransformNameByIndex()` | OpenColorIO.h:795 | MISSING | CRITICAL |
| 91 | Add view transform | `addViewTransform()` | OpenColorIO.h:800 | MISSING | CRITICAL |
| 92 | Clear view transforms | `clearViewTransforms()` | OpenColorIO.h:806 | MISSING | MEDIUM |
| 93 | Default view transform | `getDefaultSceneToDisplayViewTransform()` | OpenColorIO.h:808 | MISSING | HIGH |
| 94 | Default VT name | `getDefaultViewTransformName()`, `setDefaultViewTransformName()` | OpenColorIO.h:815-823 | MISSING | HIGH |

## 12. NAMED TRANSFORMS (OCIO v2)

| # | Feature | OCIO Method | File | Status | Priority |
|---|---------|-------------|------|--------|----------|
| 95 | Named transform visibility | `getNumNamedTransforms(visibility)` | OpenColorIO.h:828-831 | MISSING | HIGH |
| 96 | Get named transform | `getNamedTransform()` | OpenColorIO.h:825 | MISSING | CRITICAL |
| 97 | Named transform count | `getNumNamedTransforms()` | OpenColorIO.h:833 | MISSING | CRITICAL |
| 98 | Named transform by idx | `getNamedTransformNameByIndex()` | OpenColorIO.h:838 | MISSING | CRITICAL |
| 99 | Index for named transform | `getIndexForNamedTransform()` | OpenColorIO.h:843 | MISSING | HIGH |
| 100 | Add named transform | `addNamedTransform()` | OpenColorIO.h:848 | MISSING | CRITICAL |
| 101 | Remove named transform | `removeNamedTransform()` | OpenColorIO.h:852 | MISSING | MEDIUM |
| 102 | Clear named transforms | `clearNamedTransforms()` | OpenColorIO.h:857 | MISSING | MEDIUM |

## 13. FILE RULES

| # | Feature | OCIO Method | File | Status | Priority |
|---|---------|-------------|------|--------|----------|
| 103 | Get file rules | `getFileRules()` | OpenColorIO.h:859 | PARTIAL | CRITICAL |
| 104 | Set file rules | `setFileRules()` | OpenColorIO.h:865 | MISSING | CRITICAL |
| 105 | Colorspace from filepath | `getColorSpaceFromFilepath()` | OpenColorIO.h:870-875 | MISSING | CRITICAL |
| 106 | Default rule only | `filepathOnlyMatchesDefaultRule()` | OpenColorIO.h:880 | MISSING | HIGH |
| 107 | Strict parsing | `isStrictParsingEnabled()`, `setStrictParsingEnabled()` | OpenColorIO.h:894-895 | MISSING | MEDIUM |

## 14. PROCESSORS - ADVANCED

| # | Feature | OCIO Method | File | Status | Priority |
|---|---------|-------------|------|--------|----------|
| 108 | Processor from pair | `getProcessor(context, src, dst)` | OpenColorIO.h:901-914 | PARTIAL | CRITICAL |
| 109 | Processor from display | `getProcessor(src, display, view, dir)` | OpenColorIO.h:916-922 | PARTIAL | CRITICAL |
| 110 | Processor from named | `getProcessor(namedTransform, dir)` | OpenColorIO.h:924-933 | MISSING | HIGH |
| 111 | Processor from transform | `getProcessor(transform)` | OpenColorIO.h:935-941 | PARTIAL | HIGH |
| 112 | To builtin processor | `GetProcessorToBuiltinColorSpace()` | OpenColorIO.h:943-955 | MISSING | HIGH |
| 113 | From builtin processor | `GetProcessorFromBuiltinColorSpace()` | OpenColorIO.h:957-963 | MISSING | HIGH |
| 114 | Processor from configs | `GetProcessorFromConfigs()` (9 overloads) | OpenColorIO.h:965-1057 | MISSING | CRITICAL |
| 115 | Processor cache flags | `getProcessorCacheFlags()`, `setProcessorCacheFlags()` | OpenColorIO.h:1059-1073 | MISSING | MEDIUM |
| 116 | Config IO proxy | `setConfigIOProxy()`, `getConfigIOProxy()` | OpenColorIO.h:1075-1076 | MISSING | MEDIUM |

## 15. ARCHIVING

| # | Feature | OCIO Method | File | Status | Priority |
|---|---------|-------------|------|--------|----------|
| 117 | Is archivable | `isArchivable()` | OpenColorIO.h:1078-1100 | MISSING | MEDIUM |
| 118 | Archive to stream | `archive(std::ostream&)` | OpenColorIO.h:1102-1119 | MISSING | HIGH |
| 119 | Extract archive | `ExtractOCIOZArchive()` | OpenColorIO.h:220-235 | MISSING | MEDIUM |

## 16. TRANSFORM TYPES - MISSING

| # | Feature | OCIO Class | File | Status | Priority |
|---|---------|------------|------|--------|----------|
| 120 | Lut1DTransform | `Lut1DTransform` | OpenColorTransforms.h:1803-1906 | MISSING | CRITICAL |
| 121 | Lut3DTransform | `Lut3DTransform` | OpenColorTransforms.h:1907-2000 | MISSING | CRITICAL |
| 122 | LogAffineTransform | `LogAffineTransform` | OpenColorTransforms.h:1604-1650 | MISSING | HIGH |
| 123 | LogCameraTransform | `LogCameraTransform` | OpenColorTransforms.h:1652-1710 | MISSING | HIGH |
| 124 | ExponentWithLinear | `ExponentWithLinearTransform` | OpenColorTransforms.h:952-1008 | MISSING | MEDIUM |
| 125 | GradingHueCurve | `GradingHueCurveTransform` | OpenColorTransforms.h:1325-1401 | MISSING | HIGH |

## 17. DYNAMIC PROPERTIES

| # | Feature | OCIO Class | File | Status | Priority |
|---|---------|------------|------|--------|----------|
| 126 | Dynamic property base | `DynamicProperty` | OpenColorTransforms.h:766-895 | MISSING | CRITICAL |
| 127 | Dynamic double | `DynamicPropertyDouble` | OpenColorTransforms.h | MISSING | CRITICAL |
| 128 | Dynamic grading primary | `DynamicPropertyGradingPrimary` | OpenColorTransforms.h | MISSING | CRITICAL |
| 129 | Dynamic RGB curve | `DynamicPropertyGradingRGBCurve` | OpenColorTransforms.h | MISSING | CRITICAL |
| 130 | Dynamic hue curve | `DynamicPropertyGradingHueCurve` | OpenColorTransforms.h | MISSING | CRITICAL |
| 131 | Dynamic grading tone | `DynamicPropertyGradingTone` | OpenColorTransforms.h | MISSING | CRITICAL |

## 18. GLOBAL FUNCTIONS & UTILITIES

| # | Feature | OCIO Function | File | Status | Priority |
|---|---------|---------------|------|--------|----------|
| 132 | Current config | `GetCurrentConfig()`, `SetCurrentConfig()` | OpenColorIO.h:181-182 | PARTIAL | CRITICAL |
| 133 | Builtin config | `Config::CreateFromBuiltinConfig()` | OpenColorIO.h:327-373 | MISSING | CRITICAL |

---

