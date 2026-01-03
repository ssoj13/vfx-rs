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

use crate::{AttrValue, ImageData, IoError, IoResult, Metadata, PixelData, PixelFormat};
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
    let output_info = reader.next_frame(&mut buf)
        .map_err(|e: png::DecodingError| IoError::DecodeError(e.to_string()))?;
    let info = reader.info();
    
    let width = output_info.width;
    let height = output_info.height;
    
    let (channels, format, data) = match (output_info.color_type, output_info.bit_depth) {
        (png::ColorType::Rgb, png::BitDepth::Eight) => {
            (3, PixelFormat::U8, PixelData::U8(buf[..output_info.buffer_size()].to_vec()))
        }
        (png::ColorType::Rgba, png::BitDepth::Eight) => {
            (4, PixelFormat::U8, PixelData::U8(buf[..output_info.buffer_size()].to_vec()))
        }
        (png::ColorType::Rgb, png::BitDepth::Sixteen) => {
            let u16_data = bytes_to_u16(&buf[..output_info.buffer_size()]);
            (3, PixelFormat::U16, PixelData::U16(u16_data))
        }
        (png::ColorType::Rgba, png::BitDepth::Sixteen) => {
            let u16_data = bytes_to_u16(&buf[..output_info.buffer_size()]);
            (4, PixelFormat::U16, PixelData::U16(u16_data))
        }
        (png::ColorType::Grayscale, png::BitDepth::Eight) => {
            // Convert grayscale to RGB
            let rgb: Vec<u8> = buf[..output_info.buffer_size()]
                .iter()
                .flat_map(|&g| [g, g, g])
                .collect();
            (3, PixelFormat::U8, PixelData::U8(rgb))
        }
        (png::ColorType::GrayscaleAlpha, png::BitDepth::Eight) => {
            // Convert grayscale+alpha to RGBA
            let rgba: Vec<u8> = buf[..output_info.buffer_size()]
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
    metadata
        .attrs
        .set("ImageWidth", AttrValue::UInt(width));
    metadata
        .attrs
        .set("ImageHeight", AttrValue::UInt(height));
    metadata
        .attrs
        .set("ColorType", AttrValue::Str(format!("{:?}", info.color_type)));
    metadata
        .attrs
        .set("BitDepth", AttrValue::UInt(bit_depth_to_u32(info.bit_depth)));

    if let Some(gamma) = info.gamma() {
        let gamma = gamma.into_value();
        metadata.gamma = Some(gamma);
        metadata.attrs.set("Gamma", AttrValue::Float(gamma));
    }

    if let Some(dim) = info.pixel_dims {
        if dim.xppu > 0 && dim.yppu > 0 {
            match dim.unit {
                png::Unit::Meter => {
                    let x_dpi = (dim.xppu as f64 * 0.0254) as f32;
                    let y_dpi = (dim.yppu as f64 * 0.0254) as f32;
                    metadata.attrs.set("XResolution", AttrValue::Float(x_dpi));
                    metadata.attrs.set("YResolution", AttrValue::Float(y_dpi));
                    metadata
                        .attrs
                        .set("ResolutionUnit", AttrValue::Str("dpi".to_string()));
                    if (x_dpi - y_dpi).abs() < f32::EPSILON {
                        metadata.dpi = Some(x_dpi);
                    }
                }
                png::Unit::Unspecified => {
                    metadata.attrs.set(
                        "PixelAspectRatio",
                        AttrValue::Str(format!("{}:{}", dim.xppu, dim.yppu)),
                    );
                }
            }
        }
    }

    if let Some(intent) = info.srgb {
        metadata
            .attrs
            .set("sRGBRendering", AttrValue::Str(format!("{:?}", intent)));
    }

    if let Some(icc) = info.icc_profile.as_deref() {
        metadata
            .attrs
            .set("ICCProfileSize", AttrValue::UInt(icc.len() as u32));
        if icc.len() >= 20 {
            if let Ok(space) = std::str::from_utf8(&icc[16..20]) {
                metadata
                    .attrs
                    .set("ICCColorSpace", AttrValue::Str(space.trim().to_string()));
            }
        }
    }

    if let Some(exif) = info.exif_metadata.as_deref() {
        metadata
            .attrs
            .set("ExifSize", AttrValue::UInt(exif.len() as u32));
        if exif.len() <= 65536 {
            metadata
                .attrs
                .set("ExifData", AttrValue::Bytes(exif.to_vec()));
        }
    }

    for text in &info.uncompressed_latin1_text {
        let key = format!("Text:{}", text.keyword);
        metadata.attrs.set(key, AttrValue::Str(text.text.clone()));
    }

    for text in info.compressed_latin1_text.clone() {
        if let Ok(value) = text.get_text() {
            let key = format!("Text:{}", text.keyword);
            metadata.attrs.set(key, AttrValue::Str(value));
        }
    }

    for text in info.utf8_text.clone() {
        if let Ok(value) = text.get_text() {
            let key = format!("Text:{}", text.keyword);
            metadata.attrs.set(key, AttrValue::Str(value));
        }
    }
    
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

fn bit_depth_to_u32(depth: png::BitDepth) -> u32 {
    match depth {
        png::BitDepth::One => 1,
        png::BitDepth::Two => 2,
        png::BitDepth::Four => 4,
        png::BitDepth::Eight => 8,
        png::BitDepth::Sixteen => 16,
    }
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
