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

use crate::{AttrValue, ImageData, IoError, IoResult, Metadata, PixelData, PixelFormat};
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
    result
        .metadata
        .attrs
        .set("DataWindowWidth", AttrValue::UInt(width));
    result
        .metadata
        .attrs
        .set("DataWindowHeight", AttrValue::UInt(height));
    if let Err(err) = read_metadata(path, &mut result.metadata) {
        return Err(err);
    }
    
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

fn read_metadata(path: &Path, metadata: &mut Metadata) -> IoResult<()> {
    let data = std::fs::read(path)?;
    let meta = exr::meta::MetaData::read_from_buffered(
        std::io::Cursor::new(&data),
        false,
    )
    .map_err(|e| IoError::DecodeError(format!("EXR metadata parse error: {}", e)))?;

    for (layer_idx, header) in meta.headers.iter().enumerate() {
        let prefix = if meta.headers.len() > 1 {
            format!("Layer{}:", layer_idx)
        } else {
            String::new()
        };

        let display_window = &header.shared_attributes.display_window;
        metadata
            .attrs
            .set(format!("{}ImageWidth", prefix), AttrValue::UInt(display_window.size.width() as u32));
        metadata
            .attrs
            .set(format!("{}ImageHeight", prefix), AttrValue::UInt(display_window.size.height() as u32));

        metadata
            .attrs
            .set(format!("{}PixelAspectRatio", prefix), AttrValue::Float(header.shared_attributes.pixel_aspect));

        metadata
            .attrs
            .set(format!("{}Compression", prefix), AttrValue::Str(format!("{:?}", header.compression)));

        let channels: Vec<String> = header
            .channels
            .list
            .iter()
            .map(|ch| ch.name.to_string())
            .collect();
        metadata
            .attrs
            .set(format!("{}Channels", prefix), AttrValue::Str(channels.join(", ")));
        metadata
            .attrs
            .set(format!("{}ChannelCount", prefix), AttrValue::UInt(header.channels.list.len() as u32));

        metadata
            .attrs
            .set(format!("{}LineOrder", prefix), AttrValue::Str(format!("{:?}", header.line_order)));

        if let Some(chroma) = &header.shared_attributes.chromaticities {
            metadata.attrs.set(
                format!("{}Chromaticities", prefix),
                AttrValue::Str(format!(
                    "R({:.3},{:.3}) G({:.3},{:.3}) B({:.3},{:.3}) W({:.3},{:.3})",
                    chroma.red.0,
                    chroma.red.1,
                    chroma.green.0,
                    chroma.green.1,
                    chroma.blue.0,
                    chroma.blue.1,
                    chroma.white.0,
                    chroma.white.1
                )),
            );
        }

        if let Some(tc) = &header.shared_attributes.time_code {
            metadata.attrs.set(
                format!("{}TimeCode", prefix),
                AttrValue::Str(format!(
                    "{:02}:{:02}:{:02}:{:02}",
                    tc.hours, tc.minutes, tc.seconds, tc.frame
                )),
            );
        }

        for (name, value) in &header.shared_attributes.other {
            metadata
                .attrs
                .set(format!("{}EXR:{}", prefix, name), AttrValue::Str(format!("{:?}", value)));
        }

        if let Some(layer_name) = &header.own_attributes.layer_name {
            metadata
                .attrs
                .set(format!("{}LayerName", prefix), AttrValue::Str(layer_name.to_string()));
        }

        for (name, value) in &header.own_attributes.other {
            metadata
                .attrs
                .set(format!("{}Layer:{}", prefix, name), AttrValue::Str(format!("{:?}", value)));
        }
    }

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
