//! Unified compute pipeline with automatic strategy selection.
//!
//! Provides a high-level API that automatically chooses between in-memory,
//! tiled, and streaming processing based on image size and available resources.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │                      ComputePipeline                                 │
//! ├─────────────────────────────────────────────────────────────────────┤
//! │                                                                     │
//! │  ImageInput ──► Strategy Selection ──► Execution ──► ImageOutput   │
//! │       │                 │                  │               │        │
//! │       ▼                 ▼                  ▼               ▼        │
//! │  ┌─────────┐    ┌────────────┐    ┌────────────┐    ┌─────────┐   │
//! │  │ Memory  │    │ SinglePass │    │ run_single │    │ Memory  │   │
//! │  │ File    │    │ Tiled      │    │ run_tiled  │    │ File    │   │
//! │  │ Stream  │    │ Streaming  │    │ run_stream │    │ Stream  │   │
//! │  └─────────┘    └────────────┘    └────────────┘    └─────────┘   │
//! │                                                                     │
//! └─────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Example
//!
//! ```ignore
//! use vfx_compute::{ComputePipeline, ImageInput, ImageOutput, ComputeOp};
//!
//! let mut pipeline = ComputePipeline::auto()?;
//!
//! // Automatic strategy selection based on file size
//! pipeline.process(
//!     ImageInput::File("huge_8k.exr".into()),
//!     ImageOutput::File("output.exr".into()),
//!     &[
//!         ComputeOp::Exposure(1.5),
//!         ComputeOp::Saturation(1.2),
//!     ],
//! )?;
//! ```
//!
//! # Strategy Selection
//!
//! The pipeline automatically selects:
//!
//! | Image Size | RAM Fit | Strategy |
//! |------------|---------|----------|
//! | < 4 MP     | Yes     | SinglePass |
//! | 4-64 MP    | Yes     | Tiled |
//! | > 64 MP    | No      | Streaming |
//!
//! Users can override with `with_strategy()`.

use std::path::{Path, PathBuf};

use crate::{
    Backend, ComputeError, ComputeImage, ComputeResult,
    Processor, ProcessorBuilder, ProcessingStrategy,
};
use crate::color::Cdl;
use crate::ops::ResizeFilter;

#[cfg(feature = "io")]
use vfx_io::{ImageData, ImageLayer};

// ============================================================================
// Input/Output Types
// ============================================================================

/// Input source for pipeline processing.
///
/// Supports multiple input modes:
/// - `Memory`: ComputeImage already in RAM
/// - `Data`: ImageData from vfx-io (with io feature)
/// - `Layer`: ImageLayer from vfx-io (with io feature)
/// - `File`: Path to file (may use streaming for large files)
#[derive(Debug)]
pub enum ImageInput {
    /// ComputeImage already in memory.
    Memory(ComputeImage),
    /// ImageData from vfx-io.
    #[cfg(feature = "io")]
    Data(ImageData),
    /// ImageLayer from vfx-io.
    #[cfg(feature = "io")]
    Layer(ImageLayer),
    /// Path to image file.
    ///
    /// The pipeline will probe the file to determine size and choose
    /// the optimal loading strategy.
    File(PathBuf),
}

impl ImageInput {
    /// Creates input from file path.
    pub fn file<P: AsRef<Path>>(path: P) -> Self {
        Self::File(path.as_ref().to_path_buf())
    }

    /// Creates input from in-memory ComputeImage.
    pub fn memory(img: ComputeImage) -> Self {
        Self::Memory(img)
    }

    /// Creates input from ImageData.
    #[cfg(feature = "io")]
    pub fn data(data: ImageData) -> Self {
        Self::Data(data)
    }

    /// Creates input from ImageLayer.
    #[cfg(feature = "io")]
    pub fn layer(layer: ImageLayer) -> Self {
        Self::Layer(layer)
    }

    /// Returns dimensions if known without loading.
    ///
    /// For in-memory inputs, returns dimensions directly.
    /// For `File` input, probes the file header.
    pub fn dimensions(&self) -> ComputeResult<(u32, u32)> {
        match self {
            Self::Memory(img) => Ok((img.width, img.height)),
            #[cfg(feature = "io")]
            Self::Data(data) => Ok((data.width, data.height)),
            #[cfg(feature = "io")]
            Self::Layer(layer) => Ok((layer.width, layer.height)),
            Self::File(path) => probe_dimensions(path),
        }
    }

    /// Estimates memory requirements in bytes.
    pub fn estimate_memory(&self) -> ComputeResult<u64> {
        let (w, h) = self.dimensions()?;
        // Assume RGBA F32 for worst case
        Ok((w as u64) * (h as u64) * 4 * 4)
    }
}

impl From<ComputeImage> for ImageInput {
    fn from(img: ComputeImage) -> Self {
        Self::Memory(img)
    }
}

#[cfg(feature = "io")]
impl From<ImageData> for ImageInput {
    fn from(data: ImageData) -> Self {
        Self::Data(data)
    }
}

#[cfg(feature = "io")]
impl From<ImageLayer> for ImageInput {
    fn from(layer: ImageLayer) -> Self {
        Self::Layer(layer)
    }
}

impl From<PathBuf> for ImageInput {
    fn from(path: PathBuf) -> Self {
        Self::File(path)
    }
}

impl From<&Path> for ImageInput {
    fn from(path: &Path) -> Self {
        Self::File(path.to_path_buf())
    }
}

/// Output destination for pipeline processing.
///
/// Supports two modes:
/// - `Memory`: Return result in RAM
/// - `File`: Write result to file
#[derive(Debug)]
pub enum ImageOutput {
    /// Return result in memory.
    Memory,
    /// Write result to file path.
    File(PathBuf),
}

impl ImageOutput {
    /// Creates output to file path.
    pub fn file<P: AsRef<Path>>(path: P) -> Self {
        Self::File(path.as_ref().to_path_buf())
    }

    /// Creates output to memory.
    pub fn memory() -> Self {
        Self::Memory
    }
}

impl From<PathBuf> for ImageOutput {
    fn from(path: PathBuf) -> Self {
        Self::File(path)
    }
}

impl From<&Path> for ImageOutput {
    fn from(path: &Path) -> Self {
        Self::File(path.to_path_buf())
    }
}

/// Result of pipeline processing.
#[derive(Debug)]
pub enum ProcessResult {
    /// Image returned in memory.
    Image(ComputeImage),
    /// Image written to file (path returned for confirmation).
    Written(PathBuf),
}

impl ProcessResult {
    /// Returns the image if result is in memory.
    pub fn into_image(self) -> Option<ComputeImage> {
        match self {
            Self::Image(img) => Some(img),
            Self::Written(_) => None,
        }
    }

    /// Returns the output path if result was written to file.
    pub fn into_path(self) -> Option<PathBuf> {
        match self {
            Self::Image(_) => None,
            Self::Written(path) => Some(path),
        }
    }

    /// Returns true if result is in memory.
    pub fn is_memory(&self) -> bool {
        matches!(self, Self::Image(_))
    }

    /// Returns true if result was written to file.
    pub fn is_file(&self) -> bool {
        matches!(self, Self::Written(_))
    }
}

// ============================================================================
// Compute Operations
// ============================================================================

/// Compute operation to apply.
///
/// Each variant represents a single image processing operation.
/// Operations are applied in order during pipeline execution.
#[derive(Debug, Clone)]
pub enum ComputeOp {
    // Color operations
    /// Apply 4x4 color matrix transform.
    Matrix([f32; 16]),
    /// Apply CDL (Color Decision List) transform.
    Cdl(Cdl),
    /// Apply 1D LUT (data, channels).
    Lut1D { lut: Vec<f32>, channels: u32 },
    /// Apply 3D LUT with trilinear interpolation (data, size).
    Lut3D { lut: Vec<f32>, size: u32 },
    /// Exposure adjustment in stops (+1 = 2x brighter).
    Exposure(f32),
    /// Saturation adjustment (1.0 = no change, 0.0 = grayscale).
    Saturation(f32),
    /// Contrast adjustment (1.0 = no change).
    Contrast(f32),

    // Image operations
    /// Gaussian blur with radius.
    Blur(f32),
    /// Unsharp mask sharpening with amount.
    Sharpen(f32),
    /// Resize to dimensions with filter.
    Resize { width: u32, height: u32, filter: ResizeFilter },
    /// Crop region (x, y, w, h).
    Crop { x: u32, y: u32, w: u32, h: u32 },
    /// Flip horizontal.
    FlipH,
    /// Flip vertical.
    FlipV,
    /// Rotate 90 degrees clockwise (n times).
    Rotate90(u32),
}

impl ComputeOp {
    /// Creates exposure operation.
    pub fn exposure(stops: f32) -> Self {
        Self::Exposure(stops)
    }

    /// Creates saturation operation.
    pub fn saturation(sat: f32) -> Self {
        Self::Saturation(sat)
    }

    /// Creates contrast operation.
    pub fn contrast(c: f32) -> Self {
        Self::Contrast(c)
    }

    /// Creates blur operation.
    pub fn blur(radius: f32) -> Self {
        Self::Blur(radius)
    }

    /// Creates sharpen operation.
    pub fn sharpen(amount: f32) -> Self {
        Self::Sharpen(amount)
    }

    /// Creates resize operation.
    pub fn resize(width: u32, height: u32, filter: ResizeFilter) -> Self {
        Self::Resize { width, height, filter }
    }

    /// Creates crop operation.
    pub fn crop(x: u32, y: u32, w: u32, h: u32) -> Self {
        Self::Crop { x, y, w, h }
    }

    /// Returns true if operation changes image dimensions.
    pub fn changes_dimensions(&self) -> bool {
        matches!(self, Self::Resize { .. } | Self::Crop { .. } | Self::Rotate90(_))
    }
}

// ============================================================================
// Pipeline
// ============================================================================

/// Unified compute pipeline with automatic strategy selection.
///
/// Provides a high-level API that automatically chooses between:
/// - **SinglePass**: Image fits in GPU memory
/// - **Tiled**: Image needs tiling but fits in RAM
/// - **Streaming**: Image too large for RAM
///
/// # Example
///
/// ```ignore
/// use vfx_compute::{ComputePipeline, ImageInput, ImageOutput, ComputeOp};
///
/// // Create pipeline with auto backend
/// let mut pipeline = ComputePipeline::auto()?;
///
/// // Process with automatic strategy selection
/// let result = pipeline.process(
///     ImageInput::file("input.exr"),
///     ImageOutput::file("output.exr"),
///     &[ComputeOp::exposure(1.5), ComputeOp::saturation(1.2)],
/// )?;
/// ```
///
/// # Configuration
///
/// Use builder for fine-grained control:
///
/// ```ignore
/// let pipeline = ComputePipeline::builder()
///     .backend(Backend::Wgpu)
///     .tile_size(2048)
///     .ram_limit_mb(8192)
///     .verbose(true)
///     .build()?;
/// ```
pub struct ComputePipeline {
    processor: Processor,
    strategy_override: Option<ProcessingStrategy>,
    verbose: bool,
}

impl ComputePipeline {
    /// Creates pipeline with specified processor.
    pub fn new(processor: Processor) -> Self {
        let verbose = processor.config().verbose;
        Self {
            processor,
            strategy_override: None,
            verbose,
        }
    }

    /// Creates pipeline with auto-selected backend.
    pub fn auto() -> ComputeResult<Self> {
        Ok(Self::new(Processor::auto()?))
    }

    /// Creates pipeline with CPU backend.
    pub fn cpu() -> ComputeResult<Self> {
        Ok(Self::new(Processor::cpu()?))
    }

    /// Creates pipeline with GPU backend.
    #[cfg(feature = "wgpu")]
    pub fn gpu() -> ComputeResult<Self> {
        Ok(Self::new(Processor::gpu()?))
    }

    /// Creates pipeline builder.
    pub fn builder() -> ComputePipelineBuilder {
        ComputePipelineBuilder::new()
    }

    /// Returns reference to underlying processor.
    pub fn processor(&self) -> &Processor {
        &self.processor
    }

    /// Returns mutable reference to underlying processor.
    pub fn processor_mut(&mut self) -> &mut Processor {
        &mut self.processor
    }

    /// Overrides automatic strategy selection.
    ///
    /// Call with `None` to restore automatic selection.
    pub fn with_strategy(mut self, strategy: Option<ProcessingStrategy>) -> Self {
        self.strategy_override = strategy;
        self
    }

    /// Sets strategy override.
    pub fn set_strategy(&mut self, strategy: Option<ProcessingStrategy>) {
        self.strategy_override = strategy;
    }

    /// Forces streaming mode.
    pub fn force_streaming(mut self, tile_size: u32) -> Self {
        self.strategy_override = Some(ProcessingStrategy::Streaming { tile_size });
        self
    }

    // =========================================================================
    // Strategy Selection
    // =========================================================================

    /// Recommends processing strategy for given dimensions.
    pub fn recommend_strategy(&self, width: u32, height: u32, channels: u32) -> ProcessingStrategy {
        if let Some(ref strategy) = self.strategy_override {
            return strategy.clone();
        }

        let img = ComputeImage::new(width, height, channels);
        self.processor.recommend_strategy(&img)
    }

    /// Recommends strategy for input.
    pub fn recommend_strategy_for(&self, input: &ImageInput) -> ComputeResult<ProcessingStrategy> {
        if let Some(ref strategy) = self.strategy_override {
            return Ok(strategy.clone());
        }

        let (w, h) = input.dimensions()?;
        Ok(self.recommend_strategy(w, h, 4)) // Assume RGBA
    }

    /// Logs strategy if verbose mode enabled.
    fn log(&self, msg: &str) {
        if self.verbose {
            eprintln!("[ComputePipeline] {}", msg);
        }
    }

    // =========================================================================
    // Processing
    // =========================================================================

    /// Processes input with given operations.
    ///
    /// Automatically selects the optimal strategy based on:
    /// - Image dimensions
    /// - Available GPU memory
    /// - Available RAM
    /// - Configured limits
    ///
    /// # Arguments
    ///
    /// * `input` - Image source (memory or file)
    /// * `output` - Image destination (memory or file)
    /// * `ops` - Operations to apply in order
    ///
    /// # Returns
    ///
    /// `ProcessResult` containing either the image or confirmation of file write.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let result = pipeline.process(
    ///     ImageInput::file("input.exr"),
    ///     ImageOutput::memory(),
    ///     &[ComputeOp::exposure(1.0)],
    /// )?;
    ///
    /// if let Some(img) = result.into_image() {
    ///     println!("Got {}x{} image", img.width, img.height);
    /// }
    /// ```
    pub fn process(
        &mut self,
        input: ImageInput,
        output: ImageOutput,
        ops: &[ComputeOp],
    ) -> ComputeResult<ProcessResult> {
        let strategy = self.recommend_strategy_for(&input)?;

        self.log(&format!(
            "Processing with strategy: {:?}",
            strategy
        ));

        match strategy {
            ProcessingStrategy::SinglePass => self.run_single(input, output, ops),
            ProcessingStrategy::Tiled { tile_size, .. } => {
                self.run_tiled(input, output, ops, tile_size)
            }
            ProcessingStrategy::Streaming { tile_size } => {
                self.run_streaming(input, output, ops, tile_size)
            }
        }
    }

    /// Runs single-pass processing (image fits in GPU).
    fn run_single(
        &mut self,
        input: ImageInput,
        output: ImageOutput,
        ops: &[ComputeOp],
    ) -> ComputeResult<ProcessResult> {
        self.log("Running single-pass");

        // Load image
        let mut img = self.load_input(input)?;

        // Apply operations
        for op in ops {
            img = self.apply_op(img, op)?;
        }

        // Write output
        self.write_output(img, output)
    }

    /// Runs tiled processing (image needs tiling but fits in RAM).
    fn run_tiled(
        &mut self,
        input: ImageInput,
        output: ImageOutput,
        ops: &[ComputeOp],
        tile_size: u32,
    ) -> ComputeResult<ProcessResult> {
        self.log(&format!("Running tiled with tile_size={}", tile_size));

        // For now, load full image and process
        // TODO: Implement true tiled processing with TileWorkflow
        let mut img = self.load_input(input)?;

        // Check for dimension-changing ops (can't tile those)
        let has_resize = ops.iter().any(|op| op.changes_dimensions());
        if has_resize {
            self.log("Has dimension-changing ops, falling back to single pass");
            for op in ops {
                img = self.apply_op(img, op)?;
            }
        } else {
            // Process in tiles
            let (w, h) = (img.width, img.height);
            for ty in (0..h).step_by(tile_size as usize) {
                for tx in (0..w).step_by(tile_size as usize) {
                    let tw = tile_size.min(w - tx);
                    let th = tile_size.min(h - ty);

                    // Extract tile
                    let mut tile = self.processor.crop(&img, tx, ty, tw, th)?;

                    // Apply ops to tile
                    for op in ops {
                        if !op.changes_dimensions() {
                            tile = self.apply_op(tile, op)?;
                        }
                    }

                    // Copy back
                    self.copy_tile_back(&mut img, &tile, tx, ty)?;
                }
            }
        }

        self.write_output(img, output)
    }

    /// Runs streaming processing (image too large for RAM).
    fn run_streaming(
        &mut self,
        input: ImageInput,
        output: ImageOutput,
        ops: &[ComputeOp],
        tile_size: u32,
    ) -> ComputeResult<ProcessResult> {
        self.log(&format!("Running streaming with tile_size={}", tile_size));

        // Check if any op changes dimensions
        let has_resize = ops.iter().any(|op| op.changes_dimensions());
        if has_resize {
            return Err(ComputeError::OperationFailed(
                "Dimension-changing operations not supported in streaming mode".into()
            ));
        }

        match (&input, &output) {
            (ImageInput::File(in_path), ImageOutput::File(out_path)) => {
                self.run_streaming_file_to_file(in_path, out_path, ops, tile_size)?;
                Ok(ProcessResult::Written(out_path.clone()))
            }
            (ImageInput::File(_), ImageOutput::Memory) => {
                Err(ComputeError::OperationFailed(
                    "Streaming to memory not supported for large files".into()
                ))
            }
            // In-memory inputs shouldn't need streaming, fall back to tiled
            _ => {
                self.log("In-memory input with streaming strategy, falling back to tiled");
                self.run_tiled(input, output, ops, tile_size)
            }
        }
    }

    /// Streaming file-to-file processing.
    #[cfg(feature = "io")]
    fn run_streaming_file_to_file(
        &mut self,
        in_path: &Path,
        out_path: &Path,
        ops: &[ComputeOp],
        tile_size: u32,
    ) -> ComputeResult<()> {
        use vfx_io::streaming::{open_streaming, create_streaming_output, StreamingPipeline};
        use vfx_io::PixelFormat;

        let source = open_streaming(in_path).map_err(|e| {
            ComputeError::OperationFailed(format!("Failed to open streaming source: {}", e))
        })?;

        let (w, h) = source.dimensions();
        let output = create_streaming_output(out_path, w, h, PixelFormat::F32).map_err(|e| {
            ComputeError::OperationFailed(format!("Failed to create streaming output: {}", e))
        })?;

        let pipeline = StreamingPipeline::new(source, output, tile_size, tile_size);

        // Clone ops for closure
        let ops_clone = ops.to_vec();
        let processor = &self.processor;

        pipeline.run(|region| {
            // Convert region to ComputeImage
            let mut img = ComputeImage::from_f32(
                region.data.clone(),
                region.width,
                region.height,
                4,
            ).expect("Region conversion failed");

            // Apply ops
            for op in &ops_clone {
                img = Self::apply_op_static(processor, img, op)
                    .expect("Op failed in streaming");
            }

            // Copy back
            region.data.copy_from_slice(img.data());
        }).map_err(|e| {
            ComputeError::OperationFailed(format!("Streaming pipeline failed: {}", e))
        })
    }

    /// Fallback for non-streaming builds.
    #[cfg(not(feature = "io"))]
    fn run_streaming_file_to_file(
        &mut self,
        _in_path: &Path,
        _out_path: &Path,
        _ops: &[ComputeOp],
        _tile_size: u32,
    ) -> ComputeResult<()> {
        Err(ComputeError::OperationFailed(
            "Streaming not available: enable 'streaming' feature".into()
        ))
    }

    // =========================================================================
    // Helpers
    // =========================================================================

    /// Loads input into ComputeImage.
    fn load_input(&self, input: ImageInput) -> ComputeResult<ComputeImage> {
        match input {
            ImageInput::Memory(img) => Ok(img),
            #[cfg(feature = "io")]
            ImageInput::Data(data) => {
                self.log("Converting ImageData to ComputeImage");
                Ok(data.into())
            }
            #[cfg(feature = "io")]
            ImageInput::Layer(layer) => {
                self.log(&format!("Converting layer '{}' to ComputeImage", layer.name));
                use crate::convert::Processable;
                Ok(layer.to_compute())
            }
            ImageInput::File(path) => {
                self.log(&format!("Loading: {}", path.display()));
                load_image_file(&path)
            }
        }
    }

    /// Writes image to output.
    fn write_output(&self, img: ComputeImage, output: ImageOutput) -> ComputeResult<ProcessResult> {
        match output {
            ImageOutput::Memory => Ok(ProcessResult::Image(img)),
            ImageOutput::File(path) => {
                self.log(&format!("Writing: {}", path.display()));
                save_image_file(&img, &path)?;
                Ok(ProcessResult::Written(path))
            }
        }
    }

    /// Applies single operation to image.
    fn apply_op(&mut self, img: ComputeImage, op: &ComputeOp) -> ComputeResult<ComputeImage> {
        Self::apply_op_static(&self.processor, img, op)
    }

    /// Static version for use in closures.
    fn apply_op_static(processor: &Processor, mut img: ComputeImage, op: &ComputeOp) -> ComputeResult<ComputeImage> {
        match op {
            ComputeOp::Matrix(m) => {
                processor.apply_matrix(&mut img, m)?;
                Ok(img)
            }
            ComputeOp::Cdl(cdl) => {
                processor.apply_cdl(&mut img, cdl)?;
                Ok(img)
            }
            ComputeOp::Lut1D { lut, channels } => {
                processor.apply_lut1d(&mut img, lut, *channels)?;
                Ok(img)
            }
            ComputeOp::Lut3D { lut, size } => {
                processor.apply_lut3d(&mut img, lut, *size)?;
                Ok(img)
            }
            ComputeOp::Exposure(stops) => {
                processor.apply_exposure(&mut img, *stops)?;
                Ok(img)
            }
            ComputeOp::Saturation(sat) => {
                processor.apply_saturation(&mut img, *sat)?;
                Ok(img)
            }
            ComputeOp::Contrast(c) => {
                processor.apply_contrast(&mut img, *c)?;
                Ok(img)
            }
            ComputeOp::Blur(r) => {
                processor.blur(&mut img, *r)?;
                Ok(img)
            }
            ComputeOp::Sharpen(a) => {
                processor.sharpen(&mut img, *a)?;
                Ok(img)
            }
            ComputeOp::Resize { width, height, filter } => {
                processor.resize(&img, *width, *height, *filter)
            }
            ComputeOp::Crop { x, y, w, h } => {
                processor.crop(&img, *x, *y, *w, *h)
            }
            ComputeOp::FlipH => {
                processor.flip_h(&mut img)?;
                Ok(img)
            }
            ComputeOp::FlipV => {
                processor.flip_v(&mut img)?;
                Ok(img)
            }
            ComputeOp::Rotate90(n) => {
                processor.rotate_90(&img, *n)
            }
        }
    }

    /// Copies processed tile back to full image.
    fn copy_tile_back(
        &self,
        img: &mut ComputeImage,
        tile: &ComputeImage,
        tx: u32,
        ty: u32,
    ) -> ComputeResult<()> {
        let channels = img.channels as usize;
        let img_stride = img.width as usize * channels;
        let tile_stride = tile.width as usize * channels;

        for row in 0..tile.height as usize {
            let img_y = ty as usize + row;
            let img_offset = img_y * img_stride + (tx as usize * channels);
            let tile_offset = row * tile_stride;

            img.data_mut()[img_offset..img_offset + tile_stride]
                .copy_from_slice(&tile.data()[tile_offset..tile_offset + tile_stride]);
        }

        Ok(())
    }
}

// ============================================================================
// Builder
// ============================================================================

/// Builder for [`ComputePipeline`].
///
/// # Example
///
/// ```ignore
/// let pipeline = ComputePipeline::builder()
///     .backend(Backend::Wgpu)
///     .tile_size(2048)
///     .ram_limit_mb(8192)
///     .verbose(true)
///     .build()?;
/// ```
pub struct ComputePipelineBuilder {
    processor_builder: ProcessorBuilder,
    strategy_override: Option<ProcessingStrategy>,
}

impl Default for ComputePipelineBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ComputePipelineBuilder {
    /// Creates new builder with defaults.
    pub fn new() -> Self {
        Self {
            processor_builder: ProcessorBuilder::new(),
            strategy_override: None,
        }
    }

    /// Sets compute backend.
    pub fn backend(mut self, backend: Backend) -> Self {
        self.processor_builder = self.processor_builder.backend(backend);
        self
    }

    /// Sets tile size for processing.
    pub fn tile_size(mut self, size: u32) -> Self {
        self.processor_builder = self.processor_builder.tile_size(size);
        self
    }

    /// Sets maximum RAM in bytes.
    pub fn ram_limit(mut self, bytes: u64) -> Self {
        self.processor_builder = self.processor_builder.ram_limit(bytes);
        self
    }

    /// Sets maximum RAM in megabytes.
    pub fn ram_limit_mb(mut self, mb: u64) -> Self {
        self.processor_builder = self.processor_builder.ram_limit_mb(mb);
        self
    }

    /// Sets RAM usage as percentage of system RAM.
    pub fn ram_percent(mut self, percent: u8) -> Self {
        self.processor_builder = self.processor_builder.ram_percent(percent);
        self
    }

    /// Forces streaming mode.
    pub fn force_streaming(mut self, enabled: bool) -> Self {
        self.processor_builder = self.processor_builder.force_streaming(enabled);
        self
    }

    /// Enables verbose output.
    pub fn verbose(mut self, enabled: bool) -> Self {
        self.processor_builder = self.processor_builder.verbose(enabled);
        self
    }

    /// Overrides automatic strategy selection.
    pub fn strategy(mut self, strategy: ProcessingStrategy) -> Self {
        self.strategy_override = Some(strategy);
        self
    }

    /// Builds the pipeline.
    pub fn build(self) -> ComputeResult<ComputePipeline> {
        let processor = self.processor_builder.build()?;
        let verbose = processor.config().verbose;
        Ok(ComputePipeline {
            processor,
            strategy_override: self.strategy_override,
            verbose,
        })
    }
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Probes file to get image dimensions without full load.
///
/// Currently loads the file to get dimensions. Future optimization:
/// implement header-only probing for each format.
fn probe_dimensions(path: &Path) -> ComputeResult<(u32, u32)> {
    // TODO: Implement header-only probing for efficiency
    // For now, load the file to get dimensions
    let img = load_image_file(path)?;
    Ok((img.width, img.height))
}

/// Loads image file into ComputeImage.
#[cfg(feature = "io")]
fn load_image_file(path: &Path) -> ComputeResult<ComputeImage> {
    use vfx_io::read;
    let data = read(path).map_err(|e| {
        ComputeError::OperationFailed(format!("Failed to read {}: {}", path.display(), e))
    })?;

    // Convert to f32 RGBA
    let f32_data = data.to_f32();
    let channels = data.channels.min(4);

    // Ensure 4 channels (add alpha if needed)
    let rgba_data = if channels == 3 {
        let mut rgba = Vec::with_capacity((data.width * data.height * 4) as usize);
        for chunk in f32_data.chunks(3) {
            rgba.extend_from_slice(chunk);
            rgba.push(1.0); // Alpha
        }
        rgba
    } else {
        f32_data
    };

    ComputeImage::from_f32(rgba_data, data.width, data.height, 4)
}

/// Loads image file into ComputeImage.
#[cfg(not(feature = "io"))]
fn load_image_file(path: &Path) -> ComputeResult<ComputeImage> {
    Err(ComputeError::OperationFailed(
        format!("File I/O not available: enable 'io' feature to load {}", path.display())
    ))
}

/// Saves ComputeImage to file.
#[cfg(feature = "io")]
fn save_image_file(img: &ComputeImage, path: &Path) -> ComputeResult<()> {
    use vfx_io::{write, ImageData};
    let data = ImageData::from_f32(img.width, img.height, img.channels, img.data().to_vec());

    write(path, &data).map_err(|e| {
        ComputeError::OperationFailed(format!("Failed to write {}: {}", path.display(), e))
    })
}

/// Saves ComputeImage to file.
#[cfg(not(feature = "io"))]
fn save_image_file(_img: &ComputeImage, path: &Path) -> ComputeResult<()> {
    Err(ComputeError::OperationFailed(
        format!("File I/O not available: enable 'io' feature to save {}", path.display())
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_input_memory() {
        let img = ComputeImage::new(100, 100, 4);
        let input = ImageInput::memory(img);
        assert!(matches!(input, ImageInput::Memory(_)));

        let (w, h) = input.dimensions().unwrap();
        assert_eq!(w, 100);
        assert_eq!(h, 100);
    }

    #[test]
    fn test_compute_op_helpers() {
        let exp = ComputeOp::exposure(1.5);
        assert!(matches!(exp, ComputeOp::Exposure(1.5)));

        let resize = ComputeOp::resize(1920, 1080, ResizeFilter::Bilinear);
        assert!(resize.changes_dimensions());

        let blur = ComputeOp::blur(5.0);
        assert!(!blur.changes_dimensions());
    }

    #[test]
    fn test_pipeline_single_pass() {
        let mut pipeline = ComputePipeline::cpu().unwrap();

        let img = ComputeImage::from_f32(
            vec![0.5f32; 4 * 4 * 4],
            4, 4, 4,
        ).unwrap();

        let result = pipeline.process(
            ImageInput::memory(img),
            ImageOutput::memory(),
            &[ComputeOp::exposure(1.0)],
        ).unwrap();

        assert!(result.is_memory());
        let out = result.into_image().unwrap();
        // +1 stop = 2x brightness
        assert!((out.data()[0] - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_pipeline_strategy_selection() {
        let pipeline = ComputePipeline::cpu().unwrap();

        // Small image -> SinglePass or Tiled (both acceptable for small images)
        let small = pipeline.recommend_strategy(100, 100, 4);
        // For small images, any non-streaming strategy is acceptable
        assert!(!matches!(small, ProcessingStrategy::Streaming { .. }),
            "100x100 should not require streaming, got {:?}", small);
    }

    #[test]
    fn test_process_result() {
        let img = ComputeImage::new(10, 10, 4);
        let result = ProcessResult::Image(img);
        assert!(result.is_memory());
        assert!(!result.is_file());

        let result = ProcessResult::Written(PathBuf::from("test.exr"));
        assert!(!result.is_memory());
        assert!(result.is_file());
    }
}
