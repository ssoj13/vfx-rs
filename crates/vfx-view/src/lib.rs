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

use std::path::{Path, PathBuf};

/// Run the image viewer with optional initial file.
///
/// If no path provided, tries to load last opened file from persistence.
///
/// # Arguments
/// * `path` - Optional path to image file
/// * `config` - Viewer configuration
///
/// # Returns
/// Exit code: 0 for success, 1 for error
pub fn run_opt(path: Option<PathBuf>, config: ViewerConfig) -> i32 {
    // Resolve path: argument > last file from persistence
    let resolved_path = path.or_else(|| {
        load_persistence().and_then(|p| p.last_file).filter(|f| f.exists())
    });

    if let Some(ref p) = resolved_path {
        if config.verbose > 0 {
            eprintln!("[viewer] Starting: {}", p.display());
        }
    } else if config.verbose > 0 {
        eprintln!("[viewer] Starting empty viewer");
    }

    // Window title
    let title = resolved_path
        .as_ref()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .map(|n| format!("vfx view - {}", n))
        .unwrap_or_else(|| "vfx view".into());

    run_internal(resolved_path, title, config)
}

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

    run_internal(Some(path.to_path_buf()), title, config)
}

/// Internal run implementation.
fn run_internal(path: Option<PathBuf>, title: String, config: ViewerConfig) -> i32 {

    // Configure eframe
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(&title)
            .with_inner_size([1280.0, 720.0])
            .with_min_inner_size([640.0, 480.0]),
        persistence_path: persistence_path(),
        ..Default::default()
    };

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
            Ok(Box::new(ViewerApp::new(cc, path, config)))
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
fn persistence_path() -> Option<PathBuf> {
    dirs::config_dir().map(|p| p.join("vfx-rs").join("viewer"))
}

/// Load persistence from disk.
fn load_persistence() -> Option<ViewerPersistence> {
    let path = persistence_path()?.join("app.ron");
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| ron::from_str(&s).ok())
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
