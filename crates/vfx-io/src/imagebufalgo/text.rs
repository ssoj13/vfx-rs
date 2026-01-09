//! Text rendering for ImageBuf.
//!
//! Provides high-quality text rasterization using cosmic-text:
//! - Subpixel antialiasing
//! - Proper text shaping (HarfBuzz)
//! - Unicode support
//! - Multi-line layout
//!
//! # Example
//!
//! ```ignore
//! use vfx_io::imagebufalgo::text::{render_text, TextAlign, TextStyle};
//!
//! let style = TextStyle::new()
//!     .font_size(48.0)
//!     .color([1.0, 1.0, 1.0, 1.0])
//!     .align(TextAlign::Center);
//!
//! let img = render_text("Hello World!", &style, 512, 128);
//! ```

use cosmic_text::{
    Attrs as TextAttrs, Buffer, Color, Family, FontSystem, Metrics, Shaping, SwashCache,
};
use std::sync::Mutex;

use crate::imagebuf::{ImageBuf, InitializePixels, WrapMode};
use vfx_core::ImageSpec;

// Global font system (expensive to create, reuse across calls)
lazy_static::lazy_static! {
    static ref FONT_SYSTEM: Mutex<FontSystem> = Mutex::new(FontSystem::new());
    static ref SWASH_CACHE: Mutex<SwashCache> = Mutex::new(SwashCache::new());
}

/// Text alignment options.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextAlign {
    /// Left-aligned text.
    #[default]
    Left,
    /// Center-aligned text.
    Center,
    /// Right-aligned text.
    Right,
}

impl TextAlign {
    /// Parse alignment from string.
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "center" | "c" => TextAlign::Center,
            "right" | "r" => TextAlign::Right,
            _ => TextAlign::Left,
        }
    }

    /// Convert to string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            TextAlign::Left => "left",
            TextAlign::Center => "center",
            TextAlign::Right => "right",
        }
    }
}

/// Text rendering style options.
#[derive(Clone, Debug)]
pub struct TextStyle {
    /// Font family name or path to .ttf/.otf file.
    pub font: String,
    /// Font size in pixels.
    pub font_size: f32,
    /// Text color as RGBA [0-1].
    pub color: [f32; 4],
    /// Background color as RGBA [0-1].
    pub bg_color: [f32; 4],
    /// Text alignment.
    pub align: TextAlign,
    /// Line height multiplier (1.0 = normal).
    pub line_height: f32,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            font: "sans-serif".to_string(),
            font_size: 48.0,
            color: [1.0, 1.0, 1.0, 1.0],       // white
            bg_color: [0.0, 0.0, 0.0, 0.0],    // transparent
            align: TextAlign::Left,
            line_height: 1.2,
        }
    }
}

impl TextStyle {
    /// Create new style with defaults.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set font family or path.
    pub fn font(mut self, font: &str) -> Self {
        self.font = font.to_string();
        self
    }

    /// Set font size in pixels.
    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }

    /// Set text color as RGBA [0-1].
    pub fn color(mut self, rgba: [f32; 4]) -> Self {
        self.color = rgba;
        self
    }

    /// Set background color as RGBA [0-1].
    pub fn bg_color(mut self, rgba: [f32; 4]) -> Self {
        self.bg_color = rgba;
        self
    }

    /// Set text alignment.
    pub fn align(mut self, align: TextAlign) -> Self {
        self.align = align;
        self
    }

    /// Set line height multiplier.
    pub fn line_height(mut self, mult: f32) -> Self {
        self.line_height = mult;
        self
    }
}

/// Render text to ImageBuf.
///
/// # Arguments
/// * `text` - Text content (supports \n for newlines)
/// * `style` - Text rendering style
/// * `width` - Output width (0 = auto-size to fit text)
/// * `height` - Output height (0 = auto-size to fit text)
///
/// # Returns
/// RGBA ImageBuf with rendered text.
pub fn render_text(text: &str, style: &TextStyle, width: u32, height: u32) -> ImageBuf {
    // Lock font system
    let mut font_system = FONT_SYSTEM.lock().unwrap();
    let mut swash_cache = SWASH_CACHE.lock().unwrap();

    // Metrics: font size and line height
    let line_height = style.font_size * style.line_height;
    let metrics = Metrics::new(style.font_size, line_height);

    // Create text buffer
    let mut buffer = Buffer::new(&mut font_system, metrics);

    // Determine buffer width for layout
    let layout_width = if width > 0 {
        width as f32
    } else {
        // Auto-width: use large value, will trim later
        4096.0
    };

    buffer.set_size(&mut font_system, Some(layout_width), None);

    // Determine font family
    let family = if style.font.contains('/') || style.font.contains('\\') {
        // Path to font file
        Family::Name(&style.font)
    } else {
        // Named family
        match style.font.to_lowercase().as_str() {
            "serif" => Family::Serif,
            "monospace" | "mono" => Family::Monospace,
            "cursive" => Family::Cursive,
            "fantasy" => Family::Fantasy,
            _ => Family::SansSerif,
        }
    };

    let text_attrs = TextAttrs::new().family(family);
    buffer.set_text(&mut font_system, text, &text_attrs, Shaping::Advanced);

    // Shape and layout
    buffer.shape_until_scroll(&mut font_system, false);

    // Calculate actual text bounds
    let (text_width, text_height) = {
        let mut max_x = 0.0f32;
        let mut max_y = 0.0f32;

        for run in buffer.layout_runs() {
            for glyph in run.glyphs.iter() {
                let x = glyph.x + glyph.w;
                if x > max_x {
                    max_x = x;
                }
            }
            let y = run.line_y + line_height;
            if y > max_y {
                max_y = y;
            }
        }

        (max_x.ceil() as usize, max_y.ceil() as usize)
    };

    // Final dimensions
    let final_width = if width > 0 {
        width as usize
    } else {
        text_width.max(1)
    };

    let final_height = if height > 0 {
        height as usize
    } else {
        text_height.max(1)
    };

    // Create output ImageBuf
    let spec = ImageSpec::rgba(final_width as u32, final_height as u32);
    let mut result = ImageBuf::new(spec, InitializePixels::No);

    // Fill background using setpixel
    let bg = style.bg_color;
    for y in 0..final_height as i32 {
        for x in 0..final_width as i32 {
            result.setpixel(x, y, 0, &bg);
        }
    }

    // Text color
    let text_color = Color::rgba(
        (style.color[0] * 255.0) as u8,
        (style.color[1] * 255.0) as u8,
        (style.color[2] * 255.0) as u8,
        (style.color[3] * 255.0) as u8,
    );

    // Calculate alignment offset once
    let align_offset = match style.align {
        TextAlign::Left => 0.0,
        TextAlign::Center => (final_width as f32 - text_width as f32) / 2.0,
        TextAlign::Right => final_width as f32 - text_width as f32,
    };

    // Render glyphs
    buffer.draw(&mut font_system, &mut swash_cache, text_color, |x, y, w, h, color| {
        let px = (x as f32 + align_offset) as i32;
        let py = y;

        // Bounds check
        if px < 0 || py < 0 || px >= final_width as i32 || py >= final_height as i32 {
            return;
        }

        // Draw the glyph coverage rectangle
        for dy in 0..h as i32 {
            for dx in 0..w as i32 {
                let dest_x = px + dx;
                let dest_y = py + dy;

                if dest_x < 0 || dest_x >= final_width as i32 || 
                   dest_y < 0 || dest_y >= final_height as i32 {
                    continue;
                }

                // Source color (normalized)
                let src_r = color.r() as f32 / 255.0;
                let src_g = color.g() as f32 / 255.0;
                let src_b = color.b() as f32 / 255.0;
                let src_a = color.a() as f32 / 255.0;

                // Read destination color
                let mut dst = [0.0f32; 4];
                result.getpixel(dest_x, dest_y, 0, &mut dst, WrapMode::Clamp);

                // Alpha blend (Porter-Duff over)
                let out_a = src_a + dst[3] * (1.0 - src_a);
                if out_a > 0.0 {
                    let out = [
                        (src_r * src_a + dst[0] * dst[3] * (1.0 - src_a)) / out_a,
                        (src_g * src_a + dst[1] * dst[3] * (1.0 - src_a)) / out_a,
                        (src_b * src_a + dst[2] * dst[3] * (1.0 - src_a)) / out_a,
                        out_a,
                    ];
                    result.setpixel(dest_x, dest_y, 0, &out);
                }
            }
        }
    });

    result
}

/// Render text to existing ImageBuf (in-place).
///
/// Text is rendered on top of existing content with alpha blending.
pub fn render_text_into(
    dst: &mut ImageBuf,
    text: &str,
    x: i32,
    y: i32,
    style: &TextStyle,
) {
    // Render text to temporary buffer
    let text_buf = render_text(text, style, 0, 0);

    // Composite onto destination
    let dst_width = dst.width() as i32;
    let dst_height = dst.height() as i32;
    let src_width = text_buf.width() as i32;
    let src_height = text_buf.height() as i32;

    for sy in 0..src_height {
        let dy = y + sy;
        if dy < 0 || dy >= dst_height {
            continue;
        }

        for sx in 0..src_width {
            let dx = x + sx;
            if dx < 0 || dx >= dst_width {
                continue;
            }

            // Read source pixel
            let mut src = [0.0f32; 4];
            text_buf.getpixel(sx, sy, 0, &mut src, WrapMode::Clamp);

            if src[3] < 0.001 {
                continue; // Skip transparent pixels
            }

            // Read destination pixel
            let mut dst_pixel = [0.0f32; 4];
            dst.getpixel(dx, dy, 0, &mut dst_pixel, WrapMode::Clamp);

            // Alpha blend
            let out_a = src[3] + dst_pixel[3] * (1.0 - src[3]);
            if out_a > 0.0 {
                let out = [
                    (src[0] * src[3] + dst_pixel[0] * dst_pixel[3] * (1.0 - src[3])) / out_a,
                    (src[1] * src[3] + dst_pixel[1] * dst_pixel[3] * (1.0 - src[3])) / out_a,
                    (src[2] * src[3] + dst_pixel[2] * dst_pixel[3] * (1.0 - src[3])) / out_a,
                    out_a,
                ];
                dst.setpixel(dx, dy, 0, &out);
            }
        }
    }
}

/// Returns the bounding box size of text without rendering.
///
/// This matches OIIO's `text_size()` function.
///
/// # Arguments
///
/// * `text` - Text to measure
/// * `font_size` - Font size in pixels
/// * `font` - Optional font name (defaults to system sans-serif)
///
/// # Returns
///
/// Tuple of (width, height) in pixels.
///
/// # Example
///
/// ```ignore
/// use vfx_io::imagebufalgo::text_size;
///
/// let (width, height) = text_size("Hello World", 24.0, None);
/// println!("Text dimensions: {}x{}", width, height);
/// ```
pub fn text_size(text: &str, font_size: f32, font: Option<&str>) -> (u32, u32) {
    let mut font_system = FONT_SYSTEM.lock().unwrap();

    // Set up font attributes
    let family = match font {
        Some(name) if !name.is_empty() => Family::Name(name),
        _ => Family::SansSerif,
    };

    let attrs = TextAttrs::new().family(family);
    let metrics = Metrics::new(font_size, font_size * 1.2);

    // Create a buffer for measuring
    let mut buffer = Buffer::new(&mut font_system, metrics);
    buffer.set_size(&mut font_system, Some(10000.0), None);
    buffer.set_text(&mut font_system, text, &attrs, Shaping::Advanced);
    buffer.shape_until_scroll(&mut font_system, false);

    // Calculate bounds by iterating layout runs
    let mut max_width = 0.0f32;
    let mut line_count = 0u32;

    for run in buffer.layout_runs() {
        max_width = max_width.max(run.line_w);
        line_count = line_count.max(run.line_i as u32 + 1);
    }
    let total_height = line_count as f32 * font_size * 1.2;

    // Account for minimum size
    let width = (max_width.ceil() as u32).max(1);
    let height = (total_height.ceil() as u32).max(1);

    (width, height)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_align_parse() {
        assert_eq!(TextAlign::from_str("left"), TextAlign::Left);
        assert_eq!(TextAlign::from_str("center"), TextAlign::Center);
        assert_eq!(TextAlign::from_str("right"), TextAlign::Right);
        assert_eq!(TextAlign::from_str("CENTER"), TextAlign::Center);
        assert_eq!(TextAlign::from_str("c"), TextAlign::Center);
    }

    #[test]
    fn test_text_style_builder() {
        let style = TextStyle::new()
            .font("serif")
            .font_size(72.0)
            .color([1.0, 0.0, 0.0, 1.0])
            .align(TextAlign::Center);

        assert_eq!(style.font, "serif");
        assert!((style.font_size - 72.0).abs() < 0.01);
        assert!((style.color[0] - 1.0).abs() < 0.01);
        assert_eq!(style.align, TextAlign::Center);
    }

    #[test]
    fn test_render_text_basic() {
        let style = TextStyle::new().font_size(24.0);
        let img = render_text("Hello", &style, 0, 0);

        // Should have created an image
        assert!(img.width() > 0);
        assert!(img.height() > 0);
        assert_eq!(img.nchannels(), 4);
    }

    #[test]
    fn test_render_text_fixed_size() {
        let style = TextStyle::new().font_size(24.0);
        let img = render_text("Test", &style, 200, 100);

        assert_eq!(img.width(), 200);
        assert_eq!(img.height(), 100);
    }

    #[test]
    fn test_render_text_multiline() {
        let style = TextStyle::new().font_size(24.0);
        let single = render_text("Hello", &style, 0, 0);
        let multi = render_text("Hello\nWorld", &style, 0, 0);

        // Multi-line should be taller
        assert!(multi.height() > single.height());
    }
}
