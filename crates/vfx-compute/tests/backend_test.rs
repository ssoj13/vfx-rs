//! Backend tests for vfx-compute.

use vfx_compute::{Backend, ColorProcessor, ImageProcessor, ComputeImage, describe_backends};

#[test]
fn test_cpu_backend_available() {
    assert!(Backend::Cpu.is_available());
}

#[test]
fn test_auto_backend() {
    let processor = ColorProcessor::new(Backend::Auto).unwrap();
    println!("Auto-selected backend: {}", processor.backend_name());
}

#[test]
fn test_describe_backends() {
    let desc = describe_backends();
    println!("{}", desc);
    assert!(desc.contains("CPU"));
}

#[test]
fn test_color_matrix_identity() {
    let processor = ColorProcessor::new(Backend::Cpu).unwrap();
    
    // 2x2 RGB image
    let data = vec![
        1.0, 0.0, 0.0,  // red
        0.0, 1.0, 0.0,  // green
        0.0, 0.0, 1.0,  // blue
        1.0, 1.0, 1.0,  // white
    ];
    let mut img = ComputeImage::from_f32(data.clone(), 2, 2, 3).unwrap();
    
    // Identity matrix
    let identity = [
        1.0, 0.0, 0.0, 0.0,
        0.0, 1.0, 0.0, 0.0,
        0.0, 0.0, 1.0, 0.0,
        0.0, 0.0, 0.0, 1.0,
    ];
    
    processor.apply_matrix(&mut img, &identity).unwrap();
    
    // Should be unchanged
    for (i, (a, b)) in img.data().iter().zip(data.iter()).enumerate() {
        assert!((a - b).abs() < 1e-5, "mismatch at {}: {} vs {}", i, a, b);
    }
}

#[test]
fn test_cdl_identity() {
    let processor = ColorProcessor::new(Backend::Cpu).unwrap();
    
    let data = vec![0.5, 0.5, 0.5];
    let mut img = ComputeImage::from_f32(data.clone(), 1, 1, 3).unwrap();
    
    let cdl = vfx_compute::color::Cdl::default();
    processor.apply_cdl(&mut img, &cdl).unwrap();
    
    for (i, (a, b)) in img.data().iter().zip(data.iter()).enumerate() {
        assert!((a - b).abs() < 1e-5, "mismatch at {}: {} vs {}", i, a, b);
    }
}

#[test]
fn test_exposure() {
    let processor = ColorProcessor::new(Backend::Cpu).unwrap();
    
    let mut img = ComputeImage::from_f32(vec![0.5, 0.5, 0.5], 1, 1, 3).unwrap();
    
    // +1 stop = 2x brightness
    processor.apply_exposure(&mut img, 1.0).unwrap();
    
    assert!((img.data()[0] - 1.0).abs() < 1e-5);
}

#[test]
fn test_resize_half() {
    let processor = ImageProcessor::new(Backend::Cpu).unwrap();
    
    // 4x4 image
    let data = vec![1.0; 4 * 4 * 3];
    let img = ComputeImage::from_f32(data, 4, 4, 3).unwrap();
    
    let resized = processor.resize_half(&img).unwrap();
    
    assert_eq!(resized.width, 2);
    assert_eq!(resized.height, 2);
    assert_eq!(resized.channels, 3);
}

#[test]
fn test_blur() {
    let processor = ImageProcessor::new(Backend::Cpu).unwrap();
    
    // Single white pixel in center
    let mut data = vec![0.0; 5 * 5 * 3];
    data[12 * 3] = 1.0;     // center R
    data[12 * 3 + 1] = 1.0; // center G
    data[12 * 3 + 2] = 1.0; // center B
    
    let mut img = ComputeImage::from_f32(data, 5, 5, 3).unwrap();
    processor.blur(&mut img, 1.0).unwrap();
    
    // Center should be less bright after blur
    assert!(img.data()[12 * 3] < 1.0);
    // Neighbors should have some brightness
    assert!(img.data()[11 * 3] > 0.0);
}

#[test]
fn test_lut1d() {
    let processor = ColorProcessor::new(Backend::Cpu).unwrap();
    
    // Simple gamma LUT (sqrt)
    let lut_size = 256;
    let mut lut = Vec::with_capacity(lut_size * 3);
    for i in 0..lut_size {
        let v = (i as f32 / 255.0).sqrt();
        lut.push(v); // R
        lut.push(v); // G  
        lut.push(v); // B
    }
    
    let mut img = ComputeImage::from_f32(vec![0.25, 0.25, 0.25], 1, 1, 3).unwrap();
    processor.apply_lut1d(&mut img, &lut, 3).unwrap();
    
    // sqrt(0.25) = 0.5
    assert!((img.data()[0] - 0.5).abs() < 0.01);
}

#[cfg(feature = "wgpu")]
#[test]
fn test_wgpu_backend_check() {
    let available = Backend::Wgpu.is_available();
    println!("wgpu available: {}", available);
    
    if available {
        let processor = ColorProcessor::new(Backend::Wgpu).unwrap();
        assert_eq!(processor.backend_name(), "wgpu");
    }
}
