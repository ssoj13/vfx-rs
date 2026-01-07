//! LRU region cache for GPU source data.
//!
//! Caches uploaded image regions to avoid redundant GPU transfers.
//! Useful for:
//! - Viewer pan/zoom (cache visible regions)
//! - Animation playback (frame-to-frame coherence)
//! - Multi-pass processing (reuse source uploads)

use std::collections::{HashMap, VecDeque};
use std::hash::Hash;
use std::time::Instant;

use super::memory::cache_budget;

/// Key identifying a cached region.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RegionKey {
    /// X offset in source image.
    pub x: u32,
    /// Y offset in source image.
    pub y: u32,
    /// Region width.
    pub w: u32,
    /// Region height.
    pub h: u32,
}

impl RegionKey {
    pub fn new(x: u32, y: u32, w: u32, h: u32) -> Self {
        Self { x, y, w, h }
    }
    
    /// Full image key.
    pub fn full(w: u32, h: u32) -> Self {
        Self::new(0, 0, w, h)
    }
    
    /// Check if this region contains another.
    pub fn contains(&self, other: &RegionKey) -> bool {
        other.x >= self.x &&
        other.y >= self.y &&
        other.x + other.w <= self.x + self.w &&
        other.y + other.h <= self.y + self.h
    }
    
    /// Check if regions overlap.
    pub fn overlaps(&self, other: &RegionKey) -> bool {
        !(self.x + self.w <= other.x ||
          other.x + other.w <= self.x ||
          self.y + self.h <= other.y ||
          other.y + other.h <= self.y)
    }
    
    /// Calculate overlap ratio (0.0 - 1.0).
    pub fn overlap_ratio(&self, other: &RegionKey) -> f32 {
        if !self.overlaps(other) {
            return 0.0;
        }
        
        let ix = self.x.max(other.x);
        let iy = self.y.max(other.y);
        let ix2 = (self.x + self.w).min(other.x + other.w);
        let iy2 = (self.y + self.h).min(other.y + other.h);
        
        let intersection = ((ix2 - ix) as f64) * ((iy2 - iy) as f64);
        let union = (self.w as f64 * self.h as f64) + 
                   (other.w as f64 * other.h as f64) - intersection;
        
        (intersection / union) as f32
    }
}

/// Cached region entry.
pub struct CachedRegion<T> {
    /// The cached handle.
    pub handle: T,
    /// Region key.
    pub key: RegionKey,
    /// Size in bytes.
    pub size_bytes: u64,
    /// Last access time.
    pub last_access: Instant,
}

/// LRU cache for GPU regions.
///
/// Generic over handle type `T` (e.g., CpuImage, WgpuImage).
pub struct RegionCache<T> {
    /// Cached regions by key.
    regions: HashMap<RegionKey, CachedRegion<T>>,
    /// Access order for LRU eviction (front = oldest).
    access_order: VecDeque<RegionKey>,
    /// Total cached bytes.
    total_bytes: u64,
    /// Maximum cache size in bytes.
    max_bytes: u64,
    /// Cache hits counter.
    hits: u64,
    /// Cache misses counter.
    misses: u64,
}

impl<T> RegionCache<T> {
    /// Create new cache with default budget.
    pub fn new() -> Self {
        Self::with_budget(cache_budget())
    }
    
    /// Create cache with specific budget.
    pub fn with_budget(max_bytes: u64) -> Self {
        Self {
            regions: HashMap::new(),
            access_order: VecDeque::new(),
            total_bytes: 0,
            max_bytes,
            hits: 0,
            misses: 0,
        }
    }
    
    /// Get cached region if available.
    pub fn get(&mut self, key: &RegionKey) -> Option<&T> {
        if self.regions.contains_key(key) {
            self.hits += 1;
            self.touch(key);
            self.regions.get(key).map(|r| &r.handle)
        } else {
            self.misses += 1;
            None
        }
    }
    
    /// Get mutable reference to cached region.
    pub fn get_mut(&mut self, key: &RegionKey) -> Option<&mut T> {
        if self.regions.contains_key(key) {
            self.hits += 1;
            self.touch(key);
            self.regions.get_mut(key).map(|r| &mut r.handle)
        } else {
            self.misses += 1;
            None
        }
    }
    
    /// Insert region into cache.
    ///
    /// Evicts LRU entries if needed to fit within budget.
    pub fn insert(&mut self, key: RegionKey, handle: T, size_bytes: u64) {
        // Evict until we have space
        while self.total_bytes + size_bytes > self.max_bytes && !self.regions.is_empty() {
            self.evict_lru();
        }
        
        // Remove existing entry if present
        if let Some(old) = self.regions.remove(&key) {
            self.total_bytes = self.total_bytes.saturating_sub(old.size_bytes);
            self.access_order.retain(|k| k != &key);
        }
        
        self.regions.insert(key, CachedRegion {
            handle,
            key,
            size_bytes,
            last_access: Instant::now(),
        });
        self.access_order.push_back(key);
        self.total_bytes += size_bytes;
    }
    
    /// Remove specific region from cache.
    pub fn remove(&mut self, key: &RegionKey) -> Option<T> {
        if let Some(region) = self.regions.remove(key) {
            self.total_bytes = self.total_bytes.saturating_sub(region.size_bytes);
            self.access_order.retain(|k| k != key);
            Some(region.handle)
        } else {
            None
        }
    }
    
    /// Clear entire cache.
    pub fn clear(&mut self) {
        self.regions.clear();
        self.access_order.clear();
        self.total_bytes = 0;
    }
    
    /// Evict least recently used entry.
    pub fn evict_lru(&mut self) -> Option<CachedRegion<T>> {
        if let Some(key) = self.access_order.pop_front() {
            if let Some(region) = self.regions.remove(&key) {
                self.total_bytes = self.total_bytes.saturating_sub(region.size_bytes);
                return Some(region);
            }
        }
        None
    }
    
    /// Find a cached region that contains the requested region.
    ///
    /// Note: Does not update LRU order to avoid borrow conflicts.
    /// Use `get()` after finding if LRU tracking is needed.
    pub fn find_containing(&mut self, key: &RegionKey) -> Option<RegionKey> {
        for cached_key in self.regions.keys() {
            if cached_key.contains(key) {
                self.hits += 1;
                return Some(*cached_key);
            }
        }
        self.misses += 1;
        None
    }
    
    /// Find regions overlapping with the given region.
    pub fn find_overlapping(&self, key: &RegionKey) -> Vec<&RegionKey> {
        self.regions.keys()
            .filter(|k| k.overlaps(key))
            .collect()
    }
    
    // =========================================================================
    // Stats
    // =========================================================================
    
    /// Current cache size in bytes.
    pub fn size_bytes(&self) -> u64 {
        self.total_bytes
    }
    
    /// Maximum cache size in bytes.
    pub fn max_bytes(&self) -> u64 {
        self.max_bytes
    }
    
    /// Number of cached entries.
    pub fn len(&self) -> usize {
        self.regions.len()
    }
    
    /// Check if cache is empty.
    pub fn is_empty(&self) -> bool {
        self.regions.is_empty()
    }
    
    /// Cache hit count.
    pub fn hits(&self) -> u64 {
        self.hits
    }
    
    /// Cache miss count.
    pub fn misses(&self) -> u64 {
        self.misses
    }
    
    /// Cache hit ratio (0.0 - 1.0).
    pub fn hit_ratio(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }
    
    /// Reset statistics.
    pub fn reset_stats(&mut self) {
        self.hits = 0;
        self.misses = 0;
    }
    
    // =========================================================================
    // Internal
    // =========================================================================
    
    /// Update access order (move to back).
    fn touch(&mut self, key: &RegionKey) {
        self.access_order.retain(|k| k != key);
        self.access_order.push_back(*key);
        
        if let Some(region) = self.regions.get_mut(key) {
            region.last_access = Instant::now();
        }
    }
}

impl<T> Default for RegionCache<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_region_key_contains() {
        let outer = RegionKey::new(0, 0, 100, 100);
        let inner = RegionKey::new(10, 10, 50, 50);
        let outside = RegionKey::new(90, 90, 50, 50);
        
        assert!(outer.contains(&inner));
        assert!(!inner.contains(&outer));
        assert!(!outer.contains(&outside));
    }

    #[test]
    fn test_region_key_overlaps() {
        let a = RegionKey::new(0, 0, 100, 100);
        let b = RegionKey::new(50, 50, 100, 100);
        let c = RegionKey::new(200, 200, 100, 100);
        
        assert!(a.overlaps(&b));
        assert!(b.overlaps(&a));
        assert!(!a.overlaps(&c));
    }

    #[test]
    fn test_cache_insert_get() {
        let mut cache: RegionCache<String> = RegionCache::with_budget(1000);
        
        let key = RegionKey::new(0, 0, 10, 10);
        cache.insert(key, "test".to_string(), 100);
        
        assert_eq!(cache.len(), 1);
        assert_eq!(cache.get(&key), Some(&"test".to_string()));
    }

    #[test]
    fn test_cache_eviction() {
        let mut cache: RegionCache<i32> = RegionCache::with_budget(200);
        
        let k1 = RegionKey::new(0, 0, 10, 10);
        let k2 = RegionKey::new(10, 0, 10, 10);
        let k3 = RegionKey::new(20, 0, 10, 10);
        
        cache.insert(k1, 1, 100);
        cache.insert(k2, 2, 100);
        
        assert_eq!(cache.len(), 2);
        
        // This should evict k1
        cache.insert(k3, 3, 100);
        
        assert_eq!(cache.len(), 2);
        assert!(cache.get(&k1).is_none());
        assert!(cache.get(&k2).is_some());
        assert!(cache.get(&k3).is_some());
    }

    #[test]
    fn test_cache_lru_order() {
        let mut cache: RegionCache<i32> = RegionCache::with_budget(300);
        
        let k1 = RegionKey::new(0, 0, 10, 10);
        let k2 = RegionKey::new(10, 0, 10, 10);
        let k3 = RegionKey::new(20, 0, 10, 10);
        
        cache.insert(k1, 1, 100);
        cache.insert(k2, 2, 100);
        cache.insert(k3, 3, 100);
        
        // Access k1 to make it recent
        let _ = cache.get(&k1);
        
        // Insert k4, should evict k2 (oldest now)
        let k4 = RegionKey::new(30, 0, 10, 10);
        cache.insert(k4, 4, 100);
        
        assert!(cache.get(&k1).is_some()); // Was accessed
        assert!(cache.get(&k2).is_none()); // Evicted
        assert!(cache.get(&k3).is_some());
        assert!(cache.get(&k4).is_some());
    }

    #[test]
    fn test_cache_stats() {
        let mut cache: RegionCache<i32> = RegionCache::with_budget(1000);
        
        let k1 = RegionKey::new(0, 0, 10, 10);
        cache.insert(k1, 1, 100);
        
        let _ = cache.get(&k1); // hit
        let _ = cache.get(&k1); // hit
        let _ = cache.get(&RegionKey::new(100, 100, 10, 10)); // miss
        
        assert_eq!(cache.hits(), 2);
        assert_eq!(cache.misses(), 1);
        assert!((cache.hit_ratio() - 0.666).abs() < 0.01);
    }
}
