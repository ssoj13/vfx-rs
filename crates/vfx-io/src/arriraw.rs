//! ARRI Raw (.ari) format support.
//!
//! ARRIRAW is a proprietary format used by ARRI Alexa cameras.
//! Full support requires the ARRI SDK which is not open source.
//!
//! Current implementation provides:
//! - Format detection (magic bytes and extension)
//! - Metadata extraction (header parsing)
//! - Stub decode that returns "SDK required" error
//!
//! To enable full decode support, the ARRI SDK must be integrated.

use crate::{ImageData, IoError, IoResult};
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;

/// ARRIRAW file header information.
#[derive(Debug, Clone)]
pub struct ArriRawHeader {
    /// Image width in pixels.
    pub width: u32,
    /// Image height in pixels.
    pub height: u32,
    /// Camera model (e.g., "ALEXA Mini", "ALEXA 65").
    pub camera_model: String,
    /// Camera serial number.
    pub serial: String,
    /// Sensor mode.
    pub sensor_mode: String,
    /// White balance (Kelvin).
    pub white_balance: u32,
    /// ISO/ASA value.
    pub iso: u32,
    /// Shutter angle (degrees).
    pub shutter_angle: f32,
    /// Frame rate.
    pub fps: f32,
    /// Timecode.
    pub timecode: String,
    /// Color science version.
    pub color_science: String,
}

impl Default for ArriRawHeader {
    fn default() -> Self {
        Self {
            width: 0,
            height: 0,
            camera_model: String::new(),
            serial: String::new(),
            sensor_mode: String::new(),
            white_balance: 5600,
            iso: 800,
            shutter_angle: 180.0,
            fps: 24.0,
            timecode: String::new(),
            color_science: String::new(),
        }
    }
}

/// Reads ARRIRAW header metadata.
///
/// This parses the file header to extract camera and shot metadata
/// without requiring the full SDK for decode.
pub fn read_header<P: AsRef<Path>>(path: P) -> IoResult<ArriRawHeader> {
    let file = File::open(path.as_ref())?;
    let mut reader = BufReader::new(file);

    // Read magic bytes
    let mut magic = [0u8; 4];
    reader.read_exact(&mut magic)?;

    if &magic != b"ARRI" {
        return Err(IoError::Format("Not a valid ARRIRAW file".into()));
    }

    // Skip to header block (offset varies by version)
    // This is a simplified parser - real implementation needs full spec
    let mut header = ArriRawHeader::default();

    // Read version
    let mut version = [0u8; 4];
    reader.read_exact(&mut version)?;

    // Basic header parsing (simplified)
    // Real implementation would need full ARRI header specification
    reader.seek(SeekFrom::Start(32))?;

    let mut buf = [0u8; 4];
    reader.read_exact(&mut buf)?;
    header.width = u32::from_le_bytes(buf);

    reader.read_exact(&mut buf)?;
    header.height = u32::from_le_bytes(buf);

    // Camera model is typically at a fixed offset
    reader.seek(SeekFrom::Start(128))?;
    let mut model_buf = [0u8; 32];
    reader.read_exact(&mut model_buf)?;
    header.camera_model = String::from_utf8_lossy(&model_buf)
        .trim_end_matches('\0')
        .to_string();

    Ok(header)
}

/// Decodes ARRIRAW to linear RGB image data.
///
/// **Note**: This is a stub implementation. Full decode requires the ARRI SDK.
///
/// # Errors
///
/// Returns `IoError::UnsupportedFeature` as SDK is not available.
pub fn decode<P: AsRef<Path>>(_path: P) -> IoResult<ImageData> {
    Err(IoError::UnsupportedFeature(
        "ARRIRAW decode requires ARRI SDK. Header metadata is available via read_header()".into()
    ))
}

/// Decodes ARRIRAW with specific processing options.
///
/// **Note**: This is a stub implementation. Full decode requires the ARRI SDK.
#[derive(Debug, Clone)]
pub struct ArriDecodeOptions {
    /// Output colorspace (e.g., "LogC", "ACEScg").
    pub colorspace: String,
    /// Debayer quality.
    pub debayer_quality: DebayerQuality,
    /// Apply in-camera CDL.
    pub apply_cdl: bool,
}

/// Debayer quality settings.
#[derive(Debug, Clone, Copy, Default)]
pub enum DebayerQuality {
    /// Fast preview quality.
    Preview,
    /// Standard quality.
    #[default]
    Standard,
    /// High quality (slower).
    High,
}

impl Default for ArriDecodeOptions {
    fn default() -> Self {
        Self {
            colorspace: "LogC".into(),
            debayer_quality: DebayerQuality::Standard,
            apply_cdl: false,
        }
    }
}

/// Decodes with options.
///
/// **Note**: Stub implementation.
pub fn decode_with_options<P: AsRef<Path>>(_path: P, _options: &ArriDecodeOptions) -> IoResult<ImageData> {
    Err(IoError::UnsupportedFeature(
        "ARRIRAW decode requires ARRI SDK".into()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_returns_sdk_error() {
        let result = decode("nonexistent.ari");
        assert!(result.is_err());
        if let Err(IoError::UnsupportedFeature(msg)) = result {
            assert!(msg.contains("ARRI SDK"));
        }
    }
}
