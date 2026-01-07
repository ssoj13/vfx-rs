//! Tests for demosaic, texture (mipmaps), and fillholes functions.

use vfx_io::imagebuf::{ImageBuf, InitializePixels, WrapMode};
use vfx_io::imagebufalgo::demosaic::{demosaic, BayerPattern, DemosaicAlgorithm};
use vfx_io::imagebufalgo::texture::{make_texture, make_mip_level, mip_level_count, mip_dimensions, MipmapOptions, MipmapFilter};
use vfx_io::imagebufalgo::fillholes::{fillholes_pushpull, has_holes, count_holes, FillHolesOptions};
use vfx_core::{ImageSpec, DataFormat};

// ============================================================================
// Demosaic Tests
// ============================================================================

fn create_bayer_image(width: u32, height: u32, pattern: BayerPattern) -> ImageBuf {
    let spec = ImageSpec::new(width, height, 1, DataFormat::F32);
    let mut buf = ImageBuf::new(spec, InitializePixels::Yes);
    
    // Fill with pattern values (R=1.0, G=0.5, B=0.25)
    for y in 0..height as i32 {
        for x in 0..width as i32 {
            let color = match pattern {
                BayerPattern::RGGB => match ((x & 1), (y & 1)) {
                    (0, 0) => 1.0,   // R
                    (1, 0) => 0.5,   // G
                    (0, 1) => 0.5,   // G
                    (1, 1) => 0.25,  // B
                    _ => 0.0,
                },
                BayerPattern::BGGR => match ((x & 1), (y & 1)) {
                    (0, 0) => 0.25,  // B
                    (1, 0) => 0.5,   // G
                    (0, 1) => 0.5,   // G
                    (1, 1) => 1.0,   // R
                    _ => 0.0,
                },
                BayerPattern::GRBG => match ((x & 1), (y & 1)) {
                    (0, 0) => 0.5,   // G
                    (1, 0) => 1.0,   // R
                    (0, 1) => 0.25,  // B
                    (1, 1) => 0.5,   // G
                    _ => 0.0,
                },
                BayerPattern::GBRG => match ((x & 1), (y & 1)) {
                    (0, 0) => 0.5,   // G
                    (1, 0) => 0.25,  // B
                    (0, 1) => 1.0,   // R
                    (1, 1) => 0.5,   // G
                    _ => 0.0,
                },
            };
            buf.setpixel(x, y, 0, &[color]);
        }
    }
    buf
}

#[test]
fn test_demosaic_rggb_bilinear() {
    let bayer = create_bayer_image(8, 8, BayerPattern::RGGB);
    let rgb = demosaic(&bayer, BayerPattern::RGGB, DemosaicAlgorithm::Bilinear);
    
    assert_eq!(rgb.width(), 8);
    assert_eq!(rgb.height(), 8);
    assert_eq!(rgb.nchannels(), 3);
    
    // Check center pixel has reasonable interpolated values
    let mut pixel = [0.0f32; 3];
    rgb.getpixel(4, 4, 0, &mut pixel, WrapMode::Clamp);
    
    // All channels should have values (interpolation happened)
    assert!(pixel[0] > 0.0, "R should be > 0");
    assert!(pixel[1] > 0.0, "G should be > 0");
    assert!(pixel[2] > 0.0, "B should be > 0");
}

#[test]
fn test_demosaic_bggr_vng() {
    let bayer = create_bayer_image(16, 16, BayerPattern::BGGR);
    let rgb = demosaic(&bayer, BayerPattern::BGGR, DemosaicAlgorithm::VNG);
    
    assert_eq!(rgb.width(), 16);
    assert_eq!(rgb.height(), 16);
    assert_eq!(rgb.nchannels(), 3);
}

#[test]
fn test_demosaic_all_patterns() {
    for pattern in [BayerPattern::RGGB, BayerPattern::BGGR, BayerPattern::GRBG, BayerPattern::GBRG] {
        let bayer = create_bayer_image(8, 8, pattern);
        let rgb = demosaic(&bayer, pattern, DemosaicAlgorithm::Bilinear);
        
        assert_eq!(rgb.nchannels(), 3, "Pattern {:?} should produce RGB", pattern);
    }
}

#[test]
fn test_demosaic_small_image() {
    let bayer = create_bayer_image(4, 4, BayerPattern::RGGB);
    let rgb = demosaic(&bayer, BayerPattern::RGGB, DemosaicAlgorithm::Bilinear);
    
    assert_eq!(rgb.width(), 4);
    assert_eq!(rgb.height(), 4);
}

// ============================================================================
// Mipmap/Texture Tests
// ============================================================================

fn create_gradient_image(width: u32, height: u32, channels: u8) -> ImageBuf {
    let spec = ImageSpec::new(width, height, channels, DataFormat::F32);
    let mut buf = ImageBuf::new(spec, InitializePixels::Yes);
    
    let mut pixel = vec![0.0f32; channels as usize];
    for y in 0..height as i32 {
        for x in 0..width as i32 {
            let u = x as f32 / (width - 1).max(1) as f32;
            let v = y as f32 / (height - 1).max(1) as f32;
            for c in 0..channels as usize {
                pixel[c] = (u + v) * 0.5;
            }
            buf.setpixel(x, y, 0, &pixel);
        }
    }
    buf
}

#[test]
fn test_mip_level_count() {
    assert_eq!(mip_level_count(1, 1), 1);
    assert_eq!(mip_level_count(2, 2), 2);
    assert_eq!(mip_level_count(4, 4), 3);
    assert_eq!(mip_level_count(256, 256), 9);
    assert_eq!(mip_level_count(1024, 512), 11);
}

#[test]
fn test_mip_dimensions() {
    // 256x256 image
    assert_eq!(mip_dimensions(256, 256, 0), (256, 256));
    assert_eq!(mip_dimensions(256, 256, 1), (128, 128));
    assert_eq!(mip_dimensions(256, 256, 2), (64, 64));
    assert_eq!(mip_dimensions(256, 256, 8), (1, 1));
    
    // Non-square
    assert_eq!(mip_dimensions(256, 128, 0), (256, 128));
    assert_eq!(mip_dimensions(256, 128, 1), (128, 64));
}

#[test]
fn test_make_texture_basic() {
    let img = create_gradient_image(64, 64, 3);
    let opts = MipmapOptions::default();
    
    let mips = make_texture(&img, &opts);
    
    // 64x64 -> 7 levels: 64, 32, 16, 8, 4, 2, 1
    assert_eq!(mips.len(), 7);
    assert_eq!(mips[0].width(), 64);
    assert_eq!(mips[1].width(), 32);
    assert_eq!(mips[6].width(), 1);
}

#[test]
fn test_make_texture_non_power_of_two() {
    let img = create_gradient_image(100, 60, 4);
    let opts = MipmapOptions::default();
    
    let mips = make_texture(&img, &opts);
    
    assert!(mips.len() > 1);
    assert_eq!(mips[0].width(), 100);
    assert_eq!(mips[0].height(), 60);
    
    // Last mip should be 1x1
    let last = mips.last().unwrap();
    assert_eq!(last.width(), 1);
    assert_eq!(last.height(), 1);
}

#[test]
fn test_make_mip_level() {
    let img = create_gradient_image(128, 128, 3);
    let opts = MipmapOptions::default();
    
    let mip0 = make_mip_level(&img, 0, &opts);
    assert_eq!(mip0.width(), 128);
    
    let mip1 = make_mip_level(&img, 1, &opts);
    assert_eq!(mip1.width(), 64);
    
    let mip3 = make_mip_level(&img, 3, &opts);
    assert_eq!(mip3.width(), 16);
}

#[test]
fn test_mipmap_filters() {
    let img = create_gradient_image(32, 32, 3);
    
    for filter in [MipmapFilter::Box, MipmapFilter::Bilinear, MipmapFilter::Lanczos, MipmapFilter::Kaiser] {
        let opts = MipmapOptions {
            filter,
            ..Default::default()
        };
        let mips = make_texture(&img, &opts);
        assert!(mips.len() > 1, "Filter {:?} should produce mipmaps", filter);
    }
}

// ============================================================================
// Fillholes Tests
// ============================================================================

fn create_image_with_holes(width: u32, height: u32) -> ImageBuf {
    let spec = ImageSpec::new(width, height, 4, DataFormat::F32); // RGBA
    let mut buf = ImageBuf::new(spec, InitializePixels::Yes);
    
    // Fill with solid color, but leave some holes
    for y in 0..height as i32 {
        for x in 0..width as i32 {
            // Create a checkerboard of holes
            let is_hole = (x / 4 + y / 4) % 2 == 0;
            
            if is_hole {
                buf.setpixel(x, y, 0, &[0.0, 0.0, 0.0, 0.0]); // Hole
            } else {
                buf.setpixel(x, y, 0, &[1.0, 0.5, 0.25, 1.0]); // Valid pixel
            }
        }
    }
    buf
}

fn create_solid_image(width: u32, height: u32) -> ImageBuf {
    let spec = ImageSpec::new(width, height, 4, DataFormat::F32);
    let mut buf = ImageBuf::new(spec, InitializePixels::Yes);
    
    for y in 0..height as i32 {
        for x in 0..width as i32 {
            buf.setpixel(x, y, 0, &[1.0, 0.5, 0.25, 1.0]);
        }
    }
    buf
}

#[test]
fn test_has_holes_with_holes() {
    let img = create_image_with_holes(32, 32);
    let opts = FillHolesOptions::default();
    
    assert!(has_holes(&img, &opts), "Should detect holes");
}

#[test]
fn test_has_holes_no_holes() {
    let img = create_solid_image(32, 32);
    let opts = FillHolesOptions::default();
    
    assert!(!has_holes(&img, &opts), "Should not detect holes in solid image");
}

#[test]
fn test_count_holes() {
    let img = create_image_with_holes(16, 16);
    let opts = FillHolesOptions::default();
    
    let count = count_holes(&img, &opts);
    assert!(count > 0, "Should count some holes");
    assert!(count < 16 * 16, "Should not be all holes");
}

#[test]
fn test_fillholes_basic() {
    let img = create_image_with_holes(32, 32);
    let opts = FillHolesOptions::default();
    
    assert!(has_holes(&img, &opts), "Input should have holes");
    
    let filled = fillholes_pushpull(&img, &opts);
    
    assert_eq!(filled.width(), 32);
    assert_eq!(filled.height(), 32);
    
    // After filling, no holes should remain
    assert!(!has_holes(&filled, &opts), "Output should have no holes");
}

#[test]
fn test_fillholes_preserves_valid_pixels() {
    // Create image with single hole in center
    let spec = ImageSpec::new(8, 8, 4, DataFormat::F32);
    let mut img = ImageBuf::new(spec, InitializePixels::Yes);
    
    // Fill everything with known color
    for y in 0..8 {
        for x in 0..8 {
            img.setpixel(x, y, 0, &[0.8, 0.4, 0.2, 1.0]);
        }
    }
    // Make center a hole
    img.setpixel(4, 4, 0, &[0.0, 0.0, 0.0, 0.0]);
    
    let opts = FillHolesOptions::default();
    let filled = fillholes_pushpull(&img, &opts);
    
    // Corner should still be original color
    let mut corner = [0.0f32; 4];
    filled.getpixel(0, 0, 0, &mut corner, WrapMode::Clamp);
    
    assert!((corner[0] - 0.8).abs() < 0.01, "R should be preserved");
    assert!((corner[1] - 0.4).abs() < 0.01, "G should be preserved");
}

#[test]
fn test_fillholes_no_holes() {
    let img = create_solid_image(16, 16);
    let opts = FillHolesOptions::default();
    
    let filled = fillholes_pushpull(&img, &opts);
    
    // Should return same dimensions, unchanged
    assert_eq!(filled.width(), 16);
    assert_eq!(filled.height(), 16);
}

#[test]
fn test_fillholes_threshold() {
    let spec = ImageSpec::new(8, 8, 4, DataFormat::F32);
    let mut img = ImageBuf::new(spec, InitializePixels::Yes);
    
    // Fill with very low alpha (below threshold)
    for y in 0..8 {
        for x in 0..8 {
            img.setpixel(x, y, 0, &[1.0, 0.5, 0.25, 0.0005]); // Very low alpha
        }
    }
    
    let opts = FillHolesOptions {
        alpha_threshold: 0.001, // Pixels with alpha < 0.001 are holes
        ..Default::default()
    };
    
    // With threshold 0.001, these should be detected as holes
    assert!(has_holes(&img, &opts), "Low alpha pixels should be holes");
}
