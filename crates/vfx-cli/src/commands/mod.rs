//! CLI command implementations

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

use vfx_io::{ChannelKind, ImageData};
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
pub fn ensure_color_processing(image: &ImageData, op: &str) -> Result<()> {
    let layer = image.to_layer("input");
    for channel in &layer.channels {
        match channel.kind {
            ChannelKind::Color | ChannelKind::Alpha | ChannelKind::Depth => {}
            ChannelKind::Id | ChannelKind::Mask | ChannelKind::Generic => {
                bail!(
                    "{} is not supported for channel '{}' (kind: {:?})",
                    op,
                    channel.name,
                    channel.kind
                );
            }
        }
    }
    Ok(())
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
