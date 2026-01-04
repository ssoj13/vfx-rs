//! 3-dimensional lookup table.
//!
//! A 3D LUT maps RGB input to RGB output through a cube of color values.
//! Common uses include:
//! - Color grading / Look development
//! - Display calibration
//! - Color space conversion

use crate::{Interpolation, LutError, LutResult};

/// A 3-dimensional lookup table.
///
/// Stores a cube of RGB values indexed by input RGB. Standard sizes are
/// 17x17x17, 33x33x33, or 65x65x65.
///
/// # Structure
///
/// - `size^3` entries, each containing RGB output values
/// - Stored in R-major order: R varies fastest, then G, then B
/// - Trilinear or tetrahedral interpolation for lookup
///
/// # Example
///
/// ```rust
/// use vfx_lut::Lut3D;
///
/// // Create identity LUT
/// let lut = Lut3D::identity(33);
///
/// // Apply to RGB
/// let output = lut.apply([0.5, 0.3, 0.2]);
/// ```
#[derive(Debug, Clone)]
pub struct Lut3D {
    /// LUT data: [R][G][B] -> [R', G', B']
    /// Flattened as: [(r0,g0,b0), (r1,g0,b0), ..., (rN,gN,bN)]
    pub data: Vec<[f32; 3]>,
    /// Cube size (typically 17, 33, or 65)
    pub size: usize,
    /// Input domain minimum (per channel)
    pub domain_min: [f32; 3],
    /// Input domain maximum (per channel)
    pub domain_max: [f32; 3],
    /// Interpolation method
    pub interpolation: Interpolation,
}

impl Lut3D {
    /// Creates an identity (pass-through) 3D LUT.
    ///
    /// # Arguments
    ///
    /// * `size` - Cube size (e.g., 33)
    ///
    /// # Example
    ///
    /// ```rust
    /// use vfx_lut::Lut3D;
    ///
    /// let lut = Lut3D::identity(17);
    /// let result = lut.apply([0.5, 0.3, 0.8]);
    /// assert!((result[0] - 0.5).abs() < 0.1);
    /// ```
    pub fn identity(size: usize) -> Self {
        let total = size * size * size;
        let mut data = Vec::with_capacity(total);

        for b in 0..size {
            for g in 0..size {
                for r in 0..size {
                    let rf = r as f32 / (size - 1) as f32;
                    let gf = g as f32 / (size - 1) as f32;
                    let bf = b as f32 / (size - 1) as f32;
                    data.push([rf, gf, bf]);
                }
            }
        }

        Self {
            data,
            size,
            domain_min: [0.0, 0.0, 0.0],
            domain_max: [1.0, 1.0, 1.0],
            interpolation: Interpolation::Linear,
        }
    }

    /// Creates a 3D LUT from raw data.
    ///
    /// Data must be in R-major order with exactly `size^3` entries.
    pub fn from_data(data: Vec<[f32; 3]>, size: usize) -> LutResult<Self> {
        let expected = size * size * size;
        if data.len() != expected {
            return Err(LutError::InvalidSize(format!(
                "expected {} entries for size {}, got {}",
                expected, size, data.len()
            )));
        }
        Ok(Self {
            data,
            size,
            domain_min: [0.0, 0.0, 0.0],
            domain_max: [1.0, 1.0, 1.0],
            interpolation: Interpolation::Linear,
        })
    }

    /// Sets the input domain.
    pub fn with_domain(mut self, min: [f32; 3], max: [f32; 3]) -> Self {
        self.domain_min = min;
        self.domain_max = max;
        self
    }

    /// Sets the interpolation method.
    pub fn with_interpolation(mut self, interp: Interpolation) -> Self {
        self.interpolation = interp;
        self
    }

    /// Returns the total number of entries in the LUT.
    #[inline]
    pub fn entry_count(&self) -> usize {
        self.size * self.size * self.size
    }

    /// Returns the index for a given (r, g, b) grid position.
    #[inline]
    fn index(&self, r: usize, g: usize, b: usize) -> usize {
        b * self.size * self.size + g * self.size + r
    }

    /// Gets the value at grid position (r, g, b).
    #[inline]
    fn get(&self, r: usize, g: usize, b: usize) -> [f32; 3] {
        let idx = self.index(r, g, b);
        self.data[idx]
    }

    /// Applies the LUT to an RGB value.
    ///
    /// # Example
    ///
    /// ```rust
    /// use vfx_lut::Lut3D;
    ///
    /// let lut = Lut3D::identity(33);
    /// let output = lut.apply([0.5, 0.3, 0.2]);
    /// ```
    pub fn apply(&self, rgb: [f32; 3]) -> [f32; 3] {
        match self.interpolation {
            Interpolation::Nearest => self.apply_nearest(rgb),
            Interpolation::Linear => self.apply_trilinear(rgb),
            Interpolation::Tetrahedral => self.apply_tetrahedral(rgb),
        }
    }

    /// Nearest-neighbor lookup (no interpolation).
    fn apply_nearest(&self, rgb: [f32; 3]) -> [f32; 3] {
        let (r, g, b) = self.normalize(rgb);
        let ri = (r * (self.size - 1) as f32).round() as usize;
        let gi = (g * (self.size - 1) as f32).round() as usize;
        let bi = (b * (self.size - 1) as f32).round() as usize;
        self.get(
            ri.min(self.size - 1),
            gi.min(self.size - 1),
            bi.min(self.size - 1),
        )
    }

    /// Trilinear interpolation.
    fn apply_trilinear(&self, rgb: [f32; 3]) -> [f32; 3] {
        let (r, g, b) = self.normalize(rgb);
        let n = (self.size - 1) as f32;

        // Grid coordinates
        let ri = (r * n).floor() as usize;
        let gi = (g * n).floor() as usize;
        let bi = (b * n).floor() as usize;

        // Clamp to valid range
        let ri = ri.min(self.size - 2);
        let gi = gi.min(self.size - 2);
        let bi = bi.min(self.size - 2);

        // Fractional parts
        let rf = r * n - ri as f32;
        let gf = g * n - gi as f32;
        let bf = b * n - bi as f32;

        // Get the 8 corner values
        let c000 = self.get(ri, gi, bi);
        let c100 = self.get(ri + 1, gi, bi);
        let c010 = self.get(ri, gi + 1, bi);
        let c110 = self.get(ri + 1, gi + 1, bi);
        let c001 = self.get(ri, gi, bi + 1);
        let c101 = self.get(ri + 1, gi, bi + 1);
        let c011 = self.get(ri, gi + 1, bi + 1);
        let c111 = self.get(ri + 1, gi + 1, bi + 1);

        // Trilinear interpolation
        let mut result = [0.0f32; 3];
        for i in 0..3 {
            let c00 = c000[i] * (1.0 - rf) + c100[i] * rf;
            let c01 = c001[i] * (1.0 - rf) + c101[i] * rf;
            let c10 = c010[i] * (1.0 - rf) + c110[i] * rf;
            let c11 = c011[i] * (1.0 - rf) + c111[i] * rf;

            let c0 = c00 * (1.0 - gf) + c10 * gf;
            let c1 = c01 * (1.0 - gf) + c11 * gf;

            result[i] = c0 * (1.0 - bf) + c1 * bf;
        }

        result
    }

    /// Tetrahedral interpolation (higher quality).
    fn apply_tetrahedral(&self, rgb: [f32; 3]) -> [f32; 3] {
        let (r, g, b) = self.normalize(rgb);
        let n = (self.size - 1) as f32;

        // Grid coordinates
        let ri = ((r * n).floor() as usize).min(self.size - 2);
        let gi = ((g * n).floor() as usize).min(self.size - 2);
        let bi = ((b * n).floor() as usize).min(self.size - 2);

        // Fractional parts
        let rf = r * n - ri as f32;
        let gf = g * n - gi as f32;
        let bf = b * n - bi as f32;

        // Get the 8 corner values
        let c000 = self.get(ri, gi, bi);
        let c100 = self.get(ri + 1, gi, bi);
        let c010 = self.get(ri, gi + 1, bi);
        let c110 = self.get(ri + 1, gi + 1, bi);
        let c001 = self.get(ri, gi, bi + 1);
        let c101 = self.get(ri + 1, gi, bi + 1);
        let c011 = self.get(ri, gi + 1, bi + 1);
        let c111 = self.get(ri + 1, gi + 1, bi + 1);

        // Select tetrahedron and interpolate
        let mut result = [0.0f32; 3];
        
        for i in 0..3 {
            result[i] = if rf > gf {
                if gf > bf {
                    // T1: rf > gf > bf
                    c000[i] + rf * (c100[i] - c000[i]) + gf * (c110[i] - c100[i]) + bf * (c111[i] - c110[i])
                } else if rf > bf {
                    // T2: rf > bf > gf
                    c000[i] + rf * (c100[i] - c000[i]) + bf * (c101[i] - c100[i]) + gf * (c111[i] - c101[i])
                } else {
                    // T3: bf > rf > gf
                    c000[i] + bf * (c001[i] - c000[i]) + rf * (c101[i] - c001[i]) + gf * (c111[i] - c101[i])
                }
            } else if gf > bf {
                if rf > bf {
                    // T4: gf > rf > bf
                    c000[i] + gf * (c010[i] - c000[i]) + rf * (c110[i] - c010[i]) + bf * (c111[i] - c110[i])
                } else {
                    // T5: gf > bf > rf
                    c000[i] + gf * (c010[i] - c000[i]) + bf * (c011[i] - c010[i]) + rf * (c111[i] - c011[i])
                }
            } else {
                // T6: bf > gf > rf
                c000[i] + bf * (c001[i] - c000[i]) + gf * (c011[i] - c001[i]) + rf * (c111[i] - c011[i])
            };
        }

        result
    }

    /// Normalizes input RGB to [0, 1] based on domain.
    fn normalize(&self, rgb: [f32; 3]) -> (f32, f32, f32) {
        let r = (rgb[0] - self.domain_min[0]) / (self.domain_max[0] - self.domain_min[0]);
        let g = (rgb[1] - self.domain_min[1]) / (self.domain_max[1] - self.domain_min[1]);
        let b = (rgb[2] - self.domain_min[2]) / (self.domain_max[2] - self.domain_min[2]);
        (r.clamp(0.0, 1.0), g.clamp(0.0, 1.0), b.clamp(0.0, 1.0))
    }

    /// Inverts the 3D LUT using Newton-Raphson iteration.
    ///
    /// Creates a new LUT that approximates the inverse transform.
    /// Works best for monotonic (bijective) LUTs.
    ///
    /// # Example
    ///
    /// ```rust
    /// use vfx_lut::Lut3D;
    ///
    /// let lut = Lut3D::identity(17);
    /// let inv = lut.invert().unwrap();
    /// let rgb = [0.5, 0.3, 0.2];
    /// let fwd = lut.apply(rgb);
    /// let back = inv.apply(fwd);
    /// // back ≈ rgb
    /// ```
    pub fn invert(&self) -> LutResult<Self> {
        let size = self.size;
        let mut inverted = Vec::with_capacity(size * size * size);
        
        let max_iters = 30;
        let tolerance = 1e-6f32;
        let damping = 0.8f32;
        
        for bi in 0..size {
            for gi in 0..size {
                for ri in 0..size {
                    // Target: the output we want to achieve
                    let target = [
                        self.domain_min[0] + (ri as f32 / (size - 1) as f32) * (self.domain_max[0] - self.domain_min[0]),
                        self.domain_min[1] + (gi as f32 / (size - 1) as f32) * (self.domain_max[1] - self.domain_min[1]),
                        self.domain_min[2] + (bi as f32 / (size - 1) as f32) * (self.domain_max[2] - self.domain_min[2]),
                    ];
                    
                    // Initial guess: normalized grid position
                    let mut guess = [
                        ri as f32 / (size - 1) as f32,
                        gi as f32 / (size - 1) as f32,
                        bi as f32 / (size - 1) as f32,
                    ];
                    
                    // Newton-Raphson iteration
                    for _ in 0..max_iters {
                        let eval = self.apply_tetrahedral([
                            self.domain_min[0] + guess[0] * (self.domain_max[0] - self.domain_min[0]),
                            self.domain_min[1] + guess[1] * (self.domain_max[1] - self.domain_min[1]),
                            self.domain_min[2] + guess[2] * (self.domain_max[2] - self.domain_min[2]),
                        ]);
                        
                        let err = [
                            eval[0] - target[0],
                            eval[1] - target[1],
                            eval[2] - target[2],
                        ];
                        
                        let err_mag = (err[0]*err[0] + err[1]*err[1] + err[2]*err[2]).sqrt();
                        if err_mag < tolerance {
                            break;
                        }
                        
                        // Compute Jacobian numerically
                        let delta = 1e-4f32;
                        let mut jacobian = [[0.0f32; 3]; 3];
                        
                        for j in 0..3 {
                            let mut g_plus = guess;
                            g_plus[j] = (g_plus[j] + delta).min(1.0);
                            let eval_plus = self.apply_tetrahedral([
                                self.domain_min[0] + g_plus[0] * (self.domain_max[0] - self.domain_min[0]),
                                self.domain_min[1] + g_plus[1] * (self.domain_max[1] - self.domain_min[1]),
                                self.domain_min[2] + g_plus[2] * (self.domain_max[2] - self.domain_min[2]),
                            ]);
                            for i in 0..3 {
                                jacobian[i][j] = (eval_plus[i] - eval[i]) / delta;
                            }
                        }
                        
                        // Solve 3x3 system using Cramer's rule
                        let dx = solve_3x3(&jacobian, &[-err[0], -err[1], -err[2]]);
                        
                        guess[0] = (guess[0] + damping * dx[0]).clamp(0.0, 1.0);
                        guess[1] = (guess[1] + damping * dx[1]).clamp(0.0, 1.0);
                        guess[2] = (guess[2] + damping * dx[2]).clamp(0.0, 1.0);
                    }
                    
                    inverted.push([
                        self.domain_min[0] + guess[0] * (self.domain_max[0] - self.domain_min[0]),
                        self.domain_min[1] + guess[1] * (self.domain_max[1] - self.domain_min[1]),
                        self.domain_min[2] + guess[2] * (self.domain_max[2] - self.domain_min[2]),
                    ]);
                }
            }
        }
        
        Ok(Self {
            data: inverted,
            size,
            domain_min: self.domain_min,
            domain_max: self.domain_max,
            interpolation: self.interpolation,
        })
    }
}

/// Solves 3x3 linear system Ax = b using Cramer's rule.
fn solve_3x3(a: &[[f32; 3]; 3], b: &[f32; 3]) -> [f32; 3] {
    let det = a[0][0] * (a[1][1] * a[2][2] - a[1][2] * a[2][1])
            - a[0][1] * (a[1][0] * a[2][2] - a[1][2] * a[2][0])
            + a[0][2] * (a[1][0] * a[2][1] - a[1][1] * a[2][0]);
    
    if det.abs() < 1e-10 {
        return [0.0, 0.0, 0.0];
    }
    
    let det_x = b[0] * (a[1][1] * a[2][2] - a[1][2] * a[2][1])
              - a[0][1] * (b[1] * a[2][2] - a[1][2] * b[2])
              + a[0][2] * (b[1] * a[2][1] - a[1][1] * b[2]);
    
    let det_y = a[0][0] * (b[1] * a[2][2] - a[1][2] * b[2])
              - b[0] * (a[1][0] * a[2][2] - a[1][2] * a[2][0])
              + a[0][2] * (a[1][0] * b[2] - b[1] * a[2][0]);
    
    let det_z = a[0][0] * (a[1][1] * b[2] - b[1] * a[2][1])
              - a[0][1] * (a[1][0] * b[2] - b[1] * a[2][0])
              + b[0] * (a[1][0] * a[2][1] - a[1][1] * a[2][0]);
    
    [det_x / det, det_y / det, det_z / det]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity() {
        let lut = Lut3D::identity(17);
        let result = lut.apply([0.5, 0.3, 0.8]);
        assert!((result[0] - 0.5).abs() < 0.1);
        assert!((result[1] - 0.3).abs() < 0.1);
        assert!((result[2] - 0.8).abs() < 0.1);
    }

    #[test]
    fn test_corners() {
        let lut = Lut3D::identity(33);
        
        // Black
        let black = lut.apply([0.0, 0.0, 0.0]);
        assert!((black[0]).abs() < 0.01);
        
        // White
        let white = lut.apply([1.0, 1.0, 1.0]);
        assert!((white[0] - 1.0).abs() < 0.01);
        
        // Red
        let red = lut.apply([1.0, 0.0, 0.0]);
        assert!((red[0] - 1.0).abs() < 0.01);
        assert!((red[1]).abs() < 0.01);
    }

    #[test]
    fn test_tetrahedral() {
        let lut = Lut3D::identity(33).with_interpolation(Interpolation::Tetrahedral);
        let result = lut.apply([0.5, 0.3, 0.8]);
        assert!((result[0] - 0.5).abs() < 0.1);
        assert!((result[1] - 0.3).abs() < 0.1);
        assert!((result[2] - 0.8).abs() < 0.1);
    }

    #[test]
    fn test_from_data() {
        let data: Vec<[f32; 3]> = (0..8).map(|_| [0.5, 0.5, 0.5]).collect();
        let lut = Lut3D::from_data(data, 2).unwrap();
        let result = lut.apply([0.5, 0.5, 0.5]);
        assert_eq!(result, [0.5, 0.5, 0.5]);
    }
}
