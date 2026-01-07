//! Drawing operations for Python.
//!
//! Provides basic drawing primitives for images.

use pyo3::prelude::*;
use pyo3::exceptions::PyIOError;

use vfx_io::imagebuf::ImageBuf;
use vfx_io::imagebufalgo::drawing as rust_drawing;
#[cfg(feature = "text")]
use vfx_io::imagebufalgo::text as rust_text;
#[cfg(feature = "text")]
use vfx_io::imagebufalgo::text::{TextAlign as RustTextAlign, TextStyle as RustTextStyle};
use vfx_core::Roi3D as RustRoi3D;

use crate::Image;
use crate::core::Roi3D;

// ============================================================================
// Helper Functions
// ============================================================================

fn image_to_imagebuf(img: &Image) -> ImageBuf {
    ImageBuf::from_image_data(img.as_image_data())
}

fn imagebuf_to_image(buf: &ImageBuf) -> PyResult<Image> {
    let data = buf.to_image_data()
        .map_err(|e| PyIOError::new_err(format!("Conversion failed: {}", e)))?;
    Ok(Image::from_image_data(data))
}

fn py_roi_to_rust(roi: &Roi3D) -> RustRoi3D {
    RustRoi3D {
        xbegin: roi.xbegin,
        xend: roi.xend,
        ybegin: roi.ybegin,
        yend: roi.yend,
        zbegin: roi.zbegin,
        zend: roi.zend,
        chbegin: roi.chbegin,
        chend: roi.chend,
    }
}

fn convert_roi(roi: Option<&Roi3D>) -> Option<RustRoi3D> {
    roi.map(py_roi_to_rust)
}

// ============================================================================
// Drawing Functions
// ============================================================================

/// Draw a single pixel point at (x, y).
///
/// The color is blended with the existing pixel using alpha-over compositing
/// if an alpha channel is present.
///
/// Args:
///     image: Image to draw on
///     x: X coordinate of the point
///     y: Y coordinate of the point
///     color: Color values as list of floats (e.g., [1.0, 0.0, 0.0, 1.0] for red)
///     roi: Optional region of interest
///
/// Returns:
///     Modified image
///
/// Example:
///     >>> img = render_point(img, 50, 50, [1.0, 0.0, 0.0, 1.0])
#[pyfunction]
#[pyo3(signature = (image, x, y, color, roi=None))]
pub fn render_point(
    image: &Image,
    x: i32,
    y: i32,
    color: Vec<f32>,
    roi: Option<&Roi3D>,
) -> PyResult<Image> {
    let mut buf = image_to_imagebuf(image);
    rust_drawing::render_point(&mut buf, x, y, &color, convert_roi(roi));
    imagebuf_to_image(&buf)
}

/// Draw a line from (x1, y1) to (x2, y2).
///
/// Uses Bresenham's algorithm. The color is blended with existing pixels
/// using alpha-over compositing.
///
/// Args:
///     image: Image to draw on
///     x1, y1: Start point
///     x2, y2: End point
///     color: Color values as list of floats
///     skip_first_point: If True, don't draw the first point (useful for polylines)
///     roi: Optional region of interest
///
/// Returns:
///     Modified image
///
/// Example:
///     >>> img = render_line(img, 10, 10, 100, 50, [1.0, 1.0, 1.0, 1.0])
#[pyfunction]
#[pyo3(signature = (image, x1, y1, x2, y2, color, skip_first_point=false, roi=None))]
pub fn render_line(
    image: &Image,
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
    color: Vec<f32>,
    skip_first_point: bool,
    roi: Option<&Roi3D>,
) -> PyResult<Image> {
    let mut buf = image_to_imagebuf(image);
    rust_drawing::render_line(&mut buf, x1, y1, x2, y2, &color, skip_first_point, convert_roi(roi));
    imagebuf_to_image(&buf)
}

/// Draw a rectangle (filled or outline only).
///
/// Args:
///     image: Image to draw on
///     x1, y1: First corner
///     x2, y2: Opposite corner
///     color: Color values as list of floats
///     fill: If True, fill the rectangle; if False, draw only outline
///     roi: Optional region of interest
///
/// Returns:
///     Modified image
///
/// Example:
///     >>> # Draw a filled red rectangle
///     >>> img = render_box(img, 10, 10, 100, 50, [1.0, 0.0, 0.0, 1.0], fill=True)
///     >>> # Draw a green rectangle outline
///     >>> img = render_box(img, 10, 10, 100, 50, [0.0, 1.0, 0.0, 1.0], fill=False)
#[pyfunction]
#[pyo3(signature = (image, x1, y1, x2, y2, color, fill=false, roi=None))]
pub fn render_box(
    image: &Image,
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
    color: Vec<f32>,
    fill: bool,
    roi: Option<&Roi3D>,
) -> PyResult<Image> {
    let mut buf = image_to_imagebuf(image);
    rust_drawing::render_box(&mut buf, x1, y1, x2, y2, &color, fill, convert_roi(roi));
    imagebuf_to_image(&buf)
}

/// Draw a circle (filled or outline only).
///
/// Args:
///     image: Image to draw on
///     cx, cy: Center of the circle
///     radius: Radius of the circle
///     color: Color values as list of floats
///     fill: If True, fill the circle; if False, draw only outline
///     roi: Optional region of interest
///
/// Returns:
///     Modified image
///
/// Example:
///     >>> # Draw a filled blue circle
///     >>> img = render_circle(img, 100, 100, 25, [0.0, 0.0, 1.0, 1.0], fill=True)
#[pyfunction]
#[pyo3(signature = (image, cx, cy, radius, color, fill=false, roi=None))]
pub fn render_circle(
    image: &Image,
    cx: i32,
    cy: i32,
    radius: i32,
    color: Vec<f32>,
    fill: bool,
    roi: Option<&Roi3D>,
) -> PyResult<Image> {
    let mut buf = image_to_imagebuf(image);
    rust_drawing::render_circle(&mut buf, cx, cy, radius, &color, fill, convert_roi(roi));
    imagebuf_to_image(&buf)
}

/// Draw an ellipse (filled or outline only).
///
/// Args:
///     image: Image to draw on
///     cx, cy: Center of the ellipse
///     rx, ry: X and Y radii
///     color: Color values as list of floats
///     fill: If True, fill the ellipse; if False, draw only outline
///     roi: Optional region of interest
///
/// Returns:
///     Modified image
#[pyfunction]
#[pyo3(signature = (image, cx, cy, rx, ry, color, fill=false, roi=None))]
pub fn render_ellipse(
    image: &Image,
    cx: i32,
    cy: i32,
    rx: i32,
    ry: i32,
    color: Vec<f32>,
    fill: bool,
    roi: Option<&Roi3D>,
) -> PyResult<Image> {
    let mut buf = image_to_imagebuf(image);
    rust_drawing::render_ellipse(&mut buf, cx, cy, rx, ry, &color, fill, convert_roi(roi));
    imagebuf_to_image(&buf)
}

/// Draw a polygon (filled or outline only).
///
/// Args:
///     image: Image to draw on
///     points: List of (x, y) vertex coordinates as tuples
///     color: Color values as list of floats
///     fill: If True, fill the polygon; if False, draw only outline
///     roi: Optional region of interest
///
/// Returns:
///     Modified image
///
/// Example:
///     >>> # Draw a triangle
///     >>> vertices = [(100, 50), (50, 150), (150, 150)]
///     >>> img = render_polygon(img, vertices, [1.0, 1.0, 0.0, 1.0], fill=True)
#[pyfunction]
#[pyo3(signature = (image, points, color, fill=false, roi=None))]
pub fn render_polygon(
    image: &Image,
    points: Vec<(i32, i32)>,
    color: Vec<f32>,
    fill: bool,
    roi: Option<&Roi3D>,
) -> PyResult<Image> {
    let mut buf = image_to_imagebuf(image);
    rust_drawing::render_polygon(&mut buf, &points, &color, fill, convert_roi(roi));
    imagebuf_to_image(&buf)
}

// ============================================================================
// Text Rendering (feature-gated)
// ============================================================================

/// Text alignment options.
#[cfg(feature = "text")]
#[pyclass(eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAlign {
    Left = 0,
    Center = 1,
    Right = 2,
}

#[cfg(feature = "text")]
impl From<TextAlign> for RustTextAlign {
    fn from(a: TextAlign) -> Self {
        match a {
            TextAlign::Left => RustTextAlign::Left,
            TextAlign::Center => RustTextAlign::Center,
            TextAlign::Right => RustTextAlign::Right,
        }
    }
}

/// Text rendering style options.
#[cfg(feature = "text")]
#[pyclass]
#[derive(Debug, Clone)]
pub struct TextStyle {
    #[pyo3(get, set)]
    pub font: String,
    #[pyo3(get, set)]
    pub font_size: f32,
    #[pyo3(get, set)]
    pub color: [f32; 4],
    #[pyo3(get, set)]
    pub bg_color: [f32; 4],
    #[pyo3(get, set)]
    pub align: TextAlign,
    #[pyo3(get, set)]
    pub line_height: f32,
}

#[cfg(feature = "text")]
#[pymethods]
impl TextStyle {
    #[new]
    #[pyo3(signature = (font="sans-serif", font_size=48.0, color=None, bg_color=None, align=None, line_height=1.2))]
    fn new(
        font: &str,
        font_size: f32,
        color: Option<[f32; 4]>,
        bg_color: Option<[f32; 4]>,
        align: Option<TextAlign>,
        line_height: f32,
    ) -> Self {
        Self {
            font: font.to_string(),
            font_size,
            color: color.unwrap_or([1.0, 1.0, 1.0, 1.0]),
            bg_color: bg_color.unwrap_or([0.0, 0.0, 0.0, 0.0]),
            align: align.unwrap_or(TextAlign::Left),
            line_height,
        }
    }
}

#[cfg(feature = "text")]
impl From<&TextStyle> for RustTextStyle {
    fn from(s: &TextStyle) -> Self {
        RustTextStyle {
            font: s.font.clone(),
            font_size: s.font_size,
            color: s.color,
            bg_color: s.bg_color,
            align: s.align.into(),
            line_height: s.line_height,
        }
    }
}

/// Render text to a new image.
///
/// Uses cosmic-text for high-quality text rendering with proper
/// Unicode shaping and antialiasing.
///
/// Args:
///     text: Text content (supports \n for newlines)
///     style: Text rendering style options
///     width: Output width (0 = auto-size)
///     height: Output height (0 = auto-size)
///
/// Returns:
///     RGBA image with rendered text
///
/// Example:
///     >>> style = TextStyle(font_size=64.0, color=[1.0, 0.0, 0.0, 1.0])
///     >>> img = render_text("Hello World!", style, 512, 128)
#[cfg(feature = "text")]
#[pyfunction]
#[pyo3(signature = (text, style=None, width=0, height=0))]
pub fn render_text(
    text: &str,
    style: Option<&TextStyle>,
    width: u32,
    height: u32,
) -> PyResult<Image> {
    let rust_style = style.map(|s| s.into()).unwrap_or_default();
    let result = rust_text::render_text(text, &rust_style, width, height);
    imagebuf_to_image(&result)
}

/// Render text into an existing image.
///
/// Args:
///     image: Target image to render into
///     text: Text content
///     x, y: Position to render at
///     style: Text rendering style options
///
/// Returns:
///     Modified image with text rendered
#[cfg(feature = "text")]
#[pyfunction]
#[pyo3(signature = (image, text, x, y, style=None))]
pub fn render_text_into(
    image: &Image,
    text: &str,
    x: i32,
    y: i32,
    style: Option<&TextStyle>,
) -> PyResult<Image> {
    let mut buf = image_to_imagebuf(image);
    let rust_style = style.map(|s| s.into()).unwrap_or_default();
    rust_text::render_text_into(&mut buf, text, x, y, &rust_style);
    imagebuf_to_image(&buf)
}

// ============================================================================
// Module Registration
// ============================================================================

/// Register all drawing functions to the module.
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(render_point, m)?)?;
    m.add_function(wrap_pyfunction!(render_line, m)?)?;
    m.add_function(wrap_pyfunction!(render_box, m)?)?;
    m.add_function(wrap_pyfunction!(render_circle, m)?)?;
    m.add_function(wrap_pyfunction!(render_ellipse, m)?)?;
    m.add_function(wrap_pyfunction!(render_polygon, m)?)?;

    // Text rendering (feature-gated)
    #[cfg(feature = "text")]
    {
        m.add_class::<TextAlign>()?;
        m.add_class::<TextStyle>()?;
        m.add_function(wrap_pyfunction!(render_text, m)?)?;
        m.add_function(wrap_pyfunction!(render_text_into, m)?)?;
    }

    Ok(())
}
