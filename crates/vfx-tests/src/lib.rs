//! Integration tests for VFX-RS crates.
//!
//! This crate contains end-to-end tests that verify the interaction
//! between different VFX-RS crates.

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    /// Test full image processing pipeline: load -> process -> save
    #[test]
    fn test_io_roundtrip_exr() {
        use vfx_io::ImageData;

        let dir = tempdir().unwrap();
        let path = dir.path().join("test.exr");

        let width = 64u32;
        let height = 64u32;
        let channels = 4u32; // EXR typically uses RGBA
        let data: Vec<f32> = (0..width * height * channels)
            .map(|i| (i as f32) / ((width * height * channels) as f32))
            .collect();

        let image = ImageData::from_f32(width, height, channels, data.clone());

        vfx_io::write(&path, &image).expect("Failed to write EXR");
        let loaded = vfx_io::read(&path).expect("Failed to read EXR");

        assert_eq!(loaded.width, width);
        assert_eq!(loaded.height, height);
        assert_eq!(loaded.channels, channels);

        let loaded_data = loaded.to_f32();
        for (orig, load) in data.iter().zip(loaded_data.iter()) {
            assert!((orig - load).abs() < 1e-5);
        }
    }

    #[test]
    fn test_io_roundtrip_png() {
        use vfx_io::ImageData;

        let dir = tempdir().unwrap();
        let path = dir.path().join("test.png");

        let width = 32u32;
        let height = 32u32;
        let channels = 4u32;
        let data: Vec<f32> = (0..width * height * channels)
            .map(|i| (i % 256) as f32 / 255.0)
            .collect();

        let image = ImageData::from_f32(width, height, channels, data);

        vfx_io::write(&path, &image).expect("Failed to write PNG");
        let loaded = vfx_io::read(&path).expect("Failed to read PNG");

        assert_eq!(loaded.width, width);
        assert_eq!(loaded.height, height);
    }

    #[test]
    fn test_io_roundtrip_tiff() {
        use vfx_io::ImageData;

        let dir = tempdir().unwrap();
        let path = dir.path().join("test.tiff");

        let width = 32u32;
        let height = 32u32;
        let channels = 3u32;
        let data: Vec<f32> = (0..width * height * channels)
            .map(|i| (i as f32) / ((width * height * channels) as f32))
            .collect();

        let image = ImageData::from_f32(width, height, channels, data);

        vfx_io::write(&path, &image).expect("Failed to write TIFF");
        let loaded = vfx_io::read(&path).expect("Failed to read TIFF");

        assert_eq!(loaded.width, width);
        assert_eq!(loaded.height, height);
    }

    #[test]
    fn test_io_roundtrip_dpx() {
        use vfx_io::ImageData;

        let dir = tempdir().unwrap();
        let path = dir.path().join("test.dpx");

        let width = 64u32;
        let height = 64u32;
        let channels = 3u32;
        let data: Vec<f32> = (0..width * height * channels)
            .map(|i| (i as f32) / ((width * height * channels) as f32))
            .collect();

        let image = ImageData::from_f32(width, height, channels, data);

        vfx_io::write(&path, &image).expect("Failed to write DPX");
        let loaded = vfx_io::read(&path).expect("Failed to read DPX");

        assert_eq!(loaded.width, width);
        assert_eq!(loaded.height, height);
    }

    #[test]
    fn test_resize_pipeline() {
        use vfx_io::ImageData;
        use vfx_ops::resize::{resize_f32, Filter};

        let dir = tempdir().unwrap();
        let input_path = dir.path().join("input.exr");
        let output_path = dir.path().join("output.exr");

        let width = 100u32;
        let height = 100u32;
        let channels = 3u32;
        let data: Vec<f32> = vec![0.5; (width * height * channels) as usize];
        let image = ImageData::from_f32(width, height, channels, data);
        vfx_io::write(&input_path, &image).unwrap();

        let loaded = vfx_io::read(&input_path).unwrap();
        let src_data = loaded.to_f32();

        let new_width = 50usize;
        let new_height = 50usize;
        let resized = resize_f32(
            &src_data,
            loaded.width as usize,
            loaded.height as usize,
            loaded.channels as usize,
            new_width,
            new_height,
            Filter::Lanczos3,
        ).unwrap();

        let output = ImageData::from_f32(new_width as u32, new_height as u32, channels, resized);
        vfx_io::write(&output_path, &output).unwrap();

        let final_image = vfx_io::read(&output_path).unwrap();
        assert_eq!(final_image.width, 50);
        assert_eq!(final_image.height, 50);
    }

    #[test]
    fn test_color_pipeline() {
        use vfx_transfer::srgb;

        let linear = 0.5f32;
        let encoded = srgb::oetf(linear);
        let decoded = srgb::eotf(encoded);
        assert!((decoded - linear).abs() < 0.001);
    }

    #[test]
    fn test_lut_pipeline() {
        use vfx_lut::{Lut1D, Lut3D};

        let lut1d = Lut1D::gamma(256, 2.2);
        let input = 0.5f32;
        let output = lut1d.apply(input);
        let expected = input.powf(2.2);
        assert!((output - expected).abs() < 0.01);

        let lut3d = Lut3D::identity(17);
        let rgb = [0.5f32, 0.3, 0.8];
        let result = lut3d.apply(rgb);
        for i in 0..3 {
            assert!((result[i] - rgb[i]).abs() < 0.1);
        }
    }

    #[test]
    fn test_composite_pipeline() {
        use vfx_ops::composite::over;

        let width = 4usize;
        let height = 4usize;

        let fg: Vec<f32> = (0..width * height)
            .flat_map(|_| [1.0f32, 0.0, 0.0, 0.5])
            .collect();

        let bg: Vec<f32> = (0..width * height)
            .flat_map(|_| [0.0f32, 1.0, 0.0, 1.0])
            .collect();

        let result = over(&fg, &bg, width, height).unwrap();

        assert!((result[0] - 0.5).abs() < 0.1);
        assert!((result[1] - 0.5).abs() < 0.1);
    }

    #[test]
    fn test_transform_pipeline() {
        use vfx_ops::transform::{flip_h, flip_v, crop};

        let width = 4usize;
        let height = 4usize;
        let channels = 3usize;

        let data: Vec<f32> = (0..width * height * channels)
            .map(|i| i as f32 / (width * height * channels) as f32)
            .collect();

        let flipped_h = flip_h(&data, width, height, channels);
        assert_eq!(flipped_h.len(), data.len());

        let flipped_v = flip_v(&data, width, height, channels);
        assert_eq!(flipped_v.len(), data.len());

        let cropped = crop(&data, width, height, channels, 1, 1, 2, 2).unwrap();
        assert_eq!(cropped.len(), 2 * 2 * channels);
    }

    #[test]
    fn test_parallel_pipeline() {
        use vfx_ops::parallel;

        let width = 64usize;
        let height = 64usize;
        let channels = 3usize;

        let data: Vec<f32> = vec![0.5; width * height * channels];

        let blurred = parallel::box_blur(&data, width, height, channels, 3).unwrap();
        assert_eq!(blurred.len(), data.len());

        let resized = parallel::resize(
            &data, width, height, channels, 32, 32, vfx_ops::Filter::Lanczos3
        ).unwrap();
        assert_eq!(resized.len(), 32 * 32 * channels);
    }

    #[test]
    fn test_format_detection() {
        use vfx_io::Format;
        use std::path::Path;

        assert_eq!(Format::from_extension(Path::new("test.exr")), Format::Exr);
        assert_eq!(Format::from_extension(Path::new("test.png")), Format::Png);
        assert_eq!(Format::from_extension(Path::new("test.jpg")), Format::Jpeg);
        assert_eq!(Format::from_extension(Path::new("test.tiff")), Format::Tiff);
        assert_eq!(Format::from_extension(Path::new("test.dpx")), Format::Dpx);
    }

    #[test]
    fn test_math_utilities() {
        use vfx_math::{Vec3, Mat3, lerp, clamp};

        let v1 = Vec3::new(1.0, 2.0, 3.0);
        let v2 = Vec3::new(4.0, 5.0, 6.0);
        let dot = v1.dot(v2);
        assert!((dot - 32.0).abs() < 0.001);

        let m = Mat3::IDENTITY;
        let result = m * v1;
        assert!((result.x - 1.0).abs() < 0.001);

        assert!((lerp(0.0, 10.0, 0.5) - 5.0).abs() < 0.001);
        assert!((clamp(1.5, 0.0, 1.0) - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_transfer_functions() {
        use vfx_transfer::{srgb, pq, log_c};

        let linear = 0.5f32;

        let encoded_srgb = srgb::oetf(linear);
        let decoded_srgb = srgb::eotf(encoded_srgb);
        assert!((decoded_srgb - linear).abs() < 0.001);

        let encoded_pq = pq::oetf(linear);
        let decoded_pq = pq::eotf(encoded_pq);
        assert!((decoded_pq - linear).abs() < 0.01);

        let encoded_logc = log_c::encode(linear);
        let decoded_logc = log_c::decode(encoded_logc);
        assert!((decoded_logc - linear).abs() < 0.01);
    }

    #[test]
    fn test_primaries_conversion() {
        use vfx_primaries::{SRGB, DCI_P3, rgb_to_rgb_matrix};

        let srgb_to_p3 = rgb_to_rgb_matrix(&SRGB, &DCI_P3);
        let srgb_red = vfx_math::Vec3::new(1.0, 0.0, 0.0);
        let p3_red = srgb_to_p3 * srgb_red;

        assert!(p3_red.x <= 1.0 && p3_red.x >= 0.0);
    }
}
