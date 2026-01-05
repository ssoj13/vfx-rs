//! Multi-layer image support for Python.
//!
//! Exposes LayeredImage, ImageLayer, and ImageChannel for EXR-style workflows.

use pyo3::prelude::*;
use pyo3::exceptions::{PyValueError, PyIndexError};
use numpy::{PyArray1, ToPyArray};
use vfx_io::{
    LayeredImage as RustLayeredImage,
    ImageLayer as RustImageLayer,
    ImageChannel as RustImageChannel,
    ChannelKind as RustChannelKind,
    ChannelSampleType as RustChannelSampleType,

};

use crate::Image;

/// Channel semantic type.
#[pyclass(eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelKind {
    /// Color data (RGB/YCbCr/etc).
    Color = 0,
    /// Alpha/opacity.
    Alpha = 1,
    /// Depth/Z data.
    Depth = 2,
    /// Object or material ID.
    Id = 3,
    /// Matte/mask data.
    Mask = 4,
    /// Unknown or generic.
    Generic = 5,
}

impl From<RustChannelKind> for ChannelKind {
    fn from(kind: RustChannelKind) -> Self {
        match kind {
            RustChannelKind::Color => Self::Color,
            RustChannelKind::Alpha => Self::Alpha,
            RustChannelKind::Depth => Self::Depth,
            RustChannelKind::Id => Self::Id,
            RustChannelKind::Mask => Self::Mask,
            RustChannelKind::Generic => Self::Generic,
        }
    }
}

impl From<ChannelKind> for RustChannelKind {
    fn from(kind: ChannelKind) -> Self {
        match kind {
            ChannelKind::Color => Self::Color,
            ChannelKind::Alpha => Self::Alpha,
            ChannelKind::Depth => Self::Depth,
            ChannelKind::Id => Self::Id,
            ChannelKind::Mask => Self::Mask,
            ChannelKind::Generic => Self::Generic,
        }
    }
}

#[pymethods]
impl ChannelKind {
    fn __repr__(&self) -> String {
        format!("ChannelKind.{:?}", self)
    }
}

/// Channel sample type.
#[pyclass(eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SampleType {
    /// 16-bit half float.
    F16 = 0,
    /// 32-bit float.
    F32 = 1,
    /// 32-bit unsigned int.
    U32 = 2,
}

impl From<RustChannelSampleType> for SampleType {
    fn from(st: RustChannelSampleType) -> Self {
        match st {
            RustChannelSampleType::F16 => Self::F16,
            RustChannelSampleType::F32 => Self::F32,
            RustChannelSampleType::U32 => Self::U32,
        }
    }
}

impl From<SampleType> for RustChannelSampleType {
    fn from(st: SampleType) -> Self {
        match st {
            SampleType::F16 => Self::F16,
            SampleType::F32 => Self::F32,
            SampleType::U32 => Self::U32,
        }
    }
}

#[pymethods]
impl SampleType {
    fn __repr__(&self) -> String {
        format!("SampleType.{:?}", self)
    }
}

/// A single image channel with planar data.
///
/// Channels store pixel data in planar format (one value per pixel).
#[pyclass]
#[derive(Clone)]
pub struct ImageChannel {
    pub(crate) inner: RustImageChannel,
}

#[pymethods]
impl ImageChannel {
    /// Channel name (e.g., "R", "G", "B", "A", "Z").
    #[getter]
    fn name(&self) -> &str {
        &self.inner.name
    }
    
    /// Semantic kind of this channel.
    #[getter]
    fn kind(&self) -> ChannelKind {
        self.inner.kind.into()
    }
    
    /// Sample type for serialization.
    #[getter]
    fn sample_type(&self) -> SampleType {
        self.inner.sample_type.into()
    }
    
    /// Number of samples in this channel.
    #[getter]
    fn len(&self) -> usize {
        self.inner.samples.len()
    }
    
    /// Whether channel is empty.
    fn is_empty(&self) -> bool {
        self.inner.samples.is_empty()
    }
    
    /// Get samples as numpy array (f32).
    fn numpy<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<f32>> {
        let data = self.inner.samples.to_f32();
        data.to_pyarray(py)
    }
    
    fn __repr__(&self) -> String {
        format!(
            "ImageChannel('{}', kind={:?}, samples={})",
            self.inner.name, self.inner.kind, self.inner.samples.len()
        )
    }
}

/// A named image layer with multiple channels.
///
/// Layers group related channels (e.g., RGB, depth, ID).
#[pyclass]
#[derive(Clone)]
pub struct ImageLayer {
    pub(crate) inner: RustImageLayer,
}

#[pymethods]
impl ImageLayer {
    /// Layer name (e.g., "beauty", "diffuse", "depth").
    #[getter]
    fn name(&self) -> &str {
        &self.inner.name
    }
    
    /// Layer width in pixels.
    #[getter]
    fn width(&self) -> u32 {
        self.inner.width
    }
    
    /// Layer height in pixels.
    #[getter]
    fn height(&self) -> u32 {
        self.inner.height
    }
    
    /// Number of channels.
    #[getter]
    fn num_channels(&self) -> usize {
        self.inner.channels.len()
    }
    
    /// List of channel names.
    #[getter]
    fn channel_names(&self) -> Vec<String> {
        self.inner.channels.iter().map(|c| c.name.clone()).collect()
    }
    
    /// Get channel by name.
    fn channel(&self, name: &str) -> PyResult<ImageChannel> {
        self.inner.channels
            .iter()
            .find(|c| c.name == name)
            .map(|c| ImageChannel { inner: c.clone() })
            .ok_or_else(|| PyValueError::new_err(format!("Channel '{}' not found", name)))
    }
    
    /// Get channel by index.
    fn channel_at(&self, index: usize) -> PyResult<ImageChannel> {
        self.inner.channels
            .get(index)
            .map(|c| ImageChannel { inner: c.clone() })
            .ok_or_else(|| PyIndexError::new_err(format!("Index {} out of range", index)))
    }
    
    /// Convert to flat Image (interleaved RGBA).
    fn to_image(&self) -> PyResult<Image> {
        let data = self.inner.to_image_data()
            .map_err(|e| PyValueError::new_err(format!("Conversion failed: {}", e)))?;
        Ok(Image::from_image_data(data))
    }
    
    fn __repr__(&self) -> String {
        format!(
            "ImageLayer('{}', {}x{}, channels=[{}])",
            self.inner.name, self.inner.width, self.inner.height,
            self.channel_names().join(", ")
        )
    }
    
    fn __len__(&self) -> usize {
        self.inner.channels.len()
    }
    
    fn __getitem__(&self, key: &Bound<'_, PyAny>) -> PyResult<ImageChannel> {
        if let Ok(idx) = key.extract::<usize>() {
            self.channel_at(idx)
        } else if let Ok(name) = key.extract::<String>() {
            self.channel(&name)
        } else {
            Err(PyValueError::new_err("Key must be int or str"))
        }
    }
}

/// Multi-layer image container.
///
/// Supports EXR-style workflows with multiple named layers,
/// each containing arbitrary channels.
///
/// # Example
/// ```python
/// import vfx_rs
///
/// # Read multi-layer EXR
/// layered = vfx_rs.read_layered("render.exr")
///
/// # Access layers
/// beauty = layered["beauty"]
/// depth = layered["depth"]
///
/// # Convert layer to flat image
/// img = beauty.to_image()
/// ```
#[pyclass]
#[derive(Clone)]
pub struct LayeredImage {
    pub(crate) inner: RustLayeredImage,
}

#[pymethods]
impl LayeredImage {
    /// Create empty layered image.
    #[new]
    fn new() -> Self {
        Self { inner: RustLayeredImage::default() }
    }
    
    /// Number of layers.
    #[getter]
    fn num_layers(&self) -> usize {
        self.inner.layers.len()
    }
    
    /// List of layer names.
    #[getter]
    fn layer_names(&self) -> Vec<String> {
        self.inner.layers.iter().map(|l| l.name.clone()).collect()
    }
    
    /// Get layer by name.
    fn layer(&self, name: &str) -> PyResult<ImageLayer> {
        self.inner.layers
            .iter()
            .find(|l| l.name == name)
            .map(|l| ImageLayer { inner: l.clone() })
            .ok_or_else(|| PyValueError::new_err(format!("Layer '{}' not found", name)))
    }
    
    /// Get layer by index.
    fn layer_at(&self, index: usize) -> PyResult<ImageLayer> {
        self.inner.layers
            .get(index)
            .map(|l| ImageLayer { inner: l.clone() })
            .ok_or_else(|| PyIndexError::new_err(format!("Index {} out of range", index)))
    }
    
    /// Add a layer from an Image.
    ///
    /// Converts interleaved Image to planar layer.
    fn add_layer(&mut self, name: &str, image: &Image) {
        let layer = image.as_image_data().to_layer(name);
        self.inner.layers.push(layer);
    }
    
    /// Convert to flat Image (first layer only).
    ///
    /// Raises error if multiple layers exist.
    fn to_image(&self) -> PyResult<Image> {
        let data = self.inner.to_image_data()
            .map_err(|e| PyValueError::new_err(format!("Conversion failed: {}", e)))?;
        Ok(Image::from_image_data(data))
    }
    
    /// Create LayeredImage from a flat Image.
    #[staticmethod]
    fn from_image(image: &Image, layer_name: &str) -> Self {
        Self { inner: image.as_image_data().to_layered(layer_name) }
    }
    
    fn __repr__(&self) -> String {
        let layers: Vec<_> = self.inner.layers.iter()
            .map(|l| format!("'{}'", l.name))
            .collect();
        format!("LayeredImage(layers=[{}])", layers.join(", "))
    }
    
    fn __len__(&self) -> usize {
        self.inner.layers.len()
    }
    
    fn __getitem__(&self, key: &Bound<'_, PyAny>) -> PyResult<ImageLayer> {
        if let Ok(idx) = key.extract::<usize>() {
            self.layer_at(idx)
        } else if let Ok(name) = key.extract::<String>() {
            self.layer(&name)
        } else {
            Err(PyValueError::new_err("Key must be int or str"))
        }
    }
    
    fn __iter__(slf: PyRef<'_, Self>) -> LayeredImageIter {
        LayeredImageIter {
            inner: slf.inner.clone(),
            index: 0,
        }
    }
}

/// Iterator over layers.
#[pyclass]
pub struct LayeredImageIter {
    inner: RustLayeredImage,
    index: usize,
}

#[pymethods]
impl LayeredImageIter {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }
    
    fn __next__(&mut self) -> Option<ImageLayer> {
        if self.index < self.inner.layers.len() {
            let layer = ImageLayer { inner: self.inner.layers[self.index].clone() };
            self.index += 1;
            Some(layer)
        } else {
            None
        }
    }
}

/// Register layered image types.
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<ChannelKind>()?;
    m.add_class::<SampleType>()?;
    m.add_class::<ImageChannel>()?;
    m.add_class::<ImageLayer>()?;
    m.add_class::<LayeredImage>()?;
    Ok(())
}
