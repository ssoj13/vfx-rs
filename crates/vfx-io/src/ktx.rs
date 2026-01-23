//! KTX2 (Khronos Texture 2.0) format support.
//!
//! KTX2 is the next-generation texture container format from Khronos,
//! supporting mipmaps, cubemaps, arrays, and various compression formats.
//!
//! # Features
//!
//! - Read KTX2 file headers and metadata (key-value pairs)
//! - Support for uncompressed textures (R8, RG8, RGBA8, RGBA16F, RGBA32F)
//! - Mipmap chain access
//!
//! # Limitations
//!
//! - BC-compressed textures (BC1-BC7) not supported - use DDS format instead
//! - Basis Universal transcoding requires external tooling (ktx2-rw, basisu)
//! - ASTC/ETC decompression not yet implemented
//! - Supercompressed textures (zstd, BasisLZ) not supported
//!
//! # Example
//!
//! ```no_run
//! use vfx_io::ktx::{read, read_info};
//!
//! // Read KTX2 texture as RGBA image
//! let image = read("texture.ktx2")?;
//!
//! // Get texture metadata
//! let info = read_info("texture.ktx2")?;
//! println!("{}x{}, format: {:?}", info.width, info.height, info.format);
//! # Ok::<(), vfx_io::IoError>(())
//! ```

use crate::{ImageData, IoError, IoResult};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

/// KTX2 texture format information.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KtxFormat {
    /// Uncompressed R8.
    R8,
    /// Uncompressed RG8.
    Rg8,
    /// Uncompressed RGBA8.
    Rgba8,
    /// Uncompressed RGBA16 float.
    Rgba16Float,
    /// Uncompressed RGBA32 float.
    Rgba32Float,
    /// BC1/DXT1 (RGB, 1-bit alpha).
    Bc1,
    /// BC2/DXT3 (RGBA with explicit alpha).
    Bc2,
    /// BC3/DXT5 (RGBA with interpolated alpha).
    Bc3,
    /// BC4 (single channel).
    Bc4,
    /// BC5 (two channels, normal maps).
    Bc5,
    /// BC6H (HDR).
    Bc6h,
    /// BC7 (high quality RGBA).
    Bc7,
    /// ETC1 (mobile).
    Etc1,
    /// ETC2 (mobile).
    Etc2,
    /// ASTC (adaptive scalable).
    Astc,
    /// Basis Universal ETC1S.
    BasisEtc1s,
    /// Basis Universal UASTC.
    BasisUastc,
    /// Unknown format.
    Unknown(u32),
}

impl KtxFormat {
    /// Create format from VkFormat value.
    fn from_vk_format(vk_format: u32) -> Self {
        // VkFormat enum values from Vulkan spec
        match vk_format {
            9 => KtxFormat::R8,           // VK_FORMAT_R8_UNORM
            16 => KtxFormat::Rg8,         // VK_FORMAT_R8G8_UNORM
            37 => KtxFormat::Rgba8,       // VK_FORMAT_R8G8B8A8_UNORM
            43 => KtxFormat::Rgba8,       // VK_FORMAT_R8G8B8A8_SRGB
            97 => KtxFormat::Rgba16Float, // VK_FORMAT_R16G16B16A16_SFLOAT
            109 => KtxFormat::Rgba32Float, // VK_FORMAT_R32G32B32A32_SFLOAT
            // BC formats
            131 | 132 => KtxFormat::Bc1, // VK_FORMAT_BC1_RGB[A]_UNORM_BLOCK
            135 | 136 => KtxFormat::Bc2, // VK_FORMAT_BC2_UNORM_BLOCK
            139 | 140 => KtxFormat::Bc3, // VK_FORMAT_BC3_UNORM_BLOCK
            143 | 144 => KtxFormat::Bc4, // VK_FORMAT_BC4_UNORM_BLOCK
            147 | 148 => KtxFormat::Bc5, // VK_FORMAT_BC5_UNORM_BLOCK
            151 | 152 => KtxFormat::Bc6h, // VK_FORMAT_BC6H_UFLOAT_BLOCK
            155 | 156 => KtxFormat::Bc7, // VK_FORMAT_BC7_UNORM_BLOCK
            // ETC2
            147..=160 => KtxFormat::Etc2,
            // ASTC (various block sizes)
            157..=184 => KtxFormat::Astc,
            // Unknown
            other => KtxFormat::Unknown(other),
        }
    }

    /// Returns true if format is block-compressed.
    pub fn is_compressed(&self) -> bool {
        matches!(
            self,
            KtxFormat::Bc1
                | KtxFormat::Bc2
                | KtxFormat::Bc3
                | KtxFormat::Bc4
                | KtxFormat::Bc5
                | KtxFormat::Bc6h
                | KtxFormat::Bc7
                | KtxFormat::Etc1
                | KtxFormat::Etc2
                | KtxFormat::Astc
                | KtxFormat::BasisEtc1s
                | KtxFormat::BasisUastc
        )
    }

    /// Returns true if format is HDR.
    pub fn is_hdr(&self) -> bool {
        matches!(
            self,
            KtxFormat::Rgba16Float | KtxFormat::Rgba32Float | KtxFormat::Bc6h
        )
    }

    /// Returns true if format requires Basis Universal transcoding.
    pub fn requires_basis_transcoding(&self) -> bool {
        matches!(self, KtxFormat::BasisEtc1s | KtxFormat::BasisUastc)
    }
}

/// KTX2 texture information.
#[derive(Debug, Clone)]
pub struct KtxInfo {
    /// Texture width.
    pub width: u32,
    /// Texture height.
    pub height: u32,
    /// Texture depth (1 for 2D textures).
    pub depth: u32,
    /// Number of mipmap levels.
    pub mip_count: u32,
    /// Array size (1 for non-arrays, 6 for cubemaps).
    pub array_size: u32,
    /// Number of faces (1 for 2D, 6 for cubemap).
    pub face_count: u32,
    /// Pixel format.
    pub format: KtxFormat,
    /// True if texture uses supercompression (zstd, zlib, BasisLZ).
    pub is_supercompressed: bool,
    /// Metadata key-value pairs.
    pub metadata: Vec<(String, Vec<u8>)>,
}

/// KTX2 file header (spec v2.0).
#[repr(C)]
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
struct Ktx2Header {
    identifier: [u8; 12],
    vk_format: u32,
    type_size: u32,
    pixel_width: u32,
    pixel_height: u32,
    pixel_depth: u32,
    layer_count: u32,
    face_count: u32,
    level_count: u32,
    supercompression_scheme: u32,
    // Index data follows (not included in struct)
}

const KTX2_IDENTIFIER: [u8; 12] = [
    0xAB, 0x4B, 0x54, 0x58, 0x20, 0x32, 0x30, 0xBB, 0x0D, 0x0A, 0x1A, 0x0A,
];

/// Reads KTX2 texture info without fully loading pixel data.
pub fn read_info<P: AsRef<Path>>(path: P) -> IoResult<KtxInfo> {
    // Read file to memory to enable metadata parsing (needs full buffer access)
    let data = std::fs::read(path.as_ref())?;
    read_info_from_memory(&data)
}

/// Reads KTX2 info from memory.
pub fn read_info_from_memory(data: &[u8]) -> IoResult<KtxInfo> {
    read_info_impl_with_metadata(data)
}

/// Parses KTX2 info from a full data buffer (supports metadata parsing).
fn read_info_impl_with_metadata(data: &[u8]) -> IoResult<KtxInfo> {
    if data.len() < 80 {
        return Err(IoError::Format("KTX2 file too small for header".into()));
    }

    // Check magic bytes
    if &data[0..12] != &KTX2_IDENTIFIER {
        return Err(IoError::Format("Not a valid KTX2 file".into()));
    }

    // Parse header fields (little-endian)
    let vk_format = u32::from_le_bytes([data[12], data[13], data[14], data[15]]);
    let pixel_width = u32::from_le_bytes([data[20], data[21], data[22], data[23]]);
    let pixel_height = u32::from_le_bytes([data[24], data[25], data[26], data[27]]);
    let pixel_depth = u32::from_le_bytes([data[28], data[29], data[30], data[31]]);
    let layer_count = u32::from_le_bytes([data[32], data[33], data[34], data[35]]);
    let face_count = u32::from_le_bytes([data[36], data[37], data[38], data[39]]);
    let level_count = u32::from_le_bytes([data[40], data[41], data[42], data[43]]);
    let supercompression_scheme = u32::from_le_bytes([data[44], data[45], data[46], data[47]]);

    // KV data offset and length (bytes 56-63)
    let kvd_offset = u32::from_le_bytes([data[56], data[57], data[58], data[59]]) as usize;
    let kvd_length = u32::from_le_bytes([data[60], data[61], data[62], data[63]]) as usize;

    // Determine format
    let mut format = KtxFormat::from_vk_format(vk_format);

    // Check for Basis supercompression
    if supercompression_scheme == 1 {
        format = KtxFormat::BasisEtc1s;
    } else if vk_format == 0 {
        format = KtxFormat::BasisUastc;
    }

    // Parse metadata from Key/Value Data section
    let metadata = parse_ktx2_metadata(data, kvd_offset, kvd_length);

    Ok(KtxInfo {
        width: pixel_width,
        height: pixel_height.max(1),
        depth: pixel_depth.max(1),
        mip_count: level_count.max(1),
        array_size: layer_count.max(1),
        face_count: face_count.max(1),
        format,
        is_supercompressed: supercompression_scheme != 0,
        metadata,
    })
}

/// Parses KTX2 Key/Value Data section.
/// Format: repeated [keyAndValueByteLength: u32, keyAndValue: bytes, padding]
fn parse_ktx2_metadata(data: &[u8], offset: usize, length: usize) -> Vec<(String, Vec<u8>)> {
    let mut metadata = Vec::new();

    if offset == 0 || length == 0 || offset + length > data.len() {
        return metadata; // No metadata or invalid range
    }

    let kvd = &data[offset..offset + length];
    let mut pos = 0;

    while pos + 4 <= kvd.len() {
        // Read keyAndValueByteLength
        let kv_len = u32::from_le_bytes([
            kvd[pos],
            kvd[pos + 1],
            kvd[pos + 2],
            kvd[pos + 3],
        ]) as usize;
        pos += 4;

        if kv_len == 0 || pos + kv_len > kvd.len() {
            break; // Invalid or truncated entry
        }

        // Key is null-terminated string, value follows
        let kv_data = &kvd[pos..pos + kv_len];
        if let Some(null_pos) = kv_data.iter().position(|&b| b == 0) {
            let key = String::from_utf8_lossy(&kv_data[..null_pos]).into_owned();
            let value = kv_data[null_pos + 1..].to_vec();
            metadata.push((key, value));
        }

        // Advance to next entry (4-byte aligned)
        pos += kv_len;
        pos = (pos + 3) & !3; // Round up to 4-byte boundary
    }

    metadata
}

/// Reads a KTX2 file and returns the top mip level as RGBA image.
///
/// # Supported Formats
///
/// - Uncompressed: R8, RG8, RGBA8, RGBA16F, RGBA32F
///
/// # Errors
///
/// Returns error for:
/// - BC-compressed formats (BC1-BC7) - use DDS format instead
/// - Supercompressed textures (zstd, BasisLZ, etc.)
/// - Basis Universal formats (ETC1S, UASTC) - requires external tooling
/// - ASTC/ETC formats (not yet implemented)
/// - Invalid or corrupted files
pub fn read<P: AsRef<Path>>(path: P) -> IoResult<ImageData> {
    let data = std::fs::read(path.as_ref())?;
    read_from_memory(&data)
}

/// Reads KTX2 from memory buffer.
pub fn read_from_memory(data: &[u8]) -> IoResult<ImageData> {
    let info = read_info_from_memory(data)?;

    if info.is_supercompressed {
        return Err(IoError::UnsupportedFeature(
            "KTX2 supercompression requires additional tooling (zstd, BasisLZ)".into(),
        ));
    }

    if info.format.requires_basis_transcoding() {
        return Err(IoError::UnsupportedFeature(
            "Basis Universal transcoding requires ktx2-rw or basisu tooling".into(),
        ));
    }

    // For BC formats, we could potentially convert to DDS and use image_dds
    // For now, support only uncompressed formats
    match info.format {
        KtxFormat::Rgba8 => decode_rgba8(data, &info),
        KtxFormat::Rgba16Float => decode_rgba16f(data, &info),
        KtxFormat::Rgba32Float => decode_rgba32f(data, &info),
        KtxFormat::R8 => decode_r8(data, &info),
        KtxFormat::Rg8 => decode_rg8(data, &info),
        KtxFormat::Bc1
        | KtxFormat::Bc2
        | KtxFormat::Bc3
        | KtxFormat::Bc4
        | KtxFormat::Bc5
        | KtxFormat::Bc6h
        | KtxFormat::Bc7 => Err(IoError::UnsupportedFeature(
            "BC-compressed KTX2 decode not yet implemented. Use DDS format or external tooling."
                .into(),
        )),
        KtxFormat::Etc1 | KtxFormat::Etc2 | KtxFormat::Astc => Err(IoError::UnsupportedFeature(
            "ETC/ASTC decompression not yet implemented".into(),
        )),
        KtxFormat::Unknown(vk) => {
            Err(IoError::UnsupportedFormat(format!("Unknown VkFormat: {vk}")))
        }
        _ => Err(IoError::UnsupportedFormat(format!(
            "Unsupported KTX2 format: {:?}",
            info.format
        ))),
    }
}

/// Get offset to level data in KTX2 file.
fn get_level_offset(data: &[u8], level: u32, info: &KtxInfo) -> IoResult<(usize, usize)> {
    // KTX2 header is 80 bytes
    // Level index follows: level_count * 24 bytes (byteOffset, byteLength, uncompressedByteLength)
    let level_index_offset = 80;
    let level_entry_size = 24;

    if level >= info.mip_count {
        return Err(IoError::InvalidFile("Mip level out of range".into()));
    }

    let entry_offset = level_index_offset + (level as usize) * level_entry_size;
    if entry_offset + level_entry_size > data.len() {
        return Err(IoError::InvalidFile("Truncated level index".into()));
    }

    let byte_offset = u64::from_le_bytes([
        data[entry_offset],
        data[entry_offset + 1],
        data[entry_offset + 2],
        data[entry_offset + 3],
        data[entry_offset + 4],
        data[entry_offset + 5],
        data[entry_offset + 6],
        data[entry_offset + 7],
    ]) as usize;

    let byte_length = u64::from_le_bytes([
        data[entry_offset + 8],
        data[entry_offset + 9],
        data[entry_offset + 10],
        data[entry_offset + 11],
        data[entry_offset + 12],
        data[entry_offset + 13],
        data[entry_offset + 14],
        data[entry_offset + 15],
    ]) as usize;

    Ok((byte_offset, byte_length))
}

fn decode_rgba8(data: &[u8], info: &KtxInfo) -> IoResult<ImageData> {
    let (offset, length) = get_level_offset(data, 0, info)?;

    if offset + length > data.len() {
        return Err(IoError::InvalidFile("Truncated pixel data".into()));
    }

    let pixel_data = &data[offset..offset + length];
    let expected = (info.width * info.height * 4) as usize;

    if pixel_data.len() < expected {
        return Err(IoError::InvalidFile("Insufficient pixel data".into()));
    }

    // Convert u8 RGBA to f32 RGBA
    let pixels: Vec<f32> = pixel_data
        .iter()
        .take(expected)
        .map(|&b| b as f32 / 255.0)
        .collect();

    Ok(ImageData::from_f32(info.width, info.height, 4, pixels))
}

fn decode_rgba16f(data: &[u8], info: &KtxInfo) -> IoResult<ImageData> {
    let (offset, length) = get_level_offset(data, 0, info)?;

    if offset + length > data.len() {
        return Err(IoError::InvalidFile("Truncated pixel data".into()));
    }

    let pixel_data = &data[offset..offset + length];
    let pixel_count = (info.width * info.height * 4) as usize;
    let expected_bytes = pixel_count * 2;

    if pixel_data.len() < expected_bytes {
        return Err(IoError::InvalidFile("Insufficient pixel data".into()));
    }

    // Convert f16 to f32
    let mut pixels = Vec::with_capacity(pixel_count);
    for i in 0..pixel_count {
        let bytes = [pixel_data[i * 2], pixel_data[i * 2 + 1]];
        let half = half::f16::from_le_bytes(bytes);
        pixels.push(half.to_f32());
    }

    Ok(ImageData::from_f32(info.width, info.height, 4, pixels))
}

fn decode_rgba32f(data: &[u8], info: &KtxInfo) -> IoResult<ImageData> {
    let (offset, length) = get_level_offset(data, 0, info)?;

    if offset + length > data.len() {
        return Err(IoError::InvalidFile("Truncated pixel data".into()));
    }

    let pixel_data = &data[offset..offset + length];
    let pixel_count = (info.width * info.height * 4) as usize;
    let expected_bytes = pixel_count * 4;

    if pixel_data.len() < expected_bytes {
        return Err(IoError::InvalidFile("Insufficient pixel data".into()));
    }

    // Read f32 directly
    let mut pixels = Vec::with_capacity(pixel_count);
    for i in 0..pixel_count {
        let bytes = [
            pixel_data[i * 4],
            pixel_data[i * 4 + 1],
            pixel_data[i * 4 + 2],
            pixel_data[i * 4 + 3],
        ];
        pixels.push(f32::from_le_bytes(bytes));
    }

    Ok(ImageData::from_f32(info.width, info.height, 4, pixels))
}

fn decode_r8(data: &[u8], info: &KtxInfo) -> IoResult<ImageData> {
    let (offset, length) = get_level_offset(data, 0, info)?;

    if offset + length > data.len() {
        return Err(IoError::InvalidFile("Truncated pixel data".into()));
    }

    let pixel_data = &data[offset..offset + length];
    let expected = (info.width * info.height) as usize;

    if pixel_data.len() < expected {
        return Err(IoError::InvalidFile("Insufficient pixel data".into()));
    }

    // Convert R8 to RGBA
    let mut pixels = Vec::with_capacity(expected * 4);
    for &r in pixel_data.iter().take(expected) {
        let v = r as f32 / 255.0;
        pixels.extend_from_slice(&[v, v, v, 1.0]);
    }

    Ok(ImageData::from_f32(info.width, info.height, 4, pixels))
}

fn decode_rg8(data: &[u8], info: &KtxInfo) -> IoResult<ImageData> {
    let (offset, length) = get_level_offset(data, 0, info)?;

    if offset + length > data.len() {
        return Err(IoError::InvalidFile("Truncated pixel data".into()));
    }

    let pixel_data = &data[offset..offset + length];
    let pixel_count = (info.width * info.height) as usize;
    let expected = pixel_count * 2;

    if pixel_data.len() < expected {
        return Err(IoError::InvalidFile("Insufficient pixel data".into()));
    }

    // Convert RG8 to RGBA
    let mut pixels = Vec::with_capacity(pixel_count * 4);
    for i in 0..pixel_count {
        let r = pixel_data[i * 2] as f32 / 255.0;
        let g = pixel_data[i * 2 + 1] as f32 / 255.0;
        pixels.extend_from_slice(&[r, g, 0.0, 1.0]);
    }

    Ok(ImageData::from_f32(info.width, info.height, 4, pixels))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ktx_format_properties() {
        assert!(KtxFormat::Bc1.is_compressed());
        assert!(KtxFormat::Bc7.is_compressed());
        assert!(!KtxFormat::Rgba8.is_compressed());
        assert!(KtxFormat::Bc6h.is_hdr());
        assert!(KtxFormat::Rgba32Float.is_hdr());
        assert!(!KtxFormat::Bc3.is_hdr());
        assert!(KtxFormat::BasisEtc1s.requires_basis_transcoding());
        assert!(!KtxFormat::Bc7.requires_basis_transcoding());
    }

    #[test]
    fn test_ktx2_identifier() {
        assert_eq!(KTX2_IDENTIFIER[0], 0xAB);
        assert_eq!(KTX2_IDENTIFIER[1], 0x4B); // 'K'
        assert_eq!(KTX2_IDENTIFIER[2], 0x54); // 'T'
        assert_eq!(KTX2_IDENTIFIER[3], 0x58); // 'X'
    }

    #[test]
    fn test_vk_format_mapping() {
        assert_eq!(KtxFormat::from_vk_format(37), KtxFormat::Rgba8);
        assert_eq!(KtxFormat::from_vk_format(109), KtxFormat::Rgba32Float);
        assert_eq!(KtxFormat::from_vk_format(131), KtxFormat::Bc1);
        assert_eq!(KtxFormat::from_vk_format(155), KtxFormat::Bc7);
    }
}
