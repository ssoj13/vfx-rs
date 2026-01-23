//! Iterator support for ImageBuf.
//!
//! Provides efficient iteration over ImageBuf pixels with OIIO-compatible semantics.

use super::{ImageBuf, WrapMode};
use vfx_core::Roi3D;

/// Iterator over pixels in an ImageBuf.
///
/// Iterates over all pixels in the specified ROI, yielding pixel coordinates.
pub struct PixelIterator {
    roi: Roi3D,
    x: i32,
    y: i32,
    z: i32,
}

impl PixelIterator {
    /// Creates a new iterator over the specified ROI.
    pub fn new(roi: Roi3D) -> Self {
        Self {
            x: roi.xbegin,
            y: roi.ybegin,
            z: roi.zbegin,
            roi,
        }
    }

    /// Creates an iterator over the entire image.
    pub fn all(buf: &ImageBuf) -> Self {
        Self::new(buf.roi())
    }
}

impl Iterator for PixelIterator {
    type Item = (i32, i32, i32);

    fn next(&mut self) -> Option<Self::Item> {
        if self.z >= self.roi.zend {
            return None;
        }

        let result = (self.x, self.y, self.z);

        // Advance to next pixel
        self.x += 1;
        if self.x >= self.roi.xend {
            self.x = self.roi.xbegin;
            self.y += 1;
            if self.y >= self.roi.yend {
                self.y = self.roi.ybegin;
                self.z += 1;
            }
        }

        Some(result)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.roi.npixels() as usize;
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for PixelIterator {}

/// Iterator that yields pixel values as f32 slices.
pub struct PixelValueIterator<'a> {
    buf: &'a ImageBuf,
    iter: PixelIterator,
    nchannels: usize,
    wrap: WrapMode,
}

impl<'a> PixelValueIterator<'a> {
    /// Creates a new pixel value iterator.
    pub fn new(buf: &'a ImageBuf, roi: Roi3D, wrap: WrapMode) -> Self {
        Self {
            buf,
            iter: PixelIterator::new(roi),
            nchannels: buf.nchannels() as usize,
            wrap,
        }
    }

    /// Creates an iterator over all pixels.
    pub fn all(buf: &'a ImageBuf) -> Self {
        Self::new(buf, buf.roi(), WrapMode::Black)
    }
}

impl<'a> Iterator for PixelValueIterator<'a> {
    type Item = (i32, i32, i32, Vec<f32>);

    fn next(&mut self) -> Option<Self::Item> {
        let (x, y, z) = self.iter.next()?;
        let mut pixel = vec![0.0f32; self.nchannels];
        self.buf.getpixel(x, y, z, &mut pixel, self.wrap);
        Some((x, y, z, pixel))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

/// Iterator for scanning lines (rows) of an image.
pub struct ScanlineIterator {
    #[allow(dead_code)]  // Stored for potential future use
    width: i32,
    xbegin: i32,
    xend: i32,
    y: i32,
    ybegin: i32,
    yend: i32,
    z: i32,
    zend: i32,
}

impl ScanlineIterator {
    /// Creates a new scanline iterator for the given ROI.
    pub fn new(roi: Roi3D) -> Self {
        Self {
            width: roi.width(),
            xbegin: roi.xbegin,
            xend: roi.xend,
            y: roi.ybegin,
            ybegin: roi.ybegin,
            yend: roi.yend,
            z: roi.zbegin,
            zend: roi.zend,
        }
    }

    /// Creates a scanline iterator for the entire image.
    pub fn all(buf: &ImageBuf) -> Self {
        Self::new(buf.roi())
    }
}

impl Iterator for ScanlineIterator {
    /// Yields (y, z, xbegin, xend) for each scanline.
    type Item = (i32, i32, i32, i32);

    fn next(&mut self) -> Option<Self::Item> {
        if self.z >= self.zend {
            return None;
        }

        let result = (self.y, self.z, self.xbegin, self.xend);

        self.y += 1;
        if self.y >= self.yend {
            self.y = self.ybegin; // Reset to ROI start row
            self.z += 1;
        }

        Some(result)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        // Calculate remaining scanlines considering ROI bounds
        let rows_per_slice = (self.yend - self.ybegin) as usize;
        let remaining_in_current = (self.yend - self.y) as usize;
        let remaining_slices = (self.zend - self.z - 1).max(0) as usize;
        let remaining = remaining_in_current + remaining_slices * rows_per_slice;
        (remaining, Some(remaining))
    }
}

/// Iterator for tiles in a tiled image.
pub struct TileIterator {
    tile_width: i32,
    tile_height: i32,
    tile_depth: i32,
    tx: i32,
    ty: i32,
    tz: i32,
    tx_end: i32,
    ty_end: i32,
    tz_end: i32,
    roi: Roi3D,
}

impl TileIterator {
    /// Creates a tile iterator for the given image and ROI.
    pub fn new(buf: &ImageBuf, roi: Roi3D) -> Self {
        let spec = buf.spec();
        let tile_width = spec.tile_width.max(spec.width) as i32;
        let tile_height = spec.tile_height.max(spec.height) as i32;
        let tile_depth = spec.tile_depth.max(1) as i32;

        let tx_start = (roi.xbegin - spec.x) / tile_width;
        let ty_start = (roi.ybegin - spec.y) / tile_height;
        let tz_start = (roi.zbegin - spec.z) / tile_depth;

        let tx_end = ((roi.xend - spec.x) + tile_width - 1) / tile_width;
        let ty_end = ((roi.yend - spec.y) + tile_height - 1) / tile_height;
        let tz_end = ((roi.zend - spec.z) + tile_depth - 1) / tile_depth;

        Self {
            tile_width,
            tile_height,
            tile_depth,
            tx: tx_start,
            ty: ty_start,
            tz: tz_start,
            tx_end,
            ty_end,
            tz_end,
            roi,
        }
    }

    /// Creates a tile iterator for all tiles.
    pub fn all(buf: &ImageBuf) -> Self {
        Self::new(buf, buf.roi())
    }
}

impl Iterator for TileIterator {
    /// Yields ROI for each tile.
    type Item = Roi3D;

    fn next(&mut self) -> Option<Self::Item> {
        if self.tz >= self.tz_end {
            return None;
        }

        let xbegin = self.roi.xbegin + self.tx * self.tile_width;
        let ybegin = self.roi.ybegin + self.ty * self.tile_height;
        let zbegin = self.roi.zbegin + self.tz * self.tile_depth;

        let xend = (xbegin + self.tile_width).min(self.roi.xend);
        let yend = (ybegin + self.tile_height).min(self.roi.yend);
        let zend = (zbegin + self.tile_depth).min(self.roi.zend);

        let tile_roi = Roi3D::new(
            xbegin.max(self.roi.xbegin),
            xend,
            ybegin.max(self.roi.ybegin),
            yend,
            zbegin.max(self.roi.zbegin),
            zend,
            self.roi.chbegin,
            self.roi.chend,
        );

        // Advance to next tile
        self.tx += 1;
        if self.tx >= self.tx_end {
            self.tx = 0;
            self.ty += 1;
            if self.ty >= self.ty_end {
                self.ty = 0;
                self.tz += 1;
            }
        }

        Some(tile_roi)
    }
}

/// Parallel iteration support.
#[cfg(feature = "rayon")]
pub mod parallel {
    use super::*;
    use rayon::prelude::*;

    /// Parallel pixel iterator.
    impl ImageBuf {
        /// Applies a function to each pixel in parallel.
        pub fn par_for_each_pixel<F>(&self, roi: Option<Roi3D>, f: F)
        where
            F: Fn(i32, i32, i32, &[f32]) + Sync + Send,
        {
            let roi = roi.unwrap_or_else(|| self.roi());
            let nch = self.nchannels() as usize;

            (roi.zbegin..roi.zend).into_par_iter().for_each(|z| {
                for y in roi.ybegin..roi.yend {
                    for x in roi.xbegin..roi.xend {
                        let mut pixel = vec![0.0f32; nch];
                        self.getpixel(x, y, z, &mut pixel, WrapMode::Black);
                        f(x, y, z, &pixel);
                    }
                }
            });
        }

        /// Transforms each pixel in parallel.
        pub fn par_transform<F>(&mut self, roi: Option<Roi3D>, f: F)
        where
            F: Fn(i32, i32, i32, &[f32]) -> Vec<f32> + Sync + Send,
        {
            let roi = roi.unwrap_or_else(|| self.roi());
            let nch = self.nchannels() as usize;

            // Collect results first (can't mutate while iterating in parallel)
            let results: Vec<_> = (roi.zbegin..roi.zend)
                .into_par_iter()
                .flat_map(|z| {
                    (roi.ybegin..roi.yend)
                        .flat_map(|y| {
                            (roi.xbegin..roi.xend).map(move |x| {
                                let mut pixel = vec![0.0f32; nch];
                                self.getpixel(x, y, z, &mut pixel, WrapMode::Black);
                                let result = f(x, y, z, &pixel);
                                (x, y, z, result)
                            })
                        })
                        .collect::<Vec<_>>()
                })
                .collect();

            // Apply results
            for (x, y, z, pixel) in results {
                self.setpixel(x, y, z, &pixel);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vfx_core::ImageSpec;
    use crate::imagebuf::InitializePixels;

    #[test]
    fn test_pixel_iterator() {
        let roi = Roi3D::new_2d(0, 10, 0, 10);
        let iter = PixelIterator::new(roi);

        let pixels: Vec<_> = iter.collect();
        assert_eq!(pixels.len(), 100);
        assert_eq!(pixels[0], (0, 0, 0));
        assert_eq!(pixels[99], (9, 9, 0));
    }

    #[test]
    fn test_pixel_value_iterator() {
        let spec = ImageSpec::rgba(10, 10);
        let mut buf = ImageBuf::new(spec, InitializePixels::Yes);
        buf.setpixel(5, 5, 0, &[1.0, 0.0, 0.0, 1.0]);

        let iter = PixelValueIterator::all(&buf);
        let found = iter.filter(|(x, y, _, pixel)| *x == 5 && *y == 5 && pixel[0] > 0.5).count();
        assert_eq!(found, 1);
    }

    #[test]
    fn test_scanline_iterator() {
        let roi = Roi3D::new_2d(0, 100, 0, 50);
        let iter = ScanlineIterator::new(roi);

        let scanlines: Vec<_> = iter.collect();
        assert_eq!(scanlines.len(), 50);
        assert_eq!(scanlines[0], (0, 0, 0, 100));
    }
}
