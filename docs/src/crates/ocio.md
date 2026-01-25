# vfx-ocio

OpenColorIO-compatible color management for VFX.

## Purpose

Native Rust implementation of OCIO functionality. Parses `.ocio` config files and provides color space transformations compatible with the OCIO standard.

## Quick Start

```rust
use vfx_ocio::{Config, builtin};

// Use built-in ACES 1.3 config
let config = builtin::aces_1_3();

// Look up color spaces
let acescg = config.colorspace("ACEScg").unwrap();
println!("Working space: {}", acescg.name());

// Create processor
let processor = config.processor("ACEScg", "sRGB").unwrap();

// Apply to pixels
let mut pixels = [[0.18f32, 0.18, 0.18]];
processor.apply_rgb(&mut pixels);
```

## Loading Configs

### From File

```rust
use vfx_ocio::Config;

let config = Config::from_file("path/to/config.ocio")?;

// List color spaces
for cs in config.colorspaces() {
    println!("{}: {:?}", cs.name(), cs.encoding());
}
```

### From String

```rust
use std::path::PathBuf;

let yaml = std::fs::read_to_string("config.ocio")?;
let working_dir = PathBuf::from("path/to/config/dir");
let config = Config::from_yaml_str(&yaml, working_dir)?;
```

### Built-in Configs

```rust
use vfx_ocio::builtin;

let aces = builtin::aces_1_3();     // ACES 1.3
let srgb = builtin::srgb_linear();  // Simple sRGB/Linear
```

## Color Spaces

### Lookup

```rust
// By name
let cs = config.colorspace("ACEScg")?;

// By role (semantic alias)
let linear = config.colorspace("scene_linear")?;  // Resolves to ACEScg

// List all
for cs in config.colorspaces() {
    println!("{}", cs.name());
}
```

### Properties

```rust
let cs = config.colorspace("ACEScg")?;

println!("Name: {}", cs.name());
println!("Family: {:?}", cs.family());
println!("Encoding: {:?}", cs.encoding());
println!("Bit depth: {:?}", cs.bit_depth());
```

## Roles

Semantic names for color spaces:

```rust
use vfx_ocio::role_names;

// Standard roles
let scene_linear = config.colorspace(role_names::SCENE_LINEAR)?;
let compositing_log = config.colorspace(role_names::COMPOSITING_LOG)?;
let color_picking = config.colorspace(role_names::COLOR_PICKING)?;
let texture_paint = config.colorspace(role_names::TEXTURE_PAINT)?;
let matte_paint = config.colorspace(role_names::MATTE_PAINT)?;
```

## Processors

### Create Processor

```rust
// Color space to color space
let proc = config.processor("ACEScg", "sRGB")?;

// With display/view
let proc = config.display_processor("ACEScg", "sRGB", "Film")?;
```

### Apply Transform

```rust
// Single pixel (inline)
let mut pixels = [[0.18f32, 0.18, 0.18]];
proc.apply_rgb(&mut pixels);

// Buffer
proc.apply_rgb(&mut pixel_buffer);

// With optimization level
use vfx_ocio::OptimizationLevel;
let proc = config.processor_with_opts("ACEScg", "sRGB", OptimizationLevel::Good)?;
```

## Transforms

Building custom transform chains:

```rust
use vfx_ocio::{
    Transform, TransformDirection, Processor,
    MatrixTransform, CdlTransform, FileTransform,
};

// CDL transform
let cdl = Transform::Cdl(CdlTransform {
    slope: [1.1, 1.0, 0.9],
    offset: [0.0, 0.0, 0.0],
    power: [1.0, 1.0, 1.0],
    saturation: 1.0,
    ..Default::default()
});

let proc = Processor::from_transform(&cdl, TransformDirection::Forward)?;
proc.apply_rgb(&mut pixels);
```

### Transform Types

| Type | Description |
|------|-------------|
| `MatrixTransform` | 3x3 or 4x4 matrix |
| `CdlTransform` | ASC CDL (slope/offset/power/sat) |
| `ExponentTransform` | Power function |
| `LogTransform` | Log/antilog |
| `FileTransform` | External LUT file |
| `RangeTransform` | Clamp/scale range |
| `GroupTransform` | Chain of transforms |
| `ColorSpaceTransform` | Convert between spaces |
| `BuiltinTransform` | ACES, camera IDTs |
| `FixedFunctionTransform` | Tonemaps, gamut compress |

## Displays and Views

```rust
// List displays
for display in config.displays() {
    println!("Display: {}", display.name());
    for view in display.views() {
        println!("  View: {}", view.name());
    }
}

// Create display processor
let proc = config.display_processor(
    "ACEScg",      // Input space
    "sRGB",        // Display
    "ACES 1.0 SDR" // View
)?;
```

## Looks

Color grades applied during display:

```rust
// List looks
for look in config.looks() {
    println!("Look: {}", look.name());
}

// Apply look
let proc = config.processor_with_looks("ACEScg", "sRGB", "ShowLUT")?;
```

## Named Transforms (OCIO v2.0+)

Reusable transform definitions not tied to a specific color space:

```rust
// List named transforms
for nt in config.named_transforms() {
    println!("Named transform: {}", nt.name);
    if let Some(family) = &nt.family {
        println!("  Family: {}", family);
    }
    if let Some(desc) = &nt.description {
        println!("  Description: {}", desc);
    }
}

// Get by name
if let Some(nt) = config.named_transform("Utility - sRGB - Texture") {
    println!("Found: {}", nt.name);
}

// Count
println!("Total named transforms: {}", config.num_named_transforms());
```

## Shared Views (OCIO v2.3+)

Views that can be shared across multiple displays:

```rust
// List shared views
for sv in config.shared_views() {
    println!("Shared view: {}", sv.name);
    println!("  Colorspace: {}", sv.display_colorspace);
    if let Some(vt) = &sv.view_transform {
        println!("  View transform: {}", vt);
    }
}
```

## Context Variables

Environment variable substitution:

```rust
use vfx_ocio::Context;

let mut ctx = Context::new();
ctx.set("SHOT", "sh010");
ctx.set("SEQ", "sq01");

let resolved = ctx.resolve("/shows/$SEQ/$SHOT/luts/grade.cube");
// → "/shows/sq01/sh010/luts/grade.cube"
```

## Config Validation

Check config for issues:

```rust
use vfx_ocio::{validate_config, Severity};

let issues = validate_config(&config);

for issue in &issues {
    match issue.severity {
        Severity::Error => eprintln!("ERROR: {}", issue.message),
        Severity::Warning => eprintln!("WARN: {}", issue.message),
        Severity::Info => println!("INFO: {}", issue.message),
    }
}

if vfx_ocio::has_errors(&issues) {
    panic!("Config has errors");
}
```

## File Rules

Automatic color space assignment:

```rust
use vfx_ocio::FileRuleKind;

// Iterate file rules
for rule in config.file_rules() {
    println!("Rule: {} → colorspace: {}", rule.name, rule.colorspace);
    match &rule.kind {
        FileRuleKind::Basic { pattern, .. } => println!("  Pattern: {}", pattern),
        FileRuleKind::Regex { regex } => println!("  Regex: {}", regex),
        FileRuleKind::Default => println!("  (default rule)"),
    }
}

// Get space for filename (returns Option)
if let Some(space) = config.colorspace_from_filepath("texture_diffuse.exr") {
    println!("Colorspace: {}", space);
}
```

## ACES 1.3 Built-in

The built-in ACES 1.3 config includes:

**Color Spaces:**
- ACES 2065-1 (AP0 linear)
- ACEScg (AP1 linear)
- ACEScct, ACEScc (log grading)
- Utility spaces (Raw, sRGB, Rec.709, etc.)

**Displays:**
- sRGB, Rec.709, P3-D65, Rec.2100-PQ

**Views:**
- ACES 1.0 SDR, HDR
- Un-tone-mapped

## Compatibility

### vs OpenColorIO

| Feature | vfx-ocio | OCIO |
|---------|----------|------|
| Config parsing | Yes | Yes |
| Basic transforms | Yes | Yes |
| CPU processing | Yes | Yes |
| GPU processing | Via vfx-compute | Yes |
| Python bindings | Via vfx-rs-py | Yes |
| Nuke/Mari plugins | No | Yes |

### Algorithm Parity

Numerical accuracy verified against OpenColorIO 2.5.1:

| Component | Max Diff | Notes |
|-----------|----------|-------|
| LUT3D Index | 0 | Blue-major order |
| LUT3D Tetrahedral | 1.19e-07 | All 6 tetrahedra match |
| LUT3D Trilinear | 0.0 | Perfect match |
| CDL (power=1) | 0.0 | Bit-perfect |
| CDL (power≠1) | 2.98e-07 | OCIO-identical `fast_pow` |
| sRGB | 2.41e-05 | f32 precision |
| PQ | 2.74e-06 | Relative error |
| HLG | 6.66e-16 | Machine epsilon |

**Key implementation details:**
- `fast_pow` uses identical Chebyshev polynomial coefficients as OCIO SSE.h
- LUT3D uses Blue-major indexing: `idx = B + dim*G + dim²*R`
- CDL saturation uses OCIO-compatible operation order

See implementation source for full parity details.

### Config Version

```rust
use vfx_ocio::ConfigVersion;

match config.version() {
    ConfigVersion::V1 => println!("OCIO v1 config"),
    ConfigVersion::V2 => println!("OCIO v2 config"),
}
```

## Dependencies

- `vfx-core`, `vfx-math`, `vfx-lut`, `vfx-transfer`, `vfx-primaries`
- `saphyr` - YAML parsing
- `glob`, `regex` - File matching
