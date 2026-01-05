//! AVIF format support (write-only).
//!
//! Write AVIF images via the `image` crate (rav1e encoder).
//! Reading requires dav1d library - add "avif-native" feature when available.
//!
//! # Example
//!
//! ```ignore
//! use vfx_io::avif;
//! avif::write("output.avif", &img)?;
//! ```

use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

use image::DynamicImage;

use crate::{ImageData, IoError, IoResult};

/// AVIF writer options.
#[derive(Debug, Clone)]
pub struct AvifWriterOptions {
    /// Quality (0-100). Default: 80.
    pub quality: u8,
    /// Encoding speed (1-10, higher = faster). Default: 6.
    pub speed: u8,
}

impl Default for AvifWriterOptions {
    fn default() -> Self {
        Self { quality: 80, speed: 6 }
    }
}

/// Writes an image to AVIF format.
pub fn write<P: AsRef<Path>>(path: P, image: &ImageData) -> IoResult<()> {
    write_with_options(path, image, &AvifWriterOptions::default())
}

/// Writes an image to AVIF with custom options.
pub fn write_with_options<P: AsRef<Path>>(
    path: P,
    image: &ImageData,
    options: &AvifWriterOptions,
) -> IoResult<()> {
    let file = File::create(path.as_ref())?;
    let writer = BufWriter::new(file);
    
    let dyn_img = to_dynamic(image)?;
    let encoder = image::codecs::avif::AvifEncoder::new_with_speed_quality(
        writer, options.speed, options.quality,
    );
    
    dyn_img.write_with_encoder(encoder)
        .map_err(|e| IoError::EncodeError(e.to_string()))
}

fn to_dynamic(image: &ImageData) -> IoResult<DynamicImage> {
    let data = image.to_u8();
    match image.channels {
        1 => image::GrayImage::from_raw(image.width, image.height, data)
            .map(DynamicImage::ImageLuma8)
            .ok_or_else(|| IoError::EncodeError("Invalid image data".into())),
        3 => image::RgbImage::from_raw(image.width, image.height, data)
            .map(DynamicImage::ImageRgb8)
            .ok_or_else(|| IoError::EncodeError("Invalid image data".into())),
        _ => image::RgbaImage::from_raw(image.width, image.height, image.to_rgba_u8())
            .map(DynamicImage::ImageRgba8)
            .ok_or_else(|| IoError::EncodeError("Invalid image data".into())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_avif_write() {
        let image = ImageData::from_u8(64, 64, 3, vec![128u8; 64 * 64 * 3]);
        let temp = std::env::temp_dir().join("test_avif.avif");
        write(&temp, &image).unwrap();
        assert!(std::fs::metadata(&temp).unwrap().len() > 0);
        std::fs::remove_file(temp).ok();
    }
}
