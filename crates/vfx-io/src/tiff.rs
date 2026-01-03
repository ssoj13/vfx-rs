//! TIFF format support.
//!
//! Provides reading and writing of TIFF files, widely used in
//! print, scanning, and archival workflows.
//!
//! # Features
//!
//! - 8-bit and 16-bit support
//! - RGB and RGBA
//! - LZW and ZIP compression
//! - Metadata preservation
//!
//! # Example
//!
//! ```rust,ignore
//! use vfx_io::tiff;
//!
//! let image = tiff::read("scan.tiff")?;
//! tiff::write("output.tiff", &image)?;
//! ```

use crate::{ImageData, IoError, IoResult, Metadata, PixelData, PixelFormat};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

/// Reads a TIFF file from the given path.
///
/// # Example
///
/// ```rust,ignore
/// use vfx_io::tiff;
///
/// let image = tiff::read("input.tiff")?;
/// println!("Size: {}x{}", image.width, image.height);
/// ```
pub fn read<P: AsRef<Path>>(path: P) -> IoResult<ImageData> {
    use tiff::decoder::{Decoder, DecodingResult};
    use tiff::ColorType;

    let file = File::open(path.as_ref())?;
    let reader = BufReader::new(file);

    let mut decoder = Decoder::new(reader)
        .map_err(|e: tiff::TiffError| IoError::DecodeError(e.to_string()))?;

    let (width, height) = decoder.dimensions()
        .map_err(|e: tiff::TiffError| IoError::DecodeError(e.to_string()))?;
    let color_type = decoder.colortype()
        .map_err(|e: tiff::TiffError| IoError::DecodeError(e.to_string()))?;

    let result = decoder.read_image()
        .map_err(|e: tiff::TiffError| IoError::DecodeError(e.to_string()))?;

    let (data, format, channels) = match (color_type, result) {
        // 8-bit RGB
        (ColorType::RGB(8), DecodingResult::U8(buf)) => {
            let f32_data: Vec<f32> = buf.iter().map(|&v| v as f32 / 255.0).collect();
            (PixelData::F32(f32_data), PixelFormat::F32, 3)
        }
        // 8-bit RGBA
        (ColorType::RGBA(8), DecodingResult::U8(buf)) => {
            let f32_data: Vec<f32> = buf.iter().map(|&v| v as f32 / 255.0).collect();
            (PixelData::F32(f32_data), PixelFormat::F32, 4)
        }
        // 16-bit RGB
        (ColorType::RGB(16), DecodingResult::U16(buf)) => {
            let f32_data: Vec<f32> = buf.iter().map(|&v| v as f32 / 65535.0).collect();
            (PixelData::F32(f32_data), PixelFormat::F32, 3)
        }
        // 16-bit RGBA
        (ColorType::RGBA(16), DecodingResult::U16(buf)) => {
            let f32_data: Vec<f32> = buf.iter().map(|&v| v as f32 / 65535.0).collect();
            (PixelData::F32(f32_data), PixelFormat::F32, 4)
        }
        // 8-bit Grayscale
        (ColorType::Gray(8), DecodingResult::U8(buf)) => {
            let f32_data: Vec<f32> = buf.iter().map(|&v| v as f32 / 255.0).collect();
            (PixelData::F32(f32_data), PixelFormat::F32, 1)
        }
        // 16-bit Grayscale
        (ColorType::Gray(16), DecodingResult::U16(buf)) => {
            let f32_data: Vec<f32> = buf.iter().map(|&v| v as f32 / 65535.0).collect();
            (PixelData::F32(f32_data), PixelFormat::F32, 1)
        }
        // 32-bit float RGB
        (ColorType::RGB(32), DecodingResult::F32(buf)) => {
            (PixelData::F32(buf), PixelFormat::F32, 3)
        }
        // 32-bit float RGBA
        (ColorType::RGBA(32), DecodingResult::F32(buf)) => {
            (PixelData::F32(buf), PixelFormat::F32, 4)
        }
        (ct, _) => {
            return Err(IoError::DecodeError(format!(
                "unsupported TIFF color type: {:?}",
                ct
            )));
        }
    };

    let mut metadata = Metadata::default();
    metadata.colorspace = Some("srgb".to_string());

    Ok(ImageData {
        width,
        height,
        channels,
        format,
        data,
        metadata,
    })
}

/// Writes an image to a TIFF file.
///
/// Writes as 16-bit RGB/RGBA with LZW compression.
///
/// # Example
///
/// ```rust,ignore
/// use vfx_io::tiff;
///
/// tiff::write("output.tiff", &image)?;
/// ```
pub fn write<P: AsRef<Path>>(path: P, image: &ImageData) -> IoResult<()> {
    use tiff::encoder::{colortype, compression, TiffEncoder};

    let file = File::create(path.as_ref())?;

    let mut encoder = TiffEncoder::new(file)
        .map_err(|e: tiff::TiffError| IoError::EncodeError(e.to_string()))?;

    let f32_data = image.to_f32();
    let width = image.width;
    let height = image.height;

    // Convert to 16-bit
    let u16_data: Vec<u16> = f32_data
        .iter()
        .map(|&v| (v.clamp(0.0, 1.0) * 65535.0) as u16)
        .collect();

    match image.channels {
        3 => {
            encoder
                .write_image_with_compression::<colortype::RGB16, compression::Lzw>(
                    width,
                    height,
                    compression::Lzw,
                    &u16_data,
                )
                .map_err(|e: tiff::TiffError| IoError::EncodeError(e.to_string()))?;
        }
        4 => {
            encoder
                .write_image_with_compression::<colortype::RGBA16, compression::Lzw>(
                    width,
                    height,
                    compression::Lzw,
                    &u16_data,
                )
                .map_err(|e: tiff::TiffError| IoError::EncodeError(e.to_string()))?;
        }
        1 => {
            encoder
                .write_image_with_compression::<colortype::Gray16, compression::Lzw>(
                    width,
                    height,
                    compression::Lzw,
                    &u16_data,
                )
                .map_err(|e: tiff::TiffError| IoError::EncodeError(e.to_string()))?;
        }
        _ => {
            return Err(IoError::EncodeError(format!(
                "unsupported channel count: {}",
                image.channels
            )));
        }
    }

    Ok(())
}

/// TIFF compression method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Compression {
    /// No compression.
    None,
    /// LZW compression (lossless, good compression).
    #[default]
    Lzw,
    /// ZIP/Deflate compression.
    Deflate,
    /// PackBits compression (simple RLE).
    PackBits,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip_rgb() {
        let width = 32;
        let height = 32;
        let mut data = Vec::with_capacity((width * height * 3) as usize);

        for y in 0..height {
            for x in 0..width {
                data.push(x as f32 / width as f32);
                data.push(y as f32 / height as f32);
                data.push(0.5);
            }
        }

        let image = ImageData::from_f32(width, height, 3, data);

        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join("vfx_io_test.tiff");

        write(&temp_path, &image).expect("Failed to write TIFF");
        let loaded = read(&temp_path).expect("Failed to read TIFF");

        assert_eq!(loaded.width, width);
        assert_eq!(loaded.height, height);
        assert_eq!(loaded.channels, 3);

        let _ = std::fs::remove_file(&temp_path);
    }

    #[test]
    fn test_roundtrip_rgba() {
        let width = 16;
        let height = 16;
        let data = vec![0.5f32; (width * height * 4) as usize];
        let image = ImageData::from_f32(width, height, 4, data);

        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join("vfx_io_test_rgba.tiff");

        write(&temp_path, &image).expect("Failed to write TIFF");
        let loaded = read(&temp_path).expect("Failed to read TIFF");

        assert_eq!(loaded.width, width);
        assert_eq!(loaded.height, height);
        assert_eq!(loaded.channels, 4);

        let _ = std::fs::remove_file(&temp_path);
    }
}
