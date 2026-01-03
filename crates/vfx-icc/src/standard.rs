//! Standard ICC profile definitions.

use crate::Profile;
use lcms2::{CIExyY, CIExyYTRIPLE, Profile as LcmsProfile, ToneCurve};

/// Standard color profile specifications.
///
/// Pre-defined color spaces commonly used in VFX and color pipelines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StandardProfile {
    /// IEC 61966-2-1 sRGB.
    Srgb,
    /// Linear sRGB (gamma 1.0).
    LinearSrgb,
    /// Adobe RGB (1998).
    AdobeRgb,
    /// Display P3 (Apple).
    DisplayP3,
    /// DCI-P3 (Digital Cinema).
    DciP3,
    /// ACES AP0 (Academy).
    AcesAp0,
    /// ACES AP1 / ACEScg.
    AcesAp1,
    /// ITU-R BT.709.
    Rec709,
    /// ITU-R BT.2020.
    Rec2020,
}

impl StandardProfile {
    /// Creates an ICC profile from this standard.
    pub fn to_profile(self) -> Profile {
        match self {
            StandardProfile::Srgb => Profile {
                inner: LcmsProfile::new_srgb(),
            },
            StandardProfile::LinearSrgb => {
                let white = d65_white();
                let primaries = srgb_primaries();
                let curve = ToneCurve::new(1.0);
                let curves = [&curve, &curve, &curve];
                Profile {
                    inner: LcmsProfile::new_rgb(&white, &primaries, &curves)
                        .unwrap_or_else(|_| LcmsProfile::new_srgb()),
                }
            }
            StandardProfile::AdobeRgb => {
                let white = d65_white();
                let primaries = CIExyYTRIPLE {
                    Red: CIExyY { x: 0.6400, y: 0.3300, Y: 1.0 },
                    Green: CIExyY { x: 0.2100, y: 0.7100, Y: 1.0 },
                    Blue: CIExyY { x: 0.1500, y: 0.0600, Y: 1.0 },
                };
                let curve = ToneCurve::new(2.2);
                let curves = [&curve, &curve, &curve];
                Profile {
                    inner: LcmsProfile::new_rgb(&white, &primaries, &curves)
                        .unwrap_or_else(|_| LcmsProfile::new_srgb()),
                }
            }
            StandardProfile::DisplayP3 => {
                let white = d65_white();
                let primaries = p3_primaries();
                // sRGB transfer function approximation
                let curve = ToneCurve::new(2.2);
                let curves = [&curve, &curve, &curve];
                Profile {
                    inner: LcmsProfile::new_rgb(&white, &primaries, &curves)
                        .unwrap_or_else(|_| LcmsProfile::new_srgb()),
                }
            }
            StandardProfile::DciP3 => {
                // DCI white point (x=0.314, y=0.351)
                let white = CIExyY { x: 0.314, y: 0.351, Y: 1.0 };
                let primaries = p3_primaries();
                let curve = ToneCurve::new(2.6);
                let curves = [&curve, &curve, &curve];
                Profile {
                    inner: LcmsProfile::new_rgb(&white, &primaries, &curves)
                        .unwrap_or_else(|_| LcmsProfile::new_srgb()),
                }
            }
            StandardProfile::AcesAp0 => {
                let white = aces_white();
                let primaries = CIExyYTRIPLE {
                    Red: CIExyY { x: 0.7347, y: 0.2653, Y: 1.0 },
                    Green: CIExyY { x: 0.0000, y: 1.0000, Y: 1.0 },
                    Blue: CIExyY { x: 0.0001, y: -0.0770, Y: 1.0 },
                };
                let curve = ToneCurve::new(1.0);
                let curves = [&curve, &curve, &curve];
                Profile {
                    inner: LcmsProfile::new_rgb(&white, &primaries, &curves)
                        .unwrap_or_else(|_| LcmsProfile::new_srgb()),
                }
            }
            StandardProfile::AcesAp1 => {
                let white = aces_white();
                let primaries = CIExyYTRIPLE {
                    Red: CIExyY { x: 0.713, y: 0.293, Y: 1.0 },
                    Green: CIExyY { x: 0.165, y: 0.830, Y: 1.0 },
                    Blue: CIExyY { x: 0.128, y: 0.044, Y: 1.0 },
                };
                let curve = ToneCurve::new(1.0);
                let curves = [&curve, &curve, &curve];
                Profile {
                    inner: LcmsProfile::new_rgb(&white, &primaries, &curves)
                        .unwrap_or_else(|_| LcmsProfile::new_srgb()),
                }
            }
            StandardProfile::Rec709 => {
                let white = d65_white();
                let primaries = srgb_primaries(); // Same as sRGB
                // BT.709 transfer (simplified as gamma 2.4)
                let curve = ToneCurve::new(2.4);
                let curves = [&curve, &curve, &curve];
                Profile {
                    inner: LcmsProfile::new_rgb(&white, &primaries, &curves)
                        .unwrap_or_else(|_| LcmsProfile::new_srgb()),
                }
            }
            StandardProfile::Rec2020 => {
                let white = d65_white();
                let primaries = CIExyYTRIPLE {
                    Red: CIExyY { x: 0.708, y: 0.292, Y: 1.0 },
                    Green: CIExyY { x: 0.170, y: 0.797, Y: 1.0 },
                    Blue: CIExyY { x: 0.131, y: 0.046, Y: 1.0 },
                };
                // BT.2020 transfer (simplified)
                let curve = ToneCurve::new(2.4);
                let curves = [&curve, &curve, &curve];
                Profile {
                    inner: LcmsProfile::new_rgb(&white, &primaries, &curves)
                        .unwrap_or_else(|_| LcmsProfile::new_srgb()),
                }
            }
        }
    }
}

/// D65 white point.
fn d65_white() -> CIExyY {
    CIExyY { x: 0.3127, y: 0.3290, Y: 1.0 }
}

/// ACES white point (approximately D60).
fn aces_white() -> CIExyY {
    CIExyY { x: 0.32168, y: 0.33767, Y: 1.0 }
}

/// sRGB / Rec.709 primaries.
fn srgb_primaries() -> CIExyYTRIPLE {
    CIExyYTRIPLE {
        Red: CIExyY { x: 0.6400, y: 0.3300, Y: 1.0 },
        Green: CIExyY { x: 0.3000, y: 0.6000, Y: 1.0 },
        Blue: CIExyY { x: 0.1500, y: 0.0600, Y: 1.0 },
    }
}

/// P3 primaries.
fn p3_primaries() -> CIExyYTRIPLE {
    CIExyYTRIPLE {
        Red: CIExyY { x: 0.680, y: 0.320, Y: 1.0 },
        Green: CIExyY { x: 0.265, y: 0.690, Y: 1.0 },
        Blue: CIExyY { x: 0.150, y: 0.060, Y: 1.0 },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_standards() {
        let standards = [
            StandardProfile::Srgb,
            StandardProfile::LinearSrgb,
            StandardProfile::AdobeRgb,
            StandardProfile::DisplayP3,
            StandardProfile::DciP3,
            StandardProfile::AcesAp0,
            StandardProfile::AcesAp1,
            StandardProfile::Rec709,
            StandardProfile::Rec2020,
        ];

        for std in standards {
            let profile = std.to_profile();
            assert!(profile.is_rgb(), "{:?} should be RGB", std);
        }
    }
}
