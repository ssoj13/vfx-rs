//! Image cache with LRU eviction.
//!
//! Provides efficient caching of image tiles for texture systems.
//! Thread-safe with configurable memory limits.
//!
//! # Example
//!
//! ```ignore
//! use vfx_io::cache::ImageCache;
//!
//! let cache = ImageCache::new(512 * 1024 * 1024); // 512MB limit
//! let tile = cache.get_tile("texture.exr", 0, 0, 0)?;
//! ```

use std::collections::HashMap;

use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock, Mutex};
use std::time::Instant;

use crate::{IoResult, IoError};
use crate::streaming::{self, BoxedSource};

/// Default tile size in pixels.
pub const DEFAULT_TILE_SIZE: u32 = 64;

/// Default cache size in bytes (256MB).
pub const DEFAULT_CACHE_SIZE: usize = 256 * 1024 * 1024;

/// Default streaming threshold in bytes (512MB).
/// Images larger than this will use streaming instead of full load.
pub const DEFAULT_STREAMING_THRESHOLD: u64 = 512 * 1024 * 1024;

/// Key for cached tiles.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TileKey {
    /// File path.
    pub path: PathBuf,
    /// Subimage index (for multi-part files).
    pub subimage: u32,
    /// Mipmap level (0 = full resolution).
    pub mip_level: u32,
    /// Tile X coordinate (in tiles, not pixels).
    pub tile_x: u32,
    /// Tile Y coordinate (in tiles, not pixels).
    pub tile_y: u32,
}

impl TileKey {
    /// Creates a new tile key.
    pub fn new(path: impl Into<PathBuf>, subimage: u32, mip_level: u32, tile_x: u32, tile_y: u32) -> Self {
        Self {
            path: path.into(),
            subimage,
            mip_level,
            tile_x,
            tile_y,
        }
    }
}

/// Cached tile data.
#[derive(Debug, Clone)]
pub struct Tile {
    /// Tile width in pixels.
    pub width: u32,
    /// Tile height in pixels.
    pub height: u32,
    /// Number of channels.
    pub channels: u32,
    /// Pixel data (always f32).
    pub data: Vec<f32>,
    /// Last access time for LRU.
    last_access: Instant,
}

impl Tile {
    /// Creates a new tile.
    pub fn new(width: u32, height: u32, channels: u32, data: Vec<f32>) -> Self {
        Self {
            width,
            height,
            channels,
            data,
            last_access: Instant::now(),
        }
    }

    /// Memory size in bytes.
    #[inline]
    pub fn size_bytes(&self) -> usize {
        self.data.len() * std::mem::size_of::<f32>()
    }

    /// Updates last access time.
    fn touch(&mut self) {
        self.last_access = Instant::now();
    }
}

/// Image metadata for caching.
#[derive(Debug, Clone)]
pub struct CachedImageInfo {
    /// Full image width.
    pub width: u32,
    /// Full image height.
    pub height: u32,
    /// Number of channels.
    pub channels: u32,
    /// Tile width.
    pub tile_width: u32,
    /// Tile height.
    pub tile_height: u32,
    /// Number of mip levels.
    pub mip_levels: u32,
    /// Number of subimages.
    pub subimages: u32,
}

/// Cached full image data for efficient tile extraction.
#[derive(Clone)]
struct CachedImageData {
    /// Full resolution f32 pixel data.
    data: Vec<f32>,
    /// Image width.
    width: u32,
    /// Image height.
    height: u32,
    /// Number of channels.
    channels: u32,
    /// Pre-generated mip levels (level -> data).
    mips: HashMap<u32, Vec<f32>>,
}

/// Image storage mode - either full data or streaming source.
enum ImageStorage {
    /// Full image data in memory (for small images).
    Full(CachedImageData),
    /// Streaming source for large images (lazy tile loading).
    Streaming {
        source: BoxedSource,
        width: u32,
        height: u32,
        channels: u32,
    },
}

impl CachedImageInfo {
    /// Number of tiles in X direction.
    #[inline]
    pub fn tiles_x(&self) -> u32 {
        (self.width + self.tile_width - 1) / self.tile_width
    }

    /// Number of tiles in Y direction.
    #[inline]
    pub fn tiles_y(&self) -> u32 {
        (self.height + self.tile_height - 1) / self.tile_height
    }

    /// Width at given mip level.
    pub fn width_at_mip(&self, level: u32) -> u32 {
        (self.width >> level).max(1)
    }

    /// Height at given mip level.
    pub fn height_at_mip(&self, level: u32) -> u32 {
        (self.height >> level).max(1)
    }
}

/// LRU node for cache management.
struct LruNode {
    prev: Option<TileKey>,
    next: Option<TileKey>,
}

/// Thread-safe image cache with LRU eviction.
///
/// Caches image tiles in memory with configurable size limit.
/// Automatically evicts least recently used tiles when full.
///
/// # Tiled EXR optimization
///
/// For efficient tile access, the cache loads full images once and extracts
/// tiles on demand. For native tiled EXR block reading (avoiding full image load),
/// use exr crate's `block::FilteredChunksReader` with `BlockIndex::pixel_position`.
pub struct ImageCache {
    /// Maximum cache size in bytes.
    max_size: usize,
    /// Current cache size in bytes.
    current_size: RwLock<usize>,
    /// Tile storage.
    tiles: RwLock<HashMap<TileKey, Tile>>,
    /// Image info cache.
    image_info: RwLock<HashMap<PathBuf, CachedImageInfo>>,
    /// Image data storage (full or streaming), keyed by (path, subimage).
    image_storage: RwLock<HashMap<(PathBuf, u32), ImageStorage>>,
    /// LRU list head (most recent).
    lru_head: Mutex<Option<TileKey>>,
    /// LRU list tail (least recent).
    lru_tail: Mutex<Option<TileKey>>,
    /// LRU node map.
    lru_nodes: RwLock<HashMap<TileKey, LruNode>>,
    /// Tile size for new loads.
    tile_size: u32,
    /// Streaming threshold in bytes (images larger use streaming).
    streaming_threshold: u64,
    /// Statistics.
    stats: RwLock<CacheStats>,
}

/// Cache statistics.
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    /// Number of cache hits.
    pub hits: u64,
    /// Number of cache misses.
    pub misses: u64,
    /// Number of tiles evicted.
    pub evictions: u64,
    /// Total tiles currently cached.
    pub tile_count: u64,
    /// Peak memory usage in bytes.
    pub peak_size: usize,
}

impl CacheStats {
    /// Hit rate as percentage.
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            (self.hits as f64 / total as f64) * 100.0
        }
    }
}

impl ImageCache {
    /// Creates a new cache with the given size limit.
    pub fn new(max_size: usize) -> Self {
        Self {
            max_size,
            current_size: RwLock::new(0),
            tiles: RwLock::new(HashMap::new()),
            image_info: RwLock::new(HashMap::new()),
            image_storage: RwLock::new(HashMap::new()),
            lru_head: Mutex::new(None),
            lru_tail: Mutex::new(None),
            lru_nodes: RwLock::new(HashMap::new()),
            tile_size: DEFAULT_TILE_SIZE,
            streaming_threshold: DEFAULT_STREAMING_THRESHOLD,
            stats: RwLock::new(CacheStats::default()),
        }
    }

    /// Creates a cache with custom streaming threshold.
    ///
    /// Images larger than `streaming_threshold` will use streaming I/O
    /// instead of loading the full image into memory.
    pub fn with_streaming_threshold(max_size: usize, streaming_threshold: u64) -> Self {
        let mut cache = Self::new(max_size);
        cache.streaming_threshold = streaming_threshold;
        cache
    }

    /// Creates a cache with default settings.
    pub fn default_cache() -> Self {
        Self::new(DEFAULT_CACHE_SIZE)
    }

    /// Sets the tile size for new loads.
    pub fn set_tile_size(&mut self, size: u32) {
        self.tile_size = size;
    }

    /// Returns the current tile size.
    pub fn tile_size(&self) -> u32 {
        self.tile_size
    }

    /// Returns current memory usage in bytes.
    pub fn size(&self) -> usize {
        *self.current_size.read().unwrap()
    }

    /// Returns maximum cache size.
    pub fn max_size(&self) -> usize {
        self.max_size
    }

    /// Returns cache statistics.
    pub fn stats(&self) -> CacheStats {
        self.stats.read().unwrap().clone()
    }

    /// Clears all cached data.
    pub fn clear(&self) {
        let mut tiles = self.tiles.write().unwrap();
        let mut lru_nodes = self.lru_nodes.write().unwrap();
        let mut current_size = self.current_size.write().unwrap();
        let mut stats = self.stats.write().unwrap();
        let mut image_storage = self.image_storage.write().unwrap();

        tiles.clear();
        lru_nodes.clear();
        image_storage.clear();
        *current_size = 0;
        *self.lru_head.lock().unwrap() = None;
        *self.lru_tail.lock().unwrap() = None;
        stats.tile_count = 0;
    }

    /// Gets image info, loading from file if needed.
    ///
    /// Uses header-only reading when possible to avoid loading full image.
    pub fn get_image_info(&self, path: impl AsRef<Path>) -> IoResult<CachedImageInfo> {
        let path = path.as_ref();

        // Check cache first
        {
            let info_cache = self.image_info.read().unwrap();
            if let Some(info) = info_cache.get(path) {
                return Ok(info.clone());
            }
        }

        // Query actual subimages count from registry
        let num_subimages = crate::registry::FormatRegistry::global()
            .num_subimages(path)
            .unwrap_or(1) as u32;

        // Try to estimate from header (fast path - no pixel loading)
        let info = if let Ok(estimate) = streaming::estimate_memory(path) {
            CachedImageInfo {
                width: estimate.width,
                height: estimate.height,
                channels: estimate.channels,
                tile_width: self.tile_size,
                tile_height: self.tile_size,
                mip_levels: compute_mip_levels(estimate.width, estimate.height),
                subimages: num_subimages,
            }
        } else {
            // Fallback: load image to get dimensions
            let image = crate::read(path)?;
            CachedImageInfo {
                width: image.width,
                height: image.height,
                channels: image.channels,
                tile_width: self.tile_size,
                tile_height: self.tile_size,
                mip_levels: compute_mip_levels(image.width, image.height),
                subimages: num_subimages,
            }
        };

        // Cache info
        {
            let mut info_cache = self.image_info.write().unwrap();
            info_cache.insert(path.to_path_buf(), info.clone());
        }

        Ok(info)
    }

    /// Gets a tile, loading from file if needed.
    pub fn get_tile(&self, path: impl AsRef<Path>, subimage: u32, mip_level: u32, tile_x: u32, tile_y: u32) -> IoResult<Arc<Tile>> {
        let path = path.as_ref();
        let key = TileKey::new(path, subimage, mip_level, tile_x, tile_y);

        // Check cache first
        {
            let mut tiles = self.tiles.write().unwrap();
            if let Some(tile) = tiles.get_mut(&key) {
                tile.touch();
                self.update_lru(&key);
                self.stats.write().unwrap().hits += 1;
                return Ok(Arc::new(tile.clone()));
            }
        }

        // Cache miss - load tile
        self.stats.write().unwrap().misses += 1;
        let tile = self.load_tile(path, subimage, mip_level, tile_x, tile_y)?;

        // Ensure space
        self.ensure_space(tile.size_bytes());

        // Insert into cache
        {
            let mut tiles = self.tiles.write().unwrap();
            let mut current_size = self.current_size.write().unwrap();
            let mut stats = self.stats.write().unwrap();

            *current_size += tile.size_bytes();
            if *current_size > stats.peak_size {
                stats.peak_size = *current_size;
            }
            stats.tile_count += 1;

            tiles.insert(key.clone(), tile.clone());
        }

        self.add_to_lru(&key);

        Ok(Arc::new(tile))
    }

    /// Loads a tile from disk using cached image data or streaming.
    ///
    /// For small images: loads full image and extracts tiles from memory.
    /// For large images: uses streaming source to read only needed regions.
    fn load_tile(&self, path: &Path, subimage: u32, mip_level: u32, tile_x: u32, tile_y: u32) -> IoResult<Tile> {
        let path_buf = path.to_path_buf();
        let storage_key = (path_buf.clone(), subimage);
        
        // Check if we have storage for this image+subimage
        let has_storage = {
            let storage = self.image_storage.read().unwrap();
            storage.contains_key(&storage_key)
        };
        
        if !has_storage {
            // Decide: streaming or full load based on size estimate
            let use_streaming = streaming::estimate_memory(path)
                .map(|est| est.f32_bytes > self.streaming_threshold)
                .unwrap_or(false);
            
            if use_streaming {
                // Open streaming source (note: streaming doesn't support subimages yet)
                let source = streaming::open_streaming(path)?;
                let (width, height) = source.dimensions();
                // Region data is always RGBA (4 channels), regardless of source format.
                // We use RGBA_CHANNELS for tile operations to match Region layout.
                let channels = streaming::RGBA_CHANNELS;
                
                let mut storage = self.image_storage.write().unwrap();
                storage.insert(storage_key.clone(), ImageStorage::Streaming {
                    source,
                    width,
                    height,
                    channels,
                });
            } else {
                // Load full image with subimage support
                let image = crate::read_subimage(path, subimage as usize, 0)?;
                let data = image.to_f32();
                let cached = CachedImageData {
                    data,
                    width: image.width,
                    height: image.height,
                    channels: image.channels,
                    mips: HashMap::new(),
                };
                
                let mut storage = self.image_storage.write().unwrap();
                storage.insert(storage_key.clone(), ImageStorage::Full(cached));
            }
        }
        
        // Now load the tile from storage
        let mut storage = self.image_storage.write().unwrap();
        let entry = storage.get_mut(&storage_key)
            .ok_or_else(|| IoError::DecodeError("Storage not found".into()))?;
        
        match entry {
            ImageStorage::Streaming { source, width, height, channels } => {
                // For mip_level > 0 in streaming mode, we need to load full image
                // and generate mips (streaming can't efficiently do this)
                if mip_level > 0 {
                    // Read full image at mip=0 and convert to Full storage
                    let full_width = *width;
                    let full_height = *height;
                    let num_channels = *channels;
                    
                    // Read all tiles at mip=0 to reconstruct full image
                    let mut full_data = vec![0.0f32; (full_width * full_height) as usize * num_channels as usize];
                    let tiles_x = (full_width + self.tile_size - 1) / self.tile_size;
                    let tiles_y = (full_height + self.tile_size - 1) / self.tile_size;
                    
                    for ty in 0..tiles_y {
                        for tx in 0..tiles_x {
                            let tile_px_x = tx * self.tile_size;
                            let tile_px_y = ty * self.tile_size;
                            let tile_w = self.tile_size.min(full_width.saturating_sub(tile_px_x));
                            let tile_h = self.tile_size.min(full_height.saturating_sub(tile_px_y));
                            
                            if tile_w == 0 || tile_h == 0 {
                                continue;
                            }
                            
                            let region = source.read_region(tile_px_x, tile_px_y, tile_w, tile_h)?;
                            
                            for y in 0..tile_h {
                                for x in 0..tile_w {
                                    let rgba = region.pixel(x, y);
                                    let dst_x = tile_px_x + x;
                                    let dst_y = tile_px_y + y;
                                    let dst_idx = ((dst_y * full_width + dst_x) as usize) * num_channels as usize;
                                    for c in 0..num_channels as usize {
                                        full_data[dst_idx + c] = rgba[c];
                                    }
                                }
                            }
                        }
                    }
                    
                    // Convert to Full storage
                    let cached = CachedImageData {
                        data: full_data,
                        width: full_width,
                        height: full_height,
                        channels: num_channels,
                        mips: HashMap::new(),
                    };
                    
                    // Replace streaming storage with full storage
                    drop(storage);
                    let mut storage = self.image_storage.write().unwrap();
                    storage.insert(storage_key.clone(), ImageStorage::Full(cached));
                    drop(storage);
                    
                    // Recursively call to use the Full path now
                    return self.load_tile(path, subimage, mip_level, tile_x, tile_y);
                }
                
                let tile_px_x = tile_x * self.tile_size;
                let tile_px_y = tile_y * self.tile_size;
                let tile_w = self.tile_size.min(width.saturating_sub(tile_px_x));
                let tile_h = self.tile_size.min(height.saturating_sub(tile_px_y));
                
                if tile_w == 0 || tile_h == 0 {
                    return Err(IoError::DecodeError("Tile out of bounds".into()));
                }
                
                // Read region from streaming source
                let region = source.read_region(tile_px_x, tile_px_y, tile_w, tile_h)?;
                
                // Convert Region (RGBA) to tile format
                let ch = *channels as usize;
                let mut tile_data = Vec::with_capacity((tile_w * tile_h) as usize * ch);
                
                for y in 0..tile_h {
                    for x in 0..tile_w {
                        let rgba = region.pixel(x, y);
                        for c in 0..ch {
                            tile_data.push(rgba[c]);
                        }
                    }
                }
                
                Ok(Tile::new(tile_w, tile_h, *channels, tile_data))
            }
            
            ImageStorage::Full(cached) => {
                // Original full-image logic
                let mip_width = (cached.width >> mip_level).max(1);
                let mip_height = (cached.height >> mip_level).max(1);
                
                // Get or generate mip level data
                let mip_data: Vec<f32> = if mip_level == 0 {
                    cached.data.clone()
                } else {
                    if let Some(mip) = cached.mips.get(&mip_level) {
                        mip.clone()
                    } else {
                        let mip = generate_mip(&cached.data, cached.width, cached.height, cached.channels, mip_level);
                        cached.mips.insert(mip_level, mip.clone());
                        mip
                    }
                };
                
                let tile_px_x = tile_x * self.tile_size;
                let tile_px_y = tile_y * self.tile_size;
                let tile_w = self.tile_size.min(mip_width.saturating_sub(tile_px_x));
                let tile_h = self.tile_size.min(mip_height.saturating_sub(tile_px_y));
                
                if tile_w == 0 || tile_h == 0 {
                    return Err(IoError::DecodeError("Tile out of bounds".into()));
                }
                
                let channels = cached.channels as usize;
                let mut tile_data = Vec::with_capacity((tile_w * tile_h) as usize * channels);
                
                for y in 0..tile_h {
                    let src_y = tile_px_y + y;
                    for x in 0..tile_w {
                        let src_x = tile_px_x + x;
                        let src_idx = ((src_y * mip_width + src_x) as usize) * channels;
                        for c in 0..channels {
                            tile_data.push(mip_data.get(src_idx + c).copied().unwrap_or(0.0));
                        }
                    }
                }
                
                Ok(Tile::new(tile_w, tile_h, cached.channels, tile_data))
            }
        }
    }

    /// Ensures there's enough space for new data.
    fn ensure_space(&self, needed: usize) {
        let max = self.max_size;
        loop {
            let current = *self.current_size.read().unwrap();
            if current + needed <= max {
                return;
            }

            // Evict LRU tile
            let evict_key = {
                let tail = self.lru_tail.lock().unwrap();
                tail.clone()
            };

            if let Some(key) = evict_key {
                self.evict(&key);
            } else {
                return; // Nothing to evict
            }
        }
    }

    /// Evicts a tile from cache.
    fn evict(&self, key: &TileKey) {
        let size = {
            let mut tiles = self.tiles.write().unwrap();
            if let Some(tile) = tiles.remove(key) {
                tile.size_bytes()
            } else {
                return;
            }
        };

        {
            let mut current_size = self.current_size.write().unwrap();
            *current_size = current_size.saturating_sub(size);
        }

        {
            let mut stats = self.stats.write().unwrap();
            stats.evictions += 1;
            stats.tile_count = stats.tile_count.saturating_sub(1);
        }

        self.remove_from_lru(key);
    }

    /// Adds a key to front of LRU list.
    fn add_to_lru(&self, key: &TileKey) {
        let mut lru_nodes = self.lru_nodes.write().unwrap();
        let mut head = self.lru_head.lock().unwrap();
        let mut tail = self.lru_tail.lock().unwrap();

        let node = LruNode {
            prev: None,
            next: head.clone(),
        };

        if let Some(ref old_head) = *head {
            if let Some(old_node) = lru_nodes.get_mut(old_head) {
                old_node.prev = Some(key.clone());
            }
        }

        lru_nodes.insert(key.clone(), node);
        *head = Some(key.clone());

        if tail.is_none() {
            *tail = Some(key.clone());
        }
    }

    /// Moves key to front of LRU list.
    fn update_lru(&self, key: &TileKey) {
        self.remove_from_lru(key);
        self.add_to_lru(key);
    }

    /// Removes key from LRU list.
    fn remove_from_lru(&self, key: &TileKey) {
        let mut lru_nodes = self.lru_nodes.write().unwrap();
        let mut head = self.lru_head.lock().unwrap();
        let mut tail = self.lru_tail.lock().unwrap();

        if let Some(node) = lru_nodes.remove(key) {
            // Update prev node's next pointer
            if let Some(ref prev_key) = node.prev {
                if let Some(prev_node) = lru_nodes.get_mut(prev_key) {
                    prev_node.next = node.next.clone();
                }
            } else {
                *head = node.next.clone();
            }

            // Update next node's prev pointer
            if let Some(ref next_key) = node.next {
                if let Some(next_node) = lru_nodes.get_mut(next_key) {
                    next_node.prev = node.prev.clone();
                }
            } else {
                *tail = node.prev.clone();
            }
        }
    }

    /// Prefetch hint - queues tiles for loading.
    ///
    /// This loads specified tiles into cache proactively.
    /// Useful when you know which tiles will be needed soon.
    ///
    /// # Arguments
    ///
    /// * `path` - Image file path
    /// * `subimage` - Subimage index  
    /// * `mip_level` - Mip level
    /// * `tile_coords` - List of (tile_x, tile_y) coordinates to prefetch
    pub fn prefetch(
        &self,
        path: impl AsRef<Path>,
        subimage: u32,
        mip_level: u32,
        tile_coords: &[(u32, u32)],
    ) -> IoResult<()> {
        let path = path.as_ref();
        for &(tile_x, tile_y) in tile_coords {
            // Ignore errors - prefetch is best-effort
            let _ = self.get_tile(path, subimage, mip_level, tile_x, tile_y);
        }
        Ok(())
    }

    /// Prefetch all tiles for a mip level.
    ///
    /// Loads entire mip level into cache. Useful for small textures.
    pub fn prefetch_mip(&self, path: impl AsRef<Path>, subimage: u32, mip_level: u32) -> IoResult<()> {
        let path = path.as_ref();
        let info = self.get_image_info(path)?;
        
        let mip_w = info.width_at_mip(mip_level);
        let mip_h = info.height_at_mip(mip_level);
        let tiles_x = (mip_w + self.tile_size - 1) / self.tile_size;
        let tiles_y = (mip_h + self.tile_size - 1) / self.tile_size;
        
        let coords: Vec<_> = (0..tiles_y)
            .flat_map(|ty| (0..tiles_x).map(move |tx| (tx, ty)))
            .collect();
        
        self.prefetch(path, subimage, mip_level, &coords)
    }

    /// Prefetch tiles in a region (pixel coordinates).
    ///
    /// # Arguments
    ///
    /// * `path` - Image file path
    /// * `x`, `y` - Top-left corner in pixels
    /// * `width`, `height` - Region size in pixels  
    /// * `mip_level` - Mip level
    pub fn prefetch_region(
        &self,
        path: impl AsRef<Path>,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        subimage: u32,
        mip_level: u32,
    ) -> IoResult<()> {
        let path = path.as_ref();
        
        let tile_x_start = x / self.tile_size;
        let tile_y_start = y / self.tile_size;
        let tile_x_end = (x + width + self.tile_size - 1) / self.tile_size;
        let tile_y_end = (y + height + self.tile_size - 1) / self.tile_size;
        
        let coords: Vec<_> = (tile_y_start..tile_y_end)
            .flat_map(|ty| (tile_x_start..tile_x_end).map(move |tx| (tx, ty)))
            .collect();
        
        self.prefetch(path, subimage, mip_level, &coords)
    }

    /// Invalidates all tiles for a path.
    pub fn invalidate(&self, path: impl AsRef<Path>) {
        let path = path.as_ref().to_path_buf();

        // Remove image info
        {
            let mut info_cache = self.image_info.write().unwrap();
            info_cache.remove(&path);
        }

        // Remove cached image storage for all subimages of this path
        {
            let mut storage = self.image_storage.write().unwrap();
            storage.retain(|k, _| k.0 != path);
        }

        // Find and remove all tiles for this path
        let keys_to_remove: Vec<_> = {
            let tiles = self.tiles.read().unwrap();
            tiles.keys()
                .filter(|k| k.path == path)
                .cloned()
                .collect()
        };

        for key in keys_to_remove {
            self.evict(&key);
        }
    }

    /// Returns current streaming threshold in bytes.
    pub fn streaming_threshold(&self) -> u64 {
        self.streaming_threshold
    }

    /// Sets streaming threshold.
    pub fn set_streaming_threshold(&mut self, threshold: u64) {
        self.streaming_threshold = threshold;
    }
}

impl Default for ImageCache {
    fn default() -> Self {
        Self::default_cache()
    }
}

/// Computes number of mip levels for given dimensions.
fn compute_mip_levels(width: u32, height: u32) -> u32 {
    let max_dim = width.max(height);
    (max_dim as f32).log2().ceil() as u32 + 1
}

/// Generates a mip level using box filter.
fn generate_mip(data: &[f32], width: u32, height: u32, channels: u32, level: u32) -> Vec<f32> {
    let mut src = data.to_vec();
    let mut src_w = width;
    let mut src_h = height;

    for _ in 0..level {
        let dst_w = (src_w / 2).max(1);
        let dst_h = (src_h / 2).max(1);
        let mut dst = vec![0.0f32; (dst_w * dst_h * channels) as usize];

        for y in 0..dst_h {
            for x in 0..dst_w {
                for c in 0..channels {
                    let mut sum = 0.0;
                    let mut count = 0;

                    for sy in 0..2 {
                        for sx in 0..2 {
                            let src_x = (x * 2 + sx).min(src_w - 1);
                            let src_y = (y * 2 + sy).min(src_h - 1);
                            let idx = ((src_y * src_w + src_x) * channels + c) as usize;
                            if idx < src.len() {
                                sum += src[idx];
                                count += 1;
                            }
                        }
                    }

                    let dst_idx = ((y * dst_w + x) * channels + c) as usize;
                    dst[dst_idx] = if count > 0 { sum / count as f32 } else { 0.0 };
                }
            }
        }

        src = dst;
        src_w = dst_w;
        src_h = dst_h;
    }

    src
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_creation() {
        let cache = ImageCache::new(1024 * 1024);
        assert_eq!(cache.max_size(), 1024 * 1024);
        assert_eq!(cache.size(), 0);
    }

    #[test]
    fn cache_stats() {
        let cache = ImageCache::default();
        let stats = cache.stats();
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 0);
        assert_eq!(stats.hit_rate(), 0.0);
    }

    #[test]
    fn mip_levels() {
        assert_eq!(compute_mip_levels(1, 1), 1);
        assert_eq!(compute_mip_levels(2, 2), 2);
        assert_eq!(compute_mip_levels(4, 4), 3);
        assert_eq!(compute_mip_levels(1024, 1024), 11);
        assert_eq!(compute_mip_levels(1920, 1080), 12); // log2(1920) = 10.9 -> 12
    }

    #[test]
    fn tile_key_hash() {
        let k1 = TileKey::new("test.exr", 0, 0, 0, 0);
        let k2 = TileKey::new("test.exr", 0, 0, 0, 0);
        let k3 = TileKey::new("test.exr", 0, 0, 1, 0);

        assert_eq!(k1, k2);
        assert_ne!(k1, k3);
    }

    #[test]
    fn generate_mip_simple() {
        // 4x4 image, single channel
        let data = vec![
            1.0, 2.0, 3.0, 4.0,
            5.0, 6.0, 7.0, 8.0,
            9.0, 10.0, 11.0, 12.0,
            13.0, 14.0, 15.0, 16.0,
        ];

        let mip1 = generate_mip(&data, 4, 4, 1, 1);
        assert_eq!(mip1.len(), 4); // 2x2

        // Average of 2x2 blocks
        assert!((mip1[0] - 3.5).abs() < 0.001); // (1+2+5+6)/4
        assert!((mip1[1] - 5.5).abs() < 0.001); // (3+4+7+8)/4
    }

    #[test]
    fn cached_image_storage_reuse() {
        // Test that image storage is cached and reused for multiple tile requests
        let cache = ImageCache::new(1024 * 1024);
        
        // Create test image (small, should use Full storage)
        let temp_path = std::env::temp_dir().join("vfx_io_cache_test.exr");
        let mut pixels = Vec::with_capacity(128 * 128 * 4);
        for y in 0..128u32 {
            for x in 0..128u32 {
                pixels.push(x as f32 / 127.0);
                pixels.push(y as f32 / 127.0);
                pixels.push(0.5);
                pixels.push(1.0);
            }
        }
        let image = crate::ImageData::from_f32(128, 128, 4, pixels);
        crate::write(&temp_path, &image).expect("Failed to write test image");
        
        // Request multiple tiles - should reuse cached image storage
        let _ = cache.get_tile(&temp_path, 0, 0, 0, 0).expect("Tile 0,0");
        let _ = cache.get_tile(&temp_path, 0, 0, 1, 0).expect("Tile 1,0");
        let _ = cache.get_tile(&temp_path, 0, 0, 0, 1).expect("Tile 0,1");
        
        // Check stats - should have 3 misses (tiles), but only 1 image load
        let stats = cache.stats();
        assert_eq!(stats.misses, 3);
        assert_eq!(stats.tile_count, 3);
        
        // Request same tile again - should be a hit
        let _ = cache.get_tile(&temp_path, 0, 0, 0, 0).expect("Tile 0,0 again");
        let stats = cache.stats();
        assert_eq!(stats.hits, 1);
        
        let _ = std::fs::remove_file(&temp_path);
    }

    #[test]
    fn streaming_threshold_config() {
        // Test streaming threshold configuration
        let cache = ImageCache::with_streaming_threshold(1024 * 1024, 256 * 1024 * 1024);
        assert_eq!(cache.streaming_threshold(), 256 * 1024 * 1024);
        
        let mut cache2 = ImageCache::new(1024 * 1024);
        cache2.set_streaming_threshold(1024 * 1024 * 1024);
        assert_eq!(cache2.streaming_threshold(), 1024 * 1024 * 1024);
    }
}
