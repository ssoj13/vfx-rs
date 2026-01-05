//! JPEG 2000 format support (read-only).
//!
//! Read JP2/J2K images via the `jpeg2k` crate (OpenJPEG bindings).
//! JPEG 2000 supports:
//! - Lossless and lossy compression
//! - 8-16 bit depth per component
//! - Multiple components (RGB, RGBA, grayscale)
//! - Tiled encoding for large images
//! - Progressive decoding (resolution levels)
//!
//! # Note
//! 
//! Writing JP2 from scratch is not supported by the underlying jpeg2k crate.
//! The crate only supports transcoding existing JP2 files.
//!
//! # Example
//!
//! ```ignore
//! use vfx_io::jp2;
//!
//! let img = jp2::read("input.jp2")?;
//! println!("{}x{}, {} channels", img.width, img.height, img.channels);
//! ```

use std::path::Path;

use jpeg2k::Image as J2kImage;

use crate::{ImageData, IoError, IoResult, PixelData, PixelFormat, Metadata};

/// Reads a JPEG 2000 image from file.
pub fn read<P: AsRef<Path>>(path: P) -> IoResult<ImageData> {
    let img = J2kImage::from_file(path.as_ref())
        .map_err(|e| IoError::DecodeError(format!("JP2 decode: {}", e)))?;
    
    j2k_to_image_data(&img)
}

/// Reads a JPEG 2000 image from bytes.
pub fn read_bytes(data: &[u8]) -> IoResult<ImageData> {
    let img = J2kImage::from_bytes(data)
        .map_err(|e| IoError::DecodeError(format!("JP2 decode: {}", e)))?;
    
    j2k_to_image_data(&img)
}

/// Converts jpeg2k Image to ImageData.
fn j2k_to_image_data(img: &J2kImage) -> IoResult<ImageData> {
    let width = img.width();
    let height = img.height();
    let num_components = img.num_components();
    
    // Get pixels with default alpha = 255
    let pixels = img.get_pixels(Some(255))
        .map_err(|e| IoError::DecodeError(format!("JP2 pixel extraction: {}", e)))?;
    
    // Determine bit depth from first component
    let components = img.components();
    let precision = if !components.is_empty() {
        components[0].precision()
    } else {
        8
    };
    
    let channels = num_components.min(4) as u32;
    
    if precision <= 8 {
        // 8-bit data
        let data: Vec<u8> = pixels.data.iter().map(|&v| v as u8).collect();
        Ok(ImageData {
            width,
            height,
            channels,
            format: PixelFormat::U8,
            data: PixelData::U8(data),
            metadata: Metadata::default(),
        })
    } else {
        // 16-bit data
        let data: Vec<u16> = pixels.data.iter().map(|&v| v as u16).collect();
        Ok(ImageData {
            width,
            height,
            channels,
            format: PixelFormat::U16,
            data: PixelData::U16(data),
            metadata: Metadata::default(),
        })
    }
}

#[cfg(test)]
mod tests {
    // JP2 tests would require sample files
    // Testing with real files in integration tests
}
