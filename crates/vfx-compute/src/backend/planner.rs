//! Execution planner with binary search for optimal tile size.
//!
//! Analyzes image dimensions and GPU limits to generate an optimal
//! execution plan with tile sizes that maximize throughput while
//! staying within memory constraints.

use super::tiling::{GpuLimits, Tile, generate_tiles, ProcessingStrategy};
use super::cluster::{TileTriple, TileCluster, SourceRegion, ClusterConfig, 
                     cluster_tiles, analyze_source_region};
use super::memory::format_bytes;

/// Constraints for execution planning.
#[derive(Debug, Clone)]
pub struct Constraints {
    /// Maximum tile dimension.
    pub max_tile_dim: u32,
    /// Memory budget in bytes.
    pub memory_budget: u64,
    /// Minimum tile dimension.
    pub min_tile_dim: u32,
    /// Kernel radius (for convolutions).
    pub kernel_radius: u32,
}

impl Default for Constraints {
    fn default() -> Self {
        Self {
            max_tile_dim: 4096,
            memory_budget: 512 * 1024 * 1024, // 512 MB
            min_tile_dim: 256,
            kernel_radius: 0,
        }
    }
}

impl Constraints {
    pub fn from_limits(limits: &GpuLimits, kernel_radius: u32) -> Self {
        Self {
            max_tile_dim: limits.max_tile_dim.min(8192),
            memory_budget: limits.available_memory,
            min_tile_dim: 256,
            kernel_radius,
        }
    }
}

/// Execution plan for processing an image.
#[derive(Debug, Clone)]
pub struct ExecutionPlan {
    /// Processing strategy.
    pub strategy: ProcessingStrategy,
    /// Tile triples (tile + source region).
    pub tiles: Vec<TileTriple>,
    /// Clustered tiles for optimized transfer.
    pub clusters: Vec<TileCluster>,
    /// Optimal tile size.
    pub tile_size: u32,
    /// Total memory estimate.
    pub total_memory: u64,
    /// Bandwidth savings from clustering (percentage).
    pub clustering_savings: f32,
}

impl ExecutionPlan {
    /// Check if single-pass is possible.
    pub fn is_single_pass(&self) -> bool {
        matches!(self.strategy, ProcessingStrategy::SinglePass)
    }
    
    /// Get tile count.
    pub fn tile_count(&self) -> usize {
        self.tiles.len()
    }
    
    /// Get cluster count.
    pub fn cluster_count(&self) -> usize {
        self.clusters.len()
    }
}

/// Execution planner.
#[derive(Clone)]
pub struct Planner {
    constraints: Constraints,
}

impl Planner {
    pub fn new(constraints: Constraints) -> Self {
        Self { constraints }
    }
    
    pub fn from_limits(limits: &GpuLimits) -> Self {
        Self::new(Constraints::from_limits(limits, 0))
    }
    
    pub fn with_kernel_radius(limits: &GpuLimits, kernel_radius: u32) -> Self {
        Self::new(Constraints::from_limits(limits, kernel_radius))
    }
    
    /// Get mutable reference to constraints.
    pub fn constraints_mut(&mut self) -> &mut Constraints {
        &mut self.constraints
    }
    
    /// Plan execution for image processing.
    pub fn plan(&self, src_width: u32, src_height: u32, channels: u32) -> ExecutionPlan {
        self.plan_with_output(src_width, src_height, src_width, src_height, channels)
    }
    
    /// Plan execution with different output dimensions.
    pub fn plan_with_output(
        &self,
        src_width: u32,
        src_height: u32,
        out_width: u32,
        out_height: u32,
        channels: u32,
    ) -> ExecutionPlan {
        let bytes_per_pixel = (channels as u64) * 4;
        let src_bytes = (src_width as u64) * (src_height as u64) * bytes_per_pixel;
        
        // Check if single pass is possible
        // Need: source + output + intermediate = ~3x source
        let single_pass_bytes = src_bytes * 3;
        
        if single_pass_bytes <= self.constraints.memory_budget &&
           src_width <= self.constraints.max_tile_dim &&
           src_height <= self.constraints.max_tile_dim {
            return ExecutionPlan {
                strategy: ProcessingStrategy::SinglePass,
                tiles: vec![TileTriple::new(
                    Tile::full(out_width, out_height),
                    SourceRegion::new(0, 0, src_width, src_height),
                )],
                clusters: vec![],
                tile_size: src_width.max(src_height),
                total_memory: single_pass_bytes,
                clustering_savings: 0.0,
            };
        }
        
        // Binary search for optimal tile size
        let tile_size = self.find_optimal_tile_size(
            src_width, src_height, out_width, out_height, channels
        );
        
        // Generate tiles
        let tiles = generate_tiles(out_width, out_height, tile_size);
        
        // Create tile triples with source regions
        let triples: Vec<TileTriple> = tiles.iter()
            .map(|t| {
                let source = analyze_source_region(
                    t, 
                    self.constraints.kernel_radius,
                    src_width,
                    src_height,
                );
                TileTriple::new(*t, source)
            })
            .collect();
        
        // Cluster tiles
        let cluster_config = ClusterConfig {
            max_cluster_bytes: self.constraints.memory_budget / 2,
            max_texture_size: self.constraints.max_tile_dim,
            ..Default::default()
        };
        let clusters = cluster_tiles(triples.clone(), &cluster_config);
        
        // Calculate savings
        let without: u64 = triples.iter().map(|t| t.source.size_bytes()).sum();
        let with: u64 = clusters.iter().map(|c| c.source_region.size_bytes()).sum();
        let savings = if without > 0 {
            1.0 - (with as f32 / without as f32)
        } else {
            0.0
        };
        
        // Determine strategy
        let total_memory: u64 = triples.iter().map(|t| t.memory_bytes).sum();
        let ram_threshold = 8 * 1024 * 1024 * 1024u64; // 8 GB
        
        let strategy = if total_memory > ram_threshold {
            ProcessingStrategy::Streaming { tile_size }
        } else {
            ProcessingStrategy::Tiled { 
                tile_size, 
                num_tiles: triples.len() as u32,
            }
        };
        
        ExecutionPlan {
            strategy,
            tiles: triples,
            clusters,
            tile_size,
            total_memory,
            clustering_savings: savings,
        }
    }
    
    /// Binary search for optimal tile size.
    fn find_optimal_tile_size(
        &self,
        src_width: u32,
        src_height: u32,
        _out_width: u32,
        _out_height: u32,
        channels: u32,
    ) -> u32 {
        let bytes_per_pixel = (channels as u64) * 4;
        let border = self.constraints.kernel_radius;
        
        // Memory needed for one tile: (tile + border)^2 * bpp * 3 (src + dst + tmp)
        let max_tile_mem = self.constraints.memory_budget / 3;
        
        // Solve: (tile + 2*border)^2 * bpp <= max_tile_mem
        let max_from_mem = ((max_tile_mem / bytes_per_pixel) as f64).sqrt() as u32;
        let max_from_mem = max_from_mem.saturating_sub(border * 2);
        
        // Clamp to constraints
        let mut tile_size = max_from_mem
            .min(self.constraints.max_tile_dim)
            .max(self.constraints.min_tile_dim);
        
        // Round down to power of 2 for GPU efficiency
        tile_size = round_down_pow2(tile_size);
        
        // Don't exceed image size
        tile_size = tile_size.min(src_width).min(src_height);
        
        // Ensure minimum
        tile_size.max(self.constraints.min_tile_dim)
    }
    
    /// Print plan summary.
    pub fn describe(&self, plan: &ExecutionPlan) -> String {
        let mut desc = String::new();
        
        desc.push_str(&format!("Strategy: {:?}\n", plan.strategy));
        desc.push_str(&format!("Tile size: {}px\n", plan.tile_size));
        desc.push_str(&format!("Tiles: {}\n", plan.tile_count()));
        desc.push_str(&format!("Clusters: {}\n", plan.cluster_count()));
        desc.push_str(&format!("Memory: {}\n", format_bytes(plan.total_memory)));
        
        if plan.clustering_savings > 0.0 {
            desc.push_str(&format!("Clustering savings: {:.1}%\n", 
                plan.clustering_savings * 100.0));
        }
        
        desc
    }
}

/// Round down to nearest power of 2.
fn round_down_pow2(n: u32) -> u32 {
    if n == 0 {
        return 0;
    }
    1 << (31 - n.leading_zeros())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plan_single_pass() {
        let limits = GpuLimits::with_vram(8 * 1024 * 1024 * 1024); // 8 GB
        let planner = Planner::from_limits(&limits);
        
        let plan = planner.plan(1024, 1024, 4);
        
        assert!(plan.is_single_pass());
        assert_eq!(plan.tile_count(), 1);
    }

    #[test]
    fn test_plan_tiled() {
        let limits = GpuLimits::with_vram(256 * 1024 * 1024); // 256 MB
        let planner = Planner::from_limits(&limits);
        
        let plan = planner.plan(8192, 8192, 4);
        
        assert!(!plan.is_single_pass());
        assert!(plan.tile_count() > 1);
        assert!(plan.tile_size >= 256);
    }

    #[test]
    fn test_plan_with_kernel() {
        let limits = GpuLimits::with_vram(512 * 1024 * 1024);
        let planner = Planner::with_kernel_radius(&limits, 10);
        
        let plan = planner.plan(4096, 4096, 4);
        
        // With kernel radius, tiles need extra border
        for triple in &plan.tiles {
            assert!(triple.source.w >= triple.tile.width);
            assert!(triple.source.h >= triple.tile.height);
        }
    }

    #[test]
    fn test_clustering_savings() {
        let limits = GpuLimits::with_vram(512 * 1024 * 1024);
        let planner = Planner::with_kernel_radius(&limits, 5);
        
        let plan = planner.plan(4096, 4096, 4);
        
        if plan.cluster_count() > 0 {
            println!("Clustering savings: {:.1}%", plan.clustering_savings * 100.0);
            // Should have some savings with overlapping regions
            assert!(plan.clustering_savings >= 0.0);
        }
    }

    #[test]
    fn test_round_down_pow2() {
        assert_eq!(round_down_pow2(1000), 512);
        assert_eq!(round_down_pow2(512), 512);
        assert_eq!(round_down_pow2(1024), 1024);
        assert_eq!(round_down_pow2(2000), 1024);
    }
}
