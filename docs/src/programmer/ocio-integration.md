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

// Apply to pixels (array of RGBA tuples)
let mut pixels: Vec<[f32; 4]> = vec![[0.5, 0.3, 0.2, 1.0]];
processor.apply_rgba(&mut pixels);
```

## Configuration

### Loading Configs

```rust
use vfx_ocio::Config;

// From file
let config = Config::from_file("/path/to/config.ocio")?;

// From YAML string
let config = Config::from_yaml_str(yaml_content, working_dir)?;

// Built-in configs
let aces = builtin::aces_1_3();
let srgb = builtin::srgb_studio();
```

### Built-in Configs

| Config | Description |
|--------|-------------|
| `aces_1_3()` | ACES 1.3 CG config |
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

// Apply to RGB data (array of 3-element arrays)
let mut rgb: Vec<[f32; 3]> = vec![[0.5, 0.3, 0.2]];
processor.apply_rgb(&mut rgb);

// Apply to RGBA data (array of 4-element arrays)
let mut rgba: Vec<[f32; 4]> = vec![[0.5, 0.3, 0.2, 1.0]];
processor.apply_rgba(&mut rgba);

// Multiple pixels
let mut pixels: Vec<[f32; 3]> = vec![
    [0.5, 0.3, 0.2],
    [0.8, 0.1, 0.4],
    [0.2, 0.6, 0.9],
];
processor.apply_rgb(&mut pixels);
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
use vfx_ocio::{DynamicProcessor, DynamicProcessorBuilder};

let processor = config.processor("ACEScg", "sRGB")?;

// Build dynamic processor
let dynamic: DynamicProcessor = DynamicProcessorBuilder::new()
    .exposure(1.5)      // +1.5 stops
    .contrast(1.1)      // 10% more contrast
    .gamma(1.0)         // neutral gamma
    .saturation(1.2)    // 20% more saturation
    .build(processor);  // consumes processor, returns DynamicProcessor

// Apply to pixels
let mut pixels: Vec<[f32; 4]> = vec![[0.5, 0.3, 0.2, 1.0]];
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
let baker = Baker::new(&processor);

// Bake 1D LUT
let lut1d = baker.bake_lut_1d(1024)?;
baker.write_cube_1d("output_1d.cube", &lut1d)?;

// Bake 3D LUT
let lut3d = baker.bake_lut_3d(33)?;
baker.write_cube_3d("output_3d.cube", &lut3d)?;
```

## Processor Cache

For repeated conversions:

```rust
use vfx_ocio::ProcessorCache;

let cache = ProcessorCache::new();

// First call creates processor, subsequent calls return cached
let proc = cache.get_or_create(&config, "ACEScg", "sRGB")?;
let proc2 = cache.get_or_create(&config, "ACEScg", "sRGB")?; // Cache hit

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
| FixedFunctionTransform | Yes | No | ACES gamut mapping, etc. |
| GradingPrimaryTransform | Yes | Yes | Primary color grading |
| GradingToneTransform | Yes | Yes | Tone curve grading |
| GradingRGBCurveTransform | Yes | No | RGB curve grading |
| AllocationTransform | Yes | Yes | GPU texture allocation |
| BuiltinTransform | Yes | No | ACES built-in transforms |

### BuiltinTransform Styles

Supported built-in transform styles (lowercase, no separators):

```rust
// Identity
"identity"

// ACES color space conversions
"acesap0toap1"           // or "aces2065toacescg"
"acesap1toap0"           // or "acescgtoaces2065"
"acesap0toxyzd65bfd"     // or "aces20651toxyzd65"
"acesap1toxyzd65bfd"     // or "acescgtoxyzd65"
"acesccttoaces20651"
"acescctoaces20651"

// Camera to ACES
"arrilogc3toaces20651"   // or "logc3toaces"
"arrilogc4toaces20651"   // or "logc4toaces"
"sonyslog3sgamut3toaces20651" // or "slog3toaces"
"redlog3g10rwgtoaces20651"    // or "log3g10toaces"
"panasonicvlogvgamuttoaces20651" // or "vlogtoaces"
"applelogtoaces20651"
"canonclog2cgamuttoaces20651"
"canonclog3cgamuttoaces20651"

// Display transforms
"displayciexyzd65tosrgb"
"displayciexyzd65todisplayp3"
"displayciexyzd65torec.1886rec.709"
"displayciexyzd65torec.2100pq"
"displayciexyzd65torec.2100hlg1000nit"

// Curves
"curvest2084tolinear"    // or "curvepqtolinear"
"curvelineartost2084"    // or "curvelineartopq"
"curvehlgtolinear"
"curvelineartohlg"
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
