# Python Examples for vfx-rs

Small, focused examples demonstrating how to work with EXR files using vfx-rs.

## Setup

Build the Python bindings first:
```bash
cd /path/to/vfx-rs
maturin develop --release -m crates/vfx-rs-py/Cargo.toml
```

Or use the pre-built module from `target/release/`.

## Examples

| File | Description |
|------|-------------|
| `01_basic_io.py` | Read/write EXR and other formats |
| `02_color_grading.py` | Exposure, saturation, contrast, CDL |
| `03_numpy_interop.py` | NumPy array access and manipulation |
| `04_image_ops.py` | Resize, rotate, blur, sharpen, filters |
| `05_layers.py` | Multi-layer EXR (render passes) |
| `06_compositing.py` | Blend modes and alpha compositing |
| `07_luts.py` | 1D and 3D LUT processing |
| `08_statistics.py` | Image analysis and statistics |
| `09_color_spaces.py` | sRGB, linear, color matrix transforms |
| `10_patterns.py` | Generate test patterns and gradients |

## Viewer

Launch the built-in image viewer:
```bash
python view.py                  # View test/owl.exr
python view.py my_render.exr    # View specific file
```

## Running Examples

```bash
cd test/py
python 01_basic_io.py
python 02_color_grading.py
# etc.
```

Output images are written to `test/out/examples/`.
