//! Image type with numpy interop.

use pyo3::prelude::*;
use pyo3::exceptions::{PyValueError, PyIOError};
use numpy::{PyArray3, PyArrayMethods, PyUntypedArrayMethods, ToPyArray};
use vfx_io::{ImageData, PixelFormat};
use vfx_io::imagebuf::ImageBuf;
use vfx_io::imagebufalgo;
use vfx_io::imagebufalgo::geometry::ResizeFilter as RustResizeFilter;
use vfx_io::imagebufalgo::demosaic::{BayerPattern as RustBayerPattern, DemosaicAlgorithm as RustDemosaicAlgorithm};
use vfx_io::imagebufalgo::texture::MipmapOptions as RustMipmapOptions;
use vfx_io::imagebufalgo::fillholes::FillHolesOptions as RustFillHolesOptions;
use vfx_core::Roi3D as RustRoi3D;

use crate::core::Roi3D;
use crate::ops::{ResizeFilter, BayerPattern, DemosaicAlgorithm, MipmapOptions, FillHolesOptions};

/// An image buffer with width, height, and channels.
///
/// Provides easy interop with numpy arrays.
///
/// **Note:** Data is copied when converting between Image and numpy.
/// True zero-copy interop requires careful lifetime management that
/// is not yet implemented.
///
/// # Example
/// ```python
/// import numpy as np
/// import vfx_rs
///
/// # From numpy (copies data)
/// arr = np.zeros((1080, 1920, 4), dtype=np.float32)
/// img = vfx_rs.Image(arr)
///
/// # To numpy (copies data)
/// arr = img.numpy()
/// ```
#[pyclass]
#[derive(Clone)]
pub struct Image {
    pub(crate) inner: ImageData,
}

#[pymethods]
impl Image {
    /// Create an image from a numpy array.
    ///
    /// Array shape must be (height, width, channels).
    /// Only float32 dtype is currently supported.
    #[new]
    #[pyo3(signature = (array, copy=false))]
    fn new(array: &Bound<'_, PyArray3<f32>>, copy: bool) -> PyResult<Self> {
        let shape = array.shape();
        let height = shape[0] as u32;
        let width = shape[1] as u32;
        let channels = shape[2] as u32;
        
        // Get data as Vec<f32>
        let data: Vec<f32> = if copy || !array.is_contiguous() {
            // Need to copy
            array.to_vec()?
        } else {
            // Can read directly (still copies for now, true zero-copy needs unsafe)
            array.to_vec()?
        };
        
        let inner = ImageData::from_f32(width, height, channels, data);
        Ok(Self { inner })
    }
    
    /// Image width in pixels.
    #[getter]
    fn width(&self) -> u32 {
        self.inner.width
    }
    
    /// Image height in pixels.
    #[getter]
    fn height(&self) -> u32 {
        self.inner.height
    }
    
    /// Number of channels (1=gray, 3=RGB, 4=RGBA).
    #[getter]
    fn channels(&self) -> u32 {
        self.inner.channels
    }
    
    /// Pixel format as string: 'u8', 'u16', 'f16', 'f32'.
    #[getter]
    fn format(&self) -> &'static str {
        match self.inner.format {
            PixelFormat::U8 => "u8",
            PixelFormat::U16 => "u16",
            PixelFormat::U32 => "u32",
            PixelFormat::F16 => "f16",
            PixelFormat::F32 => "f32",
        }
    }
    
    /// Returns image data as numpy array.
    ///
    /// Shape: (height, width, channels)
    /// Dtype: float32
    ///
    /// # Arguments
    /// * `copy` - Force a copy (default: False)
    #[pyo3(signature = (copy=false))]
    fn numpy<'py>(&self, py: Python<'py>, copy: bool) -> PyResult<Bound<'py, PyArray3<f32>>> {
        let data = self.inner.to_f32();
        let h = self.inner.height as usize;
        let w = self.inner.width as usize;
        let c = self.inner.channels as usize;
        
        // Create array and reshape
        let arr = data.to_pyarray(py);
        let reshaped = arr.reshape([h, w, c])
            .map_err(|e| PyValueError::new_err(format!("Reshape failed: {}", e)))?;
        
        if copy {
            // Force copy
            reshaped.to_owned_array().to_pyarray(py).reshape([h, w, c])
                .map_err(|e| PyValueError::new_err(format!("Copy failed: {}", e)))
        } else {
            Ok(reshaped)
        }
    }
    
    /// Create an empty image with given dimensions.
    #[staticmethod]
    #[pyo3(signature = (width, height, channels=4))]
    fn empty(width: u32, height: u32, channels: u32) -> Self {
        let size = (width * height * channels) as usize;
        let data = vec![0.0f32; size];
        Self {
            inner: ImageData::from_f32(width, height, channels, data),
        }
    }
    
    /// Create a copy of this image.
    fn copy(&self) -> Self {
        self.clone()
    }
    
    fn __repr__(&self) -> String {
        format!(
            "Image({}x{}, {} channels, {})",
            self.inner.width,
            self.inner.height,
            self.inner.channels,
            self.format()
        )
    }

    // ========================================================================
    // Geometry Operations
    // ========================================================================

    /// Flip image vertically (top to bottom).
    ///
    /// Returns:
    ///     New flipped image
    #[pyo3(signature = (roi=None))]
    fn flip(&self, roi: Option<&Roi3D>) -> PyResult<Self> {
        let buf = self.to_imagebuf();
        let result = imagebufalgo::flip(&buf, Self::convert_roi(roi));
        Self::from_imagebuf(&result)
    }

    /// Flip image horizontally (left to right).
    ///
    /// Returns:
    ///     New flopped image
    #[pyo3(signature = (roi=None))]
    fn flop(&self, roi: Option<&Roi3D>) -> PyResult<Self> {
        let buf = self.to_imagebuf();
        let result = imagebufalgo::flop(&buf, Self::convert_roi(roi));
        Self::from_imagebuf(&result)
    }

    /// Transpose image (swap x and y axes).
    #[pyo3(signature = (roi=None))]
    fn transpose(&self, roi: Option<&Roi3D>) -> PyResult<Self> {
        let buf = self.to_imagebuf();
        let result = imagebufalgo::transpose(&buf, Self::convert_roi(roi));
        Self::from_imagebuf(&result)
    }

    /// Rotate image 90 degrees clockwise.
    #[pyo3(signature = (roi=None))]
    fn rotate90(&self, roi: Option<&Roi3D>) -> PyResult<Self> {
        let buf = self.to_imagebuf();
        let result = imagebufalgo::rotate90(&buf, Self::convert_roi(roi));
        Self::from_imagebuf(&result)
    }

    /// Rotate image 180 degrees.
    #[pyo3(signature = (roi=None))]
    fn rotate180(&self, roi: Option<&Roi3D>) -> PyResult<Self> {
        let buf = self.to_imagebuf();
        let result = imagebufalgo::rotate180(&buf, Self::convert_roi(roi));
        Self::from_imagebuf(&result)
    }

    /// Rotate image 270 degrees clockwise (90 counter-clockwise).
    #[pyo3(signature = (roi=None))]
    fn rotate270(&self, roi: Option<&Roi3D>) -> PyResult<Self> {
        let buf = self.to_imagebuf();
        let result = imagebufalgo::rotate270(&buf, Self::convert_roi(roi));
        Self::from_imagebuf(&result)
    }

    /// Crop image to specified region.
    ///
    /// Args:
    ///     roi: Region to crop to
    ///
    /// Returns:
    ///     Cropped image
    fn crop(&self, roi: &Roi3D) -> PyResult<Self> {
        let buf = self.to_imagebuf();
        let rust_roi = RustRoi3D {
            xbegin: roi.xbegin,
            xend: roi.xend,
            ybegin: roi.ybegin,
            yend: roi.yend,
            zbegin: roi.zbegin,
            zend: roi.zend,
            chbegin: roi.chbegin,
            chend: roi.chend,
        };
        let result = imagebufalgo::crop(&buf, Some(rust_roi));
        Self::from_imagebuf(&result)
    }

    /// Resize image to new dimensions.
    ///
    /// Args:
    ///     width: New width
    ///     height: New height
    ///     filter: Resize filter (default: Bilinear)
    ///
    /// Returns:
    ///     Resized image
    #[pyo3(signature = (width, height, filter=None))]
    fn resize(&self, width: u32, height: u32, filter: Option<ResizeFilter>) -> PyResult<Self> {
        let buf = self.to_imagebuf();
        let rust_filter = filter.map(|f| f.into()).unwrap_or(RustResizeFilter::Bilinear);
        let result = imagebufalgo::resize(&buf, width, height, rust_filter, None);
        Self::from_imagebuf(&result)
    }

    // ========================================================================
    // Filter Operations
    // ========================================================================

    /// Apply Gaussian blur.
    ///
    /// Args:
    ///     sigma: Blur sigma (standard deviation, default 1.0)
    ///
    /// Returns:
    ///     Blurred image
    #[pyo3(signature = (sigma=1.0, roi=None))]
    fn blur(&self, sigma: f32, roi: Option<&Roi3D>) -> PyResult<Self> {
        let buf = self.to_imagebuf();
        let result = imagebufalgo::blur(&buf, sigma, Self::convert_roi(roi));
        Self::from_imagebuf(&result)
    }

    /// Apply sharpening filter.
    ///
    /// Args:
    ///     amount: Sharpening strength (default 1.0)
    ///
    /// Returns:
    ///     Sharpened image
    #[pyo3(signature = (amount=1.0, roi=None))]
    fn sharpen(&self, amount: f32, roi: Option<&Roi3D>) -> PyResult<Self> {
        let buf = self.to_imagebuf();
        let result = imagebufalgo::sharpen(&buf, amount, Self::convert_roi(roi));
        Self::from_imagebuf(&result)
    }

    /// Apply unsharp mask sharpening.
    ///
    /// Args:
    ///     sigma: Blur sigma for mask (default 1.0)
    ///     amount: Sharpening strength (default 1.0)
    ///     threshold: Edge threshold (default 0.0)
    ///
    /// Returns:
    ///     Sharpened image
    #[pyo3(signature = (sigma=1.0, amount=1.0, threshold=0.0, roi=None))]
    fn unsharp_mask(&self, sigma: f32, amount: f32, threshold: f32, roi: Option<&Roi3D>) -> PyResult<Self> {
        let buf = self.to_imagebuf();
        let result = imagebufalgo::unsharp_mask(&buf, sigma, amount, threshold, Self::convert_roi(roi));
        Self::from_imagebuf(&result)
    }

    /// Apply median filter.
    ///
    /// Args:
    ///     size: Filter size (default 3)
    ///
    /// Returns:
    ///     Filtered image
    #[pyo3(signature = (size=3, roi=None))]
    fn median(&self, size: u32, roi: Option<&Roi3D>) -> PyResult<Self> {
        let buf = self.to_imagebuf();
        let result = imagebufalgo::median(&buf, size, Self::convert_roi(roi));
        Self::from_imagebuf(&result)
    }

    /// Apply box blur (average filter).
    ///
    /// Args:
    ///     size: Filter size (default 3)
    ///
    /// Returns:
    ///     Blurred image
    #[pyo3(signature = (size=3, roi=None))]
    fn box_blur(&self, size: u32, roi: Option<&Roi3D>) -> PyResult<Self> {
        let buf = self.to_imagebuf();
        let result = imagebufalgo::box_blur(&buf, size, Self::convert_roi(roi));
        Self::from_imagebuf(&result)
    }

    /// Dilate (grow) bright regions.
    #[pyo3(signature = (size=3, roi=None))]
    fn dilate(&self, size: u32, roi: Option<&Roi3D>) -> PyResult<Self> {
        let buf = self.to_imagebuf();
        let result = imagebufalgo::dilate(&buf, size, Self::convert_roi(roi));
        Self::from_imagebuf(&result)
    }

    /// Erode (shrink) bright regions.
    #[pyo3(signature = (size=3, roi=None))]
    fn erode(&self, size: u32, roi: Option<&Roi3D>) -> PyResult<Self> {
        let buf = self.to_imagebuf();
        let result = imagebufalgo::erode(&buf, size, Self::convert_roi(roi));
        Self::from_imagebuf(&result)
    }

    /// Apply Laplacian edge detection.
    #[pyo3(signature = (roi=None))]
    fn laplacian(&self, roi: Option<&Roi3D>) -> PyResult<Self> {
        let buf = self.to_imagebuf();
        let result = imagebufalgo::laplacian(&buf, Self::convert_roi(roi));
        Self::from_imagebuf(&result)
    }

    /// Apply Sobel edge detection.
    #[pyo3(signature = (roi=None))]
    fn sobel(&self, roi: Option<&Roi3D>) -> PyResult<Self> {
        let buf = self.to_imagebuf();
        let result = imagebufalgo::sobel(&buf, Self::convert_roi(roi));
        Self::from_imagebuf(&result)
    }

    // ========================================================================
    // Arithmetic Operations
    // ========================================================================

    /// Clamp pixel values to a range.
    ///
    /// Args:
    ///     min_val: Minimum value (default 0.0)
    ///     max_val: Maximum value (default 1.0)
    ///
    /// Returns:
    ///     Clamped image
    #[pyo3(signature = (min_val=0.0, max_val=1.0, roi=None))]
    fn clamp(&self, min_val: f32, max_val: f32, roi: Option<&Roi3D>) -> PyResult<Self> {
        let buf = self.to_imagebuf();
        let result = imagebufalgo::clamp(&buf, &[min_val], &[max_val], Self::convert_roi(roi));
        Self::from_imagebuf(&result)
    }

    /// Invert pixel values: 1 - value.
    #[pyo3(signature = (roi=None))]
    fn invert(&self, roi: Option<&Roi3D>) -> PyResult<Self> {
        let buf = self.to_imagebuf();
        let result = imagebufalgo::invert(&buf, Self::convert_roi(roi));
        Self::from_imagebuf(&result)
    }

    /// Compute absolute value of each pixel.
    #[pyo3(signature = (roi=None))]
    fn abs(&self, roi: Option<&Roi3D>) -> PyResult<Self> {
        let buf = self.to_imagebuf();
        let result = imagebufalgo::abs(&buf, Self::convert_roi(roi));
        Self::from_imagebuf(&result)
    }

    /// Raise pixel values to a power.
    ///
    /// Args:
    ///     exponent: Power value
    #[pyo3(signature = (exponent, roi=None))]
    fn pow(&self, exponent: f32, roi: Option<&Roi3D>) -> PyResult<Self> {
        let buf = self.to_imagebuf();
        let result = imagebufalgo::pow(&buf, &[exponent], Self::convert_roi(roi));
        Self::from_imagebuf(&result)
    }

    // ========================================================================
    // Color Operations
    // ========================================================================

    /// Premultiply RGB by alpha.
    #[pyo3(signature = (roi=None))]
    fn premult(&self, roi: Option<&Roi3D>) -> PyResult<Self> {
        let buf = self.to_imagebuf();
        let result = imagebufalgo::premult(&buf, Self::convert_roi(roi));
        Self::from_imagebuf(&result)
    }

    /// Unpremultiply (divide RGB by alpha).
    #[pyo3(signature = (roi=None))]
    fn unpremult(&self, roi: Option<&Roi3D>) -> PyResult<Self> {
        let buf = self.to_imagebuf();
        let result = imagebufalgo::unpremult(&buf, Self::convert_roi(roi));
        Self::from_imagebuf(&result)
    }

    /// Adjust color saturation.
    ///
    /// Args:
    ///     scale: Saturation scale (0=grayscale, 1=unchanged, >1=more saturated)
    #[pyo3(signature = (scale, roi=None))]
    fn saturate(&self, scale: f32, roi: Option<&Roi3D>) -> PyResult<Self> {
        let buf = self.to_imagebuf();
        let result = imagebufalgo::saturate(&buf, scale, 0, Self::convert_roi(roi));
        Self::from_imagebuf(&result)
    }

    /// Adjust contrast.
    ///
    /// Args:
    ///     black: Input black point (default 0.0)
    ///     white: Input white point (default 1.0)
    ///     min_val: Output minimum (default 0.0)
    ///     max_val: Output maximum (default 1.0)
    #[pyo3(signature = (black=0.0, white=1.0, min_val=0.0, max_val=1.0, roi=None))]
    fn contrast(&self, black: f32, white: f32, min_val: f32, max_val: f32, roi: Option<&Roi3D>) -> PyResult<Self> {
        let buf = self.to_imagebuf();
        let result = imagebufalgo::contrast_remap(&buf, black, white, min_val, max_val, 1.0, 0.5, Self::convert_roi(roi));
        Self::from_imagebuf(&result)
    }

    /// Convert sRGB to linear RGB.
    #[pyo3(signature = (roi=None))]
    fn srgb_to_linear(&self, roi: Option<&Roi3D>) -> PyResult<Self> {
        let buf = self.to_imagebuf();
        let result = imagebufalgo::srgb_to_linear(&buf, Self::convert_roi(roi));
        Self::from_imagebuf(&result)
    }

    /// Convert linear RGB to sRGB.
    #[pyo3(signature = (roi=None))]
    fn linear_to_srgb(&self, roi: Option<&Roi3D>) -> PyResult<Self> {
        let buf = self.to_imagebuf();
        let result = imagebufalgo::linear_to_srgb(&buf, Self::convert_roi(roi));
        Self::from_imagebuf(&result)
    }

    /// Convert between color spaces using OCIO.
    ///
    /// Args:
    ///     from_space: Source color space (e.g., "ACEScg")
    ///     to_space: Target color space (e.g., "sRGB")
    ///
    /// Returns:
    ///     Color-converted image
    #[pyo3(signature = (from_space, to_space))]
    fn colorconvert(&self, from_space: &str, to_space: &str) -> PyResult<Self> {
        let buf = self.to_imagebuf();
        let result = imagebufalgo::ocio::colorconvert(&buf, from_space, to_space, None, None);
        Self::from_imagebuf(&result)
    }

    // ========================================================================
    // Channel Operations
    // ========================================================================

    /// Extract a single channel.
    ///
    /// Args:
    ///     channel: Channel index (0=R, 1=G, 2=B, 3=A)
    ///
    /// Returns:
    ///     Single-channel image
    #[pyo3(signature = (channel, roi=None))]
    fn extract_channel(&self, channel: usize, roi: Option<&Roi3D>) -> PyResult<Self> {
        let buf = self.to_imagebuf();
        let result = imagebufalgo::extract_channel(&buf, channel, Self::convert_roi(roi));
        Self::from_imagebuf(&result)
    }

    /// Append channels from another image.
    ///
    /// Args:
    ///     other: Image with channels to append
    ///
    /// Returns:
    ///     Image with combined channels
    #[pyo3(signature = (other, roi=None))]
    fn channel_append(&self, other: &Image, roi: Option<&Roi3D>) -> PyResult<Self> {
        let buf_a = self.to_imagebuf();
        let buf_b = other.to_imagebuf();
        let result = imagebufalgo::channel_append(&buf_a, &buf_b, Self::convert_roi(roi));
        Self::from_imagebuf(&result)
    }

    // ========================================================================
    // Demosaic Operations
    // ========================================================================

    /// Demosaic (debayer) a raw Bayer pattern image to RGB.
    ///
    /// Args:
    ///     pattern: Bayer pattern (RGGB, BGGR, GRBG, GBRG)
    ///     algorithm: Demosaic algorithm (Bilinear, VNG)
    ///
    /// Returns:
    ///     RGB image
    #[pyo3(signature = (pattern=None, algorithm=None))]
    fn demosaic(&self, pattern: Option<BayerPattern>, algorithm: Option<DemosaicAlgorithm>) -> PyResult<Self> {
        let buf = self.to_imagebuf();
        let p: RustBayerPattern = pattern.map(|p| p.into()).unwrap_or(RustBayerPattern::RGGB);
        let a: RustDemosaicAlgorithm = algorithm.map(|a| a.into()).unwrap_or(RustDemosaicAlgorithm::Bilinear);
        let result = imagebufalgo::demosaic(&buf, p, a);
        Self::from_imagebuf(&result)
    }

    // ========================================================================
    // Mipmap/Texture Operations
    // ========================================================================

    /// Generate all mipmap levels.
    ///
    /// Args:
    ///     options: Mipmap generation options
    ///
    /// Returns:
    ///     List of images, one per mip level (including original)
    #[pyo3(signature = (options=None))]
    fn make_texture(&self, options: Option<&MipmapOptions>) -> PyResult<Vec<Self>> {
        let buf = self.to_imagebuf();
        let opts: RustMipmapOptions = options.map(|o| o.into()).unwrap_or_default();
        let mips = imagebufalgo::make_texture(&buf, &opts);
        mips.iter().map(Self::from_imagebuf).collect()
    }

    /// Generate a specific mip level.
    ///
    /// Args:
    ///     level: Mip level (0 = original, 1 = half, etc.)
    ///     options: Mipmap generation options
    ///
    /// Returns:
    ///     Image at specified mip level
    #[pyo3(signature = (level, options=None))]
    fn make_mip_level(&self, level: u32, options: Option<&MipmapOptions>) -> PyResult<Self> {
        let buf = self.to_imagebuf();
        let opts: RustMipmapOptions = options.map(|o| o.into()).unwrap_or_default();
        let result = imagebufalgo::make_mip_level(&buf, level, &opts);
        Self::from_imagebuf(&result)
    }

    // ========================================================================
    // Hole Filling Operations
    // ========================================================================

    /// Fill holes (zero-alpha pixels) using push-pull algorithm.
    ///
    /// Args:
    ///     options: Hole filling options
    ///
    /// Returns:
    ///     Image with holes filled
    #[pyo3(signature = (options=None))]
    fn fillholes(&self, options: Option<&FillHolesOptions>) -> PyResult<Self> {
        let buf = self.to_imagebuf();
        let opts: RustFillHolesOptions = options.map(|o| o.into()).unwrap_or_default();
        let result = imagebufalgo::fillholes_pushpull(&buf, &opts);
        Self::from_imagebuf(&result)
    }

    /// Check if image has any holes (zero-alpha pixels).
    ///
    /// Returns:
    ///     True if image has holes
    #[pyo3(signature = (options=None))]
    fn has_holes(&self, options: Option<&FillHolesOptions>) -> bool {
        let buf = self.to_imagebuf();
        let opts: RustFillHolesOptions = options.map(|o| o.into()).unwrap_or_default();
        imagebufalgo::has_holes(&buf, &opts)
    }

    /// Count holes (zero-alpha pixels) in image.
    ///
    /// Returns:
    ///     Number of hole pixels
    #[pyo3(signature = (options=None))]
    fn count_holes(&self, options: Option<&FillHolesOptions>) -> usize {
        let buf = self.to_imagebuf();
        let opts: RustFillHolesOptions = options.map(|o| o.into()).unwrap_or_default();
        imagebufalgo::count_holes(&buf, &opts)
    }

}

impl Image {
    /// Create from vfx_io::ImageData
    pub fn from_image_data(data: ImageData) -> Self {
        Self { inner: data }
    }

    /// Get reference to inner ImageData
    pub fn as_image_data(&self) -> &ImageData {
        &self.inner
    }

    /// Get mutable reference to inner ImageData
    pub fn as_image_data_mut(&mut self) -> &mut ImageData {
        &mut self.inner
    }

    // Helper to convert to ImageBuf
    fn to_imagebuf(&self) -> ImageBuf {
        ImageBuf::from_image_data(&self.inner)
    }

    // Helper to convert from ImageBuf
    fn from_imagebuf(buf: &ImageBuf) -> PyResult<Self> {
        let data = buf.to_image_data()
            .map_err(|e| PyIOError::new_err(format!("Conversion failed: {}", e)))?;
        Ok(Self { inner: data })
    }

    // Helper to convert optional Python Roi3D to Rust
    fn convert_roi(roi: Option<&Roi3D>) -> Option<RustRoi3D> {
        roi.map(|r| RustRoi3D {
            xbegin: r.xbegin,
            xend: r.xend,
            ybegin: r.ybegin,
            yend: r.yend,
            zbegin: r.zbegin,
            zend: r.zend,
            chbegin: r.chbegin,
            chend: r.chend,
        })
    }
}
