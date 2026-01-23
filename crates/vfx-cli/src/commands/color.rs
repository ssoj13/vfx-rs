//! Color transform command
//!
//! Applies color adjustments: exposure, gamma, saturation, transfer functions,
//! and color space conversion.
//! Supports `--layer` for processing specific layers in multi-layer EXR.
//!
//! ## Color Space Conversion
//!
//! Use `--from` and `--to` for gamut conversion:
//! - sRGB, linear_srgb, ACEScg, ACES2065, ACEScct, ACEScc
//! - Rec709, Rec2020, DCI-P3, Display P3
//!
//! ## Transfer Functions
//!
//! Display-referred:
//! - `srgb` / `srgb_to_linear`: Decode sRGB to linear
//! - `linear_to_srgb`: Encode linear to sRGB
//! - `rec709` / `rec709_to_linear`: Decode Rec.709 OETF to linear
//! - `linear_to_rec709`: Encode linear to Rec.709 OETF
//!
//! HDR:
//! - `pq` / `pq_to_linear`: Decode PQ (ST.2084) to linear (nits)
//! - `linear_to_pq`: Encode linear to PQ
//! - `hlg` / `hlg_to_linear`: Decode HLG (BT.2100) to linear
//! - `linear_to_hlg`: Encode linear to HLG
//!
//! Camera Log:
//! - `logc` / `logc_to_linear`: Decode ARRI LogC3 to linear
//! - `linear_to_logc`: Encode linear to ARRI LogC3
//! - `logc4` / `logc4_to_linear`: Decode ARRI LogC4 to linear
//! - `linear_to_logc4`: Encode linear to ARRI LogC4
//! - `slog3` / `slog3_to_linear`: Decode Sony S-Log3 to linear
//! - `linear_to_slog3`: Encode linear to Sony S-Log3
//! - `vlog` / `vlog_to_linear`: Decode Panasonic V-Log to linear
//! - `linear_to_vlog`: Encode linear to Panasonic V-Log

use crate::ColorArgs;
#[allow(unused_imports)]
use tracing::{debug, info, trace};
use anyhow::{Result, bail};
use vfx_core::pixel::{REC709_LUMA_R, REC709_LUMA_G, REC709_LUMA_B};
use vfx_core::ColorSpaceId;
use vfx_io::ImageData;
use vfx_primaries::conversion_matrix;
use vfx_transfer::{srgb, rec709, pq, hlg, log_c, log_c4, s_log3, v_log};

pub fn run(args: ColorArgs, verbose: u8, allow_non_color: bool) -> Result<()> {
    trace!(input = %args.input.display(), "color::run");
    
    let image = super::load_image_layer(&args.input, args.layer.as_deref())?;
    super::ensure_color_processing(&image, "color", allow_non_color)?;
    let mut data = image.to_f32();
    let w = image.width as usize;
    let h = image.height as usize;
    let c = image.channels as usize;

    info!(
        exposure = ?args.exposure,
        gamma = ?args.gamma,
        saturation = ?args.saturation,
        transfer = ?args.transfer,
        from = ?args.from,
        to = ?args.to,
        "Applying color transforms"
    );
    
    if verbose > 0 {
        println!("Applying color transforms to {}", args.input.display());
    }

    // Apply color space conversion (gamut mapping)
    if let (Some(from_str), Some(to_str)) = (&args.from, &args.to) {
        let from_cs = ColorSpaceId::from_name(from_str)
            .ok_or_else(|| anyhow::anyhow!("Unknown source color space: {}. Available: sRGB, linear_srgb, ACEScg, ACES2065, ACEScct, ACEScc, Rec709, Rec2020, DCI-P3, Display_P3", from_str))?;
        let to_cs = ColorSpaceId::from_name(to_str)
            .ok_or_else(|| anyhow::anyhow!("Unknown target color space: {}. Available: sRGB, linear_srgb, ACEScg, ACES2065, ACEScct, ACEScc, Rec709, Rec2020, DCI-P3, Display_P3", to_str))?;
        
        if verbose > 0 { println!("  Color space: {} -> {}", from_cs.name(), to_cs.name()); }
        
        let matrix = conversion_matrix(from_cs, to_cs);
        apply_matrix(&mut data, w, h, c, &matrix);
    } else if args.from.is_some() || args.to.is_some() {
        bail!("Both --from and --to must be specified for color space conversion");
    }

    // Apply exposure adjustment (RGB only, preserve alpha)
    if let Some(stops) = args.exposure {
        if verbose > 0 { println!("  Exposure: {:+.2} stops", stops); }
        let factor = 2.0f32.powf(stops);
        let rgb_channels = c.min(3);
        for pixel in 0..(w * h) {
            let base = pixel * c;
            for ch in 0..rgb_channels {
                data[base + ch] *= factor;
            }
        }
    }

    // Apply gamma (RGB only, preserve alpha)
    if let Some(gamma) = args.gamma {
        if verbose > 0 { println!("  Gamma: {:.2}", gamma); }
        let rgb_channels = c.min(3);
        for pixel in 0..(w * h) {
            let base = pixel * c;
            for ch in 0..rgb_channels {
                if data[base + ch] > 0.0 {
                    data[base + ch] = data[base + ch].powf(gamma);
                }
            }
        }
    }

    // Apply saturation
    if let Some(sat) = args.saturation {
        if verbose > 0 { println!("  Saturation: {:.2}", sat); }
        apply_saturation(&mut data, w, h, c, sat);
    }

    // Apply transfer function
    if let Some(ref tf) = args.transfer {
        if verbose > 0 { println!("  Transfer: {}", tf); }
        apply_transfer(&mut data, tf);
    }

    let output = ImageData::from_f32(image.width, image.height, image.channels, data);
    super::save_image_layer(&args.output, &output, args.layer.as_deref())?;

    if verbose > 0 {
        println!("Done.");
    }

    Ok(())
}

fn apply_saturation(data: &mut [f32], width: usize, height: usize, channels: usize, sat: f32) {
    if channels < 3 { return; }

    for y in 0..height {
        for x in 0..width {
            let idx = (y * width + x) * channels;
            let r = data[idx];
            let g = data[idx + 1];
            let b = data[idx + 2];

            // Luminance (Rec.709)
            let lum = REC709_LUMA_R * r + REC709_LUMA_G * g + REC709_LUMA_B * b;

            // Interpolate between grayscale and color
            data[idx] = lum + (r - lum) * sat;
            data[idx + 1] = lum + (g - lum) * sat;
            data[idx + 2] = lum + (b - lum) * sat;
        }
    }
}

fn apply_transfer(data: &mut [f32], tf: &str) {
    match tf.to_lowercase().as_str() {
        // sRGB: decode (EOTF) and encode (OETF)
        "srgb" | "srgb_to_linear" => {
            for v in data.iter_mut() {
                *v = srgb::eotf(*v);
            }
        }
        "linear_to_srgb" => {
            for v in data.iter_mut() {
                *v = srgb::oetf(*v);
            }
        }
        // Rec.709: decode and encode
        "rec709" | "rec709_to_linear" => {
            for v in data.iter_mut() {
                *v = rec709::eotf(*v);
            }
        }
        "linear_to_rec709" => {
            for v in data.iter_mut() {
                *v = rec709::oetf(*v);
            }
        }
        // PQ (ST.2084): HDR decode/encode
        "pq" | "pq_to_linear" | "st2084" | "st2084_to_linear" => {
            for v in data.iter_mut() {
                *v = pq::eotf(*v);
            }
        }
        "linear_to_pq" | "linear_to_st2084" => {
            for v in data.iter_mut() {
                *v = pq::oetf(*v);
            }
        }
        // HLG (BT.2100): HDR decode/encode
        "hlg" | "hlg_to_linear" | "bt2100" | "bt2100_to_linear" => {
            for v in data.iter_mut() {
                *v = hlg::eotf(*v);
            }
        }
        "linear_to_hlg" | "linear_to_bt2100" => {
            for v in data.iter_mut() {
                *v = hlg::oetf(*v);
            }
        }
        // ARRI LogC3
        "logc" | "logc3" | "logc_to_linear" | "logc3_to_linear" => {
            for v in data.iter_mut() {
                *v = log_c::decode(*v);
            }
        }
        "linear_to_logc" | "linear_to_logc3" => {
            for v in data.iter_mut() {
                *v = log_c::encode(*v);
            }
        }
        // ARRI LogC4 (ALEXA 35)
        "logc4" | "logc4_to_linear" => {
            for v in data.iter_mut() {
                *v = log_c4::decode(*v);
            }
        }
        "linear_to_logc4" => {
            for v in data.iter_mut() {
                *v = log_c4::encode(*v);
            }
        }
        // Sony S-Log3
        "slog3" | "slog3_to_linear" => {
            for v in data.iter_mut() {
                *v = s_log3::decode(*v);
            }
        }
        "linear_to_slog3" => {
            for v in data.iter_mut() {
                *v = s_log3::encode(*v);
            }
        }
        // Panasonic V-Log
        "vlog" | "vlog_to_linear" => {
            for v in data.iter_mut() {
                *v = v_log::decode(*v);
            }
        }
        "linear_to_vlog" => {
            for v in data.iter_mut() {
                *v = v_log::encode(*v);
            }
        }
        _ => {}
    }
}

/// Apply 3x3 color matrix to RGB data
fn apply_matrix(data: &mut [f32], width: usize, height: usize, channels: usize, matrix: &vfx_math::Mat3) {
    if channels < 3 { return; }
    
    for y in 0..height {
        for x in 0..width {
            let idx = (y * width + x) * channels;
            let r = data[idx];
            let g = data[idx + 1];
            let b = data[idx + 2];
            
            // Matrix * RGB vector
            data[idx]     = matrix.row(0).x * r + matrix.row(0).y * g + matrix.row(0).z * b;
            data[idx + 1] = matrix.row(1).x * r + matrix.row(1).y * g + matrix.row(1).z * b;
            data[idx + 2] = matrix.row(2).x * r + matrix.row(2).y * g + matrix.row(2).z * b;
        }
    }
}
