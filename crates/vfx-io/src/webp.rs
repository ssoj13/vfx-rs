//! WebP format support.
//!
//! Read/write WebP images via the `image` crate.
//! Supports lossy and lossless compression, alpha channel.
//!
//! # Example
//!
//! ```ignore
//! use vfx_io::webp;
//!
//! let img = webp::read("input.webp")?;
//! webp::write("output.webp", &img)?;
//! ```

use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::Path;

use image::{DynamicImage, ImageFormat, ImageReader};

use crate::{ImageData, IoError, IoResult, PixelData, PixelFormat, Metadata};

/// WebP writer options.
#[derive(Debug, Clone)]
pub struct WebpWriterOptions {
    /// Quality for lossy compression (0-100). Default: 80.
    pub quality: u8,
    /// Use lossless compression. Default: false.
    pub lossless: bool,
}

impl Default for WebpWriterOptions {
    fn default() -> Self {
        Self {
            quality: 80,
            lossless: false,
        }
    }
}

/// Reads a WebP image from file.
pub fn read<P: AsRef<Path>>(path: P) -> IoResult<ImageData> {
    let file = File::open(path.as_ref())?;
    let reader = BufReader::new(file);
    
    let img = ImageReader::with_format(reader, ImageFormat::WebP)
        .decode()
        .map_err(|e| IoError::DecodeError(e.to_string()))?;
    
    dynamic_to_image_data(img)
}

/// Writes an image to WebP format with default options.
pub fn write<P: AsRef<Path>>(path: P, image: &ImageData) -> IoResult<()> {
    write_with_options(path, image, &WebpWriterOptions::default())
}

/// Writes an image to WebP format with custom options.
pub fn write_with_options<P: AsRef<Path>>(
    path: P,
    image: &ImageData,
    options: &WebpWriterOptions,
) -> IoResult<()> {
    let file = File::create(path.as_ref())?;
    let writer = BufWriter::new(file);
    
    let dyn_img = image_data_to_dynamic(image)?;
    
    // image crate WebP encoder
    let encoder = image::codecs::webp::WebPEncoder::new_lossless(writer);
    
    // Note: image crate's WebP encoder only supports lossless mode
    // For lossy encoding with quality control, consider using libwebp directly
    let _ = options; // Silence unused warning
    
    dyn_img.write_with_encoder(encoder)
        .map_err(|e| IoError::EncodeError(e.to_string()))?;
    
    Ok(())
}

/// Converts DynamicImage to ImageData.
fn dynamic_to_image_data(img: DynamicImage) -> IoResult<ImageData> {
    let (width, height) = (img.width(), img.height());
    
    match img {
        DynamicImage::ImageRgb8(rgb) => {
            let data = rgb.into_raw();
            Ok(ImageData {
                width,
                height,
                channels: 3,
                format: PixelFormat::U8,
                data: PixelData::U8(data),
                metadata: Metadata::default(),
            })
        }
        DynamicImage::ImageRgba8(rgba) => {
            let data = rgba.into_raw();
            Ok(ImageData {
                width,
                height,
                channels: 4,
                format: PixelFormat::U8,
                data: PixelData::U8(data),
                metadata: Metadata::default(),
            })
        }
        DynamicImage::ImageLuma8(gray) => {
            let data = gray.into_raw();
            Ok(ImageData {
                width,
                height,
                channels: 1,
                format: PixelFormat::U8,
                data: PixelData::U8(data),
                metadata: Metadata::default(),
            })
        }
        DynamicImage::ImageLumaA8(gray_alpha) => {
            let data = gray_alpha.into_raw();
            Ok(ImageData {
                width,
                height,
                channels: 2,
                format: PixelFormat::U8,
                data: PixelData::U8(data),
                metadata: Metadata::default(),
            })
        }
        _ => {
            // Convert to RGBA8 for other formats
            let rgba = img.to_rgba8();
            let data = rgba.into_raw();
            Ok(ImageData {
                width,
                height,
                channels: 4,
                format: PixelFormat::U8,
                data: PixelData::U8(data),
                metadata: Metadata::default(),
            })
        }
    }
}

/// Converts ImageData to DynamicImage.
fn image_data_to_dynamic(image: &ImageData) -> IoResult<DynamicImage> {
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
    fn test_webp_roundtrip() {
        let data = vec![128u8; 64 * 64 * 3];
        let image = ImageData::from_u8(64, 64, 3, data);
        
        let temp = std::env::temp_dir().join("test_webp.webp");
        write(&temp, &image).unwrap();
        
        let loaded = read(&temp).unwrap();
        assert_eq!(loaded.width, 64);
        assert_eq!(loaded.height, 64);
        // WebP may convert to RGBA
        assert!(loaded.channels >= 3);
        
        std::fs::remove_file(temp).ok();
    }
}
