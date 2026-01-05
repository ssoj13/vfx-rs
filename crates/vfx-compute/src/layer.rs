//! Direct ImageLayer processing with channel-aware GPU compute.
//!
//! Processes ImageLayer channels directly on GPU without full RGBA conversion.
//! Supports arbitrary channel grouping and U32 passthrough for ID/mask data.
//!
//! # Architecture
//!
//! ```text
//! ImageLayer
//!     ├── RGB (F32) ─── ChannelGroup ─── GPU color ops ─── back
//!     ├── A (F32) ───── ChannelGroup ─── GPU spatial ops ── back  
//!     ├── Z (F32) ───── ChannelGroup ─── GPU spatial ops ── back
//!     └── ID (U32) ──── passthrough (copy) ────────────── back
//! ```
//!
//! # Example
//!
//! ```ignore
//! use vfx_compute::{LayerProcessor, ComputeOp};
//! use vfx_io::ImageLayer;
//!
//! let mut proc = LayerProcessor::auto()?;
//!
//! // Process RGB channels with color ops
//! let output = proc.process_layer(&layer, &[
//!     ComputeOp::Exposure(1.5),
//!     ComputeOp::Saturation(1.2),
//! ])?;
//! ```

#[cfg(feature = "io")]
use vfx_io::{ImageLayer, ImageChannel, ChannelSamples, ChannelSampleType};

use crate::{ComputeImage, ComputeResult, Processor, ComputeOp};
use crate::pipeline::ComputePipeline;

// ============================================================================
// Channel Classification
// ============================================================================

/// Channel group for batch processing.
#[cfg(feature = "io")]
#[derive(Debug, Clone)]
pub struct ChannelGroup {
    /// Channel indices in source layer.
    pub indices: Vec<usize>,
    /// Channel names.
    pub names: Vec<String>,
    /// Common sample type (must match for grouping).
    pub sample_type: ChannelSampleType,
}

#[cfg(feature = "io")]
impl ChannelGroup {
    /// Number of channels in group.
    pub fn len(&self) -> usize {
        self.indices.len()
    }

    /// Check if group is empty.
    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }

    /// Bytes per pixel for this group.
    pub fn bytes_per_pixel(&self) -> u64 {
        let bps = match self.sample_type {
            ChannelSampleType::F32 => 4,
            ChannelSampleType::F16 => 2,
            ChannelSampleType::U32 => 4,
        };
        bps * self.len() as u64
    }
}

/// Classification of layer channels.
#[cfg(feature = "io")]
#[derive(Debug)]
pub struct ChannelClassification {
    /// RGB color channels (for color ops).
    pub color: Option<ChannelGroup>,
    /// Alpha channel.
    pub alpha: Option<ChannelGroup>,
    /// Other F32 channels (depth, normals, etc).
    pub other_f32: Vec<ChannelGroup>,
    /// U32 channels (IDs, masks) - passthrough only.
    pub passthrough: Vec<usize>,
}

#[cfg(feature = "io")]
impl ChannelClassification {
    /// Classify layer channels by type and semantic meaning.
    pub fn from_layer(layer: &ImageLayer) -> Self {
        let mut color_indices = Vec::new();
        let mut color_names = Vec::new();
        let mut alpha_idx = None;
        let mut alpha_name = None;
        let mut other_f32 = Vec::new();
        let mut passthrough = Vec::new();

        for (idx, ch) in layer.channels.iter().enumerate() {
            match ch.sample_type {
                ChannelSampleType::U32 => {
                    // U32 = ID data, passthrough
                    passthrough.push(idx);
                }
                ChannelSampleType::F32 | ChannelSampleType::F16 => {
                    // Check semantic meaning
                    let name_upper = ch.name.to_uppercase();
                    match name_upper.as_str() {
                        "R" | "RED" => {
                            color_indices.push(idx);
                            color_names.push(ch.name.clone());
                        }
                        "G" | "GREEN" => {
                            color_indices.push(idx);
                            color_names.push(ch.name.clone());
                        }
                        "B" | "BLUE" => {
                            color_indices.push(idx);
                            color_names.push(ch.name.clone());
                        }
                        "A" | "ALPHA" => {
                            alpha_idx = Some(idx);
                            alpha_name = Some(ch.name.clone());
                        }
                        _ => {
                            // Other channels: Z, N.x, N.y, N.z, etc.
                            other_f32.push(ChannelGroup {
                                indices: vec![idx],
                                names: vec![ch.name.clone()],
                                sample_type: ch.sample_type,
                            });
                        }
                    }
                }
            }
        }

        // Sort color channels to ensure RGB order
        let color = if !color_indices.is_empty() {
            // Sort by R, G, B order
            let mut sorted: Vec<(usize, String)> = color_indices
                .into_iter()
                .zip(color_names)
                .collect();
            sorted.sort_by(|(_, a), (_, b)| {
                let order = |n: &str| match n.to_uppercase().as_str() {
                    "R" | "RED" => 0,
                    "G" | "GREEN" => 1,
                    "B" | "BLUE" => 2,
                    _ => 3,
                };
                order(a).cmp(&order(b))
            });

            let (indices, names): (Vec<_>, Vec<_>) = sorted.into_iter().unzip();
            let sample_type = layer.channels[indices[0]].sample_type;

            Some(ChannelGroup { indices, names, sample_type })
        } else {
            None
        };

        let alpha = alpha_idx.map(|idx| ChannelGroup {
            indices: vec![idx],
            names: vec![alpha_name.unwrap()],
            sample_type: layer.channels[idx].sample_type,
        });

        Self { color, alpha, other_f32, passthrough }
    }

    /// Total processable channels.
    pub fn processable_count(&self) -> usize {
        let color_count = self.color.as_ref().map(|g| g.len()).unwrap_or(0);
        let alpha_count = self.alpha.as_ref().map(|g| g.len()).unwrap_or(0);
        let other_count: usize = self.other_f32.iter().map(|g| g.len()).sum();
        color_count + alpha_count + other_count
    }

    /// Estimate memory for processing (in bytes).
    pub fn estimate_memory(&self, width: u32, height: u32) -> u64 {
        let pixels = (width as u64) * (height as u64);
        
        let color_bytes = self.color.as_ref()
            .map(|g| g.bytes_per_pixel())
            .unwrap_or(0);
        let alpha_bytes = self.alpha.as_ref()
            .map(|g| g.bytes_per_pixel())
            .unwrap_or(0);
        let other_bytes: u64 = self.other_f32.iter()
            .map(|g| g.bytes_per_pixel())
            .sum();

        // Processing overhead: src + dst + intermediate = 3x
        pixels * (color_bytes + alpha_bytes + other_bytes) * 3
    }
}

// ============================================================================
// LayerProcessor
// ============================================================================

/// Direct ImageLayer processor with channel-aware GPU compute.
#[cfg(feature = "io")]
pub struct LayerProcessor {
    pipeline: ComputePipeline,
}

#[cfg(feature = "io")]
impl LayerProcessor {
    /// Create with auto-detected backend.
    pub fn auto() -> ComputeResult<Self> {
        let processor = Processor::auto()?;
        Ok(Self {
            pipeline: ComputePipeline::new(processor),
        })
    }

    /// Create with specific processor.
    pub fn new(processor: Processor) -> Self {
        Self {
            pipeline: ComputePipeline::new(processor),
        }
    }

    /// Access underlying processor.
    pub fn processor(&self) -> &Processor {
        self.pipeline.processor()
    }

    /// Process entire layer with operations.
    ///
    /// - Color ops (CDL, LUT, Saturation) apply to RGB channels only
    /// - Spatial ops (Blur, Resize) apply to all processable channels
    /// - U32 channels are copied unchanged
    pub fn process_layer(
        &mut self,
        layer: &ImageLayer,
        ops: &[ComputeOp],
    ) -> ComputeResult<ImageLayer> {
        let classification = ChannelClassification::from_layer(layer);
        
        // Separate color ops from spatial ops
        let (color_ops, spatial_ops): (Vec<&ComputeOp>, Vec<&ComputeOp>) = ops.iter()
            .partition(|op| op.is_color_op());

        let mut output_channels = vec![None; layer.channels.len()];

        // 1. Process color channels (RGB together)
        if let Some(ref color_group) = classification.color {
            // Use color ops if available, otherwise spatial ops
            let ops_for_color: Vec<&ComputeOp> = if !color_ops.is_empty() {
                color_ops.clone()
            } else {
                spatial_ops.clone()
            };
            let processed = self.process_channel_group(
                layer,
                color_group,
                &ops_for_color,
            )?;
            
            for (group_idx, &layer_idx) in color_group.indices.iter().enumerate() {
                output_channels[layer_idx] = Some(processed[group_idx].clone());
            }

            // If we have both color and spatial ops, apply spatial to result
            if !color_ops.is_empty() && !spatial_ops.is_empty() {
                // TODO: Apply spatial ops to color result
            }
        }

        // 2. Process alpha (spatial ops only)
        if let Some(ref alpha_group) = classification.alpha {
            let processed = self.process_channel_group(layer, alpha_group, &spatial_ops)?;
            for (group_idx, &layer_idx) in alpha_group.indices.iter().enumerate() {
                output_channels[layer_idx] = Some(processed[group_idx].clone());
            }
        }

        // 3. Process other F32 channels (spatial ops only)
        for group in &classification.other_f32 {
            let processed = self.process_channel_group(layer, group, &spatial_ops)?;
            for (group_idx, &layer_idx) in group.indices.iter().enumerate() {
                output_channels[layer_idx] = Some(processed[group_idx].clone());
            }
        }

        // 4. Passthrough U32 channels
        for &idx in &classification.passthrough {
            output_channels[idx] = Some(layer.channels[idx].clone());
        }

        // Reconstruct layer
        let channels: Vec<ImageChannel> = output_channels
            .into_iter()
            .enumerate()
            .map(|(idx, ch)| ch.unwrap_or_else(|| layer.channels[idx].clone()))
            .collect();

        Ok(ImageLayer {
            name: layer.name.clone(),
            width: layer.width,
            height: layer.height,
            channels,
        })
    }

    /// Process a specific group of channels.
    fn process_channel_group(
        &mut self,
        layer: &ImageLayer,
        group: &ChannelGroup,
        ops: &[&ComputeOp],
    ) -> ComputeResult<Vec<ImageChannel>> {
        if ops.is_empty() {
            // No ops, return copies
            return Ok(group.indices.iter()
                .map(|&idx| layer.channels[idx].clone())
                .collect());
        }

        let pixel_count = (layer.width * layer.height) as usize;
        let num_channels = group.len();

        // Pack channels into interleaved f32 buffer
        let mut data = vec![0.0f32; pixel_count * num_channels];
        for (ch_idx, &layer_idx) in group.indices.iter().enumerate() {
            let samples = layer.channels[layer_idx].samples.to_f32();
            for (px, &val) in samples.iter().enumerate() {
                if px < pixel_count {
                    data[px * num_channels + ch_idx] = val;
                }
            }
        }

        // Create ComputeImage
        let mut img = ComputeImage::from_f32(
            data,
            layer.width,
            layer.height,
            num_channels as u32,
        )?;

        // Apply operations
        for op in ops {
            img = self.apply_op(img, op)?;
        }

        // Unpack back to channels
        let result_data = img.data();
        let mut channels = Vec::with_capacity(num_channels);

        for (ch_idx, &layer_idx) in group.indices.iter().enumerate() {
            let source = &layer.channels[layer_idx];
            let mut samples = Vec::with_capacity(pixel_count);
            
            for px in 0..pixel_count {
                samples.push(result_data[px * num_channels + ch_idx]);
            }

            channels.push(ImageChannel {
                name: source.name.clone(),
                kind: source.kind,
                sample_type: source.sample_type,
                samples: ChannelSamples::F32(samples),
                sampling: source.sampling,
                quantize_linearly: source.quantize_linearly,
            });
        }

        Ok(channels)
    }

    /// Apply single operation.
    fn apply_op(&mut self, mut img: ComputeImage, op: &ComputeOp) -> ComputeResult<ComputeImage> {
        let proc = self.pipeline.processor();
        
        match op {
            ComputeOp::Exposure(stops) => {
                proc.apply_exposure(&mut img, *stops)?;
            }
            ComputeOp::Saturation(sat) => {
                proc.apply_saturation(&mut img, *sat)?;
            }
            ComputeOp::Contrast(c) => {
                proc.apply_contrast(&mut img, *c)?;
            }
            ComputeOp::Cdl(cdl) => {
                proc.apply_cdl(&mut img, cdl)?;
            }
            ComputeOp::Matrix(m) => {
                proc.apply_matrix(&mut img, m)?;
            }
            ComputeOp::Lut1D { lut, channels } => {
                proc.apply_lut1d(&mut img, lut, *channels)?;
            }
            ComputeOp::Lut3D { lut, size } => {
                proc.apply_lut3d(&mut img, lut, *size)?;
            }
            ComputeOp::Blur(radius) => {
                proc.blur(&mut img, *radius)?;
            }
            ComputeOp::Sharpen(amount) => {
                proc.sharpen(&mut img, *amount)?;
            }
            ComputeOp::Resize { width, height, filter } => {
                img = proc.resize(&img, *width, *height, *filter)?;
            }
            ComputeOp::Crop { x, y, w, h } => {
                img = proc.crop(&img, *x, *y, *w, *h)?;
            }
            ComputeOp::FlipH => {
                proc.flip_h(&mut img)?;
            }
            ComputeOp::FlipV => {
                proc.flip_v(&mut img)?;
            }
            ComputeOp::Rotate90(n) => {
                img = proc.rotate_90(&img, *n)?;
            }
        }
        
        Ok(img)
    }
}

// ============================================================================
// ComputeOp Extensions
// ============================================================================

impl ComputeOp {
    /// Check if this is a color operation (requires RGB together).
    pub fn is_color_op(&self) -> bool {
        matches!(
            self,
            Self::Matrix(_)
                | Self::Cdl(_)
                | Self::Lut1D { .. }
                | Self::Lut3D { .. }
                | Self::Exposure(_)
                | Self::Saturation(_)
                | Self::Contrast(_)
        )
    }

    /// Check if this is a spatial operation (works per-channel).
    pub fn is_spatial_op(&self) -> bool {
        matches!(
            self,
            Self::Blur(_)
                | Self::Sharpen(_)
                | Self::Resize { .. }
                | Self::Crop { .. }
                | Self::FlipH
                | Self::FlipV
                | Self::Rotate90(_)
        )
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
#[cfg(feature = "io")]
mod tests {
    use super::*;

    fn make_test_layer() -> ImageLayer {
        use vfx_io::{ImageChannel, ChannelKind, ChannelSampleType, ChannelSamples};

        let pixel_count = 4; // 2x2
        
        ImageLayer {
            name: "test".to_string(),
            width: 2,
            height: 2,
            channels: vec![
                ImageChannel {
                    name: "R".to_string(),
                    kind: ChannelKind::Color,
                    sample_type: ChannelSampleType::F32,
                    samples: ChannelSamples::F32(vec![0.5; pixel_count]),
                    sampling: (1, 1),
                    quantize_linearly: false,
                },
                ImageChannel {
                    name: "G".to_string(),
                    kind: ChannelKind::Color,
                    sample_type: ChannelSampleType::F32,
                    samples: ChannelSamples::F32(vec![0.3; pixel_count]),
                    sampling: (1, 1),
                    quantize_linearly: false,
                },
                ImageChannel {
                    name: "B".to_string(),
                    kind: ChannelKind::Color,
                    sample_type: ChannelSampleType::F32,
                    samples: ChannelSamples::F32(vec![0.2; pixel_count]),
                    sampling: (1, 1),
                    quantize_linearly: false,
                },
                ImageChannel {
                    name: "ID".to_string(),
                    kind: ChannelKind::Generic,
                    sample_type: ChannelSampleType::U32,
                    samples: ChannelSamples::U32(vec![1, 2, 3, 4]),
                    sampling: (1, 1),
                    quantize_linearly: false,
                },
            ],
        }
    }

    #[test]
    fn test_channel_classification() {
        let layer = make_test_layer();
        let class = ChannelClassification::from_layer(&layer);

        assert!(class.color.is_some());
        assert_eq!(class.color.as_ref().unwrap().len(), 3);
        assert!(class.alpha.is_none());
        assert_eq!(class.passthrough.len(), 1); // ID channel
    }

    #[test]
    fn test_channel_group_bytes() {
        let group = ChannelGroup {
            indices: vec![0, 1, 2],
            names: vec!["R".into(), "G".into(), "B".into()],
            sample_type: ChannelSampleType::F32,
        };
        assert_eq!(group.bytes_per_pixel(), 12); // 3 * 4 bytes
    }

    #[test]
    fn test_op_classification() {
        assert!(ComputeOp::Exposure(1.0).is_color_op());
        assert!(ComputeOp::Saturation(1.0).is_color_op());
        assert!(!ComputeOp::Blur(1.0).is_color_op());
        
        assert!(ComputeOp::Blur(1.0).is_spatial_op());
        assert!(ComputeOp::FlipH.is_spatial_op());
        assert!(!ComputeOp::Exposure(1.0).is_spatial_op());
    }
}
