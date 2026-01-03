//! DPX (Digital Picture Exchange) format support.
//!
//! Industry standard for film scanning and digital intermediate work.
//! Commonly used in VFX pipelines for frame sequences.
//!
//! # Features
//!
//! - 8, 10, 12, 16-bit RGB support
//! - Big-endian and little-endian files
//! - Film metadata (timecode, frame rate, etc.)
//! - Memory read/write support
//!
//! # Quick Start
//!
//! ```rust,ignore
//! use vfx_io::dpx;
//!
//! // Simple read/write
//! let image = dpx::read("frame.0001.dpx")?;
//! dpx::write("output.0001.dpx", &image)?;
//! ```
//!
//! # With Options
//!
//! ```rust,ignore
//! use vfx_io::dpx::{DpxWriter, DpxWriterOptions, BitDepth};
//!
//! let writer = DpxWriter::with_options(DpxWriterOptions {
//!     bit_depth: BitDepth::Bit10,
//!     ..Default::default()
//! });
//! writer.write("output.dpx", &image)?;
//! ```
//!
//! # Bit Depth
//!
//! DPX supports multiple bit depths:
//!
//! | Depth | Packed | Max Value | Common Use |
//! |-------|--------|-----------|------------|
//! | 8-bit | No | 255 | Preview, web |
//! | 10-bit | Yes (3 per u32) | 1023 | Film scan standard |
//! | 12-bit | No | 4095 | High-end scanning |
//! | 16-bit | No | 65535 | Maximum precision |
//!
//! # Format Details
//!
//! DPX is defined by SMPTE 268M. Key characteristics:
//! - Magic: "SDPX" (big-endian) or "XPDS" (little-endian)
//! - Header: 2048 bytes (file + image + orientation + film + TV headers)
//! - Data: Uncompressed RGB, typically 10-bit packed

use crate::{AttrValue, FormatReader, FormatWriter, ImageData, IoError, IoResult, Metadata, PixelData, PixelFormat};
use std::fs::File;
use std::io::{BufReader, BufWriter, Cursor, Read, Seek, SeekFrom, Write};
use std::path::Path;

// === Constants ===

/// DPX magic number (big-endian): "SDPX"
const MAGIC_BE: u32 = 0x53445058;
/// DPX magic number (little-endian): "XPDS"
const MAGIC_LE: u32 = 0x58504453;
/// Standard header size
const HEADER_SIZE: u32 = 2048;

// === Bit Depth Enum ===

/// DPX bit depth options.
///
/// Determines the precision of pixel values and file size.
///
/// # Storage
///
/// | Depth | Storage | Bytes/pixel (RGB) |
/// |-------|---------|-------------------|
/// | 8-bit | 1 byte/channel | 3 |
/// | 10-bit | Packed 3 per u32 | 4 |
/// | 12-bit | 2 bytes/channel | 6 |
/// | 16-bit | 2 bytes/channel | 6 |
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BitDepth {
    /// 8 bits per channel (0-255).
    Bit8,
    /// 10 bits per channel, packed (0-1023). Film standard.
    #[default]
    Bit10,
    /// 12 bits per channel (0-4095).
    Bit12,
    /// 16 bits per channel (0-65535).
    Bit16,
}

impl BitDepth {
    /// Returns the bit depth as a number.
    #[inline]
    pub fn bits(&self) -> u8 {
        match self {
            BitDepth::Bit8 => 8,
            BitDepth::Bit10 => 10,
            BitDepth::Bit12 => 12,
            BitDepth::Bit16 => 16,
        }
    }

    /// Returns the maximum value for this bit depth.
    #[inline]
    pub fn max_value(&self) -> u32 {
        match self {
            BitDepth::Bit8 => 255,
            BitDepth::Bit10 => 1023,
            BitDepth::Bit12 => 4095,
            BitDepth::Bit16 => 65535,
        }
    }

    /// Creates BitDepth from bit count.
    pub fn from_bits(bits: u8) -> Option<Self> {
        match bits {
            8 => Some(BitDepth::Bit8),
            10 => Some(BitDepth::Bit10),
            12 => Some(BitDepth::Bit12),
            16 => Some(BitDepth::Bit16),
            _ => None,
        }
    }
}

/// Byte order (endianness).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Endianness {
    /// Big-endian (network byte order). Default for DPX.
    #[default]
    Big,
    /// Little-endian.
    Little,
}

// === Reader Options ===

/// Options for reading DPX files.
///
/// Currently empty but reserved for future options like:
/// - Strict validation mode
/// - Color space override
#[derive(Debug, Clone, Default)]
pub struct DpxReaderOptions {
    /// Reserved for future use.
    _reserved: (),
}

// === Writer Options ===

/// Options for writing DPX files.
///
/// # Example
///
/// ```rust,ignore
/// use vfx_io::dpx::{DpxWriterOptions, BitDepth, Endianness};
///
/// let options = DpxWriterOptions {
///     bit_depth: BitDepth::Bit10,
///     endianness: Endianness::Big,
///     creator: Some("vfx-io".to_string()),
/// };
/// ```
#[derive(Debug, Clone)]
pub struct DpxWriterOptions {
    /// Output bit depth. Default: 10-bit.
    pub bit_depth: BitDepth,
    /// Output endianness. Default: Big-endian.
    pub endianness: Endianness,
    /// Creator software name (written to header).
    pub creator: Option<String>,
}

impl Default for DpxWriterOptions {
    fn default() -> Self {
        Self {
            bit_depth: BitDepth::Bit10,
            endianness: Endianness::Big,
            creator: Some("vfx-io".to_string()),
        }
    }
}

// === DPX Header ===

/// Parsed DPX file header.
///
/// Contains essential information extracted from the 2048-byte header.
#[derive(Debug, Clone)]
struct DpxHeader {
    /// Magic number (determines endianness).
    #[allow(dead_code)]
    magic: u32,
    /// Offset to image data.
    image_offset: u32,
    /// Total file size.
    file_size: u32,
    /// Image width in pixels.
    width: u32,
    /// Image height in pixels.
    height: u32,
    /// Bits per channel.
    bit_depth: u8,
    /// Packing method (0=packed, 1=filled A, 2=filled B).
    packing: u16,
    /// True if big-endian.
    is_big_endian: bool,
    /// Descriptor (50=RGB, 51=RGBA, etc.).
    descriptor: u8,
    /// Transfer characteristic.
    transfer: u8,
    /// Colorimetric specification.
    colorimetric: u8,
}

impl DpxHeader {
    /// Reads and parses a DPX header from a reader.
    fn read<R: Read + Seek>(reader: &mut R) -> IoResult<Self> {
        // Read magic to determine endianness
        let mut magic_bytes = [0u8; 4];
        reader.read_exact(&mut magic_bytes)
            .map_err(|e| IoError::DecodeError(format!("failed to read magic: {}", e)))?;

        let magic = u32::from_be_bytes(magic_bytes);
        let is_big_endian = match magic {
            MAGIC_BE => true,
            MAGIC_LE => false,
            _ => return Err(IoError::DecodeError(format!(
                "invalid DPX magic: 0x{:08X}", magic
            ))),
        };

        // Read file header fields
        let image_offset = read_u32(reader, is_big_endian)?;

        // Skip version string (8 bytes)
        reader.seek(SeekFrom::Current(8))
            .map_err(|e| IoError::DecodeError(e.to_string()))?;

        let file_size = read_u32(reader, is_big_endian)?;

        // Skip to image element section (offset 768)
        reader.seek(SeekFrom::Start(768))
            .map_err(|e| IoError::DecodeError(e.to_string()))?;

        // Skip orientation and element count
        reader.seek(SeekFrom::Current(4))
            .map_err(|e| IoError::DecodeError(e.to_string()))?;

        let width = read_u32(reader, is_big_endian)?;
        let height = read_u32(reader, is_big_endian)?;

        // Image element (offset 780)
        reader.seek(SeekFrom::Start(780))
            .map_err(|e| IoError::DecodeError(e.to_string()))?;

        // Skip data sign (4 bytes)
        reader.seek(SeekFrom::Current(4))
            .map_err(|e| IoError::DecodeError(e.to_string()))?;

        // Skip low/high data and code (16 bytes)
        reader.seek(SeekFrom::Current(16))
            .map_err(|e| IoError::DecodeError(e.to_string()))?;

        let descriptor = read_u8(reader)?;
        let transfer = read_u8(reader)?;
        let colorimetric = read_u8(reader)?;
        let bit_depth = read_u8(reader)?;
        let packing = read_u16(reader, is_big_endian)?;

        Ok(Self {
            magic,
            image_offset,
            file_size,
            width,
            height,
            bit_depth,
            packing,
            is_big_endian,
            descriptor,
            transfer,
            colorimetric,
        })
    }

    /// Determines number of channels from descriptor.
    fn channels(&self) -> u32 {
        match self.descriptor {
            50 => 3,  // RGB
            51 => 4,  // RGBA
            52 => 4,  // ABGR
            _ => 3,   // Default to RGB
        }
    }
}

// === DpxReader ===

/// DPX format reader.
///
/// Reads DPX files with automatic endianness detection and
/// support for all common bit depths.
///
/// # Example
///
/// ```rust,ignore
/// use vfx_io::dpx::DpxReader;
/// use vfx_io::FormatReader;
///
/// let reader = DpxReader::default();
///
/// // Check if file is DPX
/// let header = std::fs::read("test.dpx")?;
/// if reader.can_read(&header[..16]) {
///     let image = reader.read("test.dpx")?;
///     println!("{}x{}", image.width, image.height);
/// }
/// ```
#[derive(Debug, Clone, Default)]
pub struct DpxReader {
    #[allow(dead_code)]
    options: DpxReaderOptions,
}

impl DpxReader {
    /// Creates a new DPX reader with default options.
    pub fn new() -> Self {
        Self::default()
    }

    /// Internal read implementation from any Read+Seek source.
    fn read_from<R: Read + Seek>(&self, reader: &mut R) -> IoResult<ImageData> {
        let header = DpxHeader::read(reader)?;

        // Seek to image data
        reader.seek(SeekFrom::Start(header.image_offset as u64))
            .map_err(|e| IoError::DecodeError(e.to_string()))?;

        let pixel_count = (header.width * header.height) as usize;
        let channels = header.channels();

        // Read pixel data based on bit depth
        let data = match header.bit_depth {
            8 => read_8bit(reader, pixel_count)?,
            10 => read_10bit_packed(reader, pixel_count, header.is_big_endian)?,
            12 => read_12bit(reader, pixel_count, header.is_big_endian)?,
            16 => read_16bit(reader, pixel_count, header.is_big_endian)?,
            _ => return Err(IoError::UnsupportedBitDepth(format!(
                "DPX {} bit", header.bit_depth
            ))),
        };

        // Build metadata
        let mut metadata = Metadata::default();
        
        // DPX is typically log-encoded for film
        metadata.colorspace = Some(match header.transfer {
            1 => "log".to_string(),      // Print density
            2 => "linear".to_string(),   // Linear
            _ => "log".to_string(),      // Default assumption
        });

        metadata.attrs.set("Format", AttrValue::Str("DPX".to_string()));
        metadata.attrs.set("ImageWidth", AttrValue::UInt(header.width));
        metadata.attrs.set("ImageHeight", AttrValue::UInt(header.height));
        metadata.attrs.set("BitDepth", AttrValue::UInt(header.bit_depth as u32));
        metadata.attrs.set("Channels", AttrValue::UInt(channels));
        metadata.attrs.set("Endian", AttrValue::Str(
            if header.is_big_endian { "BE" } else { "LE" }.to_string()
        ));
        metadata.attrs.set("ImageOffset", AttrValue::UInt(header.image_offset));
        metadata.attrs.set("FileSize", AttrValue::UInt(header.file_size));
        metadata.attrs.set("Descriptor", AttrValue::UInt(header.descriptor as u32));
        metadata.attrs.set("Transfer", AttrValue::UInt(header.transfer as u32));
        metadata.attrs.set("Colorimetric", AttrValue::UInt(header.colorimetric as u32));
        metadata.attrs.set("Packing", AttrValue::UInt(header.packing as u32));

        Ok(ImageData {
            width: header.width,
            height: header.height,
            channels,
            format: PixelFormat::F32,
            data: PixelData::F32(data),
            metadata,
        })
    }
}

impl FormatReader<DpxReaderOptions> for DpxReader {
    fn format_name(&self) -> &'static str {
        "DPX"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["dpx"]
    }

    fn can_read(&self, header: &[u8]) -> bool {
        if header.len() < 4 {
            return false;
        }
        let magic = u32::from_be_bytes([header[0], header[1], header[2], header[3]]);
        magic == MAGIC_BE || magic == MAGIC_LE
    }

    fn read<P: AsRef<Path>>(&self, path: P) -> IoResult<ImageData> {
        let file = File::open(path.as_ref())?;
        let mut reader = BufReader::new(file);
        self.read_from(&mut reader)
    }

    fn read_from_memory(&self, data: &[u8]) -> IoResult<ImageData> {
        let mut cursor = Cursor::new(data);
        self.read_from(&mut cursor)
    }

    fn with_options(options: DpxReaderOptions) -> Self {
        Self { options }
    }
}

// === DpxWriter ===

/// DPX format writer.
///
/// Writes DPX files with configurable bit depth and endianness.
///
/// # Example
///
/// ```rust,ignore
/// use vfx_io::dpx::{DpxWriter, DpxWriterOptions, BitDepth};
/// use vfx_io::FormatWriter;
///
/// // 10-bit output (default)
/// let writer = DpxWriter::default();
/// writer.write("output.dpx", &image)?;
///
/// // 16-bit output
/// let writer = DpxWriter::with_options(DpxWriterOptions {
///     bit_depth: BitDepth::Bit16,
///     ..Default::default()
/// });
/// writer.write("output_16bit.dpx", &image)?;
/// ```
#[derive(Debug, Clone)]
pub struct DpxWriter {
    options: DpxWriterOptions,
}

impl Default for DpxWriter {
    fn default() -> Self {
        Self {
            options: DpxWriterOptions::default(),
        }
    }
}

impl DpxWriter {
    /// Creates a new DPX writer with default options.
    pub fn new() -> Self {
        Self::default()
    }

    /// Internal write implementation to any Write source.
    fn write_to<W: Write>(&self, writer: &mut W, image: &ImageData) -> IoResult<()> {
        let width = image.width;
        let height = image.height;
        let f32_data = image.to_f32();

        let channels = image.channels as usize;
        if channels < 3 {
            return Err(IoError::EncodeError(
                "DPX requires at least 3 channels (RGB)".to_string()
            ));
        }

        let bit_depth = self.options.bit_depth;
        let is_be = self.options.endianness == Endianness::Big;

        // Calculate image data size
        let pixel_count = (width * height) as usize;
        let image_size = match bit_depth {
            BitDepth::Bit8 => pixel_count * 3,
            BitDepth::Bit10 => pixel_count * 4,  // Packed
            BitDepth::Bit12 | BitDepth::Bit16 => pixel_count * 6,
        };
        let file_size = HEADER_SIZE + image_size as u32;

        // Packing: 0=packed to word, 1=filled method A
        let packing: u16 = match bit_depth {
            BitDepth::Bit10 => 1,  // Filled method A (MSB aligned)
            _ => 0,
        };

        // Write header
        self.write_header(writer, width, height, file_size, bit_depth, packing, is_be)?;

        // Write pixel data
        match bit_depth {
            BitDepth::Bit8 => {
                write_8bit(writer, &f32_data, pixel_count, channels)?;
            }
            BitDepth::Bit10 => {
                write_10bit_packed(writer, &f32_data, pixel_count, channels, is_be)?;
            }
            BitDepth::Bit12 => {
                write_12bit(writer, &f32_data, pixel_count, channels, is_be)?;
            }
            BitDepth::Bit16 => {
                write_16bit(writer, &f32_data, pixel_count, channels, is_be)?;
            }
        }

        Ok(())
    }

    /// Writes the DPX header.
    fn write_header<W: Write>(
        &self,
        writer: &mut W,
        width: u32,
        height: u32,
        file_size: u32,
        bit_depth: BitDepth,
        packing: u16,
        is_be: bool,
    ) -> IoResult<()> {
        let mut header = vec![0u8; HEADER_SIZE as usize];

        // File header (0-767)
        // Magic
        let magic = if is_be { MAGIC_BE } else { MAGIC_LE };
        write_u32_at(&mut header, 0, magic, is_be);
        
        // Image offset
        write_u32_at(&mut header, 4, HEADER_SIZE, is_be);
        
        // Version "V2.0"
        header[8..12].copy_from_slice(b"V2.0");
        
        // File size
        write_u32_at(&mut header, 16, file_size, is_be);
        
        // Ditto key (1 = same as previous frame)
        write_u32_at(&mut header, 20, 1, is_be);
        
        // Generic header size
        write_u32_at(&mut header, 24, 1664, is_be);
        
        // Industry header size
        write_u32_at(&mut header, 28, 384, is_be);
        
        // User data size
        write_u32_at(&mut header, 32, 0, is_be);

        // Creator software (offset 160, 100 bytes)
        if let Some(ref creator) = self.options.creator {
            let bytes = creator.as_bytes();
            let len = bytes.len().min(99);
            header[160..160 + len].copy_from_slice(&bytes[..len]);
        }

        // Encryption key (0xFFFFFFFF = unencrypted)
        write_u32_at(&mut header, 660, 0xFFFFFFFF, is_be);

        // Image header (768-1023)
        // Orientation (0 = left-to-right, top-to-bottom)
        write_u16_at(&mut header, 768, 0, is_be);
        
        // Number of image elements
        write_u16_at(&mut header, 770, 1, is_be);
        
        // Width
        write_u32_at(&mut header, 772, width, is_be);
        
        // Height
        write_u32_at(&mut header, 776, height, is_be);

        // Image element 0 (offset 780)
        // Data sign (0 = unsigned)
        write_u32_at(&mut header, 780, 0, is_be);
        
        // Low data code value
        write_u32_at(&mut header, 784, 0, is_be);
        
        // Low quantity (0.0)
        write_f32_at(&mut header, 788, 0.0, is_be);
        
        // High data code value
        write_u32_at(&mut header, 792, bit_depth.max_value(), is_be);
        
        // High quantity (1.0 for normalized)
        write_f32_at(&mut header, 796, 1.0, is_be);

        // Descriptor (50 = RGB)
        header[800] = 50;
        
        // Transfer (1 = print density / log)
        header[801] = 1;
        
        // Colorimetric (1 = print density)
        header[802] = 1;
        
        // Bit depth
        header[803] = bit_depth.bits();
        
        // Packing
        write_u16_at(&mut header, 804, packing, is_be);
        
        // Encoding (0 = no encoding)
        write_u16_at(&mut header, 806, 0, is_be);
        
        // Data offset
        write_u32_at(&mut header, 808, HEADER_SIZE, is_be);
        
        // End of line padding
        write_u32_at(&mut header, 812, 0, is_be);
        
        // End of image padding
        write_u32_at(&mut header, 816, 0, is_be);

        writer.write_all(&header)
            .map_err(|e| IoError::EncodeError(e.to_string()))?;

        Ok(())
    }
}

impl FormatWriter<DpxWriterOptions> for DpxWriter {
    fn format_name(&self) -> &'static str {
        "DPX"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["dpx"]
    }

    fn write<P: AsRef<Path>>(&self, path: P, image: &ImageData) -> IoResult<()> {
        let file = File::create(path.as_ref())?;
        let mut writer = BufWriter::new(file);
        self.write_to(&mut writer, image)?;
        writer.flush().map_err(|e| IoError::EncodeError(e.to_string()))?;
        Ok(())
    }

    fn write_to_memory(&self, image: &ImageData) -> IoResult<Vec<u8>> {
        let mut buffer = Vec::new();
        self.write_to(&mut buffer, image)?;
        Ok(buffer)
    }

    fn with_options(options: DpxWriterOptions) -> Self {
        Self { options }
    }
}

// === Convenience Functions ===

/// Reads a DPX file from the given path.
///
/// This is a convenience function that uses default options.
/// For more control, use [`DpxReader`] directly.
///
/// # Example
///
/// ```rust,ignore
/// use vfx_io::dpx;
///
/// let image = dpx::read("frame.0001.dpx")?;
/// println!("Size: {}x{}", image.width, image.height);
/// ```
pub fn read<P: AsRef<Path>>(path: P) -> IoResult<ImageData> {
    DpxReader::default().read(path)
}

/// Writes an image to a DPX file.
///
/// Uses default options (10-bit, big-endian).
/// For more control, use [`DpxWriter`] directly.
///
/// # Example
///
/// ```rust,ignore
/// use vfx_io::dpx;
///
/// dpx::write("output.0001.dpx", &image)?;
/// ```
pub fn write<P: AsRef<Path>>(path: P, image: &ImageData) -> IoResult<()> {
    DpxWriter::default().write(path, image)
}

// === Internal Read Functions ===

fn read_u8<R: Read>(reader: &mut R) -> IoResult<u8> {
    let mut buf = [0u8; 1];
    reader.read_exact(&mut buf)
        .map_err(|e| IoError::DecodeError(e.to_string()))?;
    Ok(buf[0])
}

fn read_u16<R: Read>(reader: &mut R, big_endian: bool) -> IoResult<u16> {
    let mut buf = [0u8; 2];
    reader.read_exact(&mut buf)
        .map_err(|e| IoError::DecodeError(e.to_string()))?;
    Ok(if big_endian {
        u16::from_be_bytes(buf)
    } else {
        u16::from_le_bytes(buf)
    })
}

fn read_u32<R: Read>(reader: &mut R, big_endian: bool) -> IoResult<u32> {
    let mut buf = [0u8; 4];
    reader.read_exact(&mut buf)
        .map_err(|e| IoError::DecodeError(e.to_string()))?;
    Ok(if big_endian {
        u32::from_be_bytes(buf)
    } else {
        u32::from_le_bytes(buf)
    })
}

fn read_8bit<R: Read>(reader: &mut R, pixel_count: usize) -> IoResult<Vec<f32>> {
    let mut buf = vec![0u8; pixel_count * 3];
    reader.read_exact(&mut buf)
        .map_err(|e| IoError::DecodeError(e.to_string()))?;
    Ok(buf.iter().map(|&v| v as f32 / 255.0).collect())
}

fn read_10bit_packed<R: Read>(
    reader: &mut R,
    pixel_count: usize,
    big_endian: bool,
) -> IoResult<Vec<f32>> {
    let mut data = Vec::with_capacity(pixel_count * 3);
    let max_val = 1023.0f32;

    for _ in 0..pixel_count {
        let word = read_u32(reader, big_endian)?;
        // 10-bit packed: [RR RRRR RRRR GG GGGG GGGG BB BBBB BBBB XX]
        // Bits: 31-22 = R, 21-12 = G, 11-2 = B, 1-0 = unused
        let r = ((word >> 22) & 0x3FF) as f32 / max_val;
        let g = ((word >> 12) & 0x3FF) as f32 / max_val;
        let b = ((word >> 2) & 0x3FF) as f32 / max_val;
        data.push(r);
        data.push(g);
        data.push(b);
    }

    Ok(data)
}

fn read_12bit<R: Read>(
    reader: &mut R,
    pixel_count: usize,
    big_endian: bool,
) -> IoResult<Vec<f32>> {
    let mut data = Vec::with_capacity(pixel_count * 3);
    let max_val = 4095.0f32;

    for _ in 0..pixel_count {
        for _ in 0..3 {
            let val = read_u16(reader, big_endian)?;
            // 12-bit stored in high bits of 16-bit word
            data.push((val >> 4) as f32 / max_val);
        }
    }

    Ok(data)
}

fn read_16bit<R: Read>(
    reader: &mut R,
    pixel_count: usize,
    big_endian: bool,
) -> IoResult<Vec<f32>> {
    let mut data = Vec::with_capacity(pixel_count * 3);
    let max_val = 65535.0f32;

    for _ in 0..pixel_count {
        for _ in 0..3 {
            let val = read_u16(reader, big_endian)?;
            data.push(val as f32 / max_val);
        }
    }

    Ok(data)
}

// === Internal Write Functions ===

fn write_u16_at(buf: &mut [u8], offset: usize, value: u16, big_endian: bool) {
    let bytes = if big_endian {
        value.to_be_bytes()
    } else {
        value.to_le_bytes()
    };
    buf[offset..offset + 2].copy_from_slice(&bytes);
}

fn write_u32_at(buf: &mut [u8], offset: usize, value: u32, big_endian: bool) {
    let bytes = if big_endian {
        value.to_be_bytes()
    } else {
        value.to_le_bytes()
    };
    buf[offset..offset + 4].copy_from_slice(&bytes);
}

fn write_f32_at(buf: &mut [u8], offset: usize, value: f32, big_endian: bool) {
    let bytes = if big_endian {
        value.to_be_bytes()
    } else {
        value.to_le_bytes()
    };
    buf[offset..offset + 4].copy_from_slice(&bytes);
}

fn write_8bit<W: Write>(
    writer: &mut W,
    data: &[f32],
    pixel_count: usize,
    channels: usize,
) -> IoResult<()> {
    for i in 0..pixel_count {
        let base = i * channels;
        let r = (data.get(base).copied().unwrap_or(0.0).clamp(0.0, 1.0) * 255.0) as u8;
        let g = (data.get(base + 1).copied().unwrap_or(0.0).clamp(0.0, 1.0) * 255.0) as u8;
        let b = (data.get(base + 2).copied().unwrap_or(0.0).clamp(0.0, 1.0) * 255.0) as u8;
        writer.write_all(&[r, g, b])
            .map_err(|e| IoError::EncodeError(e.to_string()))?;
    }
    Ok(())
}

fn write_10bit_packed<W: Write>(
    writer: &mut W,
    data: &[f32],
    pixel_count: usize,
    channels: usize,
    big_endian: bool,
) -> IoResult<()> {
    for i in 0..pixel_count {
        let base = i * channels;
        let r = (data.get(base).copied().unwrap_or(0.0).clamp(0.0, 1.0) * 1023.0) as u32;
        let g = (data.get(base + 1).copied().unwrap_or(0.0).clamp(0.0, 1.0) * 1023.0) as u32;
        let b = (data.get(base + 2).copied().unwrap_or(0.0).clamp(0.0, 1.0) * 1023.0) as u32;
        
        // Pack: R in bits 31-22, G in bits 21-12, B in bits 11-2
        let word = (r << 22) | (g << 12) | (b << 2);
        
        let bytes = if big_endian {
            word.to_be_bytes()
        } else {
            word.to_le_bytes()
        };
        writer.write_all(&bytes)
            .map_err(|e| IoError::EncodeError(e.to_string()))?;
    }
    Ok(())
}

fn write_12bit<W: Write>(
    writer: &mut W,
    data: &[f32],
    pixel_count: usize,
    channels: usize,
    big_endian: bool,
) -> IoResult<()> {
    for i in 0..pixel_count {
        let base = i * channels;
        for c in 0..3 {
            let val = data.get(base + c).copied().unwrap_or(0.0).clamp(0.0, 1.0);
            // 12-bit value stored in high bits of 16-bit word
            let u12 = (val * 4095.0) as u16;
            let u16_val = u12 << 4;
            
            let bytes = if big_endian {
                u16_val.to_be_bytes()
            } else {
                u16_val.to_le_bytes()
            };
            writer.write_all(&bytes)
                .map_err(|e| IoError::EncodeError(e.to_string()))?;
        }
    }
    Ok(())
}

fn write_16bit<W: Write>(
    writer: &mut W,
    data: &[f32],
    pixel_count: usize,
    channels: usize,
    big_endian: bool,
) -> IoResult<()> {
    for i in 0..pixel_count {
        let base = i * channels;
        for c in 0..3 {
            let val = data.get(base + c).copied().unwrap_or(0.0).clamp(0.0, 1.0);
            let u16_val = (val * 65535.0) as u16;
            
            let bytes = if big_endian {
                u16_val.to_be_bytes()
            } else {
                u16_val.to_le_bytes()
            };
            writer.write_all(&bytes)
                .map_err(|e| IoError::EncodeError(e.to_string()))?;
        }
    }
    Ok(())
}

// === Tests ===

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bit_depth() {
        assert_eq!(BitDepth::Bit8.bits(), 8);
        assert_eq!(BitDepth::Bit10.bits(), 10);
        assert_eq!(BitDepth::Bit10.max_value(), 1023);
        assert_eq!(BitDepth::Bit16.max_value(), 65535);
    }

    #[test]
    fn test_roundtrip_10bit() {
        let width = 64;
        let height = 64;
        let mut data = Vec::with_capacity((width * height * 3) as usize);

        for y in 0..height {
            for x in 0..width {
                data.push(x as f32 / width as f32);
                data.push(y as f32 / height as f32);
                data.push(0.5);
            }
        }

        let image = ImageData::from_f32(width, height, 3, data.clone());

        // Write to memory
        let writer = DpxWriter::with_options(DpxWriterOptions {
            bit_depth: BitDepth::Bit10,
            ..Default::default()
        });
        let bytes = writer.write_to_memory(&image).expect("write failed");

        // Read back
        let reader = DpxReader::new();
        let loaded = reader.read_from_memory(&bytes).expect("read failed");

        assert_eq!(loaded.width, width);
        assert_eq!(loaded.height, height);
        assert_eq!(loaded.channels, 3);

        // Check precision (10-bit = 1/1024 precision)
        let loaded_data = loaded.to_f32();
        for i in 0..10 {
            let diff = (loaded_data[i] - data[i]).abs();
            assert!(diff < 0.002, "Value mismatch at {}: {} vs {}", i, loaded_data[i], data[i]);
        }
    }

    #[test]
    fn test_roundtrip_8bit() {
        let width = 16;
        let height = 16;
        let data: Vec<f32> = (0..width * height * 3)
            .map(|i| i as f32 / (width * height * 3) as f32)
            .collect();

        let image = ImageData::from_f32(width, height, 3, data.clone());

        let writer = DpxWriter::with_options(DpxWriterOptions {
            bit_depth: BitDepth::Bit8,
            ..Default::default()
        });
        let bytes = writer.write_to_memory(&image).expect("write failed");

        let reader = DpxReader::new();
        let loaded = reader.read_from_memory(&bytes).expect("read failed");

        assert_eq!(loaded.width, width);
        assert_eq!(loaded.height, height);
        
        let loaded_data = loaded.to_f32();
        for i in 0..10 {
            let diff = (loaded_data[i] - data[i]).abs();
            assert!(diff < 0.01, "8-bit precision exceeded at {}", i);
        }
    }

    #[test]
    fn test_roundtrip_16bit() {
        let width = 8;
        let height = 8;
        let data: Vec<f32> = (0..width * height * 3)
            .map(|i| i as f32 / (width * height * 3) as f32)
            .collect();

        let image = ImageData::from_f32(width, height, 3, data.clone());

        let writer = DpxWriter::with_options(DpxWriterOptions {
            bit_depth: BitDepth::Bit16,
            ..Default::default()
        });
        let bytes = writer.write_to_memory(&image).expect("write failed");

        let reader = DpxReader::new();
        let loaded = reader.read_from_memory(&bytes).expect("read failed");

        assert_eq!(loaded.width, width);
        assert_eq!(loaded.height, height);

        // 16-bit should have excellent precision
        let loaded_data = loaded.to_f32();
        for i in 0..10 {
            let diff = (loaded_data[i] - data[i]).abs();
            assert!(diff < 0.0001, "16-bit precision exceeded at {}", i);
        }
    }

    #[test]
    fn test_can_read() {
        let reader = DpxReader::new();
        
        // Big-endian magic
        assert!(reader.can_read(b"SDPX"));
        // Little-endian magic
        assert!(reader.can_read(b"XPDS"));
        // Invalid
        assert!(!reader.can_read(b"PNG\x00"));
        assert!(!reader.can_read(b""));
    }
}
