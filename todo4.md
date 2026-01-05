# VFX-RS: План исправлений

> Минимальные изменения, максимум переиспользования.

---

## 1. CRITICAL: Chromatic Adaptation

**Файл:** `vfx-color/src/convert.rs`

**Проблема:** `convert_rgb()` не вызывает `.adapt()` когда white points разные.

**Уже есть:**
- `RgbConvert::adapt()` - метод в том же файле
- `adapt_matrix(BRADFORD, from, to)` - в vfx-math
- `D65`, `D60`, `BRADFORD` - константы в vfx-math
- `Primaries.w` - white point в каждом color space

**Исправление** (добавить ~5 строк):

```rust
// convert.rs:208, в convert_rgb()

pub fn convert_rgb(
    rgb: [f32; 3],
    decode: Option<fn(f32) -> f32>,
    from: &Primaries,
    to: &Primaries,
    encode: Option<fn(f32) -> f32>,
) -> [f32; 3] {
    let mut result = match decode {
        Some(f) => rgb.linearize(f),
        None => rgb,
    };

    if from != to {
        result = result.to_xyz(from);
        
        // NEW: chromatic adaptation if white points differ
        if from.w != to.w {
            let src_w = Vec3::new(from.w.0 / from.w.1, 1.0, (1.0 - from.w.0 - from.w.1) / from.w.1);
            let dst_w = Vec3::new(to.w.0 / to.w.1, 1.0, (1.0 - to.w.0 - to.w.1) / to.w.1);
            result = result.adapt(BRADFORD, src_w, dst_w);
        }
        
        result = result.from_xyz(to);
    }

    match encode {
        Some(f) => result.encode(f),
        None => result,
    }
}
```

**Тест:**
```rust
#[test]
fn test_srgb_to_acescg_adaptation() {
    use vfx_primaries::{SRGB, ACES_AP1};
    
    // D65 white in sRGB -> should be D60 white in ACEScg
    let white = convert_rgb([1.0, 1.0, 1.0], None, &SRGB, &ACES_AP1, None);
    
    // После адаптации белый должен остаться белым (примерно 1,1,1)
    assert!((white[0] - 1.0).abs() < 0.02);
    assert!((white[1] - 1.0).abs() < 0.02);
    assert!((white[2] - 1.0).abs() < 0.02);
}
```

---

## 2. HIGH: Tiled Processing

**Файл:** `vfx-compute/src/pipeline.rs`

**Проблема:** `run_tiled()` загружает всё в память.

**Решение:** Добавить параметр `tile_fn` для lazy loading.

```rust
// Вместо полной переработки - добавить callback для чтения тайлов

pub fn run_tiled<R, W>(
    &self,
    read_tile: R,   // fn(x, y, w, h) -> Vec<f32>
    write_tile: W,  // fn(x, y, w, h, data)
    width: usize,
    height: usize,
    tile_size: usize,
) -> Result<()>
where
    R: Fn(usize, usize, usize, usize) -> Result<Vec<f32>>,
    W: Fn(usize, usize, usize, usize, &[f32]) -> Result<()>,
{
    let overlap = self.required_overlap();
    
    for ty in (0..height).step_by(tile_size) {
        for tx in (0..width).step_by(tile_size) {
            let tw = tile_size.min(width - tx);
            let th = tile_size.min(height - ty);
            
            // Read with overlap
            let x0 = tx.saturating_sub(overlap);
            let y0 = ty.saturating_sub(overlap);
            let x1 = (tx + tw + overlap).min(width);
            let y1 = (ty + th + overlap).min(height);
            
            let mut data = read_tile(x0, y0, x1 - x0, y1 - y0)?;
            
            // Apply pipeline
            for stage in &self.stages {
                data = stage.apply(&data, x1 - x0, y1 - y0, self.channels)?;
            }
            
            // Trim overlap and write
            let trimmed = trim_overlap(&data, x1 - x0, y1 - y0, 
                                       tx - x0, ty - y0, tw, th, self.channels);
            write_tile(tx, ty, tw, th, &trimmed)?;
        }
    }
    Ok(())
}

fn required_overlap(&self) -> usize {
    self.stages.iter().map(|s| s.overlap()).max().unwrap_or(0)
}
```

---

## 3. HIGH: GPU Batching

**Файл:** `vfx-compute/src/gpu/`

**Проблема:** Каждая операция upload/download.

**Решение:** Добавить `keep_on_gpu: bool` параметр к существующим функциям.

```rust
// Вместо нового типа - расширить существующий API

impl GpuOp {
    pub fn run(&self, data: &[f32], w: u32, h: u32) -> Result<Vec<f32>> {
        self.run_opts(data, w, h, false)  // default: download
    }
    
    pub fn run_opts(&self, data: &[f32], w: u32, h: u32, keep_on_gpu: bool) -> Result<Vec<f32>> {
        // ... existing upload code ...
        
        if keep_on_gpu {
            self.last_buffer = Some(output_buffer);
            Ok(vec![])  // empty - data on GPU
        } else {
            // existing download code
        }
    }
    
    pub fn chain(&self, next: &GpuOp) -> Result<Vec<f32>> {
        // Use self.last_buffer as input for next
        let input = self.last_buffer.take().ok_or("No GPU buffer")?;
        next.run_from_buffer(input)
    }
}
```

---

## 4. MEDIUM: OCIO Auto-Invert

**Файл:** `vfx-ocio/src/config.rs`

**Проблема:** Нет fallback на инверсию `to_reference`.

**Решение:** Добавить `invert()` метод к существующему `Transform` enum.

```rust
// vfx-ocio/src/transform.rs - добавить метод

impl Transform {
    pub fn invert(&self) -> Option<Transform> {
        match self {
            Transform::Matrix { matrix, offset } => {
                let inv = invert_4x4(matrix)?;
                let inv_off = [-offset[0], -offset[1], -offset[2], -offset[3]];
                Some(Transform::Matrix { matrix: inv, offset: inv_off })
            }
            Transform::Exponent { value } => {
                if value.iter().all(|&v| v != 0.0) {
                    Some(Transform::Exponent { 
                        value: [1.0/value[0], 1.0/value[1], 1.0/value[2], 1.0/value[3]]
                    })
                } else { None }
            }
            Transform::Log { base, direction } => {
                Some(Transform::Log { base: *base, direction: direction.invert() })
            }
            Transform::Group { transforms } => {
                let inv: Option<Vec<_>> = transforms.iter().rev()
                    .map(|t| t.invert()).collect();
                inv.map(|t| Transform::Group { transforms: t })
            }
            _ => None,
        }
    }
}

// config.rs - использовать в build_transforms()
if let Some(t) = dst_cs.from_reference() {
    transforms.push(t.clone());
} else if let Some(t) = dst_cs.to_reference() {
    if let Some(inv) = t.invert() {
        transforms.push(inv);
    }
}
```

---

## 5. MEDIUM: DPX Packing

**Файл:** `vfx-io/src/dpx.rs`

**Проблема:** Только Method A.

**Решение:** Добавить параметр `method` к существующей функции.

```rust
// dpx.rs - расширить read_10bit

fn read_10bit(&mut self, method: u8) -> Result<Vec<u16>> {
    match method {
        0 => self.read_10bit_a(),  // existing code
        1 => self.read_10bit_b(),  // new
        _ => self.read_10bit_a(),  // fallback
    }
}

fn read_10bit_b(&mut self) -> Result<Vec<u16>> {
    // Bit-stream unpacking
    let mut result = Vec::new();
    let mut bits: u32 = 0;
    let mut n = 0;
    
    while result.len() < self.pixel_count {
        while n < 10 {
            bits = (bits << 8) | self.read_u8()? as u32;
            n += 8;
        }
        n -= 10;
        result.push(((bits >> n) & 0x3FF) as u16);
    }
    Ok(result)
}
```

---

## 6. MEDIUM: FixedFunction

**Файл:** `vfx-ocio/src/processor.rs`

**Проблема:** Много стилей не реализовано.

**Решение:** Добавить функции в существующий match.

```rust
// processor.rs - в apply_fixed_function()

FixedFunctionStyle::AcesRedMod03 => {
    let (h, s, _) = rgb_to_hsv(rgb[0], rgb[1], rgb[2]);
    let w = smooth_step(h, 0.0, 0.375);  // red region
    let f = 1.0 - 0.2 * w * s;
    let l = 0.2126 * rgb[0] + 0.7152 * rgb[1] + 0.0722 * rgb[2];
    [l + (rgb[0] - l) * f, l + (rgb[1] - l) * f, l + (rgb[2] - l) * f]
}

FixedFunctionStyle::AcesGlow03 => {
    let y = 0.2126 * rgb[0] + 0.7152 * rgb[1] + 0.0722 * rgb[2];
    let g = 0.05 * (1.0 / (1.0 + (-50.0 * (y - 0.08)).exp()));
    [rgb[0] + g, rgb[1] + g, rgb[2] + g]
}

// Вспомогательные - добавить рядом
fn smooth_step(x: f32, a: f32, b: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
```

---

## 7. LOW: Attrs Consolidation

**Файлы:** `vfx-io/src/metadata.rs`, `vfx-io/src/attrs/`

**Решение:** Удалить metadata.rs, alias в lib.rs.

```rust
// lib.rs
pub use attrs::{Attrs, AttrValue};

// Для совместимости (deprecated)
#[deprecated(note = "use attrs::AttrValue")]
pub mod metadata {
    pub use super::attrs::AttrValue;
}
```

---

## Порядок

1. **Chromatic adaptation** - 1 день, критично
2. **OCIO invert** - 1 день  
3. **FixedFunction** - 1 день
4. **DPX packing** - полдня
5. **Tiled processing** - 2 дня
6. **GPU batching** - 2 дня
7. **Attrs cleanup** - полдня

Итого: ~8 рабочих дней, минимум новых сущностей.
