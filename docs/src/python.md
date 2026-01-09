# Python API

vfx-rs provides Python bindings via PyO3 for seamless VFX pipeline integration.

## Installation

```bash
# From source with maturin
cd crates/vfx-rs-py
pip install maturin
maturin develop --release

# Or build wheel
maturin build --release
pip install target/wheels/vfx_rs-*.whl
```

## Quick Start

```python
import vfx_rs

# Read any supported format
img = vfx_rs.read("render.exr")
print(f"Size: {img.width}x{img.height}, {img.channels} channels, format: {img.format}")

# Write with format auto-detection
vfx_rs.write("output.png", img)

# GPU/CPU processing
proc = vfx_rs.Processor()
proc.exposure(img, 1.5)  # +1.5 stops
vfx_rs.write("graded.exr", img)
```

---

## Core Types

### Image

The main image container with numpy interop.

```python
import vfx_rs
import numpy as np

# Create from numpy (HWC layout)
arr = np.random.rand(1080, 1920, 4).astype(np.float32)
img = vfx_rs.Image(arr)

# Properties
img.width      # 1920
img.height     # 1080
img.channels   # 4
img.format     # 'f32' | 'f16' | 'u16' | 'u8'

# To numpy
arr = img.numpy()           # shape: (height, width, channels)
arr_copy = img.numpy(copy=True)  # force copy

# Static constructors
empty = vfx_rs.Image.empty(1920, 1080, channels=4)
copy = img.copy()
```

### Processor

GPU-accelerated (or CPU fallback) image processing.

```python
import vfx_rs

# Auto-select best backend (GPU if available)
proc = vfx_rs.Processor()
print(proc.backend)  # 'wgpu' or 'cpu'

# Force specific backend
proc_gpu = vfx_rs.Processor("wgpu")
proc_cpu = vfx_rs.Processor("cpu")

# Read image
img = vfx_rs.read("input.exr")

# Exposure (in stops)
proc.exposure(img, 1.5)   # +1.5 stops (2.83x brighter)
proc.exposure(img, -1.0)  # -1 stop (half brightness)

# Saturation
proc.saturation(img, 1.2)  # 20% more saturated
proc.saturation(img, 0.0)  # grayscale

# Contrast
proc.contrast(img, 1.3)  # increase contrast

# CDL (Color Decision List)
proc.cdl(
    img,
    slope=[1.1, 1.0, 0.9],     # RGB slope (gain)
    offset=[0.02, 0.0, -0.01], # RGB offset (lift)
    power=[1.0, 1.0, 1.0],     # RGB power (gamma)
    saturation=1.1             # global saturation
)
```

---

## I/O Module

Format-specific readers/writers with full control over compression and bit depth.

### EXR

```python
from vfx_rs import io

# Read
img = io.read_exr("input.exr")

# Write with options
io.write_exr("output.exr", img)                           # default: zip compression, f32
io.write_exr("output.exr", img, compression="piz")        # PIZ for renders
io.write_exr("output.exr", img, compression="dwaa")       # lossy, great for previews
io.write_exr("output.exr", img, use_half=True)            # f16, half file size
io.write_exr("output.exr", img, compression="none", use_half=False)  # uncompressed f32

# Compression options:
# - "none"  : no compression
# - "rle"   : run-length encoding
# - "zip"   : zlib (default, good balance)
# - "piz"   : wavelet, best for renders
# - "dwaa"  : lossy DCT, 32-line blocks
# - "dwab"  : lossy DCT, 256-line blocks
```

### PNG

```python
from vfx_rs import io

# Read
img = io.read_png("input.png")

# Write with options
io.write_png("output.png", img)                              # default: 8-bit
io.write_png("output.png", img, bit_depth=16)                # 16-bit for precision
io.write_png("output.png", img, compression="fast")          # fast encoding
io.write_png("output.png", img, compression="best")          # max compression

# Compression: "fast" | "default" | "best" (or 0, 1, 2)
# Bit depth: 8 | 16
```

### JPEG

```python
from vfx_rs import io

img = io.read_jpeg("photo.jpg")
io.write_jpeg("output.jpg", img, quality=95)  # 0-100, default 90
```

### DPX

```python
from vfx_rs import io

img = io.read_dpx("scan.dpx")
io.write_dpx("output.dpx", img, bit_depth=10)  # 8, 10, 12, or 16

# Using BitDepth enum
io.write_dpx("output.dpx", img, bit_depth=vfx_rs.BitDepth.Bit10)
```

### TIFF / HDR

```python
from vfx_rs import io

# TIFF
img = io.read_tiff("input.tiff")
io.write_tiff("output.tiff", img)

# HDR (Radiance RGBE)
img = io.read_hdr("environment.hdr")
io.write_hdr("output.hdr", img)
```

---

## LUT Module

Load and apply color transforms from industry-standard formats.

### .cube Files (1D/3D LUT)

```python
from vfx_rs import lut

# 1D LUT
lut1d = lut.read_cube_1d("gamma.cube")
print(f"1D LUT size: {lut1d.size}")
value = lut1d.apply(0.5)  # apply to single value

# 3D LUT
lut3d = lut.read_cube_3d("film_look.cube")
print(f"3D LUT size: {lut3d.size}")  # cube dimension (e.g., 33)
rgb = lut3d.apply([0.5, 0.3, 0.2])   # apply to RGB

# Auto-detect (returns 3D)
lut3d = lut.read_cube("creative.cube")

# Create identity LUTs
identity_1d = lut.Lut1D.identity(size=1024)
identity_3d = lut.Lut3D.identity(size=33)

# Create gamma curve
gamma_lut = lut.Lut1D.gamma(size=1024, gamma=2.2)
```

### CLF (Common LUT Format)

```python
from vfx_rs import lut

# CLF contains a chain of color operations
process_list = lut.read_clf("aces_transform.clf")
print(f"Operations: {process_list.len}")
```

---

## Multi-Layer EXR

Work with complex EXR files containing multiple AOVs/passes.

```python
import vfx_rs

# Read multi-layer EXR
layered = vfx_rs.read_layered("render.exr")

# Inspect structure
print(layered.layer_names)  # ['beauty', 'diffuse', 'specular', 'depth']
print(len(layered))         # 4 layers

# Access layers by name or index
beauty = layered["beauty"]
depth = layered["depth"]
first = layered[0]

# Layer properties
print(beauty.name)           # 'beauty'
print(beauty.width)          # 1920
print(beauty.height)         # 1080
print(beauty.channel_names)  # ['R', 'G', 'B', 'A']
print(beauty.num_channels)   # 4

# Access channels
r_channel = beauty["R"]
g_channel = beauty.channel("G")
b_channel = beauty.channel_at(2)

# Channel properties
print(r_channel.name)         # 'R'
print(r_channel.kind)         # ChannelKind.Color
print(r_channel.sample_type)  # SampleType.F32
print(len(r_channel))         # 1920 * 1080

# Get channel data as numpy
r_data = r_channel.numpy()  # 1D array of samples

# Convert layer to flat Image
beauty_img = beauty.to_image()
vfx_rs.write("beauty.exr", beauty_img)

# Iterate over layers
for layer in layered:
    print(f"{layer.name}: {layer.channel_names}")
```

### Creating Multi-Layer EXR

```python
import vfx_rs
import numpy as np

# Create empty layered image
layered = vfx_rs.LayeredImage()

# Add layers from Images
beauty = vfx_rs.read("beauty.exr")
diffuse = vfx_rs.read("diffuse.exr")

layered.add_layer("beauty", beauty)
layered.add_layer("diffuse", diffuse)

# Or create from single image
layered = vfx_rs.LayeredImage.from_image(beauty, "beauty")

# Write (use io.write_exr for layered support in future)
```

---

## ACES Workflows

Complete ACES pipeline examples.

### Basic ACES Grading

```python
import vfx_rs

# Read render in ACEScg
img = vfx_rs.read("render_acescg.exr")

# Create GPU processor
proc = vfx_rs.Processor()

# Apply CDL grade (ASC-CDL standard)
proc.cdl(
    img,
    slope=[1.05, 1.0, 0.95],   # warm shadows
    offset=[0.0, 0.0, 0.0],
    power=[1.0, 1.0, 1.0],
    saturation=1.1
)

# Adjust exposure
proc.exposure(img, 0.5)  # +0.5 stops

# Save graded result (still in ACEScg)
vfx_rs.io.write_exr("graded_acescg.exr", img)
```

### CDL Values from Color Grading Software

```python
import vfx_rs

# Values exported from DaVinci Resolve/Baselight/etc
cdl_values = {
    "slope": [1.0280, 1.0100, 0.9850],
    "offset": [0.0050, 0.0000, -0.0030],
    "power": [1.0200, 1.0000, 0.9800],
    "saturation": 1.05
}

img = vfx_rs.read("plate.exr")
proc = vfx_rs.Processor()
proc.cdl(img, **cdl_values)
vfx_rs.write("graded.exr", img)
```

### Batch ACES Processing

```python
import vfx_rs
from pathlib import Path

def batch_grade(input_dir: Path, output_dir: Path, cdl: dict):
    """Apply CDL grade to entire sequence."""
    output_dir.mkdir(exist_ok=True)

    proc = vfx_rs.Processor()
    print(f"Using backend: {proc.backend}")

    for exr in sorted(input_dir.glob("*.exr")):
        img = vfx_rs.read(str(exr))
        proc.cdl(img, **cdl)

        out_path = output_dir / exr.name
        vfx_rs.io.write_exr(str(out_path), img, compression="piz")
        print(f"Processed: {exr.name}")

# Apply grade to sequence
batch_grade(
    Path("renders/shot_010/"),
    Path("graded/shot_010/"),
    cdl={
        "slope": [1.02, 1.0, 0.98],
        "offset": [0.01, 0.0, -0.005],
        "power": [1.0, 1.0, 1.0],
        "saturation": 1.1
    }
)
```

### Multi-Layer ACES Workflow

```python
import vfx_rs

# Read layered EXR with all AOVs
layered = vfx_rs.read_layered("render.exr")
print(f"Layers: {layered.layer_names}")

proc = vfx_rs.Processor()

# Grade only the beauty pass
if "beauty" in layered.layer_names:
    beauty = layered["beauty"].to_image()

    proc.exposure(beauty, 0.3)
    proc.cdl(beauty, slope=[1.05, 1.0, 0.95], saturation=1.1)

    vfx_rs.io.write_exr("beauty_graded.exr", beauty, compression="piz")

# Extract depth for compositing
if "depth" in layered.layer_names:
    depth = layered["depth"].to_image()
    # Depth pass - no color grading needed
    vfx_rs.io.write_exr("depth.exr", depth, compression="zip")
```

---

## ColorConfig API

Native OCIO config access without PyOpenColorIO dependency.

```python
import vfx_rs

# Load config
config = vfx_rs.ColorConfig()  # Default ACES 1.3
config = vfx_rs.ColorConfig.from_file("config.ocio")
config = vfx_rs.ColorConfig.aces_1_3()

# Color spaces
print(config.colorspace_names())
print(config.has_colorspace("ACEScg"))
print(config.is_colorspace_linear("ACEScg"))
print(config.colorspace_family("ACEScg"))

# Aliases and categories
print(config.colorspace_aliases("ACEScg"))  # ["ACES - ACEScg", ...]
print(config.colorspace_categories("ACEScg"))  # ["scene_linear", ...]
print(config.colorspaces_by_category("file_io"))
print(config.all_categories())

# Role shortcuts
print(config.scene_linear())       # "ACEScg"
print(config.reference())          # "ACES2065-1"
print(config.compositing_log())    # "ACEScct"
print(config.color_timing())       # "ACEScct"
print(config.data_role())          # "Raw"
print(config.color_picking())
print(config.texture_paint())
print(config.matte_paint())

# Displays and views
print(config.display_names())
print(config.default_display())
print(config.active_displays())  # Active displays list
print(config.active_views())     # Active views list
print(config.viewing_rules())    # [(name, colorspaces, encodings), ...]
for display in config.display_names():
    print(f"{display}: views={config.num_views(display)}")

# Looks
print(f"Looks: {config.num_looks()}")
for i in range(config.num_looks()):
    print(config.look_name_by_index(i))

# Named Transforms (OCIO v2.0+)
print(f"Named transforms: {config.num_named_transforms()}")
for name in config.named_transform_names():
    print(f"  {name}")
    family = config.named_transform_family(name)
    if family:
        print(f"    Family: {family}")

# Shared Views (OCIO v2.3+)
print(f"Shared views: {config.num_shared_views()}")
for name in config.shared_view_names():
    print(f"  {name}")

# Serialize/save config
yaml_str = config.serialize()
config.write_to_file("output_config.ocio")
```

### Context Variables

```python
from vfx_rs import Context, ColorConfig

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
print(len(ctx))  # 2

# Create processor with context
config = ColorConfig.aces_1_3()
config.processor_with_context("ACEScg", "sRGB", ctx)
```

### GPU Shader Generation

```python
from vfx_rs import ColorConfig, GpuProcessor, GpuLanguage

config = ColorConfig.aces_1_3()

# Create GPU processor
gpu_proc = GpuProcessor.from_config(config, "ACEScg", "sRGB")

# Generate shader code
shader = gpu_proc.generate_shader(GpuLanguage.Glsl330)
print(shader.fragment_code)
print(shader.has_textures())

# Available languages: Glsl120, Glsl330, Glsl400, GlslEs300, Hlsl50, Metal
```

### Color Conversion with ColorConfig

```python
import vfx_rs
from vfx_rs import colorconvert, ociodisplay, ociolook, ociofiletransform

img = vfx_rs.read("render.exr")
config = vfx_rs.ColorConfig.from_file("/studio/aces/config.ocio")

# Convert between color spaces
srgb = colorconvert(img, "ACEScg", "sRGB", config=config)

# Apply display transform
display = ociodisplay(img, "sRGB", "Film", "ACEScg", config=config)

# Apply look
graded = ociolook(img, "+ShowLUT", "ACEScg", "ACEScg", config=config)

# Apply LUT file
lutet = ociofiletransform(img, "grade.cube", config=config)
```

---

## External OCIO Config Integration

Using vfx-rs with OpenColorIO configurations.

### With PyOpenColorIO

```python
import vfx_rs
import numpy as np

# Optional: Use PyOpenColorIO for color space transforms
try:
    import PyOpenColorIO as ocio

    # Load ACES config
    config = ocio.Config.CreateFromFile("/path/to/aces_1.2/config.ocio")

    # Create processor for color space conversion
    processor = config.getProcessor("ACEScg", "Output - sRGB")
    cpu = processor.getDefaultCPUProcessor()

    # Read image with vfx-rs
    img = vfx_rs.read("render.exr")
    arr = img.numpy()

    # Apply OCIO transform
    cpu.applyRGBA(arr)  # in-place

    # Write result
    result = vfx_rs.Image(arr)
    vfx_rs.io.write_png("output_srgb.png", result, bit_depth=16)

except ImportError:
    print("PyOpenColorIO not available, using built-in transforms")
```

### Hybrid Pipeline

```python
import vfx_rs
import numpy as np

def hybrid_grade_pipeline(
    input_path: str,
    output_path: str,
    cdl: dict,
    ocio_config: str = None,
    view_transform: str = "sRGB"
):
    """
    Hybrid pipeline combining vfx-rs grading with OCIO view transform.

    1. Load image (vfx-rs)
    2. Apply CDL grade (vfx-rs GPU)
    3. Apply view transform (OCIO)
    4. Save result (vfx-rs)
    """
    # Load and grade with vfx-rs
    img = vfx_rs.read(input_path)
    proc = vfx_rs.Processor()
    proc.cdl(img, **cdl)

    # Apply OCIO view transform if available
    if ocio_config:
        try:
            import PyOpenColorIO as ocio
            config = ocio.Config.CreateFromFile(ocio_config)

            # Get view transform
            display = config.getDefaultDisplay()
            view = view_transform or config.getDefaultView(display)

            processor = config.getProcessor(
                ocio.ROLE_SCENE_LINEAR,
                display,
                view,
                ocio.TRANSFORM_DIR_FORWARD
            )

            arr = img.numpy()
            processor.getDefaultCPUProcessor().applyRGBA(arr)
            img = vfx_rs.Image(arr)

        except ImportError:
            print("OCIO not available, skipping view transform")

    vfx_rs.write(output_path, img)

# Use it
hybrid_grade_pipeline(
    "render_acescg.exr",
    "final_srgb.png",
    cdl={"slope": [1.02, 1.0, 0.98], "saturation": 1.1},
    ocio_config="/studio/configs/aces_1.2/config.ocio",
    view_transform="sRGB"
)
```

### Environment-Based Config

```python
import os
import vfx_rs

def get_ocio_config():
    """Get OCIO config from environment or default locations."""
    # Check environment variable
    if "OCIO" in os.environ:
        return os.environ["OCIO"]

    # Check common locations
    paths = [
        "/studio/color/config.ocio",
        "~/.config/ocio/config.ocio",
        "/opt/aces/config.ocio",
    ]

    for p in paths:
        expanded = os.path.expanduser(p)
        if os.path.exists(expanded):
            return expanded

    return None

def process_with_studio_config(input_path: str, output_path: str):
    """Process using studio OCIO config."""
    config_path = get_ocio_config()

    if config_path:
        print(f"Using OCIO config: {config_path}")
        # ... apply OCIO transforms
    else:
        print("No OCIO config found, using defaults")

    # Always use vfx-rs for I/O and grading
    img = vfx_rs.read(input_path)
    proc = vfx_rs.Processor()
    proc.exposure(img, 0.5)
    vfx_rs.write(output_path, img)
```

---

## Pipeline Integration Examples

### Nuke Bake Script

```python
#!/usr/bin/env python
"""Bake EXR sequence with CDL grade for review."""

import vfx_rs
from pathlib import Path
import argparse

def bake_sequence(
    input_pattern: str,
    output_dir: str,
    cdl_slope: list,
    cdl_offset: list,
    cdl_power: list,
    saturation: float = 1.0,
    compression: str = "dwaa",
):
    input_path = Path(input_pattern).parent
    output_path = Path(output_dir)
    output_path.mkdir(parents=True, exist_ok=True)

    proc = vfx_rs.Processor()

    for exr in sorted(input_path.glob("*.exr")):
        img = vfx_rs.read(str(exr))

        proc.cdl(
            img,
            slope=cdl_slope,
            offset=cdl_offset,
            power=cdl_power,
            saturation=saturation
        )

        out = output_path / exr.name
        vfx_rs.io.write_exr(str(out), img, compression=compression, use_half=True)
        print(f"Baked: {out}")

if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("input", help="Input directory with EXRs")
    parser.add_argument("output", help="Output directory")
    parser.add_argument("--slope", nargs=3, type=float, default=[1, 1, 1])
    parser.add_argument("--offset", nargs=3, type=float, default=[0, 0, 0])
    parser.add_argument("--power", nargs=3, type=float, default=[1, 1, 1])
    parser.add_argument("--saturation", type=float, default=1.0)
    args = parser.parse_args()

    bake_sequence(
        args.input, args.output,
        args.slope, args.offset, args.power, args.saturation
    )
```

### Maya Render Post-Process

```python
"""Post-process Maya Arnold renders."""

import vfx_rs
from pathlib import Path

def postprocess_arnold_render(render_dir: Path):
    """Extract AOVs and apply basic grade to beauty."""

    for exr in render_dir.glob("*.exr"):
        layered = vfx_rs.read_layered(str(exr))

        # Create output directories
        aov_dir = render_dir / "aovs"
        graded_dir = render_dir / "graded"
        aov_dir.mkdir(exist_ok=True)
        graded_dir.mkdir(exist_ok=True)

        # Extract and grade beauty
        if "beauty" in layered.layer_names:
            beauty = layered["beauty"].to_image()

            proc = vfx_rs.Processor()
            proc.exposure(beauty, 0.2)
            proc.saturation(beauty, 1.05)

            vfx_rs.io.write_exr(
                str(graded_dir / exr.name),
                beauty,
                compression="piz"
            )

        # Extract individual AOVs
        for layer in layered:
            if layer.name != "beauty":
                aov_img = layer.to_image()
                vfx_rs.io.write_exr(
                    str(aov_dir / f"{exr.stem}_{layer.name}.exr"),
                    aov_img,
                    compression="zip"
                )

postprocess_arnold_render(Path("/renders/shot_010/"))
```

### Houdini Integration

```python
"""Houdini COP-style processing with vfx-rs."""

import vfx_rs
import numpy as np

class VfxGrade:
    """Houdini-friendly grading wrapper."""

    def __init__(self, backend: str = None):
        self.proc = vfx_rs.Processor(backend)

    def grade(
        self,
        input_path: str,
        output_path: str,
        exposure: float = 0.0,
        saturation: float = 1.0,
        contrast: float = 1.0,
        cdl: dict = None,
    ):
        img = vfx_rs.read(input_path)

        if exposure != 0.0:
            self.proc.exposure(img, exposure)

        if saturation != 1.0:
            self.proc.saturation(img, saturation)

        if contrast != 1.0:
            self.proc.contrast(img, contrast)

        if cdl:
            self.proc.cdl(img, **cdl)

        vfx_rs.write(output_path, img)
        return img

# Usage in Houdini Python SOP
grader = VfxGrade()
grader.grade(
    "$HIP/render/beauty.$F4.exr",
    "$HIP/comp/graded.$F4.exr",
    exposure=0.3,
    saturation=1.1,
    cdl={"slope": [1.02, 1.0, 0.98]}
)
```

---

## NumPy Interop

### Processing with SciPy/scikit-image

```python
import vfx_rs
import numpy as np
from scipy.ndimage import gaussian_filter

# Load
img = vfx_rs.read("render.exr")
arr = img.numpy()

# Process with scipy
blurred = gaussian_filter(arr, sigma=2.0)

# Save
result = vfx_rs.Image(blurred.astype(np.float32))
vfx_rs.write("blurred.exr", result)
```

### Custom Operations

```python
import vfx_rs
import numpy as np

def apply_vignette(img: vfx_rs.Image, strength: float = 0.5):
    """Apply vignette effect."""
    arr = img.numpy()
    h, w, c = arr.shape

    # Create radial gradient
    y, x = np.ogrid[:h, :w]
    cx, cy = w / 2, h / 2
    r = np.sqrt((x - cx) ** 2 + (y - cy) ** 2)
    r_max = np.sqrt(cx ** 2 + cy ** 2)

    # Apply falloff
    vignette = 1.0 - strength * (r / r_max) ** 2
    arr *= vignette[:, :, np.newaxis]

    return vfx_rs.Image(arr.astype(np.float32))

# Usage
img = vfx_rs.read("input.exr")
result = apply_vignette(img, strength=0.3)
vfx_rs.write("vignetted.exr", result)
```

---

## Viewer (Optional)

When built with the `viewer` feature:

```python
import vfx_rs

# Simple view
vfx_rs.view("render.exr")

# With display/view transforms
vfx_rs.view("render.exr", display="sRGB", view="ACES 1.0 - SDR Video")
```

---

## Error Handling

```python
import vfx_rs

try:
    img = vfx_rs.read("missing.exr")
except IOError as e:
    print(f"Read failed: {e}")

try:
    proc = vfx_rs.Processor("invalid_backend")
except RuntimeError as e:
    print(f"Processor init failed: {e}")
```

---

## Performance Tips

1. **Use GPU when available**: `Processor()` auto-selects GPU
2. **Batch operations**: Minimize read/write cycles
3. **Half-float EXR**: Use `use_half=True` for 50% smaller files
4. **PIZ compression**: Best for CG renders
5. **DWAA/DWAB**: Fast lossy compression for previews
6. **Avoid copies**: `img.numpy()` returns view when possible

```python
import vfx_rs

# Efficient batch processing
proc = vfx_rs.Processor()  # init once

for path in paths:
    img = vfx_rs.read(path)
    proc.exposure(img, 0.5)
    proc.cdl(img, slope=[1.02, 1.0, 0.98])
    vfx_rs.io.write_exr(
        path.replace(".exr", "_graded.exr"),
        img,
        compression="piz",
        use_half=True
    )
```
