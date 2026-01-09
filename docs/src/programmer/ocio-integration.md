# OCIO Integration

vfx-ocio provides OpenColorIO-compatible color management without requiring the OCIO C++ library.

## Quick Start

```rust
use vfx_ocio::{Config, builtin};

// Use built-in ACES 1.3 config
let config = builtin::aces_1_3();

// Or load from file
let config = Config::from_file("config.ocio")?;

// Create processor for color conversion
let processor = config.processor("ACEScg", "sRGB")?;

// Apply to pixels
let mut pixels = vec![0.5f32, 0.3, 0.2, 1.0]; // RGBA
processor.apply_rgba(&mut pixels);
```

## Configuration

### Loading Configs

```rust
use vfx_ocio::Config;

// From OCIO environment variable
let config = Config::from_env()?;

// From file
let config = Config::from_file("/path/to/config.ocio")?;

// From YAML string
let config = Config::from_string(yaml_content, working_dir)?;

// Built-in configs
let aces = builtin::aces_1_3();
let srgb = builtin::srgb_studio();
```

### Built-in Configs

| Config | Description |
|--------|-------------|
| `aces_1_3()` | ACES 1.3 CG config |
| `aces_1_2()` | ACES 1.2 CG config |
| `srgb_studio()` | Simple sRGB studio config |

## Color Spaces

### Querying Color Spaces

```rust
// List all color spaces
for name in config.colorspace_names() {
    println!("{}", name);
}

// Get specific color space
if let Some(cs) = config.colorspace("ACEScg") {
    println!("Name: {}", cs.name());
    println!("Family: {:?}", cs.family());
    println!("Encoding: {:?}", cs.encoding());
    println!("Aliases: {:?}", cs.aliases());
    println!("Categories: {:?}", cs.categories());
}

// Lookup by alias
let cs = config.colorspace("ACES - ACEScg"); // Works via alias
```

### Categories & Filtering

```rust
// Get all unique categories
let cats = config.all_categories();
println!("Categories: {:?}", cats);

// Filter by category
let scene_spaces = config.colorspaces_by_category("scene_linear");
for cs in scene_spaces {
    println!("{}", cs.name());
}

// Check if colorspace has category
if cs.has_category("file_io") {
    // Good for file I/O operations
}
```

### Roles

```rust
// Standard roles
let scene_linear = config.roles().get("scene_linear");
let reference = config.roles().get("reference");
let compositing_log = config.roles().get("compositing_log");

// Iterate all roles
for (role, colorspace) in config.roles().iter() {
    println!("{} -> {}", role, colorspace);
}
```

## Processors

### Basic Usage

```rust
// Color space conversion
let processor = config.processor("ACEScg", "sRGB")?;

// Apply to RGB data
let mut rgb = vec![0.5f32, 0.3, 0.2];
processor.apply_rgb(&mut rgb);

// Apply to RGBA data
let mut rgba = vec![0.5f32, 0.3, 0.2, 1.0];
processor.apply_rgba(&mut rgba);

// Batch processing (more efficient)
let mut pixels: Vec<[f32; 3]> = load_image();
processor.apply_rgb_batch(&mut pixels);
```

### Display Processing

```rust
// Display view transform
let display_proc = config.display_processor(
    "ACEScg",      // src colorspace
    "sRGB",        // display
    "ACES 1.0 SDR" // view
)?;

display_proc.apply_rgba(&mut pixels);
```

### With Looks

```rust
// Apply look
let look_proc = config.processor_with_looks(
    "ACEScg",
    "sRGB",
    "film_look"
)?;
```

### Context Variables

```rust
use vfx_ocio::Context;

let mut ctx = Context::new();
ctx.set("SHOT", "sh010");
ctx.set("SEQ", "sq01");

let processor = config.processor_with_context("src", "dst", &ctx)?;
```

## Dynamic Properties

For real-time grading adjustments:

```rust
use vfx_ocio::DynamicProcessorBuilder;

let processor = config.processor("ACEScg", "sRGB")?;

let dynamic = DynamicProcessorBuilder::new()
    .exposure(1.5)      // +1.5 stops
    .contrast(1.1)      // 10% more contrast
    .gamma(1.0)         // neutral gamma
    .saturation(1.2)    // 20% more saturation
    .build(&processor)?;

dynamic.apply_rgba(&mut pixels);

// Adjust in real-time
dynamic.set_exposure(2.0);
dynamic.apply_rgba(&mut pixels);
```

## Baker (LUT Export)

Export color transforms as LUT files:

```rust
use vfx_ocio::Baker;

let processor = config.processor("ACEScg", "sRGB")?;

// 1D LUT
let baker = Baker::new(&processor);
let lut1d = baker.bake_1d(1024)?;
lut1d.write_cube("output.cube")?;

// 3D LUT
let lut3d = baker.bake_3d(33)?;
lut3d.write_cube("output_3d.cube")?;
```

## Processor Cache

For repeated conversions:

```rust
use vfx_ocio::ProcessorCache;

let cache = ProcessorCache::new(config);

// First call creates processor, subsequent calls return cached
let proc = cache.get("ACEScg", "sRGB")?;
let proc2 = cache.get("ACEScg", "sRGB")?; // Cache hit

cache.clear(); // Clear all cached processors
```

## Transforms

### Supported Transform Types

| Transform | CPU | GPU | Description |
|-----------|-----|-----|-------------|
| MatrixTransform | Yes | Yes | 4x4 matrix with offset |
| CDLTransform | Yes | Yes | ASC CDL (slope/offset/power/sat) |
| ExponentTransform | Yes | Yes | Per-channel gamma |
| ExponentWithLinear | Yes | Yes | sRGB-style curve |
| LogTransform | Yes | Yes | Log base conversion |
| LogAffineTransform | Yes | Yes | Camera log encoding |
| LogCameraTransform | Yes | Yes | LogC3, LogC4, S-Log3, etc. |
| RangeTransform | Yes | Yes | Clamp/scale values |
| Lut1DTransform | Yes | Yes | 1D LUT application |
| Lut3DTransform | Yes | Yes | 3D LUT application |
| FileTransform | Yes | Partial | Load external LUT files |
| GroupTransform | Yes | Yes | Sequential transforms |
| ColorSpaceTransform | Yes | Yes | Named conversion |
| DisplayViewTransform | Yes | Yes | Display output |
| LookTransform | Yes | Yes | Creative look |
| ExposureContrastTransform | Yes | Yes | Dynamic exposure/contrast |
| FixedFunctionTransform | Yes | Partial | ACES gamut mapping, etc. |
| GradingPrimaryTransform | Yes | Yes | Primary color grading |
| GradingToneTransform | Yes | Yes | Tone curve grading |
| GradingRGBCurveTransform | Yes | Partial | RGB curve grading |
| AllocationTransform | Yes | Yes | GPU texture allocation |
| BuiltinTransform | Yes | No | ACES built-in transforms |

### BuiltinTransform Styles

Supported built-in transform styles:

```rust
// Identity
"IDENTITY"

// ACES color space conversions
"ACES-AP0_to_AP1"
"ACES-AP1_to_AP0"
"ACES-AP0_to_XYZ-D65"
"ACES-AP1_to_XYZ-D65"
"ACEScct_to_ACES2065-1"
"ACEScc_to_ACES2065-1"

// Camera to ACES
"ARRI_LOGC3_to_ACES2065-1"
"ARRI_LOGC4_to_ACES2065-1"
"SONY_SLOG3_SGAMUT3_to_ACES2065-1"
"RED_LOG3G10_RWG_to_ACES2065-1"
"PANASONIC_VLOG_VGAMUT_to_ACES2065-1"
```

## Validation

```rust
use vfx_ocio::validate;

let issues = validate::check(&config);

for issue in &issues {
    println!("[{:?}] {}", issue.severity, issue.message);
}

if issues.has_errors() {
    return Err("Config validation failed");
}
```

## Python Bindings

### Basic Usage

```python
from vfx import ColorConfig, colorconvert, ociodisplay, ociolook

# Load config
config = ColorConfig.from_file("config.ocio")
# or use built-in
config = ColorConfig.aces_1_3()

# Query color spaces
print(config.colorspace_names())
print(config.display_names())

# Role shortcuts
print(config.scene_linear())       # -> "ACEScg"
print(config.reference())          # -> "ACES2065-1"
print(config.compositing_log())    # -> "ACEScct"
print(config.color_timing())       # -> "ACEScct"
print(config.data_role())          # -> "Raw"

# Aliases and categories
print(config.colorspace_aliases("ACEScg"))  # -> ["ACES - ACEScg", ...]
print(config.colorspace_categories("ACEScg"))  # -> ["scene_linear", ...]
print(config.colorspaces_by_category("file_io"))

# Active displays/views
print(config.active_displays())
print(config.active_views())
print(config.viewing_rules())

# Serialize/save
yaml_str = config.serialize()
config.write_to_file("output_config.ocio")

# Convert image
from vfx import Image
img = Image.read("input.exr")
result = colorconvert(img, "ACEScg", "sRGB", config)
result.write("output.png")
```

### Context Variables

```python
from vfx import Context, ColorConfig

# Create context with variables
ctx = Context()
ctx.set("SHOT", "sh010")
ctx.set("SEQ", "sq01")

# Resolve paths
resolved = ctx.resolve("/shows/$SEQ/shots/$SHOT/luts/grade.csp")
print(resolved)  # /shows/sq01/shots/sh010/luts/grade.csp

# Check variables
print(ctx.get("SHOT"))  # sh010
print(ctx.vars())  # {"SHOT": "sh010", "SEQ": "sq01"}
print(ctx.has_unresolved("$UNKNOWN"))  # True

# Create processor with context
config = ColorConfig.aces_1_3()
config.processor_with_context("ACEScg", "sRGB", ctx)
```

### GPU Processing

```python
from vfx import ColorConfig, GpuProcessor, GpuLanguage

config = ColorConfig.aces_1_3()

# Create GPU processor
gpu_proc = GpuProcessor.from_config(config, "ACEScg", "sRGB")

# Generate shader code
shader = gpu_proc.generate_shader(GpuLanguage.Glsl330)
print(shader.fragment_code)
print(shader.has_textures())

# Check GPU compatibility
print(gpu_proc.is_complete())  # True if all ops are GPU-compatible
print(gpu_proc.num_ops())
```

### Advanced Classes

```python
from vfx import (
    ConfigBuilder, Baker, DynamicProcessor, 
    ProcessorCache, OptimizationLevel
)

# Build config programmatically
builder = ConfigBuilder("My Config")
builder.add_colorspace("linear", family="scene", encoding="scene_linear")
builder.add_colorspace("sRGB", family="display", encoding="sdr_video")
builder.set_role("scene_linear", "linear")
config = builder.build()

# Bake LUTs
baker = Baker(config, "ACEScg", "sRGB")
baker.bake_cube_1d("output_1d.cube", 4096)
baker.bake_cube_3d("output_3d.cube", 65)

# Dynamic grading
proc = DynamicProcessor(config, "ACEScg", "sRGB")
proc.exposure = 1.5  # +1.5 stops
proc.contrast = 1.2
proc.apply_rgb(pixels)  # numpy array

# Processor cache
cache = ProcessorCache()
proc = cache.get(config, "ACEScg", "sRGB")  # Compiles
proc = cache.get(config, "ACEScg", "sRGB")  # Cache hit

# Validation
from vfx import validate_config, config_has_errors
issues = validate_config(config)
for issue in issues:
    print(issue)
```

## Parity with OpenColorIO C++

| Feature | OCIO C++ | vfx-ocio | Python | Status |
|---------|----------|----------|--------|--------|
| Config loading | Yes | Yes | Yes | 100% |
| Color spaces | Yes | Yes | Yes | 100% |
| Aliases | Yes | Yes | Yes | 100% |
| Categories | Yes | Yes | Yes | 100% |
| Roles | Yes | Yes | Yes | 100% |
| Displays/Views | Yes | Yes | Yes | 100% |
| Looks | Yes | Yes | Yes | 100% |
| Context variables | Yes | Yes | Yes | 100% |
| All transforms | Yes | Yes | via ops | 98% |
| GPU processing | Yes | Yes | Yes | 90% |
| Dynamic properties | Yes | Yes | Yes | 100% |
| Baker | Yes | Yes | Yes | 100% |
| Processor cache | Yes | Yes | Yes | 100% |
| Serialize/write | Yes | Yes | Yes | 100% |
| OptimizationLevel | Yes | Yes | Yes | 100% |

**Not implemented:**
- OCIO_LOGGING_LEVEL
- Config archiving
- Some GPU LUT texture features
