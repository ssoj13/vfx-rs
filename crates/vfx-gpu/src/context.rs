//! GPU context and device management

use std::sync::Arc;
use wgpu::{Device, Queue, Instance, Adapter, DeviceDescriptor, Features, Limits};

use crate::{GpuError, GpuResult};

/// GPU context holding device and queue
pub struct GpuContext {
    pub(crate) device: Arc<Device>,
    pub(crate) queue: Arc<Queue>,
    adapter_info: wgpu::AdapterInfo,
}

impl GpuContext {
    /// Create new GPU context with default settings
    pub fn new() -> GpuResult<Self> {
        Self::with_power_preference(wgpu::PowerPreference::HighPerformance)
    }

    /// Create context with power preference
    pub fn with_power_preference(power: wgpu::PowerPreference) -> GpuResult<Self> {
        pollster::block_on(Self::new_async(power))
    }

    /// Async context creation
    async fn new_async(power: wgpu::PowerPreference) -> GpuResult<Self> {
        let instance = Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: power,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .ok_or(GpuError::NoAdapter)?;

        let adapter_info = adapter.get_info();

        let (device, queue) = adapter
            .request_device(
                &DeviceDescriptor {
                    label: Some("vfx-gpu"),
                    required_features: Features::empty(),
                    required_limits: Limits::default(),
                    memory_hints: Default::default(),
                },
                None,
            )
            .await
            .map_err(|e| GpuError::DeviceCreation(e.to_string()))?;

        Ok(Self {
            device: Arc::new(device),
            queue: Arc::new(queue),
            adapter_info,
        })
    }

    /// Get adapter info (GPU name, vendor, etc.)
    pub fn adapter_info(&self) -> &wgpu::AdapterInfo {
        &self.adapter_info
    }

    /// Get device name
    pub fn device_name(&self) -> &str {
        &self.adapter_info.name
    }

    /// Get backend type (Vulkan, DX12, Metal, etc.)
    pub fn backend(&self) -> wgpu::Backend {
        self.adapter_info.backend
    }

    /// Check if running on integrated GPU
    pub fn is_integrated(&self) -> bool {
        self.adapter_info.device_type == wgpu::DeviceType::IntegratedGpu
    }

    /// Create a compute shader module
    pub(crate) fn create_shader(&self, source: &str) -> GpuResult<wgpu::ShaderModule> {
        Ok(self.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("compute_shader"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        }))
    }

    /// Submit work and wait for completion
    pub(crate) fn submit_and_wait(&self, encoder: wgpu::CommandEncoder) {
        self.queue.submit(std::iter::once(encoder.finish()));
        self.device.poll(wgpu::Maintain::Wait);
    }
}

impl std::fmt::Debug for GpuContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GpuContext")
            .field("device", &self.adapter_info.name)
            .field("backend", &self.adapter_info.backend)
            .finish()
    }
}
