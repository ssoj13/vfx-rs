//! CLI command implementations
//!
//! Provides image processing commands with multi-layer EXR support.
//! Use `--layer <name>` to process a specific layer in multi-layer files.

pub mod info;
pub mod convert;
pub mod resize;
pub mod crop;
pub mod diff;
pub mod composite;
pub mod blur;
pub mod sharpen;
pub mod color;
pub mod lut;
pub mod transform;
pub mod maketx;
pub mod grep;
pub mod batch;
pub mod layers;
pub mod channels;
pub mod paste;
pub mod rotate;
pub mod warp;
pub mod aces;
pub mod udim;
pub mod grade;
pub mod clamp;
pub mod premult;
#[cfg(feature = "viewer")]
pub mod view;

use vfx_io::ImageData;
use vfx_io::exr::{ExrReader, ExrWriter};
use std::path::Path;
use anyhow::{Result, Context, bail};

/// Load image from path
pub fn load_image(path: &Path) -> Result<ImageData> {
    vfx_io::read(path)
        .with_context(|| format!("Failed to load: {}", path.display()))
}

/// Save image to path
pub fn save_image(path: &Path, image: &ImageData) -> Result<()> {
    vfx_io::write(path, image)
        .with_context(|| format!("Failed to save: {}", path.display()))
}

/// Ensure the image channels are valid for color processing operations.
///
/// Delegates to [`vfx_ops::guard::ensure_color_channels`] for validation.
/// This is a thin wrapper that converts OpsError to anyhow::Error.
pub fn ensure_color_processing(image: &ImageData, op: &str, allow_non_color: bool) -> Result<()> {
    vfx_ops::guard::ensure_color_channels(image, op, allow_non_color)
        .map_err(|e| anyhow::anyhow!(e))
}

/// Load image from path, optionally extracting a specific layer.
///
/// If `layer` is Some and file is EXR, extracts that layer only.
/// Otherwise loads the entire image (first layer for EXR).
pub fn load_image_layer(path: &Path, layer: Option<&str>) -> Result<ImageData> {
    let is_exr = path.extension().map(|e| e.eq_ignore_ascii_case("exr")).unwrap_or(false);
    
    if is_exr && layer.is_some() {
        let layer_name = layer.unwrap();
        let reader = ExrReader::new();
        let layered = reader.read_layers(path)
            .with_context(|| format!("Failed to load EXR: {}", path.display()))?;
        
        // Find requested layer
        for img_layer in layered.layers {
            if img_layer.name == layer_name {
                return Ok(img_layer.to_image_data()?);
            }
        }
        bail!("Layer '{}' not found in {}", layer_name, path.display());
    } else {
        load_image(path)
    }
}

/// Save image to path, optionally with a layer name for EXR.
///
/// For EXR output with layer name, creates a single-layer multi-part file.
pub fn save_image_layer(path: &Path, image: &ImageData, layer: Option<&str>) -> Result<()> {
    let is_exr = path.extension().map(|e| e.eq_ignore_ascii_case("exr")).unwrap_or(false);
    
    if is_exr && layer.is_some() {
        let layer_name = layer.unwrap();
        let layered = image.to_layered(layer_name);
        let writer = ExrWriter::new();
        writer.write_layers(path, &layered)
            .with_context(|| format!("Failed to save EXR: {}", path.display()))?;
        Ok(())
    } else {
        save_image(path, image)
    }
}

/// Format file size for display
pub fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}
