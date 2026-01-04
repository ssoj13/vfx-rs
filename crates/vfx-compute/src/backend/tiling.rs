//! GPU tiling for large images.

/// GPU resource limits.
#[derive(Debug, Clone)]
pub struct GpuLimits {
    /// Maximum texture dimension (width or height).
    pub max_tile_dim: u32,
    /// Maximum buffer size in bytes.
    pub max_buffer_bytes: u64,
    /// Available GPU memory in bytes.
    pub available_memory: u64,
}

impl Default for GpuLimits {
    fn default() -> Self {
        Self {
            max_tile_dim: 16384,
            max_buffer_bytes: 256 * 1024 * 1024, // 256 MB
            available_memory: 2 * 1024 * 1024 * 1024, // 2 GB
        }
    }
}

impl GpuLimits {
    /// Check if image needs tiling.
    pub fn needs_tiling(&self, width: u32, height: u32) -> bool {
        width > self.max_tile_dim || height > self.max_tile_dim
    }
    
    /// Check if image fits in available memory.
    pub fn fits_memory(&self, width: u32, height: u32, channels: u32) -> bool {
        let bytes = (width as u64) * (height as u64) * (channels as u64) * 4;
        bytes <= self.available_memory / 2 // Leave headroom
    }
    
    /// Calculate optimal tile size.
    pub fn optimal_tile_size(&self, width: u32, height: u32) -> u32 {
        let max_dim = width.max(height);
        if max_dim <= 1024 {
            max_dim
        } else if max_dim <= 4096 {
            2048
        } else {
            4096.min(self.max_tile_dim)
        }
    }
}

/// A tile region within an image.
#[derive(Debug, Clone, Copy)]
pub struct Tile {
    /// X offset in source image.
    pub x: u32,
    /// Y offset in source image.
    pub y: u32,
    /// Tile width.
    pub width: u32,
    /// Tile height.
    pub height: u32,
}

impl Tile {
    pub fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self { x, y, width, height }
    }
    
    /// Full image as single tile.
    pub fn full(width: u32, height: u32) -> Self {
        Self::new(0, 0, width, height)
    }
}

/// Generate tiles for processing large images.
pub fn generate_tiles(width: u32, height: u32, tile_size: u32) -> Vec<Tile> {
    let mut tiles = Vec::new();
    
    let mut y = 0;
    while y < height {
        let th = tile_size.min(height - y);
        let mut x = 0;
        while x < width {
            let tw = tile_size.min(width - x);
            tiles.push(Tile::new(x, y, tw, th));
            x += tile_size;
        }
        y += tile_size;
    }
    
    tiles
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_generate_tiles() {
        let tiles = generate_tiles(1000, 1000, 512);
        assert_eq!(tiles.len(), 4); // 2x2 grid
        
        assert_eq!(tiles[0].x, 0);
        assert_eq!(tiles[0].width, 512);
        assert_eq!(tiles[1].x, 512);
        assert_eq!(tiles[1].width, 488);
    }
    
    #[test]
    fn test_single_tile() {
        let tiles = generate_tiles(256, 256, 512);
        assert_eq!(tiles.len(), 1);
        assert_eq!(tiles[0].width, 256);
    }
}
