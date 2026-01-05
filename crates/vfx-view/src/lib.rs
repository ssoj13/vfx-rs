//! # vfx-view
//!
//! GPU-accelerated image viewer with OCIO color management.
//!
//! Features:
//! - Full OCIO display pipeline (config → display → view)
//! - Multi-layer EXR support with layer selection
//! - Channel isolation (R/G/B/A/Luminance)
//! - Exposure control
//! - Pan/zoom with mouse
//! - Keyboard shortcuts
//! - Persistent settings
//!
//! # Quick Start
//!
//! ```ignore
//! use vfx_view::{run, ViewerConfig};
//! use std::path::PathBuf;
//!
//! let config = ViewerConfig::default();
//! let exit_code = run(PathBuf::from("image.exr"), config);
//! ```
//!
//! # OCIO Configuration
//!
//! The viewer resolves OCIO config in this order:
//! 1. `--ocio` CLI argument / `ViewerConfig::ocio`
//! 2. `$OCIO` environment variable
//! 3. Built-in ACES 1.3 config
//!
//! # Keyboard Shortcuts
//!
//! | Key | Action |
//! |-----|--------|
//! | `F` | Fit image to window |
//! | `H` | Home (1:1 zoom, centered) |
//! | `+`/`-` | Zoom in/out |
//! | `R` | Red channel |
//! | `G` | Green channel |
//! | `B` | Blue channel |
//! | `A` | Alpha channel |
//! | `C` | Color mode |
//! | `L` | Luminance |
//! | `Esc` | Exit |

#![warn(missing_docs)]
#![warn(clippy::all)]

mod app;
mod handler;
mod messages;
mod state;

pub use app::{ViewerApp, ViewerConfig};
pub use state::{ChannelMode, ViewerPersistence, ViewerState};

use std::path::Path;

/// Run the image viewer.
///
/// Creates an eframe window, loads the image, and enters the event loop.
/// Returns exit code when window closes.
///
/// # Arguments
/// * `path` - Path to image file to view
/// * `config` - Viewer configuration (OCIO, display, etc.)
///
/// # Returns
/// Exit code: 0 for success, 1 for error
///
/// # Example
///
/// ```ignore
/// use vfx_view::{run, ViewerConfig};
/// use std::path::PathBuf;
///
/// let config = ViewerConfig {
///     ocio: Some(PathBuf::from("/path/to/config.ocio")),
///     display: Some("sRGB".into()),
///     view: Some("ACES 1.0 - SDR Video".into()),
///     ..Default::default()
/// };
///
/// std::process::exit(run(PathBuf::from("render.exr"), config));
/// ```
pub fn run<P: AsRef<Path>>(path: P, config: ViewerConfig) -> i32 {
    let path = path.as_ref();

    if config.verbose > 0 {
        eprintln!("[viewer] Starting: {}", path.display());
    }

    // Validate file exists
    if !path.exists() {
        eprintln!("Error: File not found: {}", path.display());
        return 1;
    }

    // Window title from filename
    let title = format!(
        "vfx view - {}",
        path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Image Viewer")
    );

    // Configure eframe
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(&title)
            .with_inner_size([1280.0, 720.0])
            .with_min_inner_size([640.0, 480.0]),
        persistence_path: persistence_path(),
        ..Default::default()
    };

    let path_owned = path.to_path_buf();
    if config.verbose > 0 {
        eprintln!("[viewer] Creating window...");
    }

    let verbose = config.verbose;
    let result = eframe::run_native(
        &title,
        native_options,
        Box::new(move |cc| {
            if verbose > 0 {
                eprintln!("[viewer] Window created, initializing app...");
            }
            Ok(Box::new(ViewerApp::new(cc, path_owned, config)))
        }),
    );

    match result {
        Ok(()) => {
            if verbose > 0 {
                eprintln!("[viewer] Exited normally");
            }
            0
        }
        Err(e) => {
            eprintln!("Viewer error: {e}");
            1
        }
    }
}

/// Get platform-specific persistence path.
fn persistence_path() -> Option<std::path::PathBuf> {
    dirs::config_dir().map(|p| p.join("vfx-rs").join("viewer"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channel_mode_labels() {
        assert_eq!(ChannelMode::Color.label(), "Color");
        assert_eq!(ChannelMode::Red.shortcut(), "R");
    }

    #[test]
    fn viewer_config_default() {
        let config = ViewerConfig::default();
        assert!(config.ocio.is_none());
        assert!(config.display.is_none());
        assert_eq!(config.verbose, 0);
    }
}
