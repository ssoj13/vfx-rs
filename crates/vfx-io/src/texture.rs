//! Texture sampling system with filtering.
//!
//! Provides high-quality texture sampling with MIP mapping,
//! bilinear/trilinear filtering, and various wrap modes.
//!
//! # Example
//!
//! ```ignore
//! use vfx_io::texture::{TextureSystem, TextureOptions, WrapMode};
//!
//! let texsys = TextureSystem::new();
//! let opts = TextureOptions {
//!     wrap_s: WrapMode::Repeat,
//!     wrap_t: WrapMode::Repeat,
//!     ..Default::default()
//! };
//! let color = texsys.sample("texture.exr", 0.5, 0.5, &opts)?;
//! ```

use std::path::Path;
use std::sync::Arc;

use crate::cache::{ImageCache, CachedImageInfo};
use crate::IoResult;

/// Texture wrap modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WrapMode {
    /// Repeat texture (tile).
    #[default]
    Repeat,
    /// Clamp to edge.
    Clamp,
    /// Return black outside [0,1].
    Black,
    /// Mirror at edges.
    Mirror,
}

/// Texture filter modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FilterMode {
    /// Nearest neighbor (point sampling).
    Nearest,
    /// Bilinear interpolation.
    #[default]
    Bilinear,
    /// Trilinear (bilinear + mip interpolation).
    Trilinear,
    /// Anisotropic filtering.
    Anisotropic,
}

/// MIP filter modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MipMode {
    /// No MIP mapping.
    None,
    /// Nearest MIP level.
    #[default]
    Nearest,
    /// Linear interpolation between MIP levels.
    Linear,
}

/// Options for texture sampling.
#[derive(Debug, Clone)]
pub struct TextureOptions {
    /// Wrap mode for S (horizontal) coordinate.
    pub wrap_s: WrapMode,
    /// Wrap mode for T (vertical) coordinate.
    pub wrap_t: WrapMode,
    /// Filter mode.
    pub filter: FilterMode,
    /// MIP filter mode.
    pub mip_filter: MipMode,
    /// Subimage index (for multi-part files).
    pub subimage: u32,
    /// Fill color for out-of-range samples.
    pub fill: [f32; 4],
    /// Maximum anisotropy (for anisotropic filter).
    pub max_anisotropy: f32,
}

impl Default for TextureOptions {
    fn default() -> Self {
        Self {
            wrap_s: WrapMode::Repeat,
            wrap_t: WrapMode::Repeat,
            filter: FilterMode::Bilinear,
            mip_filter: MipMode::Nearest,
            subimage: 0,
            fill: [0.0, 0.0, 0.0, 1.0],
            max_anisotropy: 8.0,
        }
    }
}

/// Texture system for filtered sampling.
///
/// Uses ImageCache for efficient tile-based access.
pub struct TextureSystem {
    cache: Arc<ImageCache>,
}

impl TextureSystem {
    /// Creates a new texture system with default cache.
    pub fn new() -> Self {
        Self {
            cache: Arc::new(ImageCache::default()),
        }
    }

    /// Creates a texture system with custom cache.
    pub fn with_cache(cache: Arc<ImageCache>) -> Self {
        Self { cache }
    }

    /// Returns a reference to the image cache.
    pub fn cache(&self) -> &ImageCache {
        &self.cache
    }

    /// Samples a texture at the given UV coordinates.
    ///
    /// Returns RGBA values (channels are filled with `fill` if texture has fewer).
    pub fn sample(&self, path: impl AsRef<Path>, s: f32, t: f32, opts: &TextureOptions) -> IoResult<[f32; 4]> {
        let path = path.as_ref();
        let info = self.cache.get_image_info(path)?;

        // Apply wrap modes
        let s = apply_wrap(s, opts.wrap_s);
        let t = apply_wrap(t, opts.wrap_t);

        // Check for out-of-bounds (black wrap)
        if s < 0.0 || s > 1.0 || t < 0.0 || t > 1.0 {
            return Ok(opts.fill);
        }

        match opts.filter {
            FilterMode::Nearest => self.sample_nearest(path, &info, s, t, 0, opts),
            FilterMode::Bilinear => self.sample_bilinear(path, &info, s, t, 0, opts),
            FilterMode::Trilinear | FilterMode::Anisotropic => {
                // For trilinear, we'd need derivatives; for now use bilinear at mip 0
                self.sample_bilinear(path, &info, s, t, 0, opts)
            }
        }
    }

    /// Samples with explicit derivatives for proper MIP selection.
    pub fn sample_d(&self, path: impl AsRef<Path>, s: f32, t: f32, 
                    dsdx: f32, dtdx: f32, dsdy: f32, dtdy: f32,
                    opts: &TextureOptions) -> IoResult<[f32; 4]> {
        let path = path.as_ref();
        let info = self.cache.get_image_info(path)?;

        // Apply wrap modes
        let s = apply_wrap(s, opts.wrap_s);
        let t = apply_wrap(t, opts.wrap_t);

        if s < 0.0 || s > 1.0 || t < 0.0 || t > 1.0 {
            return Ok(opts.fill);
        }

        // Compute MIP level from derivatives
        let mip_level = compute_mip_level(&info, dsdx, dtdx, dsdy, dtdy);

        match opts.filter {
            FilterMode::Nearest => self.sample_nearest(path, &info, s, t, mip_level as u32, opts),
            FilterMode::Bilinear => self.sample_bilinear(path, &info, s, t, mip_level as u32, opts),
            FilterMode::Trilinear => self.sample_trilinear(path, &info, s, t, mip_level, opts),
            FilterMode::Anisotropic => {
                self.sample_anisotropic(path, &info, s, t, dsdx, dtdx, dsdy, dtdy, opts)
            }
        }
    }

    /// Nearest neighbor sampling.
    fn sample_nearest(&self, path: &Path, info: &CachedImageInfo, s: f32, t: f32, 
                      mip: u32, opts: &TextureOptions) -> IoResult<[f32; 4]> {
        let mip = mip.min(info.mip_levels.saturating_sub(1));
        let w = info.width_at_mip(mip);
        let h = info.height_at_mip(mip);

        let x = ((s * w as f32) as u32).min(w - 1);
        let y = ((t * h as f32) as u32).min(h - 1);

        self.fetch_pixel(path, info, x, y, mip, opts)
    }

    /// Bilinear interpolation sampling.
    fn sample_bilinear(&self, path: &Path, info: &CachedImageInfo, s: f32, t: f32,
                       mip: u32, opts: &TextureOptions) -> IoResult<[f32; 4]> {
        let mip = mip.min(info.mip_levels.saturating_sub(1));
        let w = info.width_at_mip(mip) as f32;
        let h = info.height_at_mip(mip) as f32;

        // Pixel coordinates (centered)
        let px = s * w - 0.5;
        let py = t * h - 0.5;

        let x0 = px.floor() as i32;
        let y0 = py.floor() as i32;
        let x1 = x0 + 1;
        let y1 = y0 + 1;

        let fx = px - x0 as f32;
        let fy = py - y0 as f32;

        // Fetch 4 pixels with wrap handling
        let c00 = self.fetch_pixel_wrapped(path, info, x0, y0, mip, opts)?;
        let c10 = self.fetch_pixel_wrapped(path, info, x1, y0, mip, opts)?;
        let c01 = self.fetch_pixel_wrapped(path, info, x0, y1, mip, opts)?;
        let c11 = self.fetch_pixel_wrapped(path, info, x1, y1, mip, opts)?;

        // Bilinear interpolation
        let mut result = [0.0f32; 4];
        for i in 0..4 {
            let top = c00[i] * (1.0 - fx) + c10[i] * fx;
            let bot = c01[i] * (1.0 - fx) + c11[i] * fx;
            result[i] = top * (1.0 - fy) + bot * fy;
        }

        Ok(result)
    }

    /// Trilinear interpolation (bilinear + MIP blending).
    fn sample_trilinear(&self, path: &Path, info: &CachedImageInfo, s: f32, t: f32,
                        mip_f: f32, opts: &TextureOptions) -> IoResult<[f32; 4]> {
        // Floor and ceiling MIP levels
        let mip0 = mip_f.floor() as u32;
        let mip1 = (mip0 + 1).min(info.mip_levels.saturating_sub(1));

        // If at the highest level, just do bilinear
        if mip0 == mip1 || mip0 >= info.mip_levels.saturating_sub(1) {
            return self.sample_bilinear(path, info, s, t, mip0, opts);
        }

        let c0 = self.sample_bilinear(path, info, s, t, mip0, opts)?;
        let c1 = self.sample_bilinear(path, info, s, t, mip1, opts)?;

        // Blend factor is fractional part of mip level
        let blend = mip_f.fract();

        let mut result = [0.0f32; 4];
        for i in 0..4 {
            result[i] = c0[i] * (1.0 - blend) + c1[i] * blend;
        }

        Ok(result)
    }

    /// Anisotropic filtering with multiple samples along major axis.
    ///
    /// Uses EWA-like approach: samples along the major axis of the texture
    /// footprint ellipse, blended with trilinear filtering.
    fn sample_anisotropic(&self, path: &Path, info: &CachedImageInfo, s: f32, t: f32,
                          dsdx: f32, dtdx: f32, dsdy: f32, dtdy: f32,
                          opts: &TextureOptions) -> IoResult<[f32; 4]> {
        // Compute texture-space derivatives
        let dudx = dsdx * info.width as f32;
        let dvdx = dtdx * info.height as f32;
        let dudy = dsdy * info.width as f32;
        let dvdy = dtdy * info.height as f32;

        // Compute lengths of the two derivative vectors
        let len_x = (dudx * dudx + dvdx * dvdx).sqrt();
        let len_y = (dudy * dudy + dvdy * dvdy).sqrt();

        // Determine major and minor axes
        let (major_len, minor_len, major_ds, major_dt) = if len_x > len_y {
            (len_x, len_y, dsdx, dtdx)
        } else {
            (len_y, len_x, dsdy, dtdy)
        };

        // Compute anisotropic ratio (clamped to max_anisotropy)
        let aspect = (major_len / minor_len.max(0.0001)).min(opts.max_anisotropy);

        // If nearly isotropic, just use trilinear
        if aspect < 1.5 {
            let mip = compute_mip_level(info, dsdx, dtdx, dsdy, dtdy);
            return self.sample_trilinear(path, info, s, t, mip, opts);
        }

        // Number of samples along major axis (proportional to aspect ratio)
        let nsamples = ((aspect * 2.0).ceil() as usize).clamp(2, 16);

        // MIP level based on minor axis (for proper filtering along it)
        let mip = minor_len.max(1.0).log2().clamp(0.0, (info.mip_levels.saturating_sub(1)) as f32);

        // Step size along major axis in UV space
        let step_s = major_ds / nsamples as f32;
        let step_t = major_dt / nsamples as f32;

        // Accumulate samples along major axis
        let mut accum = [0.0f32; 4];
        let half = (nsamples as f32 - 1.0) / 2.0;

        for i in 0..nsamples {
            let offset = i as f32 - half;
            let sample_s = s + offset * step_s;
            let sample_t = t + offset * step_t;

            // Apply wrap modes
            let ws = apply_wrap(sample_s, opts.wrap_s);
            let wt = apply_wrap(sample_t, opts.wrap_t);

            // Skip out-of-bounds samples for black wrap
            if ws < 0.0 || ws > 1.0 || wt < 0.0 || wt > 1.0 {
                continue;
            }

            let sample = self.sample_trilinear(path, info, ws, wt, mip, opts)?;
            for c in 0..4 {
                accum[c] += sample[c];
            }
        }

        // Average the samples
        let inv_n = 1.0 / nsamples as f32;
        for c in 0..4 {
            accum[c] *= inv_n;
        }

        Ok(accum)
    }

    /// Fetches a single pixel with coordinate wrapping.
    fn fetch_pixel_wrapped(&self, path: &Path, info: &CachedImageInfo, 
                           x: i32, y: i32, mip: u32, opts: &TextureOptions) -> IoResult<[f32; 4]> {
        let w = info.width_at_mip(mip) as i32;
        let h = info.height_at_mip(mip) as i32;

        let x = wrap_coord(x, w, opts.wrap_s);
        let y = wrap_coord(y, h, opts.wrap_t);

        if x < 0 || y < 0 {
            return Ok(opts.fill);
        }

        self.fetch_pixel(path, info, x as u32, y as u32, mip, opts)
    }

    /// Fetches a single pixel from cache.
    fn fetch_pixel(&self, path: &Path, _info: &CachedImageInfo,
                   x: u32, y: u32, mip: u32, opts: &TextureOptions) -> IoResult<[f32; 4]> {
        let ts = self.cache.tile_size();
        let tile_x = x / ts;
        let tile_y = y / ts;
        let local_x = x % ts;
        let local_y = y % ts;

        let tile = self.cache.get_tile(path, opts.subimage, mip, tile_x, tile_y)?;

        let idx = ((local_y * tile.width + local_x) * tile.channels) as usize;
        let channels = tile.channels as usize;

        let mut result = opts.fill;
        for c in 0..channels.min(4) {
            if idx + c < tile.data.len() {
                result[c] = tile.data[idx + c];
            }
        }

        Ok(result)
    }

    /// Invalidates cached data for a texture.
    pub fn invalidate(&self, path: impl AsRef<Path>) {
        self.cache.invalidate(path);
    }

    /// Clears all cached texture data.
    pub fn clear(&self) {
        self.cache.clear();
    }
}

impl Default for TextureSystem {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Environment Map Sampling (OIIO-compatible)
// ============================================================================

/// Environment map layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EnvLayout {
    /// Lat-long (equirectangular) projection.
    #[default]
    LatLong,
    /// Light probe (mirror ball).
    LightProbe,
    /// Cube map (6 faces).
    CubeMap,
}

impl TextureSystem {
    /// Sample an environment map using a 3D direction vector.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to environment map texture
    /// * `dir` - Direction vector (x, y, z) - doesn't need to be normalized
    /// * `layout` - Environment map layout/projection
    /// * `opts` - Texture options
    ///
    /// # Example
    ///
    /// ```ignore
    /// use vfx_io::texture::{TextureSystem, TextureOptions, EnvLayout};
    ///
    /// let texsys = TextureSystem::new();
    /// let dir = [0.0, 1.0, 0.0]; // Looking up
    /// let color = texsys.environment("sky.exr", &dir, EnvLayout::LatLong, &TextureOptions::default())?;
    /// ```
    pub fn environment(
        &self,
        path: impl AsRef<Path>,
        dir: &[f32; 3],
        layout: EnvLayout,
        opts: &TextureOptions,
    ) -> IoResult<[f32; 4]> {
        // Normalize direction
        let len = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]).sqrt();
        if len < 1e-10 {
            return Ok(opts.fill);
        }
        let x = dir[0] / len;
        let y = dir[1] / len;
        let z = dir[2] / len;

        let (s, t) = match layout {
            EnvLayout::LatLong => {
                // Equirectangular projection
                // phi: [-pi, pi] -> [0, 1] (azimuth around Y axis)
                // theta: [0, pi] -> [0, 1] (elevation from +Y)
                let phi = z.atan2(x);
                let theta = y.acos();
                let s = (phi / std::f32::consts::PI + 1.0) * 0.5;
                let t = theta / std::f32::consts::PI;
                (s, t)
            }
            EnvLayout::LightProbe => {
                // Mirror ball projection (guard against z=-1 where r=0)
                let r = (2.0 * (1.0 + z)).sqrt().max(f32::EPSILON);
                let s = 0.5 + x / (2.0 * r);
                let t = 0.5 + y / (2.0 * r);
                (s, t)
            }
            EnvLayout::CubeMap => {
                // Simplified: project to dominant face
                let ax = x.abs();
                let ay = y.abs();
                let az = z.abs();

                let (s, t) = if ax >= ay && ax >= az {
                    // X face
                    if x > 0.0 {
                        (0.5 - z / (2.0 * ax), 0.5 - y / (2.0 * ax))
                    } else {
                        (0.5 + z / (2.0 * ax), 0.5 - y / (2.0 * ax))
                    }
                } else if ay >= ax && ay >= az {
                    // Y face
                    if y > 0.0 {
                        (0.5 + x / (2.0 * ay), 0.5 + z / (2.0 * ay))
                    } else {
                        (0.5 + x / (2.0 * ay), 0.5 - z / (2.0 * ay))
                    }
                } else {
                    // Z face
                    if z > 0.0 {
                        (0.5 + x / (2.0 * az), 0.5 - y / (2.0 * az))
                    } else {
                        (0.5 - x / (2.0 * az), 0.5 - y / (2.0 * az))
                    }
                };
                (s, t)
            }
        };

        self.sample(path, s, t, opts)
    }

    /// Sample a 3D volume texture.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to 3D texture (multi-subimage or deep image)
    /// * `s` - S coordinate [0, 1]
    /// * `t` - T coordinate [0, 1]
    /// * `r` - R coordinate [0, 1] (depth/Z)
    /// * `opts` - Texture options
    pub fn texture3d(
        &self,
        path: impl AsRef<Path>,
        s: f32,
        t: f32,
        r: f32,
        opts: &TextureOptions,
    ) -> IoResult<[f32; 4]> {
        let path = path.as_ref();
        let info = self.cache.get_image_info(path)?;

        // For 3D textures, we use subimages as depth slices
        let depth = info.subimages as f32;
        let r_clamped = r.clamp(0.0, 1.0);
        let z = r_clamped * (depth - 1.0);
        let z0 = z.floor() as u32;
        let z1 = (z0 + 1).min(info.subimages - 1);
        let fz = z - z0 as f32;

        // Sample two slices and interpolate
        let mut opts0 = opts.clone();
        opts0.subimage = z0;
        let c0 = self.sample(path, s, t, &opts0)?;

        let mut opts1 = opts.clone();
        opts1.subimage = z1;
        let c1 = self.sample(path, s, t, &opts1)?;

        // Linear interpolation between slices
        let mut result = [0.0f32; 4];
        for i in 0..4 {
            result[i] = c0[i] * (1.0 - fz) + c1[i] * fz;
        }

        Ok(result)
    }
}

// ============================================================================
// Texture Handle (OIIO-compatible)
// ============================================================================

/// Handle to a cached texture for efficient repeated sampling.
///
/// Using a handle avoids repeated path lookups and is more efficient
/// when sampling the same texture many times.
#[derive(Debug, Clone)]
pub struct TextureHandle {
    /// Path to the texture file.
    path: std::path::PathBuf,
    /// Cached image info.
    info: CachedImageInfo,
}

impl TextureHandle {
    /// Gets the texture path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Gets texture width at given mip level.
    pub fn width(&self, mip: u32) -> u32 {
        self.info.width_at_mip(mip)
    }

    /// Gets texture height at given mip level.
    pub fn height(&self, mip: u32) -> u32 {
        self.info.height_at_mip(mip)
    }

    /// Gets number of channels.
    pub fn channels(&self) -> u32 {
        self.info.channels
    }

    /// Gets number of mip levels.
    pub fn mip_levels(&self) -> u32 {
        self.info.mip_levels
    }

    /// Gets number of subimages.
    pub fn subimages(&self) -> u32 {
        self.info.subimages
    }
}

impl TextureSystem {
    /// Gets a handle to a texture for efficient repeated sampling.
    ///
    /// The handle caches image metadata and avoids repeated path lookups.
    pub fn get_handle(&self, path: impl AsRef<Path>) -> IoResult<TextureHandle> {
        let path = path.as_ref();
        let info = self.cache.get_image_info(path)?;
        Ok(TextureHandle {
            path: path.to_path_buf(),
            info,
        })
    }

    /// Samples using a texture handle.
    pub fn sample_handle(
        &self,
        handle: &TextureHandle,
        s: f32,
        t: f32,
        opts: &TextureOptions,
    ) -> IoResult<[f32; 4]> {
        let s = apply_wrap(s, opts.wrap_s);
        let t = apply_wrap(t, opts.wrap_t);

        if s < 0.0 || s > 1.0 || t < 0.0 || t > 1.0 {
            return Ok(opts.fill);
        }

        match opts.filter {
            FilterMode::Nearest => self.sample_nearest(&handle.path, &handle.info, s, t, 0, opts),
            FilterMode::Bilinear | FilterMode::Trilinear | FilterMode::Anisotropic => {
                self.sample_bilinear(&handle.path, &handle.info, s, t, 0, opts)
            }
        }
    }

    /// Samples environment map using a texture handle.
    pub fn environment_handle(
        &self,
        handle: &TextureHandle,
        dir: &[f32; 3],
        layout: EnvLayout,
        opts: &TextureOptions,
    ) -> IoResult<[f32; 4]> {
        // Normalize direction
        let len = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]).sqrt();
        if len < 1e-10 {
            return Ok(opts.fill);
        }
        let x = dir[0] / len;
        let y = dir[1] / len;
        let z = dir[2] / len;

        let (s, t) = match layout {
            EnvLayout::LatLong => {
                let phi = z.atan2(x);
                let theta = y.acos();
                let s = (phi / std::f32::consts::PI + 1.0) * 0.5;
                let t = theta / std::f32::consts::PI;
                (s, t)
            }
            EnvLayout::LightProbe => {
                // Guard against z=-1 where r=0
                let r = (2.0 * (1.0 + z)).sqrt().max(f32::EPSILON);
                let s = 0.5 + x / (2.0 * r);
                let t = 0.5 + y / (2.0 * r);
                (s, t)
            }
            EnvLayout::CubeMap => {
                let ax = x.abs();
                let ay = y.abs();
                let az = z.abs();

                if ax >= ay && ax >= az {
                    if x > 0.0 {
                        (0.5 - z / (2.0 * ax), 0.5 - y / (2.0 * ax))
                    } else {
                        (0.5 + z / (2.0 * ax), 0.5 - y / (2.0 * ax))
                    }
                } else if ay >= ax && ay >= az {
                    if y > 0.0 {
                        (0.5 + x / (2.0 * ay), 0.5 + z / (2.0 * ay))
                    } else {
                        (0.5 + x / (2.0 * ay), 0.5 - z / (2.0 * ay))
                    }
                } else if z > 0.0 {
                    (0.5 + x / (2.0 * az), 0.5 - y / (2.0 * az))
                } else {
                    (0.5 - x / (2.0 * az), 0.5 - y / (2.0 * az))
                }
            }
        };

        self.sample_handle(handle, s, t, opts)
    }
}

/// Applies wrap mode to a coordinate.
fn apply_wrap(coord: f32, mode: WrapMode) -> f32 {
    match mode {
        WrapMode::Repeat => {
            let c = coord % 1.0;
            if c < 0.0 { c + 1.0 } else { c }
        }
        WrapMode::Clamp => coord.clamp(0.0, 1.0),
        WrapMode::Black => coord, // Check bounds separately
        WrapMode::Mirror => {
            let c = coord.abs() % 2.0;
            if c > 1.0 { 2.0 - c } else { c }
        }
    }
}

/// Wraps pixel coordinate.
fn wrap_coord(coord: i32, size: i32, mode: WrapMode) -> i32 {
    match mode {
        WrapMode::Repeat => {
            let c = coord % size;
            if c < 0 { c + size } else { c }
        }
        WrapMode::Clamp => coord.clamp(0, size - 1),
        WrapMode::Black => {
            if coord < 0 || coord >= size { -1 } else { coord }
        }
        WrapMode::Mirror => {
            let c = coord.abs() % (size * 2);
            if c >= size { size * 2 - 1 - c } else { c }
        }
    }
}

/// Computes MIP level from texture derivatives.
/// Returns fractional mip level for trilinear interpolation.
fn compute_mip_level(info: &CachedImageInfo, dsdx: f32, dtdx: f32, dsdy: f32, dtdy: f32) -> f32 {
    // Compute the maximum rate of change in texture space
    let dudx = dsdx * info.width as f32;
    let dvdx = dtdx * info.height as f32;
    let dudy = dsdy * info.width as f32;
    let dvdy = dtdy * info.height as f32;

    // Length of the longer axis
    let len_x = (dudx * dudx + dvdx * dvdx).sqrt();
    let len_y = (dudy * dudy + dvdy * dvdy).sqrt();
    let max_len = len_x.max(len_y);

    // MIP level is log2 of texel footprint (fractional for trilinear)
    let level = max_len.max(1.0).log2();
    level.clamp(0.0, (info.mip_levels.saturating_sub(1)) as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_repeat() {
        assert!((apply_wrap(0.5, WrapMode::Repeat) - 0.5).abs() < 0.001);
        assert!((apply_wrap(1.5, WrapMode::Repeat) - 0.5).abs() < 0.001);
        assert!((apply_wrap(-0.25, WrapMode::Repeat) - 0.75).abs() < 0.001);
    }

    #[test]
    fn wrap_clamp() {
        assert!((apply_wrap(0.5, WrapMode::Clamp) - 0.5).abs() < 0.001);
        assert!((apply_wrap(1.5, WrapMode::Clamp) - 1.0).abs() < 0.001);
        assert!((apply_wrap(-0.5, WrapMode::Clamp) - 0.0).abs() < 0.001);
    }

    #[test]
    fn wrap_mirror() {
        assert!((apply_wrap(0.5, WrapMode::Mirror) - 0.5).abs() < 0.001);
        assert!((apply_wrap(1.5, WrapMode::Mirror) - 0.5).abs() < 0.001);
        assert!((apply_wrap(0.25, WrapMode::Mirror) - 0.25).abs() < 0.001);
    }

    #[test]
    fn wrap_coord_repeat() {
        assert_eq!(wrap_coord(5, 10, WrapMode::Repeat), 5);
        assert_eq!(wrap_coord(15, 10, WrapMode::Repeat), 5);
        assert_eq!(wrap_coord(-3, 10, WrapMode::Repeat), 7);
    }

    #[test]
    fn wrap_coord_clamp() {
        assert_eq!(wrap_coord(5, 10, WrapMode::Clamp), 5);
        assert_eq!(wrap_coord(15, 10, WrapMode::Clamp), 9);
        assert_eq!(wrap_coord(-3, 10, WrapMode::Clamp), 0);
    }

    #[test]
    fn texture_options_default() {
        let opts = TextureOptions::default();
        assert_eq!(opts.wrap_s, WrapMode::Repeat);
        assert_eq!(opts.filter, FilterMode::Bilinear);
    }

    #[test]
    fn texture_system_creation() {
        let ts = TextureSystem::new();
        assert_eq!(ts.cache().size(), 0);
    }
}
