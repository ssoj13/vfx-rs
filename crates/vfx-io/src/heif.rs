//! HEIF/HEIC image format support with HDR metadata.
//!
//! Requires the `heif` feature and system libheif >= 1.17.
//!
//! # Setup
//!
//! **Windows (vcpkg):**
//! ```bash
//! vcpkg install libheif:x64-windows
//! set VCPKG_ROOT=C:\vcpkg
//! ```
//!
//! **Linux:**
//! ```bash
//! apt install libheif-dev   # Debian/Ubuntu
//! dnf install libheif-devel # Fedora
//! ```
//!
//! **macOS:**
//! ```bash
//! brew install libheif
//! ```
//!
//! # HDR Support
//!
//! HEIF supports multiple HDR formats:
//! - **HDR10 PQ**: Absolute luminance via SMPTE ST 2084 transfer
//! - **HLG**: Hybrid Log-Gamma for broadcast compatibility
//! - **Gain Map**: SDR base + gain map for adaptive HDR (iPhone)
//!
//! # Example
//!
//! ```ignore
//! use vfx_io::heif::{read_heif, HdrMetadata};
//!
//! let (image, hdr_meta) = read_heif("photo.heic")?;
//! if let Some(meta) = hdr_meta {
//!     println!("Transfer: {:?}", meta.transfer);
//!     println!("Primaries: {:?}", meta.primaries);
//! }
//! ```

use crate::{ImageData, IoError, IoResult};
use std::path::Path;

/// NCLX transfer characteristics (CICP).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u16)]
pub enum TransferCharacteristics {
    /// BT.709 (SDR video)
    #[default]
    Bt709 = 1,
    /// sRGB
    Srgb = 13,
    /// SMPTE ST 2084 (PQ) - HDR10
    Pq = 16,
    /// ARIB STD-B67 (HLG) - HDR broadcast
    Hlg = 18,
}

/// NCLX color primaries (CICP).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u16)]
pub enum ColorPrimaries {
    /// BT.709 / sRGB
    #[default]
    Bt709 = 1,
    /// BT.2020 (wide gamut HDR)
    Bt2020 = 9,
    /// DCI-P3
    DciP3 = 11,
    /// Display P3
    DisplayP3 = 12,
}

/// HDR metadata extracted from HEIF NCLX profile.
#[derive(Debug, Clone, Default)]
pub struct HdrMetadata {
    /// Transfer function (PQ, HLG, sRGB, etc.)
    pub transfer: TransferCharacteristics,
    /// Color primaries (BT.2020, P3, etc.)
    pub primaries: ColorPrimaries,
    /// Max Content Light Level (nits), if present
    pub max_cll: Option<u16>,
    /// Max Frame Average Light Level (nits), if present  
    pub max_fall: Option<u16>,
    /// Bit depth (8, 10, 12)
    pub bit_depth: u8,
}

/// Read HEIF/HEIC image with optional HDR metadata.
///
/// Returns the image data and HDR metadata if present.
#[cfg(feature = "heif")]
pub fn read_heif<P: AsRef<Path>>(path: P) -> IoResult<(ImageData, Option<HdrMetadata>)> {
    use libheif_rs::{HeifContext, LibHeif, ColorSpace, Channel};
    
    let path = path.as_ref();
    let lib = LibHeif::new();
    let ctx = HeifContext::read_from_file(path.to_str().ok_or_else(|| {
        IoError::ReadError(format!("Invalid path: {:?}", path))
    })?).map_err(|e| IoError::ReadError(format!("HEIF read error: {}", e)))?;
    
    let handle = ctx.primary_image_handle()
        .map_err(|e| IoError::ReadError(format!("HEIF handle error: {}", e)))?;
    
    let width = handle.width() as usize;
    let height = handle.height() as usize;
    let has_alpha = handle.has_alpha_channel();
    let bit_depth = handle.luma_bits_per_pixel() as u8;
    let channels = if has_alpha { 4 } else { 3 };
    
    // Extract HDR metadata from NCLX profile
    let hdr_meta = extract_hdr_metadata(&handle, bit_depth);
    
    // Decode to RGB(A)
    let color_space = if has_alpha { ColorSpace::Rgba(libheif_rs::RgbChroma::Rgba) } 
                      else { ColorSpace::Rgb(libheif_rs::RgbChroma::Rgb) };
    
    let image = lib.decode(&handle, color_space, None)
        .map_err(|e| IoError::ReadError(format!("HEIF decode error: {}", e)))?;
    
    // Get pixel data
    let plane = image.planes().interleaved
        .ok_or_else(|| IoError::ReadError("No interleaved plane".into()))?;
    
    let stride = plane.stride;
    let data = plane.data;
    
    // Convert to f32 normalized
    let max_val = ((1u32 << bit_depth) - 1) as f32;
    let mut pixels = Vec::with_capacity(width * height * channels);
    
    for y in 0..height {
        let row_start = y * stride;
        for x in 0..width {
            let px_start = row_start + x * channels;
            for c in 0..channels {
                let val = data[px_start + c] as f32 / max_val;
                pixels.push(val);
            }
        }
    }
    
    let image_data = ImageData::from_f32(
        pixels,
        width as u32,
        height as u32,
        channels as u32,
    )?;
    
    Ok((image_data, hdr_meta))
}

#[cfg(feature = "heif")]
fn extract_hdr_metadata(handle: &libheif_rs::ImageHandle, bit_depth: u8) -> Option<HdrMetadata> {
    // Try to get NCLX color profile
    let nclx = handle.color_profile_nclx().ok()?;
    
    let transfer = match nclx.transfer_characteristics() {
        libheif_rs::TransferCharacteristics::Smpte2084 => TransferCharacteristics::Pq,
        libheif_rs::TransferCharacteristics::HLG => TransferCharacteristics::Hlg,
        libheif_rs::TransferCharacteristics::Srgb => TransferCharacteristics::Srgb,
        _ => TransferCharacteristics::Bt709,
    };
    
    let primaries = match nclx.color_primaries() {
        libheif_rs::ColorPrimaries::BT2020 => ColorPrimaries::Bt2020,
        libheif_rs::ColorPrimaries::SmpteEG4321 => ColorPrimaries::DciP3,
        _ => ColorPrimaries::Bt709,
    };
    
    Some(HdrMetadata {
        transfer,
        primaries,
        max_cll: None,  // TODO: extract from metadata boxes
        max_fall: None,
        bit_depth,
    })
}

/// Placeholder when heif feature is disabled.
#[cfg(not(feature = "heif"))]
pub fn read_heif<P: AsRef<Path>>(_path: P) -> IoResult<(ImageData, Option<HdrMetadata>)> {
    Err(IoError::UnsupportedFormat("HEIF support requires 'heif' feature".into()))
}

/// Write image as HEIF/HEIC with optional HDR metadata.
#[cfg(feature = "heif")]
pub fn write_heif<P: AsRef<Path>>(
    _image: &ImageData,
    _path: P,
    _hdr: Option<&HdrMetadata>,
) -> IoResult<()> {
    // TODO: implement HEIF writing
    Err(IoError::WriteError("HEIF writing not yet implemented".into()))
}

/// Placeholder when heif feature is disabled.
#[cfg(not(feature = "heif"))]
pub fn write_heif<P: AsRef<Path>>(
    _image: &ImageData,
    _path: P,
    _hdr: Option<&HdrMetadata>,
) -> IoResult<()> {
    Err(IoError::UnsupportedFormat("HEIF support requires 'heif' feature".into()))
}
