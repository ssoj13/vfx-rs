//! AVIF format support (write-only).
//!
//! Write AVIF images via the `image` crate (rav1e encoder).
//! AVIF is a modern image format based on AV1 codec with excellent compression.
//! Supports HDR (10/12-bit), alpha channel, wide color gamut.
//!
//! # Note
//!
//! Reading AVIF requires the system `dav1d` library which is not bundled.
//! Install dav1d and enable `avif-native` feature of the image crate for decoding.
//!
//! # Example
//!
//! ```ignore
//! use vfx_io::avif;
//!
//! // Write AVIF (encoding supported)
//! avif::write("output.avif", &img)?;
//! ```

use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

use image::DynamicImage;

use crate::{ImageData, IoError, IoResult, PixelFormat};

/// AVIF writer options.
#[derive(Debug, Clone)]
pub struct AvifWriterOptions {
    /// Quality (0-100). Default: 80.
    pub quality: u8,
    /// Encoding speed (1-10, higher = faster but worse compression). Default: 6.
    pub speed: u8,
}

impl Default for AvifWriterOptions {
    fn default() -> Self {
        Self {
            quality: 80,
            speed: 6,
        }
    }
}

/// Writes an image to AVIF format with default options.
pub fn write<P: AsRef<Path>>(path: P, image: &ImageData) -> IoResult<()> {
    write_with_options(path, image, &AvifWriterOptions::default())
}

/// Writes an image to AVIF format with custom options.
pub fn write_with_options<P: AsRef<Path>>(
    path: P,
    image: &ImageData,
    options: &AvifWriterOptions,
) -> IoResult<()> {
    let file = File::create(path.as_ref())?;
    let writer = BufWriter::new(file);
    
    let dyn_img = image_data_to_dynamic(image)?;
    
    // image crate AVIF encoder (rav1e)
    let encoder = image::codecs::avif::AvifEncoder::new_with_speed_quality(
        writer,
        options.speed,
        options.quality,
    );
    
    dyn_img.write_with_encoder(encoder)
        .map_err(|e| IoError::EncodeError(e.to_string()))?;
    
    Ok(())
}

/// Converts ImageData to DynamicImage for encoding.
fn image_data_to_dynamic(image: &ImageData) -> IoResult<DynamicImage> {
    // AVIF encoder works best with 8-bit data
    // Convert everything to 8-bit for encoding
    let data = image.to_u8();
    
    match image.channels {
        1 => {
            let img = image::GrayImage::from_raw(image.width, image.height, data)
                .ok_or_else(|| IoError::EncodeError("Failed to create grayscale image".into()))?;
            Ok(DynamicImage::ImageLuma8(img))
        }
        3 => {
            let img = image::RgbImage::from_raw(image.width, image.height, data)
                .ok_or_else(|| IoError::EncodeError("Failed to create RGB image".into()))?;
            Ok(DynamicImage::ImageRgb8(img))
        }
        4 => {
            let img = image::RgbaImage::from_raw(image.width, image.height, data)
                .ok_or_else(|| IoError::EncodeError("Failed to create RGBA image".into()))?;
            Ok(DynamicImage::ImageRgba8(img))
        }
        _ => {
            // Convert to RGBA
            let mut rgba_data = Vec::with_capacity((image.width * image.height * 4) as usize);
            for chunk in data.chunks(image.channels as usize) {
                rgba_data.push(chunk.first().copied().unwrap_or(0));
                rgba_data.push(chunk.get(1).copied().unwrap_or(0));
                rgba_data.push(chunk.get(2).copied().unwrap_or(0));
                rgba_data.push(chunk.get(3).copied().unwrap_or(255));
            }
            let img = image::RgbaImage::from_raw(image.width, image.height, rgba_data)
                .ok_or_else(|| IoError::EncodeError("Failed to create RGBA image".into()))?;
            Ok(DynamicImage::ImageRgba8(img))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_avif_write() {
        let data = vec![128u8; 64 * 64 * 3];
        let image = ImageData::from_u8(64, 64, 3, data);
        
        let temp = std::env::temp_dir().join("test_avif_write.avif");
        write(&temp, &image).unwrap();
        
        // Verify file was created and has content
        let meta = std::fs::metadata(&temp).unwrap();
        assert!(meta.len() > 0);
        
        std::fs::remove_file(temp).ok();
    }
}
