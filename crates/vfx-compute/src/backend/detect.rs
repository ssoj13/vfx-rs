//! Backend detection and auto-selection.

use super::Backend;

/// Information about a compute backend.
#[derive(Debug, Clone)]
pub struct BackendInfo {
    /// Backend type.
    pub backend: Backend,
    /// Human-readable name.
    pub name: &'static str,
    /// Whether backend is available.
    pub available: bool,
    /// Priority for auto-selection (higher = preferred).
    pub priority: u32,
    /// Description.
    pub description: &'static str,
}

/// Detect all available backends.
pub fn detect_backends() -> Vec<BackendInfo> {
    let mut backends = vec![
        BackendInfo {
            backend: Backend::Cpu,
            name: "CPU",
            available: true,
            priority: 10,
            description: "CPU with rayon parallelization",
        },
    ];
    
    #[cfg(feature = "wgpu")]
    {
        let wgpu_available = super::WgpuBackend::is_available();
        backends.push(BackendInfo {
            backend: Backend::Wgpu,
            name: "wgpu",
            available: wgpu_available,
            priority: if wgpu_available { 100 } else { 0 },
            description: "GPU via wgpu (Vulkan/Metal/DX12)",
        });
    }
    
    #[cfg(feature = "cuda")]
    {
        let cuda_available = super::CudaBackend::is_available();
        backends.push(BackendInfo {
            backend: Backend::Cuda,
            name: "CUDA",
            available: cuda_available,
            // CUDA gets higher priority than wgpu when available
            priority: if cuda_available { 150 } else { 0 },
            description: "NVIDIA GPU via CUDA",
        });
    }
    
    backends.sort_by(|a, b| b.priority.cmp(&a.priority));
    backends
}

/// Select the best available backend.
pub fn select_best_backend() -> Backend {
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
    }
    
    desc
}
