//! Backend detection and auto-selection.
//!
//! Detects available compute backends and selects the best one based on:
//! - Availability (is the backend compiled and working?)
//! - Priority (CUDA > wgpu discrete > wgpu integrated > CPU)
//! - VRAM availability
//! - Software renderer avoidance

use super::Backend;
#[allow(unused_imports)]
use super::vram::detect_vram;
use super::memory::backend_override;

/// Information about a compute backend.
#[derive(Debug, Clone)]
pub struct BackendInfo {
    /// Backend type.
    pub backend: Backend,
    /// Human-readable name.
    pub name: String,
    /// Whether backend is available.
    pub available: bool,
    /// Priority for auto-selection (higher = preferred).
    pub priority: u32,
    /// Description.
    pub description: String,
    /// Device name (GPU model).
    pub device: Option<String>,
    /// Total VRAM in bytes.
    pub vram_total: Option<u64>,
    /// Free VRAM in bytes.
    pub vram_free: Option<u64>,
    /// Reason if unavailable.
    pub unavailable_reason: Option<String>,
}

impl BackendInfo {
    fn cpu() -> Self {
        Self {
            backend: Backend::Cpu,
            name: "CPU".to_string(),
            available: true,
            priority: 10,
            description: "CPU with rayon parallelization".to_string(),
            device: None,
            vram_total: Some(super::memory::system_memory()),
            vram_free: None,
            unavailable_reason: None,
        }
    }
}

/// Detect all available backends.
pub fn detect_backends() -> Vec<BackendInfo> {
    let mut backends = vec![BackendInfo::cpu()];
    
    #[cfg(feature = "wgpu")]
    {
        backends.push(detect_wgpu());
    }
    
    #[cfg(feature = "cuda")]
    {
        backends.push(detect_cuda());
    }
    
    // Sort by priority (highest first)
    backends.sort_by(|a, b| b.priority.cmp(&a.priority));
    backends
}

/// Select the best available backend.
///
/// Respects VFX_BACKEND environment variable if set.
pub fn select_best_backend() -> Backend {
    // Check environment override
    if let Some(override_name) = backend_override() {
        match override_name.to_lowercase().as_str() {
            "cpu" => return Backend::Cpu,
            "wgpu" | "gpu" => return Backend::Wgpu,
            "cuda" => return Backend::Cuda,
            _ => {} // Ignore invalid values
        }
    }
    
    let backends = detect_backends();
    
    backends
        .into_iter()
        .filter(|b| b.available)
        .max_by_key(|b| b.priority)
        .map(|b| b.backend)
        .unwrap_or(Backend::Cpu)
}

/// Get description of available backends.
pub fn describe_backends() -> String {
    let backends = detect_backends();
    let mut desc = String::new();
    
    for info in backends {
        let status = if info.available { "+" } else { "-" };
        desc.push_str(&format!("[{}] {}: {}\n", status, info.name, info.description));
        
        if let Some(device) = &info.device {
            desc.push_str(&format!("    Device: {}\n", device));
        }
        
        if let Some(vram) = info.vram_total {
            desc.push_str(&format!("    VRAM: {}\n", super::memory::format_bytes(vram)));
        }
        
        if let Some(reason) = &info.unavailable_reason {
            desc.push_str(&format!("    Reason: {}\n", reason));
        }
    }
    
    desc
}

// =============================================================================
// wgpu Detection
// =============================================================================

#[cfg(feature = "wgpu")]
fn detect_wgpu() -> BackendInfo {
    use wgpu::{Backends, Instance, InstanceDescriptor, RequestAdapterOptions, PowerPreference};
    
    let instance = Instance::new(&InstanceDescriptor {
        backends: Backends::all(),
        ..Default::default()
    });
    
    let adapter = match pollster::block_on(instance.request_adapter(&RequestAdapterOptions {
        power_preference: PowerPreference::HighPerformance,
        compatible_surface: None,
        force_fallback_adapter: false,
    })) {
        Some(a) => a,
        None => {
            return BackendInfo {
                backend: Backend::Wgpu,
                name: "wgpu".to_string(),
                available: false,
                priority: 0,
                description: "GPU via wgpu (Vulkan/Metal/DX12)".to_string(),
                device: None,
                vram_total: None,
                vram_free: None,
                unavailable_reason: Some("No suitable GPU adapter found".to_string()),
            };
        }
    };
    
    let info = adapter.get_info();
    
    // Check for software renderer
    if super::vram::is_software_renderer(&info) {
        return BackendInfo {
            backend: Backend::Wgpu,
            name: "wgpu".to_string(),
            available: false,
            priority: 0,
            description: "GPU via wgpu (software renderer detected)".to_string(),
            device: Some(info.name.clone()),
            vram_total: None,
            vram_free: None,
            unavailable_reason: Some(format!(
                "Software renderer '{}' - use CPU backend instead", 
                info.name
            )),
        };
    }
    
    let vram = detect_vram();
    
    // Determine priority based on device type
    let priority = match info.device_type {
        wgpu::DeviceType::DiscreteGpu => 100,
        wgpu::DeviceType::IntegratedGpu => 50,
        wgpu::DeviceType::VirtualGpu => 30,
        _ => 20,
    };
    
    BackendInfo {
        backend: Backend::Wgpu,
        name: "wgpu".to_string(),
        available: true,
        priority,
        description: format!("GPU via wgpu ({:?})", info.backend),
        device: Some(info.name.clone()),
        vram_total: Some(vram.total),
        vram_free: vram.free,
        unavailable_reason: None,
    }
}

// =============================================================================
// CUDA Detection
// =============================================================================

#[cfg(feature = "cuda")]
fn detect_cuda() -> BackendInfo {
    match cudarc::driver::CudaDevice::new(0) {
        Ok(device) => {
            // Get device properties
            let name = device.name().unwrap_or_else(|_| "Unknown CUDA Device".to_string());
            
            // Get memory info
            let (free, total) = device.memory_free_and_total()
                .unwrap_or((0, 0));
            
            BackendInfo {
                backend: Backend::Cuda,
                name: "CUDA".to_string(),
                available: true,
                priority: 150, // CUDA gets highest priority
                description: "NVIDIA GPU via CUDA".to_string(),
                device: Some(name),
                vram_total: Some(total as u64),
                vram_free: Some(free as u64),
                unavailable_reason: None,
            }
        }
        Err(e) => {
            BackendInfo {
                backend: Backend::Cuda,
                name: "CUDA".to_string(),
                available: false,
                priority: 0,
                description: "NVIDIA GPU via CUDA".to_string(),
                device: None,
                vram_total: None,
                vram_free: None,
                unavailable_reason: Some(format!("CUDA init failed: {}", e)),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_backends() {
        let backends = detect_backends();
        
        // Should always have at least CPU
        assert!(!backends.is_empty());
        assert!(backends.iter().any(|b| b.backend == Backend::Cpu));
        
        // CPU should always be available
        let cpu = backends.iter().find(|b| b.backend == Backend::Cpu).unwrap();
        assert!(cpu.available);
    }

    #[test]
    fn test_select_best() {
        let best = select_best_backend();
        
        // Should return something
        assert!(best == Backend::Cpu || best == Backend::Wgpu || best == Backend::Cuda);
        
        // Selected backend should be available
        assert!(best.is_available());
    }

    #[test]
    fn test_describe_backends() {
        let desc = describe_backends();
        
        // Should contain CPU info
        assert!(desc.contains("CPU"));
        
        // Should have status markers
        assert!(desc.contains("[+]") || desc.contains("[-]"));
    }
}
