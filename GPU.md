# GPU Backend Architecture - Porting Plan from stool-rs

## Overview

Портирование backend-инфраструктуры из `_ref/stool-rs` для поддержки CPU/wgpu/CUDA с автовыбором, тайлингом и стримингом.

---

## Phase 1: Core Infrastructure (Foundation)

### 1.1 GpuPrimitives Trait (`backend/primitives.rs`)

**Источник:** `stool-rs/warper/src/backend/gpu_primitives.rs`

```rust
pub trait GpuPrimitives: Send + Sync {
    type Source: SourceHandle;
    type Output: OutputHandle;
    
    fn upload_source(&self, image: &Image) -> Result<Self::Source>;
    fn upload_source_owned(&self, image: Image) -> Result<Self::Source>;
    fn allocate_output(&self, w: u32, h: u32, channels: u8) -> Result<Self::Output>;
    fn download_output(&self, output: &Self::Output) -> Result<Image>;
    fn limits(&self) -> &GpuLimits;
}

pub trait SourceHandle: Clone + Send + Sync {
    fn size_bytes(&self) -> u64;
    fn dimensions(&self) -> (u32, u32);
}

pub trait OutputHandle: Send + Sync {
    fn size_bytes(&self) -> u64;
    fn dimensions(&self) -> (u32, u32);
}
```

**Адаптация для vfx-rs:**
- Убрать StMap (warp-специфичный)
- Добавить `execute_kernel(&self, kernel: &dyn Kernel, inputs: &[&Self::Source], output: &Self::Output)`
- Kernel trait для всех операций (blur, morpho, color и т.д.)

### 1.2 Resource Limits (`backend/limits.rs`)

**Источник:** `stool-rs/warper/src/backend/tiling.rs` + `memory.rs`

```rust
pub struct GpuLimits {
    pub max_texture_dim: u32,      // 16384 typical
    pub max_buffer_bytes: u64,     // wgpu limits
    pub max_compute_invocations: u32,
}

pub struct ResourceLimits {
    pub max_tile_dim: u32,
    pub max_buffer_bytes: u64,
    pub available_memory: u64,
    pub cache_budget: u64,
}

// Memory utilities
pub const BYTES_PER_PIXEL: u64 = 16;  // RGBA f32
pub const SAFE_MEMORY_FRACTION: u64 = 80;

pub fn available_memory() -> u64;
pub fn warp_memory_budget() -> u64;   // 70% of available
pub fn cache_memory_budget() -> u64;  // 25% of available
```

**Environment overrides (из stool-rs):**
- `VFX_RAM_MAX` - максимум RAM в bytes
- `VFX_RAM_PCT` - процент от системной RAM
- `VFX_MEM_MB` - явный лимит в MB

---

## Phase 2: Backend Detection & VRAM

### 2.1 Backend Detection (`backend/detect.rs`)

**Источник:** `stool-rs/warper/src/backend/detect.rs`

```rust
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    Auto,
    Cpu,
    Wgpu,
    Cuda,
}

pub struct BackendInfo {
    pub backend: Backend,
    pub name: String,
    pub available: bool,
    pub priority: u32,        // CPU=10, Wgpu=100, CUDA=200
    pub device: Option<String>,
    pub vram_total: Option<u64>,
    pub vram_free: Option<u64>,
    pub unavailable_reason: Option<String>,
}

pub fn detect_backends() -> Vec<BackendInfo>;
pub fn select_best_backend() -> Backend;

// Software renderer detection (llvmpipe, swiftshader, lavapipe)
fn is_software_renderer(info: &wgpu::AdapterInfo) -> bool;
```

**Приоритеты:**
1. CUDA (priority=200) - если доступен NVML
2. wgpu с discrete GPU (priority=100)
3. wgpu с integrated GPU (priority=50)
4. CPU (priority=10) - всегда fallback

### 2.2 VRAM Detection (`backend/vram.rs`)

**Источник:** `stool-rs/warper/src/vram_detect.rs`

```rust
pub trait VramBackend: Send + Sync {
    fn total(&self) -> u64;
    fn free(&self) -> Option<u64>;
    fn name(&self) -> Option<&str>;
    fn method(&self) -> &'static str;
}
```

**Platform implementations:**

| Platform | Backend | Method |
|----------|---------|--------|
| macOS | MetalBackend | `device.recommended_max_working_set_size()` |
| Windows | DxgiBackend | `DedicatedVideoMemory` + `QueryVideoMemoryInfo` |
| Linux NVIDIA | NvmlBackend | `nvmlDeviceGetMemoryInfo` |
| Linux AMD/Intel | SysfsBackend | `/sys/class/drm/cardN/device/mem_info_*` |

**Fallback chain:**
1. Try native API (Metal/DXGI/NVML)
2. Try sysfs on Linux
3. Fallback to wgpu adapter limits (often wrong)
4. Use conservative 2GB default

---

## Phase 3: CPU Backend

### 3.1 CPU Primitives (`backend/cpu/primitives.rs`)

**Источник:** `stool-rs/warper/src/backend/cpu_primitives.rs`

```rust
pub struct CpuPrimitives {
    thread_pool: rayon::ThreadPool,
}

pub struct CpuSource {
    image: Arc<Image>,      // Zero-copy via Arc
    size_bytes: u64,
}

pub struct CpuOutput {
    data: UnsafeCell<Vec<f32>>,  // Interior mutability for parallel writes
    width: u32,
    height: u32,
    channels: u8,
}

impl GpuPrimitives for CpuPrimitives {
    fn upload_source_owned(&self, image: Image) -> Result<Self::Source> {
        // No clone! Just Arc::new()
        Ok(CpuSource { image: Arc::new(image), ... })
    }
    
    fn execute_kernel(&self, kernel: &dyn Kernel, ...) -> Result<()> {
        // Rayon parallel_chunks по строкам
        (0..height).into_par_iter().for_each(|y| {
            kernel.process_row(y, inputs, output);
        });
    }
}
```

**Key optimizations из stool-rs:**
- `Arc<Image>` вместо clone для источников
- Row-based parallel processing с rayon
- `UnsafeCell` для zero-copy output writes
- SIMD где возможно (через `std::simd` или `packed_simd`)

### 3.2 CPU Backend Wrapper (`backend/cpu/mod.rs`)

```rust
pub struct CpuBackend {
    executor: Executor<CpuPrimitives>,
}

impl ProcessingBackend for CpuBackend {
    fn process(&self, op: &Operation, input: &Image) -> Result<Image>;
    fn available_memory(&self) -> u64;
    fn name(&self) -> &str { "CPU (rayon)" }
}
```

---

## Phase 4: wgpu Backend

### 4.1 wgpu Primitives (`backend/wgpu/primitives.rs`)

**Источник:** `stool-rs/warper/src/backend/wgpu_backend.rs`

```rust
pub struct WgpuPrimitives {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    limits: GpuLimits,
    pipelines: HashMap<String, wgpu::ComputePipeline>,
}

pub struct WgpuSource {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    size_bytes: u64,
}

pub struct WgpuOutput {
    texture: wgpu::Texture,
    staging_buffer: wgpu::Buffer,  // For readback
    size_bytes: u64,
}
```

**Pipeline caching:**
- Compile kernels once, cache by operation type
- Use shader specialization constants for filter sizes

### 4.2 wgpu Backend Wrapper

```rust
pub struct WgpuBackend {
    executor: Executor<WgpuPrimitives>,
    vram_detector: Box<dyn VramBackend>,
}

impl ProcessingBackend for WgpuBackend {
    fn available_memory(&self) -> u64 {
        self.vram_detector.free().unwrap_or(self.vram_detector.total() * 80 / 100)
    }
}
```

---

## Phase 5: CUDA Backend

### 5.1 CUDA Primitives (`backend/cuda/primitives.rs`)

```rust
pub struct CudaPrimitives {
    context: cuda::Context,
    stream: cuda::Stream,
    limits: GpuLimits,
}

pub struct CudaSource {
    ptr: cuda::DevicePtr<f32>,
    size_bytes: u64,
}

pub struct CudaOutput {
    ptr: cuda::DevicePtr<f32>,
    size_bytes: u64,
}
```

**Bindings:**
- Use `cudarc` crate for safe CUDA bindings
- Or `cuda-sys` + wrapper for lower level control
- NVML for memory detection

### 5.2 CUDA Kernels

```rust
// Compile PTX at build time or runtime
static BLUR_KERNEL: &str = include_str!("kernels/blur.ptx");
static MORPHO_KERNEL: &str = include_str!("kernels/morpho.ptx");
```

---

## Phase 6: Tiling Infrastructure

### 6.1 Tile Types (`backend/tiling.rs`)

**Источник:** `stool-rs/warper/src/backend/tiling.rs`

```rust
#[derive(Clone, Copy)]
pub struct Tile {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

pub struct SourceRegion {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
    pub border: u32,  // Padding for convolutions
}

pub fn generate_tiles(out_w: u32, out_h: u32, tile_size: u32) -> Vec<Tile>;
pub fn analyze_source_region(tile: &Tile, kernel_radius: u32) -> SourceRegion;
```

### 6.2 Tile Clustering (`backend/cluster.rs`)

**Источник:** `stool-rs/warper/src/backend/tile_cluster.rs`

```rust
pub struct TileCluster {
    pub tiles: Vec<Tile>,
    pub source_region: SourceRegion,  // Unified region for all tiles
    pub memory_bytes: u64,
}

pub struct ClusterConfig {
    pub max_cluster_bytes: u64,
    pub merge_overlap_threshold: f32,  // 0.2 = 20% overlap triggers merge
    pub max_texture_size: u32,
}

pub fn cluster_tiles(
    tiles: Vec<TileTriple>,
    config: &ClusterConfig,
) -> Vec<TileCluster>;

// Savings calculation
pub fn compute_savings(unclustered: &[Tile], clustered: &[TileCluster]) -> (u64, u64);
```

**Оптимизации:**
- Morton code sorting for cache locality
- Merge tiles with >20% source overlap
- 50-70% PCIe bandwidth savings typical

### 6.3 Execution Planner (`backend/planner.rs`)

**Источник:** `stool-rs/warper/src/backend/planner.rs`

```rust
pub struct ExecutionPlan {
    pub strategy: ProcessingStrategy,
    pub tiles: Vec<TileTriple>,
    pub total_memory: u64,
    pub estimated_time_ms: u64,
}

pub struct TileTriple {
    pub source: SourceRegion,
    pub tile: Tile,
    pub output: Tile,
    pub memory_bytes: u64,
}

pub struct Constraints {
    pub max_tile_dim: u32,
    pub memory_budget: u64,
    pub min_tile_dim: u32,
}

pub struct Planner {
    constraints: Constraints,
}

impl Planner {
    /// Binary search for optimal tile size
    pub fn plan(&self, src_dims: (u32, u32), out_dims: (u32, u32), kernel_radius: u32) -> ExecutionPlan;
}
```

---

## Phase 7: Processing Strategy

### 7.1 Strategy Selection (`backend/strategy.rs`)

**Источник:** `stool-rs/warper/src/backend/strategy.rs`

```rust
pub enum ProcessingStrategy {
    /// Source fits in ≤40% VRAM - upload once, process all tiles
    FullSource,
    
    /// Source 40-80% VRAM - cluster tiles, cache regions
    RegionCache,
    
    /// Source >80% VRAM - adaptive tiling with streaming
    AdaptiveTiled,
}

impl ProcessingStrategy {
    pub fn select(source_bytes: u64, available_vram: u64) -> Self {
        let ratio = source_bytes as f64 / available_vram as f64;
        match ratio {
            r if r <= 0.4 => Self::FullSource,
            r if r <= 0.8 => Self::RegionCache,
            _ => Self::AdaptiveTiled,
        }
    }
}
```

### 7.2 Unified Executor (`backend/executor.rs`)

```rust
pub struct Executor<G: GpuPrimitives> {
    gpu: G,
    source_cache: Mutex<Option<RegionCache<G::Source>>>,
    planner: Planner,
}

impl<G: GpuPrimitives> Executor<G> {
    pub fn process(&self, op: &Operation, input: &Image) -> Result<Image> {
        let strategy = ProcessingStrategy::select(input.size_bytes(), self.gpu.limits().available_memory);
        
        match strategy {
            FullSource => self.process_full(op, input),
            RegionCache => self.process_cached(op, input),
            AdaptiveTiled => self.process_tiled(op, input),
        }
    }
}
```

---

## Phase 8: Region Cache

### 8.1 LRU Cache (`backend/cache.rs`)

**Источник:** `stool-rs/warper/src/backend/region_cache.rs`

```rust
#[derive(Hash, Eq, PartialEq, Clone)]
pub struct RegionKey {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

pub struct CachedRegion<T> {
    pub handle: T,
    pub key: RegionKey,
    pub size_bytes: u64,
    pub last_access: Instant,
}

pub struct RegionCache<T> {
    regions: HashMap<RegionKey, CachedRegion<T>>,
    access_order: VecDeque<RegionKey>,  // LRU tracking
    total_bytes: u64,
    max_bytes: u64,
}

impl<T: SourceHandle> RegionCache<T> {
    pub fn get(&mut self, key: &RegionKey) -> Option<&T>;
    pub fn insert(&mut self, key: RegionKey, handle: T, size: u64);
    pub fn evict_lru(&mut self) -> Option<CachedRegion<T>>;
    pub fn clear(&mut self);
}
```

**Use cases:**
- Viewer pan/zoom - cache visible regions
- Animation playback - frame-to-frame coherence
- Multi-pass processing - reuse source uploads

---

## Phase 9: Streaming I/O

### 9.1 Streaming Traits (`backend/streaming.rs`)

**Источник:** `stool-rs/warper/src/backend/streaming_io.rs`

```rust
pub trait StreamingSource: Send {
    fn dimensions(&self) -> (u32, u32);
    fn channels(&self) -> u8;
    fn read_region(&mut self, x: u32, y: u32, w: u32, h: u32) -> Result<Image>;
    fn supports_random_access(&self) -> bool { false }
}

pub trait StreamingOutput: Send {
    fn dimensions(&self) -> (u32, u32);
    fn write_region(&mut self, x: u32, y: u32, data: &Image) -> Result<()>;
    fn finalize(self) -> Result<()>;
}
```

### 9.2 Format Implementations

```rust
// TIFF - true random access via chunks
pub struct TiffStreamingSource {
    decoder: tiff::decoder::Decoder<BufReader<File>>,
    chunk_offsets: Vec<u64>,
}

impl StreamingSource for TiffStreamingSource {
    fn supports_random_access(&self) -> bool { true }
    fn read_region(&mut self, x: u32, y: u32, w: u32, h: u32) -> Result<Image> {
        // Seek directly to required chunks
    }
}

// EXR - lazy loading (header only at open)
pub struct ExrStreamingSource {
    path: PathBuf,
    header: exr::Header,
    loaded: Option<Image>,
}

// Memory source - keeps native format
pub struct MemorySource {
    data: Vec<u8>,
    format: ImageFormat,
    decoded: Option<Image>,
}
```

### 9.3 Streaming Executor (`backend/streaming_executor.rs`)

**Источник:** `stool-rs/warper/src/backend/streaming_executor.rs`

```rust
pub struct StreamingExecutor<G: GpuPrimitives> {
    gpu: G,
    tile_size: u32,
}

impl<G: GpuPrimitives> StreamingExecutor<G> {
    pub fn process_streaming(
        &self,
        op: &Operation,
        source: &mut dyn StreamingSource,
        output: &mut dyn StreamingOutput,
    ) -> Result<()> {
        let tiles = generate_tiles(output.dimensions(), self.tile_size);
        
        for tile in tiles {
            let region = analyze_source_region(&tile, op.kernel_radius());
            let src_data = source.read_region(region.x, region.y, region.w, region.h)?;
            
            let gpu_src = self.gpu.upload_source(&src_data)?;
            let gpu_out = self.gpu.allocate_output(tile.w, tile.h, src_data.channels())?;
            
            self.gpu.execute_kernel(op, &gpu_src, &gpu_out)?;
            
            let result = self.gpu.download_output(&gpu_out)?;
            output.write_region(tile.x, tile.y, &result)?;
        }
        
        output.finalize()
    }
}
```

---

## Phase 10: Unified API

### 10.1 Backend Dispatcher (`backend/mod.rs`)

```rust
pub enum AnyBackend {
    Cpu(CpuBackend),
    Wgpu(WgpuBackend),
    Cuda(CudaBackend),
}

pub fn create_backend(backend: Backend) -> Result<AnyBackend> {
    match backend {
        Backend::Auto => {
            let selected = select_best_backend();
            create_backend(selected)
        }
        Backend::Cuda => {
            #[cfg(feature = "cuda")]
            return Ok(AnyBackend::Cuda(CudaBackend::new()?));
            #[cfg(not(feature = "cuda"))]
            anyhow::bail!("CUDA support not compiled");
        }
        Backend::Wgpu => Ok(AnyBackend::Wgpu(WgpuBackend::new()?)),
        Backend::Cpu => Ok(AnyBackend::Cpu(CpuBackend::new())),
    }
}

// Macro for DRY delegation
macro_rules! impl_backend_dispatch {
    ($self:expr, $method:ident $(, $arg:expr)*) => {
        match $self {
            AnyBackend::Cpu(b) => b.$method($($arg),*),
            AnyBackend::Wgpu(b) => b.$method($($arg),*),
            AnyBackend::Cuda(b) => b.$method($($arg),*),
        }
    };
}
```

### 10.2 High-Level Processing API

```rust
pub struct Processor {
    backend: AnyBackend,
}

impl Processor {
    pub fn new() -> Result<Self> {
        Ok(Self { backend: create_backend(Backend::Auto)? })
    }
    
    pub fn with_backend(backend: Backend) -> Result<Self> {
        Ok(Self { backend: create_backend(backend)? })
    }
    
    // Single image
    pub fn process(&self, op: &Operation, input: &Image) -> Result<Image>;
    
    // Streaming (large files)
    pub fn process_streaming(
        &self,
        op: &Operation,
        source: &mut dyn StreamingSource,
        output: &mut dyn StreamingOutput,
    ) -> Result<()>;
    
    // Batch
    pub fn process_batch(&self, op: &Operation, inputs: &[Image]) -> Result<Vec<Image>>;
}
```

---

## Implementation Order

### Sprint 1: Foundation (Week 1-2)
1. [ ] `backend/primitives.rs` - GpuPrimitives trait
2. [ ] `backend/limits.rs` - Resource limits, memory utilities
3. [ ] `backend/tiling.rs` - Tile, SourceRegion, generate_tiles
4. [ ] `backend/cpu/primitives.rs` - CpuPrimitives
5. [ ] `backend/cpu/mod.rs` - CpuBackend

### Sprint 2: Detection & VRAM (Week 3)
6. [ ] `backend/vram.rs` - VramBackend trait + implementations
7. [ ] `backend/detect.rs` - detect_backends, select_best_backend
8. [ ] `backend/wgpu/primitives.rs` - WgpuPrimitives
9. [ ] `backend/wgpu/mod.rs` - WgpuBackend

### Sprint 3: Tiling & Caching (Week 4)
10. [ ] `backend/planner.rs` - ExecutionPlan, Constraints, binary search
11. [ ] `backend/cluster.rs` - TileCluster, Morton sorting
12. [ ] `backend/cache.rs` - RegionCache LRU
13. [ ] `backend/strategy.rs` - ProcessingStrategy enum

### Sprint 4: Executor & Streaming (Week 5)
14. [ ] `backend/executor.rs` - Unified Executor<G>
15. [ ] `backend/streaming.rs` - StreamingSource/Output traits
16. [ ] `backend/streaming_executor.rs` - StreamingExecutor

### Sprint 5: CUDA & Polish (Week 6)
17. [ ] `backend/cuda/primitives.rs` - CudaPrimitives (if feature enabled)
18. [ ] `backend/cuda/mod.rs` - CudaBackend
19. [ ] `backend/mod.rs` - AnyBackend dispatcher
20. [ ] Integration tests, benchmarks

---

## Files to Port from stool-rs

| stool-rs file | vfx-rs target | Adaptation needed |
|--------------|---------------|-------------------|
| `gpu_primitives.rs` | `primitives.rs` | Remove StMap, add Kernel trait |
| `tiling.rs` | `tiling.rs` | Minor: rename warp → kernel |
| `planner.rs` | `planner.rs` | Remove warp specifics |
| `tile_cluster.rs` | `cluster.rs` | Direct port |
| `region_cache.rs` | `cache.rs` | Direct port |
| `strategy.rs` | `strategy.rs` + `executor.rs` | Split, generalize |
| `detect.rs` | `detect.rs` | Add CUDA detection |
| `vram_detect.rs` | `vram.rs` | Direct port |
| `memory.rs` | `limits.rs` | Merge with limits |
| `cpu_primitives.rs` | `cpu/primitives.rs` | Adapt for general kernels |
| `cpu_backend.rs` | `cpu/mod.rs` | Wrapper only |
| `wgpu_backend.rs` | `wgpu/mod.rs` | Adapt for general kernels |
| `streaming_io.rs` | `streaming.rs` | Direct port |
| `streaming_executor.rs` | `streaming_executor.rs` | Adapt for general ops |

---

## Feature Flags

```toml
[features]
default = ["cpu", "wgpu"]
cpu = ["rayon"]
wgpu = ["wgpu", "bytemuck"]
cuda = ["cudarc"]  # Optional, requires CUDA toolkit

# Platform-specific VRAM detection
metal = ["metal"]           # macOS
dxgi = ["windows"]          # Windows  
nvml = ["nvml-wrapper"]     # NVIDIA on any platform
```

---

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `VFX_BACKEND` | Force backend: `cpu`, `wgpu`, `cuda` | Auto |
| `VFX_RAM_MAX` | Max RAM usage in bytes | System RAM |
| `VFX_RAM_PCT` | Max RAM as percentage | 80% |
| `VFX_MEM_MB` | Explicit memory limit in MB | - |
| `VFX_TILE_SIZE` | Override tile size | Calculated |
| `VFX_DISABLE_CACHE` | Disable region caching | false |

---

## Testing Strategy

1. **Unit tests** - каждый модуль отдельно
2. **Integration tests** - полный pipeline с разными backends
3. **Memory tests** - проверка лимитов, eviction
4. **Benchmark suite** - сравнение backends на разных размерах
5. **Stress tests** - 32K+ изображения, streaming

---

## Notes

- stool-rs использует `Image` из своего формата, нам нужно адаптировать под наш `ImageBuf`
- wgpu shaders нужно переписать с warp на generic convolution/morpho
- CUDA kernels писать с нуля (можно взять алгоритмы из NPP как референс)
- Все async операции через tokio (уже есть в проекте)
