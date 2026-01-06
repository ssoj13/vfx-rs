# Deep Image Operations

Operations for deep (multi-sample per pixel) images.

## Overview

Deep images store multiple samples per pixel, each with depth and alpha:

```
Regular Image:        Deep Image:
┌─────────────┐       ┌─────────────┐
│  Pixel[0,0] │       │  Samples:   │
│  = RGBA     │       │  @ z=1.0    │
│             │       │  @ z=2.5    │
│             │       │  @ z=3.0    │
└─────────────┘       └─────────────┘
```

## Deep Merge

Merge multiple deep images into one:

```rust
use vfx_io::imagebufalgo::deep_merge;

let fg_deep = vfx_io::read("fg_deep.exr")?;
let bg_deep = vfx_io::read("bg_deep.exr")?;

// Merge deep images
let merged = deep_merge(&fg_deep, &bg_deep)?;
```

## Deep Flatten

Convert deep image to regular image:

```rust
use vfx_io::imagebufalgo::deep_flatten;

let deep = vfx_io::read("render_deep.exr")?;

// Flatten to single sample per pixel
let flat = deep_flatten(&deep)?;
```

## Deep Holdout

Apply holdout (depth-based cutout):

```rust
use vfx_io::imagebufalgo::deep_holdout;

let deep = vfx_io::read("render_deep.exr")?;
let holdout = vfx_io::read("holdout.exr")?;

// Cut out by depth
let held = deep_holdout(&deep, &holdout)?;
```

## Deep Compositing

### Why Deep Compositing?

Regular compositing:
```
FG over BG = incorrect when FG/BG intersect
```

Deep compositing:
```
FG + BG = correct merge at every depth
```

### Example Workflow

```rust
// Load deep renders from multiple sources
let hero_deep = vfx_io::read("hero_deep.exr")?;
let env_deep = vfx_io::read("environment_deep.exr")?;
let fx_deep = vfx_io::read("effects_deep.exr")?;

// Merge all layers (order doesn't matter for deep)
let mut merged = deep_merge(&hero_deep, &env_deep)?;
merged = deep_merge(&merged, &fx_deep)?;

// Flatten for final output
let final_image = deep_flatten(&merged)?;

vfx_io::write("final.exr", &final_image)?;
```

## Deep Sample Operations

### Get Sample Count

```rust
use vfx_io::imagebufalgo::deep_sample_count;

let counts = deep_sample_count(&deep)?;
// counts is 2D array of sample counts per pixel
```

### Trim Samples

```rust
use vfx_io::imagebufalgo::deep_trim;

// Remove samples outside depth range
let trimmed = deep_trim(&deep, 0.1, 100.0)?;
```

## Use Cases

### Volume Rendering

```rust
// Merge volumetric renders
let smoke_deep = vfx_io::read("smoke_deep.exr")?;
let fire_deep = vfx_io::read("fire_deep.exr")?;

let volumes = deep_merge(&smoke_deep, &fire_deep)?;
```

### Character Compositing

```rust
// Characters that intersect in 3D space
let char_a = vfx_io::read("char_a_deep.exr")?;
let char_b = vfx_io::read("char_b_deep.exr")?;

// Correct compositing even where they overlap
let chars = deep_merge(&char_a, &char_b)?;
```

### Set Extension

```rust
// Add CG to plate with correct depth interaction
let plate_deep = vfx_io::read("plate_deep.exr")?;
let cg_extension = vfx_io::read("cg_set_deep.exr")?;

let extended = deep_merge(&plate_deep, &cg_extension)?;
```

## Performance Considerations

Deep images require more memory and processing:

| Operation | Regular | Deep (avg 10 samples) |
|-----------|---------|----------------------|
| Memory | 1x | 10x |
| Merge | N/A | O(n × samples) |
| Flatten | N/A | O(n × samples) |

Optimization tips:
- Trim unnecessary depth ranges before merge
- Use reasonable sample density
- Flatten intermediate results when deep data no longer needed
