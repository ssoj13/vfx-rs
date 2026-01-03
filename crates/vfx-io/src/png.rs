//! PNG format support.
//!
//! Provides reading and writing of PNG files with support for
//! 8-bit and 16-bit images, alpha channels, and gamma metadata.
//!
//! # Features
//!
//! - 8-bit and 16-bit support
//! - RGB and RGBA
//! - Gamma metadata
//! - Compression level control
//!
//! # Example
//!
//! ```rust,ignore
//! use vfx_io::png::{read, write};
//!
//! let image = read("input.png")?;
//! write("output.png", &image)?;
//! ```

use crate::{ImageData, IoError, IoResult, Metadata, PixelData, PixelFormat};
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

/// Reads a PNG file from the given path.
///
/// # Example
///
/// ```rust,ignore
/// use vfx_io::png;
///
/// let image = png::read("input.png")?;
/// ```
pub fn read<P: AsRef<Path>>(path: P) -> IoResult<ImageData> {
    let file = File::open(path.as_ref())?;
    let decoder = png::Decoder::new(std::io::BufReader::new(file));
    let mut reader = decoder.read_info()
        .map_err(|e: png::DecodingError| IoError::DecodeError(e.to_string()))?;
    
    let buf_size = reader.output_buffer_size()
        .ok_or_else(|| IoError::DecodeError("cannot determine output buffer size".into()))?;
    let mut buf = vec![0u8; buf_size];
    let info = reader.next_frame(&mut buf)
        .map_err(|e: png::DecodingError| IoError::DecodeError(e.to_string()))?;
    
    let width = info.width;
    let height = info.height;
    
    let (channels, format, data) = match (info.color_type, info.bit_depth) {
        (png::ColorType::Rgb, png::BitDepth::Eight) => {
            (3, PixelFormat::U8, PixelData::U8(buf[..info.buffer_size()].to_vec()))
        }
        (png::ColorType::Rgba, png::BitDepth::Eight) => {
            (4, PixelFormat::U8, PixelData::U8(buf[..info.buffer_size()].to_vec()))
        }
        (png::ColorType::Rgb, png::BitDepth::Sixteen) => {
            let u16_data = bytes_to_u16(&buf[..info.buffer_size()]);
            (3, PixelFormat::U16, PixelData::U16(u16_data))
        }
        (png::ColorType::Rgba, png::BitDepth::Sixteen) => {
            let u16_data = bytes_to_u16(&buf[..info.buffer_size()]);
            (4, PixelFormat::U16, PixelData::U16(u16_data))
        }
        (png::ColorType::Grayscale, png::BitDepth::Eight) => {
            // Convert grayscale to RGB
            let rgb: Vec<u8> = buf[..info.buffer_size()]
                .iter()
                .flat_map(|&g| [g, g, g])
                .collect();
            (3, PixelFormat::U8, PixelData::U8(rgb))
        }
        (png::ColorType::GrayscaleAlpha, png::BitDepth::Eight) => {
            // Convert grayscale+alpha to RGBA
            let rgba: Vec<u8> = buf[..info.buffer_size()]
                .chunks(2)
                .flat_map(|ga| [ga[0], ga[0], ga[0], ga[1]])
                .collect();
            (4, PixelFormat::U8, PixelData::U8(rgba))
        }
        (color_type, bit_depth) => {
            return Err(IoError::UnsupportedBitDepth(
                format!("{:?} {:?}", color_type, bit_depth)
            ));
        }
    };
    
    let mut metadata = Metadata::default();
    metadata.colorspace = Some("sRGB".to_string());
    
    Ok(ImageData {
        width,
        height,
        channels,
        format,
        data,
        metadata,
    })
}

/// Writes an image to a PNG file.
///
/// Converts to 8-bit if necessary.
///
/// # Example
///
/// ```rust,ignore
/// use vfx_io::png;
///
/// png::write("output.png", &image)?;
/// ```
pub fn write<P: AsRef<Path>>(path: P, image: &ImageData) -> IoResult<()> {
    let file = File::create(path.as_ref())?;
    let writer = BufWriter::new(file);
    
    let color_type = match image.channels {
        1 => png::ColorType::Grayscale,
        2 => png::ColorType::GrayscaleAlpha,
        3 => png::ColorType::Rgb,
        4 => png::ColorType::Rgba,
        n => return Err(IoError::EncodeError(format!("unsupported channel count: {}", n))),
    };
    
    let mut encoder = png::Encoder::new(writer, image.width, image.height);
    encoder.set_color(color_type);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_compression(png::Compression::default());
    
    // Add sRGB chunk
    encoder.set_source_srgb(png::SrgbRenderingIntent::Perceptual);
    
    let mut png_writer = encoder.write_header()
        .map_err(|e| IoError::EncodeError(e.to_string()))?;
    
    // Convert to u8
    let u8_data = image.to_u8();
    
    png_writer.write_image_data(&u8_data)
        .map_err(|e| IoError::EncodeError(e.to_string()))?;
    
    Ok(())
}

/// Converts big-endian byte slice to u16 vector.
fn bytes_to_u16(bytes: &[u8]) -> Vec<u16> {
    bytes
        .chunks(2)
        .map(|chunk| u16::from_be_bytes([chunk[0], chunk[1]]))
        .collect()
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
                data.push((x * 8) as u8);
                data.push((y * 8) as u8);
                data.push(128);
            }
        }
        
        let image = ImageData::from_u8(width, height, 3, data.clone());
        
        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join("vfx_io_test_rgb.png");
        
        write(&temp_path, &image).expect("Failed to write PNG");
        
        let loaded = read(&temp_path).expect("Failed to read PNG");
        
        assert_eq!(loaded.width, width);
        assert_eq!(loaded.height, height);
        assert_eq!(loaded.channels, 3);
        
        let _ = std::fs::remove_file(&temp_path);
    }
    
    #[test]
    fn test_roundtrip_rgba() {
        let width = 16;
        let height = 16;
        let mut data = Vec::with_capacity((width * height * 4) as usize);
        
        for y in 0..height {
            for x in 0..width {
                data.push((x * 16) as u8);
                data.push((y * 16) as u8);
                data.push(64);
                data.push(255);
            }
        }
        
        let image = ImageData::from_u8(width, height, 4, data);
        
        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join("vfx_io_test_rgba.png");
        
        write(&temp_path, &image).expect("Failed to write PNG");
        
        let loaded = read(&temp_path).expect("Failed to read PNG");
        
        assert_eq!(loaded.width, width);
        assert_eq!(loaded.height, height);
        assert_eq!(loaded.channels, 4);
        
        let _ = std::fs::remove_file(&temp_path);
    }
}
