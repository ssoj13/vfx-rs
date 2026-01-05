//! Channel manipulation commands: shuffle and extract.
//!
//! Provides commands to rearrange channels (shuffle) and extract
//! specific channels to a new image.

use crate::{ChannelExtractArgs, ChannelShuffleArgs};
use tracing::{debug, info, trace};
use anyhow::{Result, bail};
use vfx_io::ImageData;

/// Runs channel-shuffle: rearrange channels according to pattern.
///
/// Pattern format: "RGB", "BGR", "RGBA", "BGRA", "RRR", etc.
/// Each letter specifies which source channel to use in that position.
/// Special chars: 0 = black, 1 = white
///
/// Examples:
///   "BGR"  - swap R and B
///   "RRR"  - grayscale from red
///   "RGB1" - RGB with alpha=1
pub fn run_shuffle(args: ChannelShuffleArgs, verbose: u8) -> Result<()> {
    let input = super::load_image(&args.input)?;
    let pattern = args.pattern.to_uppercase();
    
    if verbose > 0 {
        println!(
            "Shuffling {} ({}x{}, {} ch) with pattern '{}'",
            args.input.display(),
            input.width,
            input.height,
            input.channels,
            pattern
        );
    }
    
    let output = shuffle_channels(&input, &pattern)?;
    
    super::save_image(&args.output, &output)?;
    
    if verbose > 0 {
        println!("Saved to {} ({} channels)", args.output.display(), output.channels);
    }
    
    Ok(())
}

/// Runs channel-extract: extract specific channels to a new image.
///
/// Channels can be specified by name (R, G, B, A, Z, ID) or index (0, 1, 2).
pub fn run_extract(args: ChannelExtractArgs, verbose: u8) -> Result<()> {
    let input = super::load_image(&args.input)?;
    
    if verbose > 0 {
        println!(
            "Extracting channels [{}] from {} ({}x{}, {} ch)",
            args.channels.join(", "),
            args.input.display(),
            input.width,
            input.height,
            input.channels
        );
    }
    
    let output = extract_channels(&input, &args.channels)?;
    
    super::save_image(&args.output, &output)?;
    
    if verbose > 0 {
        println!("Saved to {} ({} channels)", args.output.display(), output.channels);
    }
    
    Ok(())
}

/// Shuffle channels according to pattern.
fn shuffle_channels(input: &ImageData, pattern: &str) -> Result<ImageData> {
    let pixel_count = input.pixel_count();
    let out_channels = pattern.len();
    let in_channels = input.channels as usize;
    
    if out_channels == 0 {
        bail!("Empty shuffle pattern");
    }
    
    // Convert to f32 for processing
    let src_data = input.to_f32();
    let mut output_data = vec![0.0f32; pixel_count * out_channels];
    
    // Build channel map: pattern char -> source channel index or special value
    for (out_idx, ch) in pattern.chars().enumerate() {
        let src_channel: Option<usize> = match ch {
            'R' | 'r' => Some(0),
            'G' | 'g' => Some(1),
            'B' | 'b' => Some(2),
            'A' | 'a' => Some(3),
            '0' => None, // Black
            '1' => None, // White (handled separately)
            c if c.is_ascii_digit() => {
                let idx = c.to_digit(10).unwrap() as usize;
                if idx >= in_channels {
                    bail!("Channel index {} out of range (image has {} channels)", idx, in_channels);
                }
                Some(idx)
            }
            _ => bail!("Unknown channel specifier '{}' in pattern", ch),
        };
        
        // Fill output channel
        for px in 0..pixel_count {
            let value = match ch {
                '0' => 0.0,
                '1' => 1.0,
                _ => {
                    if let Some(src_idx) = src_channel {
                        if src_idx < in_channels {
                            src_data[px * in_channels + src_idx]
                        } else {
                            0.0 // Missing channel = black
                        }
                    } else {
                        0.0
                    }
                }
            };
            output_data[px * out_channels + out_idx] = value;
        }
    }
    
    Ok(ImageData::from_f32(input.width, input.height, out_channels as u32, output_data))
}

/// Extract specific channels by name or index.
fn extract_channels(input: &ImageData, channel_specs: &[String]) -> Result<ImageData> {
    if channel_specs.is_empty() {
        bail!("No channels specified for extraction");
    }
    
    let pixel_count = input.pixel_count();
    let out_channels = channel_specs.len();
    let in_channels = input.channels as usize;
    
    // Convert to f32 for processing
    let src_data = input.to_f32();
    let mut output_data = vec![0.0f32; pixel_count * out_channels];
    
    for (out_idx, spec) in channel_specs.iter().enumerate() {
        let src_idx = parse_channel_spec(spec, in_channels)?;
        
        for px in 0..pixel_count {
            output_data[px * out_channels + out_idx] = src_data[px * in_channels + src_idx];
        }
    }
    
    Ok(ImageData::from_f32(input.width, input.height, out_channels as u32, output_data))
}

/// Parse channel specification: name (R, G, B, A) or index (0, 1, 2).
fn parse_channel_spec(spec: &str, num_channels: usize) -> Result<usize> {
    // Try as index first
    if let Ok(idx) = spec.parse::<usize>() {
        if idx >= num_channels {
            bail!("Channel index {} out of range (image has {} channels)", idx, num_channels);
        }
        return Ok(idx);
    }
    
    // Try as name
    let idx = match spec.to_uppercase().as_str() {
        "R" | "RED" => 0,
        "G" | "GREEN" => 1,
        "B" | "BLUE" => 2,
        "A" | "ALPHA" => 3,
        "Z" | "DEPTH" => 4, // Common depth channel
        _ => bail!("Unknown channel '{}'. Use R/G/B/A or numeric index", spec),
    };
    
    if idx >= num_channels {
        bail!("Channel '{}' (index {}) not present (image has {} channels)", spec, idx, num_channels);
    }
    
    Ok(idx)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    fn make_rgba_image() -> ImageData {
        // 2x2 RGBA image with distinct values
        ImageData::from_f32(2, 2, 4, vec![
            // Pixel 0: R=1, G=0.5, B=0.25, A=1
            1.0, 0.5, 0.25, 1.0,
            // Pixel 1: R=0.8, G=0.6, B=0.4, A=0.9
            0.8, 0.6, 0.4, 0.9,
            // Pixel 2
            0.7, 0.5, 0.3, 0.8,
            // Pixel 3
            0.6, 0.4, 0.2, 0.7,
        ])
    }
    
    #[test]
    fn test_shuffle_bgr() {
        let input = make_rgba_image();
        let output = shuffle_channels(&input, "BGR").unwrap();
        
        assert_eq!(output.channels, 3);
        let data = output.to_f32();
        // First pixel: B, G, R = 0.25, 0.5, 1.0
        assert_eq!(data[0], 0.25);
        assert_eq!(data[1], 0.5);
        assert_eq!(data[2], 1.0);
    }
    
    #[test]
    fn test_shuffle_rrr() {
        let input = make_rgba_image();
        let output = shuffle_channels(&input, "RRR").unwrap();
        
        assert_eq!(output.channels, 3);
        let data = output.to_f32();
        // First pixel: all R = 1.0
        assert_eq!(data[0], 1.0);
        assert_eq!(data[1], 1.0);
        assert_eq!(data[2], 1.0);
    }
    
    #[test]
    fn test_shuffle_with_constants() {
        let input = make_rgba_image();
        let output = shuffle_channels(&input, "RGB1").unwrap();
        
        assert_eq!(output.channels, 4);
        let data = output.to_f32();
        // First pixel alpha = 1.0
        assert_eq!(data[3], 1.0);
    }
    
    #[test]
    fn test_extract_single() {
        let input = make_rgba_image();
        let output = extract_channels(&input, &["R".to_string()]).unwrap();
        
        assert_eq!(output.channels, 1);
        let data = output.to_f32();
        assert_eq!(data[0], 1.0); // First pixel R
    }
    
    #[test]
    fn test_extract_multiple() {
        let input = make_rgba_image();
        let output = extract_channels(&input, &["B".to_string(), "A".to_string()]).unwrap();
        
        assert_eq!(output.channels, 2);
        let data = output.to_f32();
        // First pixel: B=0.25, A=1.0
        assert_eq!(data[0], 0.25);
        assert_eq!(data[1], 1.0);
    }
}
