//! wgpu backend implementation.
//!
//! GPU-accelerated image processing using wgpu compute shaders.

use std::sync::Arc;
use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

use super::{GpuLimits, ProcessingBackend, ImageHandle};
use super::gpu_primitives::{GpuPrimitives, AsAny};
use crate::{GpuError, GpuResult};
use crate::shaders;

// =============================================================================
// Uniform Buffers
// =============================================================================

/// Dimensions uniform: [width, height, channels, extra]
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct DimsUniform {
    dims: [u32; 4],
}

/// CDL parameters uniform.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct CdlUniform {
    slope: [f32; 3],
    _pad0: f32,
    offset: [f32; 3],
    _pad1: f32,
    power: [f32; 3],
    saturation: f32,
}

/// Resize uniforms.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct ResizeUniform {
    src_dims: [u32; 4],  // sw, sh, c, 0
    dst_dims: [u32; 4],  // dw, dh, 0, 0
}

// =============================================================================
// WgpuImage Handle
// =============================================================================

/// GPU buffer handle for image data.
pub struct WgpuImage {
    buffer: wgpu::Buffer,
    width: u32,
    height: u32,
    channels: u32,
    size_bytes: u64,
}

impl AsAny for WgpuImage {
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}

impl ImageHandle for WgpuImage {
    fn dimensions(&self) -> (u32, u32, u32) {
        (self.width, self.height, self.channels)
    }
    
    fn width(&self) -> u32 { self.width }
    fn height(&self) -> u32 { self.height }
    fn channels(&self) -> u32 { self.channels }
    fn size_bytes(&self) -> u64 { self.size_bytes }
}

// =============================================================================
// Pipelines
// =============================================================================

struct Pipelines {
    matrix: wgpu::ComputePipeline,
    cdl: wgpu::ComputePipeline,
    lut1d: wgpu::ComputePipeline,
    lut3d: wgpu::ComputePipeline,
    resize: wgpu::ComputePipeline,
    blur_h: wgpu::ComputePipeline,
    blur_v: wgpu::ComputePipeline,
}

// =============================================================================
// WgpuPrimitives
// =============================================================================

/// wgpu GPU primitives implementation.
pub struct WgpuPrimitives {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    pipelines: Pipelines,
    limits: GpuLimits,
}

impl WgpuPrimitives {
    /// Check if wgpu is available.
    pub fn is_available() -> bool {
        pollster::block_on(async {
            let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
                backends: wgpu::Backends::all(),
                ..Default::default()
            });
            instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::HighPerformance,
                    compatible_surface: None,
                    force_fallback_adapter: false,
                })
                .await
                .is_some()
        })
    }

    /// Create new wgpu primitives.
    pub fn new() -> GpuResult<Self> {
        pollster::block_on(Self::new_async())
    }

    /// Create new wgpu primitives asynchronously.
    pub async fn new_async() -> GpuResult<Self> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .ok_or(GpuError::NoAdapter)?;

        let adapter_limits = adapter.limits();
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("vfx_gpu_device"),
                required_features: wgpu::Features::empty(),
                required_limits: adapter_limits.clone(),
                memory_hints: wgpu::MemoryHints::Performance,
                ..Default::default()
            }, None)
            .await
            .map_err(|e| GpuError::DeviceCreation(e.to_string()))?;

        let device = Arc::new(device);
        let queue = Arc::new(queue);

        // Detect memory
        let adapter_info = adapter.get_info();
        let available_memory = estimate_vram(&adapter_info, adapter_limits.max_buffer_size);

        let limits = GpuLimits {
            max_tile_dim: adapter_limits.max_texture_dimension_2d,
            max_buffer_bytes: adapter_limits.max_buffer_size,
            available_memory,
        };

        // Create pipelines
        let pipelines = Self::create_pipelines(&device)?;

        Ok(Self {
            device,
            queue,
            pipelines,
            limits,
        })
    }

    fn create_pipelines(device: &wgpu::Device) -> GpuResult<Pipelines> {
        let create_pipeline = |source: &str, label: &str| -> GpuResult<wgpu::ComputePipeline> {
            let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(label),
                source: wgpu::ShaderSource::Wgsl(source.into()),
            });

            Ok(device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(label),
                layout: None, // Auto layout
                module: &module,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                cache: None,
            }))
        };

        Ok(Pipelines {
            matrix: create_pipeline(shaders::COLOR_MATRIX, "matrix_pipeline")?,
            cdl: create_pipeline(shaders::CDL, "cdl_pipeline")?,
            lut1d: create_pipeline(shaders::LUT1D, "lut1d_pipeline")?,
            lut3d: create_pipeline(shaders::LUT3D, "lut3d_pipeline")?,
            resize: create_pipeline(shaders::RESIZE, "resize_pipeline")?,
            blur_h: create_pipeline(shaders::BLUR_H, "blur_h_pipeline")?,
            blur_v: create_pipeline(shaders::BLUR_V, "blur_v_pipeline")?,
        })
    }

    /// Create dims uniform buffer.
    fn create_dims_buffer(&self, w: u32, h: u32, c: u32, extra: u32) -> wgpu::Buffer {
        let uniform = DimsUniform { dims: [w, h, c, extra] };
        self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("dims_uniform"),
            contents: bytemuck::bytes_of(&uniform),
            usage: wgpu::BufferUsages::UNIFORM,
        })
    }

    /// Execute compute dispatch and wait.
    fn dispatch_and_wait(&self, pipeline: &wgpu::ComputePipeline, bind_group: &wgpu::BindGroup, workgroups: (u32, u32, u32)) {
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("compute_encoder"),
        });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("compute_pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, bind_group, &[]);
            pass.dispatch_workgroups(workgroups.0, workgroups.1, workgroups.2);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        self.device.poll(wgpu::Maintain::Wait);
    }
}

impl GpuPrimitives for WgpuPrimitives {
    type Handle = WgpuImage;

    fn upload(&self, data: &[f32], width: u32, height: u32, channels: u32) -> GpuResult<Self::Handle> {
        let expected = (width * height * channels) as usize;
        if data.len() != expected {
            return Err(GpuError::BufferSizeMismatch { expected, actual: data.len() });
        }

        let size_bytes = (data.len() * 4) as u64;
        
        let buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("image_buffer"),
            contents: bytemuck::cast_slice(data),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
        });

        Ok(WgpuImage { buffer, width, height, channels, size_bytes })
    }

    fn download(&self, handle: &Self::Handle) -> GpuResult<Vec<f32>> {
        let size = handle.size_bytes;
        
        // Create staging buffer
        let staging = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("staging_buffer"),
            size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        // Copy to staging
        let mut encoder = self.device.create_command_encoder(&Default::default());
        encoder.copy_buffer_to_buffer(&handle.buffer, 0, &staging, 0, size);
        self.queue.submit(std::iter::once(encoder.finish()));

        // Map and read
        let slice = staging.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |r| { let _ = tx.send(r); });
        self.device.poll(wgpu::Maintain::Wait);

        rx.recv()
            .map_err(|_| GpuError::OperationFailed("Map channel closed".into()))?
            .map_err(|e| GpuError::OperationFailed(format!("Map failed: {e}")))?;

        let data = slice.get_mapped_range();
        let result: Vec<f32> = bytemuck::cast_slice(&data).to_vec();
        drop(data);
        staging.unmap();

        Ok(result)
    }

    fn allocate(&self, width: u32, height: u32, channels: u32) -> GpuResult<Self::Handle> {
        let size_bytes = (width as u64) * (height as u64) * (channels as u64) * 4;
        
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("output_buffer"),
            size: size_bytes,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Ok(WgpuImage { buffer, width, height, channels, size_bytes })
    }

    fn exec_matrix(&self, src: &Self::Handle, dst: &mut Self::Handle, matrix: &[f32; 16]) -> GpuResult<()> {
        let (w, h, c) = src.dimensions();
        let total = w * h;

        let dims_buf = self.create_dims_buffer(w, h, c, 0);
        
        let matrix_buf = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("matrix_uniform"),
            contents: bytemuck::cast_slice(matrix),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        let layout = self.pipelines.matrix.get_bind_group_layout(0);
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("matrix_bind_group"),
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: src.buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: dst.buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: dims_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 3, resource: matrix_buf.as_entire_binding() },
            ],
        });

        let workgroups = (total.div_ceil(256), 1, 1);
        self.dispatch_and_wait(&self.pipelines.matrix, &bind_group, workgroups);
        Ok(())
    }

    fn exec_cdl(&self, src: &Self::Handle, dst: &mut Self::Handle, slope: [f32; 3], offset: [f32; 3], power: [f32; 3], sat: f32) -> GpuResult<()> {
        let (w, h, c) = src.dimensions();
        let total = w * h;

        let dims_buf = self.create_dims_buffer(w, h, c, 0);
        
        let cdl = CdlUniform {
            slope, _pad0: 0.0,
            offset, _pad1: 0.0,
            power, saturation: sat,
        };
        let cdl_buf = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("cdl_uniform"),
            contents: bytemuck::bytes_of(&cdl),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        let layout = self.pipelines.cdl.get_bind_group_layout(0);
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("cdl_bind_group"),
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: src.buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: dst.buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: dims_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 3, resource: cdl_buf.as_entire_binding() },
            ],
        });

        let workgroups = (total.div_ceil(256), 1, 1);
        self.dispatch_and_wait(&self.pipelines.cdl, &bind_group, workgroups);
        Ok(())
    }

    fn exec_lut1d(&self, src: &Self::Handle, dst: &mut Self::Handle, lut: &[f32], channels: u32) -> GpuResult<()> {
        let (w, h, c) = src.dimensions();
        let total = w * h;
        let lut_size = lut.len() as u32 / channels;

        let dims_buf = self.create_dims_buffer(w, h, c, lut_size);
        
        let lut_buf = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("lut1d_buffer"),
            contents: bytemuck::cast_slice(lut),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let layout = self.pipelines.lut1d.get_bind_group_layout(0);
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("lut1d_bind_group"),
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: src.buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: dst.buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: dims_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 3, resource: lut_buf.as_entire_binding() },
            ],
        });

        let workgroups = (total.div_ceil(256), 1, 1);
        self.dispatch_and_wait(&self.pipelines.lut1d, &bind_group, workgroups);
        Ok(())
    }

    fn exec_lut3d(&self, src: &Self::Handle, dst: &mut Self::Handle, lut: &[f32], size: u32) -> GpuResult<()> {
        let (w, h, c) = src.dimensions();
        let total = w * h;

        let dims_buf = self.create_dims_buffer(w, h, c, size);
        
        let lut_buf = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("lut3d_buffer"),
            contents: bytemuck::cast_slice(lut),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let layout = self.pipelines.lut3d.get_bind_group_layout(0);
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("lut3d_bind_group"),
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: src.buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: dst.buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: dims_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 3, resource: lut_buf.as_entire_binding() },
            ],
        });

        let workgroups = (total.div_ceil(256), 1, 1);
        self.dispatch_and_wait(&self.pipelines.lut3d, &bind_group, workgroups);
        Ok(())
    }

    fn exec_resize(&self, src: &Self::Handle, dst: &mut Self::Handle, _filter: u32) -> GpuResult<()> {
        let (sw, sh, c) = src.dimensions();
        let (dw, dh, _) = dst.dimensions();

        let src_dims_buf = self.create_dims_buffer(sw, sh, c, 0);
        let dst_dims_buf = self.create_dims_buffer(dw, dh, 0, 0);

        let layout = self.pipelines.resize.get_bind_group_layout(0);
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("resize_bind_group"),
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: src.buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: dst.buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: src_dims_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 3, resource: dst_dims_buf.as_entire_binding() },
            ],
        });

        // Resize uses 16x16 workgroups
        let workgroups = (dw.div_ceil(16), dh.div_ceil(16), 1);
        self.dispatch_and_wait(&self.pipelines.resize, &bind_group, workgroups);
        Ok(())
    }

    fn exec_blur(&self, src: &Self::Handle, dst: &mut Self::Handle, radius: f32) -> GpuResult<()> {
        let (w, h, c) = src.dimensions();
        let total = w * h;
        let r = radius.ceil() as i32;
        let sigma = radius / 3.0;

        // Generate Gaussian kernel
        let k_size = (r * 2 + 1) as usize;
        let mut kernel = vec![0.0f32; k_size];
        let mut sum = 0.0f32;
        for i in 0..k_size {
            let x = (i as i32 - r) as f32;
            let g = (-x * x / (2.0 * sigma * sigma)).exp();
            kernel[i] = g;
            sum += g;
        }
        for k in &mut kernel { *k /= sum; }

        let dims_buf = self.create_dims_buffer(w, h, c, r as u32);
        
        let kernel_buf = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("kernel_buffer"),
            contents: bytemuck::cast_slice(&kernel),
            usage: wgpu::BufferUsages::STORAGE,
        });

        // Temp buffer for horizontal pass
        let temp = self.allocate(w, h, c)?;

        // Horizontal pass: src -> temp
        let layout_h = self.pipelines.blur_h.get_bind_group_layout(0);
        let bind_group_h = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("blur_h_bind_group"),
            layout: &layout_h,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: src.buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: temp.buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: dims_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 3, resource: kernel_buf.as_entire_binding() },
            ],
        });
        self.dispatch_and_wait(&self.pipelines.blur_h, &bind_group_h, (total.div_ceil(256), 1, 1));

        // Vertical pass: temp -> dst
        let layout_v = self.pipelines.blur_v.get_bind_group_layout(0);
        let bind_group_v = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("blur_v_bind_group"),
            layout: &layout_v,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: temp.buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: dst.buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: dims_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 3, resource: kernel_buf.as_entire_binding() },
            ],
        });
        self.dispatch_and_wait(&self.pipelines.blur_v, &bind_group_v, (total.div_ceil(256), 1, 1));

        Ok(())
    }

    fn limits(&self) -> &GpuLimits { &self.limits }
    fn name(&self) -> &'static str { "wgpu" }
}

// =============================================================================
// WgpuBackend (ProcessingBackend impl)
// =============================================================================

/// wgpu processing backend.
pub struct WgpuBackend {
    primitives: WgpuPrimitives,
}

impl WgpuBackend {
    /// Check if wgpu is available.
    pub fn is_available() -> bool {
        WgpuPrimitives::is_available()
    }

    /// Create new wgpu backend.
    pub fn new() -> GpuResult<Self> {
        Ok(Self { primitives: WgpuPrimitives::new()? })
    }
}

impl ProcessingBackend for WgpuBackend {
    fn name(&self) -> &'static str { "wgpu" }
    
    fn available_memory(&self) -> u64 { self.primitives.limits.available_memory }
    
    fn limits(&self) -> &GpuLimits { &self.primitives.limits }

    fn upload(&self, data: &[f32], width: u32, height: u32, channels: u32) -> GpuResult<Box<dyn ImageHandle>> {
        let handle = self.primitives.upload(data, width, height, channels)?;
        Ok(Box::new(handle))
    }

    fn download(&self, handle: &dyn ImageHandle) -> GpuResult<Vec<f32>> {
        let wgpu_handle = handle.as_any()
            .downcast_ref::<WgpuImage>()
            .ok_or_else(|| GpuError::OperationFailed("Invalid handle type".into()))?;
        self.primitives.download(wgpu_handle)
    }

    fn apply_matrix(&self, handle: &mut dyn ImageHandle, matrix: &[f32; 16]) -> GpuResult<()> {
        let wgpu_handle = handle.as_any_mut()
            .downcast_mut::<WgpuImage>()
            .ok_or_else(|| GpuError::OperationFailed("Invalid handle type".into()))?;
        
        let (w, h, c) = wgpu_handle.dimensions();
        let mut dst = self.primitives.allocate(w, h, c)?;
        self.primitives.exec_matrix(wgpu_handle, &mut dst, matrix)?;
        
        // Swap buffers
        std::mem::swap(&mut wgpu_handle.buffer, &mut dst.buffer);
        Ok(())
    }

    fn apply_cdl(&self, handle: &mut dyn ImageHandle, slope: [f32; 3], offset: [f32; 3], power: [f32; 3], sat: f32) -> GpuResult<()> {
        let wgpu_handle = handle.as_any_mut()
            .downcast_mut::<WgpuImage>()
            .ok_or_else(|| GpuError::OperationFailed("Invalid handle type".into()))?;
        
        let (w, h, c) = wgpu_handle.dimensions();
        let mut dst = self.primitives.allocate(w, h, c)?;
        self.primitives.exec_cdl(wgpu_handle, &mut dst, slope, offset, power, sat)?;
        
        std::mem::swap(&mut wgpu_handle.buffer, &mut dst.buffer);
        Ok(())
    }

    fn apply_lut1d(&self, handle: &mut dyn ImageHandle, lut: &[f32], channels: u32) -> GpuResult<()> {
        let wgpu_handle = handle.as_any_mut()
            .downcast_mut::<WgpuImage>()
            .ok_or_else(|| GpuError::OperationFailed("Invalid handle type".into()))?;
        
        let (w, h, c) = wgpu_handle.dimensions();
        let mut dst = self.primitives.allocate(w, h, c)?;
        self.primitives.exec_lut1d(wgpu_handle, &mut dst, lut, channels)?;
        
        std::mem::swap(&mut wgpu_handle.buffer, &mut dst.buffer);
        Ok(())
    }

    fn apply_lut3d(&self, handle: &mut dyn ImageHandle, lut: &[f32], size: u32) -> GpuResult<()> {
        let wgpu_handle = handle.as_any_mut()
            .downcast_mut::<WgpuImage>()
            .ok_or_else(|| GpuError::OperationFailed("Invalid handle type".into()))?;
        
        let (w, h, c) = wgpu_handle.dimensions();
        let mut dst = self.primitives.allocate(w, h, c)?;
        self.primitives.exec_lut3d(wgpu_handle, &mut dst, lut, size)?;
        
        std::mem::swap(&mut wgpu_handle.buffer, &mut dst.buffer);
        Ok(())
    }

    fn resize(&self, handle: &dyn ImageHandle, width: u32, height: u32, filter: u32) -> GpuResult<Box<dyn ImageHandle>> {
        let wgpu_handle = handle.as_any()
            .downcast_ref::<WgpuImage>()
            .ok_or_else(|| GpuError::OperationFailed("Invalid handle type".into()))?;
        
        let (_, _, c) = wgpu_handle.dimensions();
        let mut dst = self.primitives.allocate(width, height, c)?;
        self.primitives.exec_resize(wgpu_handle, &mut dst, filter)?;
        
        Ok(Box::new(dst))
    }

    fn blur(&self, handle: &mut dyn ImageHandle, radius: f32) -> GpuResult<()> {
        let wgpu_handle = handle.as_any_mut()
            .downcast_mut::<WgpuImage>()
            .ok_or_else(|| GpuError::OperationFailed("Invalid handle type".into()))?;
        
        let (w, h, c) = wgpu_handle.dimensions();
        let mut dst = self.primitives.allocate(w, h, c)?;
        self.primitives.exec_blur(wgpu_handle, &mut dst, radius)?;
        
        std::mem::swap(&mut wgpu_handle.buffer, &mut dst.buffer);
        Ok(())
    }
}

// =============================================================================
// VRAM Detection
// =============================================================================

fn estimate_vram(info: &wgpu::AdapterInfo, max_buffer_bytes: u64) -> u64 {
    // Check env override
    if let Ok(mb) = std::env::var("VFX_GPU_MEMORY_MB") {
        if let Ok(mb) = mb.parse::<u64>() {
            return mb.saturating_mul(1024 * 1024);
        }
    }

    let from_buffer = max_buffer_bytes.saturating_mul(2);

    let estimated = match info.device_type {
        wgpu::DeviceType::DiscreteGpu => from_buffer.clamp(2u64 << 30, 24u64 << 30),
        wgpu::DeviceType::IntegratedGpu => from_buffer.clamp(512u64 << 20, 4u64 << 30),
        wgpu::DeviceType::VirtualGpu => from_buffer.clamp(1u64 << 30, 8u64 << 30),
        _ => from_buffer.clamp(256u64 << 20, 2u64 << 30),
    };

    // 80% safe margin
    estimated.saturating_mul(80) / 100
}
