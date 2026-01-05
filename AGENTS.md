# VFX-RS Architecture & Dataflows

This document describes the internal architecture, dataflows, and codepaths of the `vfx-rs` project.

## Architecture Map

```mermaid
graph TD
    subgraph "Application Layer"
        CLI[vfx-cli]
        Tests[vfx-tests]
    end

    subgraph "Processing Layer"
        OCIO[vfx-ocio]
        Color[vfx-color]
        Compute[vfx-compute]
        Ops[vfx-ops]
    end

    subgraph "I/O Layer"
        IO[vfx-io]
        ICC[vfx-icc]
    end

    subgraph "Foundation Layer"
        Core[vfx-core]
        Math[vfx-math]
        Lut[vfx-lut]
        Transfer[vfx-transfer]
        Primaries[vfx-primaries]
    end

    CLI --> IO
    CLI --> OCIO
    CLI --> Compute
    
    IO --> Core
    OCIO --> Core
    Compute --> Core
    
    Ops --> Core
    Color --> OCIO
    Color --> Compute
    
    Core --> Math
    Lut --> Math
    Transfer --> Math
```

## Dataflow: Image Loading & Processing

The following diagram illustrates how image data flows from a file through the processing pipeline back to disk.

```mermaid
sequenceDiagram
    participant File as Image File
    participant IO as vfx-io
    participant Core as vfx-core
    participant Compute as vfx-compute
    participant OCIO as vfx-ocio
    
    File->>IO: read()
    IO->>IO: Detect Format
    IO->>Core: Create ImageSpec
    IO->>Core: Populate Attrs
    IO-->>IO: Decode Pixels
    IO->>Compute: ImageData (dynamic)
    
    Compute->>Compute: Upload to GPU
    Compute->>Compute: Create ComputeImage (f32)
    
    OCIO->>OCIO: Compile Processor (src -> dst)
    OCIO->>Compute: Apply Compiled Ops
    Compute->>Compute: GPU Kernels
    
    Compute->>IO: Download result
    IO->>File: write()
```

## Codepath: Color Transformation

How a color transform is resolved and executed:

1. **Config Loading**: `vfx_ocio::Config::from_file()` parses YAML into internal structures.
2. **Processor Creation**: `config.processor(src, dst)` builds a chain of `Transform` objects.
3. **Compilation**: `Processor::from_transform()` converts high-level transforms into optimized `Op` variants (Matrix, LUT, CDL, etc.).
4. **Execution**:
    - **CPU Path**: `processor.apply_rgb()` loops over pixels, applying each `Op` sequentially.
    - **GPU Path (Planned)**: `vfx-compute` generates a shader or kernel params from `Op` list and executes on GPU.

## Key Discrepancies & Improvements

- **Ground Truth**: `vfx-core` is being unified to be the single source of truth for metadata (`Attrs`) and image specifications (`ImageSpec`).
- **Processing Backend**: `vfx-compute` is the unified backend for both generic image operations (`vfx-ops`) and color management (`vfx-ocio`).
- **Memory Safety**: Leverages Rust's ownership model to ensure zero-copy views (`ImageView`) across the pipeline while maintaining thread-safety.
