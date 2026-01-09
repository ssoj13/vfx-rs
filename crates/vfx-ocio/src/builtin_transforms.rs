//! Built-in transform definitions for OCIO v2 BuiltinTransform.
//!
//! Contains matrices, log parameters, and transform chains for common
//! color space conversions without needing external LUT files.

#![allow(dead_code)]

use crate::processor::ProcessorOp;

/// Builtin transform definition.
#[derive(Debug, Clone)]
pub enum BuiltinDef {
    /// Identity (no-op).
    Identity,
    /// Matrix-only transform.
    Matrix {
        matrix: [f32; 16],
        offset: [f32; 4],
    },
    /// Log camera transform (to linear).
    LogCamera {
        base: f32,
        log_side_slope: [f32; 3],
        log_side_offset: [f32; 3],
        lin_side_slope: [f32; 3],
        lin_side_offset: [f32; 3],
        lin_side_break: [f32; 3],
        linear_slope: [f32; 3],
    },
    /// Transfer function (log/gamma to linear).
    Transfer {
        style: TransferStyle,
    },
    /// Chain of operations (log + matrix, etc.).
    Chain(Vec<BuiltinDef>),
}

/// Transfer function style for builtin.
#[derive(Debug, Clone, Copy)]
pub enum TransferStyle {
    AcesCct,
    AcesCc,
    LogC3,
    LogC4,
    SLog3,
    VLog,
    Log3G10,
    Srgb,
    Rec709,
}

// ============================================================================
// ACES Matrices (from ACES Technical Bulletins)
// ============================================================================

/// AP0 (ACES 2065-1) to AP1 (ACEScg) matrix.
pub const AP0_TO_AP1: [f32; 16] = [
    1.4514393161, -0.2365107469, -0.2149285693, 0.0,
    -0.0765537734, 1.1762296998, -0.0996759264, 0.0,
    0.0083161484, -0.0060324498, 0.9977163014, 0.0,
    0.0, 0.0, 0.0, 1.0,
];

/// AP1 (ACEScg) to AP0 (ACES 2065-1) matrix.
pub const AP1_TO_AP0: [f32; 16] = [
    0.6954522414, 0.1406786965, 0.1638690622, 0.0,
    0.0447945634, 0.8596711185, 0.0955343182, 0.0,
    -0.0055258826, 0.0040252103, 1.0015006723, 0.0,
    0.0, 0.0, 0.0, 1.0,
];

/// AP0 to CIE XYZ D65 (Bradford-adapted).
pub const AP0_TO_XYZ_D65: [f32; 16] = [
    0.9525523959, 0.0000000000, 0.0000936786, 0.0,
    0.3439664498, 0.7281660966, -0.0721325464, 0.0,
    0.0000000000, 0.0000000000, 1.0088251844, 0.0,
    0.0, 0.0, 0.0, 1.0,
];

/// AP1 to CIE XYZ D65 (Bradford-adapted).
pub const AP1_TO_XYZ_D65: [f32; 16] = [
    0.6624541811, 0.1340042065, 0.1561876870, 0.0,
    0.2722287168, 0.6740817658, 0.0536895174, 0.0,
    -0.0055746495, 0.0040607335, 1.0103391003, 0.0,
    0.0, 0.0, 0.0, 1.0,
];

/// sRGB to CIE XYZ D65.
pub const SRGB_TO_XYZ_D65: [f32; 16] = [
    0.4124564, 0.3575761, 0.1804375, 0.0,
    0.2126729, 0.7151522, 0.0721750, 0.0,
    0.0193339, 0.1191920, 0.9503041, 0.0,
    0.0, 0.0, 0.0, 1.0,
];

/// Rec.709 to CIE XYZ D65 (same primaries as sRGB).
pub const REC709_TO_XYZ_D65: [f32; 16] = SRGB_TO_XYZ_D65;

/// ARRI Wide Gamut 3 to ACES AP0.
pub const AWG3_TO_AP0: [f32; 16] = [
    0.6802059651, 0.2361263643, 0.0836676706, 0.0,
    0.0854721169, 0.8317721797, 0.0827557034, 0.0,
    0.0020562623, -0.0823625259, 1.0803062636, 0.0,
    0.0, 0.0, 0.0, 1.0,
];

/// ARRI Wide Gamut 4 to ACES AP0.
pub const AWG4_TO_AP0: [f32; 16] = [
    0.7504056128, 0.1440458891, 0.1055484981, 0.0,
    0.0003237726, 1.0899282987, -0.0902520713, 0.0,
    -0.0003883, -0.1507043498, 1.1510926498, 0.0,
    0.0, 0.0, 0.0, 1.0,
];

/// Sony S-Gamut3 to ACES AP0.
pub const SGAMUT3_TO_AP0: [f32; 16] = [
    0.7529825954, 0.1433702162, 0.1036471884, 0.0,
    0.0217076974, 1.0153188355, -0.0370265329, 0.0,
    -0.0094160528, 0.0033243699, 1.0060916829, 0.0,
    0.0, 0.0, 0.0, 1.0,
];

/// RED Wide Gamut RGB to ACES AP0.
pub const RWG_TO_AP0: [f32; 16] = [
    0.7853100352, 0.0838235592, 0.1308664056, 0.0,
    0.0231691634, 1.0878966853, -0.1110658487, 0.0,
    -0.0737860122, -0.3145509113, 1.3883369235, 0.0,
    0.0, 0.0, 0.0, 1.0,
];

// ============================================================================
// Log Camera Parameters
// ============================================================================

/// ARRI LogC3 (EI800) parameters.
pub fn logc3_params() -> (f32, [f32; 3], [f32; 3], [f32; 3], [f32; 3], [f32; 3], [f32; 3]) {
    let cut = 0.010591;
    let a = 5.555556;
    let b = 0.052272;
    let c = 0.247190;
    let d = 0.385537;
    let e = 5.367655;
    let _f = 0.092809;
    
    (
        10.0,
        [c, c, c],  // log_side_slope
        [d, d, d],  // log_side_offset  
        [a, a, a],  // lin_side_slope
        [b, b, b],  // lin_side_offset
        [cut, cut, cut],  // lin_side_break
        [e, e, e],  // linear_slope
    )
}

/// ARRI LogC4 parameters.
pub fn logc4_params() -> (f32, [f32; 3], [f32; 3], [f32; 3], [f32; 3], [f32; 3], [f32; 3]) {
    let a = (2f32.powf(18.0) - 16.0) / 117.45;
    let b = (16.0 - 64.0) / 117.45;
    let c = 14.0 / (2f32.ln() * 6.0);
    let s = (7.0 * 2f32.ln() * 2f32.powf(7.0 - 14.0 * c * 0.0 / 6.0)) / (a * 6.0);
    let t = (2f32.powf(7.0 - 14.0 * 0.0 / 6.0) - 64.0) / a - b;
    
    (
        2.0,
        [c, c, c],
        [0.0, 0.0, 0.0],
        [a, a, a],
        [b, b, b],
        [t, t, t],
        [s, s, s],
    )
}

/// Sony S-Log3 parameters.
pub fn slog3_params() -> (f32, [f32; 3], [f32; 3], [f32; 3], [f32; 3], [f32; 3], [f32; 3]) {
    let cut = 0.01125;
    (
        10.0,
        [0.255620723362659, 0.255620723362659, 0.255620723362659],
        [0.410557184750733, 0.410557184750733, 0.410557184750733],
        [5.26315789473684, 5.26315789473684, 5.26315789473684],
        [0.0526315789473684, 0.0526315789473684, 0.0526315789473684],
        [cut, cut, cut],
        [6.62194371177582, 6.62194371177582, 6.62194371177582],
    )
}

/// Panasonic V-Log parameters.
pub fn vlog_params() -> (f32, [f32; 3], [f32; 3], [f32; 3], [f32; 3], [f32; 3], [f32; 3]) {
    let cut = 0.01;
    let b = 0.00873;
    let c = 0.241514;
    let d = 0.598206;
    
    (
        10.0,
        [c, c, c],
        [d, d, d],
        [1.0, 1.0, 1.0],
        [b, b, b],
        [cut, cut, cut],
        [5.6, 5.6, 5.6],
    )
}

/// RED Log3G10 parameters.
pub fn log3g10_params() -> (f32, [f32; 3], [f32; 3], [f32; 3], [f32; 3], [f32; 3], [f32; 3]) {
    let a = 1.0 / 155.975327;
    let b = 0.01 - a;
    let c = 15.1927;
    
    (
        10.0,
        [1.0/c, 1.0/c, 1.0/c],
        [0.0, 0.0, 0.0],
        [a, a, a],
        [b, b, b],
        [-0.01, -0.01, -0.01],
        [c * (a.ln() * 10.0_f32.ln()), c * (a.ln() * 10.0_f32.ln()), c * (a.ln() * 10.0_f32.ln())],
    )
}

// ============================================================================
// Lookup
// ============================================================================

/// Get builtin transform definition by style name.
pub fn get_builtin(style: &str) -> Option<BuiltinDef> {
    let style_lower = style.to_lowercase().replace(['-', '_', ' '], "");
    
    match style_lower.as_str() {
        // Identity
        "identity" => Some(BuiltinDef::Identity),
        
        // ACES core transforms
        "acesap0toap1" | "aces2065toacescg" => Some(BuiltinDef::Matrix {
            matrix: AP0_TO_AP1,
            offset: [0.0; 4],
        }),
        "acesap1toap0" | "acescgtoaces2065" | "acescgtoaces20651" => Some(BuiltinDef::Matrix {
            matrix: AP1_TO_AP0,
            offset: [0.0; 4],
        }),
        "acesap0toxyzd65bfd" | "aces20651toxyzd65" => Some(BuiltinDef::Matrix {
            matrix: AP0_TO_XYZ_D65,
            offset: [0.0; 4],
        }),
        "acesap1toxyzd65bfd" | "acescgtoxyzd65" => Some(BuiltinDef::Matrix {
            matrix: AP1_TO_XYZ_D65,
            offset: [0.0; 4],
        }),
        
        // ACEScct/ACEScc to ACES2065-1
        "acesccttoaces20651" | "acesccttoaces2065" => {
            let _ = logc3_params(); // ACEScct uses Transfer style directly
            Some(BuiltinDef::Chain(vec![
                BuiltinDef::Transfer { style: TransferStyle::AcesCct },
                BuiltinDef::Matrix { matrix: AP1_TO_AP0, offset: [0.0; 4] },
            ]))
        }
        "acescctoaces20651" | "acescctoaces2065" => {
            Some(BuiltinDef::Chain(vec![
                BuiltinDef::Transfer { style: TransferStyle::AcesCc },
                BuiltinDef::Matrix { matrix: AP1_TO_AP0, offset: [0.0; 4] },
            ]))
        }
        
        // ARRI LogC to ACES
        "arrilogc3toaces20651" | "logc3toaces" | "arrilogctoaces" => {
            let (base, lss, lso, lns, lno, lnb, ls) = logc3_params();
            Some(BuiltinDef::Chain(vec![
                BuiltinDef::LogCamera { base, log_side_slope: lss, log_side_offset: lso, lin_side_slope: lns, lin_side_offset: lno, lin_side_break: lnb, linear_slope: ls },
                BuiltinDef::Matrix { matrix: AWG3_TO_AP0, offset: [0.0; 4] },
            ]))
        }
        "arrilogc4toaces20651" | "logc4toaces" => {
            let (base, lss, lso, lns, lno, lnb, ls) = logc4_params();
            Some(BuiltinDef::Chain(vec![
                BuiltinDef::LogCamera { base, log_side_slope: lss, log_side_offset: lso, lin_side_slope: lns, lin_side_offset: lno, lin_side_break: lnb, linear_slope: ls },
                BuiltinDef::Matrix { matrix: AWG4_TO_AP0, offset: [0.0; 4] },
            ]))
        }
        
        // Sony S-Log3 to ACES
        "sonyslog3sgamut3toaces20651" | "slog3toaces" | "sonyslog3toaces" => {
            let (base, lss, lso, lns, lno, lnb, ls) = slog3_params();
            Some(BuiltinDef::Chain(vec![
                BuiltinDef::LogCamera { base, log_side_slope: lss, log_side_offset: lso, lin_side_slope: lns, lin_side_offset: lno, lin_side_break: lnb, linear_slope: ls },
                BuiltinDef::Matrix { matrix: SGAMUT3_TO_AP0, offset: [0.0; 4] },
            ]))
        }
        
        // Panasonic V-Log to ACES
        "panasonicvlogvgamuttoaces20651" | "vlogtoaces" => {
            let (base, lss, lso, lns, lno, lnb, ls) = vlog_params();
            Some(BuiltinDef::Chain(vec![
                BuiltinDef::LogCamera { base, log_side_slope: lss, log_side_offset: lso, lin_side_slope: lns, lin_side_offset: lno, lin_side_break: lnb, linear_slope: ls },
                // V-Gamut to AP0 matrix would go here
                BuiltinDef::Identity, // Placeholder - V-Gamut matrix needed
            ]))
        }
        
        // RED Log3G10 to ACES
        "redlog3g10rwgtoaces20651" | "log3g10toaces" | "redlogtoaces" => {
            let (base, lss, lso, lns, lno, lnb, ls) = log3g10_params();
            Some(BuiltinDef::Chain(vec![
                BuiltinDef::LogCamera { base, log_side_slope: lss, log_side_offset: lso, lin_side_slope: lns, lin_side_offset: lno, lin_side_break: lnb, linear_slope: ls },
                BuiltinDef::Matrix { matrix: RWG_TO_AP0, offset: [0.0; 4] },
            ]))
        }
        
        // sRGB / Rec.709
        "srgbtoxyzd65" | "srgbtociexyz" => Some(BuiltinDef::Chain(vec![
            BuiltinDef::Transfer { style: TransferStyle::Srgb },
            BuiltinDef::Matrix { matrix: SRGB_TO_XYZ_D65, offset: [0.0; 4] },
        ])),
        
        _ => None,
    }
}

/// Compile a builtin definition into processor ops.
pub fn compile_builtin(def: &BuiltinDef, forward: bool, ops: &mut Vec<ProcessorOp>) {
    match def {
        BuiltinDef::Identity => {
            // No-op
        }
        
        BuiltinDef::Matrix { matrix, offset } => {
            if forward {
                ops.push(ProcessorOp::Matrix {
                    matrix: *matrix,
                    offset: *offset,
                });
            } else {
                // Compute inverse matrix
                let mat4 = glam::Mat4::from_cols_array(&[
                    matrix[0], matrix[4], matrix[8], matrix[12],
                    matrix[1], matrix[5], matrix[9], matrix[13],
                    matrix[2], matrix[6], matrix[10], matrix[14],
                    matrix[3], matrix[7], matrix[11], matrix[15],
                ]);
                let inv = mat4.inverse();
                let inv_arr = inv.to_cols_array();
                let inv_matrix = [
                    inv_arr[0], inv_arr[4], inv_arr[8], inv_arr[12],
                    inv_arr[1], inv_arr[5], inv_arr[9], inv_arr[13],
                    inv_arr[2], inv_arr[6], inv_arr[10], inv_arr[14],
                    inv_arr[3], inv_arr[7], inv_arr[11], inv_arr[15],
                ];
                let inv_offset = [
                    -(inv_matrix[0] * offset[0] + inv_matrix[1] * offset[1] + inv_matrix[2] * offset[2] + inv_matrix[3] * offset[3]),
                    -(inv_matrix[4] * offset[0] + inv_matrix[5] * offset[1] + inv_matrix[6] * offset[2] + inv_matrix[7] * offset[3]),
                    -(inv_matrix[8] * offset[0] + inv_matrix[9] * offset[1] + inv_matrix[10] * offset[2] + inv_matrix[11] * offset[3]),
                    -(inv_matrix[12] * offset[0] + inv_matrix[13] * offset[1] + inv_matrix[14] * offset[2] + inv_matrix[15] * offset[3]),
                ];
                ops.push(ProcessorOp::Matrix {
                    matrix: inv_matrix,
                    offset: inv_offset,
                });
            }
        }
        
        BuiltinDef::LogCamera { base, log_side_slope, log_side_offset, lin_side_slope, lin_side_offset, lin_side_break, linear_slope } => {
            ops.push(ProcessorOp::LogCamera {
                base: *base,
                log_side_slope: *log_side_slope,
                log_side_offset: *log_side_offset,
                lin_side_slope: *lin_side_slope,
                lin_side_offset: *lin_side_offset,
                lin_side_break: *lin_side_break,
                linear_slope: *linear_slope,
                forward,
            });
        }
        
        BuiltinDef::Transfer { style } => {
            use crate::processor::TransferStyle as ProcTransfer;
            let proc_style = match style {
                TransferStyle::AcesCct => ProcTransfer::AcesCct,
                TransferStyle::AcesCc => ProcTransfer::AcesCc,
                TransferStyle::LogC3 => ProcTransfer::LogC3,
                TransferStyle::LogC4 => ProcTransfer::LogC4,
                TransferStyle::SLog3 => ProcTransfer::SLog3,
                TransferStyle::VLog => ProcTransfer::VLog,
                TransferStyle::Log3G10 => ProcTransfer::Log3G10,
                TransferStyle::Srgb => ProcTransfer::Srgb,
                TransferStyle::Rec709 => ProcTransfer::Rec709,
            };
            ops.push(ProcessorOp::Transfer { style: proc_style, forward });
        }
        
        BuiltinDef::Chain(defs) => {
            if forward {
                for d in defs {
                    compile_builtin(d, true, ops);
                }
            } else {
                // Reverse order for inverse
                for d in defs.iter().rev() {
                    compile_builtin(d, false, ops);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity() {
        let def = get_builtin("IDENTITY").unwrap();
        let mut ops = Vec::new();
        compile_builtin(&def, true, &mut ops);
        assert!(ops.is_empty()); // Identity = no ops
    }

    #[test]
    fn test_ap0_to_ap1() {
        let def = get_builtin("ACES-AP0_to_AP1").unwrap();
        let mut ops = Vec::new();
        compile_builtin(&def, true, &mut ops);
        assert_eq!(ops.len(), 1);
        assert!(matches!(ops[0], ProcessorOp::Matrix { .. }));
    }

    #[test]
    fn test_logc3_to_aces() {
        let def = get_builtin("ARRI_LogC3_to_ACES2065-1").unwrap();
        let mut ops = Vec::new();
        compile_builtin(&def, true, &mut ops);
        // Should have LogCamera + Matrix
        assert_eq!(ops.len(), 2);
        assert!(matches!(ops[0], ProcessorOp::LogCamera { .. }));
        assert!(matches!(ops[1], ProcessorOp::Matrix { .. }));
    }

    #[test]
    fn test_acescct_to_aces() {
        let def = get_builtin("ACEScct_to_ACES2065-1").unwrap();
        let mut ops = Vec::new();
        compile_builtin(&def, true, &mut ops);
        // Should have Transfer + Matrix
        assert_eq!(ops.len(), 2);
        assert!(matches!(ops[0], ProcessorOp::Transfer { .. }));
        assert!(matches!(ops[1], ProcessorOp::Matrix { .. }));
    }

    #[test]
    fn test_unknown_returns_none() {
        assert!(get_builtin("UNKNOWN_TRANSFORM_XYZ").is_none());
    }

    #[test]
    fn test_style_normalization() {
        // All these should match the same transform
        assert!(get_builtin("ACES-AP0_to_AP1").is_some());
        assert!(get_builtin("aces_ap0_to_ap1").is_some());
        assert!(get_builtin("ACESAP0TOAP1").is_some());
        assert!(get_builtin("aces ap0 to ap1").is_some());
    }
}
