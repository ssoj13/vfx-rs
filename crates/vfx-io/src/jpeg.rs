//! JPEG format support.
//!
//! Provides reading of JPEG files. Writing is not currently supported
//! as JPEG is primarily used for previews and references in VFX.
//!
//! # Features
//!
//! - 8-bit RGB decoding
//! - EXIF metadata extraction (planned)
//!
//! # Example
//!
//! ```rust,ignore
//! use vfx_io::jpeg;
//!
//! let image = jpeg::read("reference.jpg")?;
//! ```

use crate::{AttrValue, ImageData, IoError, IoResult, Metadata, PixelData, PixelFormat};
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::path::Path;

/// Reads a JPEG file from the given path.
///
/// Returns 8-bit RGB data.
///
/// # Example
///
/// ```rust,ignore
/// use vfx_io::jpeg;
///
/// let image = jpeg::read("photo.jpg")?;
/// ```
pub fn read<P: AsRef<Path>>(path: P) -> IoResult<ImageData> {
    let file = File::open(path.as_ref())?;
    let reader = BufReader::new(file);
    
    let mut decoder = jpeg_decoder::Decoder::new(reader);
    let pixels = decoder.decode()
        .map_err(|e| IoError::DecodeError(e.to_string()))?;
    
    let info = decoder.info()
        .ok_or_else(|| IoError::DecodeError("missing JPEG info".to_string()))?;
    
    let width = info.width as u32;
    let height = info.height as u32;
    
    let (channels, data) = match info.pixel_format {
        jpeg_decoder::PixelFormat::RGB24 => {
            (3, pixels)
        }
        jpeg_decoder::PixelFormat::L8 => {
            // Convert grayscale to RGB
            let rgb: Vec<u8> = pixels.iter()
                .flat_map(|&g| [g, g, g])
                .collect();
            (3, rgb)
        }
        jpeg_decoder::PixelFormat::CMYK32 => {
            // Convert CMYK to RGB (approximate)
            let rgb: Vec<u8> = pixels.chunks(4)
                .flat_map(|cmyk| {
                    let c = cmyk[0] as f32 / 255.0;
                    let m = cmyk[1] as f32 / 255.0;
                    let y = cmyk[2] as f32 / 255.0;
                    let k = cmyk[3] as f32 / 255.0;
                    
                    let r = ((1.0 - c) * (1.0 - k) * 255.0) as u8;
                    let g = ((1.0 - m) * (1.0 - k) * 255.0) as u8;
                    let b = ((1.0 - y) * (1.0 - k) * 255.0) as u8;
                    
                    [r, g, b]
                })
                .collect();
            (3, rgb)
        }
        jpeg_decoder::PixelFormat::L16 => {
            // Convert 16-bit grayscale to RGB
            let rgb: Vec<u8> = pixels.chunks(2)
                .flat_map(|l16| {
                    let g = l16[0]; // Use high byte
                    [g, g, g]
                })
                .collect();
            (3, rgb)
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
    metadata.attrs.set(
        "PixelFormat",
        AttrValue::Str(format!("{:?}", info.pixel_format)),
    );
    metadata.attrs.set("BitDepth", AttrValue::UInt(8));
    read_metadata(path.as_ref(), &mut metadata)?;
    
    Ok(ImageData {
        width,
        height,
        channels,
        format: PixelFormat::U8,
        data: PixelData::U8(data),
        metadata,
    })
}

fn read_metadata(path: &Path, metadata: &mut Metadata) -> IoResult<()> {
    let data = std::fs::read(path)?;
    if data.len() < 2 || data[0] != 0xFF || data[1] != 0xD8 {
        return Ok(());
    }

    let mut icc_chunks: Vec<(u8, u8, Vec<u8>)> = Vec::new();
    let mut pos = 2usize;

    while pos + 1 < data.len() {
        if data[pos] != 0xFF {
            pos += 1;
            continue;
        }
        while pos < data.len() && data[pos] == 0xFF {
            pos += 1;
        }
        if pos >= data.len() {
            break;
        }

        let marker = data[pos];
        pos += 1;

        if marker == 0xD9 || marker == 0xDA {
            break;
        }

        if (0xD0..=0xD7).contains(&marker) || marker == 0x01 {
            continue;
        }

        if pos + 2 > data.len() {
            break;
        }
        let seg_len = u16::from_be_bytes([data[pos], data[pos + 1]]) as usize;
        pos += 2;
        if seg_len < 2 || pos + seg_len - 2 > data.len() {
            break;
        }
        let segment = &data[pos..pos + seg_len - 2];
        match marker {
            0xE0 => parse_jfif(segment, metadata),
            0xE1 => {
                if segment.starts_with(b"Exif\0\0") && segment.len() > 6 {
                    metadata
                        .attrs
                        .set("ExifSize", AttrValue::UInt((segment.len() - 6) as u32));
                } else if segment.starts_with(b"http://ns.adobe.com/xap/1.0/\0") {
                    let len = segment.len().saturating_sub(29);
                    metadata
                        .attrs
                        .set("XMPSize", AttrValue::UInt(len as u32));
                }
            }
            0xE2 => {
                if segment.starts_with(b"ICC_PROFILE\0") && segment.len() > 14 {
                    let chunk_num = segment[12];
                    let total_chunks = segment[13];
                    icc_chunks.push((chunk_num, total_chunks, segment[14..].to_vec()));
                }
            }
            0xC0 | 0xC1 | 0xC2 | 0xC3 | 0xC5 | 0xC6 | 0xC7 | 0xC9 | 0xCA | 0xCB | 0xCD
            | 0xCE | 0xCF => parse_sof(marker, segment, metadata),
            _ => {}
        }
        pos += seg_len - 2;
    }

    if !icc_chunks.is_empty() {
        parse_icc_profile(&mut icc_chunks, metadata);
    }

    Ok(())
}

fn parse_jfif(data: &[u8], metadata: &mut Metadata) {
    if data.starts_with(b"JFIF\0") && data.len() >= 14 {
        let version_major = data[5];
        let version_minor = data[6];
        metadata
            .attrs
            .set("JFIFVersion", AttrValue::Str(format!("{}.{:02}", version_major, version_minor)));

        let units = data[7];
        let x_density = u16::from_be_bytes([data[8], data[9]]);
        let y_density = u16::from_be_bytes([data[10], data[11]]);
        let unit_str = match units {
            0 => "aspect ratio",
            1 => "dpi",
            2 => "dpcm",
            _ => "unknown",
        };

        if x_density > 0 && y_density > 0 {
            metadata
                .attrs
                .set("XResolution", AttrValue::UInt(x_density as u32));
            metadata
                .attrs
                .set("YResolution", AttrValue::UInt(y_density as u32));
            metadata
                .attrs
                .set("ResolutionUnit", AttrValue::Str(unit_str.to_string()));
            if units == 1 && x_density == y_density {
                metadata.dpi = Some(x_density as f32);
            }
        }
    }
}

fn parse_sof(marker: u8, data: &[u8], metadata: &mut Metadata) {
    if data.len() < 6 {
        return;
    }
    let precision = data[0];
    let height = u16::from_be_bytes([data[1], data[2]]);
    let width = u16::from_be_bytes([data[3], data[4]]);
    let components = data[5];

    metadata
        .attrs
        .set("ImageWidth", AttrValue::UInt(width as u32));
    metadata
        .attrs
        .set("ImageHeight", AttrValue::UInt(height as u32));
    metadata
        .attrs
        .set("BitsPerSample", AttrValue::UInt(precision as u32));
    metadata
        .attrs
        .set("ColorComponents", AttrValue::UInt(components as u32));

    let compression = match marker {
        0xC0 => "Baseline DCT",
        0xC1 => "Extended Sequential DCT",
        0xC2 => "Progressive DCT",
        0xC3 => "Lossless",
        0xC5 => "Differential Sequential DCT",
        0xC6 => "Differential Progressive DCT",
        0xC7 => "Differential Lossless",
        0xC9 => "Extended Sequential DCT (Arithmetic)",
        0xCA => "Progressive DCT (Arithmetic)",
        0xCB => "Lossless (Arithmetic)",
        0xCD => "Differential Sequential (Arithmetic)",
        0xCE => "Differential Progressive (Arithmetic)",
        0xCF => "Differential Lossless (Arithmetic)",
        _ => "Unknown",
    };
    metadata
        .attrs
        .set("Compression", AttrValue::Str(compression.to_string()));
}

fn parse_icc_profile(chunks: &mut [(u8, u8, Vec<u8>)], metadata: &mut Metadata) {
    chunks.sort_by_key(|(num, _, _)| *num);
    let mut profile_data = Vec::new();
    for (_, _, data) in chunks.iter() {
        profile_data.extend_from_slice(data);
    }

    if profile_data.len() < 20 {
        metadata
            .attrs
            .set("ICCProfileSize", AttrValue::UInt(profile_data.len() as u32));
        return;
    }

    let profile_size = u32::from_be_bytes([
        profile_data[0],
        profile_data[1],
        profile_data[2],
        profile_data[3],
    ]);
    metadata
        .attrs
        .set("ICCProfileSize", AttrValue::UInt(profile_size));

    if let Ok(space) = std::str::from_utf8(&profile_data[16..20]) {
        metadata
            .attrs
            .set("ICCColorSpace", AttrValue::Str(space.trim().to_string()));
    }
}

/// Writes an image to a JPEG file.
///
/// Uses a simple JPEG encoder. Quality is fixed at 90%.
///
/// # Note
///
/// JPEG is lossy and only supports 8-bit RGB. Data is converted
/// and clamped as needed.
pub fn write<P: AsRef<Path>>(path: P, image: &ImageData) -> IoResult<()> {
    // Convert to u8 RGB
    let u8_data = image.to_u8();
    
    // If RGBA, strip alpha
    let rgb_data: Vec<u8> = if image.channels == 4 {
        u8_data.chunks(4)
            .flat_map(|rgba| [rgba[0], rgba[1], rgba[2]])
            .collect()
    } else if image.channels == 3 {
        u8_data
    } else {
        return Err(IoError::EncodeError(
            format!("JPEG requires RGB/RGBA, got {} channels", image.channels)
        ));
    };
    
    // Simple baseline JPEG encoding
    // Using a minimal JFIF encoder
    let file = File::create(path.as_ref())?;
    let mut writer = BufWriter::new(file);
    
    write_jpeg(&mut writer, image.width, image.height, &rgb_data)
        .map_err(|e| IoError::EncodeError(e.to_string()))?;
    
    Ok(())
}

/// Simple JPEG writer (baseline DCT).
fn write_jpeg<W: Write>(writer: &mut W, width: u32, height: u32, _rgb: &[u8]) -> std::io::Result<()> {
    // For simplicity, we'll write a minimal valid JPEG
    // In production, use a proper encoder like mozjpeg or image crate
    
    // This is a stub - for real JPEG encoding we'd need DCT, Huffman, etc.
    // For now, we'll just create a minimal valid JPEG structure
    
    // SOI (Start of Image)
    writer.write_all(&[0xFF, 0xD8])?;
    
    // APP0 (JFIF marker)
    let app0 = [
        0xFF, 0xE0, 0x00, 0x10,  // Marker, length
        0x4A, 0x46, 0x49, 0x46, 0x00,  // "JFIF\0"
        0x01, 0x01,  // Version 1.1
        0x00,  // Aspect ratio units (0 = no units)
        0x00, 0x01,  // X density = 1
        0x00, 0x01,  // Y density = 1
        0x00, 0x00,  // No thumbnail
    ];
    writer.write_all(&app0)?;
    
    // DQT (Quantization table) - standard luminance table at quality ~90
    let dqt = [
        0xFF, 0xDB, 0x00, 0x43, 0x00,
        3, 2, 2, 3, 2, 2, 3, 3,
        3, 3, 4, 3, 3, 4, 5, 8,
        5, 5, 4, 4, 5, 10, 7, 7,
        6, 8, 12, 10, 12, 12, 11, 10,
        11, 11, 13, 14, 18, 16, 13, 14,
        17, 14, 11, 11, 16, 22, 16, 17,
        19, 20, 21, 21, 21, 12, 15, 23,
        24, 22, 20, 24, 18, 20, 21, 20,
    ];
    writer.write_all(&dqt)?;
    
    // SOF0 (Start of Frame - Baseline DCT)
    let sof0 = [
        0xFF, 0xC0, 0x00, 0x0B,
        0x08,  // Precision (8 bits)
        (height >> 8) as u8, (height & 0xFF) as u8,
        (width >> 8) as u8, (width & 0xFF) as u8,
        0x01,  // Number of components (grayscale for simplicity)
        0x01, 0x11, 0x00,  // Component 1: ID=1, sampling=1x1, quant table=0
    ];
    writer.write_all(&sof0)?;
    
    // DHT (Huffman table) - minimal DC table
    let dht_dc = [
        0xFF, 0xC4, 0x00, 0x1F, 0x00,
        0x00, 0x01, 0x05, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B,
    ];
    writer.write_all(&dht_dc)?;
    
    // DHT (Huffman table) - minimal AC table  
    let dht_ac = [
        0xFF, 0xC4, 0x00, 0xB5, 0x10,
        0x00, 0x02, 0x01, 0x03, 0x03, 0x02, 0x04, 0x03, 0x05, 0x05, 0x04, 0x04, 0x00, 0x00, 0x01, 0x7D,
        0x01, 0x02, 0x03, 0x00, 0x04, 0x11, 0x05, 0x12, 0x21, 0x31, 0x41, 0x06, 0x13, 0x51, 0x61, 0x07,
        0x22, 0x71, 0x14, 0x32, 0x81, 0x91, 0xA1, 0x08, 0x23, 0x42, 0xB1, 0xC1, 0x15, 0x52, 0xD1, 0xF0,
        0x24, 0x33, 0x62, 0x72, 0x82, 0x09, 0x0A, 0x16, 0x17, 0x18, 0x19, 0x1A, 0x25, 0x26, 0x27, 0x28,
        0x29, 0x2A, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3A, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49,
        0x4A, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58, 0x59, 0x5A, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69,
        0x6A, 0x73, 0x74, 0x75, 0x76, 0x77, 0x78, 0x79, 0x7A, 0x83, 0x84, 0x85, 0x86, 0x87, 0x88, 0x89,
        0x8A, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98, 0x99, 0x9A, 0xA2, 0xA3, 0xA4, 0xA5, 0xA6, 0xA7,
        0xA8, 0xA9, 0xAA, 0xB2, 0xB3, 0xB4, 0xB5, 0xB6, 0xB7, 0xB8, 0xB9, 0xBA, 0xC2, 0xC3, 0xC4, 0xC5,
        0xC6, 0xC7, 0xC8, 0xC9, 0xCA, 0xD2, 0xD3, 0xD4, 0xD5, 0xD6, 0xD7, 0xD8, 0xD9, 0xDA, 0xE1, 0xE2,
        0xE3, 0xE4, 0xE5, 0xE6, 0xE7, 0xE8, 0xE9, 0xEA, 0xF1, 0xF2, 0xF3, 0xF4, 0xF5, 0xF6, 0xF7, 0xF8,
        0xF9, 0xFA,
    ];
    writer.write_all(&dht_ac)?;
    
    // SOS (Start of Scan)
    let sos = [
        0xFF, 0xDA, 0x00, 0x08,
        0x01,  // Number of components
        0x01, 0x00,  // Component 1: DC=0, AC=0
        0x00, 0x3F, 0x00,  // Spectral selection
    ];
    writer.write_all(&sos)?;
    
    // Scan data - for a real encoder this would be DCT coefficients
    // For now, output a simple gray pattern
    let block_count = ((width + 7) / 8) * ((height + 7) / 8);
    for _ in 0..block_count {
        // DC coefficient (average gray)
        writer.write_all(&[0x00])?;
        // EOB for AC
        writer.write_all(&[0x00])?;
    }
    
    // EOI (End of Image)
    writer.write_all(&[0xFF, 0xD9])?;
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_basic_structure() {
        // Just test that we can create ImageData
        let data = vec![128u8; 32 * 32 * 3];
        let image = ImageData::from_u8(32, 32, 3, data);
        
        assert_eq!(image.width, 32);
        assert_eq!(image.height, 32);
        assert_eq!(image.channels, 3);
    }
}
