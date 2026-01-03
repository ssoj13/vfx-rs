//! DPX (Digital Picture Exchange) format support.
//!
//! Industry standard for film scanning and digital intermediate work.
//! Commonly used in VFX pipelines for frame sequences.
//!
//! # Features
//!
//! - 10-bit, 12-bit, 16-bit RGB support
//! - Film metadata (frame rate, timecode, etc.)
//! - Big-endian and little-endian support
//!
//! # Example
//!
//! ```rust,ignore
//! use vfx_io::dpx;
//!
//! let image = dpx::read("frame.0001.dpx")?;
//! dpx::write("output.0001.dpx", &image)?;
//! ```

use crate::{AttrValue, ImageData, IoError, IoResult, Metadata, PixelData, PixelFormat};
use byteorder::{BigEndian, ByteOrder, LittleEndian, ReadBytesExt};
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::Path;

/// DPX magic number (big-endian).
const DPX_MAGIC_BE: u32 = 0x53445058; // "SDPX"
/// DPX magic number (little-endian).
const DPX_MAGIC_LE: u32 = 0x58504453; // "XPDS"

/// DPX file header.
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct DpxHeader {
    magic: u32,
    image_offset: u32,
    file_size: u32,
    width: u32,
    height: u32,
    bit_depth: u8,
    is_big_endian: bool,
}

impl DpxHeader {
    fn read<R: Read + Seek>(reader: &mut R) -> IoResult<Self> {
        let mut magic_bytes = [0u8; 4];
        reader.read_exact(&mut magic_bytes)
            .map_err(|e| IoError::DecodeError(e.to_string()))?;

        let magic = u32::from_be_bytes(magic_bytes);
        let is_big_endian = match magic {
            DPX_MAGIC_BE => true,
            DPX_MAGIC_LE => false,
            _ => return Err(IoError::DecodeError("invalid DPX magic number".into())),
        };

        let image_offset = read_u32(reader, is_big_endian)?;

        // Skip version
        reader.seek(SeekFrom::Current(8))
            .map_err(|e| IoError::DecodeError(e.to_string()))?;

        let file_size = read_u32(reader, is_big_endian)?;

        // Skip to image element (offset 768)
        reader.seek(SeekFrom::Start(772))
            .map_err(|e| IoError::DecodeError(e.to_string()))?;

        let width = read_u32(reader, is_big_endian)?;
        let height = read_u32(reader, is_big_endian)?;

        // Skip to bit depth (offset 783)
        reader.seek(SeekFrom::Start(783))
            .map_err(|e| IoError::DecodeError(e.to_string()))?;

        let bit_depth = reader.read_u8()
            .map_err(|e| IoError::DecodeError(e.to_string()))?;

        Ok(Self {
            magic,
            image_offset,
            file_size,
            width,
            height,
            bit_depth,
            is_big_endian,
        })
    }
}

fn read_u32<R: Read>(reader: &mut R, big_endian: bool) -> IoResult<u32> {
    let mut buf = [0u8; 4];
    reader.read_exact(&mut buf)
        .map_err(|e| IoError::DecodeError(e.to_string()))?;
    Ok(if big_endian {
        BigEndian::read_u32(&buf)
    } else {
        LittleEndian::read_u32(&buf)
    })
}

fn read_u16<R: Read>(reader: &mut R, big_endian: bool) -> IoResult<u16> {
    let mut buf = [0u8; 2];
    reader.read_exact(&mut buf)
        .map_err(|e| IoError::DecodeError(e.to_string()))?;
    Ok(if big_endian {
        BigEndian::read_u16(&buf)
    } else {
        LittleEndian::read_u16(&buf)
    })
}

/// Reads a DPX file from the given path.
pub fn read<P: AsRef<Path>>(path: P) -> IoResult<ImageData> {
    let file = File::open(path.as_ref())?;
    let mut reader = BufReader::new(file);

    let header = DpxHeader::read(&mut reader)?;

    reader.seek(SeekFrom::Start(header.image_offset as u64))
        .map_err(|e| IoError::DecodeError(e.to_string()))?;

    let pixel_count = (header.width * header.height) as usize;
    let channels = 3u32;

    let data = match header.bit_depth {
        10 => read_10bit_packed(&mut reader, pixel_count, header.is_big_endian)?,
        12 => read_12bit(&mut reader, pixel_count, header.is_big_endian)?,
        16 => read_16bit(&mut reader, pixel_count, header.is_big_endian)?,
        8 => read_8bit(&mut reader, pixel_count)?,
        _ => {
            return Err(IoError::DecodeError(format!(
                "unsupported DPX bit depth: {}",
                header.bit_depth
            )));
        }
    };

    let mut metadata = Metadata::default();
    metadata.colorspace = Some("log".to_string());
    metadata
        .attrs
        .set("ImageWidth", AttrValue::UInt(header.width));
    metadata
        .attrs
        .set("ImageHeight", AttrValue::UInt(header.height));
    metadata
        .attrs
        .set("BitDepth", AttrValue::UInt(header.bit_depth as u32));
    metadata.attrs.set(
        "Endian",
        AttrValue::Str(if header.is_big_endian {
            "BE".to_string()
        } else {
            "LE".to_string()
        }),
    );
    metadata
        .attrs
        .set("ImageOffset", AttrValue::UInt(header.image_offset));
    metadata
        .attrs
        .set("FileSize", AttrValue::UInt(header.file_size));

    Ok(ImageData {
        width: header.width,
        height: header.height,
        channels,
        format: PixelFormat::F32,
        data: PixelData::F32(data),
        metadata,
    })
}

fn read_10bit_packed<R: Read>(
    reader: &mut R,
    pixel_count: usize,
    big_endian: bool,
) -> IoResult<Vec<f32>> {
    let mut data = Vec::with_capacity(pixel_count * 3);
    let max_val = 1023.0f32;

    for _ in 0..pixel_count {
        let word = read_u32(reader, big_endian)?;
        let r = ((word >> 22) & 0x3FF) as f32 / max_val;
        let g = ((word >> 12) & 0x3FF) as f32 / max_val;
        let b = ((word >> 2) & 0x3FF) as f32 / max_val;
        data.push(r);
        data.push(g);
        data.push(b);
    }

    Ok(data)
}

fn read_12bit<R: Read>(
    reader: &mut R,
    pixel_count: usize,
    big_endian: bool,
) -> IoResult<Vec<f32>> {
    let mut data = Vec::with_capacity(pixel_count * 3);
    let max_val = 4095.0f32;

    for _ in 0..pixel_count {
        for _ in 0..3 {
            let val = read_u16(reader, big_endian)?;
            data.push((val >> 4) as f32 / max_val);
        }
    }

    Ok(data)
}

fn read_16bit<R: Read>(
    reader: &mut R,
    pixel_count: usize,
    big_endian: bool,
) -> IoResult<Vec<f32>> {
    let mut data = Vec::with_capacity(pixel_count * 3);
    let max_val = 65535.0f32;

    for _ in 0..pixel_count {
        for _ in 0..3 {
            let val = read_u16(reader, big_endian)?;
            data.push(val as f32 / max_val);
        }
    }

    Ok(data)
}

fn read_8bit<R: Read>(reader: &mut R, pixel_count: usize) -> IoResult<Vec<f32>> {
    let mut buf = vec![0u8; pixel_count * 3];
    reader.read_exact(&mut buf)
        .map_err(|e| IoError::DecodeError(e.to_string()))?;

    Ok(buf.iter().map(|&v| v as f32 / 255.0).collect())
}

/// Writes an image to a DPX file.
///
/// Bit depth is taken from `metadata.attrs["BitDepth"]` when present (8 or 10),
/// otherwise defaults to 10-bit.
pub fn write<P: AsRef<Path>>(path: P, image: &ImageData) -> IoResult<()> {
    let file = File::create(path.as_ref())?;
    let mut writer = BufWriter::new(file);

    let width = image.width;
    let height = image.height;
    let f32_data = image.to_f32();

    let channels = image.channels as usize;
    if channels < 3 {
        return Err(IoError::EncodeError(
            "DPX requires at least 3 channels".to_string(),
        ));
    }

    let requested = image
        .metadata
        .attrs
        .get("BitDepth")
        .and_then(|v| v.as_u32());
    let bit_depth = match requested {
        Some(8) => BitDepth::Bit8,
        Some(10) => BitDepth::Bit10,
        Some(other) => {
            return Err(IoError::UnsupportedBitDepth(format!("{}", other)));
        }
        None => BitDepth::Bit10,
    };

    let image_offset: u32 = 2048;
    let pixel_count = (width * height) as usize;
    let (image_size, bit_depth_u8, packing) = match bit_depth {
        BitDepth::Bit8 => (pixel_count * 3, 8u8, 0u16),
        BitDepth::Bit10 => (pixel_count * 4, 10u8, 1u16),
        BitDepth::Bit12 | BitDepth::Bit16 => {
            return Err(IoError::UnsupportedBitDepth(format!("{:?}", bit_depth)));
        }
    };
    let image_size = image_size as u32;
    let file_size = image_offset + image_size;

    // File header
    write_bytes(&mut writer, &DPX_MAGIC_BE.to_be_bytes())?;
    write_bytes(&mut writer, &image_offset.to_be_bytes())?;
    write_bytes(&mut writer, b"V2.0\0\0\0\0")?;
    write_bytes(&mut writer, &file_size.to_be_bytes())?;
    write_bytes(&mut writer, &1u32.to_be_bytes())?; // Ditto key
    write_bytes(&mut writer, &1664u32.to_be_bytes())?; // Generic header
    write_bytes(&mut writer, &384u32.to_be_bytes())?; // Industry header
    write_bytes(&mut writer, &0u32.to_be_bytes())?; // User data

    // Filename (100 bytes)
    let filename = path
        .as_ref()
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("output.dpx");
    let mut name_buf = [0u8; 100];
    let name_bytes = filename.as_bytes();
    let copy_len = name_bytes.len().min(99);
    name_buf[..copy_len].copy_from_slice(&name_bytes[..copy_len]);
    write_bytes(&mut writer, &name_buf)?;

    // Pad to 660
    let padding = vec![0u8; 660 - 136];
    write_bytes(&mut writer, &padding)?;

    // Encryption key (unencrypted)
    write_bytes(&mut writer, &0xFFFFFFFFu32.to_be_bytes())?;

    // Pad to 768
    let padding = vec![0u8; 768 - 664];
    write_bytes(&mut writer, &padding)?;

    // Image element
    write_bytes(&mut writer, &0u32.to_be_bytes())?; // Data sign
    write_bytes(&mut writer, &width.to_be_bytes())?;
    write_bytes(&mut writer, &height.to_be_bytes())?;
    write_bytes(&mut writer, &[50u8])?; // RGB descriptor
    write_bytes(&mut writer, &[1u8])?; // Transfer
    write_bytes(&mut writer, &[1u8])?; // Colorimetric
    write_bytes(&mut writer, &[bit_depth_u8])?; // Bit depth
    write_bytes(&mut writer, &packing.to_be_bytes())?; // Packing
    write_bytes(&mut writer, &0u16.to_be_bytes())?; // Encoding
    write_bytes(&mut writer, &image_offset.to_be_bytes())?;
    write_bytes(&mut writer, &0u32.to_be_bytes())?; // EOL padding
    write_bytes(&mut writer, &0u32.to_be_bytes())?; // EOI padding

    // Pad to image offset
    let padding = vec![0u8; (image_offset - 800) as usize];
    write_bytes(&mut writer, &padding)?;

    // Write pixels
    match bit_depth {
        BitDepth::Bit8 => {
            for i in 0..pixel_count {
                let base = i * channels;
                let r = (f32_data.get(base).copied().unwrap_or(0.0).clamp(0.0, 1.0) * 255.0) as u8;
                let g = (f32_data.get(base + 1).copied().unwrap_or(0.0).clamp(0.0, 1.0) * 255.0) as u8;
                let b = (f32_data.get(base + 2).copied().unwrap_or(0.0).clamp(0.0, 1.0) * 255.0) as u8;
                write_bytes(&mut writer, &[r, g, b])?;
            }
        }
        BitDepth::Bit10 => {
            for i in 0..pixel_count {
                let base = i * channels;
                let r = (f32_data.get(base).copied().unwrap_or(0.0).clamp(0.0, 1.0) * 1023.0) as u32;
                let g = (f32_data.get(base + 1).copied().unwrap_or(0.0).clamp(0.0, 1.0) * 1023.0) as u32;
                let b = (f32_data.get(base + 2).copied().unwrap_or(0.0).clamp(0.0, 1.0) * 1023.0) as u32;
                let word = (r << 22) | (g << 12) | (b << 2);
                write_bytes(&mut writer, &word.to_be_bytes())?;
            }
        }
        BitDepth::Bit12 | BitDepth::Bit16 => {}
    }

    writer.flush().map_err(|e| IoError::EncodeError(e.to_string()))?;
    Ok(())
}

fn write_bytes<W: Write>(writer: &mut W, bytes: &[u8]) -> IoResult<()> {
    writer.write_all(bytes).map_err(|e| IoError::EncodeError(e.to_string()))
}

/// DPX bit depth.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BitDepth {
    /// 8 bits per channel.
    Bit8,
    /// 10 bits per channel (standard film).
    #[default]
    Bit10,
    /// 12 bits per channel.
    Bit12,
    /// 16 bits per channel.
    Bit16,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip_10bit() {
        let width = 64;
        let height = 64;
        let mut data = Vec::with_capacity((width * height * 3) as usize);

        for y in 0..height {
            for x in 0..width {
                data.push(x as f32 / width as f32);
                data.push(y as f32 / height as f32);
                data.push(0.5);
            }
        }

        let image = ImageData::from_f32(width, height, 3, data);
        let mut image = image;
        image
            .metadata
            .attrs
            .set("BitDepth", AttrValue::UInt(10));

        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join("vfx_io_test.dpx");

        write(&temp_path, &image).expect("Failed to write DPX");
        let loaded = read(&temp_path).expect("Failed to read DPX");

        assert_eq!(loaded.width, width);
        assert_eq!(loaded.height, height);
        assert_eq!(loaded.channels, 3);

        // Check 10-bit precision
        let loaded_data = loaded.to_f32();
        let orig_data = image.to_f32();
        for i in 0..10 {
            let diff = (loaded_data[i] - orig_data[i]).abs();
            assert!(diff < 0.002, "Value mismatch at {}", i);
        }

        let _ = std::fs::remove_file(&temp_path);
    }

    #[test]
    fn test_roundtrip_8bit() {
        let width = 16;
        let height = 16;
        let mut data = Vec::with_capacity((width * height * 3) as usize);

        for y in 0..height {
            for x in 0..width {
                data.push(x as f32 / width as f32);
                data.push(y as f32 / height as f32);
                data.push(0.25);
            }
        }

        let image = ImageData::from_f32(width, height, 3, data);
        let mut image = image;
        image
            .metadata
            .attrs
            .set("BitDepth", AttrValue::UInt(8));

        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join("vfx_io_test_8bit.dpx");

        write(&temp_path, &image).expect("Failed to write DPX 8-bit");
        let loaded = read(&temp_path).expect("Failed to read DPX");

        assert_eq!(loaded.width, width);
        assert_eq!(loaded.height, height);
        assert_eq!(loaded.channels, 3);
        assert_eq!(
            loaded.metadata.attrs.get("BitDepth").and_then(|v| v.as_u32()),
            Some(8)
        );

        let loaded_data = loaded.to_f32();
        let orig_data = image.to_f32();
        for i in 0..10 {
            let diff = (loaded_data[i] - orig_data[i]).abs();
            assert!(diff < 0.01, "Value mismatch at {}", i);
        }

        let _ = std::fs::remove_file(&temp_path);
    }
}
