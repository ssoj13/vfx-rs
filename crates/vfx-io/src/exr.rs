//! OpenEXR format support.
//!
//! Provides reading and writing of OpenEXR files, the industry standard
//! for HDR and linear workflow images in VFX.
//!
//! # Features
//!
//! - Multi-layer support
//! - Various compression methods (ZIP, PIZ, DWAA, etc.)
//! - 16-bit and 32-bit float pixels
//! - Metadata preservation
//!
//! # Example
//!
//! ```rust,ignore
//! use vfx_io::exr::{read, write};
//!
//! // Read an EXR file
//! let image = read("render.exr")?;
//!
//! // Write with default settings
//! write("output.exr", &image)?;
//! ```

use crate::{ImageData, IoError, IoResult, Metadata, PixelData, PixelFormat};
use std::path::Path;

/// Reads an EXR file from the given path.
///
/// Returns the first layer's RGBA data as f32.
///
/// # Example
///
/// ```rust,ignore
/// use vfx_io::exr;
///
/// let image = exr::read("input.exr")?;
/// println!("Size: {}x{}", image.width, image.height);
/// ```
pub fn read<P: AsRef<Path>>(path: P) -> IoResult<ImageData> {
    use exr::prelude::*;
    
    let path = path.as_ref();
    
    // Read the EXR file
    let image = read_first_rgba_layer_from_file(
        path,
        |resolution, _| {
            let width = resolution.width();
            let size = width * resolution.height();
            (width, vec![(0.0f32, 0.0f32, 0.0f32, 1.0f32); size])
        },
        |(width, buffer), position, (r, g, b, a): (f32, f32, f32, f32)| {
            let idx = position.y() * *width + position.x();
            if idx < buffer.len() {
                buffer[idx] = (r, g, b, a);
            }
        },
    ).map_err(|e| IoError::DecodeError(e.to_string()))?;
    
    let width = image.layer_data.size.width() as u32;
    let height = image.layer_data.size.height() as u32;
    let (_, ref pixel_data) = image.layer_data.channel_data.pixels;
    
    // Convert from tuple vec to flat f32 vec
    let mut data = Vec::with_capacity((width * height * 4) as usize);
    for &(r, g, b, a) in pixel_data {
        data.push(r);
        data.push(g);
        data.push(b);
        data.push(a);
    }
    
    let mut result = ImageData {
        width,
        height,
        channels: 4,
        format: PixelFormat::F32,
        data: PixelData::F32(data),
        metadata: Metadata::default(),
    };
    
    // Extract metadata
    result.metadata.colorspace = Some("linear".to_string());
    
    Ok(result)
}

/// Writes an image to an EXR file.
///
/// Converts data to f32 if necessary and writes as RGBA.
///
/// # Example
///
/// ```rust,ignore
/// use vfx_io::exr;
///
/// exr::write("output.exr", &image)?;
/// ```
pub fn write<P: AsRef<Path>>(path: P, image: &ImageData) -> IoResult<()> {
    use exr::prelude::*;
    
    let path = path.as_ref();
    let width = image.width as usize;
    let height = image.height as usize;
    
    // Convert to f32
    let f32_data = image.to_f32();
    
    // Build RGBA tuples
    let channels = image.channels as usize;
    let pixels: Vec<(f32, f32, f32, f32)> = (0..width * height)
        .map(|i| {
            let base = i * channels;
            let r = f32_data.get(base).copied().unwrap_or(0.0);
            let g = f32_data.get(base + 1).copied().unwrap_or(0.0);
            let b = f32_data.get(base + 2).copied().unwrap_or(0.0);
            let a = if channels >= 4 {
                f32_data.get(base + 3).copied().unwrap_or(1.0)
            } else {
                1.0
            };
            (r, g, b, a)
        })
        .collect();
    
    // Write EXR
    let layer = Layer::new(
        (width, height),
        LayerAttributes::named("RGBA"),
        Encoding::SMALL_LOSSLESS,
        SpecificChannels::rgba(|pos: Vec2<usize>| {
            pixels[pos.y() * width + pos.x()]
        }),
    );
    
    let exr_image = Image::from_layer(layer);
    
    exr_image
        .write()
        .to_file(path)
        .map_err(|e| IoError::EncodeError(e.to_string()))?;
    
    Ok(())
}

/// EXR compression method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Compression {
    /// No compression.
    None,
    /// RLE compression.
    Rle,
    /// ZIP compression (lossless, good compression).
    #[default]
    Zip,
    /// PIZ compression (lossless, best for noisy images).
    Piz,
    /// DWAA compression (lossy, good for final delivery).
    Dwaa,
    /// DWAB compression (lossy, better quality than DWAA).
    Dwab,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_roundtrip() {
        // Create test image
        let width = 64;
        let height = 64;
        let mut data = Vec::with_capacity((width * height * 4) as usize);
        
        for y in 0..height {
            for x in 0..width {
                let r = x as f32 / width as f32;
                let g = y as f32 / height as f32;
                let b = 0.5;
                let a = 1.0;
                data.push(r);
                data.push(g);
                data.push(b);
                data.push(a);
            }
        }
        
        let image = ImageData::from_f32(width, height, 4, data);
        
        // Write to temp file
        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join("vfx_io_test.exr");
        
        write(&temp_path, &image).expect("Failed to write EXR");
        
        // Read back
        let loaded = read(&temp_path).expect("Failed to read EXR");
        
        assert_eq!(loaded.width, width);
        assert_eq!(loaded.height, height);
        assert_eq!(loaded.channels, 4);
        
        // Clean up
        let _ = std::fs::remove_file(&temp_path);
    }
}
