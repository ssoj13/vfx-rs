# vfx-icc

ICC color profile support for VFX pipelines.

## Purpose

High-level interface for ICC color profiles, built on the industry-standard Little CMS 2 (lcms2) library. Handles camera profiles, display calibration, and print workflows.

## Quick Start

```rust
use vfx_icc::{Profile, Transform, Intent};
use std::path::Path;

// Load camera profile
let camera = Profile::from_file(Path::new("camera.icc"))?;

// Built-in working space
let aces = Profile::aces_ap0();

// Create transform
let transform = Transform::new(&camera, &aces, Intent::Perceptual)?;

// Transform pixels
let mut pixels = vec![[0.5f32, 0.3, 0.2]; 100];
transform.apply(&mut pixels);
```

## Profiles

### Loading Profiles

```rust
use vfx_icc::Profile;
use std::path::Path;

// From file
let profile = Profile::from_file(Path::new("monitor.icc"))?;

// From embedded data (e.g., from image file)
let profile = Profile::from_icc(&icc_data)?;
```

### Built-in Profiles

```rust
use vfx_icc::Profile;

// Standard spaces
let srgb = Profile::srgb();
let adobe_rgb = Profile::adobe_rgb();
let display_p3 = Profile::display_p3();
let rec709 = Profile::rec709();
let rec2020 = Profile::rec2020();

// ACES
let aces_ap0 = Profile::aces_ap0();    // ACES 2065-1 (linear)
let aces_ap1 = Profile::aces_ap1();    // ACEScg (linear)

// Lab/XYZ
let lab = Profile::lab()?;
let xyz = Profile::xyz();
```

### Creating Profiles

```rust
use vfx_icc::{Profile, StandardProfile};

// From standard definition (returns Profile directly, not Result)
let profile = Profile::from_standard(StandardProfile::Srgb);

// Custom RGB profile (requires primaries + TRC)
// See lcms2 documentation for advanced usage
```

## Transforms

### Basic Transform

```rust
use vfx_icc::{Transform, Intent};

let transform = Transform::new(&input, &output, Intent::Perceptual)?;

// Apply to RGB pixels
let mut pixels = vec![[0.5f32, 0.3, 0.2]; 1000];
transform.apply(&mut pixels);
```

### Rendering Intents

```rust
use vfx_icc::Intent;

// Perceptual - Best for photos, compresses gamut
let t1 = Transform::new(&src, &dst, Intent::Perceptual)?;

// Relative Colorimetric - Accurate in-gamut, clips out-of-gamut
let t2 = Transform::new(&src, &dst, Intent::RelativeColorimetric)?;

// Saturation - Vivid colors, for business graphics
let t3 = Transform::new(&src, &dst, Intent::Saturation)?;

// Absolute Colorimetric - Exact match including white point
let t4 = Transform::new(&src, &dst, Intent::AbsoluteColorimetric)?;
```

### Thread Safety

Transforms with caching are not thread-safe. For multi-threaded use:

```rust
// Create uncached transform (thread-safe)
let transform = Transform::new_uncached(&src, &dst, Intent::Perceptual)?;

// Now safe to use from multiple threads
std::thread::scope(|s| {
    s.spawn(|| transform.apply(&mut pixels1));
    s.spawn(|| transform.apply(&mut pixels2));
});
```

## Convenience Function

For one-off conversions:

```rust
use vfx_icc::{convert_rgb, Intent};

let srgb = Profile::srgb();
let aces = Profile::aces_ap1();

let mut pixels = vec![[0.5f32, 0.3, 0.2]];
convert_rgb(&mut pixels, &srgb, &aces, Intent::RelativeColorimetric)?;
```

## When to Use ICC vs OCIO

| Use Case | Recommended |
|----------|-------------|
| Camera raw processing | ICC |
| Monitor calibration | ICC |
| Print proofing | ICC |
| VFX color pipeline | OCIO |
| ACES workflow | OCIO |
| LUT-based grading | OCIO |
| Cross-app consistency | OCIO |

ICC is better when:
- Working with camera-specific profiles
- Interfacing with print/photography tools
- Using display calibration profiles

OCIO is better when:
- Managing complex VFX pipelines
- Sharing color configs across apps
- Using LUT-based workflows

## Profile Information

```rust
let profile = Profile::from_file(Path::new("camera.icc"))?;

// Basic info
println!("Description: {}", profile.description()?);
println!("Copyright: {}", profile.copyright()?);
println!("Manufacturer: {}", profile.manufacturer()?);

// Color space
println!("Color space: {:?}", profile.color_space());
println!("PCS: {:?}", profile.pcs());  // Profile Connection Space
```

## Error Handling

```rust
use vfx_icc::IccError;

match result {
    Err(IccError::InvalidProfile(msg)) => println!("Invalid ICC data: {}", msg),
    Err(IccError::LoadFailed(msg)) => println!("Load failed: {}", msg),
    Err(IccError::TransformFailed(msg)) => println!("Transform failed: {}", msg),
    Err(IccError::Io(e)) => println!("I/O error: {}", e),
    _ => {}
}
```

## Integration with vfx-io

ICC profiles can be embedded in image files:

```rust
use vfx_io::read;
use vfx_icc::{Profile, Transform, Intent};

let image = read("photo.jpg")?;

// Check for embedded profile
if let Some(icc_data) = image.metadata.attrs.get_bytes("icc_profile") {
    let embedded = Profile::from_icc(icc_data)?;
    let srgb = Profile::srgb();
    
    let transform = Transform::new(&embedded, &srgb, Intent::Perceptual)?;
    // Apply to image data...
}
```

## Dependencies

- `vfx-core` - Core types
- `lcms2` - Little CMS 2 bindings
- `thiserror` - Error handling

## lcms2 Requirement

This crate requires the lcms2 library:

**Windows (vcpkg):**
```powershell
vcpkg install lcms:x64-windows
```

**Linux:**
```bash
apt install liblcms2-dev
```

**macOS:**
```bash
brew install little-cms2
```
