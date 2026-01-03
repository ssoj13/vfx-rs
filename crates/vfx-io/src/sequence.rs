//! Image sequence handling for VFX workflows.
//!
//! This module provides tools for working with numbered image sequences,
//! a common pattern in VFX and animation pipelines.
//!
//! # Frame Patterns
//!
//! Sequences are identified by patterns in filenames:
//!
//! - `shot.0001.exr` through `shot.0100.exr` - Numbered sequence
//! - `shot.%04d.exr` - Printf-style pattern
//! - `shot.####.exr` - Hash-style pattern (4 digits)
//! - `shot.@@@.exr` - At-style pattern (Nuke)
//!
//! # Example
//!
//! ```rust
//! use vfx_io::sequence::{Sequence, FrameRange};
//!
//! // Parse a sequence from a single file
//! let seq = Sequence::from_path("shot.0001.exr").unwrap();
//! assert_eq!(seq.prefix(), "shot.");
//! assert_eq!(seq.suffix(), ".exr");
//!
//! // Create a frame range
//! let range = FrameRange::new(1001, 1100);
//! assert_eq!(range.len(), 100);
//!
//! // Generate paths for all frames
//! for path in seq.paths(&range) {
//!     println!("{}", path.display());
//! }
//! ```

use std::collections::BTreeSet;
use std::fmt;
use std::path::{Path, PathBuf};

use crate::{IoError, IoResult};

/// A range of frame numbers.
///
/// Represents a contiguous range from start to end (inclusive).
///
/// # Example
///
/// ```rust
/// use vfx_io::sequence::FrameRange;
///
/// let range = FrameRange::new(1001, 1100);
/// assert_eq!(range.start(), 1001);
/// assert_eq!(range.end(), 1100);
/// assert_eq!(range.len(), 100);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FrameRange {
    start: i32,
    end: i32,
}

impl FrameRange {
    /// Creates a new frame range.
    ///
    /// # Arguments
    ///
    /// * `start` - First frame number
    /// * `end` - Last frame number (inclusive)
    ///
    /// # Example
    ///
    /// ```rust
    /// use vfx_io::sequence::FrameRange;
    ///
    /// let range = FrameRange::new(1001, 1100);
    /// ```
    pub fn new(start: i32, end: i32) -> Self {
        let (start, end) = if start <= end {
            (start, end)
        } else {
            (end, start)
        };
        Self { start, end }
    }

    /// Creates a range for a single frame.
    pub fn single(frame: i32) -> Self {
        Self::new(frame, frame)
    }

    /// Returns the start frame.
    pub fn start(&self) -> i32 {
        self.start
    }

    /// Returns the end frame.
    pub fn end(&self) -> i32 {
        self.end
    }

    /// Returns the number of frames in the range.
    pub fn len(&self) -> usize {
        (self.end - self.start + 1) as usize
    }

    /// Returns true if the range contains no frames.
    pub fn is_empty(&self) -> bool {
        false // A valid range always has at least one frame
    }

    /// Returns true if the range contains the given frame.
    pub fn contains(&self, frame: i32) -> bool {
        frame >= self.start && frame <= self.end
    }

    /// Returns an iterator over all frame numbers.
    pub fn iter(&self) -> impl Iterator<Item = i32> {
        self.start..=self.end
    }

    /// Extends the range to include the given frame.
    pub fn extend(&mut self, frame: i32) {
        if frame < self.start {
            self.start = frame;
        }
        if frame > self.end {
            self.end = frame;
        }
    }

    /// Merges with another range if they overlap or are adjacent.
    pub fn merge(&self, other: &FrameRange) -> Option<FrameRange> {
        if self.end + 1 >= other.start && other.end + 1 >= self.start {
            Some(FrameRange::new(
                self.start.min(other.start),
                self.end.max(other.end),
            ))
        } else {
            None
        }
    }
}

impl fmt::Display for FrameRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.start == self.end {
            write!(f, "{}", self.start)
        } else {
            write!(f, "{}-{}", self.start, self.end)
        }
    }
}

impl IntoIterator for FrameRange {
    type Item = i32;
    type IntoIter = std::ops::RangeInclusive<i32>;

    fn into_iter(self) -> Self::IntoIter {
        self.start..=self.end
    }
}

/// A set of frame numbers, possibly with gaps.
///
/// More flexible than [`FrameRange`], can represent discontinuous frames.
///
/// # Example
///
/// ```rust
/// use vfx_io::sequence::FrameSet;
///
/// let mut frames = FrameSet::new();
/// frames.add(1001);
/// frames.add(1005);
/// frames.add_range(1010, 1020);
///
/// assert_eq!(frames.len(), 13); // 1001, 1005, 1010-1020
/// ```
#[derive(Debug, Clone, Default)]
pub struct FrameSet {
    frames: BTreeSet<i32>,
}

impl FrameSet {
    /// Creates an empty frame set.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a frame set from a range.
    pub fn from_range(range: &FrameRange) -> Self {
        let mut set = Self::new();
        for f in range.iter() {
            set.frames.insert(f);
        }
        set
    }

    /// Adds a single frame.
    pub fn add(&mut self, frame: i32) {
        self.frames.insert(frame);
    }

    /// Adds a range of frames.
    pub fn add_range(&mut self, start: i32, end: i32) {
        let range = FrameRange::new(start, end);
        for f in range.iter() {
            self.frames.insert(f);
        }
    }

    /// Removes a frame.
    pub fn remove(&mut self, frame: i32) {
        self.frames.remove(&frame);
    }

    /// Returns the number of frames.
    pub fn len(&self) -> usize {
        self.frames.len()
    }

    /// Returns true if empty.
    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    /// Returns true if the set contains the frame.
    pub fn contains(&self, frame: i32) -> bool {
        self.frames.contains(&frame)
    }

    /// Returns the first frame, if any.
    pub fn first(&self) -> Option<i32> {
        self.frames.first().copied()
    }

    /// Returns the last frame, if any.
    pub fn last(&self) -> Option<i32> {
        self.frames.last().copied()
    }

    /// Returns the overall range (first to last).
    pub fn range(&self) -> Option<FrameRange> {
        match (self.first(), self.last()) {
            (Some(first), Some(last)) => Some(FrameRange::new(first, last)),
            _ => None,
        }
    }

    /// Returns an iterator over all frames.
    pub fn iter(&self) -> impl Iterator<Item = i32> + '_ {
        self.frames.iter().copied()
    }

    /// Returns contiguous ranges within this set.
    pub fn ranges(&self) -> Vec<FrameRange> {
        let mut result = Vec::new();
        let mut current: Option<FrameRange> = None;

        for &frame in &self.frames {
            match current {
                None => current = Some(FrameRange::single(frame)),
                Some(ref mut r) if frame == r.end() + 1 => r.extend(frame),
                Some(r) => {
                    result.push(r);
                    current = Some(FrameRange::single(frame));
                }
            }
        }

        if let Some(r) = current {
            result.push(r);
        }

        result
    }

    /// Returns missing frames within the overall range.
    pub fn missing(&self) -> FrameSet {
        let mut missing = FrameSet::new();
        if let Some(range) = self.range() {
            for f in range.iter() {
                if !self.contains(f) {
                    missing.add(f);
                }
            }
        }
        missing
    }
}

impl fmt::Display for FrameSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ranges = self.ranges();
        let parts: Vec<String> = ranges.iter().map(|r| r.to_string()).collect();
        write!(f, "{}", parts.join(","))
    }
}

/// An image sequence with a filename pattern.
///
/// Represents a series of numbered files like `shot.0001.exr` through `shot.0100.exr`.
///
/// # Example
///
/// ```rust
/// use vfx_io::sequence::Sequence;
///
/// let seq = Sequence::from_path("shot.0001.exr").unwrap();
/// assert_eq!(seq.prefix(), "shot.");
/// assert_eq!(seq.suffix(), ".exr");
/// assert_eq!(seq.padding(), 4);
/// ```
#[derive(Debug, Clone)]
pub struct Sequence {
    /// Directory containing the sequence.
    dir: PathBuf,
    /// Filename prefix (before frame number).
    prefix: String,
    /// Filename suffix (after frame number).
    suffix: String,
    /// Number of digits for padding.
    padding: usize,
    /// Known frames in this sequence.
    frames: FrameSet,
}

impl Sequence {
    /// Creates a new sequence from pattern components.
    ///
    /// # Arguments
    ///
    /// * `prefix` - Filename prefix before frame number
    /// * `suffix` - Filename suffix after frame number (usually extension)
    /// * `padding` - Number of digits for frame padding
    pub fn new(prefix: impl Into<String>, suffix: impl Into<String>, padding: usize) -> Self {
        Self {
            dir: PathBuf::new(),
            prefix: prefix.into(),
            suffix: suffix.into(),
            padding: padding.max(1),
            frames: FrameSet::new(),
        }
    }

    /// Creates a sequence by parsing a file path.
    ///
    /// Extracts the frame number and pattern from a numbered filename.
    ///
    /// # Example
    ///
    /// ```rust
    /// use vfx_io::sequence::Sequence;
    ///
    /// let seq = Sequence::from_path("shot.0042.exr").unwrap();
    /// assert_eq!(seq.prefix(), "shot.");
    /// assert_eq!(seq.suffix(), ".exr");
    /// assert_eq!(seq.padding(), 4);
    /// ```
    pub fn from_path<P: AsRef<Path>>(path: P) -> IoResult<Self> {
        let path = path.as_ref();
        let filename = path
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or_else(|| IoError::Parse("invalid path".into()))?;

        let (prefix, frame_str, suffix) = parse_frame_pattern(filename)?;
        let frame: i32 = frame_str.parse().map_err(|_| {
            IoError::Parse(format!("invalid frame number: {}", frame_str))
        })?;

        let mut seq = Self {
            dir: path.parent().map(|p| p.to_path_buf()).unwrap_or_default(),
            prefix: prefix.to_string(),
            suffix: suffix.to_string(),
            padding: frame_str.len(),
            frames: FrameSet::new(),
        };
        seq.frames.add(frame);

        Ok(seq)
    }

    /// Creates a sequence from a printf-style pattern.
    ///
    /// # Example
    ///
    /// ```rust
    /// use vfx_io::sequence::Sequence;
    ///
    /// let seq = Sequence::from_pattern("shot.%04d.exr").unwrap();
    /// assert_eq!(seq.padding(), 4);
    /// ```
    pub fn from_pattern(pattern: &str) -> IoResult<Self> {
        // Handle printf-style: %04d, %d
        if let Some(pos) = pattern.find('%') {
            if let Some(d_pos) = pattern[pos..].find('d') {
                let spec = &pattern[pos..pos + d_pos + 1];
                let prefix = &pattern[..pos];
                let suffix = &pattern[pos + d_pos + 1..];

                // Parse padding from %04d
                let padding = if spec.len() > 2 {
                    spec[1..spec.len() - 1]
                        .trim_start_matches('0')
                        .parse()
                        .unwrap_or(1)
                } else {
                    1
                };

                return Ok(Self::new(prefix, suffix, padding));
            }
        }

        // Handle hash-style: ####
        if let Some(hash_start) = pattern.find('#') {
            let hash_end = pattern[hash_start..]
                .find(|c| c != '#')
                .map(|i| hash_start + i)
                .unwrap_or(pattern.len());

            let prefix = &pattern[..hash_start];
            let suffix = &pattern[hash_end..];
            let padding = hash_end - hash_start;

            return Ok(Self::new(prefix, suffix, padding));
        }

        // Handle at-style: @@@
        if let Some(at_start) = pattern.find('@') {
            let at_end = pattern[at_start..]
                .find(|c| c != '@')
                .map(|i| at_start + i)
                .unwrap_or(pattern.len());

            let prefix = &pattern[..at_start];
            let suffix = &pattern[at_end..];
            let padding = at_end - at_start;

            return Ok(Self::new(prefix, suffix, padding));
        }

        Err(IoError::Parse("no frame pattern found".into()))
    }

    /// Returns the directory.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Sets the directory.
    pub fn with_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.dir = dir.into();
        self
    }

    /// Returns the filename prefix.
    pub fn prefix(&self) -> &str {
        &self.prefix
    }

    /// Returns the filename suffix.
    pub fn suffix(&self) -> &str {
        &self.suffix
    }

    /// Returns the frame number padding.
    pub fn padding(&self) -> usize {
        self.padding
    }

    /// Returns the known frames.
    pub fn frames(&self) -> &FrameSet {
        &self.frames
    }

    /// Returns a mutable reference to the frames.
    pub fn frames_mut(&mut self) -> &mut FrameSet {
        &mut self.frames
    }

    /// Adds a frame to the sequence.
    pub fn add_frame(&mut self, frame: i32) {
        self.frames.add(frame);
    }

    /// Returns the frame range (first to last known frame).
    pub fn range(&self) -> Option<FrameRange> {
        self.frames.range()
    }

    /// Returns the path for a specific frame.
    ///
    /// # Example
    ///
    /// ```rust
    /// use vfx_io::sequence::Sequence;
    ///
    /// let seq = Sequence::new("shot.", ".exr", 4);
    /// assert_eq!(seq.frame_path(42).to_str().unwrap(), "shot.0042.exr");
    /// ```
    pub fn frame_path(&self, frame: i32) -> PathBuf {
        let filename = format!(
            "{}{:0width$}{}",
            self.prefix,
            frame,
            self.suffix,
            width = self.padding
        );
        self.dir.join(filename)
    }

    /// Returns an iterator over paths for a frame range.
    pub fn paths<'a>(&'a self, range: &'a FrameRange) -> impl Iterator<Item = PathBuf> + 'a {
        range.iter().map(move |f| self.frame_path(f))
    }

    /// Returns the printf-style pattern.
    ///
    /// # Example
    ///
    /// ```rust
    /// use vfx_io::sequence::Sequence;
    ///
    /// let seq = Sequence::new("shot.", ".exr", 4);
    /// assert_eq!(seq.printf_pattern(), "shot.%04d.exr");
    /// ```
    pub fn printf_pattern(&self) -> String {
        format!("{}%0{}d{}", self.prefix, self.padding, self.suffix)
    }

    /// Returns the hash-style pattern.
    ///
    /// # Example
    ///
    /// ```rust
    /// use vfx_io::sequence::Sequence;
    ///
    /// let seq = Sequence::new("shot.", ".exr", 4);
    /// assert_eq!(seq.hash_pattern(), "shot.####.exr");
    /// ```
    pub fn hash_pattern(&self) -> String {
        let hashes: String = std::iter::repeat('#').take(self.padding).collect();
        format!("{}{}{}", self.prefix, hashes, self.suffix)
    }
}

impl fmt::Display for Sequence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let pattern = self.printf_pattern();
        if self.dir.as_os_str().is_empty() {
            write!(f, "{}", pattern)?;
        } else {
            write!(f, "{}/{}", self.dir.display(), pattern)?;
        }

        if let Some(range) = self.range() {
            write!(f, " [{}]", range)?;
        }

        Ok(())
    }
}

/// Scans a directory for sequences.
///
/// Groups files by pattern and returns all detected sequences.
///
/// # Example
///
/// ```rust,no_run
/// use vfx_io::sequence::scan_dir;
/// use std::path::Path;
///
/// let sequences = scan_dir(Path::new("render/")).unwrap();
/// for seq in sequences {
///     println!("{}", seq);
/// }
/// ```
pub fn scan_dir(dir: &Path) -> IoResult<Vec<Sequence>> {
    use std::collections::HashMap;

    let mut patterns: HashMap<(String, String, usize), Sequence> = HashMap::new();

    let entries = std::fs::read_dir(dir)?;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        if let Ok(seq) = Sequence::from_path(&path) {
            let key = (seq.prefix.clone(), seq.suffix.clone(), seq.padding);
            patterns
                .entry(key)
                .and_modify(|existing| {
                    for f in seq.frames.iter() {
                        existing.frames.add(f);
                    }
                })
                .or_insert_with(|| {
                    let mut s = seq;
                    s.dir = dir.to_path_buf();
                    s
                });
        }
    }

    let mut result: Vec<Sequence> = patterns.into_values().collect();
    result.sort_by(|a, b| a.prefix.cmp(&b.prefix));

    Ok(result)
}

/// Parses a frame pattern from a filename.
///
/// Returns (prefix, frame_number_str, suffix).
fn parse_frame_pattern(filename: &str) -> IoResult<(&str, &str, &str)> {
    // Find the last sequence of digits before the extension
    let mut last_digit_end = None;
    let mut last_digit_start = None;
    let mut in_digits = false;

    for (i, c) in filename.char_indices() {
        if c.is_ascii_digit() {
            if !in_digits {
                last_digit_start = Some(i);
                in_digits = true;
            }
            last_digit_end = Some(i + 1);
        } else {
            in_digits = false;
        }
    }

    match (last_digit_start, last_digit_end) {
        (Some(start), Some(end)) => {
            let prefix = &filename[..start];
            let frame = &filename[start..end];
            let suffix = &filename[end..];

            // Ensure we have a reasonable pattern
            if prefix.is_empty() && suffix.is_empty() {
                return Err(IoError::Parse("filename is just a number".into()));
            }

            Ok((prefix, frame, suffix))
        }
        _ => Err(IoError::Parse("no frame number found in filename".into())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_range() {
        let range = FrameRange::new(1001, 1100);
        assert_eq!(range.start(), 1001);
        assert_eq!(range.end(), 1100);
        assert_eq!(range.len(), 100);
        assert!(range.contains(1050));
        assert!(!range.contains(1000));
    }

    #[test]
    fn test_frame_range_reverse() {
        let range = FrameRange::new(100, 1);
        assert_eq!(range.start(), 1);
        assert_eq!(range.end(), 100);
    }

    #[test]
    fn test_frame_range_merge() {
        let r1 = FrameRange::new(1, 10);
        let r2 = FrameRange::new(11, 20);
        let r3 = FrameRange::new(30, 40);

        assert!(r1.merge(&r2).is_some());
        assert!(r1.merge(&r3).is_none());
    }

    #[test]
    fn test_frame_set() {
        let mut set = FrameSet::new();
        set.add(1);
        set.add(5);
        set.add_range(10, 15);

        assert_eq!(set.len(), 8);
        assert!(set.contains(1));
        assert!(set.contains(12));
        assert!(!set.contains(7));
    }

    #[test]
    fn test_frame_set_ranges() {
        let mut set = FrameSet::new();
        set.add_range(1, 5);
        set.add(10);
        set.add_range(15, 20);

        let ranges = set.ranges();
        assert_eq!(ranges.len(), 3);
        assert_eq!(ranges[0], FrameRange::new(1, 5));
        assert_eq!(ranges[1], FrameRange::single(10));
        assert_eq!(ranges[2], FrameRange::new(15, 20));
    }

    #[test]
    fn test_frame_set_missing() {
        let mut set = FrameSet::new();
        set.add(1);
        set.add(3);
        set.add(5);

        let missing = set.missing();
        assert_eq!(missing.len(), 2);
        assert!(missing.contains(2));
        assert!(missing.contains(4));
    }

    #[test]
    fn test_sequence_from_path() {
        let seq = Sequence::from_path("shot.0042.exr").unwrap();
        assert_eq!(seq.prefix(), "shot.");
        assert_eq!(seq.suffix(), ".exr");
        assert_eq!(seq.padding(), 4);
        assert!(seq.frames().contains(42));
    }

    #[test]
    fn test_sequence_frame_path() {
        let seq = Sequence::new("render.", ".exr", 4);
        assert_eq!(
            seq.frame_path(42).to_str().unwrap(),
            "render.0042.exr"
        );
        assert_eq!(
            seq.frame_path(1234).to_str().unwrap(),
            "render.1234.exr"
        );
    }

    #[test]
    fn test_sequence_patterns() {
        let seq = Sequence::new("shot.", ".exr", 4);
        assert_eq!(seq.printf_pattern(), "shot.%04d.exr");
        assert_eq!(seq.hash_pattern(), "shot.####.exr");
    }

    #[test]
    fn test_from_printf_pattern() {
        let seq = Sequence::from_pattern("comp.%04d.png").unwrap();
        assert_eq!(seq.prefix(), "comp.");
        assert_eq!(seq.suffix(), ".png");
        assert_eq!(seq.padding(), 4);
    }

    #[test]
    fn test_from_hash_pattern() {
        let seq = Sequence::from_pattern("shot.####.exr").unwrap();
        assert_eq!(seq.prefix(), "shot.");
        assert_eq!(seq.suffix(), ".exr");
        assert_eq!(seq.padding(), 4);
    }

    #[test]
    fn test_from_at_pattern() {
        let seq = Sequence::from_pattern("nuke.@@@@@.exr").unwrap();
        assert_eq!(seq.prefix(), "nuke.");
        assert_eq!(seq.suffix(), ".exr");
        assert_eq!(seq.padding(), 5);
    }

    #[test]
    fn test_parse_frame_pattern() {
        let (prefix, frame, suffix) = parse_frame_pattern("shot_001.0042.exr").unwrap();
        assert_eq!(prefix, "shot_001.");
        assert_eq!(frame, "0042");
        assert_eq!(suffix, ".exr");
    }

    #[test]
    fn test_display() {
        let range = FrameRange::new(1001, 1100);
        assert_eq!(format!("{}", range), "1001-1100");

        let mut set = FrameSet::new();
        set.add_range(1, 5);
        set.add(10);
        assert_eq!(format!("{}", set), "1-5,10");
    }
}
