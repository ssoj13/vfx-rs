# VFX-RS TODO

Rust port of OpenImageIO + OpenColorIO

## Completed

### Core Infrastructure
- [x] `vfx-core` - Image, Pixel, Rect, ColorSpace, ImageSpec
- [x] `vfx-math` - Mat3, Vec3, interpolation, chromatic adaptation
- [x] `vfx-math::simd` - SIMD batch operations (wide crate)

### Image I/O (OIIO)
- [x] OpenEXR (exr crate)
- [x] PNG (png crate)
- [x] JPEG (jpeg-decoder/jpeg-encoder)
- [x] TIFF (tiff crate)
- [x] DPX (10-bit, 12-bit, 16-bit)
- [x] Sequence handling (FrameRange, FrameSet, pattern parsing)
- [x] Format auto-detection (magic bytes + extension)

### LUT Support
- [x] 1D LUT (Lut1D) with linear interpolation
- [x] 3D LUT (Lut3D) with trilinear/tetrahedral interpolation
- [x] CLF (Academy Common LUT Format) - full XML parser
- [x] SPI1D/SPI3D (Sony Pictures Imageworks)

### Transfer Functions
- [x] sRGB EOTF/OETF
- [x] Gamma (configurable)
- [x] PQ (Perceptual Quantizer / ST.2084)
- [x] HLG (Hybrid Log-Gamma)
- [x] ARRI LogC3 (EI 800)
- [x] Sony S-Log3
- [x] Panasonic V-Log
- [x] Rec.709 (BT.1886)

### Color Primaries
- [x] sRGB / Rec.709
- [x] Rec.2020
- [x] DCI-P3 / Display P3
- [x] ACES AP0 / AP1
- [x] Adobe RGB
- [x] RGB<->XYZ matrices
- [x] Chromatic adaptation (Bradford, CAT02, Von Kries)

### Color Processing (OCIO)
- [x] Pipeline/Processor architecture
- [x] CDL (ASC Color Decision List) with .cc/.ccc parsing
- [x] Matrix transforms
- [x] ICC profile support (lcms2)
- [x] OCIO config system (`vfx-ocio` crate)
- [x] ColorSpace, Roles, Display/View, Looks
- [x] Context/environment variable substitution
- [x] Built-in ACES 1.3, sRGB Studio, Rec.709 configs

### Image Operations
- [x] Resize (bilinear, bicubic, Lanczos)
- [x] Crop, pad, flip, rotate
- [x] Composite (over, add, multiply, screen)
- [x] Convolution filters (blur, sharpen, edge detect)
- [x] Parallel processing (rayon)

### Infrastructure
- [x] CLI tool (vfx-cli)
- [x] Integration tests (vfx-tests)
- [x] Benchmarks (criterion)
- [x] CI/CD (GitHub Actions)
- [x] Rustdocs

---

## TODO

### OCIO Config System (Priority 1) - DONE
- [x] `.ocio` YAML config parsing
- [x] ColorSpace definitions from config
- [x] Roles (scene_linear, compositing_log, color_timing, etc.)
- [x] Display/View management
- [x] Looks (creative transforms)
- [x] Context/environment variables
- [ ] File rules (regex -> colorspace mapping) - partial
- [ ] Viewing rules
- [x] Built-in ACES configs (aces_1.3, srgb_studio, rec709_studio)

### OCIO Transforms (Priority 2) - MOSTLY DONE
- [x] AllocationTransform (log/linear allocation)
- [x] ExposureContrastTransform
- [x] FixedFunctionTransform (RGB<->HSV, ACES gamut compress)
- [x] GradingPrimaryTransform (lift/gamma/gain)
- [x] GradingRGBCurveTransform
- [x] GradingToneTransform (shadows/midtones/highlights)
- [x] RangeTransform (with clamping options)
- [x] GroupTransform (nested transforms)
- [ ] DisplayViewTransform (config-level resolution)
- [ ] LookTransform (config-level resolution)

### More Image Formats (Priority 3)
- [ ] HDR/RGBE (Radiance)
- [ ] PSD (Photoshop) - read only
- [ ] BMP
- [ ] TGA (Targa)
- [ ] GIF (read only)
- [ ] ICO
- [ ] PNM/PPM/PGM/PBM
- [ ] PFM (Portable FloatMap)
- [ ] SGI/RGB
- [ ] IFF/ILBM
- [ ] FITS (astronomical)
- [ ] RAW (libraw wrapper) - Canon, Nikon, Sony, etc.
- [ ] OpenEXR deep images
- [ ] OpenEXR multi-part

### ImageCache & TextureSystem (Priority 4)
- [ ] ImageCache - tiled image caching
- [ ] Tile-based lazy loading
- [ ] LRU cache eviction
- [ ] Memory budget management
- [ ] TextureSystem - texture sampling
- [ ] Mipmap generation
- [ ] Anisotropic filtering
- [ ] Texture wrapping modes
- [ ] `maketx` - texture preparation tool

### ImageBufAlgo Extensions (Priority 5)
- [ ] channel_append, channel_extract, flatten
- [ ] noise (gaussian, uniform, salt-pepper)
- [ ] checker, fill patterns
- [ ] histogram, computePixelStats
- [ ] compare/diff (PSNR, SSIM)
- [ ] text rendering (with font support)
- [ ] warp/remap (UV distortion)
- [ ] colorconvert with OCIO config
- [ ] premult/unpremult
- [ ] fixNonFinite (NaN/Inf handling)
- [ ] deep_merge, deep_holdout

### Camera Color Spaces (Priority 6)
- [ ] ARRI LogC4 / AWG4
- [ ] RED Log3G10 / REDWideGamutRGB
- [ ] Sony S-Log3 / S-Gamut3.Cine
- [ ] Canon Log2/Log3 / Cinema Gamut
- [ ] Blackmagic Film Gen5
- [ ] Panasonic V-Log / V-Gamut
- [ ] Fujifilm F-Log / F-Gamut
- [ ] DJI D-Log / D-Gamut
- [ ] GoPro ProTune

### GPU Acceleration (Priority 7)
- [ ] GLSL shader generation
- [ ] HLSL shader generation  
- [ ] MSL (Metal) shader generation
- [ ] wgpu compute shaders
- [ ] GPU LUT baking
- [ ] GPU-accelerated color transforms

### Additional Features
- [ ] EXIF/XMP/IPTC metadata handling
- [ ] Color picker / sampler
- [ ] Histogram display
- [ ] Waveform/Vectorscope
- [ ] False color display
- [ ] Exposure zebras
- [ ] Focus peaking

### Python Bindings
- [ ] PyO3 bindings
- [ ] NumPy array interop
- [ ] Drop-in replacement API for PyOpenColorIO

### Documentation & Examples
- [ ] mdbook documentation site
- [ ] Tutorial examples
- [ ] Migration guide from OIIO/OCIO
- [ ] Publish to crates.io

---

## Architecture Notes

```
vfx-rs workspace
├── vfx-core         # Image, Pixel, Rect, ColorSpace
├── vfx-math         # Mat3, Vec3, SIMD, interpolation
├── vfx-lut          # Lut1D, Lut3D, CLF, SPI
├── vfx-transfer     # Transfer functions (sRGB, PQ, LogC, etc.)
├── vfx-primaries    # Color primaries, RGB/XYZ matrices
├── vfx-color        # Pipeline, CDL, conversions
├── vfx-io           # Image I/O (EXR, PNG, TIFF, DPX, etc.)
├── vfx-ops          # Image operations (resize, filter, composite)
├── vfx-icc          # ICC profile support (lcms2)
├── vfx-ocio         # OCIO config system (TODO)
├── vfx-cache        # ImageCache/TextureSystem (TODO)
├── vfx-cli          # Command-line tool
├── vfx-tests        # Integration tests
└── vfx-bench        # Benchmarks
```

## Test Status

- 307 unit tests
- 176 doctests  
- **483 total tests**
- All passing on Windows/Linux/macOS
