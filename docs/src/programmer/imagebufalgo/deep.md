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

## Reading Deep Data

```rust
use vfx_io;

// Read deep EXR file
let deep = vfx_io::read_deep("render_deep.exr")?;
```

## Deep Merge

Merge multiple deep images into one:

```rust
use vfx_io::imagebufalgo::deep_merge;

let fg_deep = vfx_io::read_deep("fg_deep.exr")?;
let bg_deep = vfx_io::read_deep("bg_deep.exr")?;

// Merge deep images (returns DeepData)
let merged = deep_merge(&fg_deep, &bg_deep);
```

## Deep Flatten

Convert deep image to regular image:

```rust
use vfx_io::imagebufalgo::flatten_deep;

let deep = vfx_io::read_deep("render_deep.exr")?;

// Flatten to single sample per pixel (needs width and height)
let width = 1920;
let height = 1080;
let flat = flatten_deep(&deep, width, height);
```

## Deep Holdout

Apply holdout (depth-based cutout):

```rust
use vfx_io::imagebufalgo::{deep_holdout, deep_holdout_matte};

let mut deep = vfx_io::read_deep("render_deep.exr")?;

// Cut out samples beyond a z depth (modifies in place)
deep_holdout(&deep, 10.0);

// Or use a matte deep image for holdout
let holdout = vfx_io::read_deep("holdout_deep.exr")?;
deep_holdout_matte(&deep, &holdout);
```

## Convert Regular to Deep

```rust
use vfx_io::imagebufalgo::deepen;
use vfx_io::ImageBuf;

let image = ImageBuf::from_file("image.exr")?;

// Convert to deep with constant Z value
let deep = deepen(&image, 5.0);
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
use vfx_io::imagebufalgo::{deep_merge, flatten_deep};

// Load deep renders from multiple sources
let hero_deep = vfx_io::read_deep("hero_deep.exr")?;
let env_deep = vfx_io::read_deep("environment_deep.exr")?;
let fx_deep = vfx_io::read_deep("effects_deep.exr")?;

// Merge all layers (order doesn't matter for deep)
let merged = deep_merge(&hero_deep, &env_deep);
let merged = deep_merge(&merged, &fx_deep);

// Flatten for final output
let final_image = flatten_deep(&merged, 1920, 1080);

final_image.write("final.exr")?;
```

## Deep Statistics

```rust
use vfx_io::imagebufalgo::deep_stats;

let deep = vfx_io::read_deep("render_deep.exr")?;
let stats = deep_stats(&deep);

println!("Total samples: {}", stats.total_samples);
println!("Max samples per pixel: {}", stats.max_samples);
println!("Z range: {} - {}", stats.min_z, stats.max_z);
```

## Deep Tidy

Clean up deep data (merge overlapping samples):

```rust
use vfx_io::imagebufalgo::deep_tidy;

let mut deep = vfx_io::read_deep("noisy_deep.exr")?;

// Clean up overlapping samples
deep_tidy(&deep);
```

## Use Cases

### Volume Rendering

```rust
// Merge volumetric renders
let smoke_deep = vfx_io::read_deep("smoke_deep.exr")?;
let fire_deep = vfx_io::read_deep("fire_deep.exr")?;

let volumes = deep_merge(&smoke_deep, &fire_deep);
```

### Character Compositing

```rust
// Characters that intersect in 3D space
let char_a = vfx_io::read_deep("char_a_deep.exr")?;
let char_b = vfx_io::read_deep("char_b_deep.exr")?;

// Correct compositing even where they overlap
let chars = deep_merge(&char_a, &char_b);
```

### Set Extension

```rust
// Add CG to plate with correct depth interaction
let plate_deep = vfx_io::read_deep("plate_deep.exr")?;
let cg_extension = vfx_io::read_deep("cg_set_deep.exr")?;

let extended = deep_merge(&plate_deep, &cg_extension);
```

## Performance Considerations

Deep images require more memory and processing:

| Operation | Regular | Deep (avg 10 samples) |
|-----------|---------|----------------------|
| Memory | 1x | 10x |
| Merge | N/A | O(n × samples) |
| Flatten | N/A | O(n × samples) |

Optimization tips:
- Use holdout to remove unnecessary depth ranges before merge
- Use reasonable sample density
- Flatten intermediate results when deep data no longer needed
