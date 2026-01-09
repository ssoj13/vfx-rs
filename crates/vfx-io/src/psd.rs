//! Adobe Photoshop PSD/PSB format support.
//!
//! Provides read support for PSD files with layer access.
//!
//! # Features
//!
//! - Read flattened composite image
//! - Access individual layers by name or index
//! - Layer blend modes and opacity
//! - 8-bit RGB/RGBA support
//!
//! # Example
//!
//! ```no_run
//! use vfx_io::psd::{read, read_layers, PsdLayer};
//!
//! // Read flattened composite
//! let image = read("artwork.psd")?;
//!
//! // Read with layer info
//! let layers = read_layers("artwork.psd")?;
//! for layer in &layers {
//!     println!("Layer: {} ({}x{})", layer.name, layer.width, layer.height);
//! }
//! # Ok::<(), vfx_io::IoError>(())
//! ```

use crate::{ImageData, IoError, IoResult};
use psd::{ColorMode, Psd, PsdLayer as PsdCrateLayer};
use std::fs;
use std::path::Path;

/// PSD layer information.
#[derive(Debug, Clone)]
pub struct PsdLayer {
    /// Layer name.
    pub name: String,
    /// Layer width in pixels.
    pub width: u32,
    /// Layer height in pixels.
    pub height: u32,
    /// Layer left offset.
    pub left: i32,
    /// Layer top offset.
    pub top: i32,
    /// Layer opacity (0.0 - 1.0).
    pub opacity: f32,
    /// Layer visibility.
    pub visible: bool,
    /// Blend mode name.
    pub blend_mode: String,
    /// Layer pixel data (RGBA u8).
    pub pixels: Option<Vec<u8>>,
}

/// PSD document information.
#[derive(Debug, Clone)]
pub struct PsdInfo {
    /// Document width.
    pub width: u32,
    /// Document height.
    pub height: u32,
    /// Color mode (RGB, CMYK, etc.).
    pub color_mode: String,
    /// Bits per channel.
    pub bit_depth: u8,
    /// Number of layers.
    pub layer_count: usize,
}

/// Reads a PSD file and returns the flattened composite image.
///
/// This returns the final rendered image with all visible layers composited.
///
/// # Arguments
///
/// * `path` - Path to .psd file
///
/// # Returns
///
/// ImageData with RGBA f32 pixels.
pub fn read<P: AsRef<Path>>(path: P) -> IoResult<ImageData> {
    let data = fs::read(path.as_ref())?;
    read_from_memory(&data)
}

/// Reads PSD from memory buffer.
pub fn read_from_memory(data: &[u8]) -> IoResult<ImageData> {
    let psd = Psd::from_bytes(data)
        .map_err(|e| IoError::DecodeError(format!("PSD parse error: {e}")))?;

    let width = psd.width();
    let height = psd.height();

    // Get composited RGBA image
    let rgba = psd.rgba();

    // Validate color mode
    match psd.color_mode() {
        ColorMode::Rgb => {}
        ColorMode::Grayscale => {}
        mode => {
            return Err(IoError::UnsupportedFormat(format!(
                "PSD color mode {:?} not supported, convert to RGB",
                mode
            )));
        }
    }

    // Convert u8 RGBA to f32
    let pixel_count = (width * height) as usize;
    let mut pixels = Vec::with_capacity(pixel_count * 4);

    for chunk in rgba.chunks_exact(4) {
        pixels.push(chunk[0] as f32 / 255.0);
        pixels.push(chunk[1] as f32 / 255.0);
        pixels.push(chunk[2] as f32 / 255.0);
        pixels.push(chunk[3] as f32 / 255.0);
    }

    Ok(ImageData {
        width,
        height,
        channels: 4,
        pixels,
    })
}

/// Reads PSD document info without loading pixel data.
pub fn read_info<P: AsRef<Path>>(path: P) -> IoResult<PsdInfo> {
    let data = fs::read(path.as_ref())?;
    let psd = Psd::from_bytes(&data)
        .map_err(|e| IoError::DecodeError(format!("PSD parse error: {e}")))?;

    let color_mode = match psd.color_mode() {
        ColorMode::Bitmap => "Bitmap",
        ColorMode::Grayscale => "Grayscale",
        ColorMode::Indexed => "Indexed",
        ColorMode::Rgb => "RGB",
        ColorMode::Cmyk => "CMYK",
        ColorMode::Multichannel => "Multichannel",
        ColorMode::Duotone => "Duotone",
        ColorMode::Lab => "Lab",
    };

    Ok(PsdInfo {
        width: psd.width(),
        height: psd.height(),
        color_mode: color_mode.to_string(),
        bit_depth: psd.depth() as u8,
        layer_count: psd.layers().len(),
    })
}

/// Reads all layers from a PSD file.
///
/// Returns layer metadata and optionally pixel data for each layer.
pub fn read_layers<P: AsRef<Path>>(path: P) -> IoResult<Vec<PsdLayer>> {
    read_layers_opts(path, true)
}

/// Reads layers with option to skip pixel data.
pub fn read_layers_opts<P: AsRef<Path>>(path: P, load_pixels: bool) -> IoResult<Vec<PsdLayer>> {
    let data = fs::read(path.as_ref())?;
    let psd = Psd::from_bytes(&data)
        .map_err(|e| IoError::DecodeError(format!("PSD parse error: {e}")))?;

    let mut layers = Vec::new();

    for layer in psd.layers().iter() {
        let pixels = if load_pixels {
            layer.rgba().ok()
        } else {
            None
        };

        layers.push(PsdLayer {
            name: layer.name().to_string(),
            width: layer.width() as u32,
            height: layer.height() as u32,
            left: layer.layer_left(),
            top: layer.layer_top(),
            opacity: layer.opacity() as f32 / 255.0,
            visible: layer.visible(),
            blend_mode: format!("{:?}", layer.blend_mode()),
            pixels,
        });
    }

    Ok(layers)
}

/// Reads a specific layer by name.
pub fn read_layer_by_name<P: AsRef<Path>>(path: P, name: &str) -> IoResult<PsdLayer> {
    let data = fs::read(path.as_ref())?;
    let psd = Psd::from_bytes(&data)
        .map_err(|e| IoError::DecodeError(format!("PSD parse error: {e}")))?;

    let layer = psd
        .layer_by_name(name)
        .ok_or_else(|| IoError::MissingData(format!("Layer '{}' not found", name)))?;

    Ok(PsdLayer {
        name: layer.name().to_string(),
        width: layer.width() as u32,
        height: layer.height() as u32,
        left: layer.layer_left(),
        top: layer.layer_top(),
        opacity: layer.opacity() as f32 / 255.0,
        visible: layer.visible(),
        blend_mode: format!("{:?}", layer.blend_mode()),
        pixels: layer.rgba().ok(),
    })
}

/// Reads a specific layer by index.
pub fn read_layer_by_index<P: AsRef<Path>>(path: P, index: usize) -> IoResult<PsdLayer> {
    let data = fs::read(path.as_ref())?;
    let psd = Psd::from_bytes(&data)
        .map_err(|e| IoError::DecodeError(format!("PSD parse error: {e}")))?;

    let layer = psd
        .layer_by_idx(index)
        .ok_or_else(|| IoError::MissingData(format!("Layer index {} not found", index)))?;

    Ok(PsdLayer {
        name: layer.name().to_string(),
        width: layer.width() as u32,
        height: layer.height() as u32,
        left: layer.layer_left(),
        top: layer.layer_top(),
        opacity: layer.opacity() as f32 / 255.0,
        visible: layer.visible(),
        blend_mode: format!("{:?}", layer.blend_mode()),
        pixels: layer.rgba().ok(),
    })
}

/// Converts a PSD layer to ImageData.
///
/// The returned image is positioned at (0,0) with the layer's dimensions.
/// Use `layer.left` and `layer.top` to position it in the document.
pub fn layer_to_image(layer: &PsdLayer) -> IoResult<ImageData> {
    let pixels = layer
        .pixels
        .as_ref()
        .ok_or_else(|| IoError::MissingData("Layer has no pixel data".into()))?;

    let pixel_count = (layer.width * layer.height) as usize;
    let mut float_pixels = Vec::with_capacity(pixel_count * 4);

    for chunk in pixels.chunks_exact(4) {
        float_pixels.push(chunk[0] as f32 / 255.0);
        float_pixels.push(chunk[1] as f32 / 255.0);
        float_pixels.push(chunk[2] as f32 / 255.0);
        float_pixels.push(chunk[3] as f32 / 255.0);
    }

    Ok(ImageData {
        width: layer.width,
        height: layer.height,
        channels: 4,
        pixels: float_pixels,
    })
}

/// Flattens layers with a custom filter.
///
/// # Arguments
///
/// * `path` - Path to PSD file
/// * `filter` - Function that returns true for layers to include
pub fn flatten_with_filter<P, F>(path: P, filter: F) -> IoResult<ImageData>
where
    P: AsRef<Path>,
    F: Fn(usize, &str) -> bool,
{
    let data = fs::read(path.as_ref())?;
    let psd = Psd::from_bytes(&data)
        .map_err(|e| IoError::DecodeError(format!("PSD parse error: {e}")))?;

    let width = psd.width();
    let height = psd.height();

    let rgba = psd
        .flatten_layers_rgba(&|(idx, layer)| filter(idx, layer.name()))
        .map_err(|e| IoError::DecodeError(format!("Flatten error: {e}")))?;

    let pixel_count = (width * height) as usize;
    let mut pixels = Vec::with_capacity(pixel_count * 4);

    for chunk in rgba.chunks_exact(4) {
        pixels.push(chunk[0] as f32 / 255.0);
        pixels.push(chunk[1] as f32 / 255.0);
        pixels.push(chunk[2] as f32 / 255.0);
        pixels.push(chunk[3] as f32 / 255.0);
    }

    Ok(ImageData {
        width,
        height,
        channels: 4,
        pixels,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_psd_info_struct() {
        let info = PsdInfo {
            width: 1920,
            height: 1080,
            color_mode: "RGB".to_string(),
            bit_depth: 8,
            layer_count: 5,
        };
        assert_eq!(info.width, 1920);
        assert_eq!(info.color_mode, "RGB");
    }

    #[test]
    fn test_psd_layer_struct() {
        let layer = PsdLayer {
            name: "Background".to_string(),
            width: 100,
            height: 100,
            left: 0,
            top: 0,
            opacity: 1.0,
            visible: true,
            blend_mode: "Normal".to_string(),
            pixels: None,
        };
        assert_eq!(layer.name, "Background");
        assert!(layer.visible);
    }
}
