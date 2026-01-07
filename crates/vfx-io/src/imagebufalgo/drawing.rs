//! Drawing operations for images.
//!
//! This module provides basic drawing primitives compatible with
//! OpenImageIO's ImageBufAlgo drawing functions.
//!
//! # Functions
//!
//! - [`render_point`] - Draw a single pixel point
//! - [`render_line`] - Draw a line between two points (Bresenham's algorithm)
//! - [`render_box`] - Draw a filled or unfilled rectangle

use crate::imagebuf::{ImageBuf, WrapMode};
use vfx_core::Roi3D;

/// Draw a single pixel point at (x, y) with the specified color.
///
/// The color is blended with the existing pixel using alpha-over compositing
/// if an alpha channel is present.
///
/// # Arguments
///
/// * `dst` - Destination image buffer
/// * `x` - X coordinate of the point
/// * `y` - Y coordinate of the point
/// * `color` - Color values for each channel (alpha last if present)
/// * `roi` - Optional region of interest
///
/// # Example
///
/// ```ignore
/// use vfx_io::imagebufalgo::render_point;
///
/// // Draw a red point at (50, 50) in an RGBA image
/// render_point(&mut img, 50, 50, &[1.0, 0.0, 0.0, 1.0], None);
/// ```
pub fn render_point(
    dst: &mut ImageBuf,
    x: i32,
    y: i32,
    color: &[f32],
    roi: Option<Roi3D>,
) {
    let roi = roi.unwrap_or_else(|| dst.roi());

    // Check bounds
    if x < roi.xbegin || x >= roi.xend || y < roi.ybegin || y >= roi.yend {
        return;
    }

    let nch = dst.nchannels() as usize;
    let alpha_ch = dst.spec().alpha_channel;

    // Get existing pixel
    let mut existing = vec![0.0f32; nch];
    dst.getpixel(x, y, 0, &mut existing, WrapMode::Black);

    // Prepare color (extend or truncate to match channel count)
    let mut new_color = vec![0.0f32; nch];
    for i in 0..nch {
        new_color[i] = color.get(i).copied().unwrap_or(1.0);
    }

    // Alpha-over blend if there's an alpha channel
    if alpha_ch >= 0 && (alpha_ch as usize) < nch {
        let alpha_idx = alpha_ch as usize;
        let src_alpha = new_color[alpha_idx];
        let dst_alpha = existing[alpha_idx];

        // Over compositing: out = src * src_a + dst * dst_a * (1 - src_a)
        let out_alpha = src_alpha + dst_alpha * (1.0 - src_alpha);
        if out_alpha > 0.0 {
            for i in 0..nch {
                if i != alpha_idx {
                    new_color[i] = (new_color[i] * src_alpha + existing[i] * dst_alpha * (1.0 - src_alpha)) / out_alpha;
                }
            }
            new_color[alpha_idx] = out_alpha;
        }
    }

    dst.setpixel(x, y, 0, &new_color);
}

/// Draw a line from (x1, y1) to (x2, y2) using Bresenham's algorithm.
///
/// The color is blended with existing pixels using alpha-over compositing.
///
/// # Arguments
///
/// * `dst` - Destination image buffer
/// * `x1`, `y1` - Start point
/// * `x2`, `y2` - End point
/// * `color` - Color values for each channel
/// * `skip_first_point` - If true, don't draw the first point (useful for polylines)
/// * `roi` - Optional region of interest
///
/// # Example
///
/// ```ignore
/// use vfx_io::imagebufalgo::render_line;
///
/// // Draw a white line from (10, 10) to (100, 50)
/// render_line(&mut img, 10, 10, 100, 50, &[1.0, 1.0, 1.0, 1.0], false, None);
/// ```
pub fn render_line(
    dst: &mut ImageBuf,
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
    color: &[f32],
    skip_first_point: bool,
    roi: Option<Roi3D>,
) {
    // Bresenham's line algorithm
    let dx = (x2 - x1).abs();
    let dy = -(y2 - y1).abs();
    let sx = if x1 < x2 { 1 } else { -1 };
    let sy = if y1 < y2 { 1 } else { -1 };
    let mut err = dx + dy;

    let mut x = x1;
    let mut y = y1;
    let mut first = true;

    loop {
        if !first || !skip_first_point {
            render_point(dst, x, y, color, roi);
        }
        first = false;

        if x == x2 && y == y2 {
            break;
        }

        let e2 = 2 * err;
        if e2 >= dy {
            if x == x2 {
                break;
            }
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            if y == y2 {
                break;
            }
            err += dx;
            y += sy;
        }
    }
}

/// Draw a rectangle (filled or outline only).
///
/// # Arguments
///
/// * `dst` - Destination image buffer
/// * `x1`, `y1` - First corner
/// * `x2`, `y2` - Opposite corner
/// * `color` - Color values for each channel
/// * `fill` - If true, fill the rectangle; if false, draw only the outline
/// * `roi` - Optional region of interest
///
/// # Example
///
/// ```ignore
/// use vfx_io::imagebufalgo::render_box;
///
/// // Draw a filled red rectangle
/// render_box(&mut img, 10, 10, 100, 50, &[1.0, 0.0, 0.0, 1.0], true, None);
///
/// // Draw a green rectangle outline
/// render_box(&mut img, 10, 10, 100, 50, &[0.0, 1.0, 0.0, 1.0], false, None);
/// ```
pub fn render_box(
    dst: &mut ImageBuf,
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
    color: &[f32],
    fill: bool,
    roi: Option<Roi3D>,
) {
    // Normalize coordinates
    let (xmin, xmax) = if x1 <= x2 { (x1, x2) } else { (x2, x1) };
    let (ymin, ymax) = if y1 <= y2 { (y1, y2) } else { (y2, y1) };

    if fill {
        // Fill the entire rectangle
        for y in ymin..=ymax {
            for x in xmin..=xmax {
                render_point(dst, x, y, color, roi);
            }
        }
    } else {
        // Draw only the outline (4 lines)
        // Top edge
        render_line(dst, xmin, ymin, xmax, ymin, color, false, roi);
        // Bottom edge
        render_line(dst, xmin, ymax, xmax, ymax, color, false, roi);
        // Left edge
        render_line(dst, xmin, ymin, xmin, ymax, color, false, roi);
        // Right edge
        render_line(dst, xmax, ymin, xmax, ymax, color, false, roi);
    }
}

/// Draw a circle (filled or outline only).
///
/// Uses Bresenham's circle algorithm for the outline.
///
/// # Arguments
///
/// * `dst` - Destination image buffer
/// * `cx`, `cy` - Center of the circle
/// * `radius` - Radius of the circle
/// * `color` - Color values for each channel
/// * `fill` - If true, fill the circle; if false, draw only the outline
/// * `roi` - Optional region of interest
///
/// # Example
///
/// ```ignore
/// use vfx_io::imagebufalgo::render_circle;
///
/// // Draw a filled blue circle with radius 25 centered at (100, 100)
/// render_circle(&mut img, 100, 100, 25, &[0.0, 0.0, 1.0, 1.0], true, None);
/// ```
pub fn render_circle(
    dst: &mut ImageBuf,
    cx: i32,
    cy: i32,
    radius: i32,
    color: &[f32],
    fill: bool,
    roi: Option<Roi3D>,
) {
    if radius <= 0 {
        render_point(dst, cx, cy, color, roi);
        return;
    }

    if fill {
        // Filled circle using horizontal lines
        let r2 = radius * radius;
        for dy in -radius..=radius {
            let dx_max = ((r2 - dy * dy) as f32).sqrt() as i32;
            for dx in -dx_max..=dx_max {
                render_point(dst, cx + dx, cy + dy, color, roi);
            }
        }
    } else {
        // Bresenham's circle algorithm
        let mut x = radius;
        let mut y = 0;
        let mut err = 0;

        while x >= y {
            // Draw 8 symmetric points
            render_point(dst, cx + x, cy + y, color, roi);
            render_point(dst, cx + y, cy + x, color, roi);
            render_point(dst, cx - y, cy + x, color, roi);
            render_point(dst, cx - x, cy + y, color, roi);
            render_point(dst, cx - x, cy - y, color, roi);
            render_point(dst, cx - y, cy - x, color, roi);
            render_point(dst, cx + y, cy - x, color, roi);
            render_point(dst, cx + x, cy - y, color, roi);

            y += 1;
            err += 1 + 2 * y;
            if 2 * (err - x) + 1 > 0 {
                x -= 1;
                err += 1 - 2 * x;
            }
        }
    }
}

/// Draw an ellipse (filled or outline only).
///
/// # Arguments
///
/// * `dst` - Destination image buffer
/// * `cx`, `cy` - Center of the ellipse
/// * `rx`, `ry` - X and Y radii
/// * `color` - Color values for each channel
/// * `fill` - If true, fill the ellipse; if false, draw only the outline
/// * `roi` - Optional region of interest
pub fn render_ellipse(
    dst: &mut ImageBuf,
    cx: i32,
    cy: i32,
    rx: i32,
    ry: i32,
    color: &[f32],
    fill: bool,
    roi: Option<Roi3D>,
) {
    if rx <= 0 || ry <= 0 {
        render_point(dst, cx, cy, color, roi);
        return;
    }

    let rx2 = (rx * rx) as f64;
    let ry2 = (ry * ry) as f64;

    if fill {
        // Filled ellipse using horizontal lines
        for dy in -ry..=ry {
            let dy2 = (dy * dy) as f64;
            let dx_max = (rx2 * (1.0 - dy2 / ry2)).sqrt() as i32;
            for dx in -dx_max..=dx_max {
                render_point(dst, cx + dx, cy + dy, color, roi);
            }
        }
    } else {
        // Midpoint ellipse algorithm
        let mut x = 0i32;
        let mut y = ry;

        let mut d1 = ry2 - rx2 * ry as f64 + 0.25 * rx2;
        let mut dx = 2.0 * ry2 * x as f64;
        let mut dy = 2.0 * rx2 * y as f64;

        // Region 1
        while dx < dy {
            render_point(dst, cx + x, cy + y, color, roi);
            render_point(dst, cx - x, cy + y, color, roi);
            render_point(dst, cx + x, cy - y, color, roi);
            render_point(dst, cx - x, cy - y, color, roi);

            if d1 < 0.0 {
                x += 1;
                dx += 2.0 * ry2;
                d1 += dx + ry2;
            } else {
                x += 1;
                y -= 1;
                dx += 2.0 * ry2;
                dy -= 2.0 * rx2;
                d1 += dx - dy + ry2;
            }
        }

        // Region 2
        let mut d2 = ry2 * (x as f64 + 0.5).powi(2) + rx2 * (y as f64 - 1.0).powi(2) - rx2 * ry2;
        while y >= 0 {
            render_point(dst, cx + x, cy + y, color, roi);
            render_point(dst, cx - x, cy + y, color, roi);
            render_point(dst, cx + x, cy - y, color, roi);
            render_point(dst, cx - x, cy - y, color, roi);

            if d2 > 0.0 {
                y -= 1;
                dy -= 2.0 * rx2;
                d2 += rx2 - dy;
            } else {
                y -= 1;
                x += 1;
                dx += 2.0 * ry2;
                dy -= 2.0 * rx2;
                d2 += dx - dy + rx2;
            }
        }
    }
}

/// Draw a polygon (filled or outline only).
///
/// # Arguments
///
/// * `dst` - Destination image buffer
/// * `points` - List of (x, y) vertex coordinates
/// * `color` - Color values for each channel
/// * `fill` - If true, fill the polygon; if false, draw only the outline
/// * `roi` - Optional region of interest
///
/// # Example
///
/// ```ignore
/// use vfx_io::imagebufalgo::render_polygon;
///
/// // Draw a triangle
/// let vertices = vec![(100, 50), (50, 150), (150, 150)];
/// render_polygon(&mut img, &vertices, &[1.0, 1.0, 0.0, 1.0], true, None);
/// ```
pub fn render_polygon(
    dst: &mut ImageBuf,
    points: &[(i32, i32)],
    color: &[f32],
    fill: bool,
    roi: Option<Roi3D>,
) {
    if points.len() < 2 {
        if let Some(&(x, y)) = points.first() {
            render_point(dst, x, y, color, roi);
        }
        return;
    }

    if fill && points.len() >= 3 {
        // Scanline fill algorithm
        let mut ymin = i32::MAX;
        let mut ymax = i32::MIN;
        for &(_, y) in points {
            ymin = ymin.min(y);
            ymax = ymax.max(y);
        }

        for y in ymin..=ymax {
            let mut intersections = Vec::new();

            for i in 0..points.len() {
                let j = (i + 1) % points.len();
                let (x1, y1) = points[i];
                let (x2, y2) = points[j];

                if (y1 <= y && y < y2) || (y2 <= y && y < y1) {
                    // Calculate x intersection
                    let x = x1 + (y - y1) * (x2 - x1) / (y2 - y1);
                    intersections.push(x);
                }
            }

            intersections.sort();

            // Fill between pairs of intersections
            for chunk in intersections.chunks(2) {
                if chunk.len() == 2 {
                    for x in chunk[0]..=chunk[1] {
                        render_point(dst, x, y, color, roi);
                    }
                }
            }
        }
    } else {
        // Draw outline
        for i in 0..points.len() {
            let j = (i + 1) % points.len();
            let (x1, y1) = points[i];
            let (x2, y2) = points[j];
            render_line(dst, x1, y1, x2, y2, color, i > 0, roi);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::imagebuf::InitializePixels;
    use vfx_core::ImageSpec;

    #[test]
    fn test_render_point() {
        let spec = ImageSpec::rgba(10, 10);
        let mut img = ImageBuf::new(spec, InitializePixels::Yes);

        render_point(&mut img, 5, 5, &[1.0, 0.0, 0.0, 1.0], None);

        let mut pixel = [0.0f32; 4];
        img.getpixel(5, 5, 0, &mut pixel, WrapMode::Black);
        assert!((pixel[0] - 1.0).abs() < 0.01);
        assert!(pixel[1].abs() < 0.01);
        assert!(pixel[2].abs() < 0.01);
        assert!((pixel[3] - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_render_line() {
        let spec = ImageSpec::rgba(10, 10);
        let mut img = ImageBuf::new(spec, InitializePixels::Yes);

        render_line(&mut img, 0, 0, 9, 0, &[1.0, 1.0, 1.0, 1.0], false, None);

        // Check that horizontal line was drawn
        for x in 0..10 {
            let mut pixel = [0.0f32; 4];
            img.getpixel(x, 0, 0, &mut pixel, WrapMode::Black);
            assert!((pixel[0] - 1.0).abs() < 0.01, "Pixel at {} not white", x);
        }
    }

    #[test]
    fn test_render_box_filled() {
        let spec = ImageSpec::rgba(20, 20);
        let mut img = ImageBuf::new(spec, InitializePixels::Yes);

        render_box(&mut img, 5, 5, 15, 15, &[0.0, 1.0, 0.0, 1.0], true, None);

        // Check center pixel
        let mut pixel = [0.0f32; 4];
        img.getpixel(10, 10, 0, &mut pixel, WrapMode::Black);
        assert!((pixel[1] - 1.0).abs() < 0.01);

        // Check corner pixel
        img.getpixel(5, 5, 0, &mut pixel, WrapMode::Black);
        assert!((pixel[1] - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_render_circle() {
        let spec = ImageSpec::rgba(30, 30);
        let mut img = ImageBuf::new(spec, InitializePixels::Yes);

        render_circle(&mut img, 15, 15, 10, &[1.0, 0.0, 1.0, 1.0], true, None);

        // Check center pixel
        let mut pixel = [0.0f32; 4];
        img.getpixel(15, 15, 0, &mut pixel, WrapMode::Black);
        assert!((pixel[0] - 1.0).abs() < 0.01);
        assert!((pixel[2] - 1.0).abs() < 0.01);
    }
}
