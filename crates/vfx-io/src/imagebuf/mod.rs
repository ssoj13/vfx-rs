//! OIIO-compatible ImageBuf implementation.
//!
//! ImageBuf is an in-memory image container that supports:
//! - Multiple storage modes (local buffer, application buffer, image cache)
//! - Reading and writing various image formats
//! - Per-pixel and region-based access
//! - Interpolated pixel sampling
//! - Iterator-based traversal
//!
//! # Example
//!
//! ```ignore
//! use vfx_io::imagebuf::{ImageBuf, InitializePixels};
//! use vfx_core::ImageSpec;
//!
//! // Create a new RGBA image
//! let spec = ImageSpec::rgba(1920, 1080);
//! let mut buf = ImageBuf::new(spec, InitializePixels::Yes);
//!
//! // Set a pixel
//! buf.setpixel(100, 100, 0, &[1.0, 0.0, 0.0, 1.0]);
//!
//! // Get a pixel
//! let mut pixel = [0.0f32; 4];
//! buf.getpixel(100, 100, 0, &mut pixel);
//!
//! // Write to file
//! buf.write("output.exr", None)?;
//! ```

mod storage;
mod pixels;
mod iterators;

pub use storage::*;
pub use pixels::*;
pub use iterators::*;

use std::path::Path;
use std::sync::{Arc, RwLock};

use vfx_core::{DataFormat, ImageSpec, Roi3D};
use crate::IoResult;
use crate::cache::ImageCache;

/// Controls whether pixels are initialized when allocating an ImageBuf.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InitializePixels {
    /// Do not initialize pixels (may contain garbage).
    No,
    /// Initialize all pixels to zero/black.
    #[default]
    Yes,
}

/// Wrap mode for pixel access outside image bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WrapMode {
    /// Use the default wrap mode.
    #[default]
    Default,
    /// Return black/zero for out-of-bounds pixels.
    Black,
    /// Clamp coordinates to edge pixels.
    Clamp,
    /// Periodic/tiling wrap.
    Periodic,
    /// Mirror at boundaries.
    Mirror,
}

impl WrapMode {
    /// Apply wrap mode to get valid coordinates.
    pub fn wrap(&self, x: i32, y: i32, width: i32, height: i32) -> Option<(i32, i32)> {
        match self {
            WrapMode::Default | WrapMode::Black => {
                if x >= 0 && x < width && y >= 0 && y < height {
                    Some((x, y))
                } else {
                    None
                }
            }
            WrapMode::Clamp => {
                let x = x.clamp(0, width - 1);
                let y = y.clamp(0, height - 1);
                Some((x, y))
            }
            WrapMode::Periodic => {
                let x = x.rem_euclid(width);
                let y = y.rem_euclid(height);
                Some((x, y))
            }
            WrapMode::Mirror => {
                let x = mirror_coord(x, width);
                let y = mirror_coord(y, height);
                Some((x, y))
            }
        }
    }
}

fn mirror_coord(c: i32, size: i32) -> i32 {
    let c = c.rem_euclid(2 * size);
    if c >= size {
        2 * size - c - 1
    } else {
        c
    }
}

/// How pixels are stored in an ImageBuf.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IBStorage {
    /// Uninitialized - no image data.
    #[default]
    Uninitialized,
    /// Local buffer owned by ImageBuf.
    LocalBuffer,
    /// Application-owned buffer (ImageBuf wraps external memory).
    AppBuffer,
    /// Backed by ImageCache (lazy loading).
    ImageCache,
}

/// An in-memory image buffer compatible with OpenImageIO's ImageBuf.
///
/// ImageBuf provides:
/// - Lazy file reading (spec and pixels read on demand)
/// - Multiple storage modes
/// - Thread-safe read access
/// - Efficient pixel access with optional interpolation
/// - Integration with ImageCache for large file handling
pub struct ImageBuf {
    /// Internal state protected by RwLock for thread safety.
    inner: Arc<RwLock<ImageBufInner>>,
}

struct ImageBufInner {
    /// Image specification (dimensions, format, metadata).
    spec: ImageSpec,
    /// How pixels are stored.
    storage: IBStorage,
    /// Pixel data storage.
    pixels: PixelStorage,
    /// Source filename (if read from file).
    name: String,
    /// Subimage index for multi-image files.
    subimage: i32,
    /// MIP level for mipmapped files.
    miplevel: i32,
    /// Total number of subimages in the file.
    nsubimages: i32,
    /// Total number of MIP levels in the file.
    nmiplevels: i32,
    /// Last error message.
    error: Option<String>,
    /// Associated ImageCache (if any).
    cache: Option<Arc<ImageCache>>,
    /// Read config hints (format conversion, attributes, etc.).
    read_config: Option<ImageSpec>,
    /// Write format override.
    write_format: Option<DataFormat>,
    /// Write tile dimensions.
    write_tiles: Option<(u32, u32, u32)>,
    /// Whether spec has been read (for lazy loading).
    spec_valid: bool,
    /// Whether pixels have been read (for lazy loading).
    pixels_valid: bool,
    /// Read-only flag (for AppBuffer or ImageCache).
    read_only: bool,
}

impl Clone for ImageBuf {
    fn clone(&self) -> Self {
        let inner = self.inner.read().unwrap();
        Self {
            inner: Arc::new(RwLock::new(ImageBufInner {
                spec: inner.spec.clone(),
                storage: match inner.storage {
                    IBStorage::AppBuffer | IBStorage::ImageCache => IBStorage::LocalBuffer,
                    other => other,
                },
                pixels: inner.pixels.deep_clone(),
                name: inner.name.clone(),
                subimage: inner.subimage,
                miplevel: inner.miplevel,
                nsubimages: inner.nsubimages,
                nmiplevels: inner.nmiplevels,
                error: None,
                cache: inner.cache.clone(),
                read_config: inner.read_config.clone(),
                write_format: inner.write_format,
                write_tiles: inner.write_tiles,
                spec_valid: inner.spec_valid,
                pixels_valid: inner.pixels_valid,
                read_only: false,
            })),
        }
    }
}

impl Default for ImageBuf {
    fn default() -> Self {
        Self::new_uninit()
    }
}

impl ImageBuf {
    // =========================================================================
    // Constructors
    // =========================================================================

    /// Creates an uninitialized ImageBuf.
    pub fn new_uninit() -> Self {
        Self {
            inner: Arc::new(RwLock::new(ImageBufInner {
                spec: ImageSpec::default(),
                storage: IBStorage::Uninitialized,
                pixels: PixelStorage::Empty,
                name: String::new(),
                subimage: 0,
                miplevel: 0,
                nsubimages: 1,
                nmiplevels: 1,
                error: None,
                cache: None,
                read_config: None,
                write_format: None,
                write_tiles: None,
                spec_valid: false,
                pixels_valid: false,
                read_only: false,
            })),
        }
    }

    /// Creates an ImageBuf with allocated storage for the given spec.
    ///
    /// # Arguments
    ///
    /// * `spec` - Image specification describing dimensions and format
    /// * `zero` - Whether to initialize pixels to zero
    ///
    /// # Example
    ///
    /// ```ignore
    /// use vfx_io::imagebuf::{ImageBuf, InitializePixels};
    /// use vfx_core::ImageSpec;
    ///
    /// let spec = ImageSpec::rgba(1920, 1080);
    /// let buf = ImageBuf::new(spec, InitializePixels::Yes);
    /// ```
    pub fn new(spec: ImageSpec, zero: InitializePixels) -> Self {
        let pixels = PixelStorage::allocate(&spec, zero == InitializePixels::Yes);
        let storage = if pixels.is_empty() {
            IBStorage::Uninitialized
        } else {
            IBStorage::LocalBuffer
        };

        Self {
            inner: Arc::new(RwLock::new(ImageBufInner {
                spec,
                storage,
                pixels,
                name: String::new(),
                subimage: 0,
                miplevel: 0,
                nsubimages: 1,
                nmiplevels: 1,
                error: None,
                cache: None,
                read_config: None,
                write_format: None,
                write_tiles: None,
                spec_valid: true,
                pixels_valid: true,
                read_only: false,
            })),
        }
    }

    /// Creates a named ImageBuf with allocated storage.
    pub fn new_named(name: impl Into<String>, spec: ImageSpec, zero: InitializePixels) -> Self {
        let mut buf = Self::new(spec, zero);
        buf.set_name(name);
        buf
    }

    /// Creates an ImageBuf that wraps existing pixel data.
    ///
    /// The ImageBuf does not own the data and will not free it.
    /// The caller must ensure the data remains valid for the lifetime of the ImageBuf.
    ///
    /// # Safety
    ///
    /// The caller must ensure that `data` is large enough for the given spec
    /// and strides, and that it remains valid for the lifetime of the ImageBuf.
    pub unsafe fn wrap(
        spec: ImageSpec,
        data: *mut u8,
        xstride: Option<usize>,
        ystride: Option<usize>,
        zstride: Option<usize>,
    ) -> Self {
        // SAFETY: Caller must ensure data pointer is valid
        let pixels = unsafe {
            PixelStorage::wrap(
                data,
                &spec,
                xstride,
                ystride,
                zstride,
            )
        };

        Self {
            inner: Arc::new(RwLock::new(ImageBufInner {
                spec,
                storage: IBStorage::AppBuffer,
                pixels,
                name: String::new(),
                subimage: 0,
                miplevel: 0,
                nsubimages: 1,
                nmiplevels: 1,
                error: None,
                cache: None,
                read_config: None,
                write_format: None,
                write_tiles: None,
                spec_valid: true,
                pixels_valid: true,
                read_only: false,
            })),
        }
    }

    /// Creates an ImageBuf for reading a file (lazy loading).
    ///
    /// The file is not read immediately. The spec will be read on first
    /// access to spec(), and pixels will be read on first pixel access.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use vfx_io::imagebuf::ImageBuf;
    ///
    /// let buf = ImageBuf::from_file("render.exr");
    /// println!("Size: {}x{}", buf.spec().width, buf.spec().height);
    /// ```
    pub fn from_file<P: AsRef<Path>>(path: P) -> Self {
        Self::from_file_opts(path, 0, 0, None, None)
    }

    /// Creates an ImageBuf for reading a file with options.
    ///
    /// # Arguments
    ///
    /// * `path` - File path
    /// * `subimage` - Subimage index (0 for first)
    /// * `miplevel` - MIP level (0 for highest resolution)
    /// * `cache` - Optional ImageCache for lazy tile loading
    /// * `config` - Optional configuration spec
    pub fn from_file_opts<P: AsRef<Path>>(
        path: P,
        subimage: i32,
        miplevel: i32,
        cache: Option<Arc<ImageCache>>,
        config: Option<&ImageSpec>,
    ) -> Self {
        let name = path.as_ref().to_string_lossy().to_string();
        let storage = if cache.is_some() {
            IBStorage::ImageCache
        } else {
            IBStorage::Uninitialized // Will become LocalBuffer when read
        };

        Self {
            inner: Arc::new(RwLock::new(ImageBufInner {
                spec: ImageSpec::default(),
                storage,
                pixels: PixelStorage::Empty,
                name,
                subimage,
                miplevel,
                nsubimages: 1, // Will be updated when reading file
                nmiplevels: 1, // Will be updated when reading file
                error: None,
                read_only: cache.is_some(),
                cache,
                read_config: config.cloned(),
                write_format: None,
                write_tiles: None,
                spec_valid: false,
                pixels_valid: false,
            })),
        }
    }

    // =========================================================================
    // Reset Methods
    // =========================================================================

    /// Resets the ImageBuf to uninitialized state.
    pub fn reset(&mut self) {
        let mut inner = self.inner.write().unwrap();
        inner.spec = ImageSpec::default();
        inner.storage = IBStorage::Uninitialized;
        inner.pixels = PixelStorage::Empty;
        inner.name.clear();
        inner.subimage = 0;
        inner.miplevel = 0;
        inner.error = None;
        inner.write_format = None;
        inner.write_tiles = None;
        inner.spec_valid = false;
        inner.pixels_valid = false;
        inner.read_only = false;
    }

    /// Resets and re-initializes with a new spec.
    pub fn reset_spec(&mut self, spec: ImageSpec, zero: InitializePixels) {
        let pixels = PixelStorage::allocate(&spec, zero == InitializePixels::Yes);
        let storage = if pixels.is_empty() {
            IBStorage::Uninitialized
        } else {
            IBStorage::LocalBuffer
        };

        let mut inner = self.inner.write().unwrap();
        inner.spec = spec;
        inner.storage = storage;
        inner.pixels = pixels;
        inner.error = None;
        inner.spec_valid = true;
        inner.pixels_valid = true;
        inner.read_only = false;
    }

    /// Resets and re-initializes with a new name and spec.
    pub fn reset_named(&mut self, name: impl Into<String>, spec: ImageSpec, zero: InitializePixels) {
        self.reset_spec(spec, zero);
        self.set_name(name);
    }

    /// Resets to wrap an external buffer.
    ///
    /// # Safety
    ///
    /// The caller must ensure the data pointer remains valid for the
    /// lifetime of the ImageBuf and has sufficient size for the spec.
    pub unsafe fn reset_wrap(
        &mut self,
        spec: ImageSpec,
        data: *mut u8,
        xstride: Option<usize>,
        ystride: Option<usize>,
        zstride: Option<usize>,
    ) {
        // SAFETY: Caller guarantees data pointer validity and lifetime
        let pixels = unsafe { PixelStorage::wrap(data, &spec, xstride, ystride, zstride) };
        let mut inner = self.inner.write().unwrap();
        inner.spec = spec;
        inner.storage = IBStorage::AppBuffer;
        inner.pixels = pixels;
        inner.error = None;
        inner.spec_valid = true;
        inner.pixels_valid = true;
        inner.read_only = false;
    }

    /// Resets to read from a file.
    pub fn reset_file<P: AsRef<Path>>(
        &mut self,
        path: P,
        subimage: i32,
        miplevel: i32,
        cache: Option<Arc<ImageCache>>,
    ) {
        let name = path.as_ref().to_string_lossy().to_string();
        let storage = if cache.is_some() {
            IBStorage::ImageCache
        } else {
            IBStorage::Uninitialized
        };

        let mut inner = self.inner.write().unwrap();
        inner.spec = ImageSpec::default();
        inner.storage = storage;
        inner.pixels = PixelStorage::Empty;
        inner.name = name;
        inner.subimage = subimage;
        inner.miplevel = miplevel;
        inner.error = None;
        inner.cache = cache;
        inner.write_format = None;
        inner.write_tiles = None;
        inner.spec_valid = false;
        inner.pixels_valid = false;
        inner.read_only = inner.cache.is_some();
    }

    // =========================================================================
    // State Queries
    // =========================================================================

    /// Returns the storage type.
    pub fn storage(&self) -> IBStorage {
        self.inner.read().unwrap().storage
    }

    /// Returns true if the ImageBuf is initialized with valid data.
    pub fn initialized(&self) -> bool {
        let inner = self.inner.read().unwrap();
        inner.storage != IBStorage::Uninitialized
    }

    /// Returns the image name (filename if read from file).
    pub fn name(&self) -> String {
        self.inner.read().unwrap().name.clone()
    }

    /// Sets the image name.
    pub fn set_name(&mut self, name: impl Into<String>) {
        self.inner.write().unwrap().name = name.into();
    }

    /// Returns the current subimage index.
    pub fn subimage(&self) -> i32 {
        self.inner.read().unwrap().subimage
    }

    /// Returns the current MIP level.
    pub fn miplevel(&self) -> i32 {
        self.inner.read().unwrap().miplevel
    }

    /// Returns the number of subimages (1 if unknown or single image).
    pub fn nsubimages(&self) -> i32 {
        self.ensure_spec_read();
        self.inner.read().unwrap().nsubimages
    }

    /// Returns the number of MIP levels (1 if unknown or no mipmaps).
    pub fn nmiplevels(&self) -> i32 {
        self.ensure_spec_read();
        self.inner.read().unwrap().nmiplevels
    }

    /// Returns the last error message, if any.
    pub fn geterror(&self) -> Option<String> {
        self.inner.read().unwrap().error.clone()
    }

    /// Returns true if there was an error.
    pub fn has_error(&self) -> bool {
        self.inner.read().unwrap().error.is_some()
    }

    /// Clears the error state.
    pub fn clear_error(&mut self) {
        self.inner.write().unwrap().error = None;
    }

    // =========================================================================
    // Spec Access
    // =========================================================================

    /// Returns a copy of the image specification.
    ///
    /// This will trigger lazy reading of the spec if needed.
    pub fn spec(&self) -> ImageSpec {
        self.ensure_spec_read();
        self.inner.read().unwrap().spec.clone()
    }

    /// Returns the ROI for the pixel data window.
    pub fn roi(&self) -> Roi3D {
        self.spec().roi()
    }

    /// Returns the ROI for the full/display window.
    pub fn roi_full(&self) -> Roi3D {
        self.spec().roi_full()
    }

    /// Convenience accessors.
    #[inline]
    pub fn width(&self) -> u32 {
        self.ensure_spec_read();
        self.inner.read().unwrap().spec.width
    }

    /// Returns the image height in pixels.
    #[inline]
    pub fn height(&self) -> u32 {
        self.ensure_spec_read();
        self.inner.read().unwrap().spec.height
    }

    /// Returns the image depth (for 3D/volume images, usually 1).
    #[inline]
    pub fn depth(&self) -> u32 {
        self.ensure_spec_read();
        self.inner.read().unwrap().spec.depth
    }

    /// Returns the number of channels per pixel.
    #[inline]
    pub fn nchannels(&self) -> u8 {
        self.ensure_spec_read();
        self.inner.read().unwrap().spec.nchannels
    }

    /// Returns the x origin of the data window.
    #[inline]
    pub fn xbegin(&self) -> i32 {
        self.ensure_spec_read();
        self.inner.read().unwrap().spec.x
    }

    /// Returns the y origin of the data window.
    #[inline]
    pub fn ybegin(&self) -> i32 {
        self.ensure_spec_read();
        self.inner.read().unwrap().spec.y
    }

    /// Returns the z origin of the data window.
    #[inline]
    pub fn zbegin(&self) -> i32 {
        self.ensure_spec_read();
        self.inner.read().unwrap().spec.z
    }

    /// Returns the x end of the data window (exclusive).
    #[inline]
    pub fn xend(&self) -> i32 {
        let inner = self.inner.read().unwrap();
        inner.spec.x + inner.spec.width as i32
    }

    /// Returns the y end of the data window (exclusive).
    #[inline]
    pub fn yend(&self) -> i32 {
        let inner = self.inner.read().unwrap();
        inner.spec.y + inner.spec.height as i32
    }

    /// Returns the z end of the data window (exclusive).
    #[inline]
    pub fn zend(&self) -> i32 {
        let inner = self.inner.read().unwrap();
        inner.spec.z + inner.spec.depth as i32
    }

    /// Orientation from metadata (1-8, EXIF standard).
    pub fn orientation(&self) -> i32 {
        self.ensure_spec_read();
        self.inner.read().unwrap().spec.get_int_attribute("Orientation", 1) as i32
    }

    /// Returns true if coordinates are within the data window.
    pub fn pixels_valid_coord(&self, x: i32, y: i32, z: i32) -> bool {
        let inner = self.inner.read().unwrap();
        x >= inner.spec.x
            && x < inner.spec.x + inner.spec.width as i32
            && y >= inner.spec.y
            && y < inner.spec.y + inner.spec.height as i32
            && z >= inner.spec.z
            && z < inner.spec.z + inner.spec.depth as i32
    }

    // =========================================================================
    // Pixel Strides
    // =========================================================================

    /// Returns bytes per pixel.
    pub fn pixel_stride(&self) -> usize {
        self.ensure_spec_read();
        self.inner.read().unwrap().spec.bytes_per_pixel()
    }

    /// Returns bytes per scanline (row).
    pub fn scanline_stride(&self) -> usize {
        self.ensure_spec_read();
        self.inner.read().unwrap().spec.bytes_per_row()
    }

    /// Returns bytes per image plane (for 3D images).
    pub fn z_stride(&self) -> usize {
        let inner = self.inner.read().unwrap();
        inner.spec.bytes_per_row() * inner.spec.height as usize
    }

    /// Returns true if pixels are stored contiguously.
    ///
    /// Contiguous storage means pixels are packed without gaps or padding,
    /// i.e., x_stride equals pixel size and y_stride equals width * x_stride.
    pub fn contiguous(&self) -> bool {
        let inner = self.inner.read().unwrap();
        inner.pixels.is_contiguous()
    }

    // =========================================================================
    // Data Format
    // =========================================================================

    /// Returns the pixel data format.
    pub fn format(&self) -> DataFormat {
        self.ensure_spec_read();
        self.inner.read().unwrap().spec.format
    }

    /// Sets the format to use when writing.
    pub fn set_write_format(&mut self, format: DataFormat) {
        self.inner.write().unwrap().write_format = Some(format);
    }

    /// Sets tile dimensions for writing.
    pub fn set_write_tiles(&mut self, tile_width: u32, tile_height: u32, tile_depth: u32) {
        self.inner.write().unwrap().write_tiles = Some((tile_width, tile_height, tile_depth));
    }

    // =========================================================================
    // Make Writable
    // =========================================================================

    /// Ensures the ImageBuf is writable (copies from cache if needed).
    pub fn make_writable(&mut self, keep_cache_type: bool) -> bool {
        let storage = self.storage();

        match storage {
            IBStorage::LocalBuffer | IBStorage::AppBuffer => true,
            IBStorage::Uninitialized => false,
            IBStorage::ImageCache => {
                // Force read into local buffer
                if !self.ensure_pixels_read() {
                    return false;
                }

                let mut inner = self.inner.write().unwrap();

                if !keep_cache_type {
                    // Already read into local buffer
                    inner.storage = IBStorage::LocalBuffer;
                    inner.read_only = false;
                }

                true
            }
        }
    }

    // =========================================================================
    // Raw Pixel Access
    // =========================================================================

    /// Returns pointer to local pixel data, or None if not local.
    pub fn localpixels(&self) -> Option<*const u8> {
        let inner = self.inner.read().unwrap();
        match inner.storage {
            IBStorage::LocalBuffer | IBStorage::AppBuffer => {
                inner.pixels.as_ptr()
            }
            _ => None,
        }
    }

    /// Returns mutable pointer to local pixel data, or None if not local/writable.
    pub fn localpixels_mut(&mut self) -> Option<*mut u8> {
        let inner = self.inner.read().unwrap();
        if inner.read_only {
            return None;
        }
        match inner.storage {
            IBStorage::LocalBuffer | IBStorage::AppBuffer => {
                drop(inner);
                self.inner.write().unwrap().pixels.as_mut_ptr()
            }
            _ => None,
        }
    }

    // =========================================================================
    // I/O Operations
    // =========================================================================

    /// Reads the spec only (for lazy loading).
    pub fn init_spec<P: AsRef<Path>>(
        &mut self,
        filename: P,
        subimage: i32,
        miplevel: i32,
    ) -> bool {
        self.reset_file(filename, subimage, miplevel, None);
        self.ensure_spec_read()
    }

    /// Reads the image file.
    ///
    /// # Arguments
    ///
    /// * `subimage` - Subimage to read (0 for first)
    /// * `miplevel` - MIP level to read (0 for highest resolution)
    /// * `force` - Force full read even if cache-backed
    /// * `convert` - Convert to this format (None = native)
    pub fn read(
        &mut self,
        subimage: i32,
        miplevel: i32,
        force: bool,
        convert: Option<DataFormat>,
    ) -> bool {
        {
            let mut inner = self.inner.write().unwrap();
            inner.subimage = subimage;
            inner.miplevel = miplevel;
            inner.spec_valid = false;
            inner.pixels_valid = false;
        }

        if !self.ensure_spec_read() {
            return false;
        }

        if force || self.storage() != IBStorage::ImageCache {
            if !self.ensure_pixels_read() {
                return false;
            }
        }

        // Handle format conversion if requested
        if let Some(target_format) = convert {
            let current_format = self.format();
            if current_format != target_format {
                self.convert_format(target_format);
            }
        }

        true
    }

    /// Writes the image to a file.
    ///
    /// # Arguments
    ///
    /// * `filename` - Output file path (empty = use internal name)
    /// * `fileformat` - Format hint (None = detect from extension)
    ///
    /// Note: Uses `write_format` if set via `set_write_format()` to convert pixel data.
    /// The `write_tiles` setting is noted but tile writing requires format-specific support.
    pub fn write<P: AsRef<Path>>(
        &self,
        filename: P,
        fileformat: Option<&str>,
    ) -> IoResult<()> {
        self.ensure_spec_read();
        self.ensure_pixels_read_ref();

        // Get settings from inner while holding lock
        let (path_buf, write_format) = {
            let inner = self.inner.read().unwrap();
            let p = if filename.as_ref().as_os_str().is_empty() {
                std::path::PathBuf::from(&inner.name)
            } else {
                filename.as_ref().to_path_buf()
            };
            (p, inner.write_format)
        };

        // Convert to ImageData
        let mut image_data = self.to_image_data()?;
        
        // Apply write_format conversion if set
        if let Some(target_format) = write_format {
            let current = match &image_data.data {
                crate::PixelData::U8(_) => crate::PixelFormat::U8,
                crate::PixelData::U16(_) => crate::PixelFormat::U16,
                crate::PixelData::U32(_) => crate::PixelFormat::U32,
                crate::PixelData::F32(_) => crate::PixelFormat::F32,
            };
            if current != target_format {
                image_data = image_data.convert_to(target_format);
            }
        }

        // Write using format hint if provided
        crate::write_with_format(&path_buf, &image_data, fileformat)
    }

    // =========================================================================
    // Pixel Access - Single Pixel
    // =========================================================================

    /// Gets a single channel value at (x, y, z).
    pub fn getchannel(&self, x: i32, y: i32, z: i32, c: usize, wrap: WrapMode) -> f32 {
        self.ensure_pixels_read_ref();

        let inner = self.inner.read().unwrap();
        let nchannels = inner.spec.nchannels as usize;

        if c >= nchannels {
            return 0.0;
        }

        if let Some((wx, wy)) = wrap.wrap(
            x - inner.spec.x,
            y - inner.spec.y,
            inner.spec.width as i32,
            inner.spec.height as i32,
        ) {
            inner.pixels.get_channel(wx as usize, wy as usize, z as usize, c, &inner.spec)
        } else {
            0.0
        }
    }

    /// Gets all channels for a pixel.
    pub fn getpixel(&self, x: i32, y: i32, z: i32, pixel: &mut [f32], wrap: WrapMode) {
        self.ensure_pixels_read_ref();

        let inner = self.inner.read().unwrap();
        let _nchannels = inner.spec.nchannels as usize;

        // Clear output
        for p in pixel.iter_mut() {
            *p = 0.0;
        }

        if let Some((wx, wy)) = wrap.wrap(
            x - inner.spec.x,
            y - inner.spec.y,
            inner.spec.width as i32,
            inner.spec.height as i32,
        ) {
            inner.pixels.get_pixel(wx as usize, wy as usize, z as usize, pixel, &inner.spec);
        }
        // If wrap returns None (WrapMode::Black), pixel is already zeroed
    }

    /// Sets all channels for a pixel.
    pub fn setpixel(&mut self, x: i32, y: i32, z: i32, pixel: &[f32]) {
        if !self.make_writable(false) {
            return;
        }

        let mut inner = self.inner.write().unwrap();
        let spec = inner.spec.clone();
        let xi = (x - spec.x) as usize;
        let yi = (y - spec.y) as usize;
        let zi = (z - spec.z) as usize;

        if xi < spec.width as usize
            && yi < spec.height as usize
            && zi < spec.depth as usize
        {
            inner.pixels.set_pixel(xi, yi, zi, pixel, &spec);
        }
    }

    /// Bilinear interpolation at fractional coordinates.
    pub fn interppixel(&self, x: f32, y: f32, pixel: &mut [f32], wrap: WrapMode) {
        self.ensure_pixels_read_ref();

        let inner = self.inner.read().unwrap();
        let spec = &inner.spec;

        // Convert to local coordinates
        let lx = x - spec.x as f32;
        let ly = y - spec.y as f32;

        let x0 = lx.floor() as i32;
        let y0 = ly.floor() as i32;
        let x1 = x0 + 1;
        let y1 = y0 + 1;

        let fx = lx - x0 as f32;
        let fy = ly - y0 as f32;

        let nch = spec.nchannels as usize;
        let mut p00 = vec![0.0f32; nch];
        let mut p10 = vec![0.0f32; nch];
        let mut p01 = vec![0.0f32; nch];
        let mut p11 = vec![0.0f32; nch];

        // Get four corner pixels
        self.getpixel(x0 + spec.x, y0 + spec.y, 0, &mut p00, wrap);
        self.getpixel(x1 + spec.x, y0 + spec.y, 0, &mut p10, wrap);
        self.getpixel(x0 + spec.x, y1 + spec.y, 0, &mut p01, wrap);
        self.getpixel(x1 + spec.x, y1 + spec.y, 0, &mut p11, wrap);

        // Bilinear interpolation
        for (i, p) in pixel.iter_mut().take(nch).enumerate() {
            let top = p00[i] * (1.0 - fx) + p10[i] * fx;
            let bot = p01[i] * (1.0 - fx) + p11[i] * fx;
            *p = top * (1.0 - fy) + bot * fy;
        }
    }

    /// Bilinear interpolation using NDC coordinates (0-1 range).
    pub fn interppixel_ndc(&self, s: f32, t: f32, pixel: &mut [f32], wrap: WrapMode) {
        let inner = self.inner.read().unwrap();
        let x = s * inner.spec.width as f32 + inner.spec.x as f32;
        let y = t * inner.spec.height as f32 + inner.spec.y as f32;
        drop(inner);
        self.interppixel(x, y, pixel, wrap);
    }

    /// Bicubic interpolation at fractional coordinates.
    pub fn interppixel_bicubic(&self, x: f32, y: f32, pixel: &mut [f32], wrap: WrapMode) {
        self.ensure_pixels_read_ref();

        let inner = self.inner.read().unwrap();
        let spec = &inner.spec;
        let nch = spec.nchannels as usize;

        let lx = x - spec.x as f32;
        let ly = y - spec.y as f32;

        let x0 = lx.floor() as i32;
        let y0 = ly.floor() as i32;
        let fx = lx - x0 as f32;
        let fy = ly - y0 as f32;

        // Get 4x4 grid of pixels
        let mut grid: [[Vec<f32>; 4]; 4] = core::array::from_fn(|_| {
            core::array::from_fn(|_| vec![0.0f32; nch])
        });
        for dy in 0..4 {
            for dx in 0..4 {
                let px = x0 - 1 + dx as i32 + spec.x;
                let py = y0 - 1 + dy as i32 + spec.y;
                self.getpixel(px, py, 0, &mut grid[dy][dx], wrap);
            }
        }

        // Cubic interpolation coefficients
        fn cubic(t: f32) -> [f32; 4] {
            let t2 = t * t;
            let t3 = t2 * t;
            [
                -0.5 * t3 + t2 - 0.5 * t,
                1.5 * t3 - 2.5 * t2 + 1.0,
                -1.5 * t3 + 2.0 * t2 + 0.5 * t,
                0.5 * t3 - 0.5 * t2,
            ]
        }

        let cx = cubic(fx);
        let cy = cubic(fy);

        // Apply bicubic filter
        for c in 0..nch.min(pixel.len()) {
            let mut sum = 0.0;
            for (j, cyj) in cy.iter().enumerate() {
                for (i, cxi) in cx.iter().enumerate() {
                    sum += grid[j][i][c] * cxi * cyj;
                }
            }
            pixel[c] = sum;
        }
    }

    // =========================================================================
    // Bulk Pixel Access
    // =========================================================================

    /// Gets a rectangular region of pixels.
    pub fn get_pixels(
        &self,
        roi: &Roi3D,
        format: DataFormat,
        data: &mut [u8],
    ) -> bool {
        self.ensure_pixels_read_ref();

        let inner = self.inner.read().unwrap();
        let spec = &inner.spec;

        // Validate ROI
        let roi = if roi.is_all() {
            spec.roi()
        } else {
            *roi
        };

        // Calculate required buffer size
        let pixel_size = format.bytes_per_channel() * spec.nchannels as usize;
        let required_size = roi.npixels() as usize * pixel_size;
        if data.len() < required_size {
            return false;
        }

        // Copy pixels
        let mut offset = 0;
        for z in roi.zbegin..roi.zend {
            for y in roi.ybegin..roi.yend {
                for x in roi.xbegin..roi.xend {
                    let mut pixel = vec![0.0f32; spec.nchannels as usize];
                    self.getpixel(x, y, z, &mut pixel, WrapMode::Black);

                    // Convert and store
                    match format {
                        DataFormat::F32 => {
                            for (i, &v) in pixel.iter().enumerate() {
                                let bytes = v.to_ne_bytes();
                                data[offset + i * 4..offset + (i + 1) * 4].copy_from_slice(&bytes);
                            }
                        }
                        DataFormat::U8 => {
                            for (i, &v) in pixel.iter().enumerate() {
                                data[offset + i] = (v.clamp(0.0, 1.0) * 255.0) as u8;
                            }
                        }
                        DataFormat::U16 => {
                            for (i, &v) in pixel.iter().enumerate() {
                                let u16_val = (v.clamp(0.0, 1.0) * 65535.0) as u16;
                                let bytes = u16_val.to_ne_bytes();
                                data[offset + i * 2..offset + (i + 1) * 2].copy_from_slice(&bytes);
                            }
                        }
                        DataFormat::F16 => {
                            for (i, &v) in pixel.iter().enumerate() {
                                let h = half::f16::from_f32(v);
                                let bytes = h.to_ne_bytes();
                                data[offset + i * 2..offset + (i + 1) * 2].copy_from_slice(&bytes);
                            }
                        }
                        DataFormat::U32 => {
                            for (i, &v) in pixel.iter().enumerate() {
                                let u32_val = v.max(0.0) as u32;
                                let bytes = u32_val.to_ne_bytes();
                                data[offset + i * 4..offset + (i + 1) * 4].copy_from_slice(&bytes);
                            }
                        }
                    }

                    offset += pixel_size;
                }
            }
        }

        true
    }

    /// Sets a rectangular region of pixels.
    pub fn set_pixels(
        &mut self,
        roi: &Roi3D,
        format: DataFormat,
        data: &[u8],
    ) -> bool {
        if !self.make_writable(false) {
            return false;
        }

        let inner = self.inner.read().unwrap();
        let spec = inner.spec.clone();
        let nch = spec.nchannels as usize;
        drop(inner);

        let roi = if roi.is_all() {
            spec.roi()
        } else {
            *roi
        };

        let pixel_size = format.bytes_per_channel() * nch;
        let mut offset = 0;

        for z in roi.zbegin..roi.zend {
            for y in roi.ybegin..roi.yend {
                for x in roi.xbegin..roi.xend {
                    // Convert from source format to f32
                    let mut pixel = vec![0.0f32; nch];

                    match format {
                        DataFormat::F32 => {
                            for (i, p) in pixel.iter_mut().enumerate() {
                                let bytes: [u8; 4] = data[offset + i * 4..offset + (i + 1) * 4]
                                    .try_into()
                                    .unwrap_or([0; 4]);
                                *p = f32::from_ne_bytes(bytes);
                            }
                        }
                        DataFormat::U8 => {
                            for (i, p) in pixel.iter_mut().enumerate() {
                                *p = data[offset + i] as f32 / 255.0;
                            }
                        }
                        DataFormat::U16 => {
                            for (i, p) in pixel.iter_mut().enumerate() {
                                let bytes: [u8; 2] = data[offset + i * 2..offset + (i + 1) * 2]
                                    .try_into()
                                    .unwrap_or([0; 2]);
                                *p = u16::from_ne_bytes(bytes) as f32 / 65535.0;
                            }
                        }
                        DataFormat::F16 => {
                            for (i, p) in pixel.iter_mut().enumerate() {
                                let bytes: [u8; 2] = data[offset + i * 2..offset + (i + 1) * 2]
                                    .try_into()
                                    .unwrap_or([0; 2]);
                                *p = half::f16::from_ne_bytes(bytes).to_f32();
                            }
                        }
                        DataFormat::U32 => {
                            for (i, p) in pixel.iter_mut().enumerate() {
                                let bytes: [u8; 4] = data[offset + i * 4..offset + (i + 1) * 4]
                                    .try_into()
                                    .unwrap_or([0; 4]);
                                *p = u32::from_ne_bytes(bytes) as f32;
                            }
                        }
                    }

                    self.setpixel(x, y, z, &pixel);
                    offset += pixel_size;
                }
            }
        }

        true
    }

    // =========================================================================
    // Conversion
    // =========================================================================

    /// Converts the ImageBuf to a different format in-place.
    pub fn convert_format(&mut self, format: DataFormat) {
        if !self.make_writable(false) {
            return;
        }

        let mut inner = self.inner.write().unwrap();
        if inner.spec.format == format {
            return;
        }

        inner.pixels = inner.pixels.convert_to(format, &inner.spec);
        inner.spec.format = format;
    }

    // =========================================================================
    // Copy Operations (OIIO Parity)
    // =========================================================================

    /// Copies metadata (spec attributes) from another ImageBuf.
    ///
    /// This copies all attributes but not pixel data or core dimensions.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use vfx_io::imagebuf::ImageBuf;
    ///
    /// let mut src = ImageBuf::from_file("source.exr");
    /// let mut dst = ImageBuf::new(dst_spec, InitializePixels::Yes);
    /// dst.copy_metadata(&src);
    /// ```
    pub fn copy_metadata(&mut self, src: &ImageBuf) {
        let src_inner = src.inner.read().unwrap();
        let mut dst_inner = self.inner.write().unwrap();

        // Copy all attributes from source spec
        for (key, value) in &src_inner.spec.attributes {
            dst_inner.spec.attributes.insert(key.clone(), value.clone());
        }

        // Copy channel names if they exist
        if !src_inner.spec.channel_names.is_empty() {
            dst_inner.spec.channel_names = src_inner.spec.channel_names.clone();
        }

        // Copy alpha/z channel indices
        if src_inner.spec.alpha_channel >= 0 {
            dst_inner.spec.alpha_channel = src_inner.spec.alpha_channel;
        }
        if src_inner.spec.z_channel >= 0 {
            dst_inner.spec.z_channel = src_inner.spec.z_channel;
        }
    }

    /// Copies pixel data from another ImageBuf.
    ///
    /// Both ImageBufs must have matching dimensions. The format may differ
    /// and will be converted automatically.
    ///
    /// # Arguments
    ///
    /// * `src` - Source ImageBuf to copy from
    /// * `roi` - Region to copy (None = entire image)
    ///
    /// # Returns
    ///
    /// `true` on success, `false` on failure.
    pub fn copy_pixels(&mut self, src: &ImageBuf, roi: Option<Roi3D>) -> bool {
        if !self.make_writable(false) {
            return false;
        }

        src.ensure_pixels_read_ref();

        let src_inner = src.inner.read().unwrap();
        let dst_inner = self.inner.read().unwrap();

        let roi = roi.unwrap_or_else(|| src_inner.spec.roi());
        let roi = if roi.is_all() {
            src_inner.spec.roi()
        } else {
            roi
        };

        // Validate dimensions match or roi fits in destination
        let dst_roi = dst_inner.spec.roi();
        if roi.width() > dst_roi.width() || roi.height() > dst_roi.height() {
            return false;
        }

        drop(src_inner);
        drop(dst_inner);

        // Copy pixels
        let nch = self.nchannels() as usize;
        let mut pixel = vec![0.0f32; nch.max(src.nchannels() as usize)];

        for z in roi.zbegin..roi.zend {
            for y in roi.ybegin..roi.yend {
                for x in roi.xbegin..roi.xend {
                    src.getpixel(x, y, z, &mut pixel, WrapMode::Black);
                    self.setpixel(x, y, z, &pixel[..nch]);
                }
            }
        }

        true
    }

    /// Copies both metadata and pixel data from another ImageBuf.
    ///
    /// This is a convenience function that calls copy_metadata and copy_pixels.
    pub fn copy(&mut self, src: &ImageBuf, roi: Option<Roi3D>) -> bool {
        self.copy_metadata(src);
        self.copy_pixels(src, roi)
    }

    /// Swaps contents with another ImageBuf.
    pub fn swap(&mut self, other: &mut ImageBuf) {
        std::mem::swap(&mut self.inner, &mut other.inner);
    }

    /// Creates a deep copy of this ImageBuf.
    ///
    /// Unlike clone() which shares data for cache-backed images,
    /// deep_copy() always creates an independent copy of pixel data.
    pub fn deep_copy(&self) -> Self {
        self.clone() // Our clone already does deep copy
    }

    /// Creates a copy with different dimensions or format.
    ///
    /// # Arguments
    ///
    /// * `new_spec` - Specification for the new ImageBuf
    /// * `copy_pixels` - Whether to copy/convert pixel data
    ///
    /// # Returns
    ///
    /// A new ImageBuf with the requested spec.
    pub fn copy_with_spec(&self, new_spec: ImageSpec, do_copy_pixels: bool) -> Self {
        let mut result = Self::new(new_spec.clone(), InitializePixels::Yes);
        result.copy_metadata(self);

        if do_copy_pixels {
            // Calculate overlapping region
            let src_roi = self.roi();
            let dst_roi = new_spec.roi();

            if let Some(overlap) = src_roi.intersection(&dst_roi) {
                result.copy_pixels(self, Some(overlap));
            }
        }

        result
    }

    /// Sets all pixels in the image to zero/black.
    pub fn zero(&mut self) {
        if !self.make_writable(false) {
            return;
        }

        let mut inner = self.inner.write().unwrap();
        let spec = inner.spec.clone();
        inner.pixels.fill_zero(&spec);
    }

    /// Fills all pixels with the given color value.
    ///
    /// # Arguments
    ///
    /// * `values` - Color values for each channel
    pub fn fill(&mut self, values: &[f32]) {
        if !self.make_writable(false) {
            return;
        }

        let roi = self.roi();
        for z in roi.zbegin..roi.zend {
            for y in roi.ybegin..roi.yend {
                for x in roi.xbegin..roi.xend {
                    self.setpixel(x, y, z, values);
                }
            }
        }
    }

    /// Fills a region with the given color value.
    pub fn fill_roi(&mut self, values: &[f32], roi: &Roi3D) {
        if !self.make_writable(false) {
            return;
        }

        let roi = if roi.is_all() { self.roi() } else { *roi };

        for z in roi.zbegin..roi.zend {
            for y in roi.ybegin..roi.yend {
                for x in roi.xbegin..roi.xend {
                    self.setpixel(x, y, z, values);
                }
            }
        }
    }

    /// Converts to ImageData for use with other vfx-io functions.
    pub fn to_image_data(&self) -> IoResult<crate::ImageData> {
        if !self.ensure_pixels_read_ref() {
            let inner = self.inner.read().unwrap();
            if let Some(ref err) = inner.error {
                return Err(crate::IoError::DecodeError(err.clone()));
            }
            return Err(crate::IoError::DecodeError("Failed to load pixels".into()));
        }

        let inner = self.inner.read().unwrap();
        let spec = &inner.spec;

        let pixel_count = spec.pixel_count() as usize;
        let nch = spec.nchannels as usize;

        // Get all pixels as f32
        let mut data = vec![0.0f32; pixel_count * nch];
        for y in 0..spec.height {
            for x in 0..spec.width {
                let idx = (y * spec.width + x) as usize * nch;
                inner.pixels.get_pixel(
                    x as usize,
                    y as usize,
                    0,
                    &mut data[idx..idx + nch],
                    spec,
                );
            }
        }

        Ok(crate::ImageData {
            width: spec.width,
            height: spec.height,
            channels: spec.nchannels as u32,
            format: crate::PixelFormat::F32,
            data: crate::PixelData::F32(data),
            metadata: crate::Metadata::default(),
        })
    }

    /// Creates an ImageBuf from ImageData.
    pub fn from_image_data(data: &crate::ImageData) -> Self {
        let spec = ImageSpec::new(
            data.width,
            data.height,
            data.channels as u8,
            data.format,
        );

        let buf = Self::new(spec, InitializePixels::No);
        let f32_data = data.to_f32();

        {
            let mut inner = buf.inner.write().unwrap();
            let spec_copy = inner.spec.clone();
            let nch = spec_copy.nchannels as usize;
            let width = spec_copy.width;
            let height = spec_copy.height;

            for y in 0..height {
                for x in 0..width {
                    let idx = (y * width + x) as usize * nch;
                    inner.pixels.set_pixel(
                        x as usize,
                        y as usize,
                        0,
                        &f32_data[idx..idx + nch],
                        &spec_copy,
                    );
                }
            }
        }

        buf
    }

    // =========================================================================
    // Internal Helpers
    // =========================================================================

    fn ensure_spec_read(&self) -> bool {
        let inner = self.inner.read().unwrap();
        if inner.spec_valid {
            return true;
        }
        drop(inner);

        // Need to read spec from file
        let name = {
            let inner = self.inner.read().unwrap();
            inner.name.clone()
        };

        if name.is_empty() {
            return false;
        }

        // Use probe_image_info to get dimensions and channel count
        match crate::probe_image_info(&name) {
            Ok((width, height, channels)) => {
                let mut inner = self.inner.write().unwrap();
                inner.spec.width = width;
                inner.spec.height = height;
                inner.spec.nchannels = channels as u8;
                inner.spec.full_width = width;
                inner.spec.full_height = height;
                #[allow(deprecated)]
                {
                    inner.spec.channels = channels as u8;
                }
                
                // Try to get subimage/miplevel counts from file
                // For EXR files, each header is a subimage (part)
                if name.to_lowercase().ends_with(".exr") {
                    if let Ok(meta) = vfx_exr::meta::MetaData::read_from_file(&name, false) {
                        inner.nsubimages = meta.headers.len() as i32;
                        // EXR doesn't have traditional mipmaps in headers
                        // (they're stored in tiled scanline or ripmap modes)
                        inner.nmiplevels = 1;
                    }
                }
                
                inner.spec_valid = true;
                true
            }
            Err(e) => {
                let mut inner = self.inner.write().unwrap();
                inner.error = Some(format!("Failed to read spec: {}", e));
                false
            }
        }
    }

    fn ensure_pixels_read(&mut self) -> bool {
        {
            let inner = self.inner.read().unwrap();
            if inner.pixels_valid {
                return true;
            }
        }

        if !self.ensure_spec_read() {
            return false;
        }

        let (name, subimage, miplevel) = {
            let inner = self.inner.read().unwrap();
            (inner.name.clone(), inner.subimage as usize, inner.miplevel as usize)
        };

        if name.is_empty() {
            return false;
        }

        // Read the actual image data with subimage/miplevel support
        match crate::read_subimage(&name, subimage, miplevel) {
            Ok(image_data) => {
                let f32_data = image_data.to_f32();
                let mut inner = self.inner.write().unwrap();

                // Update spec from actual read
                inner.spec.width = image_data.width;
                inner.spec.height = image_data.height;
                inner.spec.nchannels = image_data.channels as u8;
                inner.spec.full_width = image_data.width;
                inner.spec.full_height = image_data.height;
                #[allow(deprecated)]
                {
                    inner.spec.channels = image_data.channels as u8;
                }
                inner.spec.format = DataFormat::F32;

                // Allocate and fill pixels
                inner.pixels = PixelStorage::allocate(&inner.spec, false);
                let nch = inner.spec.nchannels as usize;
                let width = inner.spec.width;
                let height = inner.spec.height;
                let spec_copy = inner.spec.clone();

                for y in 0..height {
                    for x in 0..width {
                        let idx = (y * width + x) as usize * nch;
                        inner.pixels.set_pixel(
                            x as usize,
                            y as usize,
                            0,
                            &f32_data[idx..idx.saturating_add(nch).min(f32_data.len())],
                            &spec_copy,
                        );
                    }
                }

                inner.storage = IBStorage::LocalBuffer;
                inner.pixels_valid = true;

                // Check if format conversion is needed from read_config
                let target_format = inner.read_config.as_ref()
                    .map(|cfg| cfg.format)
                    .filter(|&f| f != DataFormat::F32);
                
                // Release lock before conversion
                drop(inner);
                
                // Apply format conversion if specified in read_config
                if let Some(fmt) = target_format {
                    self.convert_format(fmt);
                }
                
                true
            }
            Err(e) => {
                let mut inner = self.inner.write().unwrap();
                inner.error = Some(format!("Failed to read pixels: {}", e));
                false
            }
        }
    }

    fn ensure_pixels_read_ref(&self) -> bool {
        {
            let inner = self.inner.read().unwrap();
            if inner.pixels_valid {
                return true;
            }
        }

        // Use interior mutability (RwLock) to load pixels even from &self.
        // First ensure spec is read.
        if !self.ensure_spec_read() {
            return false;
        }

        let (name, subimage, miplevel) = {
            let inner = self.inner.read().unwrap();
            (inner.name.clone(), inner.subimage as usize, inner.miplevel as usize)
        };

        if name.is_empty() {
            return false;
        }

        // Read the actual image data with subimage/miplevel support.
        match crate::read_subimage(&name, subimage, miplevel) {
            Ok(image_data) => {
                let f32_data = image_data.to_f32();
                let mut inner = self.inner.write().unwrap();

                // Update spec from actual read.
                inner.spec.width = image_data.width;
                inner.spec.height = image_data.height;
                inner.spec.nchannels = image_data.channels as u8;
                inner.spec.full_width = image_data.width;
                inner.spec.full_height = image_data.height;
                #[allow(deprecated)]
                {
                    inner.spec.channels = image_data.channels as u8;
                }
                inner.spec.format = DataFormat::F32;

                // Allocate and fill pixels.
                inner.pixels = PixelStorage::allocate(&inner.spec, false);
                let nch = inner.spec.nchannels as usize;
                let width = inner.spec.width;
                let height = inner.spec.height;
                let spec_copy = inner.spec.clone();

                for y in 0..height {
                    for x in 0..width {
                        let idx = (y * width + x) as usize * nch;
                        inner.pixels.set_pixel(
                            x as usize,
                            y as usize,
                            0,
                            &f32_data[idx..idx.saturating_add(nch).min(f32_data.len())],
                            &spec_copy,
                        );
                    }
                }

                inner.storage = IBStorage::LocalBuffer;
                inner.pixels_valid = true;
                true
            }
            Err(e) => {
                let mut inner = self.inner.write().unwrap();
                inner.error = Some(format!("Failed to read pixels: {}", e));
                false
            }
        }
    }
}

impl std::fmt::Debug for ImageBuf {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let inner = self.inner.read().unwrap();
        f.debug_struct("ImageBuf")
            .field("name", &inner.name)
            .field("storage", &inner.storage)
            .field("width", &inner.spec.width)
            .field("height", &inner.spec.height)
            .field("nchannels", &inner.spec.nchannels)
            .field("format", &inner.spec.format)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_imagebuf_new() {
        let spec = ImageSpec::rgba(100, 100);
        let buf = ImageBuf::new(spec, InitializePixels::Yes);

        assert_eq!(buf.width(), 100);
        assert_eq!(buf.height(), 100);
        assert_eq!(buf.nchannels(), 4);
        assert_eq!(buf.storage(), IBStorage::LocalBuffer);
    }

    #[test]
    fn test_imagebuf_pixel_access() {
        let spec = ImageSpec::rgba(10, 10);
        let mut buf = ImageBuf::new(spec, InitializePixels::Yes);

        // Set a pixel
        buf.setpixel(5, 5, 0, &[1.0, 0.5, 0.25, 1.0]);

        // Get it back
        let mut pixel = [0.0f32; 4];
        buf.getpixel(5, 5, 0, &mut pixel, WrapMode::Black);

        assert!((pixel[0] - 1.0).abs() < 0.001);
        assert!((pixel[1] - 0.5).abs() < 0.001);
        assert!((pixel[2] - 0.25).abs() < 0.001);
        assert!((pixel[3] - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_imagebuf_wrap_modes() {
        let spec = ImageSpec::rgba(10, 10);
        let mut buf = ImageBuf::new(spec, InitializePixels::Yes);

        // Set corner pixel
        buf.setpixel(0, 0, 0, &[1.0, 0.0, 0.0, 1.0]);

        // Black wrap - out of bounds should be black
        let mut pixel = [0.0f32; 4];
        buf.getpixel(-1, -1, 0, &mut pixel, WrapMode::Black);
        assert_eq!(pixel[0], 0.0);

        // Clamp wrap - should get corner pixel
        buf.getpixel(-1, -1, 0, &mut pixel, WrapMode::Clamp);
        assert!((pixel[0] - 1.0).abs() < 0.001);

        // Periodic wrap
        buf.getpixel(10, 0, 0, &mut pixel, WrapMode::Periodic);
        assert!((pixel[0] - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_imagebuf_interp() {
        let spec = ImageSpec::rgba(10, 10);
        let mut buf = ImageBuf::new(spec, InitializePixels::Yes);

        // Set two adjacent pixels
        buf.setpixel(0, 0, 0, &[0.0, 0.0, 0.0, 1.0]);
        buf.setpixel(1, 0, 0, &[1.0, 0.0, 0.0, 1.0]);

        // Interpolate halfway between
        let mut pixel = [0.0f32; 4];
        buf.interppixel(0.5, 0.0, &mut pixel, WrapMode::Black);

        // Should be approximately 0.5
        assert!((pixel[0] - 0.5).abs() < 0.1);
    }

    #[test]
    fn test_imagebuf_clone() {
        let spec = ImageSpec::rgba(10, 10);
        let mut buf = ImageBuf::new(spec, InitializePixels::Yes);
        buf.setpixel(5, 5, 0, &[1.0, 0.0, 0.0, 1.0]);

        let buf2 = buf.clone();

        let mut pixel = [0.0f32; 4];
        buf2.getpixel(5, 5, 0, &mut pixel, WrapMode::Black);
        assert!((pixel[0] - 1.0).abs() < 0.001);
    }
}
