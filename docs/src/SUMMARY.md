# Summary

[Introduction](./introduction.md)

# User Guide

- [Installation](./installation.md)
  - [Building from Source](./installation/building.md)
  - [Feature Flags](./installation/features.md)
- [CLI Reference](./cli/README.md)
  - [info - Image Information](./cli/info.md)
  - [convert - Format Conversion](./cli/convert.md)
  - [resize - Scaling](./cli/resize.md)
  - [color - Color Transforms](./cli/color.md)
  - [aces - ACES Workflow](./cli/aces.md)
  - [composite - Compositing](./cli/composite.md)
  - [layers - EXR Layers](./cli/layers.md)
  - [batch - Batch Processing](./cli/batch.md)
  - [view - Image Viewer](./cli/view.md)
- [Logging & Debugging](./logging.md)
- [Python API](./python.md)

# Architecture

- [Overview](./architecture/README.md)
- [Crate Graph](./architecture/crate-graph.md)
- [Data Flow](./architecture/data-flow.md)
- [Design Decisions](./architecture/decisions.md)

# Crate Reference

- [Overview](./crates/README.md)
- [vfx-core](./crates/core.md)
- [vfx-math](./crates/math.md)
- [vfx-transfer](./crates/transfer.md)
- [vfx-primaries](./crates/primaries.md)
- [vfx-lut](./crates/lut.md)
- [vfx-io](./crates/io.md)
- [vfx-compute](./crates/compute.md)
- [vfx-color](./crates/color.md)
- [vfx-ocio](./crates/ocio.md)
- [vfx-icc](./crates/icc.md)
- [vfx-ops](./crates/ops.md)
- [vfx-view](./crates/view.md)
- [vfx-cli](./crates/cli.md)
- [vfx-rs-py](./crates/python.md)
- [vfx-tests](./crates/tests.md)
- [vfx-bench](./crates/bench.md)

# Internals

- [Overview](./internals/README.md)
- [Image Pipeline](./internals/pipeline.md)
- [EXR Handling](./internals/exr.md)
- [Color Management](./internals/color.md)
- [GPU Compute](./internals/gpu.md)

# Development

- [Overview](./dev/README.md)
- [Testing](./dev/testing.md)
- [Benchmarks](./dev/benchmarks.md)
- [Adding Formats](./dev/adding-formats.md)
- [Adding Operations](./dev/adding-ops.md)

# Appendix

- [Overview](./appendix/README.md)
- [Format Support](./appendix/formats.md)
- [CLI Quick Reference](./appendix/cli-ref.md)
- [Glossary](./appendix/glossary.md)
