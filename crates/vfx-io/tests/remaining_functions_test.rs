//! Tests for remaining OIIO functions: rotate, fit, normalize, circular_shift,
//! reorient, resample, zover, make_kernel, color_count.

use vfx_io::imagebuf::{ImageBuf, InitializePixels};
use vfx_core::{ImageSpec, DataFormat};
use vfx_io::imagebufalgo::{
    // Geometry
    rotate, fit, circular_shift, resample, reorient, ResizeFilter,
    // Arithmetic
    normalize,
    // Composite
    zover,
    // Filters
    make_kernel, make_kernel_from_name, KernelType,
    // Stats
    color_count,
};
use std::f32::consts::PI;

// ============================================================================
// Helper functions
// ============================================================================

fn create_test_image(width: u32, height: u32, channels: u8) -> ImageBuf {
    let spec = ImageSpec::new(width, height, channels, DataFormat::F32);
    let mut buf = ImageBuf::new(spec, InitializePixels::Yes);
    
    // Fill with gradient
    let mut pixel = vec![0.0f32; channels as usize];
    for y in 0..height as i32 {
        for x in 0..width as i32 {
            let u = x as f32 / width as f32;
            let v = y as f32 / height as f32;
            pixel[0] = u;
            if channels > 1 { pixel[1] = v; }
            if channels > 2 { pixel[2] = 0.5; }
            if channels > 3 { pixel[3] = 1.0; }
            buf.setpixel(x, y, 0, &pixel);
        }
    }
    buf
}

fn create_solid_image(width: u32, height: u32, channels: u8, value: f32) -> ImageBuf {
    let spec = ImageSpec::new(width, height, channels, DataFormat::F32);
    let mut buf = ImageBuf::new(spec, InitializePixels::Yes);
    let pixel = vec![value; channels as usize];
    for y in 0..height as i32 {
        for x in 0..width as i32 {
            buf.setpixel(x, y, 0, &pixel);
        }
    }
    buf
}

// ============================================================================
// rotate tests
// ============================================================================

#[test]
fn test_rotate_zero_degrees() {
    let src = create_test_image(64, 64, 3);
    let result = rotate(&src, 0.0, ResizeFilter::Bilinear, None);
    
    assert_eq!(result.width(), 64);
    assert_eq!(result.height(), 64);
}

#[test]
fn test_rotate_90_degrees() {
    let src = create_test_image(64, 32, 3);
    let result = rotate(&src, PI / 2.0, ResizeFilter::Bilinear, None); // 90 degrees in radians
    
    // Result dimensions depend on implementation
    assert!(result.width() > 0);
    assert!(result.height() > 0);
}

#[test]
fn test_rotate_180_degrees() {
    let src = create_test_image(64, 64, 3);
    let result = rotate(&src, PI, ResizeFilter::Bilinear, None); // 180 degrees
    
    assert_eq!(result.width(), 64);
    assert_eq!(result.height(), 64);
}

#[test]
fn test_rotate_arbitrary_angle() {
    let src = create_test_image(64, 64, 3);
    let result = rotate(&src, PI / 4.0, ResizeFilter::Bilinear, None); // 45 degrees
    
    // Rotation creates a valid image
    assert!(result.width() > 0);
    assert!(result.height() > 0);
}

#[test]
fn test_rotate_negative_angle() {
    let src = create_test_image(64, 64, 3);
    let result = rotate(&src, -PI / 4.0, ResizeFilter::Lanczos3, None); // -45 degrees
    
    assert!(result.width() > 0);
    assert!(result.height() > 0);
}

// ============================================================================
// fit tests
// ============================================================================

#[test]
fn test_fit_aspect_preserve_landscape() {
    let src = create_test_image(200, 100, 3); // 2:1 aspect
    let result = fit(&src, 100, 100, Default::default(), None);
    
    // Should fit to 100x50 to preserve aspect
    assert_eq!(result.width(), 100);
    assert_eq!(result.height(), 50);
}

#[test]
fn test_fit_aspect_preserve_portrait() {
    let src = create_test_image(100, 200, 3); // 1:2 aspect
    let result = fit(&src, 100, 100, Default::default(), None);
    
    // Should fit to 50x100 to preserve aspect
    assert_eq!(result.width(), 50);
    assert_eq!(result.height(), 100);
}

#[test]
fn test_fit_square() {
    let src = create_test_image(100, 100, 3);
    let result = fit(&src, 50, 50, Default::default(), None);
    
    assert_eq!(result.width(), 50);
    assert_eq!(result.height(), 50);
}

#[test]
fn test_fit_upscale() {
    let src = create_test_image(50, 50, 3);
    let result = fit(&src, 100, 100, Default::default(), None);
    
    assert_eq!(result.width(), 100);
    assert_eq!(result.height(), 100);
}

// ============================================================================
// normalize tests
// ============================================================================

#[test]
fn test_normalize_identity() {
    // Create image with unit vector [1, 0, 0]
    let spec = ImageSpec::rgb(4, 4);
    let mut src = ImageBuf::new(spec.clone(), InitializePixels::No);
    let unit_vec = [1.0f32, 0.0, 0.0];
    for y in 0..4 {
        for x in 0..4 {
            src.setpixel(x, y, 0, &unit_vec);
        }
    }
    
    let result = normalize(&src, 0.0, 0.0, 1.0, None);
    
    // Unit vector should remain unchanged after normalization
    let mut pixel = [0.0f32; 3];
    result.getpixel(0, 0, 0, &mut pixel, Default::default());
    
    assert!((pixel[0] - 1.0).abs() < 1e-5);
    assert!(pixel[1].abs() < 1e-5);
    assert!(pixel[2].abs() < 1e-5);
}

#[test]
fn test_normalize_scale() {
    // Create image with unit vector, scale=2 should double output length
    let spec = ImageSpec::rgb(4, 4);
    let mut src = ImageBuf::new(spec.clone(), InitializePixels::No);
    let unit_vec = [1.0f32, 0.0, 0.0];
    for y in 0..4 {
        for x in 0..4 {
            src.setpixel(x, y, 0, &unit_vec);
        }
    }
    
    let result = normalize(&src, 0.0, 0.0, 2.0, None);
    
    let mut pixel = [0.0f32; 3];
    result.getpixel(0, 0, 0, &mut pixel, Default::default());
    
    // Unit vector scaled by 2
    assert!((pixel[0] - 2.0).abs() < 1e-5);
}

#[test]
fn test_normalize_offset() {
    let src = create_solid_image(32, 32, 3, 0.5);
    let result = normalize(&src, 0.5, 0.0, 1.0, None);
    
    let mut pixel = [0.0f32; 3];
    result.getpixel(0, 0, 0, &mut pixel, Default::default());
    
    // (0.5 - 0.5) * 1.0 + 0.0 = 0.0
    assert!(pixel[0].abs() < 1e-5);
}

// ============================================================================
// circular_shift tests
// ============================================================================

#[test]
fn test_circular_shift_zero() {
    let src = create_test_image(64, 64, 3);
    let result = circular_shift(&src, 0, 0, 0, None);
    
    let mut p1 = [0.0f32; 3];
    let mut p2 = [0.0f32; 3];
    src.getpixel(32, 32, 0, &mut p1, Default::default());
    result.getpixel(32, 32, 0, &mut p2, Default::default());
    
    for i in 0..3 {
        assert!((p1[i] - p2[i]).abs() < 1e-5);
    }
}

#[test]
fn test_circular_shift_horizontal() {
    let src = create_test_image(64, 64, 3);
    let result = circular_shift(&src, 32, 0, 0, None);
    
    // Pixel at (0,0) should now be at (32,0)
    let mut p1 = [0.0f32; 3];
    let mut p2 = [0.0f32; 3];
    src.getpixel(0, 0, 0, &mut p1, Default::default());
    result.getpixel(32, 0, 0, &mut p2, Default::default());
    
    for i in 0..3 {
        assert!((p1[i] - p2[i]).abs() < 1e-5);
    }
}

#[test]
fn test_circular_shift_vertical() {
    let src = create_test_image(64, 64, 3);
    let result = circular_shift(&src, 0, 32, 0, None);
    
    // Pixel at (0,0) should now be at (0,32)
    let mut p1 = [0.0f32; 3];
    let mut p2 = [0.0f32; 3];
    src.getpixel(0, 0, 0, &mut p1, Default::default());
    result.getpixel(0, 32, 0, &mut p2, Default::default());
    
    for i in 0..3 {
        assert!((p1[i] - p2[i]).abs() < 1e-5);
    }
}

#[test]
fn test_circular_shift_wrap() {
    let src = create_test_image(64, 64, 3);
    // Full wrap should return to original
    let result = circular_shift(&src, 64, 64, 0, None);
    
    let mut p1 = [0.0f32; 3];
    let mut p2 = [0.0f32; 3];
    src.getpixel(32, 32, 0, &mut p1, Default::default());
    result.getpixel(32, 32, 0, &mut p2, Default::default());
    
    for i in 0..3 {
        assert!((p1[i] - p2[i]).abs() < 1e-5);
    }
}

// ============================================================================
// reorient tests
// ============================================================================

#[test]
fn test_reorient_identity() {
    let src = create_test_image(64, 64, 3);
    let result = reorient(&src, 1); // 1 = normal orientation
    
    assert_eq!(result.width(), 64);
    assert_eq!(result.height(), 64);
}

#[test]
fn test_reorient_flip_horizontal() {
    let src = create_test_image(64, 64, 3);
    let result = reorient(&src, 2); // 2 = mirror horizontal
    
    assert_eq!(result.width(), 64);
    assert_eq!(result.height(), 64);
}

#[test]
fn test_reorient_rotate_180() {
    let src = create_test_image(64, 64, 3);
    let result = reorient(&src, 3); // 3 = rotate 180
    
    assert_eq!(result.width(), 64);
    assert_eq!(result.height(), 64);
}

#[test]
fn test_reorient_rotate_90_cw() {
    let src = create_test_image(64, 32, 3);
    let result = reorient(&src, 6); // 6 = rotate 90 CW
    
    assert_eq!(result.width(), 32);
    assert_eq!(result.height(), 64);
}

#[test]
fn test_reorient_rotate_90_ccw() {
    let src = create_test_image(64, 32, 3);
    let result = reorient(&src, 8); // 8 = rotate 90 CCW
    
    assert_eq!(result.width(), 32);
    assert_eq!(result.height(), 64);
}

// ============================================================================
// resample tests
// ============================================================================

#[test]
fn test_resample_half() {
    let src = create_test_image(64, 64, 3);
    let result = resample(&src, 32, 32, None);
    
    assert_eq!(result.width(), 32);
    assert_eq!(result.height(), 32);
}

#[test]
fn test_resample_double() {
    let src = create_test_image(32, 32, 3);
    let result = resample(&src, 64, 64, None);
    
    assert_eq!(result.width(), 64);
    assert_eq!(result.height(), 64);
}

#[test]
fn test_resample_non_uniform() {
    let src = create_test_image(64, 64, 3);
    let result = resample(&src, 128, 32, None);
    
    assert_eq!(result.width(), 128);
    assert_eq!(result.height(), 32);
}

#[test]
fn test_resample_preserves_values() {
    // Solid color should stay solid after resample
    let src = create_solid_image(64, 64, 3, 0.5);
    let result = resample(&src, 32, 32, None);
    
    let mut pixel = [0.0f32; 3];
    result.getpixel(16, 16, 0, &mut pixel, Default::default());
    
    assert!((pixel[0] - 0.5).abs() < 0.1);
}

// ============================================================================
// zover tests
// ============================================================================

#[test]
fn test_zover_front_closer() {
    // Create two images with Z channels (RGBAZ = 5 channels)
    let spec = ImageSpec::new(4, 4, 5, DataFormat::F32);
    
    let mut a = ImageBuf::new(spec.clone(), InitializePixels::Yes);
    let mut b = ImageBuf::new(spec.clone(), InitializePixels::Yes);
    
    // A: red, z=1.0
    let pixel_a = [1.0, 0.0, 0.0, 1.0, 1.0]; // RGBAZ
    // B: blue, z=2.0
    let pixel_b = [0.0, 0.0, 1.0, 1.0, 2.0];
    
    for y in 0..4 {
        for x in 0..4 {
            a.setpixel(x, y, 0, &pixel_a);
            b.setpixel(x, y, 0, &pixel_b);
        }
    }
    
    let result = zover(&a, &b, false, None);
    
    let mut pixel = [0.0f32; 5];
    result.getpixel(0, 0, 0, &mut pixel, Default::default());
    
    // A is closer (z=1.0 < z=2.0), so result should be red
    assert!(pixel[0] > 0.5); // R
    assert!(pixel[2] < 0.5); // B
}

#[test]
fn test_zover_back_closer() {
    let spec = ImageSpec::new(4, 4, 5, DataFormat::F32);
    
    let mut a = ImageBuf::new(spec.clone(), InitializePixels::Yes);
    let mut b = ImageBuf::new(spec.clone(), InitializePixels::Yes);
    
    // A: red, z=2.0
    let pixel_a = [1.0, 0.0, 0.0, 1.0, 2.0];
    // B: blue, z=1.0
    let pixel_b = [0.0, 0.0, 1.0, 1.0, 1.0];
    
    for y in 0..4 {
        for x in 0..4 {
            a.setpixel(x, y, 0, &pixel_a);
            b.setpixel(x, y, 0, &pixel_b);
        }
    }
    
    let result = zover(&a, &b, false, None);
    
    let mut pixel = [0.0f32; 5];
    result.getpixel(0, 0, 0, &mut pixel, Default::default());
    
    // B is closer (z=1.0 < z=2.0), so result should be blue
    assert!(pixel[0] < 0.5); // R
    assert!(pixel[2] > 0.5); // B
}

// ============================================================================
// make_kernel tests
// ============================================================================

#[test]
fn test_make_kernel_gaussian() {
    let kernel = make_kernel(KernelType::Gaussian, 5, 5, 1.0);
    
    assert_eq!(kernel.len(), 25); // 5x5
    
    // Center should be highest
    let center = kernel[12]; // 5*2 + 2
    let corner = kernel[0];
    assert!(center > corner);
    
    // Should be normalized (sum to ~1)
    let sum: f32 = kernel.iter().sum();
    assert!((sum - 1.0).abs() < 0.01);
}

#[test]
fn test_make_kernel_box() {
    let kernel = make_kernel(KernelType::Box, 3, 3, 0.0);
    
    assert_eq!(kernel.len(), 9);
    
    // All values should be equal (1/9)
    let expected = 1.0 / 9.0;
    for &v in &kernel {
        assert!((v - expected).abs() < 1e-5);
    }
}

#[test]
fn test_make_kernel_laplacian() {
    let kernel = make_kernel(KernelType::Laplacian, 3, 3, 0.0);
    
    assert_eq!(kernel.len(), 9);
    
    // Laplacian sums to zero
    let sum: f32 = kernel.iter().sum();
    assert!(sum.abs() < 0.01);
}

#[test]
fn test_make_kernel_from_name() {
    let gaussian = make_kernel_from_name("gaussian", 5, 5, 1.0);
    assert!(gaussian.is_some());
    
    let box_k = make_kernel_from_name("box", 3, 3, 0.0);
    assert!(box_k.is_some());
    
    let invalid = make_kernel_from_name("invalid_kernel_name", 3, 3, 0.0);
    assert!(invalid.is_none());
}

#[test]
fn test_make_kernel_sharpen() {
    let kernel = make_kernel(KernelType::Sharpen, 3, 3, 0.0);
    
    assert_eq!(kernel.len(), 9);
    
    // Center should be > 1
    let center = kernel[4];
    assert!(center > 1.0);
}

// ============================================================================
// color_count tests
// ============================================================================

#[test]
fn test_color_count_single_color() {
    let src = create_solid_image(10, 10, 3, 0.5);
    
    // Count occurrences of 0.5, 0.5, 0.5
    let colors = vec![0.5, 0.5, 0.5];
    let epsilon = vec![0.01, 0.01, 0.01];
    
    let counts = color_count(&src, &colors, &epsilon, None);
    
    assert_eq!(counts.len(), 1);
    assert_eq!(counts[0], 100); // 10x10 = 100 pixels
}

#[test]
fn test_color_count_multiple_colors() {
    let spec = ImageSpec::new(4, 4, 3, DataFormat::F32);
    let mut src = ImageBuf::new(spec, InitializePixels::Yes);
    
    // Set half red, half blue
    let red = [1.0, 0.0, 0.0];
    let blue = [0.0, 0.0, 1.0];
    
    for y in 0..4 {
        for x in 0..4 {
            if x < 2 {
                src.setpixel(x, y, 0, &red);
            } else {
                src.setpixel(x, y, 0, &blue);
            }
        }
    }
    
    // Count both colors
    let colors = vec![1.0, 0.0, 0.0, 0.0, 0.0, 1.0]; // red, blue
    let epsilon = vec![0.01, 0.01, 0.01, 0.01, 0.01, 0.01];
    
    let counts = color_count(&src, &colors, &epsilon, None);
    
    assert_eq!(counts.len(), 2);
    assert_eq!(counts[0], 8); // 8 red pixels
    assert_eq!(counts[1], 8); // 8 blue pixels
}

#[test]
fn test_color_count_with_epsilon() {
    let spec = ImageSpec::new(4, 4, 3, DataFormat::F32);
    let mut src = ImageBuf::new(spec, InitializePixels::Yes);
    
    // Set slightly varying values
    for y in 0..4 {
        for x in 0..4 {
            let v = 0.5 + (x as f32 * 0.01);
            src.setpixel(x, y, 0, &[v, v, v]);
        }
    }
    
    // With tight epsilon, should find fewer
    let colors = vec![0.5, 0.5, 0.5];
    let tight = vec![0.001, 0.001, 0.001];
    let loose = vec![0.1, 0.1, 0.1];
    
    let counts_tight = color_count(&src, &colors, &tight, None);
    let counts_loose = color_count(&src, &colors, &loose, None);
    
    assert!(counts_loose[0] >= counts_tight[0]);
}

#[test]
fn test_color_count_no_match() {
    let src = create_solid_image(10, 10, 3, 0.5);
    
    // Look for a color that doesn't exist
    let colors = vec![0.9, 0.9, 0.9];
    let epsilon = vec![0.01, 0.01, 0.01];
    
    let counts = color_count(&src, &colors, &epsilon, None);
    
    assert_eq!(counts[0], 0);
}
