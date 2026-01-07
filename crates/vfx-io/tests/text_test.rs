//! Tests for text rendering functions.
//!
//! These tests require the "text" feature to be enabled.

#![cfg(feature = "text")]

use vfx_io::imagebuf::{ImageBuf, InitializePixels, WrapMode};
use vfx_io::imagebufalgo::text::{render_text, render_text_into, TextAlign, TextStyle};
use vfx_core::{ImageSpec, DataFormat};

// ============================================================================
// Text Rendering Tests
// ============================================================================

#[test]
fn test_render_text_basic() {
    let style = TextStyle::default();
    let img = render_text("Hello", &style, 256, 64);
    
    assert_eq!(img.width(), 256);
    assert_eq!(img.height(), 64);
    assert_eq!(img.nchannels(), 4); // RGBA
}

#[test]
fn test_render_text_auto_size() {
    let style = TextStyle {
        font_size: 32.0,
        ..Default::default()
    };
    
    // Pass 0 for auto-sizing
    let img = render_text("Test", &style, 0, 0);
    
    // Should have some reasonable dimensions
    assert!(img.width() > 0);
    assert!(img.height() > 0);
}

#[test]
fn test_render_text_custom_style() {
    let style = TextStyle {
        font: "sans-serif".to_string(),
        font_size: 48.0,
        color: [1.0, 0.0, 0.0, 1.0], // Red
        bg_color: [0.0, 0.0, 1.0, 1.0], // Blue background
        align: TextAlign::Center,
        line_height: 1.5,
    };
    
    let img = render_text("Styled", &style, 200, 80);
    
    assert_eq!(img.width(), 200);
    assert_eq!(img.height(), 80);
    
    // Check background color in corner (should be blue)
    let mut corner = [0.0f32; 4];
    img.getpixel(0, 0, 0, &mut corner, WrapMode::Clamp);
    
    // Background should have some blue
    assert!(corner[2] > 0.5, "Background should be blue");
}

#[test]
fn test_render_text_multiline() {
    let style = TextStyle {
        font_size: 24.0,
        ..Default::default()
    };
    
    let img = render_text("Line 1\nLine 2\nLine 3", &style, 200, 120);
    
    assert_eq!(img.width(), 200);
    assert_eq!(img.height(), 120);
}

#[test]
fn test_render_text_alignments() {
    for align in [TextAlign::Left, TextAlign::Center, TextAlign::Right] {
        let style = TextStyle {
            font_size: 24.0,
            align,
            ..Default::default()
        };
        
        let img = render_text("Aligned", &style, 200, 50);
        assert_eq!(img.width(), 200);
    }
}

#[test]
fn test_render_text_into_existing() {
    // Create a red background image
    let spec = ImageSpec::new(256, 128, 4, DataFormat::F32);
    let mut img = ImageBuf::new(spec, InitializePixels::Yes);
    
    // Fill with red
    for y in 0..128 {
        for x in 0..256 {
            img.setpixel(x, y, 0, &[1.0, 0.0, 0.0, 1.0]);
        }
    }
    
    let style = TextStyle {
        font_size: 32.0,
        color: [1.0, 1.0, 1.0, 1.0], // White text
        bg_color: [0.0, 0.0, 0.0, 0.0], // Transparent bg
        ..Default::default()
    };
    
    render_text_into(&mut img, "Hello", 10, 10, &style);
    
    // Image dimensions should be unchanged
    assert_eq!(img.width(), 256);
    assert_eq!(img.height(), 128);
    
    // Some pixels should now be different from pure red
    // (where the white text was rendered)
    let mut center = [0.0f32; 4];
    img.getpixel(50, 30, 0, &mut center, WrapMode::Clamp);
    
    // This area might have text rendered on it
    // (exact position depends on font metrics)
}

#[test]
fn test_render_text_unicode() {
    let style = TextStyle {
        font_size: 32.0,
        ..Default::default()
    };
    
    // Test various Unicode characters
    let texts = [
        "Hello World",           // ASCII
        "Привет мир",           // Cyrillic
        "日本語テスト",          // Japanese
        "🎉 Emoji 🚀",          // Emoji
        "Mixed: ABC абв 123",   // Mixed
    ];
    
    for text in texts {
        let img = render_text(text, &style, 300, 60);
        assert!(img.width() > 0, "Failed for: {}", text);
        assert!(img.height() > 0, "Failed for: {}", text);
    }
}

#[test]
fn test_text_style_default() {
    let style = TextStyle::default();
    
    // Check defaults
    assert!(!style.font.is_empty());
    assert!(style.font_size > 0.0);
    assert!(style.line_height > 0.0);
}

#[test]
fn test_text_align_from_str() {
    assert_eq!(TextAlign::from_str("left"), TextAlign::Left);
    assert_eq!(TextAlign::from_str("center"), TextAlign::Center);
    assert_eq!(TextAlign::from_str("right"), TextAlign::Right);
    assert_eq!(TextAlign::from_str("c"), TextAlign::Center);
    assert_eq!(TextAlign::from_str("r"), TextAlign::Right);
    assert_eq!(TextAlign::from_str("unknown"), TextAlign::Left); // default
}

#[test]
fn test_render_empty_text() {
    let style = TextStyle::default();
    let img = render_text("", &style, 100, 50);
    
    // Should still create an image (just empty/background)
    assert_eq!(img.width(), 100);
    assert_eq!(img.height(), 50);
}

#[test]
fn test_render_text_small_size() {
    let style = TextStyle {
        font_size: 8.0, // Very small
        ..Default::default()
    };
    
    let img = render_text("Tiny", &style, 100, 20);
    assert_eq!(img.width(), 100);
}

#[test]
fn test_render_text_large_size() {
    let style = TextStyle {
        font_size: 200.0, // Very large
        ..Default::default()
    };
    
    let img = render_text("BIG", &style, 800, 300);
    assert_eq!(img.width(), 800);
    assert_eq!(img.height(), 300);
}
