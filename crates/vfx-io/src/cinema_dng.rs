//! CinemaDNG sequence support.
//!
//! CinemaDNG is Adobe's open RAW format for cinema cameras, stored as
//! a sequence of DNG files (TIFF-based) in a directory structure.
//!
//! # Structure
//!
//! A typical CinemaDNG folder:
//! ```text
//! MyClip/
//!   frame_000001.dng
//!   frame_000002.dng
//!   ...
//!   frame_001000.dng
//!   audio.wav          (optional)
//!   metadata.xml       (optional)
//! ```
//!
//! # Example
//!
//! ```rust,no_run
//! use vfx_io::cinema_dng::{CinemaDng, CinemaDngReader};
//!
//! // Open a CinemaDNG sequence
//! let cdng = CinemaDng::open("path/to/clip/").unwrap();
//!
//! println!("Frames: {} - {}", cdng.first_frame(), cdng.last_frame());
//! println!("Resolution: {}x{}", cdng.width(), cdng.height());
//!
//! // Read a specific frame
//! let reader = CinemaDngReader::new();
//! let frame = reader.read_frame(&cdng, 1).unwrap();
//! ```

use std::path::{Path, PathBuf};

use crate::sequence::{FrameRange, Sequence, scan_dir};
use crate::tiff::{TiffReader, TiffReaderOptions};
use crate::traits::FormatReader;
use crate::{ImageData, IoError, IoResult};

// ============================================================================
// CinemaDNG Sequence
// ============================================================================

/// A CinemaDNG sequence (directory of DNG frames).
///
/// Represents a complete CinemaDNG clip with frame range, resolution,
/// and metadata extracted from the first frame.
#[derive(Debug, Clone)]
pub struct CinemaDng {
    /// Directory path.
    dir: PathBuf,
    /// Underlying sequence pattern.
    sequence: Sequence,
    /// Cached resolution (width).
    width: u32,
    /// Cached resolution (height).
    height: u32,
    /// Number of channels.
    channels: usize,
}

impl CinemaDng {
    /// Opens a CinemaDNG sequence from a directory.
    ///
    /// Scans the directory for DNG files, detects the frame pattern,
    /// and reads metadata from the first frame.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use vfx_io::cinema_dng::CinemaDng;
    ///
    /// let cdng = CinemaDng::open("path/to/clip/")?;
    /// println!("Frame count: {}", cdng.frame_count());
    /// # Ok::<(), vfx_io::IoError>(())
    /// ```
    pub fn open<P: AsRef<Path>>(path: P) -> IoResult<Self> {
        let dir = path.as_ref().to_path_buf();
        
        if !dir.is_dir() {
            return Err(IoError::InvalidFile(format!(
                "CinemaDNG path is not a directory: {}",
                dir.display()
            )));
        }

        // Scan for DNG sequences
        let sequences = scan_dir(&dir)?;
        let dng_seq = sequences
            .into_iter()
            .find(|s| {
                let suffix = s.suffix().to_lowercase();
                suffix == ".dng"
            })
            .ok_or_else(|| {
                IoError::MissingData("No DNG sequence found in directory".into())
            })?;

        // Read first frame to get resolution
        let range = dng_seq.range().ok_or_else(|| {
            IoError::Parse("Empty CinemaDNG sequence".into())
        })?;

        let first_path = dng_seq.frame_path(range.start());
        let reader = TiffReader::new();
        let first_frame = reader.read(&first_path)?;

        Ok(Self {
            dir,
            sequence: dng_seq,
            width: first_frame.width,
            height: first_frame.height,
            channels: first_frame.channels as usize,
        })
    }

    /// Opens from a single DNG file path.
    ///
    /// Detects the sequence from the filename pattern.
    pub fn from_file<P: AsRef<Path>>(path: P) -> IoResult<Self> {
        let path = path.as_ref();
        let dir = path.parent().ok_or_else(|| {
            IoError::Parse("Cannot determine parent directory".into())
        })?;
        Self::open(dir)
    }

    /// Returns the directory path.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Returns the underlying sequence.
    pub fn sequence(&self) -> &Sequence {
        &self.sequence
    }

    /// Returns the frame range.
    pub fn range(&self) -> Option<FrameRange> {
        self.sequence.frames().range()
    }

    /// Returns the first frame number.
    pub fn first_frame(&self) -> i32 {
        self.sequence.frames().first().unwrap_or(0)
    }

    /// Returns the last frame number.
    pub fn last_frame(&self) -> i32 {
        self.sequence.frames().last().unwrap_or(0)
    }

    /// Returns the total frame count.
    pub fn frame_count(&self) -> usize {
        self.sequence.frames().len()
    }

    /// Returns image width.
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Returns image height.
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Returns number of channels.
    pub fn channels(&self) -> usize {
        self.channels
    }

    /// Returns the path for a specific frame.
    pub fn frame_path(&self, frame: i32) -> PathBuf {
        self.sequence.frame_path(frame)
    }

    /// Returns true if the frame exists.
    pub fn has_frame(&self, frame: i32) -> bool {
        self.sequence.frames().contains(frame)
    }

    /// Returns an iterator over all frame numbers.
    pub fn frames(&self) -> impl Iterator<Item = i32> + '_ {
        self.sequence.frames().iter()
    }

    /// Returns missing frames in the range.
    pub fn missing_frames(&self) -> Vec<i32> {
        self.sequence.frames().missing().iter().collect()
    }
}

// ============================================================================
// CinemaDNG Reader
// ============================================================================

/// Options for reading CinemaDNG frames.
#[derive(Debug, Clone, Default)]
pub struct CinemaDngReaderOptions {
    /// TIFF reader options.
    pub tiff_options: TiffReaderOptions,
}

/// CinemaDNG frame reader.
///
/// Reads individual frames from a CinemaDNG sequence.
#[derive(Debug, Clone)]
pub struct CinemaDngReader {
    tiff_reader: TiffReader,
}

impl CinemaDngReader {
    /// Creates a new reader with default options.
    pub fn new() -> Self {
        Self {
            tiff_reader: TiffReader::new(),
        }
    }

    /// Creates a reader with custom options.
    pub fn with_options(options: CinemaDngReaderOptions) -> Self {
        Self {
            tiff_reader: TiffReader::with_options(options.tiff_options),
        }
    }

    /// Reads a single frame from the sequence.
    ///
    /// # Arguments
    ///
    /// * `cdng` - The CinemaDNG sequence
    /// * `frame` - Frame number to read
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use vfx_io::cinema_dng::{CinemaDng, CinemaDngReader};
    ///
    /// let cdng = CinemaDng::open("clip/")?;
    /// let reader = CinemaDngReader::new();
    ///
    /// let frame_1 = reader.read_frame(&cdng, 1)?;
    /// # Ok::<(), vfx_io::IoError>(())
    /// ```
    pub fn read_frame(&self, cdng: &CinemaDng, frame: i32) -> IoResult<ImageData> {
        if !cdng.has_frame(frame) {
            return Err(IoError::MissingData(format!(
                "Frame {} not found in sequence",
                frame
            )));
        }

        let path = cdng.frame_path(frame);
        self.tiff_reader.read(&path)
    }

    /// Reads a range of frames.
    ///
    /// Returns a vector of (frame_number, ImageData) pairs.
    pub fn read_range(
        &self,
        cdng: &CinemaDng,
        range: &FrameRange,
    ) -> IoResult<Vec<(i32, ImageData)>> {
        let mut results = Vec::with_capacity(range.len());

        for frame in range.iter() {
            if cdng.has_frame(frame) {
                let data = self.read_frame(cdng, frame)?;
                results.push((frame, data));
            }
        }

        Ok(results)
    }

    /// Reads all frames in the sequence.
    ///
    /// Warning: This can consume a lot of memory for long sequences.
    pub fn read_all(&self, cdng: &CinemaDng) -> IoResult<Vec<(i32, ImageData)>> {
        let frames: Vec<i32> = cdng.frames().collect();
        let mut results = Vec::with_capacity(frames.len());

        for frame in frames {
            let data = self.read_frame(cdng, frame)?;
            results.push((frame, data));
        }

        Ok(results)
    }
}

impl Default for CinemaDngReader {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Convenience Functions
// ============================================================================

/// Opens a CinemaDNG sequence.
///
/// Convenience wrapper around [`CinemaDng::open`].
pub fn open<P: AsRef<Path>>(path: P) -> IoResult<CinemaDng> {
    CinemaDng::open(path)
}

/// Reads a single frame from a CinemaDNG directory.
///
/// Convenience function for one-off frame reads.
///
/// # Example
///
/// ```rust,no_run
/// use vfx_io::cinema_dng;
///
/// let frame = cinema_dng::read_frame("clip/", 1)?;
/// # Ok::<(), vfx_io::IoError>(())
/// ```
pub fn read_frame<P: AsRef<Path>>(path: P, frame: i32) -> IoResult<ImageData> {
    let cdng = CinemaDng::open(path)?;
    CinemaDngReader::new().read_frame(&cdng, frame)
}

/// Checks if a directory contains a CinemaDNG sequence.
pub fn is_cinema_dng<P: AsRef<Path>>(path: P) -> bool {
    let path = path.as_ref();
    if !path.is_dir() {
        return false;
    }

    // Check for DNG files
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let p = entry.path();
            if let Some(ext) = p.extension() {
                if ext.eq_ignore_ascii_case("dng") {
                    return true;
                }
            }
        }
    }

    false
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn create_test_dng_sequence(dir: &Path, count: usize) -> IoResult<()> {
        fs::create_dir_all(dir)?;

        // Create minimal valid TIFF files (DNG is TIFF-based)
        // Using a simple 2x2 grayscale image
        let tiff_data = create_minimal_tiff();

        for i in 1..=count {
            let path = dir.join(format!("frame_{:06}.dng", i));
            fs::write(&path, &tiff_data)?;
        }

        Ok(())
    }

    fn create_minimal_tiff() -> Vec<u8> {
        // Minimal valid little-endian TIFF: 2x2 grayscale 8-bit
        let mut data = Vec::new();

        // Header: II (little-endian) + 42 + offset to IFD (8)
        data.extend_from_slice(&[b'I', b'I', 0x2A, 0x00, 0x08, 0x00, 0x00, 0x00]);

        // IFD at offset 8
        let num_entries: u16 = 8;
        data.extend_from_slice(&num_entries.to_le_bytes());

        // Entry 1: ImageWidth (256) = 2
        data.extend_from_slice(&256u16.to_le_bytes()); // tag
        data.extend_from_slice(&3u16.to_le_bytes());   // type (SHORT)
        data.extend_from_slice(&1u32.to_le_bytes());   // count
        data.extend_from_slice(&2u32.to_le_bytes());   // value

        // Entry 2: ImageLength (257) = 2
        data.extend_from_slice(&257u16.to_le_bytes());
        data.extend_from_slice(&3u16.to_le_bytes());
        data.extend_from_slice(&1u32.to_le_bytes());
        data.extend_from_slice(&2u32.to_le_bytes());

        // Entry 3: BitsPerSample (258) = 8
        data.extend_from_slice(&258u16.to_le_bytes());
        data.extend_from_slice(&3u16.to_le_bytes());
        data.extend_from_slice(&1u32.to_le_bytes());
        data.extend_from_slice(&8u32.to_le_bytes());

        // Entry 4: Compression (259) = 1 (none)
        data.extend_from_slice(&259u16.to_le_bytes());
        data.extend_from_slice(&3u16.to_le_bytes());
        data.extend_from_slice(&1u32.to_le_bytes());
        data.extend_from_slice(&1u32.to_le_bytes());

        // Entry 5: PhotometricInterpretation (262) = 1 (grayscale)
        data.extend_from_slice(&262u16.to_le_bytes());
        data.extend_from_slice(&3u16.to_le_bytes());
        data.extend_from_slice(&1u32.to_le_bytes());
        data.extend_from_slice(&1u32.to_le_bytes());

        // Entry 6: StripOffsets (273) = offset to pixel data
        let pixel_offset = 8 + 2 + (num_entries as u32 * 12) + 4;
        data.extend_from_slice(&273u16.to_le_bytes());
        data.extend_from_slice(&4u16.to_le_bytes()); // LONG
        data.extend_from_slice(&1u32.to_le_bytes());
        data.extend_from_slice(&pixel_offset.to_le_bytes());

        // Entry 7: SamplesPerPixel (277) = 1
        data.extend_from_slice(&277u16.to_le_bytes());
        data.extend_from_slice(&3u16.to_le_bytes());
        data.extend_from_slice(&1u32.to_le_bytes());
        data.extend_from_slice(&1u32.to_le_bytes());

        // Entry 8: StripByteCounts (279) = 4 (2x2 pixels)
        data.extend_from_slice(&279u16.to_le_bytes());
        data.extend_from_slice(&4u16.to_le_bytes());
        data.extend_from_slice(&1u32.to_le_bytes());
        data.extend_from_slice(&4u32.to_le_bytes());

        // Next IFD offset = 0 (no more IFDs)
        data.extend_from_slice(&0u32.to_le_bytes());

        // Pixel data: 2x2 = 4 bytes
        data.extend_from_slice(&[128, 64, 192, 255]);

        data
    }

    #[test]
    fn test_is_cinema_dng() {
        let temp_dir = std::env::temp_dir().join("vfx_cdng_test_detect");
        let _ = fs::remove_dir_all(&temp_dir);

        // Empty dir - not CinemaDNG
        fs::create_dir_all(&temp_dir).unwrap();
        assert!(!is_cinema_dng(&temp_dir));

        // With DNG file - is CinemaDNG
        fs::write(temp_dir.join("frame_000001.dng"), "dummy").unwrap();
        assert!(is_cinema_dng(&temp_dir));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_open_sequence() {
        let temp_dir = std::env::temp_dir().join("vfx_cdng_test_open");
        let _ = fs::remove_dir_all(&temp_dir);

        create_test_dng_sequence(&temp_dir, 5).unwrap();

        let cdng = CinemaDng::open(&temp_dir).unwrap();
        assert_eq!(cdng.frame_count(), 5);
        assert_eq!(cdng.first_frame(), 1);
        assert_eq!(cdng.last_frame(), 5);
        assert_eq!(cdng.width(), 2);
        assert_eq!(cdng.height(), 2);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_read_frame() {
        let temp_dir = std::env::temp_dir().join("vfx_cdng_test_read");
        let _ = fs::remove_dir_all(&temp_dir);

        create_test_dng_sequence(&temp_dir, 3).unwrap();

        let cdng = CinemaDng::open(&temp_dir).unwrap();
        let reader = CinemaDngReader::new();

        let frame = reader.read_frame(&cdng, 2).unwrap();
        assert_eq!(frame.width, 2);
        assert_eq!(frame.height, 2);

        // Non-existent frame
        assert!(reader.read_frame(&cdng, 99).is_err());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_missing_frames() {
        let temp_dir = std::env::temp_dir().join("vfx_cdng_test_missing");
        let _ = fs::remove_dir_all(&temp_dir);

        // Create frames 1, 3, 5 (missing 2, 4)
        fs::create_dir_all(&temp_dir).unwrap();
        let tiff_data = create_minimal_tiff();
        for i in [1, 3, 5] {
            let path = temp_dir.join(format!("frame_{:06}.dng", i));
            fs::write(&path, &tiff_data).unwrap();
        }

        let cdng = CinemaDng::open(&temp_dir).unwrap();
        let missing = cdng.missing_frames();
        assert_eq!(missing, vec![2, 4]);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_frame_iterator() {
        let temp_dir = std::env::temp_dir().join("vfx_cdng_test_iter");
        let _ = fs::remove_dir_all(&temp_dir);

        create_test_dng_sequence(&temp_dir, 3).unwrap();

        let cdng = CinemaDng::open(&temp_dir).unwrap();
        let frames: Vec<i32> = cdng.frames().collect();
        assert_eq!(frames, vec![1, 2, 3]);

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
