//! RED REDCODE (.r3d) format support.
//!
//! REDCODE is a proprietary wavelet-compressed format used by RED cameras.
//! Full support requires the RED SDK (free for non-commercial use).
//!
//! Current implementation provides:
//! - Format detection (magic bytes and extension)
//! - Metadata extraction (header parsing)
//! - Stub decode that returns "SDK required" error
//!
//! To enable full decode support, the RED SDK must be integrated.

use crate::{ImageData, IoError, IoResult};
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;

/// REDCODE file header information.
#[derive(Debug, Clone)]
pub struct RedCodeHeader {
    /// Image width in pixels.
    pub width: u32,
    /// Image height in pixels.
    pub height: u32,
    /// Camera model (e.g., "RED ONE", "DSMC2", "V-RAPTOR").
    pub camera_model: String,
    /// Camera serial number.
    pub serial: String,
    /// Firmware version.
    pub firmware: String,
    /// White balance (Kelvin).
    pub white_balance: u32,
    /// ISO value.
    pub iso: u32,
    /// Shutter speed (1/x seconds).
    pub shutter_speed: f32,
    /// Frame rate.
    pub fps: f32,
    /// Total frames in clip.
    pub frame_count: u32,
    /// Timecode.
    pub timecode: String,
    /// REDCODE compression ratio (e.g., "5:1", "8:1").
    pub compression: String,
    /// Color science (IPP2, Legacy).
    pub color_science: String,
    /// Lens metadata (if available).
    pub lens_info: Option<LensInfo>,
}

/// Lens metadata from RED cameras.
#[derive(Debug, Clone)]
pub struct LensInfo {
    /// Lens model name.
    pub model: String,
    /// Focal length (mm).
    pub focal_length: f32,
    /// Aperture (f-stop).
    pub aperture: f32,
    /// Focus distance (meters).
    pub focus_distance: f32,
}

impl Default for RedCodeHeader {
    fn default() -> Self {
        Self {
            width: 0,
            height: 0,
            camera_model: String::new(),
            serial: String::new(),
            firmware: String::new(),
            white_balance: 5600,
            iso: 800,
            shutter_speed: 48.0,
            fps: 24.0,
            frame_count: 0,
            timecode: String::new(),
            compression: String::new(),
            color_science: "IPP2".into(),
            lens_info: None,
        }
    }
}

/// Reads REDCODE header metadata.
///
/// This parses the R3D container header to extract camera and shot metadata
/// without requiring the full SDK for decode.
pub fn read_header<P: AsRef<Path>>(path: P) -> IoResult<RedCodeHeader> {
    let file = File::open(path.as_ref())?;
    let mut reader = BufReader::new(file);

    // Read magic bytes
    let mut magic = [0u8; 4];
    reader.read_exact(&mut magic)?;

    if &magic[0..3] != b"RED" || (magic[3] != b'1' && magic[3] != b'2') {
        return Err(IoError::Format("Not a valid REDCODE file".into()));
    }

    let mut header = RedCodeHeader::default();

    // R3D container uses atoms/boxes similar to QuickTime
    // This is a simplified parser
    reader.seek(SeekFrom::Start(8))?;

    // Read basic dimensions from header
    let mut buf = [0u8; 4];
    reader.read_exact(&mut buf)?;
    header.width = u32::from_be_bytes(buf);

    reader.read_exact(&mut buf)?;
    header.height = u32::from_be_bytes(buf);

    // Camera model typically in metadata atom
    reader.seek(SeekFrom::Start(64))?;
    let mut model_buf = [0u8; 32];
    reader.read_exact(&mut model_buf)?;
    header.camera_model = String::from_utf8_lossy(&model_buf)
        .trim_end_matches('\0')
        .to_string();

    Ok(header)
}

/// Decodes REDCODE to linear RGB image data.
///
/// **Note**: This is a stub implementation. Full decode requires the RED SDK.
///
/// # Arguments
///
/// * `path` - Path to .r3d file
/// * `frame` - Frame number to decode (0-indexed)
///
/// # Errors
///
/// Returns `IoError::UnsupportedFeature` as SDK is not available.
pub fn decode<P: AsRef<Path>>(_path: P, _frame: u32) -> IoResult<ImageData> {
    Err(IoError::UnsupportedFeature(
        "REDCODE decode requires RED SDK. Header metadata is available via read_header()".into()
    ))
}

/// Decode options for REDCODE.
#[derive(Debug, Clone)]
pub struct RedDecodeOptions {
    /// Output colorspace (e.g., "REDWideGamutRGB", "sRGB", "ACEScg").
    pub colorspace: String,
    /// Output gamma (e.g., "Log3G10", "sRGB", "Linear").
    pub gamma: String,
    /// Resolution mode.
    pub resolution: ResolutionMode,
    /// Debayer quality.
    pub debayer_quality: DebayerQuality,
    /// Apply 3D LUT from camera.
    pub apply_lut: bool,
}

/// Output resolution modes.
#[derive(Debug, Clone, Copy, Default)]
pub enum ResolutionMode {
    /// Full resolution.
    #[default]
    Full,
    /// Half resolution (faster).
    Half,
    /// Quarter resolution (preview).
    Quarter,
    /// Eighth resolution (thumbnail).
    Eighth,
}

/// Debayer quality settings.
#[derive(Debug, Clone, Copy, Default)]
pub enum DebayerQuality {
    /// Nearest neighbor (fast).
    Nearest,
    /// Bilinear.
    Bilinear,
    /// Standard.
    #[default]
    Standard,
    /// Full (highest quality, slowest).
    Full,
}

impl Default for RedDecodeOptions {
    fn default() -> Self {
        Self {
            colorspace: "REDWideGamutRGB".into(),
            gamma: "Log3G10".into(),
            resolution: ResolutionMode::Full,
            debayer_quality: DebayerQuality::Standard,
            apply_lut: false,
        }
    }
}

/// Decodes with options.
///
/// **Note**: Stub implementation.
pub fn decode_with_options<P: AsRef<Path>>(
    _path: P,
    _frame: u32,
    _options: &RedDecodeOptions,
) -> IoResult<ImageData> {
    Err(IoError::UnsupportedFeature(
        "REDCODE decode requires RED SDK".into()
    ))
}

/// Returns the number of frames in an R3D clip.
///
/// **Note**: Stub implementation, returns error.
pub fn frame_count<P: AsRef<Path>>(_path: P) -> IoResult<u32> {
    Err(IoError::UnsupportedFeature(
        "REDCODE frame count requires RED SDK".into()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_returns_sdk_error() {
        let result = decode("nonexistent.r3d", 0);
        assert!(result.is_err());
        if let Err(IoError::UnsupportedFeature(msg)) = result {
            assert!(msg.contains("RED SDK"));
        }
    }
}
