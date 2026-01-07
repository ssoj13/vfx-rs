//! Memory management utilities.
//!
//! Provides system memory detection, budgeting, and environment variable overrides.
//!
//! # Environment Variables
//!
//! - `VFX_RAM_MAX` - Maximum RAM usage in bytes
//! - `VFX_RAM_PCT` - Maximum RAM as percentage (1-100)
//! - `VFX_MEM_MB` - Explicit memory limit in megabytes
//! - `VFX_TILE_SIZE` - Override tile size
//! - `VFX_DISABLE_CACHE` - Disable region caching ("1" or "true")

use std::env;
use std::sync::OnceLock;

/// Bytes per pixel for RGBA f32.
pub const BYTES_PER_PIXEL: u64 = 16;

/// Default safety margin - use at most 80% of available memory.
pub const SAFE_MEMORY_FRACTION: f64 = 0.80;

/// Memory overhead factor for processing (src + dst + intermediate).
pub const PROCESSING_OVERHEAD: f64 = 3.0;

/// Cache for system memory detection.
static SYSTEM_MEMORY: OnceLock<u64> = OnceLock::new();

/// Detect total system RAM in bytes.
pub fn system_memory() -> u64 {
    *SYSTEM_MEMORY.get_or_init(|| {
        sys_info::mem_info()
            .map(|m| m.total * 1024) // KB to bytes
            .unwrap_or(8 * 1024 * 1024 * 1024) // 8 GB fallback
    })
}

/// Get available RAM considering environment overrides.
///
/// Priority:
/// 1. `VFX_MEM_MB` - explicit MB limit
/// 2. `VFX_RAM_MAX` - explicit bytes limit
/// 3. `VFX_RAM_PCT` - percentage of system RAM
/// 4. Default: 80% of system RAM
pub fn available_memory() -> u64 {
    // Check explicit MB override
    if let Some(mb) = env_mem_mb() {
        return mb * 1024 * 1024;
    }
    
    // Check explicit bytes override
    if let Some(bytes) = env_ram_max() {
        return bytes;
    }
    
    // Check percentage override
    let pct = env_ram_pct().unwrap_or((SAFE_MEMORY_FRACTION * 100.0) as u64);
    let pct = pct.clamp(10, 95); // Sanity bounds
    
    system_memory() * pct / 100
}

/// Memory budget for processing (70% of available).
pub fn processing_budget() -> u64 {
    (available_memory() as f64 * 0.70) as u64
}

/// Memory budget for caching (25% of available).
pub fn cache_budget() -> u64 {
    (available_memory() as f64 * 0.25) as u64
}

/// Check if caching is disabled via environment.
pub fn cache_disabled() -> bool {
    env::var("VFX_DISABLE_CACHE")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

/// Get tile size override from environment.
pub fn tile_size_override() -> Option<u32> {
    env::var("VFX_TILE_SIZE")
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|&s| s >= 64 && s <= 16384)
}

/// Get backend override from environment.
pub fn backend_override() -> Option<String> {
    env::var("VFX_BACKEND").ok()
}

// =============================================================================
// Environment Variable Helpers
// =============================================================================

fn env_mem_mb() -> Option<u64> {
    env::var("VFX_MEM_MB")
        .ok()
        .and_then(|v| v.parse().ok())
}

fn env_ram_max() -> Option<u64> {
    env::var("VFX_RAM_MAX")
        .ok()
        .and_then(|v| v.parse().ok())
}

fn env_ram_pct() -> Option<u64> {
    env::var("VFX_RAM_PCT")
        .ok()
        .and_then(|v| v.parse().ok())
}

// =============================================================================
// Memory Estimation
// =============================================================================

/// Estimate memory for an image (bytes).
#[inline]
pub fn image_memory(width: u32, height: u32, channels: u32) -> u64 {
    (width as u64) * (height as u64) * (channels as u64) * 4
}

/// Estimate processing memory with overhead.
#[inline]
pub fn processing_memory(width: u32, height: u32, channels: u32) -> u64 {
    (image_memory(width, height, channels) as f64 * PROCESSING_OVERHEAD) as u64
}

/// Format bytes as human-readable string.
pub fn format_bytes(bytes: u64) -> String {
    if bytes >= 1024 * 1024 * 1024 {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    } else if bytes >= 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{} KB", bytes / 1024)
    } else {
        format!("{} B", bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_memory() {
        let mem = system_memory();
        assert!(mem > 0);
        // Should be at least 512 MB
        assert!(mem >= 512 * 1024 * 1024);
    }

    #[test]
    fn test_available_memory() {
        let avail = available_memory();
        let total = system_memory();
        // Should be less than or equal to total
        assert!(avail <= total);
        // Should be at least 10% of total
        assert!(avail >= total / 10);
    }

    #[test]
    fn test_image_memory() {
        // 1024x1024 RGBA = 16 MB
        let mem = image_memory(1024, 1024, 4);
        assert_eq!(mem, 1024 * 1024 * 4 * 4);
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(500), "500 B");
        assert_eq!(format_bytes(1500), "1 KB");
        assert_eq!(format_bytes(1_500_000), "1.4 MB");
        assert_eq!(format_bytes(1_500_000_000), "1.40 GB");
    }
}
