//! UDIM texture resolver
//!
//! UDIM (U DIMension) naming convention for multi-tile textures.
//! Formula: UDIM = 1001 + U + (V * 10)
//! Where U is column (0-9) and V is row (0-99)

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::IoResult;

/// UDIM tile coordinate
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UdimTile {
    /// U tile index (column, 0-9)
    pub u: u32,
    /// V tile index (row, 0-99)
    pub v: u32,
}

impl UdimTile {
    /// Create tile from UV indices
    pub fn new(u: u32, v: u32) -> Self {
        Self { u, v }
    }

    /// Create tile from UDIM number (1001-1999)
    pub fn from_udim(udim: u32) -> Option<Self> {
        if !(1001..=1999).contains(&udim) {
            return None;
        }
        let offset = udim - 1001;
        Some(Self {
            u: offset % 10,
            v: offset / 10,
        })
    }

    /// Get UDIM number for this tile
    pub fn udim(&self) -> u32 {
        1001 + self.u + self.v * 10
    }

    /// Create tile from UV coordinates (fractional)
    pub fn from_uv(u: f32, v: f32) -> Self {
        // Floor to get tile indices, handle negative UVs
        let ui = if u >= 0.0 { u as u32 } else { 0 };
        let vi = if v >= 0.0 { v as u32 } else { 0 };
        Self { u: ui.min(9), v: vi.min(99) }
    }
}

/// UDIM pattern markers in file paths
const UDIM_MARKERS: &[&str] = &["<UDIM>", "<udim>", "_UDIM_"];

/// UDIM texture set resolver
#[derive(Debug)]
pub struct UdimResolver {
    /// Base pattern with UDIM placeholder
    pattern: String,
    /// Discovered tiles: UDIM -> actual path
    tiles: HashMap<u32, PathBuf>,
}

impl UdimResolver {
    /// Create resolver from pattern path
    /// Pattern can contain `<UDIM>`, `<udim>`, or `_UDIM_` placeholder
    pub fn new(pattern: impl AsRef<Path>) -> IoResult<Self> {
        let pattern_str = pattern.as_ref().to_string_lossy().to_string();

        // Check for UDIM marker
        let has_marker = UDIM_MARKERS.iter().any(|m| pattern_str.contains(m));

        if !has_marker {
            // Try to detect UDIM number in path (e.g., texture.1001.exr)
            if let Some(base_pattern) = Self::detect_udim_in_path(&pattern_str) {
                let mut resolver = Self {
                    pattern: base_pattern,
                    tiles: HashMap::new(),
                };
                resolver.scan_tiles()?;
                return Ok(resolver);
            }

            // Single file, no UDIM
            let path = PathBuf::from(&pattern_str);
            if path.exists() {
                let mut tiles = HashMap::new();
                tiles.insert(1001, path);
                return Ok(Self {
                    pattern: pattern_str,
                    tiles,
                });
            }
        }

        let mut resolver = Self {
            pattern: pattern_str,
            tiles: HashMap::new(),
        };
        resolver.scan_tiles()?;
        Ok(resolver)
    }

    /// Detect UDIM number in path and return base pattern
    fn detect_udim_in_path(path: &str) -> Option<String> {
        // Look for 4-digit number 1001-1999 with delimiters
        let patterns = [
            ('.', '.'),  // texture.1001.exr
            ('_', '.'),  // texture_1001.exr
            ('_', '_'),  // texture_1001_diffuse
        ];

        for (delim_start, delim_end) in patterns {
            if let Some(result) = Self::try_detect_udim(path, delim_start, delim_end) {
                return Some(result);
            }
        }
        None
    }

    /// Try to detect UDIM with specific delimiters
    fn try_detect_udim(path: &str, delim_start: char, delim_end: char) -> Option<String> {
        let bytes = path.as_bytes();
        for i in 0..bytes.len() {
            if bytes[i] as char != delim_start {
                continue;
            }
            // Check if we have 4 digits after delimiter
            if i + 5 > bytes.len() {
                continue;
            }
            let candidate = &path[i + 1..i + 5];
            if !candidate.chars().all(|c| c.is_ascii_digit()) {
                continue;
            }
            // Check end delimiter (or end of path for some patterns)
            let has_end = if i + 5 < bytes.len() {
                bytes[i + 5] as char == delim_end
            } else {
                delim_end == '.'
            };
            if !has_end {
                continue;
            }
            // Parse and validate UDIM range
            let udim: u32 = candidate.parse().ok()?;
            if !(1001..=1999).contains(&udim) {
                continue;
            }
            // Build replacement pattern
            let mut result = String::with_capacity(path.len() + 2);
            result.push_str(&path[..i + 1]);
            result.push_str("<UDIM>");
            result.push_str(&path[i + 5..]);
            return Some(result);
        }
        None
    }

    /// Scan filesystem for existing UDIM tiles
    fn scan_tiles(&mut self) -> IoResult<()> {
        // Find parent directory
        let pattern_path = PathBuf::from(&self.pattern);
        let parent = pattern_path.parent().unwrap_or(Path::new("."));
        let filename = pattern_path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();

        if !parent.exists() {
            return Ok(());
        }

        // Scan directory
        for entry in std::fs::read_dir(parent)? {
            let entry = entry?;
            let entry_name = entry.file_name().to_string_lossy().to_string();

            if let Some(udim) = Self::extract_udim(&filename, &entry_name) {
                if (1001..=1999).contains(&udim) {
                    self.tiles.insert(udim, entry.path());
                }
            }
        }

        Ok(())
    }

    /// Extract UDIM number by comparing pattern with actual filename
    fn extract_udim(pattern: &str, actual: &str) -> Option<u32> {
        // Find UDIM marker position in pattern
        for marker in UDIM_MARKERS {
            if let Some(pos) = pattern.find(marker) {
                // Check if actual filename has digits at same position
                if actual.len() >= pos + 4 {
                    let prefix = &pattern[..pos];
                    let suffix = &pattern[pos + marker.len()..];

                    if actual.starts_with(prefix) && actual.ends_with(suffix) {
                        let mid_start = pos;
                        let mid_end = actual.len() - suffix.len();
                        if mid_end - mid_start == 4 {
                            let udim_str = &actual[mid_start..mid_end];
                            return udim_str.parse().ok();
                        }
                    }
                }
            }
        }
        None
    }

    /// Get path for specific UV coordinates
    pub fn resolve_uv(&self, u: f32, v: f32) -> Option<&Path> {
        let tile = UdimTile::from_uv(u, v);
        self.resolve_tile(tile)
    }

    /// Get path for specific tile
    pub fn resolve_tile(&self, tile: UdimTile) -> Option<&Path> {
        self.tiles.get(&tile.udim()).map(|p| p.as_path())
    }

    /// Get path for specific UDIM number
    pub fn resolve_udim(&self, udim: u32) -> Option<&Path> {
        self.tiles.get(&udim).map(|p| p.as_path())
    }

    /// Get all available tiles
    pub fn tiles(&self) -> impl Iterator<Item = (UdimTile, &Path)> {
        self.tiles.iter().filter_map(|(udim, path)| {
            UdimTile::from_udim(*udim).map(|tile| (tile, path.as_path()))
        })
    }

    /// Get number of available tiles
    pub fn tile_count(&self) -> usize {
        self.tiles.len()
    }

    /// Check if specific tile exists
    pub fn has_tile(&self, tile: UdimTile) -> bool {
        self.tiles.contains_key(&tile.udim())
    }

    /// Get bounding box of available tiles
    pub fn bounds(&self) -> Option<(UdimTile, UdimTile)> {
        if self.tiles.is_empty() {
            return None;
        }

        let mut min_u = u32::MAX;
        let mut min_v = u32::MAX;
        let mut max_u = 0u32;
        let mut max_v = 0u32;

        for udim in self.tiles.keys() {
            if let Some(tile) = UdimTile::from_udim(*udim) {
                min_u = min_u.min(tile.u);
                min_v = min_v.min(tile.v);
                max_u = max_u.max(tile.u);
                max_v = max_v.max(tile.v);
            }
        }

        Some((UdimTile::new(min_u, min_v), UdimTile::new(max_u, max_v)))
    }

    /// Build path for a specific UDIM (even if file doesn't exist)
    pub fn build_path(&self, udim: u32) -> PathBuf {
        let udim_str = udim.to_string();
        let mut result = self.pattern.clone();
        for marker in UDIM_MARKERS {
            result = result.replace(marker, &udim_str);
        }
        PathBuf::from(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn udim_tile_basic() {
        let tile = UdimTile::new(0, 0);
        assert_eq!(tile.udim(), 1001);

        let tile = UdimTile::new(1, 0);
        assert_eq!(tile.udim(), 1002);

        let tile = UdimTile::new(0, 1);
        assert_eq!(tile.udim(), 1011);

        // 1001 + 9 + 9*10 = 1100
        let tile = UdimTile::new(9, 9);
        assert_eq!(tile.udim(), 1100);
    }

    #[test]
    fn udim_from_number() {
        let tile = UdimTile::from_udim(1001).unwrap();
        assert_eq!(tile.u, 0);
        assert_eq!(tile.v, 0);

        let tile = UdimTile::from_udim(1025).unwrap();
        assert_eq!(tile.u, 4);
        assert_eq!(tile.v, 2);

        assert!(UdimTile::from_udim(1000).is_none());
        assert!(UdimTile::from_udim(2000).is_none());
    }

    #[test]
    fn udim_from_uv() {
        let tile = UdimTile::from_uv(0.5, 0.5);
        assert_eq!(tile.udim(), 1001);

        let tile = UdimTile::from_uv(1.5, 0.5);
        assert_eq!(tile.udim(), 1002);

        let tile = UdimTile::from_uv(0.5, 1.5);
        assert_eq!(tile.udim(), 1011);

        // Edge cases
        let tile = UdimTile::from_uv(-0.5, 0.5);
        assert_eq!(tile.udim(), 1001); // Clamped to 0

        let tile = UdimTile::from_uv(15.0, 0.5);
        assert_eq!(tile.udim(), 1010); // Clamped to 9
    }

    #[test]
    fn udim_roundtrip() {
        for udim in 1001..=1099 {
            let tile = UdimTile::from_udim(udim).unwrap();
            assert_eq!(tile.udim(), udim);
        }
    }

    #[test]
    fn detect_udim_pattern() {
        let path = "textures/diffuse.1001.exr";
        let base = UdimResolver::detect_udim_in_path(path);
        assert_eq!(base, Some("textures/diffuse.<UDIM>.exr".to_string()));

        let path = "textures/diffuse_1023.exr";
        let base = UdimResolver::detect_udim_in_path(path);
        assert_eq!(base, Some("textures/diffuse_<UDIM>.exr".to_string()));
    }

    #[test]
    fn extract_udim_from_filename() {
        let pattern = "texture.<UDIM>.exr";
        assert_eq!(UdimResolver::extract_udim(pattern, "texture.1001.exr"), Some(1001));
        assert_eq!(UdimResolver::extract_udim(pattern, "texture.1025.exr"), Some(1025));
        assert_eq!(UdimResolver::extract_udim(pattern, "texture.abc.exr"), None);
    }

    #[test]
    fn build_path() {
        let resolver = UdimResolver {
            pattern: "tex.<UDIM>.exr".to_string(),
            tiles: HashMap::new(),
        };

        assert_eq!(resolver.build_path(1001), PathBuf::from("tex.1001.exr"));
        assert_eq!(resolver.build_path(1023), PathBuf::from("tex.1023.exr"));
    }
}
