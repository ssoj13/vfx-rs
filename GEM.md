# VFX-RS Parity and Bug Hunt Report

## Architecture Analysis

The project is structured as a collection of crates, each handling a specific part of the VFX pipeline. While the modularity is good, there are significant discrepancies in how core entities (like color spaces and metadata) are handled across the ecosystem.

### Current Dataflow
```text
[vfx-io] --(ImageData)--> [vfx-compute] --(ComputeImage)--> [vfx-color/vfx-ocio]
   |                         |                                 |
Dynamic Format Readers    CPU/GPU Backend                  Transform Logic
```

### Key Discrepancies
1.  **Color Space Representation**:
    *   `vfx-core`: Uses compile-time marker types (`Image<Srgb, u8>`).
    *   `vfx-io`: Uses dynamic strings in metadata (`metadata.colorspace: Option<String>`).
    *   `vfx-ocio`: Uses dynamic `ColorSpace` objects and a runtime registry.
    *   *Issue*: There is no unified "ground truth" for color space definitions. `vfx-core` is too rigid for dynamic OCIO configs, while `vfx-io` is too loose.

2.  **Metadata Duplication**:
    *   `vfx-io/src/attrs` and `vfx-io/src/metadata.rs` both define attribute storage logic with slightly different `AttrValue` enums.
    *   *Issue*: Violates "single source of truth" principle.

---

## Parity Report

### OpenImageIO (OIIO) Parity
*   **Format Support**: Excellent. Support for EXR, DPX, TIFF, JPEG, PNG, HEIF, WebP, AVIF, JP2 is implemented.
*   **Traits**: `FormatReader` and `FormatWriter` correctly abstract the OIIO `ImageInput`/`ImageOutput` logic.
*   **Metadata**: Flexible attribute system exists, but lacks the unified naming convention of OIIO (e.g., "ImageWidth" in some formats, none in others).
*   **Missing**:
    *   Plugin dynamic loading (intentional, using registry instead).
    *   Advanced format options (compression levels, sub-sampling) are often TODOs.

### OpenColorIO (OCIO) Parity
*   **Config Support**: Very good. Handles `.ocio` YAML parsing for both v1 and v2.
*   **Transforms**: Almost all OCIO transforms are defined (`Matrix`, `CDL`, `Lut1D`, `Lut3D`, `Exponent`, `Log`, `Range`, `FixedFunction`, `ExposureContrast`, `Allocation`, `Grading*`).
*   **Processor**: A sophisticated processor exists that can compile and optimize transform chains.
*   **Missing**:
    *   **Automatic Transform Inversion**: OCIO inverts `from_reference` if `to_reference` is missing. The Rust port currently skips the transform if the requested direction isn't explicitly defined.
    *   **FixedFunction Styles**: ACES-specific functions like `AcesRedMod` and `AcesGlow` are missing implementations.
    *   **Chromatic Adaptation**: High-level conversion functions in `vfx-color` omit white point adaptation when converting between different primaries.

---

## Discovered Bugs & Inefficiencies

### 1. GPU Processing Inefficiency (Critical)
The `Processor` and `ComputePipeline` in `vfx-compute` perform a full Upload -> Process -> Download cycle for **every single operation** in a chain.
*   **Impact**: Severe performance degradation on GPU due to PCI-E bandwidth bottlenecks. Chained operations (e.g., Exposure + Saturation + LUT) should be executed entirely on the GPU before downloading results.

### 2. Color Science Bug: Chromatic Adaptation
`vfx_color::convert::convert_rgb` converts between primaries via XYZ but **does not apply chromatic adaptation** if the white points differ (e.g., sRGB/D65 to ACEScg/D60).
*   **Impact**: Inaccurate color conversion results when moving between color spaces with different illuminants.

### 3. Tiled Processing Regression
The `run_tiled` implementation in `ComputePipeline` contains a TODO and currently **loads the full image into RAM** before tiling.
*   **Impact**: Fails to provide memory efficiency for large images (8K+), which is the primary reason for tiled processing.

### 4. DPX Packing Implementation
The DPX reader/writer assumes Method A (MSB aligned) packing and ignores the `packing` field in the header.
*   **Impact**: Potential corruption or incorrect reading of DPX files using Method B or other packing variations.

---

## Pro-grade Solutions & Roadmap

### 1. Unified ColorSpace Bridge
Implement a bridge between `vfx-core` compile-time safety and `vfx-ocio` dynamic flexibility.
*   **Solution**: A `DynamicColorSpace` enum or trait that can represent both known (compile-time) and custom (OCIO-parsed) spaces, allowing `Image<DynamicCS, T>` to work across the ecosystem.

### 2. GPU Operation Chaining
Refactor `vfx-compute` backends to support "Command Buffers" or "Task Batches".
*   **Solution**: Introduce `GpuTask` which can hold a sequence of operations. The executor should upload once, run all kernels, and download once.

### 3. Fix Color Science Logic
Update `convert_rgb` and `Config::processor` to correctly handle white point adaptation using `vfx-math::adapt_matrix`.
Implement missing `FixedFunction` styles in `vfx-ocio/src/processor.rs` using the mathematical formulas from OCIO/CTL.

### 4. True Tiled Streaming
Complete the `TileWorkflow` in `vfx-compute` to read/process/write images piece-by-piece without ever holding the full image in RAM. Use `vfx-io::streaming` traits.

### 5. Metadata Unification
Remove `vfx-io/src/metadata.rs` and unify everything under `vfx-io/src/attrs`. Establish a standard list of attribute names (following OIIO conventions) that all format readers must use.
