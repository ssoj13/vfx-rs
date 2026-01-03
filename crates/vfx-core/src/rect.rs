//! Rectangle and Region of Interest (ROI) types for image operations.
//!
//! This module provides geometric primitives for defining image regions,
//! commonly used in:
//! - Cropping and padding operations
//! - Tile-based processing
//! - Region of Interest (ROI) selection
//! - Bounding box calculations
//!
//! # Overview
//!
//! - [`Rect`] - Basic rectangle with origin and dimensions
//! - [`Roi`] - Region of Interest, optionally unbounded
//!
//! # Coordinate System
//!
//! All coordinates use the standard image convention:
//! - Origin (0, 0) is at the **top-left** corner
//! - X increases to the right
//! - Y increases downward
//!
//! ```text
//! (0,0) ────────► X
//!   │
//!   │   ┌──────────┐
//!   │   │  Image   │
//!   │   │  Region  │
//!   │   └──────────┘
//!   ▼
//!   Y
//! ```
//!
//! # Usage
//!
//! ```rust
//! use vfx_core::Rect;
//!
//! // Create a rectangle at (10, 20) with size 100x50
//! let rect = Rect::new(10, 20, 100, 50);
//!
//! // Check if a point is inside
//! assert!(rect.contains(15, 25));
//! assert!(!rect.contains(5, 25));
//!
//! // Get intersection with another rectangle
//! let other = Rect::new(50, 40, 100, 50);
//! if let Some(intersection) = rect.intersect(&other) {
//!     println!("Overlap: {}x{}", intersection.width, intersection.height);
//! }
//! ```
//!
//! # Dependencies
//!
//! None (pure Rust types)
//!
//! # Used By
//!
//! - [`crate::image::Image`] - Crop, copy regions
//! - [`crate::image::ImageView`] - View into sub-region
//! - `vfx-io` - Display/data window specification

/// A rectangle defined by origin (x, y) and dimensions (width, height).
///
/// Represents a rectangular region in 2D image space. All values are
/// in pixels, with (0, 0) at the top-left corner.
///
/// # Invariants
///
/// - `width` and `height` should be > 0 for a valid rectangle
/// - A rectangle with zero width or height is considered empty
///
/// # Example
///
/// ```rust
/// use vfx_core::Rect;
///
/// let rect = Rect::new(10, 20, 100, 50);
/// assert_eq!(rect.right(), 110);
/// assert_eq!(rect.bottom(), 70);
/// assert_eq!(rect.area(), 5000);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(C)]
pub struct Rect {
    /// X coordinate of the left edge (inclusive)
    pub x: u32,
    /// Y coordinate of the top edge (inclusive)
    pub y: u32,
    /// Width in pixels
    pub width: u32,
    /// Height in pixels
    pub height: u32,
}

impl Rect {
    /// Creates a new rectangle with the given origin and dimensions.
    ///
    /// # Arguments
    ///
    /// * `x` - Left edge X coordinate
    /// * `y` - Top edge Y coordinate
    /// * `width` - Width in pixels
    /// * `height` - Height in pixels
    ///
    /// # Example
    ///
    /// ```rust
    /// use vfx_core::Rect;
    ///
    /// let rect = Rect::new(10, 20, 100, 50);
    /// ```
    #[inline]
    pub const fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Creates a rectangle from origin (0, 0) with given dimensions.
    ///
    /// Convenience constructor for full-image rectangles.
    ///
    /// # Example
    ///
    /// ```rust
    /// use vfx_core::Rect;
    ///
    /// let rect = Rect::from_size(1920, 1080);
    /// assert_eq!(rect.x, 0);
    /// assert_eq!(rect.y, 0);
    /// ```
    #[inline]
    pub const fn from_size(width: u32, height: u32) -> Self {
        Self::new(0, 0, width, height)
    }

    /// Creates a rectangle from two corner points.
    ///
    /// The points are (x1, y1) top-left and (x2, y2) bottom-right.
    /// If coordinates are swapped, they will be normalized.
    ///
    /// # Example
    ///
    /// ```rust
    /// use vfx_core::Rect;
    ///
    /// let rect = Rect::from_corners(10, 20, 110, 70);
    /// assert_eq!(rect.width, 100);
    /// assert_eq!(rect.height, 50);
    /// ```
    #[inline]
    pub fn from_corners(x1: u32, y1: u32, x2: u32, y2: u32) -> Self {
        let (min_x, max_x) = if x1 <= x2 { (x1, x2) } else { (x2, x1) };
        let (min_y, max_y) = if y1 <= y2 { (y1, y2) } else { (y2, y1) };
        Self::new(min_x, min_y, max_x - min_x, max_y - min_y)
    }

    /// Returns the X coordinate of the right edge (exclusive).
    ///
    /// This is `x + width`, representing the first column NOT in the rectangle.
    #[inline]
    pub const fn right(&self) -> u32 {
        self.x + self.width
    }

    /// Returns the Y coordinate of the bottom edge (exclusive).
    ///
    /// This is `y + height`, representing the first row NOT in the rectangle.
    #[inline]
    pub const fn bottom(&self) -> u32 {
        self.y + self.height
    }

    /// Returns the area of the rectangle in pixels.
    #[inline]
    pub const fn area(&self) -> u64 {
        self.width as u64 * self.height as u64
    }

    /// Returns `true` if the rectangle has zero area.
    ///
    /// A rectangle is empty if either dimension is zero.
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.width == 0 || self.height == 0
    }

    /// Returns `true` if the point (px, py) is inside this rectangle.
    ///
    /// The rectangle is inclusive on the left/top edges and exclusive
    /// on the right/bottom edges.
    ///
    /// # Example
    ///
    /// ```rust
    /// use vfx_core::Rect;
    ///
    /// let rect = Rect::new(10, 10, 100, 100);
    /// assert!(rect.contains(10, 10));   // Top-left corner included
    /// assert!(rect.contains(109, 109)); // Just inside
    /// assert!(!rect.contains(110, 110)); // On right/bottom edge, excluded
    /// ```
    #[inline]
    pub const fn contains(&self, px: u32, py: u32) -> bool {
        px >= self.x && px < self.right() && py >= self.y && py < self.bottom()
    }

    /// Returns `true` if this rectangle fully contains another.
    ///
    /// # Example
    ///
    /// ```rust
    /// use vfx_core::Rect;
    ///
    /// let outer = Rect::new(0, 0, 100, 100);
    /// let inner = Rect::new(10, 10, 50, 50);
    /// assert!(outer.contains_rect(&inner));
    /// ```
    #[inline]
    pub const fn contains_rect(&self, other: &Rect) -> bool {
        other.x >= self.x
            && other.y >= self.y
            && other.right() <= self.right()
            && other.bottom() <= self.bottom()
    }

    /// Returns `true` if this rectangle overlaps with another.
    ///
    /// Two rectangles overlap if they share at least one pixel.
    /// Empty rectangles never overlap.
    #[inline]
    pub const fn overlaps(&self, other: &Rect) -> bool {
        if self.is_empty() || other.is_empty() {
            return false;
        }
        self.x < other.right()
            && self.right() > other.x
            && self.y < other.bottom()
            && self.bottom() > other.y
    }

    /// Returns the intersection of this rectangle with another.
    ///
    /// Returns `None` if the rectangles don't overlap.
    ///
    /// # Example
    ///
    /// ```rust
    /// use vfx_core::Rect;
    ///
    /// let a = Rect::new(0, 0, 100, 100);
    /// let b = Rect::new(50, 50, 100, 100);
    /// let intersection = a.intersect(&b).unwrap();
    /// assert_eq!(intersection, Rect::new(50, 50, 50, 50));
    /// ```
    #[inline]
    pub fn intersect(&self, other: &Rect) -> Option<Rect> {
        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        let right = self.right().min(other.right());
        let bottom = self.bottom().min(other.bottom());

        if x < right && y < bottom {
            Some(Rect::new(x, y, right - x, bottom - y))
        } else {
            None
        }
    }

    /// Returns the bounding box that contains both rectangles.
    ///
    /// Also known as the union rectangle.
    ///
    /// # Example
    ///
    /// ```rust
    /// use vfx_core::Rect;
    ///
    /// let a = Rect::new(0, 0, 50, 50);
    /// let b = Rect::new(100, 100, 50, 50);
    /// let union = a.union(&b);
    /// assert_eq!(union, Rect::new(0, 0, 150, 150));
    /// ```
    #[inline]
    pub fn union(&self, other: &Rect) -> Rect {
        let x = self.x.min(other.x);
        let y = self.y.min(other.y);
        let right = self.right().max(other.right());
        let bottom = self.bottom().max(other.bottom());
        Rect::new(x, y, right - x, bottom - y)
    }

    /// Returns this rectangle translated by (dx, dy).
    ///
    /// # Example
    ///
    /// ```rust
    /// use vfx_core::Rect;
    ///
    /// let rect = Rect::new(10, 20, 100, 50);
    /// let moved = rect.translate(5, 10);
    /// assert_eq!(moved, Rect::new(15, 30, 100, 50));
    /// ```
    #[inline]
    pub const fn translate(&self, dx: i32, dy: i32) -> Rect {
        Rect::new(
            (self.x as i32 + dx) as u32,
            (self.y as i32 + dy) as u32,
            self.width,
            self.height,
        )
    }

    /// Returns this rectangle with inset (shrunk) edges.
    ///
    /// Positive values shrink the rectangle, negative values expand it.
    /// Returns `None` if the inset would make the rectangle invalid.
    ///
    /// # Example
    ///
    /// ```rust
    /// use vfx_core::Rect;
    ///
    /// let rect = Rect::new(0, 0, 100, 100);
    /// let inset = rect.inset(10).unwrap();
    /// assert_eq!(inset, Rect::new(10, 10, 80, 80));
    /// ```
    #[inline]
    pub fn inset(&self, amount: i32) -> Option<Rect> {
        let double = amount * 2;
        let new_width = self.width as i32 - double;
        let new_height = self.height as i32 - double;

        if new_width <= 0 || new_height <= 0 {
            return None;
        }

        Some(Rect::new(
            (self.x as i32 + amount) as u32,
            (self.y as i32 + amount) as u32,
            new_width as u32,
            new_height as u32,
        ))
    }

    /// Clamps this rectangle to fit within bounds.
    ///
    /// Returns the portion of this rectangle that fits within the given
    /// bounds, or `None` if there's no overlap.
    ///
    /// # Example
    ///
    /// ```rust
    /// use vfx_core::Rect;
    ///
    /// let rect = Rect::new(900, 500, 200, 200);
    /// let clamped = rect.clamp_to(1920, 1080).unwrap();
    /// // Rect is clamped to image bounds
    /// assert!(clamped.right() <= 1920);
    /// assert!(clamped.bottom() <= 1080);
    /// ```
    #[inline]
    pub fn clamp_to(&self, max_width: u32, max_height: u32) -> Option<Rect> {
        let bounds = Rect::from_size(max_width, max_height);
        self.intersect(&bounds)
    }

    /// Returns an iterator over all (x, y) coordinates in this rectangle.
    ///
    /// Iterates row by row, left to right, top to bottom.
    ///
    /// # Example
    ///
    /// ```rust
    /// use vfx_core::Rect;
    ///
    /// let rect = Rect::new(0, 0, 2, 2);
    /// let coords: Vec<_> = rect.iter_coords().collect();
    /// assert_eq!(coords, vec![(0, 0), (1, 0), (0, 1), (1, 1)]);
    /// ```
    #[inline]
    pub fn iter_coords(&self) -> impl Iterator<Item = (u32, u32)> + '_ {
        (self.y..self.bottom()).flat_map(move |y| (self.x..self.right()).map(move |x| (x, y)))
    }
}

impl std::fmt::Display for Rect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Rect({}, {}, {}x{})",
            self.x, self.y, self.width, self.height
        )
    }
}

/// Region of Interest - optionally unbounded rectangle.
///
/// Unlike [`Rect`], an ROI can represent "the entire image" without
/// knowing the image dimensions. This is useful for operations that
/// should apply to the full image by default.
///
/// # States
///
/// - `Roi::Full` - Represents the entire image (unbounded)
/// - `Roi::Region(Rect)` - A specific bounded region
///
/// # Example
///
/// ```rust
/// use vfx_core::{Rect, Roi};
///
/// // Process entire image
/// let roi = Roi::Full;
///
/// // Process specific region
/// let roi = Roi::Region(Rect::new(100, 100, 500, 500));
///
/// // Resolve to actual bounds
/// let actual = roi.resolve(1920, 1080);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Roi {
    /// The entire image (unbounded)
    #[default]
    Full,
    /// A specific rectangular region
    Region(Rect),
}

impl Roi {
    /// Creates an ROI covering the full image.
    #[inline]
    pub const fn full() -> Self {
        Self::Full
    }

    /// Creates an ROI from a specific rectangle.
    #[inline]
    pub const fn region(rect: Rect) -> Self {
        Self::Region(rect)
    }

    /// Creates an ROI from coordinates and dimensions.
    #[inline]
    pub const fn from_xywh(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self::Region(Rect::new(x, y, width, height))
    }

    /// Returns `true` if this ROI represents the full image.
    #[inline]
    pub const fn is_full(&self) -> bool {
        matches!(self, Self::Full)
    }

    /// Resolves this ROI to an actual [`Rect`] given image dimensions.
    ///
    /// - `Roi::Full` resolves to `Rect::from_size(width, height)`
    /// - `Roi::Region(r)` returns `r` clamped to image bounds
    ///
    /// # Example
    ///
    /// ```rust
    /// use vfx_core::{Rect, Roi};
    ///
    /// let roi = Roi::Full;
    /// assert_eq!(roi.resolve(1920, 1080), Rect::from_size(1920, 1080));
    ///
    /// let roi = Roi::from_xywh(100, 100, 500, 500);
    /// let resolved = roi.resolve(1920, 1080);
    /// assert_eq!(resolved, Rect::new(100, 100, 500, 500));
    /// ```
    #[inline]
    pub fn resolve(&self, width: u32, height: u32) -> Rect {
        match self {
            Self::Full => Rect::from_size(width, height),
            Self::Region(r) => r.clamp_to(width, height).unwrap_or(Rect::default()),
        }
    }
}

impl From<Rect> for Roi {
    fn from(rect: Rect) -> Self {
        Roi::Region(rect)
    }
}

impl std::fmt::Display for Roi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Roi::Full => write!(f, "Roi::Full"),
            Roi::Region(r) => write!(f, "Roi::{}", r),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rect_new() {
        let r = Rect::new(10, 20, 100, 50);
        assert_eq!(r.x, 10);
        assert_eq!(r.y, 20);
        assert_eq!(r.width, 100);
        assert_eq!(r.height, 50);
    }

    #[test]
    fn test_rect_edges() {
        let r = Rect::new(10, 20, 100, 50);
        assert_eq!(r.right(), 110);
        assert_eq!(r.bottom(), 70);
    }

    #[test]
    fn test_rect_area() {
        let r = Rect::new(0, 0, 100, 50);
        assert_eq!(r.area(), 5000);
    }

    #[test]
    fn test_rect_contains() {
        let r = Rect::new(10, 10, 100, 100);
        assert!(r.contains(10, 10));
        assert!(r.contains(50, 50));
        assert!(r.contains(109, 109));
        assert!(!r.contains(110, 110));
        assert!(!r.contains(5, 50));
    }

    #[test]
    fn test_rect_intersect() {
        let a = Rect::new(0, 0, 100, 100);
        let b = Rect::new(50, 50, 100, 100);
        let i = a.intersect(&b).unwrap();
        assert_eq!(i, Rect::new(50, 50, 50, 50));

        let c = Rect::new(200, 200, 50, 50);
        assert!(a.intersect(&c).is_none());
    }

    #[test]
    fn test_rect_union() {
        let a = Rect::new(0, 0, 50, 50);
        let b = Rect::new(100, 100, 50, 50);
        let u = a.union(&b);
        assert_eq!(u, Rect::new(0, 0, 150, 150));
    }

    #[test]
    fn test_rect_translate() {
        let r = Rect::new(10, 20, 100, 50);
        let t = r.translate(5, -10);
        assert_eq!(t, Rect::new(15, 10, 100, 50));
    }

    #[test]
    fn test_rect_inset() {
        let r = Rect::new(0, 0, 100, 100);
        let i = r.inset(10).unwrap();
        assert_eq!(i, Rect::new(10, 10, 80, 80));

        let small = Rect::new(0, 0, 10, 10);
        assert!(small.inset(10).is_none());
    }

    #[test]
    fn test_rect_iter_coords() {
        let r = Rect::new(0, 0, 2, 2);
        let coords: Vec<_> = r.iter_coords().collect();
        assert_eq!(coords, vec![(0, 0), (1, 0), (0, 1), (1, 1)]);
    }

    #[test]
    fn test_roi_full() {
        let roi = Roi::Full;
        assert!(roi.is_full());
        assert_eq!(roi.resolve(1920, 1080), Rect::from_size(1920, 1080));
    }

    #[test]
    fn test_roi_region() {
        let roi = Roi::from_xywh(100, 100, 500, 500);
        assert!(!roi.is_full());
        assert_eq!(roi.resolve(1920, 1080), Rect::new(100, 100, 500, 500));
    }

    #[test]
    fn test_roi_clamp() {
        let roi = Roi::from_xywh(900, 500, 2000, 2000);
        let resolved = roi.resolve(1920, 1080);
        assert_eq!(resolved.right(), 1920);
        assert_eq!(resolved.bottom(), 1080);
    }
}
