//! Warp (geometric transformation) operations for images.
//!
//! This module provides warping operations compatible with
//! OpenImageIO's ImageBufAlgo warp functions.
//!
//! # Functions
//!
//! - [`warp`] - Warp image using 3x3 transformation matrix
//! - [`st_warp`] - Warp image using per-pixel ST coordinates (like Nuke's STMap)

use crate::imagebuf::{ImageBuf, InitializePixels, WrapMode};
use vfx_core::{DataFormat, ImageSpec, Roi3D};

/// Wrap modes for warping operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WarpWrap {
    /// Return black for out-of-bounds coordinates
    #[default]
    Black,
    /// Clamp coordinates to image bounds
    Clamp,
    /// Tile/repeat the image periodically
    Periodic,
    /// Mirror at edges
    Mirror,
}

impl From<&str> for WarpWrap {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "clamp" => WarpWrap::Clamp,
            "periodic" | "repeat" | "tile" => WarpWrap::Periodic,
            "mirror" => WarpWrap::Mirror,
            _ => WarpWrap::Black,
        }
    }
}

/// Warp an image using a 3x3 transformation matrix.
///
/// The matrix transforms destination pixel coordinates to source pixel coordinates.
/// Uses bilinear interpolation for sampling.
///
/// # Arguments
///
/// * `src` - Source image
/// * `matrix` - 3x3 transformation matrix (row-major, 9 elements)
/// * `wrap` - Wrap mode for out-of-bounds coordinates
/// * `roi` - Optional output region (defaults to source dimensions)
///
/// # Matrix Format
///
/// The matrix is 3x3 in row-major order:
/// ```text
/// [ m[0] m[1] m[2] ]   [ a  b  tx ]
/// [ m[3] m[4] m[5] ] = [ c  d  ty ]
/// [ m[6] m[7] m[8] ]   [ px py 1  ]
/// ```
///
/// For a point (x, y) in the destination, the source coordinates are:
/// ```text
/// w = m[6]*x + m[7]*y + m[8]
/// src_x = (m[0]*x + m[1]*y + m[2]) / w
/// src_y = (m[3]*x + m[4]*y + m[5]) / w
/// ```
///
/// # Example
///
/// ```ignore
/// use vfx_io::imagebufalgo::warp;
///
/// // Identity matrix (no transformation)
/// let identity = [1.0, 0.0, 0.0,  0.0, 1.0, 0.0,  0.0, 0.0, 1.0];
///
/// // Scale by 2x
/// let scale_2x = [2.0, 0.0, 0.0,  0.0, 2.0, 0.0,  0.0, 0.0, 1.0];
///
/// // Rotate 45 degrees
/// let angle = std::f32::consts::PI / 4.0;
/// let cos_a = angle.cos();
/// let sin_a = angle.sin();
/// let rotate_45 = [cos_a, -sin_a, 0.0,  sin_a, cos_a, 0.0,  0.0, 0.0, 1.0];
///
/// let warped = warp(&src, &scale_2x, WarpWrap::Black, None);
/// ```
pub fn warp(
    src: &ImageBuf,
    matrix: &[f32; 9],
    wrap: WarpWrap,
    roi: Option<Roi3D>,
) -> ImageBuf {
    let src_roi = src.roi();
    let roi = roi.unwrap_or_else(|| src_roi.clone());

    let width = roi.width() as u32;
    let height = roi.height() as u32;
    let nch = src.nchannels() as usize;

    let spec = ImageSpec::new(width, height, nch as u8, DataFormat::F32);
    let mut dst = ImageBuf::new(spec, InitializePixels::Yes);

    warp_into(&mut dst, src, matrix, wrap, Some(roi));
    dst
}

/// Warp an image into an existing buffer.
pub fn warp_into(
    dst: &mut ImageBuf,
    src: &ImageBuf,
    matrix: &[f32; 9],
    wrap: WarpWrap,
    roi: Option<Roi3D>,
) {
    let dst_roi = roi.unwrap_or_else(|| dst.roi());

    let nch = src.nchannels() as usize;
    let mut pixel = vec![0.0f32; nch];

    for y in dst_roi.ybegin..dst_roi.yend {
        for x in dst_roi.xbegin..dst_roi.xend {
            let dx = x as f32;
            let dy = y as f32;

            // Apply homogeneous transformation
            let w = matrix[6] * dx + matrix[7] * dy + matrix[8];
            if w.abs() < 1e-10 {
                dst.setpixel(x - dst_roi.xbegin, y - dst_roi.ybegin, 0, &vec![0.0f32; nch]);
                continue;
            }

            let sx = (matrix[0] * dx + matrix[1] * dy + matrix[2]) / w;
            let sy = (matrix[3] * dx + matrix[4] * dy + matrix[5]) / w;

            // Sample source with bilinear interpolation
            sample_bilinear(src, sx, sy, wrap, &mut pixel);

            dst.setpixel(x - dst_roi.xbegin, y - dst_roi.ybegin, 0, &pixel);
        }
    }
}

/// Warp an image using per-pixel ST coordinates.
///
/// Each pixel in `stbuf` provides normalized (0-1) coordinates specifying
/// where to sample from `src`. This is similar to Nuke's STMap node.
///
/// # Arguments
///
/// * `src` - Source image to sample from
/// * `stbuf` - ST coordinate image (at least 2 channels for S and T)
/// * `chan_s` - Channel index for S coordinate in stbuf (default 0)
/// * `chan_t` - Channel index for T coordinate in stbuf (default 1)
/// * `flip_s` - Mirror S coordinate horizontally
/// * `flip_t` - Mirror T coordinate vertically
/// * `wrap` - Wrap mode for out-of-bounds sampling
/// * `roi` - Optional output region
///
/// # ST Coordinates
///
/// - S=0, T=0 corresponds to top-left of the source image
/// - S=1, T=1 corresponds to bottom-right of the source image
/// - Values outside 0-1 sample beyond image boundaries (controlled by wrap mode)
///
/// # Example
///
/// ```ignore
/// use vfx_io::imagebufalgo::st_warp;
///
/// // stbuf contains per-pixel UV coordinates
/// let warped = st_warp(&source, &stmap, 0, 1, false, false, WarpWrap::Black, None);
/// ```
pub fn st_warp(
    src: &ImageBuf,
    stbuf: &ImageBuf,
    chan_s: usize,
    chan_t: usize,
    flip_s: bool,
    flip_t: bool,
    wrap: WarpWrap,
    roi: Option<Roi3D>,
) -> ImageBuf {
    let st_roi = stbuf.roi();
    let roi = roi.unwrap_or_else(|| st_roi.clone());

    let width = roi.width() as u32;
    let height = roi.height() as u32;
    let nch = src.nchannels() as usize;

    let spec = ImageSpec::new(width, height, nch as u8, DataFormat::F32);
    let mut dst = ImageBuf::new(spec, InitializePixels::Yes);

    st_warp_into(&mut dst, src, stbuf, chan_s, chan_t, flip_s, flip_t, wrap, Some(roi));
    dst
}

/// ST warp into an existing buffer.
pub fn st_warp_into(
    dst: &mut ImageBuf,
    src: &ImageBuf,
    stbuf: &ImageBuf,
    chan_s: usize,
    chan_t: usize,
    flip_s: bool,
    flip_t: bool,
    wrap: WarpWrap,
    roi: Option<Roi3D>,
) {
    let dst_roi = roi.unwrap_or_else(|| dst.roi());
    let src_roi = src.roi();

    let st_nch = stbuf.nchannels() as usize;
    let src_nch = src.nchannels() as usize;

    let src_width = src_roi.width() as f32;
    let src_height = src_roi.height() as f32;

    let mut st_pixel = vec![0.0f32; st_nch];
    let mut src_pixel = vec![0.0f32; src_nch];

    for y in dst_roi.ybegin..dst_roi.yend {
        for x in dst_roi.xbegin..dst_roi.xend {
            // Read ST coordinates
            stbuf.getpixel(x, y, 0, &mut st_pixel, WrapMode::Black);

            let mut s = st_pixel.get(chan_s).copied().unwrap_or(0.0);
            let mut t = st_pixel.get(chan_t).copied().unwrap_or(0.0);

            // Apply flips
            if flip_s {
                s = 1.0 - s;
            }
            if flip_t {
                t = 1.0 - t;
            }

            // Convert normalized coordinates to pixel coordinates
            let sx = s * src_width + src_roi.xbegin as f32;
            let sy = t * src_height + src_roi.ybegin as f32;

            // Sample source with bilinear interpolation
            sample_bilinear(src, sx, sy, wrap, &mut src_pixel);

            dst.setpixel(x - dst_roi.xbegin, y - dst_roi.ybegin, 0, &src_pixel);
        }
    }
}

/// Bilinear interpolation sampling from an image.
fn sample_bilinear(
    src: &ImageBuf,
    x: f32,
    y: f32,
    wrap: WarpWrap,
    output: &mut [f32],
) {
    let roi = src.roi();
    let nch = output.len().min(src.nchannels() as usize);

    // Get integer and fractional parts
    let x0 = x.floor() as i32;
    let y0 = y.floor() as i32;
    let x1 = x0 + 1;
    let y1 = y0 + 1;
    let fx = x - x0 as f32;
    let fy = y - y0 as f32;

    // Get wrapped coordinates
    let (wx0, in0) = wrap_coord(x0, roi.xbegin, roi.xend, wrap);
    let (wy0, in1) = wrap_coord(y0, roi.ybegin, roi.yend, wrap);
    let (wx1, in2) = wrap_coord(x1, roi.xbegin, roi.xend, wrap);
    let (wy1, in3) = wrap_coord(y1, roi.ybegin, roi.yend, wrap);

    // Sample four corners
    let mut p00 = vec![0.0f32; nch];
    let mut p10 = vec![0.0f32; nch];
    let mut p01 = vec![0.0f32; nch];
    let mut p11 = vec![0.0f32; nch];

    if in0 && in1 {
        src.getpixel(wx0, wy0, 0, &mut p00, WrapMode::Black);
    }
    if in2 && in1 {
        src.getpixel(wx1, wy0, 0, &mut p10, WrapMode::Black);
    }
    if in0 && in3 {
        src.getpixel(wx0, wy1, 0, &mut p01, WrapMode::Black);
    }
    if in2 && in3 {
        src.getpixel(wx1, wy1, 0, &mut p11, WrapMode::Black);
    }

    // Bilinear interpolation
    for c in 0..nch {
        let top = p00[c] * (1.0 - fx) + p10[c] * fx;
        let bottom = p01[c] * (1.0 - fx) + p11[c] * fx;
        output[c] = top * (1.0 - fy) + bottom * fy;
    }
}

/// Apply wrap mode to a coordinate, returns (wrapped_coord, is_inside)
fn wrap_coord(coord: i32, begin: i32, end: i32, wrap: WarpWrap) -> (i32, bool) {
    let size = end - begin;
    if size <= 0 {
        return (begin, false);
    }

    if coord >= begin && coord < end {
        return (coord, true);
    }

    match wrap {
        WarpWrap::Black => (coord, false),
        WarpWrap::Clamp => {
            let clamped = coord.max(begin).min(end - 1);
            (clamped, true)
        }
        WarpWrap::Periodic => {
            let mut c = (coord - begin) % size;
            if c < 0 {
                c += size;
            }
            (begin + c, true)
        }
        WarpWrap::Mirror => {
            let mut c = (coord - begin) % (2 * size);
            if c < 0 {
                c += 2 * size;
            }
            if c >= size {
                c = 2 * size - 1 - c;
            }
            (begin + c, true)
        }
    }
}

// Helper functions for creating common transformation matrices

/// Create an identity matrix.
pub fn matrix_identity() -> [f32; 9] {
    [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]
}

/// Create a translation matrix.
pub fn matrix_translate(tx: f32, ty: f32) -> [f32; 9] {
    [1.0, 0.0, tx, 0.0, 1.0, ty, 0.0, 0.0, 1.0]
}

/// Create a scale matrix.
pub fn matrix_scale(sx: f32, sy: f32) -> [f32; 9] {
    [sx, 0.0, 0.0, 0.0, sy, 0.0, 0.0, 0.0, 1.0]
}

/// Create a rotation matrix (angle in radians).
pub fn matrix_rotate(angle: f32) -> [f32; 9] {
    let c = angle.cos();
    let s = angle.sin();
    [c, -s, 0.0, s, c, 0.0, 0.0, 0.0, 1.0]
}

/// Create a shear matrix.
pub fn matrix_shear(shx: f32, shy: f32) -> [f32; 9] {
    [1.0, shx, 0.0, shy, 1.0, 0.0, 0.0, 0.0, 1.0]
}

/// Multiply two 3x3 matrices.
pub fn matrix_multiply(a: &[f32; 9], b: &[f32; 9]) -> [f32; 9] {
    let mut result = [0.0f32; 9];
    for i in 0..3 {
        for j in 0..3 {
            result[i * 3 + j] = a[i * 3 + 0] * b[0 * 3 + j]
                + a[i * 3 + 1] * b[1 * 3 + j]
                + a[i * 3 + 2] * b[2 * 3 + j];
        }
    }
    result
}

/// Invert a 3x3 matrix. Returns None if the matrix is singular.
pub fn matrix_invert(m: &[f32; 9]) -> Option<[f32; 9]> {
    let det = m[0] * (m[4] * m[8] - m[5] * m[7])
        - m[1] * (m[3] * m[8] - m[5] * m[6])
        + m[2] * (m[3] * m[7] - m[4] * m[6]);

    if det.abs() < 1e-10 {
        return None;
    }

    let inv_det = 1.0 / det;

    Some([
        (m[4] * m[8] - m[5] * m[7]) * inv_det,
        (m[2] * m[7] - m[1] * m[8]) * inv_det,
        (m[1] * m[5] - m[2] * m[4]) * inv_det,
        (m[5] * m[6] - m[3] * m[8]) * inv_det,
        (m[0] * m[8] - m[2] * m[6]) * inv_det,
        (m[2] * m[3] - m[0] * m[5]) * inv_det,
        (m[3] * m[7] - m[4] * m[6]) * inv_det,
        (m[1] * m[6] - m[0] * m[7]) * inv_det,
        (m[0] * m[4] - m[1] * m[3]) * inv_det,
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_warp_identity() {
        let spec = ImageSpec::rgba(10, 10);
        let mut src = ImageBuf::new(spec, InitializePixels::No);

        // Fill with gradient
        for y in 0..10 {
            for x in 0..10 {
                src.setpixel(x, y, 0, &[x as f32 / 10.0, y as f32 / 10.0, 0.0, 1.0]);
            }
        }

        let identity = matrix_identity();
        let dst = warp(&src, &identity, WarpWrap::Black, None);

        // Check center pixel
        let mut orig = [0.0f32; 4];
        let mut warped = [0.0f32; 4];
        src.getpixel(5, 5, 0, &mut orig, WrapMode::Black);
        dst.getpixel(5, 5, 0, &mut warped, WrapMode::Black);

        assert!((orig[0] - warped[0]).abs() < 0.1);
        assert!((orig[1] - warped[1]).abs() < 0.1);
    }

    #[test]
    fn test_st_warp() {
        let spec = ImageSpec::rgba(10, 10);
        let mut src = ImageBuf::new(spec, InitializePixels::No);

        // Fill source with color gradient
        for y in 0..10 {
            for x in 0..10 {
                src.setpixel(x, y, 0, &[x as f32 / 10.0, y as f32 / 10.0, 0.0, 1.0]);
            }
        }

        // Create identity ST map (s=x/width, t=y/height)
        let st_spec = ImageSpec::new(10, 10, 2, DataFormat::F32);
        let mut stbuf = ImageBuf::new(st_spec, InitializePixels::No);

        for y in 0..10 {
            for x in 0..10 {
                let s = x as f32 / 10.0;
                let t = y as f32 / 10.0;
                stbuf.setpixel(x, y, 0, &[s, t]);
            }
        }

        let dst = st_warp(&src, &stbuf, 0, 1, false, false, WarpWrap::Black, None);

        // Check center pixel
        let mut orig = [0.0f32; 4];
        let mut warped = [0.0f32; 4];
        src.getpixel(5, 5, 0, &mut orig, WrapMode::Black);
        dst.getpixel(5, 5, 0, &mut warped, WrapMode::Black);

        assert!((orig[0] - warped[0]).abs() < 0.2);
        assert!((orig[1] - warped[1]).abs() < 0.2);
    }

    #[test]
    fn test_matrix_multiply() {
        let identity = matrix_identity();
        let scale = matrix_scale(2.0, 3.0);

        // Identity * M = M
        let combined = matrix_multiply(&identity, &scale);
        assert!((combined[0] - 2.0).abs() < 0.001);
        assert!((combined[4] - 3.0).abs() < 0.001);

        // M * Identity = M
        let combined2 = matrix_multiply(&scale, &identity);
        assert!((combined2[0] - 2.0).abs() < 0.001);
        assert!((combined2[4] - 3.0).abs() < 0.001);
    }

    #[test]
    fn test_matrix_invert() {
        let m = matrix_scale(2.0, 3.0);
        let inv = matrix_invert(&m).unwrap();

        // Inverse of scale(2, 3) is scale(0.5, 0.333...)
        assert!((inv[0] - 0.5).abs() < 0.001);
        assert!((inv[4] - 1.0 / 3.0).abs() < 0.001);
    }
}
