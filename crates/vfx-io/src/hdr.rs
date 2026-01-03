//! Radiance HDR (RGBE) format support.
//!
//! Supports reading and writing RGBE with optional RLE scanlines.

use crate::{AttrValue, ImageData, IoError, IoResult, Metadata, PixelData, PixelFormat};
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::path::Path;

const HDR_MAGIC: &str = "#?";

/// Reads an HDR (Radiance RGBE) file.
pub fn read<P: AsRef<Path>>(path: P) -> IoResult<ImageData> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    let (mut metadata, width, height, format) = read_header(&mut reader)?;
    let data = read_pixels(&mut reader, width as usize, height as usize)?;

    if format.to_lowercase().contains("xyze") {
        metadata.colorspace = Some("xyz".to_string());
    } else {
        metadata.colorspace = Some("linear".to_string());
    }

    Ok(ImageData {
        width,
        height,
        channels: 3,
        format: PixelFormat::F32,
        data: PixelData::F32(data),
        metadata,
    })
}

/// Writes an HDR (Radiance RGBE) file.
pub fn write<P: AsRef<Path>>(path: P, image: &ImageData) -> IoResult<()> {
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);

    let format_id = image
        .metadata
        .attrs
        .get("FormatIdentifier")
        .and_then(|v| v.as_str())
        .unwrap_or("RADIANCE");

    let format = image
        .metadata
        .attrs
        .get("Format")
        .and_then(|v| v.as_str())
        .unwrap_or("32-bit_rle_rgbe");

    writeln!(writer, "{}{}", HDR_MAGIC, format_id)?;
    write_header_field(&mut writer, "FORMAT", format)?;

    if let Some(v) = image.metadata.attrs.get("Software").and_then(|v| v.as_str()) {
        write_header_field(&mut writer, "SOFTWARE", v)?;
    }
    if let Some(v) = image.metadata.attrs.get("Exposure").and_then(|v| v.as_f32()) {
        write_header_field(&mut writer, "EXPOSURE", &format!("{}", v))?;
    }

    if let Some(v) = image.metadata.attrs.get("Gamma").and_then(|v| v.as_f32()) {
        write_header_field(&mut writer, "GAMMA", &format!("{}", v))?;
    } else if let Some(gamma) = image.metadata.gamma {
        write_header_field(&mut writer, "GAMMA", &format!("{}", gamma))?;
    }

    if let Some(v) = image
        .metadata
        .attrs
        .get("PixelAspectRatio")
        .and_then(|v| v.as_f32())
    {
        write_header_field(&mut writer, "PIXASPECT", &format!("{}", v))?;
    }

    if let Some(v) = image.metadata.attrs.get("Primaries").and_then(|v| v.as_str()) {
        write_header_field(&mut writer, "PRIMARIES", v)?;
    }
    if let Some(v) = image.metadata.attrs.get("ColorCorrection").and_then(|v| v.as_str()) {
        write_header_field(&mut writer, "COLORCORR", v)?;
    }
    if let Some(v) = image.metadata.attrs.get("View").and_then(|v| v.as_str()) {
        write_header_field(&mut writer, "VIEW", v)?;
    }

    for (key, value) in image.metadata.attrs.iter() {
        if let Some(hdr_key) = key.strip_prefix("HDR:") {
            if let AttrValue::Str(v) = value {
                write_header_field(&mut writer, hdr_key, v)?;
            }
        }
    }

    writeln!(writer)?;
    writeln!(writer, "-Y {} +X {}", image.height, image.width)?;

    write_pixels(&mut writer, image)?;
    Ok(())
}

fn read_header<R: BufRead>(reader: &mut R) -> IoResult<(Metadata, u32, u32, String)> {
    let mut metadata = Metadata::default();
    let mut line = String::new();

    reader.read_line(&mut line)?;
    let magic_line = trim_line(&line);
    if !magic_line.starts_with(HDR_MAGIC) {
        return Err(IoError::InvalidFile("HDR magic not found".into()));
    }

    let format_id = magic_line.trim_start_matches(HDR_MAGIC);
    if !format_id.is_empty() {
        metadata
            .attrs
            .set("FormatIdentifier", AttrValue::Str(format_id.to_string()));
    }

    let mut width = None;
    let mut height = None;
    let mut format = "32-bit_rle_rgbe".to_string();

    loop {
        line.clear();
        let bytes = reader.read_line(&mut line)?;
        if bytes == 0 {
            break;
        }
        let line = trim_line(&line);

        if line.is_empty() {
            continue;
        }

        if line.starts_with('+') || line.starts_with('-') {
            if let Some((w, h)) = parse_resolution(line) {
                width = Some(w);
                height = Some(h);
                metadata.attrs.set("ImageWidth", AttrValue::UInt(w));
                metadata.attrs.set("ImageHeight", AttrValue::UInt(h));
                break;
            } else {
                return Err(IoError::InvalidFile("Invalid HDR resolution line".into()));
            }
        }

        if line.starts_with('#') {
            continue;
        }

        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim();
            let value = value.trim();
            match key.to_uppercase().as_str() {
                "FORMAT" => {
                    format = value.to_string();
                    metadata.attrs.set("Format", AttrValue::Str(format.clone()));
                }
                "EXPOSURE" => match value.parse::<f32>() {
                    Ok(v) => {
                        metadata.attrs.set("Exposure", AttrValue::Float(v));
                    }
                    Err(_) => {
                        metadata.attrs.set("Exposure", AttrValue::Str(value.to_string()));
                    }
                },
                "GAMMA" => match value.parse::<f32>() {
                    Ok(v) => {
                        metadata.gamma = Some(v);
                        metadata.attrs.set("Gamma", AttrValue::Float(v));
                    }
                    Err(_) => {
                        metadata.attrs.set("Gamma", AttrValue::Str(value.to_string()));
                    }
                },
                "PIXASPECT" => match value.parse::<f32>() {
                    Ok(v) => {
                        metadata.attrs.set("PixelAspectRatio", AttrValue::Float(v));
                    }
                    Err(_) => {
                        metadata
                            .attrs
                            .set("PixelAspectRatio", AttrValue::Str(value.to_string()));
                    }
                },
                "SOFTWARE" => {
                    metadata.attrs.set("Software", AttrValue::Str(value.to_string()));
                }
                "PRIMARIES" => {
                    metadata.attrs.set("Primaries", AttrValue::Str(value.to_string()));
                }
                "COLORCORR" => {
                    metadata.attrs.set("ColorCorrection", AttrValue::Str(value.to_string()));
                }
                "VIEW" => {
                    metadata.attrs.set("View", AttrValue::Str(value.to_string()));
                }
                _ => {
                    metadata
                        .attrs
                        .set(format!("HDR:{}", key), AttrValue::Str(value.to_string()));
                }
            }
        }
    }

    let width = width.ok_or_else(|| IoError::InvalidFile("Missing HDR width".into()))?;
    let height = height.ok_or_else(|| IoError::InvalidFile("Missing HDR height".into()))?;

    Ok((metadata, width, height, format))
}

fn read_pixels<R: Read>(reader: &mut R, width: usize, height: usize) -> IoResult<Vec<f32>> {
    let mut first = [0u8; 4];
    reader.read_exact(&mut first)?;

    let use_rle = width >= 8
        && width <= 0x7fff
        && first[0] == 2
        && first[1] == 2
        && ((first[2] as usize) << 8 | first[3] as usize) == width;

    let mut rgbe = vec![0u8; width * height * 4];

    if use_rle {
        let mut scanline = vec![0u8; width * 4];
        decode_rle_scanline(reader, width, &mut scanline, first)?;
        rgbe[0..width * 4].copy_from_slice(&scanline);

        for y in 1..height {
            let mut header = [0u8; 4];
            reader.read_exact(&mut header)?;
            decode_rle_scanline(reader, width, &mut scanline, header)?;
            let offset = y * width * 4;
            rgbe[offset..offset + width * 4].copy_from_slice(&scanline);
        }
    } else {
        rgbe[0..4].copy_from_slice(&first);
        reader.read_exact(&mut rgbe[4..])?;
    }

    let mut data = Vec::with_capacity(width * height * 3);
    for chunk in rgbe.chunks_exact(4) {
        let (r, g, b) = rgbe_to_f32(chunk[0], chunk[1], chunk[2], chunk[3]);
        data.push(r);
        data.push(g);
        data.push(b);
    }

    Ok(data)
}

fn decode_rle_scanline<R: Read>(
    reader: &mut R,
    width: usize,
    out: &mut [u8],
    header: [u8; 4],
) -> IoResult<()> {
    if header[0] != 2 || header[1] != 2 {
        return Err(IoError::InvalidFile("HDR RLE header invalid".into()));
    }
    let encoded_width = ((header[2] as usize) << 8) | (header[3] as usize);
    if encoded_width != width {
        return Err(IoError::InvalidFile("HDR RLE width mismatch".into()));
    }

    let mut channel = vec![0u8; width];
    for c in 0..4 {
        let mut idx = 0usize;
        while idx < width {
            let mut count = [0u8; 1];
            reader.read_exact(&mut count)?;
            let count = count[0] as usize;
            if count > 128 {
                let run = count - 128;
                let mut value = [0u8; 1];
                reader.read_exact(&mut value)?;
                for _ in 0..run {
                    channel[idx] = value[0];
                    idx += 1;
                }
            } else {
                let run = count;
                reader.read_exact(&mut channel[idx..idx + run])?;
                idx += run;
            }
        }

        for x in 0..width {
            out[x * 4 + c] = channel[x];
        }
    }

    Ok(())
}

fn write_pixels<W: Write>(writer: &mut W, image: &ImageData) -> IoResult<()> {
    let width = image.width as usize;
    let height = image.height as usize;
    let channels = image.channels as usize;
    let f32_data = image.to_f32();

    let use_rle = width >= 8 && width <= 0x7fff;

    let mut scanline = vec![0u8; width * 4];
    for y in 0..height {
        for x in 0..width {
            let base = (y * width + x) * channels;
            let r = *f32_data.get(base).unwrap_or(&0.0);
            let g = *f32_data.get(base + 1).unwrap_or(&0.0);
            let b = *f32_data.get(base + 2).unwrap_or(&0.0);
            let rgbe = f32_to_rgbe(r, g, b);
            let offset = x * 4;
            scanline[offset..offset + 4].copy_from_slice(&rgbe);
        }

        if use_rle {
            let header = [2u8, 2u8, (width >> 8) as u8, (width & 0xFF) as u8];
            writer.write_all(&header)?;
            encode_rle_scanline(writer, width, &scanline)?;
        } else {
            writer.write_all(&scanline)?;
        }
    }

    Ok(())
}

fn encode_rle_scanline<W: Write>(writer: &mut W, width: usize, scanline: &[u8]) -> IoResult<()> {
    let mut channel = vec![0u8; width];
    for c in 0..4 {
        for x in 0..width {
            channel[x] = scanline[x * 4 + c];
        }
        let encoded = encode_rle_channel(&channel);
        writer.write_all(&encoded)?;
    }
    Ok(())
}

fn encode_rle_channel(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() * 2);
    let mut i = 0usize;
    while i < data.len() {
        let mut run = 1usize;
        while i + run < data.len() && run < 127 && data[i] == data[i + run] {
            run += 1;
        }

        if run >= 4 {
            out.push((128 + run) as u8);
            out.push(data[i]);
            i += run;
            continue;
        }

        let start = i;
        let mut literal = 0usize;
        while i < data.len() {
            run = 1;
            while i + run < data.len() && run < 127 && data[i] == data[i + run] {
                run += 1;
            }
            if run >= 4 {
                break;
            }
            i += 1;
            literal += 1;
            if literal == 128 {
                break;
            }
        }
        out.push(literal as u8);
        out.extend_from_slice(&data[start..start + literal]);
    }
    out
}

fn f32_to_rgbe(r: f32, g: f32, b: f32) -> [u8; 4] {
    let r = r.max(0.0);
    let g = g.max(0.0);
    let b = b.max(0.0);
    let max = r.max(g).max(b);
    if max < 1.0e-32 {
        return [0, 0, 0, 0];
    }

    let (m, e) = frexp(max);
    let scale = m * 256.0 / max;

    [
        (r * scale).clamp(0.0, 255.0) as u8,
        (g * scale).clamp(0.0, 255.0) as u8,
        (b * scale).clamp(0.0, 255.0) as u8,
        (e + 128) as u8,
    ]
}

fn rgbe_to_f32(r: u8, g: u8, b: u8, e: u8) -> (f32, f32, f32) {
    if e == 0 {
        return (0.0, 0.0, 0.0);
    }
    let exp = (e as i32) - 136;
    let f = 2.0_f32.powi(exp);
    (r as f32 * f, g as f32 * f, b as f32 * f)
}

fn frexp(x: f32) -> (f32, i32) {
    if x == 0.0 {
        return (0.0, 0);
    }
    let e = x.abs().log2().floor() as i32 + 1;
    let m = x / 2.0_f32.powi(e);
    (m, e)
}

fn parse_resolution(line: &str) -> Option<(u32, u32)> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() != 4 {
        return None;
    }

    let mut width = 0u32;
    let mut height = 0u32;

    for i in (0..4).step_by(2) {
        let axis = parts[i];
        let value: u32 = parts.get(i + 1)?.parse().ok()?;

        if axis.ends_with('X') {
            width = value;
        } else if axis.ends_with('Y') {
            height = value;
        }
    }

    if width > 0 && height > 0 {
        Some((width, height))
    } else {
        None
    }
}

fn write_header_field<W: Write>(writer: &mut W, key: &str, value: &str) -> IoResult<()> {
    writeln!(writer, "{}={}", key, value)?;
    Ok(())
}

fn trim_line(line: &str) -> &str {
    line.trim_end_matches(&['\r', '\n'][..])
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn parse_resolution_line() {
        assert_eq!(parse_resolution("-Y 2 +X 3"), Some((3, 2)));
        assert_eq!(parse_resolution("+X 4 -Y 5"), Some((4, 5)));
    }

    #[test]
    fn hdr_roundtrip_small() {
        let width = 4;
        let height = 2;
        let data: Vec<f32> = (0..(width * height * 3))
            .map(|i| (i as f32) / 10.0)
            .collect();

        let image = ImageData::from_f32(width, height, 3, data);
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("vfx_io_test.hdr");

        write(&path, &image).expect("HDR write failed");
        let loaded = read(&path).expect("HDR read failed");

        assert_eq!(loaded.width, width);
        assert_eq!(loaded.height, height);
        assert_eq!(loaded.channels, 3);

        let loaded_data = match loaded.data {
            PixelData::F32(v) => v,
            _ => panic!("Unexpected pixel format"),
        };

        assert_relative_eq!(loaded_data[0], image.to_f32()[0], epsilon = 1e-3);
    }
}
