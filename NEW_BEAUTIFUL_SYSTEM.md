# VFX-RS: Unified Image & Color Management System for Rust

## Executive Summary

Проект объединяет функциональность OpenColorIO и OpenImageIO в единую Rust-native систему с современной архитектурой, без legacy-багажа C++.

**Ключевые преимущества нового подхода:**
- Color management интегрирован в каждую image операцию
- Zero-copy где возможно, SIMD везде
- Единая система типов для pixel/color/image
- Compile-time проверки цветовых пространств
- GPU-first дизайн с CPU fallback

---

## Part 1: Architecture Overview

### 1.1 Crate Structure

```
vfx-rs/
├── vfx-core/           # Базовые типы: Pixel, ColorSpace, ImageSpec
├── vfx-color/          # Color transforms, LUT, matrices, ACES
├── vfx-io/             # Format readers/writers
├── vfx-ops/            # Image processing algorithms
├── vfx-cache/          # ImageCache, TextureSystem
├── vfx-gpu/            # GPU processing (wgpu)
└── vfx/                # Unified re-export crate
```

### 1.2 Core Design Principles

```rust
// Compile-time color space safety
let linear: Image<Linear, f32> = read("input.exr")?;
let srgb: Image<Srgb, u8> = linear.convert::<Srgb>().quantize();
write("output.jpg", &srgb)?;

// Color space mismatch = compile error!
// let bad: Image<Srgb, f32> = linear; // ERROR!
```

### 1.3 Data Flow

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   vfx-io    │────▶│  vfx-core   │────▶│  vfx-ops    │
│ (formats)   │     │ (Image<C,T>)│     │ (algorithms)│
└─────────────┘     └─────────────┘     └─────────────┘
                           │
                    ┌──────┴──────┐
                    ▼             ▼
              ┌─────────┐   ┌─────────┐
              │vfx-color│   │ vfx-gpu │
              │(ACES,LUT)│   │ (wgpu)  │
              └─────────┘   └─────────┘
```

---

## Part 2: Complete Crate Ecosystem

### 2.0 Our Custom Crates (to be created)

Мы создаём собственные специализированные крейты, которые можно использовать независимо:

```
vfx-rs/                           # Workspace root
│
├── crates/
│   │
│   ├── vfx-core/                 # Базовые типы
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── pixel.rs          # Pixel<C, T>, Rgba, Rgb
│   │   │   ├── image.rs          # Image<C, T>, ImageView
│   │   │   ├── spec.rs           # ImageSpec, metadata
│   │   │   ├── colorspace.rs     # ColorSpace trait, markers
│   │   │   ├── rect.rs           # ROI, Rect, bounds
│   │   │   └── error.rs
│   │   └── Cargo.toml
│   │
│   ├── vfx-math/                 # Математика для color/image
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── matrix.rs         # Mat3x3, Mat4x4 для color
│   │   │   ├── vector.rs         # Vec3, Vec4
│   │   │   ├── interpolate.rs    # Lerp, cubic, catmull-rom
│   │   │   └── simd.rs           # SIMD helpers
│   │   └── Cargo.toml            # deps: glam, wide
│   │
│   ├── vfx-lut/                  # LUT типы и интерполяция
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── lut1d.rs          # 1D LUT
│   │   │   ├── lut3d.rs          # 3D LUT + trilinear/tetrahedral
│   │   │   ├── halftone.rs       # Hald CLUT
│   │   │   └── ops.rs            # LUT operations (combine, invert)
│   │   └── Cargo.toml            # deps: vfx-math
│   │
│   ├── vfx-lut-formats/          # ⭐ Парсеры LUT форматов
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── cube.rs           # .cube (Resolve, Adobe)
│   │   │   ├── clf.rs            # .clf (ACES Common LUT Format)
│   │   │   ├── ctf.rs            # .ctf (OCIO v2 CTF)
│   │   │   ├── csp.rs            # .csp (Cinespace)
│   │   │   ├── spi.rs            # .spi1d, .spi3d (Sony)
│   │   │   ├── lut3dl.rs         # .3dl (Lustre, Flame, Nuke)
│   │   │   ├── mga.rs            # .mga (Pandora)
│   │   │   ├── vlt.rs            # .vlt (Panasonic VariCam)
│   │   │   └── detect.rs         # Auto-detect format
│   │   └── Cargo.toml            # deps: vfx-lut, quick-xml, nom
│   │
│   ├── vfx-icc/                  # ⭐ ICC Profile support
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── parse.rs          # ICC v2/v4 parser
│   │   │   ├── profile.rs        # Profile struct
│   │   │   ├── tags.rs           # Tag types (TRC, XYZ, etc)
│   │   │   ├── transform.rs      # Profile to transform
│   │   │   └── embed.rs          # Extract from images
│   │   └── Cargo.toml            # standalone, no deps
│   │
│   ├── vfx-transfer/             # ⭐ Transfer functions (OETF/EOTF)
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── srgb.rs           # sRGB
│   │   │   ├── gamma.rs          # Pure gamma
│   │   │   ├── bt1886.rs         # Rec.709/2020
│   │   │   ├── pq.rs             # ST.2084 PQ (HDR)
│   │   │   ├── hlg.rs            # HLG (HDR broadcast)
│   │   │   ├── log.rs            # Generic log (base, offset)
│   │   │   ├── cineon.rs         # Cineon/DPX log
│   │   │   ├── arri.rs           # ARRI LogC3, LogC4
│   │   │   ├── red.rs            # RED Log3G10
│   │   │   ├── sony.rs           # S-Log2, S-Log3
│   │   │   ├── blackmagic.rs     # BMD Film Gen 5
│   │   │   ├── canon.rs          # Canon Log 2/3
│   │   │   ├── panasonic.rs      # V-Log
│   │   │   └── acescct.rs        # ACEScct, ACEScc
│   │   └── Cargo.toml            # deps: vfx-math
│   │
│   ├── vfx-primaries/            # ⭐ Color primaries & white points
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── primaries.rs      # RGB primaries (xy coords)
│   │   │   ├── whitepoint.rs     # D50, D55, D60, D65, DCI
│   │   │   ├── standard.rs       # sRGB, Rec709, Rec2020, DCI-P3, ACES
│   │   │   ├── matrices.rs       # RGB <-> XYZ matrices
│   │   │   └── adapt.rs          # Chromatic adaptation (Bradford, etc)
│   │   └── Cargo.toml            # deps: vfx-math
│   │
│   ├── vfx-aces/                 # ⭐ ACES transforms
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── spaces.rs         # ACES2065-1, ACEScg, ACEScct, ACEScc
│   │   │   ├── idt/              # Input Device Transforms
│   │   │   │   ├── mod.rs
│   │   │   │   ├── arri.rs
│   │   │   │   ├── red.rs
│   │   │   │   ├── sony.rs
│   │   │   │   └── generic.rs
│   │   │   ├── rrt.rs            # Reference Rendering Transform
│   │   │   ├── odt/              # Output Device Transforms
│   │   │   │   ├── mod.rs
│   │   │   │   ├── srgb.rs       # sRGB 100 nits
│   │   │   │   ├── rec709.rs     # Rec.709
│   │   │   │   ├── rec2020.rs    # Rec.2020 SDR/HDR
│   │   │   │   ├── p3.rs         # DCI-P3, Display P3
│   │   │   │   └── pq.rs         # PQ 1000/2000/4000 nits
│   │   │   ├── lmt.rs            # Look Modification Transforms
│   │   │   └── ctl.rs            # CTL file parser (optional)
│   │   └── Cargo.toml            # deps: vfx-transfer, vfx-primaries, vfx-lut
│   │
│   ├── vfx-config/               # ⭐ Config file support (OCIO-compatible)
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── parse.rs          # YAML config parser
│   │   │   ├── config.rs         # Config struct
│   │   │   ├── colorspace.rs     # ColorSpace definitions
│   │   │   ├── display.rs        # Display/View definitions
│   │   │   ├── look.rs           # Look definitions
│   │   │   ├── transform.rs      # Transform chain
│   │   │   ├── builtin.rs        # Built-in transforms
│   │   │   └── validate.rs       # Config validation
│   │   └── Cargo.toml            # deps: serde_yaml, vfx-aces, vfx-lut-formats
│   │
│   ├── vfx-color/                # Unified color API (re-exports)
│   │   ├── src/
│   │   │   ├── lib.rs            # Re-exports all color crates
│   │   │   ├── processor.rs      # ColorProcessor (cached transform)
│   │   │   ├── context.rs        # ColorContext (config + cache)
│   │   │   └── ops.rs            # High-level operations
│   │   └── Cargo.toml            # deps: all vfx-* color crates
│   │
│   ├── vfx-dpx/                  # ⭐ DPX/Cineon codec
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── header.rs         # DPX header structs
│   │   │   ├── read.rs           # Reader
│   │   │   ├── write.rs          # Writer
│   │   │   ├── packing.rs        # 10-bit, 12-bit, 16-bit packing
│   │   │   ├── cineon.rs         # Cineon format
│   │   │   └── metadata.rs       # Film/TV metadata
│   │   └── Cargo.toml            # deps: vfx-core, byteorder
│   │
│   ├── vfx-exr/                  # EXR wrapper (thin layer over exr crate)
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── read.rs
│   │   │   ├── write.rs
│   │   │   ├── multipart.rs      # Multi-part support
│   │   │   └── deep.rs           # Deep image support
│   │   └── Cargo.toml            # deps: exr, vfx-core
│   │
│   ├── vfx-io/                   # Format registry & unified I/O
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── registry.rs       # Format registry
│   │   │   ├── detect.rs         # Format detection (magic bytes)
│   │   │   ├── read.rs           # Unified read API
│   │   │   ├── write.rs          # Unified write API
│   │   │   ├── formats/          # Format implementations
│   │   │   │   ├── mod.rs
│   │   │   │   ├── exr.rs        # via vfx-exr
│   │   │   │   ├── dpx.rs        # via vfx-dpx
│   │   │   │   ├── png.rs        # via image crate
│   │   │   │   ├── jpeg.rs       # via image crate
│   │   │   │   ├── tiff.rs       # via image crate
│   │   │   │   ├── webp.rs       # via image crate
│   │   │   │   └── raw.rs        # via rawkit
│   │   │   └── error.rs
│   │   └── Cargo.toml            # deps: vfx-exr, vfx-dpx, image
│   │
│   ├── vfx-ops/                  # Image operations
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── resize.rs         # Resize algorithms
│   │   │   ├── transform.rs      # Affine transforms
│   │   │   ├── crop.rs           # Crop, pad
│   │   │   ├── flip.rs           # Flip, rotate
│   │   │   ├── composite.rs      # Porter-Duff, blend modes
│   │   │   ├── filter.rs         # Blur, sharpen, etc
│   │   │   ├── histogram.rs      # Histogram ops
│   │   │   ├── noise.rs          # Noise reduction/generation
│   │   │   └── parallel.rs       # Parallel execution helpers
│   │   └── Cargo.toml            # deps: vfx-core, rayon, fast_image_resize
│   │
│   ├── vfx-grading/              # ⭐ Color grading operations
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── cdl.rs            # ASC-CDL
│   │   │   ├── lgg.rs            # Lift/Gamma/Gain
│   │   │   ├── offset.rs         # Offset/Power/Slope
│   │   │   ├── hsv.rs            # Hue/Sat/Val adjustments
│   │   │   ├── curves.rs         # RGB curves, luma curve
│   │   │   ├── exposure.rs       # Exposure, contrast
│   │   │   ├── wb.rs             # White balance
│   │   │   ├── selective.rs      # Selective color
│   │   │   └── printer.rs        # Printer lights
│   │   └── Cargo.toml            # deps: vfx-core, vfx-math
│   │
│   ├── vfx-cache/                # Image cache & texture system
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── cache.rs          # ImageCache
│   │   │   ├── tile.rs           # Tile management
│   │   │   ├── texture.rs        # TextureSystem
│   │   │   ├── mipmap.rs         # Mipmap generation
│   │   │   └── stats.rs          # Cache statistics
│   │   └── Cargo.toml            # deps: vfx-io, parking_lot, lru
│   │
│   ├── vfx-gpu/                  # GPU acceleration
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── context.rs        # GPU context (wgpu)
│   │   │   ├── image.rs          # GpuImage
│   │   │   ├── pipelines/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── lut.rs        # LUT apply
│   │   │   │   ├── matrix.rs     # Color matrix
│   │   │   │   ├── transfer.rs   # Transfer functions
│   │   │   │   ├── resize.rs     # GPU resize
│   │   │   │   └── composite.rs  # GPU composite
│   │   │   ├── shaders/          # WGSL shaders
│   │   │   │   ├── lut3d.wgsl
│   │   │   │   ├── matrix.wgsl
│   │   │   │   └── ...
│   │   │   └── batch.rs          # Batch processing
│   │   └── Cargo.toml            # deps: wgpu, vfx-core
│   │
│   └── vfx/                      # ⭐ Main crate (re-exports everything)
│       ├── src/
│       │   ├── lib.rs
│       │   └── prelude.rs        # Common imports
│       └── Cargo.toml            # deps: all vfx-* crates
│
├── tools/                        # CLI tools
│   ├── vfxconvert/               # Image conversion tool
│   ├── vfxinfo/                  # Image info tool
│   └── vfxlut/                   # LUT manipulation tool
│
├── bindings/                     # Language bindings
│   ├── vfx-py/                   # Python bindings (PyO3)
│   └── vfx-c/                    # C API
│
├── Cargo.toml                    # Workspace manifest
└── README.md
```

### 2.0.1 Crate Dependency Graph

```
                                    ┌─────────────┐
                                    │    vfx      │ (main crate)
                                    └──────┬──────┘
                                           │
           ┌───────────────┬───────────────┼───────────────┬───────────────┐
           │               │               │               │               │
           ▼               ▼               ▼               ▼               ▼
    ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌─────────────┐
    │  vfx-color  │ │   vfx-io    │ │  vfx-ops    │ │ vfx-cache   │ │  vfx-gpu    │
    └──────┬──────┘ └──────┬──────┘ └──────┬──────┘ └──────┬──────┘ └──────┬──────┘
           │               │               │               │               │
           │          ┌────┴────┐          │               │               │
           │          ▼         ▼          │               │               │
           │    ┌─────────┐ ┌─────────┐    │               │               │
           │    │ vfx-exr │ │ vfx-dpx │    │               │               │
           │    └────┬────┘ └────┬────┘    │               │               │
           │         │           │         │               │               │
    ┌──────┴─────────┴───────────┴─────────┴───────────────┴───────────────┘
    │
    ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                              vfx-core                                        │
│                    (Image<C,T>, Pixel, ColorSpace trait)                    │
└─────────────────────────────────────────────────────────────────────────────┘
    ▲
    │
    ├─────────────┬─────────────┬─────────────┬─────────────┐
    │             │             │             │             │
┌───┴───┐   ┌─────┴─────┐ ┌─────┴─────┐ ┌─────┴─────┐ ┌─────┴─────┐
│vfx-lut│   │vfx-transfer│ │vfx-primaries│ │vfx-aces │ │vfx-config │
└───┬───┘   └─────┬─────┘ └─────┬─────┘ └─────┬─────┘ └─────┬─────┘
    │             │             │             │             │
    │             └──────┬──────┴─────────────┘             │
    │                    │                                  │
    ▼                    ▼                                  │
┌─────────────┐   ┌─────────────┐                          │
│vfx-lut-formats│  │  vfx-math   │◄─────────────────────────┘
└─────────────┘   └─────────────┘
         │              ▲
         │              │
         ▼              │
    ┌─────────┐         │
    │ vfx-icc │─────────┘
    └─────────┘
```

### 2.0.2 Individual Crate Details

#### vfx-lut-formats

**Purpose:** Парсинг всех LUT форматов

**Поддерживаемые форматы:**

| Format | Extension | Used by | Complexity |
|--------|-----------|---------|------------|
| Cube | .cube | Resolve, Adobe, most tools | Low |
| CLF | .clf | ACES, OCIO v2 | High |
| CTF | .ctf | OCIO v2 | Medium |
| Cinespace | .csp | Cinespace | Low |
| SPI | .spi1d, .spi3d | Sony Pictures | Low |
| Lustre 3DL | .3dl | Autodesk Lustre, Flame | Medium |
| Nuke 3DL | .3dl | Nuke (different format!) | Medium |
| Iridas | .itx, .look | Iridas SpeedGrade | Medium |
| Pandora | .mga | Pandora | Low |
| Truelight | .cub | FilmLight | Medium |
| ICC | .icc, .icm | Color profiles | High (via vfx-icc) |

```rust
// vfx-lut-formats/src/lib.rs
pub use cube::parse_cube;
pub use clf::parse_clf;
pub use ctf::parse_ctf;
// ...

/// Auto-detect and parse any LUT format
pub fn parse_lut(path: &Path) -> Result<Lut, LutError> {
    let ext = path.extension().and_then(|s| s.to_str());
    let data = std::fs::read_to_string(path)?;
    
    match ext {
        Some("cube") => Ok(Lut::Lut3D(parse_cube(&data)?)),
        Some("clf") => Ok(Lut::ProcessList(parse_clf(&data)?)),
        Some("3dl") => detect_3dl_variant(&data),  // Lustre vs Nuke
        Some("spi1d") => Ok(Lut::Lut1D(parse_spi1d(&data)?)),
        Some("spi3d") => Ok(Lut::Lut3D(parse_spi3d(&data)?)),
        _ => Err(LutError::UnknownFormat),
    }
}
```

#### vfx-transfer

**Purpose:** Все transfer functions (OETF/EOTF) для камер и дисплеев

```rust
// vfx-transfer/src/lib.rs

pub trait TransferFunction: Send + Sync {
    /// To linear (EOTF)
    fn to_linear(&self, v: f32) -> f32;
    /// From linear (OETF)
    fn from_linear(&self, v: f32) -> f32;
    /// Name
    fn name(&self) -> &'static str;
    /// SIMD batch (default impl, can be overridden)
    fn to_linear_simd(&self, data: &mut [f32]) {
        data.iter_mut().for_each(|v| *v = self.to_linear(*v));
    }
}

// Camera log curves
pub struct ArriLogC3 { pub ei: f32 }  // EI 800, 1600, 3200...
pub struct ArriLogC4;
pub struct RedLog3G10;
pub struct SLog2;
pub struct SLog3;
pub struct CanonLog;
pub struct CanonLog2;
pub struct CanonLog3;
pub struct VLog;  // Panasonic
pub struct BMDFilmGen5;
pub struct FLog;  // Fuji
pub struct NLog;  // Nikon

// Display curves  
pub struct Srgb;
pub struct Gamma { pub gamma: f32 }
pub struct Bt1886 { pub gamma: f32 }  // Rec.709/2020
pub struct Pq;  // HDR ST.2084
pub struct Hlg;  // HDR BBC/NHK

// Scene-referred log
pub struct CineonLog { pub black: f32, pub white: f32, pub gamma: f32 }
pub struct AcesCct;
pub struct AcesCc;
```

#### vfx-aces

**Purpose:** Полная реализация ACES pipeline

```rust
// vfx-aces/src/lib.rs

/// ACES version
pub const ACES_VERSION: &str = "1.3";

/// Input Device Transform (camera to ACES)
pub trait Idt: Send + Sync {
    fn camera_name(&self) -> &str;
    fn to_aces(&self, camera_rgb: [f32; 3]) -> [f32; 3];
}

/// Reference Rendering Transform
pub struct Rrt {
    pub version: RrtVersion,
}

pub enum RrtVersion {
    V1_0,  // ACES 1.0
    V1_1,  // ACES 1.1 (sweeteners)
    V1_2,  // ACES 1.2
}

impl Rrt {
    pub fn apply(&self, aces: [f32; 3]) -> [f32; 3] {
        // 1. Glow module
        // 2. Red modifier
        // 3. Global desaturation
        // 4. ACES to RGB rendering space
        // 5. Global tone scale (RRT tone curve)
        // 6. RGB rendering space to OCES
        todo!()
    }
}

/// Output Device Transform (OCES to display)
pub trait Odt: Send + Sync {
    fn display_name(&self) -> &str;
    fn peak_luminance(&self) -> f32;  // nits
    fn from_oces(&self, oces: [f32; 3]) -> [f32; 3];
}

/// Pre-built ODTs
pub mod odt {
    pub struct Srgb100Nits;
    pub struct Rec709D60sim100Nits;
    pub struct Rec709D65100Nits;
    pub struct Rec2020100Nits;
    pub struct Rec2020Pq1000Nits;
    pub struct Rec2020Pq2000Nits;
    pub struct Rec2020Pq4000Nits;
    pub struct Rec2020Hlg1000Nits;
    pub struct P3D60;
    pub struct P3D65;
    pub struct P3Dci;
}

/// Combined Output Transform (RRT + ODT)
pub struct OutputTransform {
    pub rrt: Rrt,
    pub odt: Box<dyn Odt>,
}

impl OutputTransform {
    /// ACES -> Display
    pub fn apply(&self, aces: [f32; 3]) -> [f32; 3] {
        let oces = self.rrt.apply(aces);
        self.odt.from_oces(oces)
    }
}
```

#### vfx-dpx

**Purpose:** Полная поддержка DPX 2.0 (SMPTE 268M-2014)

```rust
// vfx-dpx/src/lib.rs

/// DPX Descriptor (image type)
#[derive(Debug, Clone, Copy)]
pub enum Descriptor {
    Luminance = 6,
    ChrominanceCbCr = 7,
    Rgb = 50,
    Rgba = 51,
    Abgr = 52,
    CbYCrY = 100,  // 4:2:2
    CbYACrYA = 101,
    CbYCr = 102,   // 4:4:4
    CbYCrA = 103,
}

/// Bit depth and packing
#[derive(Debug, Clone, Copy)]
pub enum BitDepth {
    Bit8,
    Bit10Packed,    // Method A: 3 pixels in 4 bytes
    Bit10FilledA,   // Method A filled: 1 pixel per 32-bit
    Bit10FilledB,   // Method B filled
    Bit12Packed,
    Bit12Filled,
    Bit16,
}

/// Transfer characteristic
#[derive(Debug, Clone, Copy)]
pub enum Transfer {
    UserDefined = 0,
    PrintingDensity = 1,
    Linear = 2,
    Logarithmic = 3,
    UnspecifiedVideo = 4,
    Smpte274m = 5,  // Rec.709
    Bt709 = 6,
    Bt601_625 = 7,
    Bt601_525 = 8,
    CompositeNtsc = 9,
    CompositePal = 10,
    ZLinear = 11,
    ZHomogeneous = 12,
}

pub struct DpxReader {
    header: DpxHeader,
    file: File,
}

impl DpxReader {
    pub fn open(path: &Path) -> Result<Self, DpxError>;
    pub fn spec(&self) -> &ImageSpec;
    pub fn read(&mut self) -> Result<DynImage, DpxError>;
    pub fn read_scanlines(&mut self, y: u32, count: u32) -> Result<Vec<u8>, DpxError>;
}

pub struct DpxWriter { ... }
```

---

## Part 2: Existing Rust Crates to Use

### 2.1 MUST USE - Production Ready

| Crate | Purpose | Link | Notes |
|-------|---------|------|-------|
| `exr` | OpenEXR read/write | https://github.com/johannesvollmer/exrs | Pure Rust, полная поддержка EXR 2.0 |
| `image` | PNG/JPEG/TIFF/etc | https://github.com/image-rs/image | Де-факто стандарт |
| `half` | f16 type | https://github.com/starkat99/half-rs | IEEE 754 half-precision |
| `rayon` | Parallelism | https://github.com/rayon-rs/rayon | Data parallelism |
| `glam` | Math (fast) | https://github.com/bitshifter/glam-rs | SIMD vectors/matrices |
| `nalgebra` | Math (complete) | https://github.com/dimforge/nalgebra | Full linear algebra |
| `serde` + `serde_yaml` | Config parsing | https://github.com/dtolnay/serde | OCIO config format |
| `wgpu` | GPU compute | https://github.com/gfx-rs/wgpu | Cross-platform GPU |
| `parking_lot` | Fast mutexes | https://github.com/Amanieu/parking_lot | For cache |

### 2.2 SHOULD USE - Good Quality

| Crate | Purpose | Link | Notes |
|-------|---------|------|-------|
| `wide` | Portable SIMD | https://github.com/Lokathor/wide | Stable Rust SIMD |
| `simdeez` | Multi-arch SIMD | https://github.com/arduano/simdeez | SSE/AVX/NEON |
| `fast_image_resize` | SIMD resize | https://github.com/Cykooz/fast_image_resize | Production ready |
| `rawkit` | RAW decode | https://github.com/GraphiteEditor/Graphite/tree/master/libraries/rawkit | Camera RAW |
| `qcms` | ICC profiles | https://github.com/nicholascross/qcms-rust | Mozilla's ICC |
| `jpeg-decoder` | JPEG decode | https://github.com/image-rs/jpeg-decoder | Fast |
| `png` | PNG codec | https://github.com/image-rs/image-png | Complete |
| `tiff` | TIFF codec | https://github.com/image-rs/image-tiff | Good |
| `zune-image` | Fast codecs | https://github.com/etemesi254/zune-image | SIMD optimized |

### 2.3 CONSIDER - Partial Use

| Crate | Purpose | Link | Notes |
|-------|---------|------|-------|
| `palette` | Color spaces | https://github.com/Ogeon/palette | Хорошая база, но нет ACES |
| `colorgrad` | Gradients | https://github.com/mazznoer/colorgrad-rs | Для ramps |
| `lutgen-rs` | LUT generation | https://github.com/ozwaldorf/lutgen-rs | Можно взять код |
| `gluten` | LUT parsing | https://github.com/cszach/gluten | WIP, но идеи хорошие |

### 2.4 NOT AVAILABLE - Must Implement

| Component | Complexity | Priority |
|-----------|------------|----------|
| ACES transforms | HIGH | P0 |
| CLF/CTF parser | MEDIUM | P0 |
| .cube parser | LOW | P1 |
| 3DL parser | LOW | P2 |
| CDL (ASC-CDL) | MEDIUM | P1 |
| ICC v4 full | HIGH | P2 |
| DPX codec | MEDIUM | P1 |
| Cineon codec | MEDIUM | P2 |
| GPU LUT apply | MEDIUM | P1 |

---

## Part 3: Core Types Design

### 3.1 Pixel Types

```rust
// vfx-core/src/pixel.rs

use half::f16;

/// Marker traits for color spaces (compile-time safety)
pub trait ColorSpace: Copy + Clone + Send + Sync + 'static {
    const NAME: &'static str;
    const IS_LINEAR: bool;
    const WHITE_POINT: (f32, f32);  // CIE xy
    const PRIMARIES: [(f32, f32); 3];  // RGB xy
}

/// Linear scene-referred (ACES working space)
#[derive(Copy, Clone, Debug)]
pub struct AcesCg;
impl ColorSpace for AcesCg {
    const NAME: &'static str = "ACEScg";
    const IS_LINEAR: bool = true;
    const WHITE_POINT: (f32, f32) = (0.32168, 0.33767);  // ACES white
    const PRIMARIES: [(f32, f32); 3] = [
        (0.713, 0.293),   // Red
        (0.165, 0.830),   // Green
        (0.128, 0.044),   // Blue
    ];
}

/// sRGB display space
#[derive(Copy, Clone, Debug)]
pub struct Srgb;
impl ColorSpace for Srgb {
    const NAME: &'static str = "sRGB";
    const IS_LINEAR: bool = false;
    const WHITE_POINT: (f32, f32) = (0.3127, 0.3290);  // D65
    const PRIMARIES: [(f32, f32); 3] = [
        (0.64, 0.33),
        (0.30, 0.60),
        (0.15, 0.06),
    ];
}

/// Linear sRGB (same primaries, no transfer function)
#[derive(Copy, Clone, Debug)]
pub struct LinearSrgb;

/// Rec.709
#[derive(Copy, Clone, Debug)]
pub struct Rec709;

/// Rec.2020 / BT.2020
#[derive(Copy, Clone, Debug)]
pub struct Rec2020;

/// DCI-P3
#[derive(Copy, Clone, Debug)]
pub struct DciP3;

/// ACES2065-1 (full ACES archival)
#[derive(Copy, Clone, Debug)]
pub struct Aces2065;

/// Generic pixel with compile-time color space
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Rgba<C: ColorSpace, T> {
    pub r: T,
    pub g: T,
    pub b: T,
    pub a: T,
    _colorspace: std::marker::PhantomData<C>,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Rgb<C: ColorSpace, T> {
    pub r: T,
    pub g: T,
    pub b: T,
    _colorspace: std::marker::PhantomData<C>,
}

/// Trait for pixel data types
pub trait PixelData: Copy + Clone + Send + Sync + 'static {
    const BITS: u32;
    const IS_FLOAT: bool;
    fn to_f32(self) -> f32;
    fn from_f32(v: f32) -> Self;
}

impl PixelData for u8 {
    const BITS: u32 = 8;
    const IS_FLOAT: bool = false;
    fn to_f32(self) -> f32 { self as f32 / 255.0 }
    fn from_f32(v: f32) -> Self { (v.clamp(0.0, 1.0) * 255.0) as u8 }
}

impl PixelData for u16 {
    const BITS: u32 = 16;
    const IS_FLOAT: bool = false;
    fn to_f32(self) -> f32 { self as f32 / 65535.0 }
    fn from_f32(v: f32) -> Self { (v.clamp(0.0, 1.0) * 65535.0) as u16 }
}

impl PixelData for f16 {
    const BITS: u32 = 16;
    const IS_FLOAT: bool = true;
    fn to_f32(self) -> f32 { self.to_f32() }
    fn from_f32(v: f32) -> Self { f16::from_f32(v) }
}

impl PixelData for f32 {
    const BITS: u32 = 32;
    const IS_FLOAT: bool = true;
    fn to_f32(self) -> f32 { self }
    fn from_f32(v: f32) -> Self { v }
}
```

### 3.2 Image Buffer

```rust
// vfx-core/src/image.rs

use std::sync::Arc;

/// Image specification (metadata)
#[derive(Clone, Debug)]
pub struct ImageSpec {
    pub width: u32,
    pub height: u32,
    pub channels: u8,
    pub bit_depth: u8,
    pub is_float: bool,
    pub colorspace: String,
    pub metadata: HashMap<String, MetaValue>,
}

/// Zero-copy image buffer with color space tracking
pub struct Image<C: ColorSpace, T: PixelData> {
    data: Arc<[T]>,
    spec: ImageSpec,
    _colorspace: std::marker::PhantomData<C>,
}

impl<C: ColorSpace, T: PixelData> Image<C, T> {
    /// Create new image
    pub fn new(width: u32, height: u32, channels: u8) -> Self {
        let size = (width * height * channels as u32) as usize;
        let data: Arc<[T]> = vec![T::from_f32(0.0); size].into();
        Self {
            data,
            spec: ImageSpec {
                width,
                height,
                channels,
                bit_depth: T::BITS as u8,
                is_float: T::IS_FLOAT,
                colorspace: C::NAME.to_string(),
                metadata: HashMap::new(),
            },
            _colorspace: std::marker::PhantomData,
        }
    }

    /// Convert to different color space
    pub fn convert<C2: ColorSpace>(self) -> Image<C2, T> 
    where
        ColorTransform<C, C2>: Exists,
    {
        // Implementation uses color matrices
        todo!()
    }

    /// Change bit depth
    pub fn quantize<T2: PixelData>(self) -> Image<C, T2> {
        // Parallel pixel conversion
        todo!()
    }

    /// Parallel pixel iteration
    pub fn par_pixels_mut(&mut self) -> impl ParallelIterator<Item = &mut [T]> {
        // Using rayon
        todo!()
    }
}

/// View into image (no ownership)
pub struct ImageView<'a, C: ColorSpace, T: PixelData> {
    data: &'a [T],
    stride: usize,
    width: u32,
    height: u32,
    _colorspace: std::marker::PhantomData<C>,
}

/// Mutable view
pub struct ImageViewMut<'a, C: ColorSpace, T: PixelData> {
    data: &'a mut [T],
    stride: usize,
    width: u32,
    height: u32,
    _colorspace: std::marker::PhantomData<C>,
}
```

### 3.3 Color Transforms

```rust
// vfx-color/src/transform.rs

use glam::Mat3;

/// Color transformation matrix
#[derive(Clone, Debug)]
pub struct ColorMatrix {
    pub matrix: Mat3,
    pub offset: [f32; 3],
}

impl ColorMatrix {
    /// sRGB to XYZ D65
    pub const SRGB_TO_XYZ: Self = Self {
        matrix: Mat3::from_cols_array(&[
            0.4124564, 0.3575761, 0.1804375,
            0.2126729, 0.7151522, 0.0721750,
            0.0193339, 0.1191920, 0.9503041,
        ]),
        offset: [0.0; 3],
    };

    /// ACEScg to XYZ
    pub const ACESCG_TO_XYZ: Self = Self {
        matrix: Mat3::from_cols_array(&[
            0.6624542, 0.1340042, 0.1561877,
            0.2722287, 0.6740818, 0.0536895,
            -0.0055746, 0.0040607, 1.0103391,
        ]),
        offset: [0.0; 3],
    };

    /// Combine two transforms
    pub fn then(&self, other: &Self) -> Self {
        Self {
            matrix: other.matrix * self.matrix,
            offset: [
                other.offset[0] + self.offset[0],
                other.offset[1] + self.offset[1],
                other.offset[2] + self.offset[2],
            ],
        }
    }

    /// Apply to RGB values (SIMD optimized)
    #[inline]
    pub fn apply(&self, rgb: [f32; 3]) -> [f32; 3] {
        let v = glam::Vec3::from(rgb);
        let result = self.matrix * v;
        [
            result.x + self.offset[0],
            result.y + self.offset[1],
            result.z + self.offset[2],
        ]
    }
}

/// Transfer functions (OETF/EOTF)
pub trait TransferFunction: Send + Sync {
    fn to_linear(&self, v: f32) -> f32;
    fn from_linear(&self, v: f32) -> f32;
    
    /// SIMD batch version
    fn to_linear_batch(&self, data: &mut [f32]) {
        for v in data.iter_mut() {
            *v = self.to_linear(*v);
        }
    }
    
    fn from_linear_batch(&self, data: &mut [f32]) {
        for v in data.iter_mut() {
            *v = self.from_linear(*v);
        }
    }
}

/// sRGB transfer function
pub struct SrgbTransfer;

impl TransferFunction for SrgbTransfer {
    #[inline]
    fn to_linear(&self, v: f32) -> f32 {
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    }

    #[inline]
    fn from_linear(&self, v: f32) -> f32 {
        if v <= 0.0031308 {
            v * 12.92
        } else {
            1.055 * v.powf(1.0 / 2.4) - 0.055
        }
    }
}

/// Rec.709 / Rec.2020 transfer (BT.1886)
pub struct Bt1886Transfer {
    pub gamma: f32,  // typically 2.4
}

/// PQ (Perceptual Quantizer) for HDR
pub struct PqTransfer;

impl TransferFunction for PqTransfer {
    fn to_linear(&self, v: f32) -> f32 {
        const M1: f32 = 0.1593017578125;
        const M2: f32 = 78.84375;
        const C1: f32 = 0.8359375;
        const C2: f32 = 18.8515625;
        const C3: f32 = 18.6875;

        let vp = v.powf(1.0 / M2);
        let num = (vp - C1).max(0.0);
        let den = C2 - C3 * vp;
        10000.0 * (num / den).powf(1.0 / M1)
    }

    fn from_linear(&self, v: f32) -> f32 {
        const M1: f32 = 0.1593017578125;
        const M2: f32 = 78.84375;
        const C1: f32 = 0.8359375;
        const C2: f32 = 18.8515625;
        const C3: f32 = 18.6875;

        let y = (v / 10000.0).max(0.0);
        let yp = y.powf(M1);
        let num = C1 + C2 * yp;
        let den = 1.0 + C3 * yp;
        (num / den).powf(M2)
    }
}

/// HLG (Hybrid Log-Gamma) for HDR broadcast
pub struct HlgTransfer;
```

---

## Part 4: LUT System

### 4.1 LUT Types

```rust
// vfx-color/src/lut.rs

/// 1D LUT
#[derive(Clone, Debug)]
pub struct Lut1D {
    pub size: usize,
    pub data: Vec<f32>,       // R channel
    pub data_g: Option<Vec<f32>>,  // G (if different)
    pub data_b: Option<Vec<f32>>,  // B (if different)
    pub input_range: (f32, f32),
    pub output_range: (f32, f32),
}

impl Lut1D {
    /// Linear interpolation lookup
    #[inline]
    pub fn lookup(&self, v: f32) -> f32 {
        let normalized = (v - self.input_range.0) 
            / (self.input_range.1 - self.input_range.0);
        let idx = normalized * (self.size - 1) as f32;
        let idx_floor = idx.floor() as usize;
        let idx_ceil = (idx_floor + 1).min(self.size - 1);
        let t = idx.fract();
        
        self.data[idx_floor] * (1.0 - t) + self.data[idx_ceil] * t
    }
}

/// 3D LUT
#[derive(Clone, Debug)]
pub struct Lut3D {
    pub size: usize,  // cube size (e.g., 33 for 33x33x33)
    pub data: Vec<[f32; 3]>,  // RGB triplets
    pub input_range: (f32, f32),
    pub output_range: (f32, f32),
}

impl Lut3D {
    /// Trilinear interpolation
    pub fn lookup(&self, rgb: [f32; 3]) -> [f32; 3] {
        let scale = (self.size - 1) as f32;
        let r = ((rgb[0] - self.input_range.0) 
            / (self.input_range.1 - self.input_range.0) * scale).clamp(0.0, scale);
        let g = ((rgb[1] - self.input_range.0) 
            / (self.input_range.1 - self.input_range.0) * scale).clamp(0.0, scale);
        let b = ((rgb[2] - self.input_range.0) 
            / (self.input_range.1 - self.input_range.0) * scale).clamp(0.0, scale);

        let r0 = r.floor() as usize;
        let g0 = g.floor() as usize;
        let b0 = b.floor() as usize;
        let r1 = (r0 + 1).min(self.size - 1);
        let g1 = (g0 + 1).min(self.size - 1);
        let b1 = (b0 + 1).min(self.size - 1);

        let tr = r.fract();
        let tg = g.fract();
        let tb = b.fract();

        // Trilinear interpolation
        let idx = |r, g, b| r + g * self.size + b * self.size * self.size;
        
        let c000 = self.data[idx(r0, g0, b0)];
        let c100 = self.data[idx(r1, g0, b0)];
        let c010 = self.data[idx(r0, g1, b0)];
        let c110 = self.data[idx(r1, g1, b0)];
        let c001 = self.data[idx(r0, g0, b1)];
        let c101 = self.data[idx(r1, g0, b1)];
        let c011 = self.data[idx(r0, g1, b1)];
        let c111 = self.data[idx(r1, g1, b1)];

        let lerp = |a: f32, b: f32, t: f32| a * (1.0 - t) + b * t;
        let lerp3 = |a: [f32; 3], b: [f32; 3], t: f32| {
            [lerp(a[0], b[0], t), lerp(a[1], b[1], t), lerp(a[2], b[2], t)]
        };

        let c00 = lerp3(c000, c100, tr);
        let c01 = lerp3(c001, c101, tr);
        let c10 = lerp3(c010, c110, tr);
        let c11 = lerp3(c011, c111, tr);

        let c0 = lerp3(c00, c10, tg);
        let c1 = lerp3(c01, c11, tg);

        lerp3(c0, c1, tb)
    }

    /// Tetrahedral interpolation (higher quality)
    pub fn lookup_tetrahedral(&self, rgb: [f32; 3]) -> [f32; 3] {
        // More accurate but slightly slower
        todo!()
    }
}
```

### 4.2 LUT Formats

```rust
// vfx-color/src/lut/formats.rs

/// Supported LUT formats
pub enum LutFormat {
    Cube,       // .cube (Resolve, Adobe)
    Clf,        // .clf (ACES Common LUT Format)
    Ctf,        // .ctf (OCIO v2)
    Csp,        // .csp (Cinespace)
    Spi1d,      // .spi1d (Sony Pictures Imageworks)
    Spi3d,      // .spi3d
    Lut3dl,     // .3dl (Lustre, Flame)
    Icc,        // .icc/.icm (ICC profiles)
}

/// Parse .cube file
/// Reference: https://github.com/ozwaldorf/lutgen-rs/blob/main/src/lib.rs
pub fn parse_cube(input: &str) -> Result<Lut3D, LutError> {
    let mut size = 0usize;
    let mut data = Vec::new();
    let mut domain_min = [0.0f32; 3];
    let mut domain_max = [1.0f32; 3];

    for line in input.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if line.starts_with("LUT_3D_SIZE") {
            size = line.split_whitespace()
                .nth(1)
                .ok_or(LutError::Parse("Missing size"))?
                .parse()?;
            data.reserve(size * size * size);
        } else if line.starts_with("DOMAIN_MIN") {
            let parts: Vec<f32> = line.split_whitespace()
                .skip(1)
                .map(|s| s.parse())
                .collect::<Result<_, _>>()?;
            domain_min = [parts[0], parts[1], parts[2]];
        } else if line.starts_with("DOMAIN_MAX") {
            let parts: Vec<f32> = line.split_whitespace()
                .skip(1)
                .map(|s| s.parse())
                .collect::<Result<_, _>>()?;
            domain_max = [parts[0], parts[1], parts[2]];
        } else if !line.starts_with("TITLE") && !line.starts_with("LUT_1D") {
            let parts: Vec<f32> = line.split_whitespace()
                .map(|s| s.parse())
                .collect::<Result<_, _>>()?;
            if parts.len() == 3 {
                data.push([parts[0], parts[1], parts[2]]);
            }
        }
    }

    Ok(Lut3D {
        size,
        data,
        input_range: (domain_min[0], domain_max[0]),
        output_range: (0.0, 1.0),
    })
}

/// Parse CLF (Common LUT Format) - ACES standard
/// Spec: https://docs.acescentral.com/clf/specification/
pub fn parse_clf(xml: &str) -> Result<ProcessList, LutError> {
    // CLF is XML-based with multiple process nodes
    todo!()
}

/// CLF Process Node types
pub enum ClfNode {
    Matrix(ColorMatrix),
    Lut1D(Lut1D),
    Lut3D(Lut3D),
    Range { in_min: f32, in_max: f32, out_min: f32, out_max: f32 },
    Log { style: LogStyle, base: f32 },
    Exponent { style: ExpStyle, value: f32 },
    Asc { slope: [f32; 3], offset: [f32; 3], power: [f32; 3], sat: f32 },
}

/// Process list (chain of operations)
pub struct ProcessList {
    pub id: String,
    pub name: String,
    pub input_descriptor: String,
    pub output_descriptor: String,
    pub nodes: Vec<ClfNode>,
}
```

---

## Part 5: ACES Implementation

### 5.1 ACES Color Spaces

```rust
// vfx-color/src/aces/spaces.rs

/// ACES 2065-1 (archival, full gamut)
/// Primaries at spectral locus, AP0
pub const ACES_2065_1_TO_XYZ: ColorMatrix = ColorMatrix {
    matrix: glam::Mat3::from_cols_array(&[
        0.9525523959, 0.0000000000, 0.0000936786,
        0.3439664498, 0.7281660966, -0.0721325464,
        0.0000000000, 0.0000000000, 1.0088251844,
    ]),
    offset: [0.0; 3],
};

/// ACEScg (CG working space)
/// AP1 primaries, linear
pub const ACESCG_TO_XYZ: ColorMatrix = ColorMatrix {
    matrix: glam::Mat3::from_cols_array(&[
        0.6624541811, 0.1340042065, 0.1561876870,
        0.2722287168, 0.6740817658, 0.0536895174,
        -0.0055746495, 0.0040607335, 1.0103391003,
    ]),
    offset: [0.0; 3],
};

/// ACEScct (color correction, log-like)
pub struct AcesCctTransfer;

impl TransferFunction for AcesCctTransfer {
    fn to_linear(&self, x: f32) -> f32 {
        const X_BREAK: f32 = 0.155251141552511;
        const A: f32 = 10.5402377416545;
        const B: f32 = 0.0729055341958355;

        if x <= X_BREAK {
            (x - B) / A
        } else {
            2.0f32.powf(x * 17.52 - 9.72)
        }
    }

    fn from_linear(&self, y: f32) -> f32 {
        const Y_BREAK: f32 = 0.0078125;
        const A: f32 = 10.5402377416545;
        const B: f32 = 0.0729055341958355;

        if y <= Y_BREAK {
            A * y + B
        } else {
            (y.log2() + 9.72) / 17.52
        }
    }
}

/// ACEScc (color correction, pure log)  
pub struct AcesCcTransfer;

impl TransferFunction for AcesCcTransfer {
    fn to_linear(&self, x: f32) -> f32 {
        if x < -0.3013698630 {
            (2.0f32.powf(x * 17.52 - 9.72) - 0.00004770608) * 2.0
        } else if x < (9.72 - 15.0) / 17.52 {
            2.0f32.powf(x * 17.52 - 9.72)
        } else {
            65504.0  // max half float
        }
    }

    fn from_linear(&self, y: f32) -> f32 {
        if y <= 0.0 {
            -0.3584474886  // log2(2^-16) * 17.52 + 9.72) / 17.52
        } else if y < 0.00003051757 {
            ((y / 2.0 + 0.00004770608).log2() + 9.72) / 17.52
        } else {
            (y.log2() + 9.72) / 17.52
        }
    }
}
```

### 5.2 ACES Output Transforms (RRT + ODT)

```rust
// vfx-color/src/aces/output_transform.rs

/// Reference Rendering Transform (RRT) parameters
pub struct RrtParams {
    pub red_modifier: RedModifier,
    pub glow: GlowParams,
    pub global_desat: f32,
}

/// Output Device Transform (ODT) targets
pub enum OdtTarget {
    Srgb100Nits,
    Rec709100Nits,
    Rec2020100Nits,
    Rec2020Pq1000Nits,
    Rec2020Hlg1000Nits,
    DciP348Nits,
    Custom { primaries: Primaries, max_nits: f32, transfer: Box<dyn TransferFunction> },
}

/// Complete ACES Output Transform
pub struct AcesOutputTransform {
    pub rrt: RrtParams,
    pub odt: OdtTarget,
}

impl AcesOutputTransform {
    /// Apply full RRT+ODT
    pub fn apply(&self, acescg: [f32; 3]) -> [f32; 3] {
        // 1. ACEScg to ACES2065-1
        let aces = self.acescg_to_aces2065(acescg);
        
        // 2. RRT
        let rrt_out = self.apply_rrt(aces);
        
        // 3. ODT
        self.apply_odt(rrt_out)
    }

    fn apply_rrt(&self, aces: [f32; 3]) -> [f32; 3] {
        // Glow module
        let glow = self.compute_glow(aces);
        let rgb = [
            aces[0] * glow,
            aces[1] * glow,
            aces[2] * glow,
        ];

        // Red modifier
        let rgb = self.red_modifier(rgb);

        // Global desaturation
        let rgb = self.global_desat(rgb);

        // RRT tonescale
        self.rrt_tonescale(rgb)
    }

    fn rrt_tonescale(&self, rgb: [f32; 3]) -> [f32; 3] {
        // S-curve tonemap
        rgb.map(|v| {
            const A: f32 = 0.0245786;
            const B: f32 = 0.000090537;
            const C: f32 = 0.983729;
            const D: f32 = 0.4329510;
            const E: f32 = 0.238081;
            
            (v * (A * v + B)) / (v * (C * v + D) + E)
        })
    }
}
```

### 5.3 ACES Config Loading

```rust
// vfx-color/src/aces/config.rs

/// ACES config (similar to OCIO config but simplified)
#[derive(Debug, Clone, Deserialize)]
pub struct AcesConfig {
    pub version: String,
    pub colorspaces: Vec<ColorSpaceDef>,
    pub displays: Vec<DisplayDef>,
    pub views: Vec<ViewDef>,
    pub looks: Vec<LookDef>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ColorSpaceDef {
    pub name: String,
    pub family: Option<String>,
    pub description: Option<String>,
    pub encoding: Encoding,
    pub to_reference: Option<Transform>,
    pub from_reference: Option<Transform>,
}

#[derive(Debug, Clone, Deserialize)]
pub enum Transform {
    Matrix { matrix: [f32; 9], offset: Option<[f32; 3]> },
    FileTransform { src: PathBuf },
    LogTransform { base: f32, style: LogStyle },
    ExponentTransform { value: f32 },
    GroupTransform { children: Vec<Transform> },
    Builtin { style: String },
}

impl AcesConfig {
    /// Load from YAML file
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path)?;
        Ok(serde_yaml::from_str(&content)?)
    }

    /// Get transform from one space to another
    pub fn get_transform(&self, from: &str, to: &str) -> Result<Box<dyn ColorOp>, ConfigError> {
        // Build transform chain through reference space
        todo!()
    }
}
```

---

## Part 6: Image I/O

### 6.1 Format Registry

```rust
// vfx-io/src/registry.rs

use std::collections::HashMap;

/// Format capabilities
#[derive(Debug, Clone)]
pub struct FormatCaps {
    pub can_read: bool,
    pub can_write: bool,
    pub supports_tiles: bool,
    pub supports_mipmap: bool,
    pub supports_multipart: bool,
    pub supports_deep: bool,
    pub max_channels: u32,
    pub supported_types: Vec<PixelType>,
}

/// Format handler trait
pub trait ImageFormat: Send + Sync {
    fn name(&self) -> &str;
    fn extensions(&self) -> &[&str];
    fn magic_bytes(&self) -> Option<&[u8]>;
    fn capabilities(&self) -> FormatCaps;
    
    fn read(&self, path: &Path) -> Result<DynImage, IoError>;
    fn write(&self, path: &Path, image: &DynImage) -> Result<(), IoError>;
    
    fn read_header(&self, path: &Path) -> Result<ImageSpec, IoError>;
    fn read_region(&self, path: &Path, region: Rect) -> Result<DynImage, IoError> {
        // Default: read full and crop
        let full = self.read(path)?;
        Ok(full.crop(region))
    }
}

/// Global format registry
pub struct FormatRegistry {
    formats: HashMap<String, Box<dyn ImageFormat>>,
    by_extension: HashMap<String, String>,
}

impl FormatRegistry {
    pub fn new() -> Self {
        let mut reg = Self {
            formats: HashMap::new(),
            by_extension: HashMap::new(),
        };
        
        // Register built-in formats
        reg.register(Box::new(ExrFormat::new()));
        reg.register(Box::new(PngFormat::new()));
        reg.register(Box::new(JpegFormat::new()));
        reg.register(Box::new(TiffFormat::new()));
        reg.register(Box::new(DpxFormat::new()));
        
        reg
    }

    pub fn register(&mut self, format: Box<dyn ImageFormat>) {
        let name = format.name().to_string();
        for ext in format.extensions() {
            self.by_extension.insert(ext.to_string(), name.clone());
        }
        self.formats.insert(name, format);
    }

    pub fn read(&self, path: &Path) -> Result<DynImage, IoError> {
        let ext = path.extension()
            .and_then(|s| s.to_str())
            .ok_or(IoError::UnknownFormat)?;
        
        let format_name = self.by_extension.get(ext)
            .ok_or(IoError::UnsupportedFormat(ext.to_string()))?;
        
        let format = self.formats.get(format_name).unwrap();
        format.read(path)
    }
}

/// Thread-local cached registry
thread_local! {
    static REGISTRY: std::cell::RefCell<FormatRegistry> = 
        std::cell::RefCell::new(FormatRegistry::new());
}

/// Convenience function
pub fn read(path: impl AsRef<Path>) -> Result<DynImage, IoError> {
    REGISTRY.with(|r| r.borrow().read(path.as_ref()))
}
```

### 6.2 EXR Format (using exr crate)

```rust
// vfx-io/src/formats/exr.rs

use exr::prelude::*;

pub struct ExrFormat;

impl ImageFormat for ExrFormat {
    fn name(&self) -> &str { "exr" }
    fn extensions(&self) -> &[&str] { &["exr"] }
    fn magic_bytes(&self) -> Option<&[u8]> { Some(&[0x76, 0x2f, 0x31, 0x01]) }
    
    fn capabilities(&self) -> FormatCaps {
        FormatCaps {
            can_read: true,
            can_write: true,
            supports_tiles: true,
            supports_mipmap: true,
            supports_multipart: true,
            supports_deep: true,
            max_channels: 1024,
            supported_types: vec![PixelType::F16, PixelType::F32, PixelType::U32],
        }
    }

    fn read(&self, path: &Path) -> Result<DynImage, IoError> {
        // Using the exr crate: https://github.com/johannesvollmer/exrs
        let image = exr::prelude::read_all_data_from_file(path)
            .map_err(|e| IoError::Read(e.to_string()))?;

        // Convert to our Image type
        // Handle layers, channels, etc.
        todo!()
    }

    fn write(&self, path: &Path, image: &DynImage) -> Result<(), IoError> {
        // Convert our Image to exr format and write
        todo!()
    }
}
```

### 6.3 DPX Format (needs implementation)

```rust
// vfx-io/src/formats/dpx.rs

/// DPX file header (SMPTE 268M-2003)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DpxFileHeader {
    pub magic: u32,           // 0x53445058 ("SDPX") or 0x58504453 ("XPDS")
    pub offset: u32,          // Offset to image data
    pub version: [u8; 8],     // "V2.0" etc
    pub file_size: u32,
    pub ditto_key: u32,
    pub generic_header_size: u32,
    pub industry_header_size: u32,
    pub user_data_size: u32,
    pub filename: [u8; 100],
    pub create_time: [u8; 24],
    pub creator: [u8; 100],
    pub project: [u8; 200],
    pub copyright: [u8; 200],
    pub encrypt_key: u32,
    pub reserved: [u8; 104],
}

/// DPX image element
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DpxImageElement {
    pub data_sign: u32,
    pub low_data: u32,
    pub low_quantity: f32,
    pub high_data: u32,
    pub high_quantity: f32,
    pub descriptor: u8,
    pub transfer: u8,
    pub colorimetric: u8,
    pub bit_depth: u8,
    pub packing: u16,
    pub encoding: u16,
    pub data_offset: u32,
    pub eol_padding: u32,
    pub eoi_padding: u32,
    pub description: [u8; 32],
}

pub struct DpxFormat;

impl DpxFormat {
    /// Read 10-bit packed pixels
    fn read_10bit_packed(&self, data: &[u8], width: usize) -> Vec<u16> {
        // DPX 10-bit is packed as 3 pixels in 4 bytes (method A)
        // Or 1 pixel per 32-bit word with 2 padding bits
        todo!()
    }
}
```

---

## Part 7: Image Operations

### 7.1 Core Operations

```rust
// vfx-ops/src/lib.rs

use rayon::prelude::*;

/// Resize algorithms
#[derive(Debug, Clone, Copy)]
pub enum ResizeFilter {
    Nearest,
    Bilinear,
    Bicubic,
    Lanczos2,
    Lanczos3,
    Mitchell,
    CatmullRom,
}

/// Resize image
/// Uses fast_image_resize crate: https://github.com/Cykooz/fast_image_resize
pub fn resize<C: ColorSpace, T: PixelData>(
    src: &Image<C, T>,
    new_width: u32,
    new_height: u32,
    filter: ResizeFilter,
) -> Image<C, T> {
    // Use SIMD-optimized resize
    todo!()
}

/// Crop image (zero-copy view)
pub fn crop<C: ColorSpace, T: PixelData>(
    src: &Image<C, T>,
    x: u32, y: u32,
    width: u32, height: u32,
) -> ImageView<'_, C, T> {
    ImageView::new(src, x, y, width, height)
}

/// Flip/rotate operations
pub fn flip_horizontal<C: ColorSpace, T: PixelData>(src: &mut Image<C, T>) {
    let width = src.width() as usize;
    let height = src.height() as usize;
    let channels = src.channels() as usize;
    
    src.rows_mut().par_bridge().for_each(|row| {
        for x in 0..width / 2 {
            let left = x * channels;
            let right = (width - 1 - x) * channels;
            for c in 0..channels {
                row.swap(left + c, right + c);
            }
        }
    });
}

/// Premultiply alpha
pub fn premultiply<C: ColorSpace, T: PixelData>(src: &mut Image<C, T>) {
    if src.channels() != 4 { return; }
    
    src.pixels_mut().par_bridge().for_each(|pixel| {
        let a = pixel[3].to_f32();
        pixel[0] = T::from_f32(pixel[0].to_f32() * a);
        pixel[1] = T::from_f32(pixel[1].to_f32() * a);
        pixel[2] = T::from_f32(pixel[2].to_f32() * a);
    });
}

/// Unpremultiply alpha
pub fn unpremultiply<C: ColorSpace, T: PixelData>(src: &mut Image<C, T>) {
    if src.channels() != 4 { return; }
    
    src.pixels_mut().par_bridge().for_each(|pixel| {
        let a = pixel[3].to_f32();
        if a > 1e-6 {
            let inv_a = 1.0 / a;
            pixel[0] = T::from_f32(pixel[0].to_f32() * inv_a);
            pixel[1] = T::from_f32(pixel[1].to_f32() * inv_a);
            pixel[2] = T::from_f32(pixel[2].to_f32() * inv_a);
        }
    });
}
```

### 7.2 Color Grading Operations

```rust
// vfx-ops/src/grading.rs

/// ASC-CDL (Color Decision List)
/// Reference: https://en.wikipedia.org/wiki/ASC_CDL
#[derive(Debug, Clone)]
pub struct Cdl {
    pub slope: [f32; 3],   // Multiply
    pub offset: [f32; 3],  // Add
    pub power: [f32; 3],   // Gamma
    pub saturation: f32,
}

impl Cdl {
    pub const IDENTITY: Self = Self {
        slope: [1.0, 1.0, 1.0],
        offset: [0.0, 0.0, 0.0],
        power: [1.0, 1.0, 1.0],
        saturation: 1.0,
    };

    /// Apply CDL: out = (in * slope + offset) ^ power
    #[inline]
    pub fn apply(&self, rgb: [f32; 3]) -> [f32; 3] {
        let r = ((rgb[0] * self.slope[0] + self.offset[0]).max(0.0)).powf(self.power[0]);
        let g = ((rgb[1] * self.slope[1] + self.offset[1]).max(0.0)).powf(self.power[1]);
        let b = ((rgb[2] * self.slope[2] + self.offset[2]).max(0.0)).powf(self.power[2]);

        // Saturation
        let luma = 0.2126 * r + 0.7152 * g + 0.0722 * b;
        [
            luma + self.saturation * (r - luma),
            luma + self.saturation * (g - luma),
            luma + self.saturation * (b - luma),
        ]
    }
}

/// Lift/Gamma/Gain (alternative to CDL)
#[derive(Debug, Clone)]
pub struct LiftGammaGain {
    pub lift: [f32; 3],    // Shadows
    pub gamma: [f32; 3],   // Midtones
    pub gain: [f32; 3],    // Highlights
}

impl LiftGammaGain {
    #[inline]
    pub fn apply(&self, rgb: [f32; 3]) -> [f32; 3] {
        rgb.iter().enumerate().map(|(i, &v)| {
            let lifted = v * (1.0 - self.lift[i]) + self.lift[i];
            let gained = lifted * self.gain[i];
            gained.powf(1.0 / self.gamma[i])
        }).collect::<Vec<_>>().try_into().unwrap()
    }
}

/// Exposure adjustment (EV stops)
pub fn exposure(rgb: [f32; 3], ev: f32) -> [f32; 3] {
    let factor = 2.0f32.powf(ev);
    [rgb[0] * factor, rgb[1] * factor, rgb[2] * factor]
}

/// Contrast adjustment
pub fn contrast(rgb: [f32; 3], contrast: f32, pivot: f32) -> [f32; 3] {
    rgb.map(|v| pivot + (v - pivot) * contrast)
}
```

### 7.3 Compositing Operations

```rust
// vfx-ops/src/composite.rs

/// Blend modes
#[derive(Debug, Clone, Copy)]
pub enum BlendMode {
    Over,
    Under,
    In,
    Out,
    Atop,
    Xor,
    Plus,
    Multiply,
    Screen,
    Overlay,
    Difference,
}

impl BlendMode {
    /// Porter-Duff compositing
    pub fn composite(&self, a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
        match self {
            BlendMode::Over => {
                // A over B: A + B*(1-Aa)
                let inv_a = 1.0 - a[3];
                [
                    a[0] + b[0] * inv_a,
                    a[1] + b[1] * inv_a,
                    a[2] + b[2] * inv_a,
                    a[3] + b[3] * inv_a,
                ]
            }
            BlendMode::Multiply => {
                [
                    a[0] * b[0],
                    a[1] * b[1],
                    a[2] * b[2],
                    a[3] * b[3],
                ]
            }
            BlendMode::Screen => {
                [
                    1.0 - (1.0 - a[0]) * (1.0 - b[0]),
                    1.0 - (1.0 - a[1]) * (1.0 - b[1]),
                    1.0 - (1.0 - a[2]) * (1.0 - b[2]),
                    1.0 - (1.0 - a[3]) * (1.0 - b[3]),
                ]
            }
            // ... other modes
            _ => todo!()
        }
    }
}
```

---

## Part 8: GPU Processing

### 8.1 WGPU Backend

```rust
// vfx-gpu/src/lib.rs

use wgpu::*;

/// GPU context for image processing
pub struct GpuContext {
    device: Device,
    queue: Queue,
    lut_pipeline: ComputePipeline,
    color_matrix_pipeline: ComputePipeline,
}

impl GpuContext {
    pub async fn new() -> Result<Self, GpuError> {
        let instance = Instance::new(InstanceDescriptor::default());
        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: PowerPreference::HighPerformance,
                ..Default::default()
            })
            .await
            .ok_or(GpuError::NoAdapter)?;

        let (device, queue) = adapter
            .request_device(&DeviceDescriptor::default(), None)
            .await?;

        // Create compute pipelines
        let lut_shader = device.create_shader_module(include_wgsl!("shaders/lut3d.wgsl"));
        let lut_pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: Some("LUT3D"),
            layout: None,
            module: &lut_shader,
            entry_point: Some("apply_lut3d"),
            compilation_options: Default::default(),
            cache: None,
        });

        Ok(Self {
            device,
            queue,
            lut_pipeline,
            color_matrix_pipeline: todo!(),
        })
    }

    /// Apply 3D LUT on GPU
    pub fn apply_lut3d(&self, image: &GpuImage, lut: &Lut3D) -> GpuImage {
        // Upload LUT to 3D texture
        // Run compute shader
        // Return result
        todo!()
    }
}

/// GPU-resident image
pub struct GpuImage {
    texture: Texture,
    view: TextureView,
    width: u32,
    height: u32,
    format: TextureFormat,
}

impl GpuImage {
    /// Upload from CPU image
    pub fn upload<C: ColorSpace>(ctx: &GpuContext, image: &Image<C, f32>) -> Self {
        todo!()
    }

    /// Download to CPU image
    pub fn download<C: ColorSpace>(&self, ctx: &GpuContext) -> Image<C, f32> {
        todo!()
    }
}
```

### 8.2 GPU Shaders

```wgsl
// vfx-gpu/src/shaders/lut3d.wgsl

@group(0) @binding(0) var input_tex: texture_2d<f32>;
@group(0) @binding(1) var output_tex: texture_storage_2d<rgba32float, write>;
@group(0) @binding(2) var lut_tex: texture_3d<f32>;
@group(0) @binding(3) var lut_sampler: sampler;

struct Params {
    lut_size: u32,
    input_range_min: f32,
    input_range_max: f32,
    _padding: u32,
}

@group(0) @binding(4) var<uniform> params: Params;

@compute @workgroup_size(16, 16)
fn apply_lut3d(@builtin(global_invocation_id) id: vec3<u32>) {
    let dims = textureDimensions(input_tex);
    if (id.x >= dims.x || id.y >= dims.y) {
        return;
    }

    let pixel = textureLoad(input_tex, vec2<i32>(id.xy), 0);
    
    // Normalize to LUT range
    let range = params.input_range_max - params.input_range_min;
    let normalized = (pixel.rgb - params.input_range_min) / range;
    
    // Sample LUT with trilinear interpolation
    let lut_coord = clamp(normalized, vec3<f32>(0.0), vec3<f32>(1.0));
    let lut_color = textureSampleLevel(lut_tex, lut_sampler, lut_coord, 0.0).rgb;
    
    textureStore(output_tex, vec2<i32>(id.xy), vec4<f32>(lut_color, pixel.a));
}
```

```wgsl
// vfx-gpu/src/shaders/color_matrix.wgsl

@group(0) @binding(0) var input_tex: texture_2d<f32>;
@group(0) @binding(1) var output_tex: texture_storage_2d<rgba32float, write>;

struct ColorMatrix {
    matrix: mat3x3<f32>,
    offset: vec3<f32>,
    _padding: f32,
}

@group(0) @binding(2) var<uniform> transform: ColorMatrix;

@compute @workgroup_size(16, 16)
fn apply_matrix(@builtin(global_invocation_id) id: vec3<u32>) {
    let dims = textureDimensions(input_tex);
    if (id.x >= dims.x || id.y >= dims.y) {
        return;
    }

    let pixel = textureLoad(input_tex, vec2<i32>(id.xy), 0);
    let rgb = transform.matrix * pixel.rgb + transform.offset;
    
    textureStore(output_tex, vec2<i32>(id.xy), vec4<f32>(rgb, pixel.a));
}
```

---

## Part 9: Image Cache & Texture System

### 9.1 Tile-based Cache

```rust
// vfx-cache/src/lib.rs

use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

/// Cache configuration
pub struct CacheConfig {
    pub max_memory_mb: usize,
    pub tile_size: u32,
    pub max_open_files: usize,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_memory_mb: 1024,  // 1GB
            tile_size: 64,
            max_open_files: 100,
        }
    }
}

/// Tile key
#[derive(Hash, Eq, PartialEq, Clone, Debug)]
struct TileKey {
    file_id: u64,
    mip_level: u8,
    tile_x: u32,
    tile_y: u32,
}

/// Cached tile data
struct CachedTile {
    data: Vec<u8>,
    last_access: std::time::Instant,
    size_bytes: usize,
}

/// Image cache (similar to OIIO ImageCache)
pub struct ImageCache {
    config: CacheConfig,
    tiles: RwLock<HashMap<TileKey, Arc<CachedTile>>>,
    current_memory: std::sync::atomic::AtomicUsize,
    file_handles: RwLock<lru::LruCache<u64, Box<dyn ImageReader>>>,
}

impl ImageCache {
    pub fn new(config: CacheConfig) -> Self {
        Self {
            config,
            tiles: RwLock::new(HashMap::new()),
            current_memory: std::sync::atomic::AtomicUsize::new(0),
            file_handles: RwLock::new(lru::LruCache::new(
                std::num::NonZeroUsize::new(100).unwrap()
            )),
        }
    }

    /// Get tile (load if not cached)
    pub fn get_tile(&self, file: &Path, mip: u8, tx: u32, ty: u32) -> Result<Arc<CachedTile>, CacheError> {
        let file_id = self.file_id(file);
        let key = TileKey { file_id, mip_level: mip, tile_x: tx, tile_y: ty };

        // Check cache
        if let Some(tile) = self.tiles.read().get(&key) {
            return Ok(Arc::clone(tile));
        }

        // Load tile
        let tile = self.load_tile(file, mip, tx, ty)?;
        let tile = Arc::new(tile);

        // Maybe evict old tiles
        self.maybe_evict(tile.size_bytes);

        // Insert
        self.tiles.write().insert(key, Arc::clone(&tile));
        self.current_memory.fetch_add(tile.size_bytes, std::sync::atomic::Ordering::Relaxed);

        Ok(tile)
    }

    fn maybe_evict(&self, needed: usize) {
        let max = self.config.max_memory_mb * 1024 * 1024;
        let current = self.current_memory.load(std::sync::atomic::Ordering::Relaxed);

        if current + needed <= max {
            return;
        }

        // LRU eviction
        let mut tiles = self.tiles.write();
        let mut to_remove = Vec::new();
        let mut freed = 0usize;

        // Find oldest tiles
        let mut entries: Vec<_> = tiles.iter()
            .map(|(k, v)| (k.clone(), v.last_access, v.size_bytes))
            .collect();
        entries.sort_by_key(|(_, t, _)| *t);

        for (key, _, size) in entries {
            if freed >= needed {
                break;
            }
            to_remove.push(key);
            freed += size;
        }

        for key in to_remove {
            if let Some(tile) = tiles.remove(&key) {
                self.current_memory.fetch_sub(tile.size_bytes, std::sync::atomic::Ordering::Relaxed);
            }
        }
    }

    fn file_id(&self, path: &Path) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        path.hash(&mut hasher);
        hasher.finish()
    }

    fn load_tile(&self, file: &Path, mip: u8, tx: u32, ty: u32) -> Result<CachedTile, CacheError> {
        todo!()
    }
}

/// Texture system (filtered texture lookup)
pub struct TextureSystem {
    cache: Arc<ImageCache>,
}

impl TextureSystem {
    pub fn new(cache: Arc<ImageCache>) -> Self {
        Self { cache }
    }

    /// Sample texture with filtering
    pub fn sample(&self, file: &Path, uv: (f32, f32), filter_width: f32) -> [f32; 4] {
        // Determine mip level from filter width
        // Sample and filter tiles
        todo!()
    }
}
```

---

## Part 10: Implementation Plan (Revised)

### Overview: 6-Month Roadmap

```
Month 1-2: Foundation Layer
├── vfx-core      ████████████████ DONE
├── vfx-math      ████████████████ DONE  
├── vfx-lut       ████████████████ DONE
├── vfx-transfer  ████████████████ DONE
└── vfx-primaries ████████████████ DONE

Month 2-3: Color Pipeline
├── vfx-lut-formats ████████████████ DONE
├── vfx-icc         ████████░░░░░░░░ 50%
├── vfx-aces        ████████████████ DONE
└── vfx-config      ████████████████ DONE

Month 3-4: Image I/O
├── vfx-exr    ████████████████ DONE
├── vfx-dpx    ████████████████ DONE
├── vfx-io     ████████████████ DONE
└── vfx-color  ████████████████ DONE

Month 4-5: Operations
├── vfx-ops     ████████████████ DONE
├── vfx-grading ████████████████ DONE
└── vfx-cache   ████████████████ DONE

Month 5-6: GPU & Polish
├── vfx-gpu  ████████████████ DONE
├── vfx      ████████████████ DONE
├── tools    ████████████████ DONE
└── bindings ████████░░░░░░░░ 50%
```

---

### Phase 1: Foundation (Weeks 1-4)

**Goal:** Core types and basic EXR I/O

| Task | Priority | Effort | Dependencies |
|------|----------|--------|--------------|
| Set up workspace with all crates | P0 | 2d | - |
| Implement core pixel types | P0 | 3d | - |
| Implement Image<C, T> buffer | P0 | 5d | Pixel types |
| Integrate `exr` crate | P0 | 3d | Image buffer |
| Basic PNG/JPEG via `image` crate | P1 | 2d | Image buffer |
| Color space marker traits | P0 | 2d | - |
| ColorMatrix implementation | P0 | 2d | - |
| sRGB/Linear transfer functions | P0 | 2d | ColorMatrix |

**Deliverable:** Read EXR -> convert color space -> write PNG

### Phase 2: Color Pipeline (Weeks 5-8)

**Goal:** ACES and LUT support

| Task | Priority | Effort | Dependencies |
|------|----------|--------|--------------|
| ACES color spaces (all) | P0 | 5d | ColorMatrix |
| ACEScct/ACEScc transfers | P0 | 3d | TransferFunction |
| .cube parser | P0 | 2d | - |
| Lut3D trilinear lookup | P0 | 3d | Cube parser |
| CLF/CTF parser (basic) | P1 | 5d | - |
| CDL implementation | P1 | 2d | - |
| RRT tonemap (simplified) | P1 | 5d | ACES spaces |
| Config file loading (YAML) | P1 | 3d | All spaces |

**Deliverable:** Load OCIO-style config, apply ACES pipeline

### Phase 3: Image Operations (Weeks 9-12)

**Goal:** Essential image processing

| Task | Priority | Effort | Dependencies |
|------|----------|--------|--------------|
| Integrate `fast_image_resize` | P0 | 2d | Image buffer |
| Parallel pixel iteration (rayon) | P0 | 2d | Image buffer |
| Crop/ROI views | P0 | 2d | Image buffer |
| Flip/rotate | P1 | 1d | - |
| Premultiply/unpremultiply | P0 | 1d | - |
| Porter-Duff compositing | P1 | 3d | - |
| Exposure/contrast/saturation | P1 | 2d | - |
| Lift/Gamma/Gain | P1 | 2d | - |

**Deliverable:** Full color correction pipeline

### Phase 4: More Formats (Weeks 13-16)

**Goal:** DPX and additional formats

| Task | Priority | Effort | Dependencies |
|------|----------|--------|--------------|
| DPX reader | P0 | 5d | - |
| DPX writer | P1 | 3d | DPX reader |
| TIFF improvements (deep color) | P1 | 3d | - |
| Format registry | P0 | 2d | All formats |
| ICC profile support (qcms) | P2 | 5d | - |
| RAW support (rawkit) | P2 | 3d | - |

**Deliverable:** Production-ready file I/O

### Phase 5: GPU Acceleration (Weeks 17-20)

**Goal:** GPU compute pipeline

| Task | Priority | Effort | Dependencies |
|------|----------|--------|--------------|
| WGPU context setup | P0 | 3d | - |
| GPU image upload/download | P0 | 3d | WGPU context |
| LUT3D compute shader | P0 | 3d | LUT3D, GPU image |
| Color matrix shader | P0 | 2d | ColorMatrix |
| Transfer function shader | P1 | 2d | - |
| GPU resize | P1 | 3d | - |
| Batch processing | P1 | 3d | All GPU ops |

**Deliverable:** GPU-accelerated color pipeline

### Phase 6: Caching & Performance (Weeks 21-24)

**Goal:** Production performance

| Task | Priority | Effort | Dependencies |
|------|----------|--------|--------------|
| ImageCache implementation | P0 | 5d | File I/O |
| Tile-based reading | P0 | 5d | ImageCache |
| TextureSystem (filtered lookup) | P1 | 5d | ImageCache |
| SIMD optimization audit | P1 | 5d | All ops |
| Benchmarks | P0 | 3d | Everything |
| Memory profiling | P1 | 2d | - |

**Deliverable:** Production-ready performance

---

## Part 11: External Resources

### 11.1 Specifications

| Spec | URL | Notes |
|------|-----|-------|
| ACES | https://github.com/ampas/aces-dev | Official transforms |
| CLF | https://docs.acescentral.com/clf/specification/ | LUT format spec |
| OpenEXR | https://openexr.com/en/latest/TechnicalIntroduction.html | File format |
| DPX | https://ieeexplore.ieee.org/document/7290186 | SMPTE 268M |
| ICC | https://www.color.org/specification/ICC.1-2022-05.pdf | Color profiles |

### 11.2 Reference Implementations

| Project | URL | What to take |
|---------|-----|--------------|
| OpenColorIO | https://github.com/AcademySoftwareFoundation/OpenColorIO | Config format, ACES transforms |
| OpenImageIO | https://github.com/AcademySoftwareFoundation/OpenImageIO | DPX, cache design |
| colour-science | https://github.com/colour-science/colour | Python ACES reference |
| CTL | https://github.com/ampas/CTL | ACES transform language |
| rawtoaces | https://github.com/AcademySoftwareFoundation/rawtoaces | Camera IDT |

### 11.3 Rust Ecosystem

| Crate | Repository | Version |
|-------|------------|---------|
| exr | https://github.com/johannesvollmer/exrs | 1.72+ |
| image | https://github.com/image-rs/image | 0.25+ |
| half | https://github.com/starkat99/half-rs | 2.4+ |
| glam | https://github.com/bitshifter/glam-rs | 0.29+ |
| rayon | https://github.com/rayon-rs/rayon | 1.10+ |
| wgpu | https://github.com/gfx-rs/wgpu | 23+ |
| fast_image_resize | https://github.com/Cykooz/fast_image_resize | 5+ |
| simdeez | https://github.com/arduano/simdeez | 2+ |

---

## Part 12: Risk Assessment

### 12.1 Technical Risks

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| ACES transforms numerically different | Medium | High | Extensive testing against CTL |
| Performance worse than C++ | Low | High | SIMD audit, benchmarks |
| GPU compatibility issues | Medium | Medium | Test on multiple GPUs |
| LUT interpolation artifacts | Low | Medium | Use tetrahedral, test |

### 12.2 Scope Risks

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Feature creep | High | Medium | Strict MVP scope |
| Too many formats | Medium | Medium | Focus on EXR/DPX/PNG/JPEG |
| Complex GPU shaders | Medium | Medium | Start simple, iterate |

---

## Part 13: Success Criteria

### 13.1 MVP (Month 3)

- [ ] Read/write EXR, PNG, JPEG
- [ ] ACES color spaces (all variants)
- [ ] 3D LUT apply (CPU)
- [ ] Basic resize, crop, composite
- [ ] Config file loading

### 13.2 Production Ready (Month 6)

- [ ] DPX read/write
- [ ] Full ACES output transforms
- [ ] GPU acceleration
- [ ] ImageCache with tiling
- [ ] Performance parity with OIIO

### 13.3 Metrics

| Metric | Target |
|--------|--------|
| EXR read speed | >= OIIO |
| LUT apply speed | >= OCIO |
| Memory efficiency | < 1.5x OIIO |
| Color accuracy | < 1e-5 delta vs CTL |

---

## Appendix A: Quick Start Code

```rust
use vfx::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load config
    let config = AcesConfig::load("aces_config.yaml")?;

    // Read EXR in ACEScg
    let img: Image<AcesCg, f32> = vfx::read("input.exr")?;

    // Apply CDL
    let cdl = Cdl {
        slope: [1.1, 1.0, 0.9],
        offset: [0.0, 0.0, 0.02],
        power: [1.0, 1.0, 1.0],
        saturation: 1.1,
    };
    let graded = img.apply(|rgb| cdl.apply(rgb));

    // Apply LUT
    let lut = Lut3D::load("grade.cube")?;
    let graded = graded.apply(|rgb| lut.lookup(rgb));

    // Convert to sRGB for display
    let display: Image<Srgb, u8> = graded
        .convert::<Srgb>()
        .quantize();

    // Write output
    vfx::write("output.png", &display)?;

    Ok(())
}
```

---

## Appendix B: Comparison with Existing Solutions

| Feature | VFX-RS | OCIO+OIIO | Notes |
|---------|--------|-----------|-------|
| Language | Rust | C++ | Memory safety |
| Color space compile check | Yes | No | Type system |
| Unified API | Yes | No | Two libraries |
| GPU | wgpu | OpenGL/Metal | Cross-platform |
| Config format | YAML | YAML | Compatible |
| LUT formats | CLF/cube/3dl | CLF/cube/3dl/etc | Parity |
| Build system | Cargo | CMake | Simpler |
| Dependencies | ~15 crates | 20+ libs | Fewer |

---

---

## Appendix C: Detailed Task Breakdown by Crate

### vfx-core (5 days)

| Task | Est | Priority |
|------|-----|----------|
| Pixel<C, T> generic struct | 0.5d | P0 |
| ColorSpace marker trait | 0.5d | P0 |
| Standard color spaces (Srgb, AcesCg, Rec709, etc) | 1d | P0 |
| Image<C, T> buffer with Arc storage | 1d | P0 |
| ImageView / ImageViewMut | 0.5d | P0 |
| ImageSpec (metadata) | 0.5d | P0 |
| Rect, ROI types | 0.5d | P1 |
| Error types | 0.5d | P0 |

### vfx-math (3 days)

| Task | Est | Priority |
|------|-----|----------|
| ColorMatrix (3x3 + offset) | 0.5d | P0 |
| Mat4x4 for homogeneous coords | 0.5d | P1 |
| Vec3/Vec4 wrappers over glam | 0.5d | P0 |
| Interpolation (lerp, cubic, catmull-rom) | 0.5d | P0 |
| SIMD batch helpers | 1d | P0 |

### vfx-lut (4 days)

| Task | Est | Priority |
|------|-----|----------|
| Lut1D struct | 0.5d | P0 |
| Lut1D linear interpolation | 0.5d | P0 |
| Lut3D struct | 0.5d | P0 |
| Lut3D trilinear interpolation | 1d | P0 |
| Lut3D tetrahedral interpolation | 1d | P1 |
| Hald CLUT support | 0.5d | P2 |

### vfx-lut-formats (8 days)

| Task | Est | Priority |
|------|-----|----------|
| .cube parser | 1d | P0 |
| .cube writer | 0.5d | P1 |
| .clf parser (XML, complex) | 2d | P0 |
| .ctf parser | 1d | P1 |
| .spi1d / .spi3d parser | 0.5d | P1 |
| .3dl parser (Lustre variant) | 0.5d | P1 |
| .3dl parser (Nuke variant) | 0.5d | P1 |
| .csp parser | 0.5d | P2 |
| Auto-detection | 0.5d | P0 |
| Unit tests with real LUTs | 1d | P0 |

### vfx-transfer (5 days)

| Task | Est | Priority |
|------|-----|----------|
| TransferFunction trait | 0.5d | P0 |
| sRGB | 0.25d | P0 |
| Pure gamma | 0.25d | P0 |
| BT.1886 (Rec.709) | 0.25d | P0 |
| PQ (ST.2084) | 0.5d | P0 |
| HLG | 0.5d | P1 |
| Cineon log | 0.5d | P0 |
| ARRI LogC3/LogC4 | 0.5d | P0 |
| RED Log3G10 | 0.25d | P1 |
| Sony S-Log2/S-Log3 | 0.5d | P1 |
| Canon Log 2/3 | 0.25d | P2 |
| Panasonic V-Log | 0.25d | P2 |
| BMD Film Gen 5 | 0.25d | P2 |
| ACEScct / ACEScc | 0.5d | P0 |
| SIMD batch implementations | 0.5d | P1 |

### vfx-primaries (3 days)

| Task | Est | Priority |
|------|-----|----------|
| Primaries struct (RGB xy) | 0.25d | P0 |
| WhitePoint struct (xy) | 0.25d | P0 |
| Standard primaries (sRGB, Rec709, Rec2020, P3, ACES AP0/AP1) | 0.5d | P0 |
| Standard white points (D50, D55, D60, D65, DCI) | 0.25d | P0 |
| RGB to XYZ matrix generation | 0.5d | P0 |
| XYZ to RGB matrix generation | 0.25d | P0 |
| Chromatic adaptation (Bradford) | 0.5d | P0 |
| Chromatic adaptation (Von Kries, CAT02) | 0.5d | P2 |

### vfx-aces (10 days)

| Task | Est | Priority |
|------|-----|----------|
| ACES color space definitions | 0.5d | P0 |
| ACES2065-1 <-> ACEScg matrices | 0.5d | P0 |
| RRT v1.0 core | 2d | P0 |
| RRT sweeteners (v1.1/1.2) | 1d | P1 |
| ODT: sRGB 100 nits | 0.5d | P0 |
| ODT: Rec.709 100 nits | 0.5d | P0 |
| ODT: Rec.2020 SDR | 0.5d | P1 |
| ODT: Rec.2020 PQ 1000 nits | 1d | P1 |
| ODT: P3 variants | 0.5d | P1 |
| IDT: Generic sRGB | 0.5d | P0 |
| IDT: ARRI | 0.5d | P1 |
| IDT: RED | 0.5d | P2 |
| LMT framework | 1d | P2 |
| Validation against CTL reference | 1d | P0 |

### vfx-config (5 days)

| Task | Est | Priority |
|------|-----|----------|
| YAML schema definition | 0.5d | P0 |
| Config struct with serde | 1d | P0 |
| ColorSpace definition parsing | 0.5d | P0 |
| Display/View parsing | 0.5d | P0 |
| Look parsing | 0.5d | P1 |
| Transform chain building | 1d | P0 |
| Built-in transforms registry | 0.5d | P0 |
| Config validation | 0.5d | P1 |

### vfx-icc (6 days)

| Task | Est | Priority |
|------|-----|----------|
| ICC v2 header parsing | 1d | P1 |
| ICC v4 header parsing | 0.5d | P2 |
| Tag parsing (TRC, XYZ, etc) | 1.5d | P1 |
| Profile struct | 0.5d | P1 |
| Profile to ColorMatrix | 1d | P1 |
| Parametric curve support | 1d | P2 |
| Embedded profile extraction | 0.5d | P1 |

### vfx-dpx (7 days)

| Task | Est | Priority |
|------|-----|----------|
| DPX header structs (SMPTE 268M) | 1d | P0 |
| Byte order handling (BE/LE) | 0.5d | P0 |
| 8-bit read/write | 0.5d | P0 |
| 10-bit packed read | 1d | P0 |
| 10-bit packed write | 0.5d | P1 |
| 10-bit filled read/write | 0.5d | P1 |
| 12-bit read/write | 0.5d | P1 |
| 16-bit read/write | 0.5d | P0 |
| Metadata (timecode, keycode) | 0.5d | P1 |
| Cineon format support | 1d | P2 |
| Unit tests with real files | 0.5d | P0 |

### vfx-exr (4 days)

| Task | Est | Priority |
|------|-----|----------|
| Wrapper over `exr` crate | 0.5d | P0 |
| Read to Image<C, f16/f32> | 1d | P0 |
| Write from Image<C, f16/f32> | 1d | P0 |
| Multi-part support | 0.5d | P1 |
| Channel mapping | 0.5d | P0 |
| Deep image support | 0.5d | P2 |

### vfx-io (4 days)

| Task | Est | Priority |
|------|-----|----------|
| ImageFormat trait | 0.5d | P0 |
| FormatRegistry | 0.5d | P0 |
| Magic bytes detection | 0.5d | P0 |
| EXR integration | 0.5d | P0 |
| DPX integration | 0.5d | P0 |
| PNG/JPEG/TIFF via image crate | 0.5d | P0 |
| Unified read/write API | 0.5d | P0 |
| RAW support via rawkit | 0.5d | P2 |

### vfx-ops (6 days)

| Task | Est | Priority |
|------|-----|----------|
| Resize via fast_image_resize | 1d | P0 |
| Crop (zero-copy view) | 0.5d | P0 |
| Flip horizontal/vertical | 0.25d | P1 |
| Rotate 90/180/270 | 0.5d | P1 |
| Premultiply/unpremultiply | 0.5d | P0 |
| Porter-Duff compositing | 1d | P1 |
| Blend modes (multiply, screen, overlay...) | 0.5d | P1 |
| Gaussian blur | 0.5d | P2 |
| Unsharp mask | 0.5d | P2 |
| Parallel execution helpers | 0.75d | P0 |

### vfx-grading (4 days)

| Task | Est | Priority |
|------|-----|----------|
| ASC-CDL implementation | 0.5d | P0 |
| Lift/Gamma/Gain | 0.5d | P0 |
| Exposure (EV stops) | 0.25d | P0 |
| Contrast | 0.25d | P0 |
| Saturation | 0.25d | P0 |
| White balance | 0.5d | P1 |
| RGB curves | 0.5d | P1 |
| Luma curve | 0.5d | P1 |
| HSV adjustments | 0.5d | P2 |
| Printer lights | 0.25d | P2 |

### vfx-cache (5 days)

| Task | Est | Priority |
|------|-----|----------|
| CacheConfig | 0.25d | P0 |
| TileKey / CachedTile | 0.5d | P0 |
| ImageCache struct | 1d | P0 |
| LRU eviction | 0.5d | P0 |
| Tile loading | 1d | P0 |
| File handle cache | 0.5d | P1 |
| TextureSystem (filtered lookup) | 1d | P1 |
| Cache statistics | 0.25d | P1 |

### vfx-gpu (8 days)

| Task | Est | Priority |
|------|-----|----------|
| WGPU context setup | 1d | P0 |
| GpuImage upload/download | 1d | P0 |
| LUT3D compute shader | 1d | P0 |
| Color matrix shader | 0.5d | P0 |
| Transfer function shaders | 0.5d | P1 |
| Resize shader | 1d | P1 |
| Composite shader | 0.5d | P2 |
| Batch processing | 1d | P1 |
| Fallback to CPU | 0.5d | P0 |
| Multi-GPU support | 1d | P3 |

### vfx (main crate) (2 days)

| Task | Est | Priority |
|------|-----|----------|
| Re-export all crates | 0.5d | P0 |
| Prelude module | 0.5d | P0 |
| Integration tests | 0.5d | P0 |
| Documentation | 0.5d | P0 |

### tools (4 days)

| Task | Est | Priority |
|------|-----|----------|
| vfxconvert CLI | 1.5d | P1 |
| vfxinfo CLI | 1d | P1 |
| vfxlut CLI | 1d | P2 |
| Shell completions | 0.5d | P2 |

### bindings (8 days)

| Task | Est | Priority |
|------|-----|----------|
| vfx-c: C header generation | 2d | P2 |
| vfx-c: Core types | 1d | P2 |
| vfx-c: I/O functions | 1d | P2 |
| vfx-py: PyO3 setup | 1d | P2 |
| vfx-py: Image class | 1d | P2 |
| vfx-py: Color transforms | 1d | P2 |
| vfx-py: NumPy integration | 1d | P2 |

---

## Appendix D: Reference Implementations to Study

### ACES CTL Reference

```
https://github.com/ampas/aces-dev/tree/main/transforms/ctl
├── idt/         # Input Device Transforms
├── lib/         # Utility functions
├── lmt/         # Look Modification Transforms  
├── odt/         # Output Device Transforms
├── rrt/         # Reference Rendering Transform
└── utilities/   # Color space conversions
```

**Key files to port:**
- `rrt/RRT.ctl` - Main RRT implementation
- `odt/sRGB/ODT.Academy.sRGB_100nits_dim.ctl` - sRGB ODT
- `lib/ACESlib.Tonescales.ctl` - Tone curve math
- `lib/ACESlib.Transform_Common.ctl` - Common transforms

### OCIO Source Reference

```
https://github.com/AcademySoftwareFoundation/OpenColorIO/tree/main/src/OpenColorIO
├── ops/           # Transform operations
│   ├── lut1d/
│   ├── lut3d/
│   ├── matrix/
│   ├── log/
│   └── gamma/
├── fileformats/   # LUT file parsers
│   ├── FileFormatCLF.cpp
│   ├── FileFormatCTF.cpp
│   └── FileFormat*.cpp
└── transforms/    # Transform types
```

**Key files to study:**
- `ops/lut3d/Lut3DOp.cpp` - 3D LUT interpolation
- `fileformats/FileFormatCLF.cpp` - CLF parser
- `builtinconfigs/` - Built-in ACES configs

### OIIO Source Reference

```
https://github.com/AcademySoftwareFoundation/OpenImageIO/tree/main/src
├── libOpenImageIO/
│   ├── imagecache.cpp      # ImageCache implementation
│   ├── imagebufalgo_*.cpp  # Image operations
│   └── formatspec.cpp      # ImageSpec
├── dpx.imageio/            # DPX format plugin
├── cineon.imageio/         # Cineon format plugin
└── openexr.imageio/        # EXR format plugin
```

**Key files to study:**
- `dpx.imageio/dpxinput.cpp` - DPX reader
- `libOpenImageIO/imagecache.cpp` - Cache architecture
- `libOpenImageIO/imagebufalgo_compare.cpp` - Image comparison

---

## Appendix E: Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_srgb_roundtrip() {
        let transfer = Srgb;
        for v in [0.0, 0.1, 0.5, 0.9, 1.0] {
            let linear = transfer.to_linear(v);
            let back = transfer.from_linear(linear);
            assert_relative_eq!(v, back, epsilon = 1e-6);
        }
    }

    #[test]
    fn test_color_matrix_identity() {
        let identity = ColorMatrix::IDENTITY;
        let rgb = [0.5, 0.3, 0.7];
        let result = identity.apply(rgb);
        assert_eq!(rgb, result);
    }

    #[test]
    fn test_lut3d_corners() {
        // LUT should return exact values at lattice points
        let lut = Lut3D::identity(17);
        assert_eq!(lut.lookup([0.0, 0.0, 0.0]), [0.0, 0.0, 0.0]);
        assert_eq!(lut.lookup([1.0, 1.0, 1.0]), [1.0, 1.0, 1.0]);
    }
}
```

### Integration Tests

```rust
// tests/integration/aces_pipeline.rs

#[test]
fn test_aces_srgb_roundtrip() {
    // Load reference image
    let input = vfx::read("tests/fixtures/marcie_srgb.png").unwrap();
    
    // Convert to ACEScg
    let acescg: Image<AcesCg, f32> = input.convert();
    
    // Apply look
    let graded = acescg.apply_cdl(&Cdl { 
        slope: [1.0, 1.0, 1.0],
        offset: [0.0, 0.0, 0.0],
        power: [1.0, 1.0, 1.0],
        saturation: 1.0,
    });
    
    // Output transform
    let output_transform = OutputTransform::new(
        Rrt::v1_2(),
        odt::Srgb100Nits::new(),
    );
    let display = graded.apply(&output_transform);
    
    // Compare with reference
    let reference = vfx::read("tests/fixtures/marcie_aces_srgb_ref.png").unwrap();
    assert!(display.compare(&reference).max_diff() < 1e-4);
}
```

### CTL Validation

```rust
// tests/ctl_validation/mod.rs

/// Validate our ACES implementation against CTL reference
#[test]
fn validate_rrt_against_ctl() {
    // Test vectors generated from official CTL
    let test_cases = [
        ([0.18, 0.18, 0.18], [0.179, 0.179, 0.179]),  // 18% gray
        ([1.0, 0.0, 0.0], [0.891, 0.098, 0.033]),     // Red
        ([0.0, 1.0, 0.0], [0.098, 0.891, 0.033]),     // Green
        // ... more test vectors
    ];
    
    let rrt = Rrt::v1_2();
    for (input, expected) in test_cases {
        let result = rrt.apply(input);
        for i in 0..3 {
            assert_relative_eq!(result[i], expected[i], epsilon = 1e-3);
        }
    }
}
```

---

*Document version: 1.1*
*Created: 2025-01*
*Updated: 2025-01*
*Author: VFX-RS Team*
