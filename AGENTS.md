# Agents

## Dataflow (ASCII)

Input (file or memory)
  |
  v
vfx-io::read
  |
  v
Format::detect (magic bytes -> extension)
  |
  +--> exr::read -> vfx-exr read builder -> ImageData (RGBA, f32)
  |
  +--> png/jpeg/tiff/... -> format reader -> ImageData
  |
  v
ImageData / LayeredImage

Deep EXR (intended)
  |
  v
vfx-io::exr::read_deep
  |
  v
vfx-io::exr_deep::read_deep_exr
  |
  v
vfx-exr::image::read::deep (DeepSamples SoA)
  |
  v
DeepData (AoS) or DeepSamples (SoA)

Output (file or memory)
  |
  v
vfx-io::write
  |
  v
Format::from_extension
  |
  +--> exr::write -> vfx-exr write -> file/buffer
  |
  +--> png/jpeg/tiff/... -> format writer -> file/buffer

## Codepath (ASCII)

EXR read (flat)
  vfx-io::exr::read
    -> ExrReader::read
      -> ExrReader::read_impl
        -> vfx-exr::image::read::read()
        -> first_valid_layer().rgba_channels()
        -> ImageData (RGBA, f32)
        -> metadata extraction (vfx-exr MetaData)

EXR write (flat)
  vfx-io::exr::write
    -> ExrWriter::write
      -> ExrWriter::write_impl
        -> ImageData -> RGBA tuples -> vfx-exr::Image::from_layer
        -> write().to_buffered()

EXR deep read/write
  vfx-io::exr::read_deep
    -> vfx-io::exr_deep::read_deep_exr
    -> vfx-exr::image::read::deep::read_first_deep_layer_from_file
    -> DeepSamples (SoA) -> DeepData (AoS)
  vfx-io::exr::write_deep*
    -> DeepData (AoS) -> DeepSamples (SoA)
    -> vfx-exr::image::write::deep::write_deep_scanlines_to_file
