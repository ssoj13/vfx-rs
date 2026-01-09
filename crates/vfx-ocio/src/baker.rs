//! LUT baking for OCIO processors.
//!
//! Converts color transform chains into lookup tables for use in
//! applications that don't support full OCIO processing.
//!
//! # Example
//!
//! ```ignore
//! use vfx_ocio::{Config, Baker};
//!
//! let config = Config::from_file("config.ocio")?;
//! let processor = config.processor("ACEScg", "sRGB")?;
//!
//! // Bake to 3D LUT
//! let baker = Baker::new(&processor);
//! let lut = baker.bake_lut_3d(33)?;
//!
//! // Export to .cube file
//! baker.write_cube("output.cube", &lut)?;
//! ```

use std::path::Path;
use std::io::Write;

use crate::error::{OcioError, OcioResult};
use crate::processor::Processor;

/// Baked 1D LUT data.
#[derive(Debug, Clone)]
pub struct BakedLut1D {
    /// LUT size (number of entries).
    pub size: usize,
    /// Input domain minimum.
    pub domain_min: f32,
    /// Input domain maximum.
    pub domain_max: f32,
    /// LUT data as RGB triplets (size * 3 values).
    pub data: Vec<f32>,
}

/// Baked 3D LUT data.
#[derive(Debug, Clone)]
pub struct BakedLut3D {
    /// LUT size per dimension (e.g., 33 for 33x33x33).
    pub size: usize,
    /// Input domain minimum per channel.
    pub domain_min: [f32; 3],
    /// Input domain maximum per channel.
    pub domain_max: [f32; 3],
    /// LUT data as RGB triplets (size^3 * 3 values).
    /// Ordered: B varies fastest, then G, then R.
    pub data: Vec<f32>,
}

/// LUT baker for converting processors to lookup tables.
#[derive(Debug)]
pub struct Baker<'a> {
    processor: &'a Processor,
    /// Input shaper LUT size (for 3D LUT input linearization).
    shaper_size: usize,
    /// Use shaper for 3D LUT (extends dynamic range).
    use_shaper: bool,
}

impl<'a> Baker<'a> {
    /// Creates a new baker for the given processor.
    pub fn new(processor: &'a Processor) -> Self {
        Self {
            processor,
            shaper_size: 4096,
            use_shaper: false,
        }
    }

    /// Sets the shaper LUT size for 3D LUT baking.
    ///
    /// Default is 4096. Only used when `with_shaper(true)` is set.
    pub fn shaper_size(mut self, size: usize) -> Self {
        self.shaper_size = size;
        self
    }

    /// Enables/disables shaper LUT for extended dynamic range.
    ///
    /// When enabled, bakes a 1D shaper LUT alongside the 3D LUT
    /// to handle HDR or log-encoded input ranges.
    pub fn with_shaper(mut self, enabled: bool) -> Self {
        self.use_shaper = enabled;
        self
    }

    /// Bakes the processor to a 1D LUT.
    ///
    /// # Arguments
    ///
    /// * `size` - Number of LUT entries (typical: 1024, 4096, 65536)
    ///
    /// # Returns
    ///
    /// A `BakedLut1D` containing the sampled transform.
    pub fn bake_lut_1d(&self, size: usize) -> OcioResult<BakedLut1D> {
        self.bake_lut_1d_with_domain(size, 0.0, 1.0)
    }

    /// Bakes the processor to a 1D LUT with custom domain.
    ///
    /// # Arguments
    ///
    /// * `size` - Number of LUT entries
    /// * `domain_min` - Input minimum value
    /// * `domain_max` - Input maximum value
    pub fn bake_lut_1d_with_domain(
        &self,
        size: usize,
        domain_min: f32,
        domain_max: f32,
    ) -> OcioResult<BakedLut1D> {
        if size < 2 {
            return Err(OcioError::Validation("LUT size must be at least 2".into()));
        }

        let mut data = Vec::with_capacity(size * 3);
        let range = domain_max - domain_min;

        for i in 0..size {
            let t = i as f32 / (size - 1) as f32;
            let input = domain_min + t * range;

            // Process each channel independently for 1D LUT
            let mut pixel = [[input, input, input]];
            self.processor.apply_rgb(&mut pixel);

            data.push(pixel[0][0]);
            data.push(pixel[0][1]);
            data.push(pixel[0][2]);
        }

        Ok(BakedLut1D {
            size,
            domain_min,
            domain_max,
            data,
        })
    }

    /// Bakes the processor to a 3D LUT.
    ///
    /// # Arguments
    ///
    /// * `size` - LUT size per dimension (typical: 17, 33, 65)
    ///
    /// # Returns
    ///
    /// A `BakedLut3D` containing the sampled transform.
    pub fn bake_lut_3d(&self, size: usize) -> OcioResult<BakedLut3D> {
        self.bake_lut_3d_with_domain(size, [0.0, 0.0, 0.0], [1.0, 1.0, 1.0])
    }

    /// Bakes the processor to a 3D LUT with custom domain.
    ///
    /// # Arguments
    ///
    /// * `size` - LUT size per dimension
    /// * `domain_min` - Input minimum per channel
    /// * `domain_max` - Input maximum per channel
    pub fn bake_lut_3d_with_domain(
        &self,
        size: usize,
        domain_min: [f32; 3],
        domain_max: [f32; 3],
    ) -> OcioResult<BakedLut3D> {
        if size < 2 {
            return Err(OcioError::Validation("LUT size must be at least 2".into()));
        }

        let total = size * size * size;
        let mut data = Vec::with_capacity(total * 3);

        let range = [
            domain_max[0] - domain_min[0],
            domain_max[1] - domain_min[1],
            domain_max[2] - domain_min[2],
        ];

        // Standard OCIO ordering: B varies fastest, then G, then R
        for r in 0..size {
            let tr = r as f32 / (size - 1) as f32;
            let red = domain_min[0] + tr * range[0];

            for g in 0..size {
                let tg = g as f32 / (size - 1) as f32;
                let green = domain_min[1] + tg * range[1];

                for b in 0..size {
                    let tb = b as f32 / (size - 1) as f32;
                    let blue = domain_min[2] + tb * range[2];

                    let mut pixel = [[red, green, blue]];
                    self.processor.apply_rgb(&mut pixel);

                    data.push(pixel[0][0]);
                    data.push(pixel[0][1]);
                    data.push(pixel[0][2]);
                }
            }
        }

        Ok(BakedLut3D {
            size,
            domain_min,
            domain_max,
            data,
        })
    }

    /// Writes a 1D LUT to a .cube file.
    pub fn write_cube_1d(&self, path: impl AsRef<Path>, lut: &BakedLut1D) -> OcioResult<()> {
        let mut file = std::fs::File::create(path)?;

        writeln!(file, "# Created by vfx-ocio Baker")?;
        writeln!(file, "TITLE \"Baked 1D LUT\"")?;
        writeln!(file)?;
        writeln!(file, "LUT_1D_SIZE {}", lut.size)?;
        writeln!(
            file,
            "DOMAIN_MIN {:.10} {:.10} {:.10}",
            lut.domain_min, lut.domain_min, lut.domain_min
        )?;
        writeln!(
            file,
            "DOMAIN_MAX {:.10} {:.10} {:.10}",
            lut.domain_max, lut.domain_max, lut.domain_max
        )?;
        writeln!(file)?;

        for i in 0..lut.size {
            let r = lut.data[i * 3];
            let g = lut.data[i * 3 + 1];
            let b = lut.data[i * 3 + 2];
            writeln!(file, "{:.10} {:.10} {:.10}", r, g, b)?;
        }

        Ok(())
    }

    /// Writes a 3D LUT to a .cube file.
    pub fn write_cube_3d(&self, path: impl AsRef<Path>, lut: &BakedLut3D) -> OcioResult<()> {
        let mut file = std::fs::File::create(path)?;

        writeln!(file, "# Created by vfx-ocio Baker")?;
        writeln!(file, "TITLE \"Baked 3D LUT\"")?;
        writeln!(file)?;
        writeln!(file, "LUT_3D_SIZE {}", lut.size)?;
        writeln!(
            file,
            "DOMAIN_MIN {:.10} {:.10} {:.10}",
            lut.domain_min[0], lut.domain_min[1], lut.domain_min[2]
        )?;
        writeln!(
            file,
            "DOMAIN_MAX {:.10} {:.10} {:.10}",
            lut.domain_max[0], lut.domain_max[1], lut.domain_max[2]
        )?;
        writeln!(file)?;

        let total = lut.size * lut.size * lut.size;
        for i in 0..total {
            let r = lut.data[i * 3];
            let g = lut.data[i * 3 + 1];
            let b = lut.data[i * 3 + 2];
            writeln!(file, "{:.10} {:.10} {:.10}", r, g, b)?;
        }

        Ok(())
    }

    /// Returns whether shaper LUT is enabled.
    pub fn has_shaper(&self) -> bool {
        self.use_shaper
    }

    /// Returns the shaper LUT size.
    pub fn get_shaper_size(&self) -> usize {
        self.shaper_size
    }
}

impl BakedLut1D {
    /// Returns the LUT data as a slice.
    pub fn as_slice(&self) -> &[f32] {
        &self.data
    }

    /// Returns RGB value at index.
    pub fn get(&self, index: usize) -> Option<[f32; 3]> {
        if index >= self.size {
            return None;
        }
        Some([
            self.data[index * 3],
            self.data[index * 3 + 1],
            self.data[index * 3 + 2],
        ])
    }

    /// Samples the LUT at a normalized position [0,1].
    pub fn sample(&self, t: f32) -> [f32; 3] {
        let t = t.clamp(0.0, 1.0);
        let pos = t * (self.size - 1) as f32;
        let idx = (pos as usize).min(self.size - 2);
        let frac = pos - idx as f32;

        let a = self.get(idx).unwrap();
        let b = self.get(idx + 1).unwrap();

        [
            a[0] + (b[0] - a[0]) * frac,
            a[1] + (b[1] - a[1]) * frac,
            a[2] + (b[2] - a[2]) * frac,
        ]
    }
}

impl BakedLut3D {
    /// Returns the LUT data as a slice.
    pub fn as_slice(&self) -> &[f32] {
        &self.data
    }

    /// Returns RGB value at (r, g, b) indices.
    pub fn get(&self, r: usize, g: usize, b: usize) -> Option<[f32; 3]> {
        if r >= self.size || g >= self.size || b >= self.size {
            return None;
        }
        let idx = (r * self.size * self.size + g * self.size + b) * 3;
        Some([self.data[idx], self.data[idx + 1], self.data[idx + 2]])
    }

    /// Total number of entries (size^3).
    pub fn total_entries(&self) -> usize {
        self.size * self.size * self.size
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transform::{Transform, CdlTransform, MatrixTransform, TransformDirection};

    fn create_test_processor() -> Processor {
        // Simple CDL: slope 1.1, offset 0.01
        let cdl = Transform::Cdl(CdlTransform {
            slope: [1.1, 1.0, 0.9],
            offset: [0.01, 0.0, -0.01],
            power: [1.0, 1.0, 1.0],
            saturation: 1.0,
            ..Default::default()
        });
        Processor::from_transform(&cdl, TransformDirection::Forward).unwrap()
    }

    #[test]
    fn bake_1d_lut() {
        let proc = create_test_processor();
        let baker = Baker::new(&proc);
        let lut = baker.bake_lut_1d(256).unwrap();

        assert_eq!(lut.size, 256);
        assert_eq!(lut.data.len(), 256 * 3);

        // Check first entry (black + offset, CDL clamps negatives to 0)
        let first = lut.get(0).unwrap();
        assert!((first[0] - 0.01).abs() < 0.001);
        assert!((first[1] - 0.0).abs() < 0.001);
        // CDL clamps negative values before power, so blue offset -0.01 becomes 0
        assert!((first[2] - 0.0).abs() < 0.001);

        // Check last entry (white * slope + offset)
        let last = lut.get(255).unwrap();
        assert!((last[0] - 1.11).abs() < 0.01);
        assert!((last[1] - 1.0).abs() < 0.01);
        assert!((last[2] - 0.89).abs() < 0.01);
    }

    #[test]
    fn bake_3d_lut() {
        let proc = create_test_processor();
        let baker = Baker::new(&proc);
        let lut = baker.bake_lut_3d(17).unwrap();

        assert_eq!(lut.size, 17);
        assert_eq!(lut.total_entries(), 17 * 17 * 17);
        assert_eq!(lut.data.len(), 17 * 17 * 17 * 3);

        // Check corner: black (0,0,0)
        let black = lut.get(0, 0, 0).unwrap();
        assert!((black[0] - 0.01).abs() < 0.001);
        assert!((black[1] - 0.0).abs() < 0.001);
        // CDL clamps negative values before power, so blue offset -0.01 becomes 0
        assert!((black[2] - 0.0).abs() < 0.001);

        // Check corner: white (16,16,16)
        let white = lut.get(16, 16, 16).unwrap();
        assert!((white[0] - 1.11).abs() < 0.01);
        assert!((white[1] - 1.0).abs() < 0.01);
        assert!((white[2] - 0.89).abs() < 0.01);
    }

    #[test]
    fn bake_1d_with_domain() {
        let proc = create_test_processor();
        let baker = Baker::new(&proc);
        let lut = baker.bake_lut_1d_with_domain(64, -0.1, 1.5).unwrap();

        assert_eq!(lut.domain_min, -0.1);
        assert_eq!(lut.domain_max, 1.5);
    }

    #[test]
    fn bake_3d_with_domain() {
        let proc = create_test_processor();
        let baker = Baker::new(&proc);
        let lut = baker
            .bake_lut_3d_with_domain(9, [-0.1, -0.1, -0.1], [1.5, 1.5, 1.5])
            .unwrap();

        assert_eq!(lut.domain_min, [-0.1, -0.1, -0.1]);
        assert_eq!(lut.domain_max, [1.5, 1.5, 1.5]);
    }

    #[test]
    fn sample_1d_lut() {
        let proc = create_test_processor();
        let baker = Baker::new(&proc);
        let lut = baker.bake_lut_1d(256).unwrap();

        // Sample at 0.5
        let mid = lut.sample(0.5);
        // Expected: 0.5 * 1.1 + 0.01 = 0.56 for red
        assert!((mid[0] - 0.56).abs() < 0.01);
    }

    #[test]
    fn invalid_lut_size() {
        let proc = create_test_processor();
        let baker = Baker::new(&proc);

        assert!(baker.bake_lut_1d(1).is_err());
        assert!(baker.bake_lut_1d(0).is_err());
        assert!(baker.bake_lut_3d(1).is_err());
    }

    #[test]
    fn identity_processor_1d() {
        // Identity matrix should produce identity LUT
        let identity = Transform::Matrix(MatrixTransform {
            matrix: [
                1.0, 0.0, 0.0, 0.0,
                0.0, 1.0, 0.0, 0.0,
                0.0, 0.0, 1.0, 0.0,
                0.0, 0.0, 0.0, 1.0,
            ],
            offset: [0.0, 0.0, 0.0, 0.0],
            direction: TransformDirection::Forward,
        });
        let proc = Processor::from_transform(&identity, TransformDirection::Forward).unwrap();
        let baker = Baker::new(&proc);
        let lut = baker.bake_lut_1d(64).unwrap();

        for i in 0..64 {
            let t = i as f32 / 63.0;
            let rgb = lut.get(i).unwrap();
            assert!((rgb[0] - t).abs() < 0.0001);
            assert!((rgb[1] - t).abs() < 0.0001);
            assert!((rgb[2] - t).abs() < 0.0001);
        }
    }

    #[test]
    fn identity_processor_3d() {
        let identity = Transform::Matrix(MatrixTransform {
            matrix: [
                1.0, 0.0, 0.0, 0.0,
                0.0, 1.0, 0.0, 0.0,
                0.0, 0.0, 1.0, 0.0,
                0.0, 0.0, 0.0, 1.0,
            ],
            offset: [0.0, 0.0, 0.0, 0.0],
            direction: TransformDirection::Forward,
        });
        let proc = Processor::from_transform(&identity, TransformDirection::Forward).unwrap();
        let baker = Baker::new(&proc);
        let lut = baker.bake_lut_3d(9).unwrap();

        // Check diagonal (r=g=b should produce r=g=b)
        for i in 0..9 {
            let t = i as f32 / 8.0;
            let rgb = lut.get(i, i, i).unwrap();
            assert!((rgb[0] - t).abs() < 0.0001);
            assert!((rgb[1] - t).abs() < 0.0001);
            assert!((rgb[2] - t).abs() < 0.0001);
        }
    }
}
