//! High-level operations on ImageLayer.
//!
//! Provides convenient wrappers around low-level operations that work
//! directly with [`ImageLayer`] instead of raw pixel buffers.
//!
//! # Example
//!
//! ```rust,ignore
//! use vfx_ops::layer_ops::{resize_layer, blur_layer};
//! use vfx_ops::resize::Filter;
//!
//! let layer = reader.read_layer("input.exr", "diffuse")?;
//! let resized = resize_layer(&layer, 1920, 1080, Filter::Lanczos3)?;
//! let blurred = blur_layer(&resized, 5)?;
//! ```

use vfx_io::{ImageLayer, ImageChannel, ChannelSamples};
use crate::{OpsResult, OpsError};
use crate::resize::{resize_f32, Filter};
use crate::filter::{box_blur, Kernel, convolve};
use crate::transform::crop as crop_raw;

/// Resize an image layer to new dimensions.
///
/// Applies the specified resampling filter to all channels uniformly.
/// Preserves channel metadata (names, kinds, sampling).
///
/// # Arguments
///
/// * `layer` - Source image layer
/// * `dst_width` - Target width in pixels
/// * `dst_height` - Target height in pixels
/// * `filter` - Resampling filter to use
pub fn resize_layer(
    layer: &ImageLayer,
    dst_width: usize,
    dst_height: usize,
    filter: Filter,
) -> OpsResult<ImageLayer> {
    let src_w = layer.width as usize;
    let src_h = layer.height as usize;
    
    let mut new_channels = Vec::with_capacity(layer.channels.len());
    
    for channel in &layer.channels {
        let src_data = channel.samples.to_f32();
        let resized = resize_f32(&src_data, src_w, src_h, 1, dst_width, dst_height, filter)?;
        
        new_channels.push(ImageChannel {
            name: channel.name.clone(),
            kind: channel.kind,
            sample_type: channel.sample_type,
            samples: ChannelSamples::F32(resized),
            sampling: channel.sampling,
            quantize_linearly: channel.quantize_linearly,
        });
    }
    
    Ok(ImageLayer {
        name: layer.name.clone(),
        width: dst_width as u32,
        height: dst_height as u32,
        channels: new_channels,
    })
}

/// Apply gaussian blur to an image layer.
///
/// # Arguments
///
/// * `layer` - Source image layer
/// * `radius` - Blur radius in pixels
pub fn blur_layer(layer: &ImageLayer, radius: usize) -> OpsResult<ImageLayer> {
    let w = layer.width as usize;
    let h = layer.height as usize;
    let sigma = radius as f32 / 2.0;
    let kernel = Kernel::gaussian(radius * 2 + 1, sigma);
    
    let mut new_channels = Vec::with_capacity(layer.channels.len());
    
    for channel in &layer.channels {
        let src_data = channel.samples.to_f32();
        let blurred = convolve(&src_data, w, h, 1, &kernel)?;
        
        new_channels.push(ImageChannel {
            name: channel.name.clone(),
            kind: channel.kind,
            sample_type: channel.sample_type,
            samples: ChannelSamples::F32(blurred),
            sampling: channel.sampling,
            quantize_linearly: channel.quantize_linearly,
        });
    }
    
    Ok(ImageLayer {
        name: layer.name.clone(),
        width: layer.width,
        height: layer.height,
        channels: new_channels,
    })
}

/// Apply box blur to an image layer.
pub fn box_blur_layer(layer: &ImageLayer, radius: usize) -> OpsResult<ImageLayer> {
    let w = layer.width as usize;
    let h = layer.height as usize;
    
    let mut new_channels = Vec::with_capacity(layer.channels.len());
    
    for channel in &layer.channels {
        let src_data = channel.samples.to_f32();
        let blurred = box_blur(&src_data, w, h, 1, radius)?;
        
        new_channels.push(ImageChannel {
            name: channel.name.clone(),
            kind: channel.kind,
            sample_type: channel.sample_type,
            samples: ChannelSamples::F32(blurred),
            sampling: channel.sampling,
            quantize_linearly: channel.quantize_linearly,
        });
    }
    
    Ok(ImageLayer {
        name: layer.name.clone(),
        width: layer.width,
        height: layer.height,
        channels: new_channels,
    })
}

/// Crop a rectangular region from an image layer.
///
/// # Arguments
///
/// * `layer` - Source image layer
/// * `x`, `y` - Top-left corner of crop region
/// * `width`, `height` - Size of crop region
pub fn crop_layer(
    layer: &ImageLayer,
    x: usize,
    y: usize,
    width: usize,
    height: usize,
) -> OpsResult<ImageLayer> {
    let src_w = layer.width as usize;
    let src_h = layer.height as usize;
    
    // Validate bounds
    if x + width > src_w || y + height > src_h {
        return Err(OpsError::InvalidDimensions(format!(
            "crop region {}x{} @ ({},{}) exceeds image {}x{}",
            width, height, x, y, src_w, src_h
        )));
    }
    
    let mut new_channels = Vec::with_capacity(layer.channels.len());
    
    for channel in &layer.channels {
        let src_data = channel.samples.to_f32();
        let cropped = crop_raw(&src_data, src_w, src_h, 1, x, y, width, height)?;
        
        new_channels.push(ImageChannel {
            name: channel.name.clone(),
            kind: channel.kind,
            sample_type: channel.sample_type,
            samples: ChannelSamples::F32(cropped),
            sampling: channel.sampling,
            quantize_linearly: channel.quantize_linearly,
        });
    }
    
    Ok(ImageLayer {
        name: layer.name.clone(),
        width: width as u32,
        height: height as u32,
        channels: new_channels,
    })
}

/// Apply sharpening to an image layer.
///
/// # Arguments
///
/// * `layer` - Source image layer
/// * `amount` - Sharpening strength (1.0 = normal, >1 = stronger)
pub fn sharpen_layer(layer: &ImageLayer, amount: f32) -> OpsResult<ImageLayer> {
    let w = layer.width as usize;
    let h = layer.height as usize;
    let kernel = Kernel::sharpen(amount);
    
    let mut new_channels = Vec::with_capacity(layer.channels.len());
    
    for channel in &layer.channels {
        let src_data = channel.samples.to_f32();
        let sharpened = convolve(&src_data, w, h, 1, &kernel)?;
        
        new_channels.push(ImageChannel {
            name: channel.name.clone(),
            kind: channel.kind,
            sample_type: channel.sample_type,
            samples: ChannelSamples::F32(sharpened),
            sampling: channel.sampling,
            quantize_linearly: channel.quantize_linearly,
        });
    }
    
    Ok(ImageLayer {
        name: layer.name.clone(),
        width: layer.width,
        height: layer.height,
        channels: new_channels,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use vfx_io::{ChannelKind, ChannelSampleType};

    fn make_test_layer(width: u32, height: u32) -> ImageLayer {
        let pixel_count = (width * height) as usize;
        ImageLayer {
            name: "test".to_string(),
            width,
            height,
            channels: vec![
                ImageChannel {
                    name: "R".to_string(),
                    kind: ChannelKind::Color,
                    sample_type: ChannelSampleType::F32,
                    samples: ChannelSamples::F32(vec![0.5; pixel_count]),
                    sampling: (1, 1),
                    quantize_linearly: true,
                },
                ImageChannel {
                    name: "G".to_string(),
                    kind: ChannelKind::Color,
                    sample_type: ChannelSampleType::F32,
                    samples: ChannelSamples::F32(vec![0.3; pixel_count]),
                    sampling: (1, 1),
                    quantize_linearly: true,
                },
            ],
        }
    }

    #[test]
    fn test_resize_layer() {
        let layer = make_test_layer(64, 64);
        let resized = resize_layer(&layer, 32, 32, Filter::Bilinear).unwrap();
        
        assert_eq!(resized.width, 32);
        assert_eq!(resized.height, 32);
        assert_eq!(resized.channels.len(), 2);
        assert_eq!(resized.name, "test");
    }

    #[test]
    fn test_crop_layer() {
        let layer = make_test_layer(64, 64);
        let cropped = crop_layer(&layer, 10, 10, 32, 32).unwrap();
        
        assert_eq!(cropped.width, 32);
        assert_eq!(cropped.height, 32);
    }

    #[test]
    fn test_crop_out_of_bounds() {
        let layer = make_test_layer(64, 64);
        let result = crop_layer(&layer, 50, 50, 32, 32);
        
        assert!(result.is_err());
    }

    #[test]
    fn test_blur_layer() {
        let layer = make_test_layer(64, 64);
        let blurred = blur_layer(&layer, 3).unwrap();
        
        assert_eq!(blurred.width, 64);
        assert_eq!(blurred.height, 64);
    }
}
