//! DirectDraw Surface (DDS) format support.
//!
//! DDS is a container format for storing GPU textures with mipmaps,
//! cube maps, and compressed formats (BC1-BC7).
//!
//! # Features
//!
//! - Read uncompressed and BC-compressed DDS files
//! - Support for 2D textures, cube maps, and texture arrays
//! - Mipmap chain access
//! - Automatic BC decompression to RGBA
//!
//! # Example
//!
//! ```no_run
//! use vfx_io::dds::{read, read_info, DdsInfo};
//!
//! // Read DDS as RGBA image (decompressed)
//! let image = read("texture.dds")?;
//!
//! // Get texture info
//! let info = read_info("texture.dds")?;
//! println!("{}x{}, {} mips", info.width, info.height, info.mip_count);
//! # Ok::<(), vfx_io::IoError>(())
//! ```

use crate::{ImageData, IoError, IoResult};
use ddsfile::Dds;
use image_dds::{dds_image_format, ImageFormat, Surface};
use std::fs::File;
use std::io::{BufReader, Cursor};
use std::path::Path;

/// DDS texture format information.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DdsFormat {
    /// Uncompressed R8.
    R8,
    /// Uncompressed RG8.
    Rg8,
    /// Uncompressed RGBA8.
    Rgba8,
    /// Uncompressed BGRA8.
    Bgra8,
    /// Uncompressed RGBA16 float.
    Rgba16Float,
    /// Uncompressed RGBA32 float.
    Rgba32Float,
    /// BC1/DXT1 - RGB, 1-bit alpha.
    Bc1,
    /// BC2/DXT3 - RGBA with explicit alpha.
    Bc2,
    /// BC3/DXT5 - RGBA with interpolated alpha.
    Bc3,
    /// BC4 - Single channel.
    Bc4,
    /// BC5 - Two channel (normal maps).
    Bc5,
    /// BC6H - HDR format.
    Bc6h,
    /// BC7 - High quality RGBA.
    Bc7,
    /// Unknown format.
    Unknown,
}

impl DdsFormat {
    fn from_image_format(format: ImageFormat) -> Self {
        match format {
            ImageFormat::R8Unorm => DdsFormat::R8,
            ImageFormat::Rg8Unorm | ImageFormat::Rg8Snorm => DdsFormat::Rg8,
            ImageFormat::Rgba8Unorm | ImageFormat::Rgba8UnormSrgb => DdsFormat::Rgba8,
            ImageFormat::Bgra8Unorm | ImageFormat::Bgra8UnormSrgb => DdsFormat::Bgra8,
            ImageFormat::Rgba16Float => DdsFormat::Rgba16Float,
            ImageFormat::Rgba32Float => DdsFormat::Rgba32Float,
            ImageFormat::BC1RgbaUnorm | ImageFormat::BC1RgbaUnormSrgb => DdsFormat::Bc1,
            ImageFormat::BC2RgbaUnorm | ImageFormat::BC2RgbaUnormSrgb => DdsFormat::Bc2,
            ImageFormat::BC3RgbaUnorm | ImageFormat::BC3RgbaUnormSrgb => DdsFormat::Bc3,
            ImageFormat::BC4RUnorm | ImageFormat::BC4RSnorm => DdsFormat::Bc4,
            ImageFormat::BC5RgUnorm | ImageFormat::BC5RgSnorm => DdsFormat::Bc5,
            ImageFormat::BC6hRgbUfloat | ImageFormat::BC6hRgbSfloat => DdsFormat::Bc6h,
            ImageFormat::BC7RgbaUnorm | ImageFormat::BC7RgbaUnormSrgb => DdsFormat::Bc7,
            _ => DdsFormat::Unknown,
        }
    }

    /// Returns true if format is block-compressed.
    pub fn is_compressed(&self) -> bool {
        matches!(
            self,
            DdsFormat::Bc1
                | DdsFormat::Bc2
                | DdsFormat::Bc3
                | DdsFormat::Bc4
                | DdsFormat::Bc5
                | DdsFormat::Bc6h
                | DdsFormat::Bc7
        )
    }

    /// Returns true if format is HDR/float.
    pub fn is_hdr(&self) -> bool {
        matches!(
            self,
            DdsFormat::Rgba16Float | DdsFormat::Rgba32Float | DdsFormat::Bc6h
        )
    }
}

/// DDS texture information.
#[derive(Debug, Clone)]
pub struct DdsInfo {
    /// Texture width.
    pub width: u32,
    /// Texture height.
    pub height: u32,
    /// Texture depth (for 3D textures).
    pub depth: u32,
    /// Number of mipmap levels.
    pub mip_count: u32,
    /// Array size (1 for non-array, 6 for cube maps).
    pub array_size: u32,
    /// Pixel format.
    pub format: DdsFormat,
    /// True if this is a cube map.
    pub is_cubemap: bool,
}

/// Reads DDS texture info without fully loading pixel data.
pub fn read_info<P: AsRef<Path>>(path: P) -> IoResult<DdsInfo> {
    let file = File::open(path.as_ref())?;
    let mut reader = BufReader::new(file);

    let dds = Dds::read(&mut reader)
        .map_err(|e| IoError::DecodeError(format!("DDS read error: {e}")))?;

    dds_to_info(&dds)
}

/// Reads DDS info from memory.
pub fn read_info_from_memory(data: &[u8]) -> IoResult<DdsInfo> {
    let mut cursor = Cursor::new(data);
    let dds = Dds::read(&mut cursor)
        .map_err(|e| IoError::DecodeError(format!("DDS read error: {e}")))?;

    dds_to_info(&dds)
}

fn dds_to_info(dds: &Dds) -> IoResult<DdsInfo> {
    let header = &dds.header;
    let format = dds_image_format(dds)
        .map(DdsFormat::from_image_format)
        .unwrap_or(DdsFormat::Unknown);

    let mip_count = header.mip_map_count.unwrap_or(1);
    let depth = header.depth.unwrap_or(1);

    // Check for cube map via header10 or caps2
    let is_cubemap = dds.header10.as_ref().map_or(false, |h10| {
        h10.misc_flag.contains(ddsfile::MiscFlag::TEXTURECUBE)
    });

    let array_size = dds.header10.as_ref().map_or(1, |h10| h10.array_size);
    let array_size = if is_cubemap { 6 } else { array_size };

    Ok(DdsInfo {
        width: header.width,
        height: header.height,
        depth,
        mip_count,
        array_size,
        format,
        is_cubemap,
    })
}

/// Reads a DDS file and returns the top mip level as RGBA image.
///
/// Compressed formats are automatically decompressed.
pub fn read<P: AsRef<Path>>(path: P) -> IoResult<ImageData> {
    let file = File::open(path.as_ref())?;
    let mut reader = BufReader::new(file);

    let dds = Dds::read(&mut reader)
        .map_err(|e| IoError::DecodeError(format!("DDS read error: {e}")))?;

    decode_dds_to_image(&dds)
}

/// Reads DDS from memory buffer.
pub fn read_from_memory(data: &[u8]) -> IoResult<ImageData> {
    let mut cursor = Cursor::new(data);
    let dds = Dds::read(&mut cursor)
        .map_err(|e| IoError::DecodeError(format!("DDS read error: {e}")))?;

    decode_dds_to_image(&dds)
}

fn decode_dds_to_image(dds: &Dds) -> IoResult<ImageData> {
    let width = dds.header.width;
    let height = dds.header.height;

    // Create a surface view from the DDS
    let surface = Surface::from_dds(dds)
        .map_err(|e| IoError::DecodeError(format!("DDS surface error: {e:?}")))?;

    // Decode to RGBA f32 - returns SurfaceRgba32Float with flat Vec<f32> data
    let rgba = surface.decode_rgbaf32()
        .map_err(|e| IoError::DecodeError(format!("DDS decode error: {e:?}")))?;

    // Data is flat f32 array: [R, G, B, A, R, G, B, A, ...]
    let expected_len = (width * height * 4) as usize;
    let pixels: Vec<f32> = if rgba.data.len() >= expected_len {
        rgba.data[..expected_len].to_vec()
    } else {
        rgba.data.clone()
    };

    Ok(ImageData::from_f32(width, height, 4, pixels))
}

/// Reads all mip levels from a DDS file.
pub fn read_all_mips<P: AsRef<Path>>(path: P) -> IoResult<Vec<ImageData>> {
    let file = File::open(path.as_ref())?;
    let mut reader = BufReader::new(file);

    let dds = Dds::read(&mut reader)
        .map_err(|e| IoError::DecodeError(format!("DDS read error: {e}")))?;

    let mip_count = dds.header.mip_map_count.unwrap_or(1);
    let mut width = dds.header.width;
    let mut height = dds.header.height;

    // Create surface from DDS
    let surface = Surface::from_dds(&dds)
        .map_err(|e| IoError::DecodeError(format!("DDS surface error: {e:?}")))?;

    // Decode to RGBA f32
    let rgba = surface.decode_rgbaf32()
        .map_err(|e| IoError::DecodeError(format!("DDS decode error: {e:?}")))?;

    let mut mips = Vec::with_capacity(mip_count as usize);
    let mut offset = 0usize;

    for _mip in 0..mip_count {
        let float_count = (width * height * 4) as usize;

        // Extract this mip's data from the flat array
        let end = (offset + float_count).min(rgba.data.len());
        let pixels: Vec<f32> = rgba.data[offset..end].to_vec();

        mips.push(ImageData::from_f32(width, height, 4, pixels));

        offset += float_count;
        width = (width / 2).max(1);
        height = (height / 2).max(1);
    }

    Ok(mips)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dds_format_properties() {
        assert!(DdsFormat::Bc1.is_compressed());
        assert!(DdsFormat::Bc7.is_compressed());
        assert!(!DdsFormat::Rgba8.is_compressed());
        assert!(DdsFormat::Bc6h.is_hdr());
        assert!(DdsFormat::Rgba32Float.is_hdr());
        assert!(!DdsFormat::Bc3.is_hdr());
    }
}
