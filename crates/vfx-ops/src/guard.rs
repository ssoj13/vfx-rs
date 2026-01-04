//! Channel validation guards for image operations.
//!
//! Some operations (blur, resize, color transforms) should only be applied
//! to color-like channels. Applying them to ID passes, masks, or other
//! non-color data can produce incorrect results.
//!
//! This module provides validation functions to guard against such misuse.
//!
//! # Example
//!
//! ```rust,ignore
//! use vfx_ops::guard::ensure_color_channels;
//!
//! // Fails if image has Id/Mask/Generic channels
//! ensure_color_channels(&image, "blur", false)?;
//!
//! // Bypass check with allow_non_color flag
//! ensure_color_channels(&image, "blur", true)?;
//! ```

use vfx_io::{ChannelKind, ImageData, ImageLayer};
use crate::{OpsError, OpsResult};

/// Validates that an image layer contains only color-processable channels.
///
/// Color-processable channels are:
/// - `Color` (R, G, B, Y, etc.)
/// - `Alpha` (A, opacity)
/// - `Depth` (Z, distance)
///
/// Non-color channels that will fail validation:
/// - `Id` - object/material IDs (integer data)
/// - `Mask` - binary masks (should not be filtered)
/// - `Generic` - unknown purpose
///
/// # Arguments
///
/// * `layer` - The image layer to validate
/// * `op` - Name of the operation (for error messages)
/// * `allow_non_color` - If true, bypasses validation
///
/// # Returns
///
/// `Ok(())` if all channels are color-processable or bypass is enabled.
/// `Err(NonColorChannel)` if a non-color channel is found.
pub fn ensure_color_channels_layer(
    layer: &ImageLayer,
    op: &str,
    allow_non_color: bool,
) -> OpsResult<()> {
    if allow_non_color {
        return Ok(());
    }

    for channel in &layer.channels {
        match channel.kind {
            // These are safe for color processing
            ChannelKind::Color | ChannelKind::Alpha | ChannelKind::Depth => {}
            // These should not have color operations applied
            ChannelKind::Id | ChannelKind::Mask | ChannelKind::Generic => {
                return Err(OpsError::NonColorChannel {
                    channel: channel.name.clone(),
                    kind: format!("{:?}", channel.kind),
                    op: op.to_string(),
                });
            }
        }
    }

    Ok(())
}

/// Validates that an image contains only color-processable channels.
///
/// Convenience wrapper around [`ensure_color_channels_layer`] that
/// converts ImageData to a temporary layer for validation.
///
/// # Arguments
///
/// * `image` - The image to validate
/// * `op` - Name of the operation (for error messages)
/// * `allow_non_color` - If true, bypasses validation
///
/// # Example
///
/// ```rust,ignore
/// use vfx_ops::guard::ensure_color_channels;
///
/// let image = vfx_io::read("render.exr")?;
/// ensure_color_channels(&image, "gaussian_blur", false)?;
/// // Now safe to apply blur
/// ```
pub fn ensure_color_channels(
    image: &ImageData,
    op: &str,
    allow_non_color: bool,
) -> OpsResult<()> {
    let layer = image.to_layer("input");
    ensure_color_channels_layer(&layer, op, allow_non_color)
}

#[cfg(test)]
mod tests {
    use super::*;
    use vfx_io::{ImageChannel, ChannelSampleType, ChannelSamples};

    fn make_layer(channels: Vec<(&str, ChannelKind)>) -> ImageLayer {
        ImageLayer {
            name: "test".to_string(),
            width: 64,
            height: 64,
            channels: channels
                .into_iter()
                .map(|(name, kind)| ImageChannel {
                    name: name.to_string(),
                    kind,
                    sample_type: ChannelSampleType::F32,
                    samples: ChannelSamples::F32(vec![0.0; 64 * 64]),
                    sampling: (1, 1),
                    quantize_linearly: true,
                })
                .collect(),
        }
    }

    #[test]
    fn test_color_channels_pass() {
        let layer = make_layer(vec![
            ("R", ChannelKind::Color),
            ("G", ChannelKind::Color),
            ("B", ChannelKind::Color),
            ("A", ChannelKind::Alpha),
        ]);
        assert!(ensure_color_channels_layer(&layer, "blur", false).is_ok());
    }

    #[test]
    fn test_depth_channel_passes() {
        let layer = make_layer(vec![
            ("R", ChannelKind::Color),
            ("Z", ChannelKind::Depth),
        ]);
        assert!(ensure_color_channels_layer(&layer, "resize", false).is_ok());
    }

    #[test]
    fn test_id_channel_fails() {
        let layer = make_layer(vec![
            ("R", ChannelKind::Color),
            ("objectId", ChannelKind::Id),
        ]);
        let result = ensure_color_channels_layer(&layer, "blur", false);
        assert!(result.is_err());
        
        let err = result.unwrap_err();
        assert!(matches!(err, OpsError::NonColorChannel { .. }));
    }

    #[test]
    fn test_mask_channel_fails() {
        let layer = make_layer(vec![
            ("matte", ChannelKind::Mask),
        ]);
        assert!(ensure_color_channels_layer(&layer, "sharpen", false).is_err());
    }

    #[test]
    fn test_allow_non_color_bypasses() {
        let layer = make_layer(vec![
            ("objectId", ChannelKind::Id),
            ("cryptomatte", ChannelKind::Generic),
        ]);
        // With allow_non_color=true, should pass
        assert!(ensure_color_channels_layer(&layer, "blur", true).is_ok());
    }
}
