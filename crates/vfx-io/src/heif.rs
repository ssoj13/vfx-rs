//! HEIF/HEIC image format support with HDR metadata.
//!
//! Provides reading and writing of HEIF/HEIC images including HDR content
//! with PQ (ST 2084) and HLG transfer functions.
//!
//! # Requirements
//!
//! Requires the `heif` feature and system libheif >= 1.17.
//!
//! ## Windows (vcpkg)
//!
//! ```bash
//! vcpkg install libheif:x64-windows
//! set VCPKG_ROOT=C:\vcpkg
//! set VCPKGRS_TRIPLET=x64-windows
//! set VCPKGRS_DYNAMIC=1
//! ```
//!
//! ## Linux
//!
//! ```bash
//! apt install libheif-dev   # Debian/Ubuntu
//! dnf install libheif-devel # Fedora
//! ```
//!
//! ## macOS
//!
//! ```bash
//! brew install libheif
//! ```
//!
//! # HDR Support
//!
//! HEIF supports multiple HDR formats via NCLX color profiles:
//!
//! - **HDR10 PQ**: Absolute luminance via SMPTE ST 2084 transfer (up to 10000 nits)
//! - **HLG**: Hybrid Log-Gamma for broadcast compatibility (relative luminance)
//! - **Gain Map**: SDR base + gain map for adaptive HDR (iPhone, future ISO 21496-1)
//!
//! # Example
//!
//! ```ignore
//! use vfx_io::heif::{read_heif, write_heif, HdrMetadata, TransferCharacteristics};
//!
//! // Read HEIC with HDR metadata
//! let (image, hdr_meta) = read_heif("photo.heic")?;
//! if let Some(meta) = &hdr_meta {
//!     println!("Transfer: {:?}, Primaries: {:?}", meta.transfer, meta.primaries);
//!     println!("Bit depth: {}", meta.bit_depth);
//! }
//!
//! // Write as HEIF (preserving HDR metadata if present)
//! write_heif("output.heif", &image, hdr_meta.as_ref())?;
//! ```

#[cfg(feature = "heif")]
use libheif_rs::{
    ColorSpace, HeifContext, LibHeif, RgbChroma,
    CompressionFormat, EncoderQuality, Image as HeifImage,
    ColorProfileNCLX,
};

use crate::{IoError, IoResult};
use std::path::Path;

use crate::ImageData;

#[cfg(feature = "heif")]
use crate::{PixelData, PixelFormat, Metadata};

/// NCLX transfer characteristics (CICP / ITU-T H.273).
///
/// Defines the electro-optical transfer function (EOTF) for the image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u16)]
pub enum TransferCharacteristics {
    /// BT.709 (SDR video, traditional gamma ~2.4)
    #[default]
    Bt709 = 1,
    /// Unspecified
    Unspecified = 2,
    /// BT.601 (legacy SD video)
    Bt601 = 6,
    /// Linear (no transfer function)
    Linear = 8,
    /// sRGB (~2.2 gamma with linear toe)
    Srgb = 13,
    /// BT.2020 10-bit (same as BT.709)
    Bt202010 = 14,
    /// BT.2020 12-bit
    Bt202012 = 15,
    /// SMPTE ST 2084 (PQ) - HDR10, absolute luminance
    Pq = 16,
    /// SMPTE ST 428-1 (DCI)
    St428 = 17,
    /// ARIB STD-B67 (HLG) - HDR broadcast
    Hlg = 18,
}

/// NCLX color primaries (CICP / ITU-T H.273).
///
/// Defines the color gamut of the image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u16)]
pub enum ColorPrimaries {
    /// BT.709 / sRGB (standard gamut)
    #[default]
    Bt709 = 1,
    /// Unspecified
    Unspecified = 2,
    /// BT.601 625-line (PAL)
    Bt601_625 = 5,
    /// BT.601 525-line (NTSC)
    Bt601_525 = 6,
    /// BT.2020 / BT.2100 (wide gamut HDR)
    Bt2020 = 9,
    /// DCI-P3 (cinema)
    DciP3 = 11,
    /// Display P3 (Apple)
    DisplayP3 = 12,
}

/// NCLX matrix coefficients for YCbCr conversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u16)]
pub enum MatrixCoefficients {
    /// Identity (RGB, no conversion)
    Identity = 0,
    /// BT.709
    #[default]
    Bt709 = 1,
    /// Unspecified
    Unspecified = 2,
    /// BT.601
    Bt601 = 6,
    /// BT.2020 non-constant luminance
    Bt2020Ncl = 9,
    /// BT.2020 constant luminance
    Bt2020Cl = 10,
    /// ICtCp (used with PQ)
    ICtCp = 14,
}

/// HDR metadata extracted from HEIF NCLX color profile.
///
/// Contains color space information needed for correct HDR display.
#[derive(Debug, Clone, Default)]
pub struct HdrMetadata {
    /// Transfer function (PQ, HLG, sRGB, etc.)
    pub transfer: TransferCharacteristics,
    /// Color primaries (BT.2020, P3, etc.)
    pub primaries: ColorPrimaries,
    /// Matrix coefficients for YCbCr
    pub matrix: MatrixCoefficients,
    /// Full range (0-255) vs limited range (16-235)
    pub full_range: bool,
    /// Max Content Light Level in nits (HDR10 metadata)
    pub max_cll: Option<u16>,
    /// Max Frame Average Light Level in nits (HDR10 metadata)
    pub max_fall: Option<u16>,
    /// Bit depth (8, 10, 12)
    pub bit_depth: u8,
}

impl HdrMetadata {
    /// Returns true if this is HDR content (PQ or HLG transfer).
    #[inline]
    pub fn is_hdr(&self) -> bool {
        matches!(self.transfer, TransferCharacteristics::Pq | TransferCharacteristics::Hlg)
    }

    /// Returns true if this uses wide color gamut (BT.2020 or P3).
    #[inline]
    pub fn is_wide_gamut(&self) -> bool {
        matches!(self.primaries, ColorPrimaries::Bt2020 | ColorPrimaries::DciP3 | ColorPrimaries::DisplayP3)
    }
}

/// Read HEIF/HEIC image with HDR metadata.
///
/// Returns the image data and HDR metadata if NCLX color profile is present.
///
/// # Arguments
///
/// * `path` - Path to HEIF/HEIC file
///
/// # Returns
///
/// Tuple of (ImageData, Option<HdrMetadata>). HDR metadata is Some if NCLX
/// color profile was found in the file.
///
/// # Example
///
/// ```ignore
/// let (image, hdr) = read_heif("photo.heic")?;
/// println!("Size: {}x{}", image.width, image.height);
/// if let Some(meta) = hdr {
///     if meta.is_hdr() {
///         println!("HDR image with {:?} transfer", meta.transfer);
///     }
/// }
/// ```
#[cfg(feature = "heif")]
pub fn read_heif<P: AsRef<Path>>(path: P) -> IoResult<(ImageData, Option<HdrMetadata>)> {
    let path = path.as_ref();
    let path_str = path.to_str().ok_or_else(|| {
        IoError::DecodeError(format!("Invalid path: {:?}", path))
    })?;

    let ctx = HeifContext::read_from_file(path_str)
        .map_err(|e| IoError::DecodeError(format!("HEIF read error: {}", e)))?;

    let handle = ctx.primary_image_handle()
        .map_err(|e| IoError::DecodeError(format!("HEIF handle error: {}", e)))?;

    let width = handle.width() as u32;
    let height = handle.height() as u32;
    let has_alpha = handle.has_alpha_channel();
    let bit_depth = handle.luma_bits_per_pixel() as u8;
    let channels = if has_alpha { 4u32 } else { 3u32 };

    // Extract HDR metadata from NCLX profile
    let hdr_meta = extract_nclx_metadata(&handle, bit_depth);

    // Decode to interleaved RGB(A)
    let lib = LibHeif::new();
    let chroma = if has_alpha { RgbChroma::Rgba } else { RgbChroma::Rgb };
    let color_space = ColorSpace::Rgb(chroma);

    let image = lib.decode(&handle, color_space, None)
        .map_err(|e| IoError::DecodeError(format!("HEIF decode error: {}", e)))?;

    // Get interleaved plane
    let plane = image.planes().interleaved
        .ok_or_else(|| IoError::DecodeError("No interleaved RGB plane".into()))?;

    let stride = plane.stride;
    let src_data = plane.data;
    let bytes_per_pixel = channels as usize * if bit_depth > 8 { 2 } else { 1 };

    // Convert to f32 normalized [0, 1]
    let max_val = ((1u32 << bit_depth) - 1) as f32;
    let pixel_count = (width * height) as usize;
    let mut pixels = Vec::with_capacity(pixel_count * channels as usize);

    if bit_depth <= 8 {
        // 8-bit path
        for y in 0..height as usize {
            let row_start = y * stride;
            for x in 0..width as usize {
                let px_start = row_start + x * channels as usize;
                for c in 0..channels as usize {
                    let val = src_data[px_start + c] as f32 / max_val;
                    pixels.push(val);
                }
            }
        }
    } else {
        // 10/12/16-bit path (stored as u16)
        for y in 0..height as usize {
            let row_start = y * stride;
            for x in 0..width as usize {
                let px_start = row_start + x * bytes_per_pixel;
                for c in 0..channels as usize {
                    let byte_offset = px_start + c * 2;
                    let val_u16 = u16::from_le_bytes([
                        src_data[byte_offset],
                        src_data[byte_offset + 1],
                    ]);
                    let val = val_u16 as f32 / max_val;
                    pixels.push(val);
                }
            }
        }
    }

    let mut metadata = Metadata::default();
    
    // Set colorspace hint based on HDR metadata
    if let Some(ref hdr) = hdr_meta {
        let cs_name = match (&hdr.primaries, &hdr.transfer) {
            (ColorPrimaries::Bt2020, TransferCharacteristics::Pq) => "Rec.2100-PQ",
            (ColorPrimaries::Bt2020, TransferCharacteristics::Hlg) => "Rec.2100-HLG",
            (ColorPrimaries::Bt2020, _) => "Rec.2020",
            (ColorPrimaries::DisplayP3, _) => "Display P3",
            (ColorPrimaries::DciP3, _) => "DCI-P3",
            (_, TransferCharacteristics::Srgb) => "sRGB",
            (_, TransferCharacteristics::Linear) => "Linear",
            _ => "Rec.709",
        };
        metadata.colorspace = Some(cs_name.to_string());
    }

    let image_data = ImageData {
        width,
        height,
        channels,
        format: PixelFormat::F32,
        data: PixelData::F32(pixels),
        metadata,
    };

    Ok((image_data, hdr_meta))
}

/// Extract NCLX color profile metadata from image handle.
#[cfg(feature = "heif")]
fn extract_nclx_metadata(handle: &libheif_rs::ImageHandle, bit_depth: u8) -> Option<HdrMetadata> {
    // color_profile_nclx() returns Option<ColorProfileNCLX>
    let nclx = handle.color_profile_nclx()?;

    // Map libheif-rs TransferCharacteristics to our enum
    let transfer = match nclx.transfer_characteristics() {
        libheif_rs::TransferCharacteristics::ITU_R_BT_2100_0_PQ => TransferCharacteristics::Pq,
        libheif_rs::TransferCharacteristics::ITU_R_BT_2100_0_HLG => TransferCharacteristics::Hlg,
        libheif_rs::TransferCharacteristics::IEC_61966_2_1 => TransferCharacteristics::Srgb,
        libheif_rs::TransferCharacteristics::Linear => TransferCharacteristics::Linear,
        libheif_rs::TransferCharacteristics::ITU_R_BT_709_5 => TransferCharacteristics::Bt709,
        libheif_rs::TransferCharacteristics::ITU_R_BT_601_6 => TransferCharacteristics::Bt601,
        libheif_rs::TransferCharacteristics::ITU_R_BT_2020_2_10bit => TransferCharacteristics::Bt202010,
        libheif_rs::TransferCharacteristics::ITU_R_BT_2020_2_12bit => TransferCharacteristics::Bt202012,
        _ => TransferCharacteristics::Unspecified,
    };

    // Map libheif-rs ColorPrimaries to our enum
    let primaries = match nclx.color_primaries() {
        libheif_rs::ColorPrimaries::ITU_R_BT_709_5 => ColorPrimaries::Bt709,
        libheif_rs::ColorPrimaries::ITU_R_BT_2020_2_and_2100_0 => ColorPrimaries::Bt2020,
        libheif_rs::ColorPrimaries::SMPTE_EG_432_1 => ColorPrimaries::DciP3,
        _ => ColorPrimaries::Unspecified,
    };

    // Map libheif-rs MatrixCoefficients to our enum
    let matrix = match nclx.matrix_coefficients() {
        libheif_rs::MatrixCoefficients::RGB_GBR => MatrixCoefficients::Identity,
        libheif_rs::MatrixCoefficients::ITU_R_BT_709_5 => MatrixCoefficients::Bt709,
        libheif_rs::MatrixCoefficients::ITU_R_BT_601_6 => MatrixCoefficients::Bt601,
        libheif_rs::MatrixCoefficients::ITU_R_BT_2020_2_NonConstantLuminance => MatrixCoefficients::Bt2020Ncl,
        libheif_rs::MatrixCoefficients::ITU_R_BT_2020_2_ConstantLuminance => MatrixCoefficients::Bt2020Cl,
        libheif_rs::MatrixCoefficients::ICtCp => MatrixCoefficients::ICtCp,
        _ => MatrixCoefficients::Unspecified,
    };

    Some(HdrMetadata {
        transfer,
        primaries,
        matrix,
        full_range: nclx.full_range_flag() != 0,  // u8 to bool
        // CLLI/MDCV extraction requires libheif-sys FFI calls:
        // - heif_image_handle_get_content_light_level() for max_cll/max_fall
        // - heif_image_handle_get_mastering_display_colour_volume() for display metadata
        // libheif-rs 2.x does not expose these functions in its safe wrapper.
        // See: https://github.com/Cykooz/libheif-rs/issues for feature requests.
        max_cll: None,
        max_fall: None,
        bit_depth,
    })
}

/// Write image as HEIF/HEIC with optional HDR metadata.
///
/// # Arguments
///
/// * `path` - Output file path (.heif or .heic)
/// * `image` - Image data to encode
/// * `hdr` - Optional HDR metadata for NCLX color profile
///
/// # Note
///
/// Due to libheif-rs API limitations, only color primaries from HDR metadata
/// are written to NCLX profile. Transfer characteristics and matrix coefficients
/// use library defaults. For full HDR metadata support, use ICC profiles.
///
/// # Example
///
/// ```ignore
/// use vfx_io::heif::{write_heif, HdrMetadata, TransferCharacteristics, ColorPrimaries};
///
/// // Write SDR image
/// write_heif("output.heif", &image, None)?;
///
/// // Write HDR PQ image
/// let hdr = HdrMetadata {
///     transfer: TransferCharacteristics::Pq,
///     primaries: ColorPrimaries::Bt2020,
///     bit_depth: 10,
///     ..Default::default()
/// };
/// write_heif("output_hdr.heif", &image, Some(&hdr))?;
/// ```
#[cfg(feature = "heif")]
pub fn write_heif<P: AsRef<Path>>(
    path: P,
    image: &ImageData,
    hdr: Option<&HdrMetadata>,
) -> IoResult<()> {
    let path = path.as_ref();
    let path_str = path.to_str().ok_or_else(|| {
        IoError::EncodeError(format!("Invalid path: {:?}", path))
    })?;

    let width = image.width;
    let height = image.height;
    let channels = image.channels;
    let has_alpha = channels == 4;

    // Determine output bit depth
    let bit_depth = hdr.map(|h| h.bit_depth).unwrap_or(8);
    let bit_depth = if bit_depth > 8 { 10 } else { 8 }; // libheif supports 8 or 10-bit

    // Create HEIF image with appropriate colorspace
    let colorspace = if bit_depth > 8 {
        libheif_rs::ColorSpace::Rgb(if has_alpha { RgbChroma::HdrRgbaLe } else { RgbChroma::HdrRgbLe })
    } else {
        libheif_rs::ColorSpace::Rgb(if has_alpha { RgbChroma::Rgba } else { RgbChroma::Rgb })
    };

    let mut heif_image = HeifImage::new(width, height, colorspace)
        .map_err(|e| IoError::EncodeError(format!("Failed to create HEIF image: {}", e)))?;

    // Add interleaved plane
    heif_image.create_plane(
        libheif_rs::Channel::Interleaved,
        width,
        height,
        bit_depth,
    ).map_err(|e| IoError::EncodeError(format!("Failed to create plane: {}", e)))?;

    // Get plane for writing
    let plane = heif_image.planes_mut().interleaved
        .ok_or_else(|| IoError::EncodeError("No interleaved plane".into()))?;

    let stride = plane.stride;
    let dst_data = plane.data;

    // Convert from f32 to output format
    let src_f32 = image.to_f32();
    let max_val = ((1u32 << bit_depth) - 1) as f32;

    if bit_depth <= 8 {
        // 8-bit output
        for y in 0..height as usize {
            let row_start = y * stride;
            for x in 0..width as usize {
                let src_idx = (y * width as usize + x) * channels as usize;
                let dst_idx = row_start + x * channels as usize;
                for c in 0..channels as usize {
                    let val = (src_f32[src_idx + c].clamp(0.0, 1.0) * max_val) as u8;
                    dst_data[dst_idx + c] = val;
                }
            }
        }
    } else {
        // 10-bit output (stored as u16 LE)
        for y in 0..height as usize {
            let row_start = y * stride;
            for x in 0..width as usize {
                let src_idx = (y * width as usize + x) * channels as usize;
                let dst_idx = row_start + x * channels as usize * 2;
                for c in 0..channels as usize {
                    let val = (src_f32[src_idx + c].clamp(0.0, 1.0) * max_val) as u16;
                    let bytes = val.to_le_bytes();
                    dst_data[dst_idx + c * 2] = bytes[0];
                    dst_data[dst_idx + c * 2 + 1] = bytes[1];
                }
            }
        }
    }

    // Set NCLX color profile if HDR metadata provided
    // Note: libheif-rs only exposes set_color_primaries(), other NCLX fields
    // cannot be set via the Rust API. For full HDR metadata, use ICC profiles.
    if let Some(hdr_meta) = hdr {
        let primaries = match hdr_meta.primaries {
            ColorPrimaries::Bt2020 => libheif_rs::ColorPrimaries::ITU_R_BT_2020_2_and_2100_0,
            ColorPrimaries::DciP3 | ColorPrimaries::DisplayP3 => libheif_rs::ColorPrimaries::SMPTE_EG_432_1,
            _ => libheif_rs::ColorPrimaries::ITU_R_BT_709_5,
        };

        // Create NCLX profile and set primaries (only setter available in libheif-rs)
        if let Some(mut nclx) = ColorProfileNCLX::new() {
            nclx.set_color_primaries(primaries);
            
            heif_image.set_color_profile_nclx(&nclx)
                .map_err(|e| IoError::EncodeError(format!("Failed to set NCLX profile: {}", e)))?;
        }
    }

    // Create context and encode
    let mut ctx = HeifContext::new()
        .map_err(|e| IoError::EncodeError(format!("Failed to create context: {}", e)))?;

    let lib = LibHeif::new();
    let mut encoder = lib.encoder_for_format(CompressionFormat::Hevc)
        .map_err(|e| IoError::EncodeError(format!("Failed to get HEVC encoder: {}", e)))?;

    // Set quality (85 is good balance)
    encoder.set_quality(EncoderQuality::Lossy(85))
        .map_err(|e| IoError::EncodeError(format!("Failed to set quality: {}", e)))?;

    ctx.encode_image(&heif_image, &mut encoder, None)
        .map_err(|e| IoError::EncodeError(format!("HEIF encode error: {}", e)))?;

    ctx.write_to_file(path_str)
        .map_err(|e| IoError::EncodeError(format!("HEIF write error: {}", e)))?;

    Ok(())
}

// === Stubs when heif feature is disabled ===

/// Read HEIF/HEIC image (requires `heif` feature).
#[cfg(not(feature = "heif"))]
pub fn read_heif<P: AsRef<Path>>(_path: P) -> IoResult<(ImageData, Option<HdrMetadata>)> {
    Err(IoError::UnsupportedFormat("HEIF support requires 'heif' feature".into()))
}

/// Write HEIF/HEIC image (requires `heif` feature).
#[cfg(not(feature = "heif"))]
pub fn write_heif<P: AsRef<Path>>(
    _path: P,
    _image: &ImageData,
    _hdr: Option<&HdrMetadata>,
) -> IoResult<()> {
    Err(IoError::UnsupportedFormat("HEIF support requires 'heif' feature".into()))
}

#[cfg(all(test, feature = "heif"))]
mod tests {
    use super::*;

    #[test]
    fn test_hdr_metadata_is_hdr() {
        let sdr = HdrMetadata::default();
        assert!(!sdr.is_hdr());

        let pq = HdrMetadata {
            transfer: TransferCharacteristics::Pq,
            ..Default::default()
        };
        assert!(pq.is_hdr());

        let hlg = HdrMetadata {
            transfer: TransferCharacteristics::Hlg,
            ..Default::default()
        };
        assert!(hlg.is_hdr());
    }

    #[test]
    fn test_hdr_metadata_wide_gamut() {
        let sdr = HdrMetadata::default();
        assert!(!sdr.is_wide_gamut());

        let wide = HdrMetadata {
            primaries: ColorPrimaries::Bt2020,
            ..Default::default()
        };
        assert!(wide.is_wide_gamut());
    }
}
