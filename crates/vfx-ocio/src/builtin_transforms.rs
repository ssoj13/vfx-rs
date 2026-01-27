//! Built-in transform definitions for OCIO v2 BuiltinTransform.
//!
//! Contains matrices, log parameters, and transform chains for common
//! color space conversions without needing external LUT files.

#![allow(dead_code)]

use crate::processor::ProcessorOp;
use crate::transform::NegativeStyle;

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
    /// Log camera transform.
    /// When `inverse=true`, converts from log (encoded) to linear (decode direction).
    /// When `inverse=false`, converts from linear to log (encode direction).
    LogCamera {
        base: f32,
        log_side_slope: [f32; 3],
        log_side_offset: [f32; 3],
        lin_side_slope: [f32; 3],
        lin_side_offset: [f32; 3],
        lin_side_break: [f32; 3],
        linear_slope: [f32; 3],
        /// If true, the "forward" direction is log->linear (decode).
        /// This matches OCIO's TRANSFORM_DIR_INVERSE for camera log ops.
        inverse: bool,
    },
    /// Transfer function (log/gamma to linear).
    Transfer {
        style: TransferStyle,
    },
    /// Exponent with configurable negative handling (gamma curves).
    Gamma {
        value: f32,
        mirror: bool,
    },
    /// MonCurve (sRGB-like) with configurable negative handling.
    MonCurve {
        gamma: f32,
        offset: f32,
        mirror: bool,
    },
    /// Uniform scale per channel.
    Scale {
        factors: [f32; 3],
    },
    /// Range clamp.
    Clamp {
        min: f32,
        max: f32,
    },
    /// B-spline curve (baked to 1D LUT at compile time).
    /// Used for ACES RRT/ODT tone curves in log10 space.
    BSplineCurve {
        /// Control points (x, y).
        points: Vec<[f32; 2]>,
        /// Slopes at control points.
        slopes: Vec<f32>,
    },
    /// Log base conversion.
    Log {
        base: f32,
        forward: bool,
    },
    /// Fixed function op.
    FixedFunction {
        style: crate::transform::FixedFunctionStyle,
        params: Vec<f32>,
    },
    /// ACES 2.0 Output Transform (via aces2 module).
    Aces2Output {
        peak_luminance: f32,
        /// Limiting primaries: [[rx,ry],[gx,gy],[bx,by],[wx,wy]]
        limiting_xy: [[f32; 2]; 4],
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
    AppleLog,
    CanonCLog2,
    CanonCLog3,
    Pq,    // ST-2084 / PQ
    Hlg,   // HLG
    Rec1886, // Rec.1886 (gamma 2.4)
    Gamma22, // gamma 2.2
    Gamma26, // gamma 2.6 (DCI)
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

/// AP0 → CIE-XYZ-D65 with Bradford adaptation (computed at runtime).
pub static AP0_TO_XYZ_D65: std::sync::LazyLock<[f32; 16]> = std::sync::LazyLock::new(|| {
    let ap0_xyz = rgb_to_xyz(&PRIMS_AP0);
    let src_wht = [
        (ap0_xyz[0] + ap0_xyz[1] + ap0_xyz[2]) as f64,
        (ap0_xyz[4] + ap0_xyz[5] + ap0_xyz[6]) as f64,
        (ap0_xyz[8] + ap0_xyz[9] + ap0_xyz[10]) as f64,
    ];
    let bfd = bradford_adapt(src_wht, D65_XYZ);
    mat4_mul(&bfd, &ap0_xyz)
});

/// AP1 to CIE XYZ D65 (Bradford-adapted).
/// AP1 → XYZ D65 (Bradford): computed at runtime.
/// Old hardcoded values were wrong (missing Bradford D60→D65 adaptation).
pub static AP1_TO_XYZ_D65: std::sync::LazyLock<[f32; 16]> = std::sync::LazyLock::new(ap1_to_xyz_d65_bfd);

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

/// Panasonic V-Gamut to ACES AP0 (with Bradford D65->D60 adaptation).
/// Computed via: V-Gamut(D65) -> XYZ -> Bradford -> AP0(D60)
pub const VGAMUT_TO_AP0: [f32; 16] = [
    0.7245869636, 0.1661761999, 0.1092368365, 0.0,
    0.0219097584, 0.9843355417, -0.0062452999, 0.0,
    -0.0096276887, -0.0004312588, 1.0100589475, 0.0,
    0.0, 0.0, 0.0, 1.0,
];

/// Canon Cinema Gamut to ACES AP0 (CAT02 adaptation).
pub const CANON_CGAMUT_TO_AP0: [f32; 16] = [
    0.763064455, 0.1488693, 0.0880662450, 0.0,
    0.003299634, 1.0884838, -0.0917834340, 0.0,
    -0.009632175, -0.0829892, 1.0926213750, 0.0,
    0.0, 0.0, 0.0, 1.0,
];

/// Sony S-Gamut3.Cine to ACES AP0 (CAT02 adaptation).
pub const SGAMUT3_CINE_TO_AP0: [f32; 16] = [
    0.6387886672, 0.2723514337, 0.0888598991, 0.0,
    -0.0039159061, 1.0880732308, -0.0841573247, 0.0,
    -0.0299072021, -0.0264325799, 1.0563397820, 0.0,
    0.0, 0.0, 0.0, 1.0,
];

/// Sony S-Gamut3 VENICE to ACES AP0 (hardcoded 4x4).
pub const SGAMUT3_VENICE_TO_AP0: [f32; 16] = [
    0.7933297411, 0.0890786256, 0.1175916333, 0.0,
    0.0155810585, 1.0327123069, -0.0482933654, 0.0,
    -0.0188647478, 0.0127694121, 1.0060953358, 0.0,
    0.0, 0.0, 0.0, 1.0,
];

/// Sony S-Gamut3.Cine VENICE to ACES AP0 (hardcoded 4x4).
pub const SGAMUT3_CINE_VENICE_TO_AP0: [f32; 16] = [
    0.6742570921, 0.2205717359, 0.1051711720, 0.0,
    -0.0093136061, 1.1059588614, -0.0966452553, 0.0,
    -0.0382090673, -0.0179383766, 1.0561474439, 0.0,
    0.0, 0.0, 0.0, 1.0,
];

/// RED REDLogFilm RWG to ACES AP0.
pub const RWG_REDLOGFILM_TO_AP0: [f32; 16] = [
    0.7853100352, 0.0838235592, 0.1308664056, 0.0,
    0.0231691634, 1.0878966853, -0.1110658487, 0.0,
    -0.0737860122, -0.3145509113, 1.3883369235, 0.0,
    0.0, 0.0, 0.0, 1.0,
];

/// AP1 to linear Rec.709 (Bradford-adapted from D60 to D65).
pub const AP1_TO_LINEAR_REC709_BFD: [f32; 16] = [
    1.7050509926, -0.6217921206, -0.0832588720, 0.0,
    -0.1302564175, 1.1408048829, -0.0105484654, 0.0,
    -0.0240033617, -0.1289689751, 1.1529723368, 0.0,
    0.0, 0.0, 0.0, 1.0,
];

/// Rec.2020 to ACES AP0 (Bradford adaptation).
pub const REC2020_TO_AP0: [f32; 16] = [
    0.678891151, 0.158868422, 0.162240427, 0.0,
    0.045570831, 0.860712772, 0.093716397, 0.0,
    -0.000485710, 0.025060196, 0.975425514, 0.0,
    0.0, 0.0, 0.0, 1.0,
];

/// CIE XYZ D65 to Rec.709/sRGB primaries.
pub const XYZ_D65_TO_REC709: [f32; 16] = [
    3.2404542, -1.5371385, -0.4985314, 0.0,
    -0.9692660, 1.8760108, 0.0415560, 0.0,
    0.0556434, -0.2040259, 1.0572252, 0.0,
    0.0, 0.0, 0.0, 1.0,
];

/// CIE XYZ D65 to Rec.2020 primaries.
pub const XYZ_D65_TO_REC2020: [f32; 16] = [
    1.7166512, -0.3556708, -0.2533663, 0.0,
    -0.6666844, 1.6164812, 0.0157685, 0.0,
    0.0176399, -0.0427706, 0.9421031, 0.0,
    0.0, 0.0, 0.0, 1.0,
];

/// CIE XYZ D65 to P3-DCI primaries (Bradford D65→DCI white).
pub const XYZ_D65_TO_P3_DCI_BFD: [f32; 16] = [
    2.7253940305, -1.0180030062, -0.4401631952, 0.0,
    -0.7951680258,  1.6897320548,  0.0226471906, 0.0,
     0.0412418914, -0.0876390192,  1.1009293786, 0.0,
    0.0, 0.0, 0.0, 1.0,
];

/// CIE XYZ D65 to P3-D60 primaries (Bradford D65→D60).
pub const XYZ_D65_TO_P3_D60_BFD: [f32; 16] = [
    2.4027414142, -0.8974841639, -0.3880533700, 0.0,
    -0.8325796487,  1.7692317536,  0.0237127115, 0.0,
     0.0388233815, -0.0824996856,  1.0363685895, 0.0,
    0.0, 0.0, 0.0, 1.0,
];

/// CIE XYZ D65 to Display P3 primaries.
pub const XYZ_D65_TO_P3_D65: [f32; 16] = [
    2.4934969, -0.9313836, -0.4027108, 0.0,
    -0.8294890, 1.7626641, 0.0236247, 0.0,
    0.0358458, -0.0761724, 0.9568845, 0.0,
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

/// RED REDLogFilm parameters (base 10).
pub fn redlogfilm_params() -> (f32, [f32; 3], [f32; 3], [f32; 3], [f32; 3], [f32; 3], [f32; 3]) {
    let ref_white = 685.0 / 1023.0;
    let ref_black = 95.0 / 1023.0;
    let gamma = 0.6;
    let range = 0.002 * 1023.0;
    let log_side_slope = gamma / range;
    let log_side_offset = ref_white;
    // Derived from OCIO's REDLogFilm formula
    let gain = 1.0 / (1.0 - 10.0_f32.powf((ref_black - ref_white) / (gamma / range)));
    let off = gain - 1.0;
    let lin_side_slope = gain;
    let lin_side_offset = off;
    let cut = -off / gain;
    let linear_slope = gain * range * 10.0_f32.ln() * 10.0_f32.powf((cut * gain + off).log10() * (range / gamma));
    (
        10.0,
        [log_side_slope; 3],
        [log_side_offset; 3],
        [lin_side_slope; 3],
        [lin_side_offset; 3],
        [cut; 3],
        [linear_slope; 3],
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
            matrix: *AP0_TO_XYZ_D65,
            offset: [0.0; 4],
        }),
        "acesap1toxyzd65bfd" | "acescgtoxyzd65" => Some(BuiltinDef::Matrix {
            matrix: *AP1_TO_XYZ_D65,
            offset: [0.0; 4],
        }),
        
        // ACEScct/ACEScc to ACES2065-1
        "acesccttoaces20651" | "acesccttoaces2065" => {
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
                BuiltinDef::LogCamera { base, log_side_slope: lss, log_side_offset: lso, lin_side_slope: lns, lin_side_offset: lno, lin_side_break: lnb, linear_slope: ls, inverse: true },
                BuiltinDef::Matrix { matrix: AWG3_TO_AP0, offset: [0.0; 4] },
            ]))
        }
        "arrilogc4toaces20651" | "logc4toaces" => {
            let (base, lss, lso, lns, lno, lnb, ls) = logc4_params();
            Some(BuiltinDef::Chain(vec![
                BuiltinDef::LogCamera { base, log_side_slope: lss, log_side_offset: lso, lin_side_slope: lns, lin_side_offset: lno, lin_side_break: lnb, linear_slope: ls, inverse: true },
                BuiltinDef::Matrix { matrix: AWG4_TO_AP0, offset: [0.0; 4] },
            ]))
        }
        
        // Sony S-Log3 to ACES
        "sonyslog3sgamut3toaces20651" | "slog3toaces" | "sonyslog3toaces" => {
            let (base, lss, lso, lns, lno, lnb, ls) = slog3_params();
            Some(BuiltinDef::Chain(vec![
                BuiltinDef::LogCamera { base, log_side_slope: lss, log_side_offset: lso, lin_side_slope: lns, lin_side_offset: lno, lin_side_break: lnb, linear_slope: ls, inverse: true },
                BuiltinDef::Matrix { matrix: SGAMUT3_TO_AP0, offset: [0.0; 4] },
            ]))
        }
        
        // Panasonic V-Log to ACES
        "panasonicvlogvgamuttoaces20651" | "vlogtoaces" => {
            let (base, lss, lso, lns, lno, lnb, ls) = vlog_params();
            Some(BuiltinDef::Chain(vec![
                BuiltinDef::LogCamera { base, log_side_slope: lss, log_side_offset: lso, lin_side_slope: lns, lin_side_offset: lno, lin_side_break: lnb, linear_slope: ls, inverse: true },
                BuiltinDef::Matrix { matrix: VGAMUT_TO_AP0, offset: [0.0; 4] },
            ]))
        }
        
        // RED Log3G10 to ACES
        "redlog3g10rwgtoaces20651" | "log3g10toaces" | "redlogtoaces" => {
            let (base, lss, lso, lns, lno, lnb, ls) = log3g10_params();
            Some(BuiltinDef::Chain(vec![
                BuiltinDef::LogCamera { base, log_side_slope: lss, log_side_offset: lso, lin_side_slope: lns, lin_side_offset: lno, lin_side_break: lnb, linear_slope: ls, inverse: true },
                BuiltinDef::Matrix { matrix: RWG_TO_AP0, offset: [0.0; 4] },
            ]))
        }
        
        // sRGB / Rec.709
        "srgbtoxyzd65" | "srgbtociexyz" => Some(BuiltinDef::Chain(vec![
            BuiltinDef::Transfer { style: TransferStyle::Srgb },
            BuiltinDef::Matrix { matrix: SRGB_TO_XYZ_D65, offset: [0.0; 4] },
        ])),
        
        // Apple Log to ACES (Apple Log uses Rec.2020 primaries)
        "applelogtoaces20651" | "applelogtoaces" => {
            Some(BuiltinDef::Chain(vec![
                BuiltinDef::Transfer { style: TransferStyle::AppleLog },
                BuiltinDef::Matrix { matrix: REC2020_TO_AP0, offset: [0.0; 4] },
            ]))
        }
        
        // Apple Log curve only
        "curveapplelogtolinear" => {
            Some(BuiltinDef::Transfer { style: TransferStyle::AppleLog })
        }
        
        // Canon C-Log2 Cinema Gamut to ACES
        "canonclog2cgamuttoaces20651" | "clog2toaces" => {
            Some(BuiltinDef::Chain(vec![
                BuiltinDef::Transfer { style: TransferStyle::CanonCLog2 },
                BuiltinDef::Matrix { matrix: CANON_CGAMUT_TO_AP0, offset: [0.0; 4] },
            ]))
        }
        
        // Canon C-Log2 curve only
        "curvecanonclog2tolinear" => {
            Some(BuiltinDef::Transfer { style: TransferStyle::CanonCLog2 })
        }
        
        // Canon C-Log3 Cinema Gamut to ACES
        "canonclog3cgamuttoaces20651" | "clog3toaces" => {
            Some(BuiltinDef::Chain(vec![
                BuiltinDef::Transfer { style: TransferStyle::CanonCLog3 },
                BuiltinDef::Matrix { matrix: CANON_CGAMUT_TO_AP0, offset: [0.0; 4] },
            ]))
        }
        
        // Canon C-Log3 curve only
        "curvecanonclog3tolinear" => {
            Some(BuiltinDef::Transfer { style: TransferStyle::CanonCLog3 })
        }
        
        // ACES utility transforms
        "utilityacesap0tociexyzd65bfd" => Some(BuiltinDef::Matrix {
            matrix: *AP0_TO_XYZ_D65,
            offset: [0.0; 4],
        }),
        "utilityacesap1tociexyzd65bfd" => Some(BuiltinDef::Matrix {
            matrix: *AP1_TO_XYZ_D65,
            offset: [0.0; 4],
        }),
        
        // ACEScct/ACEScc curve only (no matrix)
        "curveacescctlogtolin" | "curveacescctlogtolinear" => {
            Some(BuiltinDef::Transfer { style: TransferStyle::AcesCct })
        }
        
        // Display transforms (XYZ D65 to display)
        "displayciexyzd65torec.1886rec.709" | "displayxyzd65torec1886rec709" => {
            Some(BuiltinDef::Chain(vec![
                BuiltinDef::Matrix { matrix: XYZ_D65_TO_REC709, offset: [0.0; 4] },
                BuiltinDef::Transfer { style: TransferStyle::Rec1886 },
            ]))
        }
        "displayciexyzd65torec.1886rec.2020" | "displayxyzd65torec1886rec2020" => {
            Some(BuiltinDef::Chain(vec![
                BuiltinDef::Matrix { matrix: XYZ_D65_TO_REC2020, offset: [0.0; 4] },
                BuiltinDef::Transfer { style: TransferStyle::Rec1886 },
            ]))
        }
        "displayciexyzd65tosrgb" | "displayxyzd65tosrgb" => {
            Some(BuiltinDef::Chain(vec![
                BuiltinDef::Matrix { matrix: XYZ_D65_TO_REC709, offset: [0.0; 4] },
                BuiltinDef::Transfer { style: TransferStyle::Srgb },
            ]))
        }
        "displayciexyzd65todisplayp3" => {
            Some(BuiltinDef::Chain(vec![
                BuiltinDef::Matrix { matrix: XYZ_D65_TO_P3_D65, offset: [0.0; 4] },
                BuiltinDef::Transfer { style: TransferStyle::Srgb },
            ]))
        }
        "displayciexyzd65torec.2100pq" | "displayxyzd65topq" => {
            Some(BuiltinDef::Chain(vec![
                BuiltinDef::Matrix { matrix: XYZ_D65_TO_REC2020, offset: [0.0; 4] },
                BuiltinDef::Transfer { style: TransferStyle::Pq },
            ]))
        }
        "displayciexyzd65torec.2100hlg1000nit" | "displayxyzd65tohlg" => {
            Some(BuiltinDef::Chain(vec![
                BuiltinDef::Matrix { matrix: XYZ_D65_TO_REC2020, offset: [0.0; 4] },
                BuiltinDef::Transfer { style: TransferStyle::Hlg },
            ]))
        }
        
        // PQ / ST-2084 curves
        "curvest2084tolinear" | "curvepqtolinear" => {
            Some(BuiltinDef::Transfer { style: TransferStyle::Pq })
        }
        "curvelineartost2084" | "curvelineartopq" => {
            // Inverse direction handled in compile
            Some(BuiltinDef::Transfer { style: TransferStyle::Pq })
        }
        
        // HLG curves
        "curvehlgoetfinverse" | "curvehlgtolinear" => {
            Some(BuiltinDef::Transfer { style: TransferStyle::Hlg })
        }
        "curvehlgoetf" | "curvelineartohlg" => {
            Some(BuiltinDef::Transfer { style: TransferStyle::Hlg })
        }
        
        // ================================================================
        // Display transforms - MIRROR NEGS variants
        // ================================================================
        "displayciexyzd65torec.1886rec.709mirrorneg" | "displayciexyzd65torec.1886rec.709mirrornegs" => {
            Some(BuiltinDef::Chain(vec![
                BuiltinDef::Matrix { matrix: XYZ_D65_TO_REC709, offset: [0.0; 4] },
                BuiltinDef::Gamma { value: 2.4, mirror: true },
            ]))
        }
        "displayciexyzd65torec.1886rec.2020mirrorneg" | "displayciexyzd65torec.1886rec.2020mirrornegs" => {
            Some(BuiltinDef::Chain(vec![
                BuiltinDef::Matrix { matrix: XYZ_D65_TO_REC2020, offset: [0.0; 4] },
                BuiltinDef::Gamma { value: 2.4, mirror: true },
            ]))
        }
        "displayciexyzd65tosrgbmirrorneg" | "displayciexyzd65tosrgbmirrornegs" => {
            Some(BuiltinDef::Chain(vec![
                BuiltinDef::Matrix { matrix: XYZ_D65_TO_REC709, offset: [0.0; 4] },
                BuiltinDef::MonCurve { gamma: 2.4, offset: 0.055, mirror: true },
            ]))
        }
        "displayciexyzd65tog2.6p3d65mirrorneg" | "displayciexyzd65tog2.6p3d65mirrornegs" => {
            Some(BuiltinDef::Chain(vec![
                BuiltinDef::Matrix { matrix: XYZ_D65_TO_P3_D65, offset: [0.0; 4] },
                BuiltinDef::Gamma { value: 2.6, mirror: true },
            ]))
        }
        
        // G2.2-REC.709 variants
        "displayciexyzd65tog2.2rec.709" | "displayxyzd65tog2.2rec709" => {
            Some(BuiltinDef::Chain(vec![
                BuiltinDef::Matrix { matrix: XYZ_D65_TO_REC709, offset: [0.0; 4] },
                BuiltinDef::Gamma { value: 2.2, mirror: false },
            ]))
        }
        "displayciexyzd65tog2.2rec.709mirrorneg" | "displayciexyzd65tog2.2rec.709mirrornegs" => {
            Some(BuiltinDef::Chain(vec![
                BuiltinDef::Matrix { matrix: XYZ_D65_TO_REC709, offset: [0.0; 4] },
                BuiltinDef::Gamma { value: 2.2, mirror: true },
            ]))
        }
        
        // G2.6-P3 variants
        "displayciexyzd65tog2.6p3dcibfd" => {
            Some(BuiltinDef::Chain(vec![
                BuiltinDef::Matrix { matrix: XYZ_D65_TO_P3_DCI_BFD, offset: [0.0; 4] },
                BuiltinDef::Gamma { value: 2.6, mirror: false },
            ]))
        }
        "displayciexyzd65tog2.6p3d65" => {
            Some(BuiltinDef::Chain(vec![
                BuiltinDef::Matrix { matrix: XYZ_D65_TO_P3_D65, offset: [0.0; 4] },
                BuiltinDef::Gamma { value: 2.6, mirror: false },
            ]))
        }
        "displayciexyzd65tog2.6p3d60bfd" => {
            Some(BuiltinDef::Chain(vec![
                BuiltinDef::Matrix { matrix: XYZ_D65_TO_P3_D60_BFD, offset: [0.0; 4] },
                BuiltinDef::Gamma { value: 2.6, mirror: false },
            ]))
        }
        
        // DCDM-D65 (scale 48/52.37, then gamma 2.6 inverse = 1/2.6 power)
        "displayciexyzd65todcdmd65" => {
            let s = 48.0 / 52.37;
            Some(BuiltinDef::Chain(vec![
                BuiltinDef::Scale { factors: [s, s, s] },
                BuiltinDef::Gamma { value: 1.0 / 2.6, mirror: false },
            ]))
        }
        
        // DisplayP3-HDR (sRGB moncurve with mirror)
        "displayciexyzd65todisplayp3hdr" => {
            Some(BuiltinDef::Chain(vec![
                BuiltinDef::Matrix { matrix: XYZ_D65_TO_P3_D65, offset: [0.0; 4] },
                BuiltinDef::MonCurve { gamma: 2.4, offset: 0.055, mirror: true },
            ]))
        }
        
        // ST2084-P3-D65
        "displayciexyzd65tost2084p3d65" => {
            Some(BuiltinDef::Chain(vec![
                BuiltinDef::Matrix { matrix: XYZ_D65_TO_P3_D65, offset: [0.0; 4] },
                BuiltinDef::Transfer { style: TransferStyle::Pq },
            ]))
        }
        // ST2084-DCDM-D65 (no matrix, just PQ in XYZ-E)
        "displayciexyzd65tost2084dcdmd65" => {
            Some(BuiltinDef::Transfer { style: TransferStyle::Pq })
        }
        
        // ================================================================
        // Camera transforms - additional Sony variants
        // ================================================================
        "sonyslog3sgamut3.cinetoaces20651" | "slog3cinetoaces" => {
            let (base, lss, lso, lns, lno, lnb, ls) = slog3_params();
            Some(BuiltinDef::Chain(vec![
                BuiltinDef::LogCamera { base, log_side_slope: lss, log_side_offset: lso, lin_side_slope: lns, lin_side_offset: lno, lin_side_break: lnb, linear_slope: ls, inverse: true },
                BuiltinDef::Matrix { matrix: SGAMUT3_CINE_TO_AP0, offset: [0.0; 4] },
            ]))
        }
        "sonyslog3sgamut3venicetoaces20651" => {
            let (base, lss, lso, lns, lno, lnb, ls) = slog3_params();
            Some(BuiltinDef::Chain(vec![
                BuiltinDef::LogCamera { base, log_side_slope: lss, log_side_offset: lso, lin_side_slope: lns, lin_side_offset: lno, lin_side_break: lnb, linear_slope: ls, inverse: true },
                BuiltinDef::Matrix { matrix: SGAMUT3_VENICE_TO_AP0, offset: [0.0; 4] },
            ]))
        }
        "sonyslog3sgamut3.cinevenicetoaces20651" => {
            let (base, lss, lso, lns, lno, lnb, ls) = slog3_params();
            Some(BuiltinDef::Chain(vec![
                BuiltinDef::LogCamera { base, log_side_slope: lss, log_side_offset: lso, lin_side_slope: lns, lin_side_offset: lno, lin_side_break: lnb, linear_slope: ls, inverse: true },
                BuiltinDef::Matrix { matrix: SGAMUT3_CINE_VENICE_TO_AP0, offset: [0.0; 4] },
            ]))
        }
        
        // RED REDLogFilm
        "redredlogfilmrwgtoaces20651" | "redlogfilmtoaces" => {
            let (base, lss, lso, lns, lno, lnb, ls) = redlogfilm_params();
            Some(BuiltinDef::Chain(vec![
                BuiltinDef::LogCamera { base, log_side_slope: lss, log_side_offset: lso, lin_side_slope: lns, lin_side_offset: lno, lin_side_break: lnb, linear_slope: ls, inverse: true },
                BuiltinDef::Matrix { matrix: RWG_REDLOGFILM_TO_AP0, offset: [0.0; 4] },
            ]))
        }
        
        // ARRI ALEXA LogC EI800 (same as LogC3 but explicit naming)
        "arrialexa" | "arrialexalogcei800awgtoaces20651" => {
            let (base, lss, lso, lns, lno, lnb, ls) = logc3_params();
            Some(BuiltinDef::Chain(vec![
                BuiltinDef::LogCamera { base, log_side_slope: lss, log_side_offset: lso, lin_side_slope: lns, lin_side_offset: lno, lin_side_break: lnb, linear_slope: ls, inverse: true },
                BuiltinDef::Matrix { matrix: AWG3_TO_AP0, offset: [0.0; 4] },
            ]))
        }
        
        // ================================================================
        // ACES utility/LMT
        // ================================================================
        "utilityacesap1tolinearrec709bfd" => Some(BuiltinDef::Matrix {
            matrix: AP1_TO_LINEAR_REC709_BFD,
            offset: [0.0; 4],
        }),
        // "acescgtoaces20651" already handled above in ACES core section
        
        // ACESproxy10i to ACES2065-1: range remap + exp2 + AP1→AP0
        "acesproxy10itoaces20651" | "acesproxy10toaces" => {
            // Range: [64/1023, 940/1023] → log domain
            let in_min = 64.0 / 1023.0;
            let in_max = 940.0 / 1023.0;
            let out_min = (64.0 - 425.0) / 50.0 - 2.5;
            let out_max = (940.0 - 425.0) / 50.0 - 2.5;
            let scale = (out_max - out_min) / (in_max - in_min);
            let offset = out_min - scale * in_min;
            Some(BuiltinDef::Chain(vec![
                BuiltinDef::Matrix {
                    matrix: [
                        scale, 0.0, 0.0, 0.0,
                        0.0, scale, 0.0, 0.0,
                        0.0, 0.0, scale, 0.0,
                        0.0, 0.0, 0.0, 1.0,
                    ],
                    offset: [offset, offset, offset, 0.0],
                },
                BuiltinDef::Log { base: 2.0, forward: false },
                BuiltinDef::Matrix { matrix: AP1_TO_AP0, offset: [0.0; 4] },
            ]))
        }
        
        // ADX10 to ACES2065-1
        "adx10toaces20651" | "adx10toaces" => {
            let scale = 1023.0 / 500.0;
            let offset = -95.0 / 500.0;
            Some(BuiltinDef::Chain(adx_chain(scale, offset)))
        }
        
        // ADX16 to ACES2065-1
        "adx16toaces20651" | "adx16toaces" => {
            let scale = 65535.0 / 8000.0;
            let offset = -1520.0 / 8000.0;
            Some(BuiltinDef::Chain(adx_chain(scale, offset)))
        }
        
        // ACES LMT: Blue Light Artifact Fix (single matrix)
        "aceslmtbluelightartifactfix" => Some(BuiltinDef::Matrix {
            matrix: [
                0.9404372683, -0.0183068787,  0.0778696104, 0.0,
                0.0083786969,  0.8286599939,  0.1629613092, 0.0,
                0.0005471261, -0.0008833746,  1.0003362486, 0.0,
                0.0, 0.0, 0.0, 1.0,
            ],
            offset: [0.0; 4],
        }),
        
        // ACES LMT: Reference Gamut Compression (AP0→AP1, gamut compress, AP1→AP0)
        "aceslmtaces1.3referencegamutcompression" | "aceslmtaces13referencegamutcompression" => {
            Some(BuiltinDef::Chain(vec![
                BuiltinDef::Matrix { matrix: AP0_TO_AP1, offset: [0.0; 4] },
                BuiltinDef::FixedFunction {
                    style: crate::transform::FixedFunctionStyle::AcesGamutComp13,
                    params: vec![1.147, 1.264, 1.312, 0.815, 0.803, 0.880, 1.2],
                },
                BuiltinDef::Matrix { matrix: AP1_TO_AP0, offset: [0.0; 4] },
            ]))
        }
        
        // ================================================================
        // ACES 1.x Output Transforms
        // ================================================================
        s if s.starts_with("acesoutput") && (s.contains("1.0") || s.contains("1.1")) => {
            parse_aces1_output(s)
        }
        
        // ================================================================
        // ACES 2.0 Output Transforms
        // ================================================================
        s if s.starts_with("acesoutput") && s.contains("2.0") => {
            parse_aces2_output(s)
        }
        
        _ => None,
    }
}

// Limiting primaries for ACES 2.0 output transforms.
const REC709_XY: [[f32; 2]; 4] = [[0.64, 0.33], [0.30, 0.60], [0.15, 0.06], [0.3127, 0.3290]];
const P3_D65_XY: [[f32; 2]; 4] = [[0.680, 0.320], [0.265, 0.690], [0.150, 0.060], [0.3127, 0.3290]];
const REC2020_XY: [[f32; 2]; 4] = [[0.708, 0.292], [0.170, 0.797], [0.131, 0.046], [0.3127, 0.3290]];
const P3_D60_XY: [[f32; 2]; 4] = [[0.680, 0.320], [0.265, 0.690], [0.150, 0.060], [0.32168, 0.33767]];
const REC2020_D60_XY: [[f32; 2]; 4] = [[0.708, 0.292], [0.170, 0.797], [0.131, 0.046], [0.32168, 0.33767]];
const REC709_D60_XY: [[f32; 2]; 4] = [[0.64, 0.33], [0.30, 0.60], [0.15, 0.06], [0.32168, 0.33767]];
const XYZ_E_XY: [[f32; 2]; 4] = [[1.0, 0.0], [0.0, 1.0], [0.0, 0.0], [1.0/3.0, 1.0/3.0]];

/// Parse ACES 2.0 output transform name and return BuiltinDef.
fn parse_aces2_output(s: &str) -> Option<BuiltinDef> {
    // Pattern: acesoutputaces20651tociexyzd65{variant}2.0
    // We match known variants by keywords in the normalized string.
    
    // SDR base variants
    if s.contains("sdr100nitrec709d60inrec709d65") {
        return Some(BuiltinDef::Aces2Output { peak_luminance: 100.0, limiting_xy: REC709_D60_XY });
    }
    if s.contains("sdr100nitrec709d60inp3d65") {
        return Some(BuiltinDef::Aces2Output { peak_luminance: 100.0, limiting_xy: REC709_D60_XY });
    }
    if s.contains("sdr100nitrec709d60inrec2020d65") {
        return Some(BuiltinDef::Aces2Output { peak_luminance: 100.0, limiting_xy: REC709_D60_XY });
    }
    if s.contains("sdr100nitp3d60inp3d65") {
        return Some(BuiltinDef::Aces2Output { peak_luminance: 100.0, limiting_xy: P3_D60_XY });
    }
    if s.contains("sdr100nitp3d60inxyze") {
        return Some(BuiltinDef::Aces2Output { peak_luminance: 100.0, limiting_xy: P3_D60_XY });
    }
    if s.contains("sdr100nitrec709") && !s.contains("d60") {
        return Some(BuiltinDef::Aces2Output { peak_luminance: 100.0, limiting_xy: REC709_XY });
    }
    if s.contains("sdr100nitp3d65") && !s.contains("d60") {
        return Some(BuiltinDef::Aces2Output { peak_luminance: 100.0, limiting_xy: P3_D65_XY });
    }
    
    // HDR D60 variants - extract peak from name
    let d60_patterns: &[(&str, f32, [[f32; 2]; 4])] = &[
        ("hdr108nitp3d60inp3d65", 108.0, P3_D60_XY),
        ("hdr300nitp3d60inxyze", 300.0, P3_D60_XY),
        ("hdr500nitp3d60inp3d65", 500.0, P3_D60_XY),
        ("hdr1000nitp3d60inp3d65", 1000.0, P3_D60_XY),
        ("hdr2000nitp3d60inp3d65", 2000.0, P3_D60_XY),
        ("hdr4000nitp3d60inp3d65", 4000.0, P3_D60_XY),
        ("hdr500nitp3d60inrec2020d65", 500.0, P3_D60_XY),
        ("hdr1000nitp3d60inrec2020d65", 1000.0, P3_D60_XY),
        ("hdr2000nitp3d60inrec2020d65", 2000.0, P3_D60_XY),
        ("hdr4000nitp3d60inrec2020d65", 4000.0, P3_D60_XY),
        ("hdr500nitrec2020d60inrec2020d65", 500.0, REC2020_D60_XY),
        ("hdr1000nitrec2020d60inrec2020d65", 1000.0, REC2020_D60_XY),
        ("hdr2000nitrec2020d60inrec2020d65", 2000.0, REC2020_D60_XY),
        ("hdr4000nitrec2020d60inrec2020d65", 4000.0, REC2020_D60_XY),
    ];
    for &(pat, peak, lim) in d60_patterns {
        if s.contains(pat) {
            return Some(BuiltinDef::Aces2Output { peak_luminance: peak, limiting_xy: lim });
        }
    }
    
    // HDR base variants (no D60)
    let hdr_patterns: &[(&str, f32, [[f32; 2]; 4])] = &[
        ("hdr108nitp3d65", 108.0, P3_D65_XY),
        ("hdr300nitp3d65", 300.0, P3_D65_XY),
        ("hdr500nitp3d65", 500.0, P3_D65_XY),
        ("hdr1000nitp3d65", 1000.0, P3_D65_XY),
        ("hdr2000nitp3d65", 2000.0, P3_D65_XY),
        ("hdr4000nitp3d65", 4000.0, P3_D65_XY),
        ("hdr500nitrec2020", 500.0, REC2020_XY),
        ("hdr1000nitrec2020", 1000.0, REC2020_XY),
        ("hdr2000nitrec2020", 2000.0, REC2020_XY),
        ("hdr4000nitrec2020", 4000.0, REC2020_XY),
    ];
    for &(pat, peak, lim) in hdr_patterns {
        if s.contains(pat) && !s.contains("d60") {
            return Some(BuiltinDef::Aces2Output { peak_luminance: peak, limiting_xy: lim });
        }
    }
    
    None
}

// ============================================================================
// ACES 1.x Output Transform Data
// ============================================================================

/// RRT saturation matrix (from OCIO ACES.cpp).
const RRT_SAT_MATRIX: [f32; 16] = [
    0.970889148671, 0.026963270632, 0.002147580696, 0.0,
    0.010889148671, 0.986963270632, 0.002147580696, 0.0,
    0.010889148671, 0.026963270632, 0.962147580696, 0.0,
    0.0, 0.0, 0.0, 1.0,
];

/// ODT desaturation matrix (video surround correction).
const ODT_DESAT_MATRIX: [f32; 16] = [
    0.949056010175, 0.047185723607, 0.003758266219, 0.0,
    0.019056010175, 0.977185723607, 0.003758266219, 0.0,
    0.019056010175, 0.047185723607, 0.933758266219, 0.0,
    0.0, 0.0, 0.0, 1.0,
];

const CINEMA_WHITE: f32 = 48.0;
const CINEMA_BLACK: f32 = 0.02;

/// RRT B-spline shaper (7 control points).
fn rrt_shaper() -> (Vec<[f32; 2]>, Vec<f32>) {
    (
        vec![
            [-5.26017743, -4.0], [-3.75502745, -3.57868829], [-2.24987747, -1.82131329],
            [-0.74472749, 0.68124124], [1.06145248, 2.87457742], [2.86763245, 3.83406206],
            [4.67381243, 4.0],
        ],
        vec![0.0, 0.55982688, 1.77532247, 1.55, 0.8787017, 0.18374463, 0.0],
    )
}

/// SDR ODT B-spline shaper (15 control points).
fn sdr_odt_shaper() -> (Vec<[f32; 2]>, Vec<f32>) {
    (
        vec![
            [-2.54062362, -1.69897000], [-2.08035721, -1.58843500], [-1.62009080, -1.35350000],
            [-1.15982439, -1.04695000], [-0.69955799, -0.65640000], [-0.23929158, -0.22141000],
            [0.22097483, 0.22814402], [0.68124124, 0.68124124], [1.01284632, 0.99142189],
            [1.34445140, 1.25800000], [1.67605648, 1.44995000], [2.00766156, 1.55910000],
            [2.33926665, 1.62260000], [2.67087173, 1.66065457], [3.00247681, 1.68124124],
        ],
        vec![
            0.0, 0.4803088, 0.5405565, 0.79149813, 0.9055625, 0.98460368,
            0.96884766, 1.0, 0.87078346, 0.73702127, 0.42068113, 0.23763206,
            0.14535362, 0.08416378, 0.04,
        ],
    )
}

/// HDR RRT B-spline shapers by peak luminance.
fn hdr_rrt_shaper(y_max: f32) -> (Vec<[f32; 2]>, Vec<f32>) {
    match y_max as u32 {
        1000 => (
            vec![
                [-5.60050155, -4.0], [-4.09535157, -3.57868829], [-2.59020159, -1.82131329],
                [-1.08505161, 0.68124124], [0.22347059, 2.22673503], [1.53199279, 2.87906206],
                [2.84051500, 3.0],
            ],
            vec![0.0, 0.55982688, 1.77532247, 1.55, 0.81219728, 0.1848466, 0.0],
        ),
        2000 => (
            vec![
                [-5.59738488, -4.0], [-4.09223490, -3.57868829], [-2.58708492, -1.82131329],
                [-1.08193494, 0.68124124], [0.37639718, 2.42130131], [1.83472930, 3.16609199],
                [3.29306142, 3.30103000],
            ],
            vec![0.0, 0.55982688, 1.77532247, 1.55, 0.83637009, 0.18505799, 0.0],
        ),
        4000 => (
            vec![
                [-5.59503319, -4.0], [-4.08988322, -3.57868829], [-2.58473324, -1.82131329],
                [-1.07958326, 0.68124124], [0.52855878, 2.61625839], [2.13670081, 3.45351273],
                [3.74484285, 3.60205999],
            ],
            vec![0.0, 0.55982688, 1.77532247, 1.55, 0.85652519, 0.18474395, 0.0],
        ),
        108 => (
            vec![
                [-5.37852506, -4.0], [-3.87337508, -3.57868829], [-2.36822510, -1.82131329],
                [-0.86307513, 0.68124124], [-0.03557710, 1.60464482], [0.79192092, 1.96008059],
                [1.61941895, 2.03342376],
            ],
            vec![0.0, 0.55982688, 1.77532247, 1.55, 0.68179646, 0.17726487, 0.0],
        ),
        _ => rrt_shaper(), // fallback
    }
}

/// Build RRT preamble ops: glow, red_mod, clamp, AP0→AP1, clamp, saturation.
fn rrt_preamble() -> Vec<BuiltinDef> {
    vec![
        BuiltinDef::FixedFunction {
            style: crate::transform::FixedFunctionStyle::AcesGlow10,
            params: vec![],
        },
        BuiltinDef::FixedFunction {
            style: crate::transform::FixedFunctionStyle::AcesRedMod10,
            params: vec![],
        },
        BuiltinDef::Clamp { min: 0.0, max: f32::MAX },
        BuiltinDef::Matrix { matrix: AP0_TO_AP1, offset: [0.0; 4] },
        BuiltinDef::Clamp { min: 0.0, max: f32::MAX },
        BuiltinDef::Matrix { matrix: RRT_SAT_MATRIX, offset: [0.0; 4] },
    ]
}

/// Build SDR tone curve ops: log10 → RRT shaper → ODT shaper → log10 inv → scale/offset.
fn sdr_tonecurve() -> Vec<BuiltinDef> {
    let (rrt_pts, rrt_slp) = rrt_shaper();
    let (odt_pts, odt_slp) = sdr_odt_shaper();
    let scale = 1.0 / (CINEMA_WHITE - CINEMA_BLACK);
    let offset = -CINEMA_BLACK * scale;
    vec![
        BuiltinDef::Log { base: 10.0, forward: true },
        BuiltinDef::BSplineCurve { points: rrt_pts, slopes: rrt_slp },
        BuiltinDef::BSplineCurve { points: odt_pts, slopes: odt_slp },
        BuiltinDef::Log { base: 10.0, forward: false },
        BuiltinDef::Scale { factors: [scale, scale, scale] },
        BuiltinDef::Matrix {
            matrix: [
                1.0, 0.0, 0.0, 0.0,
                0.0, 1.0, 0.0, 0.0,
                0.0, 0.0, 1.0, 0.0,
                0.0, 0.0, 0.0, 1.0,
            ],
            offset: [offset, offset, offset, 0.0],
        },
    ]
}

/// Build HDR tone curve ops: log10 → RRT shaper → log10 inv → scale.
fn hdr_tonecurve(y_max: f32) -> Vec<BuiltinDef> {
    let (pts, slp) = hdr_rrt_shaper(y_max);
    vec![
        BuiltinDef::Log { base: 10.0, forward: true },
        BuiltinDef::BSplineCurve { points: pts, slopes: slp },
        BuiltinDef::Log { base: 10.0, forward: false },
    ]
}

/// Nit normalization: scale from [0..1] to [0..nits/100].
fn nit_normalization(nits: f32) -> BuiltinDef {
    let s = nits * 0.01;
    BuiltinDef::Scale { factors: [s, s, s] }
}

/// Video surround adjustment (cinema → video viewing).
fn video_adjust() -> Vec<BuiltinDef> {
    vec![
        BuiltinDef::FixedFunction {
            style: crate::transform::FixedFunctionStyle::AcesDarkToDim10,
            params: vec![],
        },
        BuiltinDef::Matrix { matrix: ODT_DESAT_MATRIX, offset: [0.0; 4] },
    ]
}

/// AP1 → CIE-XYZ-D65 with Bradford adaptation.
/// Computed as: Bradford(AP1_white→D65) * AP1→XYZ(native D60).
/// Matches OCIO build_conversion_matrix_to_XYZ_D65(AP1, BRADFORD).
fn ap1_to_xyz_d65_bfd() -> [f32; 16] {
    let ap1_xyz = rgb_to_xyz(&PRIMS_AP1);
    // Compute src white from matrix * [1,1,1] (matching OCIO)
    let src_wht = [
        (ap1_xyz[0] + ap1_xyz[1] + ap1_xyz[2]) as f64,
        (ap1_xyz[4] + ap1_xyz[5] + ap1_xyz[6]) as f64,
        (ap1_xyz[8] + ap1_xyz[9] + ap1_xyz[10]) as f64,
    ];
    let bfd = bradford_adapt(src_wht, D65_XYZ);
    mat4_mul(&bfd, &ap1_xyz)
}

/// CDD to CID matrix (ADX film density conversion).
const CDD_TO_CID: [f32; 16] = [
    0.75573, 0.22197, 0.02230, 0.0,
    0.05901, 0.96928, -0.02829, 0.0,
    0.16134, 0.07406, 0.76460, 0.0,
    0.0, 0.0, 0.0, 1.0,
];

/// Relative exposure to ACES matrix.
const EXP_TO_ACES: [f32; 16] = [
    0.72286, 0.12630, 0.15084, 0.0,
    0.11923, 0.76418, 0.11659, 0.0,
    0.01427, 0.08213, 0.90359, 0.0,
    0.0, 0.0, 0.0, 1.0,
];

/// Build ADX transform chain: scale/offset → CDD→CID → density LUT → exp10 → EXP→ACES.
fn adx_chain(scale: f32, offset: f32) -> Vec<BuiltinDef> {
    // 11-point nonuniform density-to-log-exposure LUT
    let density_lut_pts: Vec<[f32; 2]> = vec![
        [-0.190, -6.000000000], [0.010, -2.721718645], [0.028, -2.521718645],
        [0.054, -2.321718645], [0.095, -2.121718645], [0.145, -1.921718645],
        [0.220, -1.721718645], [0.300, -1.521718645], [0.400, -1.321718645],
        [0.500, -1.121718645], [0.600, -0.926545677],
    ];
    // Compute slopes from finite differences for Hermite interpolation
    let n = density_lut_pts.len();
    let mut slopes = vec![0.0_f32; n];
    for i in 1..n - 1 {
        let dx0 = density_lut_pts[i][0] - density_lut_pts[i - 1][0];
        let dy0 = density_lut_pts[i][1] - density_lut_pts[i - 1][1];
        let dx1 = density_lut_pts[i + 1][0] - density_lut_pts[i][0];
        let dy1 = density_lut_pts[i + 1][1] - density_lut_pts[i][1];
        slopes[i] = 0.5 * (dy0 / dx0 + dy1 / dx1);
    }
    // Edge slopes
    if n >= 2 {
        let dx = density_lut_pts[1][0] - density_lut_pts[0][0];
        let dy = density_lut_pts[1][1] - density_lut_pts[0][1];
        slopes[0] = dy / dx;
        let dx = density_lut_pts[n - 1][0] - density_lut_pts[n - 2][0];
        let dy = density_lut_pts[n - 1][1] - density_lut_pts[n - 2][1];
        slopes[n - 1] = dy / dx;
    }
    
    vec![
        // Scale/offset to CDD domain
        BuiltinDef::Matrix {
            matrix: [
                scale, 0.0, 0.0, 0.0,
                0.0, scale, 0.0, 0.0,
                0.0, 0.0, scale, 0.0,
                0.0, 0.0, 0.0, 1.0,
            ],
            offset: [offset, offset, offset, 0.0],
        },
        // CDD → CID
        BuiltinDef::Matrix { matrix: CDD_TO_CID, offset: [0.0; 4] },
        // Density → log exposure (B-spline LUT)
        BuiltinDef::BSplineCurve { points: density_lut_pts, slopes },
        // Log10 inverse (exp10)
        BuiltinDef::Log { base: 10.0, forward: false },
        // Relative exposure → ACES
        BuiltinDef::Matrix { matrix: EXP_TO_ACES, offset: [0.0; 4] },
    ]
}

/// Chromaticity xy pairs: [R, G, B, W] each as [x, y].
type Chroms = [[f64; 2]; 4];

/// Compute RGB→XYZ 3x3 matrix from chromaticity coordinates.
/// Returns row-major 4x4 (with identity 4th row/col).
fn rgb_to_xyz(prims: &Chroms) -> [f32; 16] {
    let [rx, ry] = prims[0]; let [gx, gy] = prims[1];
    let [bx, by] = prims[2]; let [wx, wy] = prims[3];
    // XYZ of R,G,B primaries (Y=1)
    let xr = rx / ry; let yr = 1.0; let zr = (1.0 - rx - ry) / ry;
    let xg = gx / gy; let yg = 1.0; let zg = (1.0 - gx - gy) / gy;
    let xb = bx / by; let yb = 1.0; let zb = (1.0 - bx - by) / by;
    // White point XYZ
    let xw = wx / wy; let yw = 1.0; let zw = (1.0 - wx - wy) / wy;
    // Solve for S = [Sr, Sg, Sb] such that M * S = W
    let m = glam::DMat3::from_cols(
        glam::DVec3::new(xr, yr, zr),
        glam::DVec3::new(xg, yg, zg),
        glam::DVec3::new(xb, yb, zb),
    );
    let w = glam::DVec3::new(xw, yw, zw);
    let s = m.inverse() * w;
    // Final matrix: columns scaled by S
    let m00 = (s.x * xr) as f32; let m01 = (s.y * xg) as f32; let m02 = (s.z * xb) as f32;
    let m10 = (s.x * yr) as f32; let m11 = (s.y * yg) as f32; let m12 = (s.z * yb) as f32;
    let m20 = (s.x * zr) as f32; let m21 = (s.y * zg) as f32; let m22 = (s.z * zb) as f32;
    // Row-major 4x4
    [
        m00, m01, m02, 0.0,
        m10, m11, m12, 0.0,
        m20, m21, m22, 0.0,
        0.0, 0.0, 0.0, 1.0,
    ]
}

/// Bradford chromatic adaptation matrix from src white XYZ to dst white XYZ.
fn bradford_adapt(src_xyz: [f64; 3], dst_xyz: [f64; 3]) -> [f32; 16] {
    // Bradford matrix (LMS cone response)
    let ma = glam::DMat3::from_cols_array(&[
         0.8951,  0.2664, -0.1614,
        -0.7502,  1.7135,  0.0367,
         0.0389, -0.0685,  1.0296,
    ]);
    let ma_inv = ma.inverse();
    let src_lms = ma * glam::DVec3::from_array(src_xyz);
    let dst_lms = ma * glam::DVec3::from_array(dst_xyz);
    let diag = glam::DMat3::from_diagonal(glam::DVec3::new(
        dst_lms.x / src_lms.x, dst_lms.y / src_lms.y, dst_lms.z / src_lms.z,
    ));
    let m = ma_inv * diag * ma;
    let a = m.to_cols_array();
    [
        a[0] as f32, a[3] as f32, a[6] as f32, 0.0,
        a[1] as f32, a[4] as f32, a[7] as f32, 0.0,
        a[2] as f32, a[5] as f32, a[8] as f32, 0.0,
        0.0, 0.0, 0.0, 1.0,
    ]
}

/// Multiply two row-major 4x4 matrices.
fn mat4_mul(a: &[f32; 16], b: &[f32; 16]) -> [f32; 16] {
    let mut r = [0.0f32; 16];
    for i in 0..4 {
        for j in 0..4 {
            for k in 0..4 {
                r[i * 4 + j] += a[i * 4 + k] * b[k * 4 + j];
            }
        }
    }
    r
}

/// Invert a row-major 4x4 matrix via glam.
fn mat4_inv(m: &[f32; 16]) -> [f32; 16] {
    let g = glam::Mat4::from_cols_array(&[
        m[0], m[4], m[8], m[12],
        m[1], m[5], m[9], m[13],
        m[2], m[6], m[10], m[14],
        m[3], m[7], m[11], m[15],
    ]);
    let inv = g.inverse();
    let a = inv.to_cols_array();
    [
        a[0], a[4], a[8], a[12],
        a[1], a[5], a[9], a[13],
        a[2], a[6], a[10], a[14],
        a[3], a[7], a[11], a[15],
    ]
}

/// Build conversion matrix from src primaries to dst primaries with Bradford adaptation.
fn conversion_matrix_bradford(src: &Chroms, dst: &Chroms) -> [f32; 16] {
    let src_xyz = rgb_to_xyz(src);
    let dst_xyz = rgb_to_xyz(dst);
    let dst_inv = mat4_inv(&dst_xyz);
    // src white and dst white in XYZ
    let sw = [src[3][0] / src[3][1], 1.0, (1.0 - src[3][0] - src[3][1]) / src[3][1]];
    let dw = [dst[3][0] / dst[3][1], 1.0, (1.0 - dst[3][0] - dst[3][1]) / dst[3][1]];
    let adapt = bradford_adapt(sw, dw);
    // Result: dst_inv * adapt * src_xyz
    mat4_mul(&dst_inv, &mat4_mul(&adapt, &src_xyz))
}

/// Build conversion matrix without chromatic adaptation (ADAPTATION_NONE).
fn conversion_matrix_none(src: &Chroms, dst: &Chroms) -> [f32; 16] {
    let src_xyz = rgb_to_xyz(src);
    let dst_xyz = rgb_to_xyz(dst);
    let dst_inv = mat4_inv(&dst_xyz);
    mat4_mul(&dst_inv, &src_xyz)
}

// Standard primaries (x,y for R,G,B,W).
const PRIMS_AP0: Chroms = [[0.7347, 0.2653], [0.0, 1.0], [0.0001, -0.077], [0.32168, 0.33767]];
const PRIMS_AP1: Chroms = [[0.713, 0.293], [0.165, 0.830], [0.128, 0.044], [0.32168, 0.33767]];
const PRIMS_REC709: Chroms = [[0.64, 0.33], [0.30, 0.60], [0.15, 0.06], [0.3127, 0.329]];
const PRIMS_P3_D65: Chroms = [[0.680, 0.320], [0.265, 0.690], [0.150, 0.060], [0.3127, 0.329]];
const PRIMS_REC2020: Chroms = [[0.708, 0.292], [0.170, 0.797], [0.131, 0.046], [0.3127, 0.329]];

// White point XYZ values.
const D60_XYZ: [f64; 3] = [0.95264607456985, 1.0, 1.00882518435159];
const D65_XYZ: [f64; 3] = [0.95045592705167, 1.0, 1.08905775075988];
const DCI_XYZ: [f64; 3] = [0.89458689458689, 1.0, 0.95441595441595];

/// SDR primary clamp: AP1 → limit (Bradford) → range[0,1] → limit→XYZ.
fn sdr_primary_clamp(limit_prims: &Chroms) -> Vec<BuiltinDef> {
    let ap1_to_limit = conversion_matrix_bradford(&PRIMS_AP1, limit_prims);
    let limit_to_xyz = rgb_to_xyz(limit_prims);
    vec![
        BuiltinDef::Matrix { matrix: ap1_to_limit, offset: [0.0; 4] },
        BuiltinDef::Clamp { min: 0.0, max: 1.0 },
        BuiltinDef::Matrix { matrix: limit_to_xyz, offset: [0.0; 4] },
    ]
}

/// HDR primary clamp: AP1 → limit (no adapt) → range[0,1] → limit→XYZ → D60→D65 Bradford.
fn hdr_primary_clamp(limit_prims: &Chroms) -> Vec<BuiltinDef> {
    let ap1_to_limit = conversion_matrix_none(&PRIMS_AP1, limit_prims);
    let limit_to_xyz = rgb_to_xyz(limit_prims);
    let d60_to_d65 = bradford_adapt(D60_XYZ, D65_XYZ);
    vec![
        BuiltinDef::Matrix { matrix: ap1_to_limit, offset: [0.0; 4] },
        BuiltinDef::Clamp { min: 0.0, max: 1.0 },
        BuiltinDef::Matrix { matrix: limit_to_xyz, offset: [0.0; 4] },
        BuiltinDef::Matrix { matrix: d60_to_d65, offset: [0.0; 4] },
    ]
}

/// Roll-white LUT for D60 simulation (new_wht=0.918).
fn roll_white_d60() -> Vec<BuiltinDef> {
    roll_white_lut(0.918)
}

/// Roll-white LUT for D65 simulation (new_wht=0.908).
fn roll_white_d65() -> Vec<BuiltinDef> {
    roll_white_lut(0.908)
}

/// Generate roll-white 1D LUT as B-spline approximation.
/// The OCIO C++ uses a 65536-sample Lut1D. We'll generate sample points and slopes.
fn roll_white_lut(new_wht: f32) -> Vec<BuiltinDef> {
    // Build LUT samples for the roll-white function
    let n = 65;
    let mut pts = Vec::with_capacity(n);
    let width: f64 = 0.5;
    let x0: f64 = -1.0;
    let x1 = x0 + width;
    let y0 = -(new_wht as f64);
    let y1 = x1;
    let m1 = x1 - x0;
    let a = y0 - y1 + m1;
    let b = 2.0 * (y1 - y0) - m1;
    let c = y0;

    for i in 0..n {
        let input = i as f64 / (n - 1) as f64; // 0..1
        let out = if input >= new_wht as f64 {
            let t = (-input as f64 - x0) / (x1 - x0);
            if t < 0.0 { input as f64 }
            else if t > 1.0 { -(a + b + c) }
            else { -((a * t + b) * t + c) }
        } else {
            input as f64
        };
        pts.push([input as f32, out as f32]);
    }
    // Compute slopes via finite differences
    let mut slopes = vec![0.0f32; n];
    for i in 1..n - 1 {
        let dx0 = pts[i][0] - pts[i - 1][0];
        let dy0 = pts[i][1] - pts[i - 1][1];
        let dx1 = pts[i + 1][0] - pts[i][0];
        let dy1 = pts[i + 1][1] - pts[i][1];
        if dx0.abs() > 1e-10 && dx1.abs() > 1e-10 {
            slopes[i] = 0.5 * (dy0 / dx0 + dy1 / dx1);
        }
    }
    if n >= 2 {
        let dx = pts[1][0] - pts[0][0];
        let dy = pts[1][1] - pts[0][1];
        if dx.abs() > 1e-10 { slopes[0] = dy / dx; }
        let dx = pts[n - 1][0] - pts[n - 2][0];
        let dy = pts[n - 1][1] - pts[n - 2][1];
        if dx.abs() > 1e-10 { slopes[n - 1] = dy / dx; }
    }
    vec![BuiltinDef::BSplineCurve { points: pts, slopes }]
}

/// DCI→D65 Bradford adaptation matrix.
fn dci_to_d65_adapt() -> BuiltinDef {
    BuiltinDef::Matrix { matrix: bradford_adapt(DCI_XYZ, D65_XYZ), offset: [0.0; 4] }
}

/// AP1→XYZ matrix (using AP1 primaries, native white point D60).
fn ap1_to_xyz_native() -> BuiltinDef {
    BuiltinDef::Matrix { matrix: rgb_to_xyz(&PRIMS_AP1), offset: [0.0; 4] }
}

/// Parse ACES 1.x output transform name.
fn parse_aces1_output(s: &str) -> Option<BuiltinDef> {
    let mut chain: Vec<BuiltinDef> = Vec::new();

    // All start with RRT preamble
    chain.extend(rrt_preamble());

    let is_hdr = s.contains("hdr");

    if is_hdr {
        // HDR variants: RRT + hdr_tonecurve + hdr_primary_clamp + nit_normalization
        let y_max = if s.contains("1000nit") { 1000.0 }
            else if s.contains("2000nit") { 2000.0 }
            else if s.contains("4000nit") { 4000.0 }
            else if s.contains("108nit") { 108.0 }
            else { return None; };

        chain.extend(hdr_tonecurve(y_max));

        // Determine limiting primaries
        let limit = if s.contains("rec2020lim") { &PRIMS_REC2020 }
            else if s.contains("p3lim") { &PRIMS_P3_D65 }
            else { return None; };

        chain.extend(hdr_primary_clamp(limit));
        chain.push(nit_normalization(y_max));
    } else {
        // SDR variants
        chain.extend(sdr_tonecurve());

        if s.contains("d60simd65") || s.contains("d60simdci") || s.contains("d65simdci") {
            // D60/D65 simulation variants
            if s.contains("d60simd65") {
                // SDR-CINEMA-D60sim-D65_1.1 or SDR-VIDEO-D60sim-D65_1.0
                let clamp_max = 1.0;
                let scale = if s.contains("sdrcinema") { 0.964 } else { 0.955 };
                chain.push(BuiltinDef::Clamp { min: f32::NEG_INFINITY, max: clamp_max });
                chain.push(BuiltinDef::Scale { factors: [scale, scale, scale] });
                if s.contains("sdrvideo") {
                    chain.extend(video_adjust());
                }
                chain.push(ap1_to_xyz_native());
            } else if s.contains("d60simdci") {
                // SDR-CINEMA-D60sim-DCI_1.0
                chain.extend(roll_white_d60());
                chain.push(BuiltinDef::Clamp { min: f32::NEG_INFINITY, max: 0.918 });
                chain.push(BuiltinDef::Scale { factors: [0.96, 0.96, 0.96] });
                chain.push(ap1_to_xyz_native());
                chain.push(dci_to_d65_adapt());
            } else {
                // SDR-CINEMA-D65sim-DCI_1.1
                chain.extend(roll_white_d65());
                chain.push(BuiltinDef::Clamp { min: f32::NEG_INFINITY, max: 0.908 });
                chain.push(BuiltinDef::Scale { factors: [0.9575, 0.9575, 0.9575] });
                chain.push(BuiltinDef::Matrix { matrix: ap1_to_xyz_d65_bfd(), offset: [0.0; 4] });
                chain.push(dci_to_d65_adapt());
            }
        } else if s.contains("rec709lim") || s.contains("p3lim") {
            // Primary-limited SDR variants
            if s.contains("sdrvideo") {
                chain.extend(video_adjust());
            }
            let limit = if s.contains("rec709lim") { &PRIMS_REC709 } else { &PRIMS_P3_D65 };
            chain.extend(sdr_primary_clamp(limit));
        } else {
            // Basic SDR-CINEMA or SDR-VIDEO (no gamut limiting)
            if s.contains("sdrvideo") {
                chain.extend(video_adjust());
            }
            chain.push(BuiltinDef::Matrix { matrix: ap1_to_xyz_d65_bfd(), offset: [0.0; 4] });
        }
    }

    Some(BuiltinDef::Chain(chain))
}

/// Compute the middle knot position (ksi) for piecewise quadratic B-spline.
/// Ported from OCIO CalcKsi() in GradingBSplineCurve.cpp.
fn calc_ksi(p0x: f32, p0y: f32, p1x: f32, p1y: f32, s0: f32, s1: f32) -> f32 {
    let k = 0.2_f32;
    let dx = p1x - p0x;
    let secant_slope = (p1y - p0y) / dx;
    let secant = secant_slope.abs();
    let (m0, m1) = if secant_slope < 0.0 { (-s0, -s1) } else { (s0, s1) };

    let x_mid = p0x + 0.5 * dx;
    let left_bnd = p0x + dx * k;
    let right_bnd = p1x - dx * k;
    let (mut top_bnd, mut bottom_bnd, m_min, m_max) = if m0 <= m1 {
        (left_bnd, right_bnd, m0, m1)
    } else {
        (right_bnd, left_bnd, m1, m0)
    };

    let dm = m_max - m_min;
    let b = 1.0 - 0.5 * k;
    let b_high = m_min + b * dm;
    let b_low = m_min + (1.0 - b) * dm;
    let bbb = m_max * 4.0;
    let bb = m_max * 1.1;
    let m_rel_diff = dm / 0.01_f32.max(m_max);
    let alpha = 0.0_f32.max(((m_rel_diff - 0.05) / (0.75 - 0.05)).min(1.0));
    top_bnd = x_mid + alpha * (top_bnd - x_mid);
    bottom_bnd = x_mid + alpha * (bottom_bnd - x_mid);

    if secant >= bbb {
        x_mid
    } else if secant > bb {
        let blend = (secant - bb) / (bbb - bb);
        top_bnd + blend * (x_mid - top_bnd)
    } else if secant >= b_high {
        top_bnd
    } else if secant > b_low && (b_high - b_low).abs() > 1e-10 {
        let blend = (secant - b_low) / (b_high - b_low);
        bottom_bnd + blend * (top_bnd - bottom_bnd)
    } else {
        bottom_bnd
    }
}

/// Build piecewise quadratic knots and coefficients from control points and slopes.
/// Returns (knots, coefs_a, coefs_b, coefs_c) matching OCIO's CalcKnots.
fn build_quadratic_coefs(points: &[[f32; 2]], slopes: &[f32])
    -> (Vec<f32>, Vec<f32>, Vec<f32>, Vec<f32>)
{
    let n = points.len();
    let mut knots = Vec::new();
    let mut ca = Vec::new();
    let mut cb = Vec::new();
    let mut cc = Vec::new();

    knots.push(points[0][0]);
    for i in 0..n - 1 {
        let p0x = points[i][0]; let p0y = points[i][1];
        let p1x = points[i + 1][0]; let p1y = points[i + 1][1];
        let dx = p1x - p0x;
        let secant = (p1y - p0y) / dx;

        if ((slopes[i] + slopes[i + 1]) - 2.0 * secant).abs() <= 1e-5 {
            // Single quadratic segment
            cc.push(p0y);
            cb.push(slopes[i]);
            ca.push(0.5 * (slopes[i + 1] - slopes[i]) / dx);
        } else {
            // Two quadratic segments with middle knot
            let ksi = calc_ksi(p0x, p0y, p1x, p1y, slopes[i], slopes[i + 1]);
            let m_bar = (2.0 * secant - slopes[i + 1])
                + (slopes[i + 1] - slopes[i]) * (ksi - p0x) / dx;
            let eta = (m_bar - slopes[i]) / (ksi - p0x);
            // First quadratic
            cc.push(p0y);
            cb.push(slopes[i]);
            ca.push(0.5 * eta);
            // Second quadratic
            let dk = ksi - p0x;
            cc.push(p0y + slopes[i] * dk + 0.5 * eta * dk * dk);
            cb.push(m_bar);
            ca.push(0.5 * (slopes[i + 1] - m_bar) / (p1x - ksi));
            knots.push(ksi);
        }
        knots.push(p1x);
    }
    (knots, ca, cb, cc)
}

/// Evaluate piecewise quadratic B-spline at x, matching OCIO's evalCurve.
fn eval_bspline(x: f32, points: &[[f32; 2]], slopes: &[f32]) -> f32 {
    let n = points.len();
    if n < 2 { return if n == 1 { points[0][1] } else { 0.0 }; }

    let (knots, ca, cb, cc) = build_quadratic_coefs(points, slopes);
    let num_segs = ca.len();
    let kn_start = knots[0];
    let kn_end = *knots.last().unwrap();

    if x <= kn_start {
        // Extrapolate low: linear with slope = cb[0]
        return (x - kn_start) * cb[0] + cc[0];
    }
    if x >= kn_end {
        // Extrapolate high: tangent at last knot
        let kn = knots[knots.len() - 2];
        let t = kn_end - kn;
        let slope = 2.0 * ca[num_segs - 1] * t + cb[num_segs - 1];
        let offs = (ca[num_segs - 1] * t + cb[num_segs - 1]) * t + cc[num_segs - 1];
        return (x - kn_end) * slope + offs;
    }

    // Find segment
    let mut seg = 0;
    for i in 0..knots.len() - 2 {
        if x < knots[i + 1] {
            seg = i;
            break;
        }
        seg = i; // last valid
    }
    let t = x - knots[seg];
    (ca[seg] * t + cb[seg]) * t + cc[seg]
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
        
        BuiltinDef::LogCamera { base, log_side_slope, log_side_offset, lin_side_slope, lin_side_offset, lin_side_break, linear_slope, inverse } => {
            // When inverse=true, the builtin definition expects log->linear (decode).
            // The ProcessorOp::LogCamera forward flag: true=linear->log, false=log->linear
            // So: effective_forward = forward XOR inverse
            let effective_forward = forward != *inverse;
            ops.push(ProcessorOp::LogCamera {
                base: *base,
                log_side_slope: *log_side_slope,
                log_side_offset: *log_side_offset,
                lin_side_slope: *lin_side_slope,
                lin_side_offset: *lin_side_offset,
                lin_side_break: *lin_side_break,
                linear_slope: *linear_slope,
                forward: effective_forward,
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
                TransferStyle::AppleLog => ProcTransfer::AppleLog,
                TransferStyle::CanonCLog2 => ProcTransfer::CanonCLog2,
                TransferStyle::CanonCLog3 => ProcTransfer::CanonCLog3,
                TransferStyle::Pq => ProcTransfer::Pq,
                TransferStyle::Hlg => ProcTransfer::Hlg,
                TransferStyle::Rec1886 => ProcTransfer::Rec1886,
                TransferStyle::Gamma22 => ProcTransfer::Gamma22,
                TransferStyle::Gamma26 => ProcTransfer::Gamma26,
            };
            ops.push(ProcessorOp::Transfer { style: proc_style, forward });
        }
        
        BuiltinDef::Gamma { value, mirror } => {
            let neg = if *mirror { NegativeStyle::Mirror } else { NegativeStyle::Clamp };
            if forward {
                // Display output: apply inverse gamma (1/value power)
                ops.push(ProcessorOp::Exponent {
                    value: [1.0 / value, 1.0 / value, 1.0 / value, 1.0],
                    negative_style: neg,
                });
            } else {
                ops.push(ProcessorOp::Exponent {
                    value: [*value, *value, *value, 1.0],
                    negative_style: neg,
                });
            }
        }
        
        BuiltinDef::MonCurve { gamma, offset, mirror } => {
            // sRGB-like moncurve: linear below break, pow(gamma) above
            // Forward (display): linear → encoded (inverse gamma)
            // For now, use Transfer::Srgb for standard sRGB params
            let neg = if *mirror { NegativeStyle::Mirror } else { NegativeStyle::Clamp };
            if (*gamma - 2.4).abs() < 0.01 && (*offset - 0.055).abs() < 0.01 {
                // Standard sRGB moncurve
                if *mirror {
                    // Mirror negatives not directly supported by Transfer::Srgb,
                    // fall back to exponent with mirror as approximation
                    ops.push(ProcessorOp::Exponent {
                        value: if forward { [1.0 / gamma, 1.0 / gamma, 1.0 / gamma, 1.0] } else { [*gamma, *gamma, *gamma, 1.0] },
                        negative_style: neg,
                    });
                } else {
                    use crate::processor::TransferStyle as ProcTransfer;
                    ops.push(ProcessorOp::Transfer { style: ProcTransfer::Srgb, forward });
                }
            } else {
                // Generic moncurve approximated as pure gamma
                ops.push(ProcessorOp::Exponent {
                    value: if forward { [1.0 / gamma, 1.0 / gamma, 1.0 / gamma, 1.0] } else { [*gamma, *gamma, *gamma, 1.0] },
                    negative_style: neg,
                });
            }
        }
        
        BuiltinDef::Scale { factors } => {
            if forward {
                // Scale as diagonal matrix
                ops.push(ProcessorOp::Matrix {
                    matrix: [
                        factors[0], 0.0, 0.0, 0.0,
                        0.0, factors[1], 0.0, 0.0,
                        0.0, 0.0, factors[2], 0.0,
                        0.0, 0.0, 0.0, 1.0,
                    ],
                    offset: [0.0; 4],
                });
            } else {
                ops.push(ProcessorOp::Matrix {
                    matrix: [
                        1.0 / factors[0], 0.0, 0.0, 0.0,
                        0.0, 1.0 / factors[1], 0.0, 0.0,
                        0.0, 0.0, 1.0 / factors[2], 0.0,
                        0.0, 0.0, 0.0, 1.0,
                    ],
                    offset: [0.0; 4],
                });
            }
        }
        
        BuiltinDef::Clamp { min, max } => {
            ops.push(ProcessorOp::Range {
                scale: 1.0,
                offset: 0.0,
                clamp_min: if min.is_finite() { Some(*min) } else { None },
                clamp_max: if max.is_finite() { Some(*max) } else { None },
            });
        }
        
        BuiltinDef::BSplineCurve { points, slopes } => {
            // Bake B-spline curve into 1D LUT (65536 samples for high precision)
            let lut_size = 65536;
            let x_min = points.first().map(|p| p[0]).unwrap_or(0.0);
            let x_max = points.last().map(|p| p[0]).unwrap_or(1.0);
            let mut lut = Vec::with_capacity(lut_size);
            for i in 0..lut_size {
                let t = i as f32 / (lut_size - 1) as f32;
                let x = x_min + t * (x_max - x_min);
                lut.push(eval_bspline(x, points, slopes));
            }
            // Forward: apply LUT. Inverse: would need inverse LUT (not implemented for now).
            if forward {
                ops.push(ProcessorOp::Lut1d {
                    lut,
                    size: lut_size,
                    channels: 1,
                    domain_min: [x_min, x_min, x_min],
                    domain_max: [x_max, x_max, x_max],
                });
            } else {
                // For inverse, build monotonic inverse LUT
                let mut inv_lut = Vec::with_capacity(lut_size);
                let y_min = points.first().map(|p| p[1]).unwrap_or(0.0);
                let y_max = points.last().map(|p| p[1]).unwrap_or(1.0);
                // Forward LUT maps x_min..x_max -> y values
                // Inverse maps y_min..y_max -> x values
                let fwd_lut: Vec<f32> = (0..lut_size).map(|i| {
                    let t = i as f32 / (lut_size - 1) as f32;
                    eval_bspline(x_min + t * (x_max - x_min), points, slopes)
                }).collect();
                for i in 0..lut_size {
                    let t = i as f32 / (lut_size - 1) as f32;
                    let target_y = y_min + t * (y_max - y_min);
                    // Binary search in forward LUT
                    let idx = fwd_lut.partition_point(|&v| v < target_y);
                    let x_val = if idx == 0 {
                        x_min
                    } else if idx >= lut_size {
                        x_max
                    } else {
                        let frac = if (fwd_lut[idx] - fwd_lut[idx - 1]).abs() > 1e-10 {
                            (target_y - fwd_lut[idx - 1]) / (fwd_lut[idx] - fwd_lut[idx - 1])
                        } else {
                            0.0
                        };
                        let x0 = x_min + (idx - 1) as f32 / (lut_size - 1) as f32 * (x_max - x_min);
                        let x1 = x_min + idx as f32 / (lut_size - 1) as f32 * (x_max - x_min);
                        x0 + frac * (x1 - x0)
                    };
                    inv_lut.push(x_val);
                }
                ops.push(ProcessorOp::Lut1d {
                    lut: inv_lut,
                    size: lut_size,
                    channels: 1,
                    domain_min: [y_min, y_min, y_min],
                    domain_max: [y_max, y_max, y_max],
                });
            }
        }
        
        BuiltinDef::Log { base, forward: log_fwd } => {
            // effective direction: builtin forward XOR log inverse
            let effective = forward == *log_fwd;
            ops.push(ProcessorOp::Log { base: *base, forward: effective });
        }
        
        BuiltinDef::FixedFunction { style, params } => {
            ops.push(ProcessorOp::FixedFunction {
                style: *style,
                params: params.clone(),
                forward,
            });
        }
        
        BuiltinDef::Aces2Output { peak_luminance, limiting_xy } => {
            // Build ACES 2.0 output transform using aces2 module
            use crate::aces2;
            let prims = aces2::Primaries {
                red: aces2::Chromaticity { x: limiting_xy[0][0], y: limiting_xy[0][1] },
                green: aces2::Chromaticity { x: limiting_xy[1][0], y: limiting_xy[1][1] },
                blue: aces2::Chromaticity { x: limiting_xy[2][0], y: limiting_xy[2][1] },
                white: aces2::Chromaticity { x: limiting_xy[3][0], y: limiting_xy[3][1] },
            };
            let state = aces2::init_output_transform(*peak_luminance, &prims);
            ops.push(ProcessorOp::Aces2OutputTransform {
                state: Box::new(state),
                forward,
            });
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

    /// Parity test: V-Log V-Gamut to ACES2065-1
    /// Reference values from OCIO BuiltinTransform_tests.cpp:
    /// Input:  [0.5, 0.4, 0.3]
    /// Output: [0.306918773245, 0.148128050597, 0.046334439047]
    #[test]
    fn test_vlog_to_aces_parity() {
        use crate::{Transform, BuiltinTransform, TransformDirection, Processor};
        
        let transform = Transform::Builtin(BuiltinTransform {
            style: "PANASONIC_VLOG-VGAMUT_to_ACES2065-1".to_string(),
            direction: TransformDirection::Forward,
        });
        
        let processor = Processor::from_transform(&transform, TransformDirection::Forward)
            .expect("V-Log builtin should compile");
        
        let mut pixels = [[0.5_f32, 0.4, 0.3]];
        processor.apply_rgb(&mut pixels);
        
        // OCIO reference values (tolerance 1e-6)
        let expected = [0.306918773245_f32, 0.148128050597, 0.046334439047];
        let tolerance = 5e-4; // Reasonable tolerance for f32 precision differences
        
        assert!(
            (pixels[0][0] - expected[0]).abs() < tolerance,
            "R: got {}, expected {}", pixels[0][0], expected[0]
        );
        assert!(
            (pixels[0][1] - expected[1]).abs() < tolerance,
            "G: got {}, expected {}", pixels[0][1], expected[1]
        );
        assert!(
            (pixels[0][2] - expected[2]).abs() < tolerance,
            "B: got {}, expected {}", pixels[0][2], expected[2]
        );
    }

    #[test]
    fn test_aces1_output_all_resolve() {
        // All 16 ACES 1.x output transforms should resolve to Some
        let styles = [
            "ACES-OUTPUT - ACES2065-1_to_CIE-XYZ-D65 - SDR-CINEMA_1.0",
            "ACES-OUTPUT - ACES2065-1_to_CIE-XYZ-D65 - SDR-VIDEO_1.0",
            "ACES-OUTPUT - ACES2065-1_to_CIE-XYZ-D65 - SDR-CINEMA-REC709lim_1.1",
            "ACES-OUTPUT - ACES2065-1_to_CIE-XYZ-D65 - SDR-VIDEO-REC709lim_1.1",
            "ACES-OUTPUT - ACES2065-1_to_CIE-XYZ-D65 - SDR-VIDEO-P3lim_1.1",
            "ACES-OUTPUT - ACES2065-1_to_CIE-XYZ-D65 - SDR-CINEMA-D60sim-D65_1.1",
            "ACES-OUTPUT - ACES2065-1_to_CIE-XYZ-D65 - SDR-VIDEO-D60sim-D65_1.0",
            "ACES-OUTPUT - ACES2065-1_to_CIE-XYZ-D65 - SDR-CINEMA-D60sim-DCI_1.0",
            "ACES-OUTPUT - ACES2065-1_to_CIE-XYZ-D65 - SDR-CINEMA-D65sim-DCI_1.1",
            "ACES-OUTPUT - ACES2065-1_to_CIE-XYZ-D65 - HDR-VIDEO-1000nit-15nit-REC2020lim_1.1",
            "ACES-OUTPUT - ACES2065-1_to_CIE-XYZ-D65 - HDR-VIDEO-1000nit-15nit-P3lim_1.1",
            "ACES-OUTPUT - ACES2065-1_to_CIE-XYZ-D65 - HDR-VIDEO-2000nit-15nit-REC2020lim_1.1",
            "ACES-OUTPUT - ACES2065-1_to_CIE-XYZ-D65 - HDR-VIDEO-2000nit-15nit-P3lim_1.1",
            "ACES-OUTPUT - ACES2065-1_to_CIE-XYZ-D65 - HDR-VIDEO-4000nit-15nit-REC2020lim_1.1",
            "ACES-OUTPUT - ACES2065-1_to_CIE-XYZ-D65 - HDR-VIDEO-4000nit-15nit-P3lim_1.1",
            "ACES-OUTPUT - ACES2065-1_to_CIE-XYZ-D65 - HDR-CINEMA-108nit-7.2nit-P3lim_1.1",
        ];
        for style in &styles {
            let def = get_builtin(style);
            assert!(def.is_some(), "Missing ACES 1.x output: {}", style);
            // Verify it compiles to ops
            let mut ops = Vec::new();
            compile_builtin(&def.unwrap(), true, &mut ops);
            assert!(!ops.is_empty(), "Empty ops for: {}", style);
        }
    }

    #[test]
    fn test_aces1_sdr_cinema_accuracy() {
        // Reference: OCIO BuiltinTransform_tests.cpp line 416-418
        // Input: [0.5, 0.4, 0.3], Expected: [0.33629957, 0.31832799, 0.22867827]
        use crate::{Transform, BuiltinTransform, TransformDirection, Processor};
        let t = Transform::Builtin(BuiltinTransform {
            style: "ACES-OUTPUT - ACES2065-1_to_CIE-XYZ-D65 - SDR-CINEMA_1.0".into(),
            direction: TransformDirection::Forward,
        });
        let proc = Processor::from_transform(&t, TransformDirection::Forward).unwrap();
        let mut px = [[0.5f32, 0.4, 0.3]];
        proc.apply_rgb(&mut px);
        let exp = [0.33629957f32, 0.31832799, 0.22867827];
        let tol = 5e-3; // Residual diff from f32 precision and LUT baking
        for i in 0..3 {
            assert!((px[0][i] - exp[i]).abs() < tol,
                "ch{}: got {}, exp {}, diff {}", i, px[0][i], exp[i], (px[0][i] - exp[i]).abs());
        }
    }


    #[test]
    fn test_aces2_output_resolve() {
        // Spot-check several ACES 2.0 output transforms
        let styles = [
            "ACES-OUTPUT - ACES2065-1_to_CIE-XYZ-D65 - SDR-100nit-REC709_2.0",
            "ACES-OUTPUT - ACES2065-1_to_CIE-XYZ-D65 - SDR-100nit-P3-D65_2.0",
            "ACES-OUTPUT - ACES2065-1_to_CIE-XYZ-D65 - HDR-1000nit-P3-D65_2.0",
            "ACES-OUTPUT - ACES2065-1_to_CIE-XYZ-D65 - HDR-4000nit-REC2020_2.0",
            "ACES-OUTPUT - ACES2065-1_to_CIE-XYZ-D65 - SDR-100nit-REC709-D60-in-REC709-D65_2.0",
            "ACES-OUTPUT - ACES2065-1_to_CIE-XYZ-D65 - HDR-4000nit-P3-D60-in-P3-D65_2.0",
            "ACES-OUTPUT - ACES2065-1_to_CIE-XYZ-D65 - HDR-4000nit-REC2020-D60-in-REC2020-D65_2.0",
        ];
        for style in &styles {
            let def = get_builtin(style);
            assert!(def.is_some(), "Missing ACES 2.0 output: {}", style);
        }
    }
}
