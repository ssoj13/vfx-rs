//! Viewer state and persistence.
//!
//! Stores UI state that persists between sessions via eframe storage.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Default exposure value (EV stops).
pub const DEFAULT_EXPOSURE: f32 = 0.0;

/// Default viewport size.
pub const DEFAULT_VIEWPORT: [f32; 2] = [1280.0, 720.0];

/// Channel display mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ChannelMode {
    /// Full color (RGB/RGBA).
    #[default]
    Color,
    /// Red channel only.
    Red,
    /// Green channel only.
    Green,
    /// Blue channel only.
    Blue,
    /// Alpha channel only.
    Alpha,
    /// Luminance (grayscale).
    Luminance,
}

impl ChannelMode {
    /// Display label for UI.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Color => "Color",
            Self::Red => "Red",
            Self::Green => "Green",
            Self::Blue => "Blue",
            Self::Alpha => "Alpha",
            Self::Luminance => "Luminance",
        }
    }

    /// All available modes.
    pub const fn all() -> &'static [Self] {
        &[
            Self::Color,
            Self::Red,
            Self::Green,
            Self::Blue,
            Self::Alpha,
            Self::Luminance,
        ]
    }

    /// Keyboard shortcut hint.
    pub const fn shortcut(self) -> &'static str {
        match self {
            Self::Color => "C",
            Self::Red => "R",
            Self::Green => "G",
            Self::Blue => "B",
            Self::Alpha => "A",
            Self::Luminance => "L",
        }
    }
}

/// Persistent viewer settings (saved between sessions).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewerPersistence {
    /// Last opened file path.
    pub last_file: Option<PathBuf>,
    /// Last used OCIO config path.
    pub ocio_path: Option<PathBuf>,
    /// Last used display name.
    pub display: Option<String>,
    /// Last used view name.
    pub view: Option<String>,
    /// Last exposure value.
    pub exposure: f32,
    /// Last channel mode.
    pub channel_mode: ChannelMode,
}

impl Default for ViewerPersistence {
    fn default() -> Self {
        Self {
            last_file: None,
            ocio_path: None,
            display: None,
            view: None,
            exposure: DEFAULT_EXPOSURE,
            channel_mode: ChannelMode::Color,
        }
    }
}

/// Runtime viewer state (not persisted).
#[derive(Debug, Clone)]
pub struct ViewerState {
    // OCIO settings
    /// Current OCIO config path (None = builtin).
    pub ocio_path: Option<PathBuf>,
    /// Available displays from config.
    pub displays: Vec<String>,
    /// Current display name.
    pub display: String,
    /// Available views for current display.
    pub views: Vec<String>,
    /// Current view name.
    pub view: String,

    // Input color space
    /// Available color spaces from config.
    pub colorspaces: Vec<String>,
    /// Current input color space.
    pub input_colorspace: String,

    // Display controls
    /// Exposure adjustment (EV stops).
    pub exposure: f32,
    /// Channel display mode.
    pub channel_mode: ChannelMode,

    // Layer selection (for multi-layer EXR)
    /// Available layers.
    pub layers: Vec<String>,
    /// Current layer name.
    pub layer: String,

    // View controls
    /// Zoom level (1.0 = 100%).
    pub zoom: f32,
    /// Pan offset in image pixels.
    pub pan: [f32; 2],
    /// Viewport size in screen pixels.
    pub viewport_size: [f32; 2],

    // Image info
    /// Image dimensions.
    pub image_dims: Option<(u32, u32)>,
    /// Image file path.
    pub image_path: Option<PathBuf>,
}

impl Default for ViewerState {
    fn default() -> Self {
        Self {
            ocio_path: None,
            displays: Vec::new(),
            display: String::new(),
            views: Vec::new(),
            view: String::new(),
            colorspaces: Vec::new(),
            input_colorspace: String::new(),
            exposure: DEFAULT_EXPOSURE,
            channel_mode: ChannelMode::Color,
            layers: Vec::new(),
            layer: String::new(),
            zoom: 1.0,
            pan: [0.0, 0.0],
            viewport_size: DEFAULT_VIEWPORT,
            image_dims: None,
            image_path: None,
        }
    }
}

impl ViewerState {
    /// Creates state from persistence and CLI args.
    pub fn from_persistence(
        persistence: &ViewerPersistence,
        ocio_override: Option<PathBuf>,
        display_override: Option<&str>,
        view_override: Option<&str>,
    ) -> Self {
        Self {
            ocio_path: ocio_override.or_else(|| persistence.ocio_path.clone()),
            display: display_override
                .map(String::from)
                .or_else(|| persistence.display.clone())
                .unwrap_or_default(),
            view: view_override
                .map(String::from)
                .or_else(|| persistence.view.clone())
                .unwrap_or_default(),
            exposure: persistence.exposure,
            channel_mode: persistence.channel_mode,
            ..Default::default()
        }
    }

    /// Converts to persistence for saving.
    pub fn to_persistence(&self) -> ViewerPersistence {
        ViewerPersistence {
            last_file: self.image_path.clone(),
            ocio_path: self.ocio_path.clone(),
            display: Some(self.display.clone()),
            view: Some(self.view.clone()),
            exposure: self.exposure,
            channel_mode: self.channel_mode,
        }
    }
}
