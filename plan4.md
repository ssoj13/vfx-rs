# Plan 4: Unified Format Interface + exiftool Integration

## Summary

Унификация всех форматов vfx-io через struct + trait pattern и интеграция метаданных из exiftool-rs.

## Current State Analysis

### Problem 1: Traits defined but unused
- `ImageReader` / `ImageWriter` traits exist in traits.rs
- All 6 formats use free functions instead of struct implementations
- No `read_from_memory` / `write_to_memory` support

### Problem 2: Inconsistent format implementations
| Format | Parser lib | Options | Metadata extraction |
|--------|------------|---------|---------------------|
| EXR | `exr` crate | Compression | Full via exr::meta |
| PNG | `png` crate | - | Gamma, text chunks, ICC |
| JPEG | `jpeg-decoder` | - | JFIF, EXIF size, ICC |
| TIFF | `tiff` crate | Compression | Tags via decoder |
| HDR | manual | - | Header fields |
| DPX | manual (`byteorder`) | BitDepth | Header fields only |

### Problem 3: exiftool-rs not integrated
- metadata.rs is minimal (inspired by exiftool-attrs but simplified)
- No rich EXIF/XMP parsing
- No MakerNotes support

## Phase 1: Copy exiftool-attrs to vfx-io

Copy from `C:\projects\projects.rust\_done\exiftool-rs\crates\exiftool-attrs`:
- `value.rs` -> `crates/vfx-io/src/attrs/value.rs`
- `schema.rs` -> `crates/vfx-io/src/attrs/schema.rs`
- Adapt error handling to use IoError

Result: Replace current simple `Attrs`/`AttrValue` with richer exiftool-attrs version.

## Phase 2: Define Unified Traits

```rust
// traits.rs - updated

/// Reader configuration options
pub trait ReaderOptions: Default + Clone {
    /// Returns true if these are default options
    fn is_default(&self) -> bool { true }
}

/// Writer configuration options
pub trait WriterOptions: Default + Clone {
    fn is_default(&self) -> bool { true }
}

/// Format reader with options
pub trait FormatReader<O: ReaderOptions = ()>: Send + Sync {
    /// Format name ("EXR", "PNG", etc.)
    fn format_name(&self) -> &'static str;
    
    /// Check if can parse by magic bytes
    fn can_read(&self, header: &[u8]) -> bool;
    
    /// Read from file path
    fn read<P: AsRef<Path>>(&self, path: P) -> IoResult<ImageData>;
    
    /// Read from memory buffer
    fn read_from_memory(&self, data: &[u8]) -> IoResult<ImageData>;
    
    /// Create with options
    fn with_options(options: O) -> Self where Self: Sized;
}

/// Format writer with options  
pub trait FormatWriter<O: WriterOptions = ()>: Send + Sync {
    /// Format name
    fn format_name(&self) -> &'static str;
    
    /// Write to file path
    fn write<P: AsRef<Path>>(&self, path: P, image: &ImageData) -> IoResult<()>;
    
    /// Write to memory buffer
    fn write_to_memory(&self, image: &ImageData) -> IoResult<Vec<u8>>;
    
    /// Create with options
    fn with_options(options: O) -> Self where Self: Sized;
}
```

## Phase 3: Refactor Each Format

### 3.1 DPX (priority - most different)

```rust
// dpx.rs - new structure

/// DPX reader options
#[derive(Debug, Clone, Default)]
pub struct DpxReaderOptions {
    /// Force specific endianness (auto-detect if None)
    pub endianness: Option<Endianness>,
}

/// DPX writer options
#[derive(Debug, Clone, Default)]
pub struct DpxWriterOptions {
    /// Output bit depth (default: 10)
    pub bit_depth: BitDepth,
    /// Output endianness (default: BigEndian)
    pub endianness: Endianness,
}

pub struct DpxReader {
    options: DpxReaderOptions,
}

pub struct DpxWriter {
    options: DpxWriterOptions,
}

impl FormatReader<DpxReaderOptions> for DpxReader { ... }
impl FormatWriter<DpxWriterOptions> for DpxWriter { ... }

// Convenience functions (call struct internally)
pub fn read<P: AsRef<Path>>(path: P) -> IoResult<ImageData> {
    DpxReader::default().read(path)
}
pub fn write<P: AsRef<Path>>(path: P, image: &ImageData) -> IoResult<()> {
    DpxWriter::default().write(path, image)
}
```

### 3.2 Other formats (same pattern)

Apply same struct pattern to: EXR, PNG, JPEG, TIFF, HDR

Each gets:
- `{Format}Reader` struct + options
- `{Format}Writer` struct + options
- impl FormatReader / FormatWriter
- Keep convenience free functions

## Phase 4: DPX Improvements

Current DPX limitations:
- Writer only supports 8-bit and 10-bit
- No 12-bit or 16-bit write support
- Header parsing is incomplete (missing many SMPTE fields)

Add:
1. 12-bit and 16-bit write support
2. More complete header parsing (timecode, film info, TV info)
3. Transfer characteristic mapping (log/linear)

## Phase 5: Registry Pattern

```rust
// registry.rs - new file

pub struct FormatRegistry {
    readers: Vec<Box<dyn FormatReader>>,
    writers: Vec<Box<dyn FormatWriter>>,
}

impl FormatRegistry {
    pub fn new() -> Self { /* register all formats */ }
    pub fn detect(&self, header: &[u8]) -> Option<&dyn FormatReader>;
    pub fn reader_for_extension(&self, ext: &str) -> Option<&dyn FormatReader>;
    pub fn writer_for_extension(&self, ext: &str) -> Option<&dyn FormatWriter>;
}
```

## Implementation Order

1. **Phase 1**: Copy exiftool-attrs (~1 task)
2. **Phase 2**: Update traits.rs (~1 task)
3. **Phase 3.1**: Refactor DPX first (test new pattern) (~1 task)
4. **Phase 3.2**: Refactor EXR (~1 task)
5. **Phase 3.3**: Refactor PNG (~1 task)
6. **Phase 3.4**: Refactor JPEG (~1 task)
7. **Phase 3.5**: Refactor TIFF (~1 task)
8. **Phase 3.6**: Refactor HDR (~1 task)
9. **Phase 4**: DPX improvements (~1 task)
10. **Phase 5**: Registry pattern (~1 task)
11. **Tests**: Update all tests (~1 task)

## Files to Create/Modify

### New files:
- `crates/vfx-io/src/attrs/mod.rs`
- `crates/vfx-io/src/attrs/value.rs`
- `crates/vfx-io/src/attrs/schema.rs`
- `crates/vfx-io/src/registry.rs`

### Modified files:
- `crates/vfx-io/src/lib.rs`
- `crates/vfx-io/src/traits.rs`
- `crates/vfx-io/src/metadata.rs` (use new attrs)
- `crates/vfx-io/src/dpx.rs`
- `crates/vfx-io/src/exr.rs`
- `crates/vfx-io/src/png.rs`
- `crates/vfx-io/src/jpeg.rs`
- `crates/vfx-io/src/tiff.rs`
- `crates/vfx-io/src/hdr.rs`

## Backwards Compatibility

Keep free functions `read()`/`write()` as convenience wrappers calling struct implementations. Existing code continues to work.

---
Created: 2026-01-03
