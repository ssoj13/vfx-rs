//! Message types for UI <-> Worker communication.
//!
//! Uses crossbar pattern: UI sends commands, worker sends events.

use std::path::PathBuf;
use egui::Color32;

use crate::state::ChannelMode;

/// Generation counter for invalidating stale results.
pub type Generation = u64;

/// Messages from UI thread to worker thread.
#[derive(Debug, Clone)]
pub enum ViewerMsg {
    /// Load an image file.
    LoadImage(PathBuf),

    /// Set OCIO config (None = builtin).
    SetOcioConfig(Option<PathBuf>),

    /// Set display name.
    SetDisplay(String),

    /// Set view name.
    SetView(String),

    /// Set input color space.
    SetInputColorspace(String),

    /// Set exposure value (EV stops).
    SetExposure(f32),

    /// Set channel display mode.
    SetChannelMode(ChannelMode),

    /// Set current layer (for multi-layer images).
    SetLayer(String),

    /// Regenerate display texture.
    Regenerate,

    /// Sync generation counter.
    SyncGeneration(Generation),

    /// Zoom by factor around center point.
    Zoom {
        factor: f32,
        center: [f32; 2],
    },

    /// Pan by delta.
    Pan {
        delta: [f32; 2],
    },

    /// Fit image to viewport.
    FitToWindow,

    /// Reset to 1:1 zoom, centered.
    Home,

    /// Set viewport size.
    SetViewport([f32; 2]),

    /// Close viewer.
    Close,
}

/// Events from worker thread to UI thread.
#[derive(Debug)]
pub enum ViewerEvent {
    /// Image loaded successfully.
    ImageLoaded {
        path: PathBuf,
        dims: (u32, u32),
        layers: Vec<String>,
        colorspace: Option<String>,
    },

    /// OCIO config loaded.
    OcioConfigLoaded {
        displays: Vec<String>,
        default_display: String,
        colorspaces: Vec<String>,
    },

    /// Display changed, views updated.
    DisplayChanged {
        views: Vec<String>,
        default_view: String,
    },

    /// Display texture ready.
    TextureReady {
        generation: Generation,
        width: u32,
        height: u32,
        pixels: Vec<Color32>,
    },

    /// View state sync (zoom/pan changed by worker).
    StateSync {
        zoom: f32,
        pan: [f32; 2],
    },

    /// Error occurred.
    Error(String),
}
