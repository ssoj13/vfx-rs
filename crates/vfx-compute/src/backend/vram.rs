//! Cross-platform VRAM detection.
//!
//! Detects GPU video memory using platform-specific APIs:
//! - Windows: DXGI (DirectX Graphics Infrastructure)
//! - macOS: Metal API
//! - Linux NVIDIA: NVML (NVIDIA Management Library)
//! - Linux AMD/Intel: sysfs `/sys/class/drm/`
//!
//! Falls back to wgpu adapter limits if native detection fails.

use std::sync::OnceLock;

/// VRAM detection result.
#[derive(Debug, Clone)]
pub struct VramInfo {
    /// Total VRAM in bytes.
    pub total: u64,
    /// Free VRAM in bytes (if detectable).
    pub free: Option<u64>,
    /// GPU name.
    pub name: Option<String>,
    /// Detection method used.
    pub method: &'static str,
}

impl Default for VramInfo {
    fn default() -> Self {
        Self {
            total: 2 * 1024 * 1024 * 1024, // 2 GB default
            free: None,
            name: None,
            method: "default",
        }
    }
}

/// Cached VRAM info.
static VRAM_INFO: OnceLock<VramInfo> = OnceLock::new();

/// Detect VRAM, caching the result.
pub fn detect_vram() -> &'static VramInfo {
    VRAM_INFO.get_or_init(detect_vram_impl)
}

/// Get total VRAM in bytes.
pub fn total_vram() -> u64 {
    detect_vram().total
}

/// Get free VRAM if detectable.
pub fn free_vram() -> Option<u64> {
    detect_vram().free
}

/// Get available VRAM (free if known, else 80% of total).
pub fn available_vram() -> u64 {
    let info = detect_vram();
    info.free.unwrap_or((info.total as f64 * 0.8) as u64)
}

// =============================================================================
// Platform Detection
// =============================================================================

fn detect_vram_impl() -> VramInfo {
    // Try platform-specific detection first
    #[cfg(target_os = "windows")]
    if let Some(info) = detect_dxgi() {
        return info;
    }

    #[cfg(target_os = "macos")]
    if let Some(info) = detect_metal() {
        return info;
    }

    #[cfg(target_os = "linux")]
    {
        // Try NVML first (NVIDIA), then sysfs (AMD/Intel)
        if let Some(info) = detect_nvml() {
            return info;
        }
        if let Some(info) = detect_sysfs() {
            return info;
        }
    }

    // Fallback to wgpu detection
    #[cfg(feature = "wgpu")]
    if let Some(info) = detect_wgpu() {
        return info;
    }

    VramInfo::default()
}

// =============================================================================
// Windows DXGI Detection
// =============================================================================

#[cfg(target_os = "windows")]
fn detect_dxgi() -> Option<VramInfo> {
    use windows::Win32::Graphics::Dxgi::*;
    use windows::core::Interface;

    unsafe {
        let factory: IDXGIFactory4 = CreateDXGIFactory2(DXGI_CREATE_FACTORY_FLAGS(0)).ok()?;
        
        let mut adapter_idx = 0u32;
        while let Ok(adapter) = factory.EnumAdapters1(adapter_idx) {
            adapter_idx += 1;
            
            let desc = adapter.GetDesc1().ok()?;
            
            // Skip software adapters
            if desc.Flags & DXGI_ADAPTER_FLAG_SOFTWARE.0 as u32 != 0 {
                continue;
            }
            
            // Skip Microsoft Basic Render Driver
            let name = String::from_utf16_lossy(&desc.Description)
                .trim_end_matches('\0')
                .to_string();
            if name.contains("Basic Render") || name.contains("Microsoft") {
                continue;
            }
            
            let total = desc.DedicatedVideoMemory as u64;
            if total == 0 {
                continue;
            }
            
            // Try to get current memory usage via QueryVideoMemoryInfo
            let free = if let Ok(adapter3) = adapter.cast::<IDXGIAdapter3>() {
                let mut mem_info = DXGI_QUERY_VIDEO_MEMORY_INFO::default();
                if adapter3.QueryVideoMemoryInfo(
                    0,
                    DXGI_MEMORY_SEGMENT_GROUP_LOCAL,
                    &mut mem_info,
                ).is_ok() {
                    Some(mem_info.Budget.saturating_sub(mem_info.CurrentUsage))
                } else {
                    None
                }
            } else {
                None
            };
            
            return Some(VramInfo {
                total,
                free,
                name: Some(name),
                method: "dxgi",
            });
        }
    }
    
    None
}

#[cfg(not(target_os = "windows"))]
fn detect_dxgi() -> Option<VramInfo> {
    None
}

// =============================================================================
// macOS Metal Detection
// =============================================================================

#[cfg(target_os = "macos")]
fn detect_metal() -> Option<VramInfo> {
    use objc2_metal::{MTLCreateSystemDefaultDevice, MTLDevice};
    
    // Link CoreGraphics (required by MTLCreateSystemDefaultDevice)
    #[link(name = "CoreGraphics", kind = "framework")]
    unsafe extern "C" {}
    
    let device = MTLCreateSystemDefaultDevice()?;
    
    // recommendedMaxWorkingSetSize is the best estimate on Metal
    let total = device.recommendedMaxWorkingSetSize() as u64;
    
    // currentAllocatedSize gives current usage
    let used = device.currentAllocatedSize() as u64;
    let free = total.saturating_sub(used);
    
    let name = device.name().to_string();
    
    Some(VramInfo {
        total,
        free: Some(free),
        name: Some(name),
        method: "metal",
    })
}

#[cfg(not(target_os = "macos"))]
#[allow(dead_code)]  // Needed for cross-platform compilation
fn detect_metal() -> Option<VramInfo> {
    None
}

// =============================================================================
// Linux NVML Detection (NVIDIA)
// =============================================================================

#[cfg(target_os = "linux")]
fn detect_nvml() -> Option<VramInfo> {
    // Try loading NVML dynamically
    let nvml = nvml_wrapper::Nvml::init().ok()?;
    let device = nvml.device_by_index(0).ok()?;
    
    let mem_info = device.memory_info().ok()?;
    let name = device.name().ok();
    
    Some(VramInfo {
        total: mem_info.total,
        free: Some(mem_info.free),
        name,
        method: "nvml",
    })
}

#[cfg(not(target_os = "linux"))]
#[allow(dead_code)]  // Needed for cross-platform compilation
fn detect_nvml() -> Option<VramInfo> {
    None
}

// =============================================================================
// Linux sysfs Detection (AMD/Intel)
// =============================================================================

#[cfg(target_os = "linux")]
fn detect_sysfs() -> Option<VramInfo> {
    use std::fs;
    use std::path::Path;
    
    // Look for DRM cards
    let drm_dir = Path::new("/sys/class/drm");
    if !drm_dir.exists() {
        return None;
    }
    
    for entry in fs::read_dir(drm_dir).ok()? {
        let entry = entry.ok()?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        
        // Look for card0, card1, etc.
        if !name_str.starts_with("card") || name_str.contains('-') {
            continue;
        }
        
        let device_dir = entry.path().join("device");
        
        // AMD: mem_info_vram_total / mem_info_vram_used
        let total_path = device_dir.join("mem_info_vram_total");
        let used_path = device_dir.join("mem_info_vram_used");
        
        if total_path.exists() {
            if let Ok(total_str) = fs::read_to_string(&total_path) {
                if let Ok(total) = total_str.trim().parse::<u64>() {
                    let free = fs::read_to_string(&used_path)
                        .ok()
                        .and_then(|s| s.trim().parse::<u64>().ok())
                        .map(|used| total.saturating_sub(used));
                    
                    // Try to get device name
                    let gpu_name = fs::read_to_string(device_dir.join("product_name"))
                        .or_else(|_| fs::read_to_string(device_dir.join("device")))
                        .ok()
                        .map(|s| s.trim().to_string());
                    
                    return Some(VramInfo {
                        total,
                        free,
                        name: gpu_name,
                        method: "sysfs",
                    });
                }
            }
        }
        
        // Intel: Try GTT size as fallback
        let gtt_path = device_dir.join("drm").join(&name_str).join("gtt_total");
        if gtt_path.exists() {
            if let Ok(total_str) = fs::read_to_string(&gtt_path) {
                if let Ok(total) = total_str.trim().parse::<u64>() {
                    return Some(VramInfo {
                        total,
                        free: None,
                        name: Some("Intel GPU".to_string()),
                        method: "sysfs-gtt",
                    });
                }
            }
        }
    }
    
    None
}

#[cfg(not(target_os = "linux"))]
#[allow(dead_code)]  // Needed for cross-platform compilation
fn detect_sysfs() -> Option<VramInfo> {
    None
}

// =============================================================================
// wgpu Fallback Detection
// =============================================================================

#[cfg(feature = "wgpu")]
fn detect_wgpu() -> Option<VramInfo> {
    use wgpu::{Backends, Instance, InstanceDescriptor, RequestAdapterOptions, PowerPreference};
    
    let instance = Instance::new(&InstanceDescriptor {
        backends: Backends::all(),
        ..Default::default()
    });
    
    let adapter = pollster::block_on(instance.request_adapter(&RequestAdapterOptions {
        power_preference: PowerPreference::HighPerformance,
        compatible_surface: None,
        force_fallback_adapter: false,
    }))?;
    
    let info = adapter.get_info();
    
    // Check for software renderer
    if is_software_renderer(&info) {
        return None;
    }
    
    let limits = adapter.limits();
    
    // wgpu doesn't expose total VRAM directly, estimate from buffer limits
    // This is very rough - max_buffer_size is often 256MB even on 8GB cards
    let total = (limits.max_buffer_size as u64).max(2 * 1024 * 1024 * 1024);
    
    Some(VramInfo {
        total,
        free: None,
        name: Some(info.name.clone()),
        method: "wgpu",
    })
}

#[cfg(not(feature = "wgpu"))]
#[allow(dead_code)]  // Needed when wgpu feature disabled
fn detect_wgpu() -> Option<VramInfo> {
    None
}

/// Check if wgpu adapter is a software renderer.
#[cfg(feature = "wgpu")]
pub fn is_software_renderer(info: &wgpu::AdapterInfo) -> bool {
    let name = info.name.to_lowercase();
    
    // Known software renderers
    name.contains("llvmpipe") ||
    name.contains("softpipe") ||
    name.contains("swiftshader") ||
    name.contains("lavapipe") ||
    name.contains("software") ||
    name.contains("microsoft basic render") ||
    info.device_type == wgpu::DeviceType::Cpu
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_vram() {
        let info = detect_vram();
        println!("VRAM: {} ({}) via {}", 
            super::super::memory::format_bytes(info.total),
            info.name.as_deref().unwrap_or("unknown"),
            info.method
        );
        
        // Should have detected something
        assert!(info.total > 0);
    }

    #[test]
    fn test_available_vram() {
        let avail = available_vram();
        let total = total_vram();
        
        // Available should be <= total
        assert!(avail <= total);
        // Should be at least 100 MB
        assert!(avail >= 100 * 1024 * 1024);
    }
}
