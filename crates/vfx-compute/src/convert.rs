//! Conversions between vfx-io types and ComputeImage.
//!
//! Provides seamless integration between the I/O layer (ImageData, ImageLayer)
//! and the compute layer (ComputeImage).
//!
//! # Example
//!
//! ```ignore
//! use vfx_io::read;
//! use vfx_compute::{ComputeImage, Processor};
//!
//! // Read image and convert to compute format
//! let data = read("input.exr")?;
//! let mut img: ComputeImage = data.into();
//!
//! // Process
//! let proc = Processor::auto()?;
//! proc.apply_exposure(&mut img, 1.5)?;
//!
//! // Convert back
//! let output: ImageData = img.into();
//! ```

#[cfg(feature = "io")]
use vfx_io::{ImageData, ImageLayer, PixelFormat};

use crate::{ComputeImage, ComputeResult};

// ============================================================================
// ImageData <-> ComputeImage
// ============================================================================

#[cfg(feature = "io")]
impl From<ImageData> for ComputeImage {
    /// Converts ImageData to ComputeImage.
    ///
    /// Pixel data is converted to f32 RGBA format.
    /// RGB images get alpha=1.0 added.
    fn from(data: ImageData) -> Self {
        let f32_data = data.to_f32();
        let channels = data.channels;

        // Ensure 4 channels (add alpha if needed)
        let rgba_data = if channels == 3 {
            let mut rgba = Vec::with_capacity((data.width * data.height * 4) as usize);
            for chunk in f32_data.chunks(3) {
                rgba.extend_from_slice(chunk);
                rgba.push(1.0);
            }
            rgba
        } else if channels == 4 {
            f32_data
        } else if channels == 1 {
            // Grayscale -> RGBA
            let mut rgba = Vec::with_capacity((data.width * data.height * 4) as usize);
            for &v in &f32_data {
                rgba.push(v);
                rgba.push(v);
                rgba.push(v);
                rgba.push(1.0);
            }
            rgba
        } else {
            // Unsupported channel count, pad or truncate
            let mut rgba = Vec::with_capacity((data.width * data.height * 4) as usize);
            for chunk in f32_data.chunks(channels as usize) {
                rgba.push(chunk.get(0).copied().unwrap_or(0.0));
                rgba.push(chunk.get(1).copied().unwrap_or(0.0));
                rgba.push(chunk.get(2).copied().unwrap_or(0.0));
                rgba.push(chunk.get(3).copied().unwrap_or(1.0));
            }
            rgba
        };

        ComputeImage::from_f32(rgba_data, data.width, data.height, 4)
            .expect("ImageData conversion should not fail")
    }
}

#[cfg(feature = "io")]
impl From<&ImageData> for ComputeImage {
    fn from(data: &ImageData) -> Self {
        data.clone().into()
    }
}

#[cfg(feature = "io")]
impl From<ComputeImage> for ImageData {
    /// Converts ComputeImage back to ImageData.
    ///
    /// Output is F32 format with the same channel count as input.
    fn from(img: ComputeImage) -> Self {
        ImageData::from_f32(img.width, img.height, img.channels, img.data().to_vec())
    }
}

#[cfg(feature = "io")]
impl From<&ComputeImage> for ImageData {
    fn from(img: &ComputeImage) -> Self {
        ImageData::from_f32(img.width, img.height, img.channels, img.data().to_vec())
    }
}

// ============================================================================
// ImageLayer <-> ComputeImage
// ============================================================================

/// Metadata preserved during ImageLayer conversion.
#[cfg(feature = "io")]
#[derive(Debug, Clone)]
pub struct LayerMeta {
    /// Layer name.
    pub name: String,
    /// Original channel names and order.
    pub channel_names: Vec<String>,
}

#[cfg(feature = "io")]
impl LayerMeta {
    /// Creates metadata from layer.
    pub fn from_layer(layer: &ImageLayer) -> Self {
        Self {
            name: layer.name.clone(),
            channel_names: layer.channels.iter().map(|c| c.name.clone()).collect(),
        }
    }
}

/// Trait for types that can be converted to/from ComputeImage.
pub trait Processable: Sized {
    /// Metadata type preserved during conversion.
    type Meta;

    /// Converts to ComputeImage for processing.
    fn to_compute(&self) -> ComputeImage;

    /// Extracts metadata for later reconstruction.
    fn extract_meta(&self) -> Self::Meta;

    /// Reconstructs from ComputeImage with preserved metadata.
    fn from_compute(img: ComputeImage, meta: Self::Meta) -> ComputeResult<Self>;
}

#[cfg(feature = "io")]
impl Processable for ImageData {
    type Meta = (u32, PixelFormat); // original channels and format

    fn to_compute(&self) -> ComputeImage {
        self.into()
    }

    fn extract_meta(&self) -> Self::Meta {
        (self.channels, self.format)
    }

    fn from_compute(img: ComputeImage, meta: Self::Meta) -> ComputeResult<Self> {
        let (orig_channels, _orig_format) = meta;
        
        // Convert to original channel count
        if orig_channels == img.channels {
            Ok(img.into())
        } else if orig_channels == 3 && img.channels == 4 {
            // Drop alpha
            let mut rgb = Vec::with_capacity((img.width * img.height * 3) as usize);
            for chunk in img.data().chunks(4) {
                rgb.extend_from_slice(&chunk[..3]);
            }
            Ok(ImageData::from_f32(img.width, img.height, 3, rgb))
        } else {
            Ok(img.into())
        }
    }
}

#[cfg(feature = "io")]
impl Processable for ImageLayer {
    type Meta = LayerMeta;

    fn to_compute(&self) -> ComputeImage {
        // Extract RGBA from layer channels
        let pixel_count = (self.width * self.height) as usize;
        let mut rgba = vec![0.0f32; pixel_count * 4];

        // Map common channel names to RGBA indices
        for (idx, channel) in self.channels.iter().enumerate() {
            let target_idx = match channel.name.to_uppercase().as_str() {
                "R" | "RED" => Some(0),
                "G" | "GREEN" => Some(1),
                "B" | "BLUE" => Some(2),
                "A" | "ALPHA" => Some(3),
                _ if idx < 4 => Some(idx), // Fallback: use order
                _ => None,
            };

            if let Some(ti) = target_idx {
                let samples = channel.samples.to_f32();
                for (i, &v) in samples.iter().enumerate() {
                    if i < pixel_count {
                        rgba[i * 4 + ti] = v;
                    }
                }
            }
        }

        // Set alpha to 1.0 if no alpha channel
        let has_alpha = self.channels.iter().any(|c| {
            matches!(c.name.to_uppercase().as_str(), "A" | "ALPHA")
        });
        if !has_alpha {
            for i in 0..pixel_count {
                rgba[i * 4 + 3] = 1.0;
            }
        }

        ComputeImage::from_f32(rgba, self.width, self.height, 4)
            .expect("ImageLayer conversion should not fail")
    }

    fn extract_meta(&self) -> Self::Meta {
        LayerMeta::from_layer(self)
    }

    fn from_compute(img: ComputeImage, meta: Self::Meta) -> ComputeResult<Self> {
        use vfx_io::{ImageChannel, ChannelKind, ChannelSampleType, ChannelSamples};

        let pixel_count = (img.width * img.height) as usize;
        let data = img.data();

        // Reconstruct channels based on metadata
        let mut channels = Vec::new();
        
        for (idx, name) in meta.channel_names.iter().enumerate() {
            let source_idx = match name.to_uppercase().as_str() {
                "R" | "RED" => 0,
                "G" | "GREEN" => 1,
                "B" | "BLUE" => 2,
                "A" | "ALPHA" => 3,
                _ if idx < 4 => idx,
                _ => continue,
            };

            let mut samples = Vec::with_capacity(pixel_count);
            for i in 0..pixel_count {
                samples.push(data[i * 4 + source_idx]);
            }

            channels.push(ImageChannel {
                name: name.clone(),
                kind: if source_idx == 3 { ChannelKind::Alpha } else { ChannelKind::Color },
                sample_type: ChannelSampleType::F32,
                samples: ChannelSamples::F32(samples),
                sampling: (1, 1),
                quantize_linearly: false,
            });
        }

        Ok(ImageLayer {
            name: meta.name,
            width: img.width,
            height: img.height,
            channels,
        })
    }
}

// ============================================================================
// Convenience Functions
// ============================================================================

/// Converts ImageData to ComputeImage (pads to 4 channels).
#[cfg(feature = "io")]
pub fn from_image_data(data: &ImageData) -> ComputeImage {
    data.into()
}

/// Converts ImageData to ComputeImage preserving original channel count.
///
/// Use this for optimal memory usage when you don't need RGBA padding.
/// Note: Color operations (CDL, LUT, Saturation) require at least 3 channels.
#[cfg(feature = "io")]
pub fn from_image_data_direct(data: &ImageData) -> ComputeImage {
    let f32_data = data.to_f32();
    ComputeImage::from_f32(f32_data, data.width, data.height, data.channels)
        .expect("ImageData conversion should not fail")
}

/// Converts ComputeImage to ImageData.
#[cfg(feature = "io")]
pub fn to_image_data(img: &ComputeImage) -> ImageData {
    img.into()
}

/// Converts ImageLayer to ComputeImage.
#[cfg(feature = "io")]
pub fn from_layer(layer: &ImageLayer) -> ComputeImage {
    layer.to_compute()
}

/// Converts ComputeImage to ImageLayer with given name.
#[cfg(feature = "io")]
pub fn to_layer(img: &ComputeImage, name: &str) -> ComputeResult<ImageLayer> {
    let meta = LayerMeta {
        name: name.to_string(),
        channel_names: vec!["R".into(), "G".into(), "B".into(), "A".into()],
    };
    ImageLayer::from_compute(img.clone(), meta)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_image_roundtrip() {
        let img = ComputeImage::from_f32(
            vec![0.5, 0.3, 0.2, 1.0, 0.1, 0.2, 0.3, 0.8],
            2, 1, 4,
        ).unwrap();

        #[cfg(feature = "io")]
        {
            let data: ImageData = (&img).into();
            assert_eq!(data.width, 2);
            assert_eq!(data.height, 1);
            assert_eq!(data.channels, 4);

            let back: ComputeImage = data.into();
            assert_eq!(back.width, img.width);
            assert!((back.data()[0] - 0.5).abs() < 1e-5);
        }
    }

    #[cfg(feature = "io")]
    #[test]
    fn test_processable_image_data() {
        let data = ImageData::from_f32(2, 2, 3, vec![0.5; 12]);
        let meta = data.extract_meta();
        let compute = data.to_compute();

        assert_eq!(compute.channels, 4); // Converted to RGBA
        
        let back = ImageData::from_compute(compute, meta).unwrap();
        assert_eq!(back.channels, 3); // Restored to RGB
    }

    #[cfg(feature = "io")]
    #[test]
    fn test_from_image_data_direct() {
        // Test direct conversion preserves channel count
        let data_rgb = ImageData::from_f32(2, 2, 3, vec![0.5; 12]);
        let compute_rgb = from_image_data_direct(&data_rgb);
        assert_eq!(compute_rgb.channels, 3); // Preserved!

        let data_gray = ImageData::from_f32(2, 2, 1, vec![0.5; 4]);
        let compute_gray = from_image_data_direct(&data_gray);
        assert_eq!(compute_gray.channels, 1); // Preserved!

        let data_rgba = ImageData::from_f32(2, 2, 4, vec![0.5; 16]);
        let compute_rgba = from_image_data_direct(&data_rgba);
        assert_eq!(compute_rgba.channels, 4); // Preserved!
    }

    #[cfg(feature = "io")]
    #[test]
    fn test_rgb_color_ops() {
        use crate::Processor;

        // Test that color ops work with 3 channels
        let data = vec![0.5, 0.3, 0.2,  0.4, 0.4, 0.4]; // 2 pixels RGB
        let mut img = ComputeImage::from_f32(data, 2, 1, 3).unwrap();
        
        let proc = Processor::auto().unwrap();
        
        // Exposure should work on RGB
        proc.apply_exposure(&mut img, 1.0).unwrap();
        
        // Values should be doubled (2^1 = 2x)
        assert!((img.data()[0] - 1.0).abs() < 0.01); // 0.5 * 2 = 1.0
        assert!((img.data()[1] - 0.6).abs() < 0.01); // 0.3 * 2 = 0.6
    }
}
