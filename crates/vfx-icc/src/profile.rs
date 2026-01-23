//! ICC profile wrapper.

use crate::{IccError, IccResult, StandardProfile};
use lcms2::{ColorSpaceSignature, Profile as LcmsProfile};
use std::path::Path;

/// An ICC color profile.
///
/// Represents a color space and its associated color management data.
/// Profiles can be loaded from files, created from raw ICC data, or
/// generated from standard specifications.
///
/// # Example
///
/// ```rust,no_run
/// use vfx_icc::Profile;
/// use std::path::Path;
///
/// // Load from file
/// let profile = Profile::from_file(Path::new("camera.icc")).unwrap();
///
/// // Create standard profile
/// let srgb = Profile::srgb();
///
/// // Get profile info
/// println!("Description: {}", profile.description());
/// ```
pub struct Profile {
    /// Internal lcms2 profile handle.
    pub(crate) inner: LcmsProfile,
}

impl Profile {
    /// Loads a profile from an ICC file.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the .icc or .icm file
    ///
    /// # Errors
    ///
    /// Returns error if file cannot be read or contains invalid data.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use vfx_icc::Profile;
    /// use std::path::Path;
    ///
    /// let profile = Profile::from_file(Path::new("monitor.icc")).unwrap();
    /// ```
    pub fn from_file(path: &Path) -> IccResult<Self> {
        let inner = LcmsProfile::new_file(path)
            .map_err(|e| IccError::LoadFailed(format!("{}: {}", path.display(), e)))?;
        Ok(Self { inner })
    }

    /// Creates a profile from raw ICC data.
    ///
    /// # Arguments
    ///
    /// * `data` - Raw ICC profile bytes
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use vfx_icc::Profile;
    ///
    /// let icc_data = std::fs::read("profile.icc").unwrap();
    /// let profile = Profile::from_icc(&icc_data).unwrap();
    /// ```
    pub fn from_icc(data: &[u8]) -> IccResult<Self> {
        let inner = LcmsProfile::new_icc(data)
            .map_err(|e| IccError::InvalidProfile(e.to_string()))?;
        Ok(Self { inner })
    }

    /// Creates an sRGB profile.
    ///
    /// The standard IEC 61966-2-1 sRGB color space.
    ///
    /// # Example
    ///
    /// ```rust
    /// use vfx_icc::Profile;
    ///
    /// let srgb = Profile::srgb();
    /// ```
    pub fn srgb() -> Self {
        Self {
            inner: LcmsProfile::new_srgb(),
        }
    }

    /// Creates a profile from a standard specification.
    ///
    /// # Arguments
    ///
    /// * `standard` - The standard profile type
    ///
    /// # Example
    ///
    /// ```rust
    /// use vfx_icc::{Profile, StandardProfile};
    ///
    /// let p3 = Profile::from_standard(StandardProfile::DisplayP3);
    /// ```
    pub fn from_standard(standard: StandardProfile) -> Self {
        standard.to_profile()
    }

    /// Creates a linear sRGB profile (gamma 1.0).
    ///
    /// Uses sRGB primaries but with a linear transfer function.
    /// Suitable for compositing and rendering.
    pub fn linear_srgb() -> Self {
        Self::from_standard(StandardProfile::LinearSrgb)
    }

    /// Creates an Adobe RGB (1998) profile.
    pub fn adobe_rgb() -> Self {
        Self::from_standard(StandardProfile::AdobeRgb)
    }

    /// Creates a Display P3 profile.
    ///
    /// Wide-gamut profile used by Apple displays.
    pub fn display_p3() -> Self {
        Self::from_standard(StandardProfile::DisplayP3)
    }

    /// Creates a DCI-P3 profile.
    ///
    /// Digital Cinema Initiative P3 with DCI white point.
    pub fn dci_p3() -> Self {
        Self::from_standard(StandardProfile::DciP3)
    }

    /// Creates an ACES AP0 profile (linear).
    ///
    /// Academy Color Encoding System primary color space.
    /// Very wide gamut, suitable as a scene-referred working space.
    pub fn aces_ap0() -> Self {
        Self::from_standard(StandardProfile::AcesAp0)
    }

    /// Creates an ACES AP1 / ACEScg profile (linear).
    ///
    /// ACES computer graphics working space.
    /// Practical wide gamut for CGI and compositing.
    pub fn aces_ap1() -> Self {
        Self::from_standard(StandardProfile::AcesAp1)
    }

    /// Creates a Rec. 709 profile.
    ///
    /// ITU-R BT.709 HD television standard.
    pub fn rec709() -> Self {
        Self::from_standard(StandardProfile::Rec709)
    }

    /// Creates a Rec. 2020 profile.
    ///
    /// ITU-R BT.2020 UHD/HDR television standard.
    pub fn rec2020() -> Self {
        Self::from_standard(StandardProfile::Rec2020)
    }

    /// Creates a grayscale profile with the specified gamma.
    ///
    /// # Arguments
    ///
    /// * `gamma` - Transfer function gamma (e.g., 2.2)
    pub fn gray(gamma: f64) -> IccResult<Self> {
        let curve = lcms2::ToneCurve::new(gamma);
        let inner = LcmsProfile::new_gray(&lcms2::CIExyY::d50(), &curve)
            .map_err(|e| IccError::CreateFailed(e.to_string()))?;
        Ok(Self { inner })
    }

    /// Creates a CIE XYZ profile.
    ///
    /// The standard CIE 1931 XYZ color space (D50 adapted).
    pub fn xyz() -> Self {
        Self {
            inner: LcmsProfile::new_xyz(),
        }
    }

    /// Creates a CIE Lab profile.
    ///
    /// CIE L*a*b* perceptually uniform color space (D50).
    pub fn lab() -> IccResult<Self> {
        let inner = LcmsProfile::new_lab4_context(lcms2::GlobalContext::new(), &lcms2::CIExyY::d50())
            .map_err(|e| IccError::CreateFailed(e.to_string()))?;
        Ok(Self { inner })
    }

    /// Returns the profile description.
    ///
    /// # Example
    ///
    /// ```rust
    /// use vfx_icc::Profile;
    ///
    /// let srgb = Profile::srgb();
    /// println!("{}", srgb.description());
    /// ```
    pub fn description(&self) -> String {
        self.inner
            .info(lcms2::InfoType::Description, lcms2::Locale::none())
            .unwrap_or_default()
    }

    /// Returns the profile manufacturer.
    pub fn manufacturer(&self) -> String {
        self.inner
            .info(lcms2::InfoType::Manufacturer, lcms2::Locale::none())
            .unwrap_or_default()
    }

    /// Returns the profile model.
    pub fn model(&self) -> String {
        self.inner
            .info(lcms2::InfoType::Model, lcms2::Locale::none())
            .unwrap_or_default()
    }

    /// Returns the profile copyright.
    pub fn copyright(&self) -> String {
        self.inner
            .info(lcms2::InfoType::Copyright, lcms2::Locale::none())
            .unwrap_or_default()
    }

    /// Returns the color space signature.
    pub fn color_space(&self) -> String {
        format!("{:?}", self.inner.color_space())
    }

    /// Returns true if this is an RGB profile.
    pub fn is_rgb(&self) -> bool {
        matches!(self.inner.color_space(), ColorSpaceSignature::RgbData)
    }

    /// Returns true if this is a CMYK profile.
    pub fn is_cmyk(&self) -> bool {
        matches!(self.inner.color_space(), ColorSpaceSignature::CmykData)
    }

    /// Returns true if this is a grayscale profile.
    pub fn is_gray(&self) -> bool {
        matches!(self.inner.color_space(), ColorSpaceSignature::GrayData)
    }

    /// Exports the profile as ICC data.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use vfx_icc::Profile;
    /// use std::fs;
    ///
    /// let profile = Profile::srgb();
    /// let data = profile.to_icc().unwrap();
    /// fs::write("srgb.icc", data).unwrap();
    /// ```
    pub fn to_icc(&self) -> IccResult<Vec<u8>> {
        self.inner
            .icc()
            .map_err(|e| IccError::CreateFailed(e.to_string()))
    }
}

impl std::fmt::Debug for Profile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Profile")
            .field("description", &self.description())
            .field("color_space", &self.color_space())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_srgb() {
        let profile = Profile::srgb();
        assert!(profile.is_rgb());
        assert!(!profile.description().is_empty());
    }

    #[test]
    fn test_xyz() {
        let profile = Profile::xyz();
        assert!(!profile.is_rgb());
    }

    #[test]
    fn test_lab() {
        let profile = Profile::lab().unwrap();
        assert!(!profile.is_rgb());
    }

    #[test]
    fn test_gray() {
        let profile = Profile::gray(2.2).unwrap();
        assert!(profile.is_gray());
    }

    #[test]
    fn test_to_icc() {
        let profile = Profile::srgb();
        let data = profile.to_icc().unwrap();
        assert!(!data.is_empty());

        // Round-trip
        let reloaded = Profile::from_icc(&data).unwrap();
        assert!(reloaded.is_rgb());
    }
}
