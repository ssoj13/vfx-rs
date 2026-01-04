# VFX-COMPUTE Implementation Plan

> Детальный план рефакторинга для выживания context compaction.
> Статус: **Phase 2 DONE, Phase 3 TODO**

## Цель

Превратить `vfx-gpu` в `vfx-compute` — единый execution layer для всего workspace.

## Текущее состояние

### Phase 1 (DONE)
- [x] Renamed vfx-gpu → vfx-compute
- [x] Updated workspace Cargo.toml
- [x] Updated all imports

### Phase 2 (DONE)
- [x] GpuError → ComputeError
- [x] GpuResult → ComputeResult  
- [x] GpuImage → ComputeImage
- [x] Created unified `Processor` struct
- [x] 15 tests passing (5 unit + 10 integration)

### Phase 3 (TODO)
- [ ] Add vfx-compute dependency to vfx-color
- [ ] Update ColorPipeline to use Processor

## Архитектура

```
┌─────────────────────────────────────────────────────────────┐
│                    Layer 2: High-Level APIs                  │
│  vfx-color (ColorPipeline uses vfx-compute)                 │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                Layer 1: vfx-compute (Execution)              │
│  ┌─────────────────┐  ┌─────────────────┐                   │
│  │   CpuBackend    │  │   WgpuBackend   │                   │
│  │  (rayon-based)  │  │  (WGSL shaders) │                   │
│  └─────────────────┘  └─────────────────┘                   │
│                                                              │
│  Operations: matrix, cdl, lut1d, lut3d, resize, blur        │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│              Layer 0: Types & Pure Math                      │
│  vfx-core, vfx-math, vfx-lut, vfx-transfer, vfx-primaries   │
└─────────────────────────────────────────────────────────────┘
```

---

## Phase 1: Переименование (TODO)

### 1.1 Переименовать директорию
```powershell
# В корне workspace
Move-Item crates/vfx-gpu crates/vfx-compute
```

### 1.2 Обновить workspace Cargo.toml
```toml
members = [
    # ... другие
    "crates/vfx-compute",  # было vfx-gpu
]

[workspace.dependencies]
vfx-compute = { path = "crates/vfx-compute" }  # добавить
# vfx-gpu удалить из dependencies если есть
```

### 1.3 Обновить crates/vfx-compute/Cargo.toml
```toml
[package]
name = "vfx-compute"
description = "Unified compute backend for VFX workflows (CPU + GPU)"
```

### 1.4 Обновить все imports во всех crates
```
vfx-gpu -> vfx-compute
```

---

## Phase 2: Реорганизация модулей (TODO)

### 2.1 Текущая структура vfx-gpu
```
src/
├── lib.rs
├── backend/
│   ├── mod.rs          # ProcessingBackend trait
│   ├── cpu_backend.rs  # CpuBackend
│   └── wgpu_backend.rs # WgpuBackend (feature: wgpu)
├── shaders/
│   └── mod.rs          # WGSL shaders
├── image.rs            # GpuImage struct
├── color.rs            # ColorProcessor
└── ops.rs              # ImageProcessor
```

### 2.2 Целевая структура vfx-compute
```
src/
├── lib.rs              # Public API exports
├── error.rs            # GpuError -> ComputeError
├── image.rs            # GpuImage -> ComputeImage
├── backend/
│   ├── mod.rs          # Backend trait, create_backend()
│   ├── cpu.rs          # CpuBackend (rayon)
│   └── wgpu.rs         # WgpuBackend (feature: wgpu)
├── shaders/
│   └── mod.rs          # WGSL (unchanged)
├── ops/
│   ├── mod.rs          # Re-exports
│   ├── color.rs        # apply_matrix, apply_cdl, apply_lut*
│   └── image.rs        # resize, blur, sharpen
└── processor.rs        # Unified Processor (combines color + image)
```

### 2.3 Главный API (processor.rs)
```rust
pub struct Processor {
    backend: Box<dyn ProcessingBackend>,
}

impl Processor {
    pub fn new(backend: Backend) -> ComputeResult<Self>;
    pub fn auto() -> ComputeResult<Self>;  // Auto-select best
    
    // Color operations
    pub fn apply_matrix(&self, img: &mut ComputeImage, matrix: &[f32; 16]) -> ComputeResult<()>;
    pub fn apply_cdl(&self, img: &mut ComputeImage, cdl: &Cdl) -> ComputeResult<()>;
    pub fn apply_lut1d(&self, img: &mut ComputeImage, lut: &[f32]) -> ComputeResult<()>;
    pub fn apply_lut3d(&self, img: &mut ComputeImage, lut: &[f32], size: u32) -> ComputeResult<()>;
    
    // Image operations
    pub fn resize(&self, img: &ComputeImage, w: u32, h: u32, filter: ResizeFilter) -> ComputeResult<ComputeImage>;
    pub fn blur(&self, img: &mut ComputeImage, radius: f32) -> ComputeResult<()>;
    pub fn sharpen(&self, img: &mut ComputeImage, amount: f32) -> ComputeResult<()>;
    
    // Info
    pub fn backend_name(&self) -> &'static str;
    pub fn available_memory(&self) -> u64;
}
```

---

## Phase 3: Интеграция с vfx-color (TODO)

### 3.1 Добавить зависимость в vfx-color/Cargo.toml
```toml
[dependencies]
vfx-compute = { workspace = true }
```

### 3.2 Обновить ColorPipeline
```rust
// vfx-color/src/pipeline.rs
use vfx_compute::{Processor, Backend, ComputeImage};

pub struct ColorPipeline {
    processor: Option<Processor>,  // Lazy init
    // ... existing fields
}

impl ColorPipeline {
    pub fn with_compute(backend: Backend) -> Self {
        Self {
            processor: Some(Processor::new(backend).ok()),
            ..Default::default()
        }
    }
    
    pub fn process_buffer(&self, data: &mut [f32], width: u32, height: u32) {
        if let Some(proc) = &self.processor {
            let mut img = ComputeImage::from_f32_ref(data, width, height, 3);
            // Use processor for operations
        } else {
            // Fallback to pixel-by-pixel (current behavior)
        }
    }
}
```

---

## Phase 4: Deprecate vfx-ops (TODO)

### 4.1 Что в vfx-ops сейчас
- `buffer.rs` - buffer operations (apply_*, convert_*)  
- `parallel.rs` - rayon wrappers
- `convert.rs` - format conversions

### 4.2 План миграции
1. Скопировать полезный код в vfx-compute/src/ops/
2. Добавить deprecation warning в vfx-ops
3. Обновить зависимости других crates
4. В будущем: удалить vfx-ops

---

## Phase 5: Тестирование (TODO)

### 5.1 Unit tests
- [ ] ComputeImage creation/conversion
- [ ] CpuBackend all operations
- [ ] WgpuBackend all operations (if available)
- [ ] Processor unified API

### 5.2 Integration tests
- [ ] Large image processing (tiling)
- [ ] Backend fallback (wgpu -> cpu)
- [ ] vfx-color with vfx-compute backend

### 5.3 Benchmarks
- [ ] CPU vs GPU performance
- [ ] Different image sizes
- [ ] Memory usage

---

## Checklist для каждой фазы

### Phase 1 Checklist
- [ ] `Move-Item crates/vfx-gpu crates/vfx-compute`
- [ ] Update workspace Cargo.toml members
- [ ] Update workspace dependencies  
- [ ] Update package name in crate Cargo.toml
- [ ] `cargo build` passes
- [ ] `cargo test` passes

### Phase 2 Checklist
- [ ] Rename GpuError -> ComputeError
- [ ] Rename GpuImage -> ComputeImage
- [ ] Create unified Processor
- [ ] Update lib.rs exports
- [ ] `cargo build` passes
- [ ] `cargo test` passes

### Phase 3 Checklist
- [ ] Add vfx-compute dependency to vfx-color
- [ ] Add compute backend option to ColorPipeline
- [ ] Test ColorPipeline with GPU backend
- [ ] `cargo build` passes
- [ ] `cargo test` passes

### Phase 4 Checklist
- [ ] Audit vfx-ops code
- [ ] Copy useful code to vfx-compute
- [ ] Add #[deprecated] to vfx-ops
- [ ] Update dependent crates
- [ ] `cargo build` passes

---

## Команды для быстрого старта

```powershell
# Проверить текущее состояние
cd C:\projects\projects.rust\_vfx-rs
cargo build --all-features
cargo test -p vfx-gpu

# После Phase 1
cargo test -p vfx-compute

# Полная проверка
cargo build --workspace --all-features
cargo test --workspace
```

---

## Риски и митигация

| Риск | Митигация |
|------|-----------|
| Breaking changes в API | Добавить re-exports со старыми именами |
| wgpu feature conflicts | Тестировать с и без feature |
| Performance regression | Benchmark до и после |
| vfx-ops зависимости | Проверить все crates перед удалением |

---

## Логирование прогресса

### 2026-01-04
- Создан plan документ
- WgpuBackend готов
- Phase 1 DONE: Renamed vfx-gpu → vfx-compute
- Phase 2 DONE: Renamed types, created Processor
- 15 tests passing

