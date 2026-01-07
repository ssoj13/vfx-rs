//! Tests for OCIO named transform functions.

use vfx_io::imagebuf::{ImageBuf, InitializePixels};
use vfx_core::{ImageSpec, DataFormat};
use vfx_io::imagebufalgo::ocionamedtransform;

fn create_test_image(width: u32, height: u32, channels: u8, value: f32) -> ImageBuf {
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

#[test]
fn test_ocionamedtransform_without_config() {
    // Test with no config - should return unchanged or handle gracefully
    let src = create_test_image(32, 32, 3, 0.5);
    
    // This may fail if no OCIO config is available, which is expected
    let result = ocionamedtransform(&src, "unknown_transform", false, true, None, None);
    
    // Should return an image (possibly unchanged if transform not found)
    assert_eq!(result.width(), 32);
    assert_eq!(result.height(), 32);
}

#[test]
fn test_ocionamedtransform_dimensions_preserved() {
    let src = create_test_image(64, 48, 4, 0.25);
    
    let result = ocionamedtransform(&src, "any_transform", false, true, None, None);
    
    // Dimensions should always be preserved
    assert_eq!(result.width(), 64);
    assert_eq!(result.height(), 48);
    assert_eq!(result.nchannels(), 4);
}

#[test]
fn test_ocionamedtransform_inverse_flag() {
    let src = create_test_image(32, 32, 3, 0.5);
    
    // Forward transform
    let forward = ocionamedtransform(&src, "test", false, true, None, None);
    // Inverse transform
    let inverse = ocionamedtransform(&src, "test", true, true, None, None);
    
    // Both should return valid images
    assert_eq!(forward.width(), 32);
    assert_eq!(inverse.width(), 32);
}

#[test]
fn test_ocionamedtransform_unpremult_flag() {
    let src = create_test_image(32, 32, 4, 0.5);
    
    // With unpremult
    let with_unpremult = ocionamedtransform(&src, "test", false, true, None, None);
    // Without unpremult
    let without_unpremult = ocionamedtransform(&src, "test", false, false, None, None);
    
    // Both should return valid images
    assert_eq!(with_unpremult.width(), 32);
    assert_eq!(without_unpremult.width(), 32);
}
