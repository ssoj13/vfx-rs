//! Worker thread handler for heavy operations.
//!
//! Handles image loading, OCIO processing, and texture generation
//! off the main UI thread for responsive interaction.

use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};

use egui::Color32;
use vfx_io::{ImageData, LayeredImage};
use vfx_ocio::Config;

use crate::messages::{Generation, ViewerEvent, ViewerMsg};
use crate::state::{ChannelMode, DEFAULT_EXPOSURE, DEFAULT_VIEWPORT};

// =============================================================================
// Constants
// =============================================================================

/// Margin for fit-to-window (0.95 = 5% padding)
const FIT_MARGIN: f32 = 0.95;

/// Zoom limits
const ZOOM_MIN: f32 = 0.1;
const ZOOM_MAX: f32 = 100.0;

/// Rec.709 luminance coefficients
const LUMA_R: f32 = 0.2126;
const LUMA_G: f32 = 0.7152;
const LUMA_B: f32 = 0.0722;

/// Worker thread handler.
pub struct ViewerHandler {
    rx: Receiver<ViewerMsg>,
    tx: Sender<ViewerEvent>,
    
    // Current state
    generation: Generation,
    ocio_config: Option<Config>,
    image: Option<LayeredImage>,
    image_path: Option<PathBuf>,
    
    // Processing settings
    display: String,
    view: String,
    input_colorspace: String,
    exposure: f32,
    channel_mode: ChannelMode,
    layer: String,
    
    // View state
    zoom: f32,
    pan: [f32; 2],
    viewport: [f32; 2],
    
    verbose: u8,
}

impl ViewerHandler {
    /// Creates a new handler.
    pub fn new(rx: Receiver<ViewerMsg>, tx: Sender<ViewerEvent>, verbose: u8) -> Self {
        Self {
            rx,
            tx,
            generation: 0,
            ocio_config: None,
            image: None,
            image_path: None,
            display: String::new(),
            view: String::new(),
            input_colorspace: String::new(),
            exposure: DEFAULT_EXPOSURE,
            channel_mode: ChannelMode::Color,
            layer: String::new(),
            zoom: 1.0,
            pan: [0.0, 0.0],
            viewport: DEFAULT_VIEWPORT,
            verbose,
        }
    }

    /// Main event loop.
    pub fn run(mut self) {
        // Load default OCIO config
        self.load_ocio_config(None);
        
        while let Ok(msg) = self.rx.recv() {
            match msg {
                ViewerMsg::Close => break,
                ViewerMsg::SyncGeneration(g) => self.generation = g,
                ViewerMsg::LoadImage(path) => self.load_image(path),
                ViewerMsg::SetOcioConfig(path) => self.load_ocio_config(path),
                ViewerMsg::SetDisplay(display) => self.set_display(display),
                ViewerMsg::SetView(view) => self.set_view(view),
                ViewerMsg::SetInputColorspace(cs) => self.set_input_colorspace(cs),
                ViewerMsg::SetExposure(ev) => self.set_exposure(ev),
                ViewerMsg::SetChannelMode(mode) => self.set_channel_mode(mode),
                ViewerMsg::SetLayer(layer) => self.set_layer(layer),
                ViewerMsg::Regenerate => self.regenerate_texture(),
                ViewerMsg::Zoom { factor, center } => self.zoom(factor, center),
                ViewerMsg::Pan { delta } => self.pan(delta),
                ViewerMsg::FitToWindow => self.fit_to_window(),
                ViewerMsg::Home => self.home(),
                ViewerMsg::SetViewport(size) => self.viewport = size,
                ViewerMsg::QueryPixel { x, y } => self.query_pixel(x, y),
            }
        }
        
        if self.verbose > 0 {
            eprintln!("[viewer] Handler shutdown");
        }
    }

    fn send(&self, event: ViewerEvent) {
        let _ = self.tx.send(event);
    }

    /// Sends current view state (zoom/pan) to UI.
    fn sync_view_state(&self) {
        self.send(ViewerEvent::StateSync {
            zoom: self.zoom,
            pan: self.pan,
        });
    }

    fn log(&self, msg: &str) {
        if self.verbose > 0 {
            eprintln!("[viewer] {msg}");
        }
    }

    /// Loads OCIO config from path or uses builtin.
    fn load_ocio_config(&mut self, path: Option<PathBuf>) {
        let config = match &path {
            Some(p) => {
                self.log(&format!("Loading OCIO config: {}", p.display()));
                match Config::from_file(p) {
                    Ok(c) => c,
                    Err(e) => {
                        self.send(ViewerEvent::Error(format!("Failed to load OCIO config: {e}")));
                        return;
                    }
                }
            }
            None => {
                // Try $OCIO env var first
                if let Ok(ocio_path) = std::env::var("OCIO") {
                    self.log(&format!("Loading OCIO from $OCIO: {ocio_path}"));
                    match Config::from_file(&ocio_path) {
                        Ok(c) => c,
                        Err(e) => {
                            self.log(&format!("$OCIO failed: {e}, using builtin"));
                            vfx_ocio::builtin::aces_1_3()
                        }
                    }
                } else {
                    self.log("Using builtin ACES 1.3 config");
                    vfx_ocio::builtin::aces_1_3()
                }
            }
        };

        // Extract displays
        let displays: Vec<String> = config
            .displays()
            .displays()
            .iter()
            .map(|d| d.name().to_string())
            .collect();

        let default_display = config
            .default_display()
            .map(String::from)
            .unwrap_or_else(|| displays.first().cloned().unwrap_or_default());

        // Extract color spaces
        let colorspaces: Vec<String> = config
            .colorspaces()
            .iter()
            .map(|cs| cs.name().to_string())
            .collect();

        self.ocio_config = Some(config);
        
        // Set default display
        if self.display.is_empty() || !displays.contains(&self.display) {
            self.display = default_display.clone();
        }

        self.send(ViewerEvent::OcioConfigLoaded {
            displays,
            default_display,
            colorspaces,
        });

        // Trigger display change to populate views
        self.update_views();
    }

    fn update_views(&mut self) {
        let Some(config) = &self.ocio_config else { return };
        
        let views: Vec<String> = config
            .displays()
            .display(&self.display)
            .map(|d| d.views().iter().map(|v| v.name().to_string()).collect())
            .unwrap_or_default();

        let default_view = config
            .default_view(&self.display)
            .map(String::from)
            .unwrap_or_else(|| views.first().cloned().unwrap_or_default());

        if self.view.is_empty() || !views.contains(&self.view) {
            self.view = default_view.clone();
        }

        self.send(ViewerEvent::DisplayChanged {
            views,
            default_view,
        });
    }

    fn set_display(&mut self, display: String) {
        if self.display != display {
            self.display = display;
            self.update_views();
            self.regenerate_texture();
        }
    }

    fn set_view(&mut self, view: String) {
        if self.view != view {
            self.view = view;
            self.regenerate_texture();
        }
    }

    fn set_input_colorspace(&mut self, cs: String) {
        if self.input_colorspace != cs {
            self.input_colorspace = cs;
            self.regenerate_texture();
        }
    }

    fn set_exposure(&mut self, ev: f32) {
        if (self.exposure - ev).abs() > 0.001 {
            self.exposure = ev;
            self.regenerate_texture();
        }
    }

    fn set_channel_mode(&mut self, mode: ChannelMode) {
        if self.channel_mode != mode {
            self.channel_mode = mode;
            self.regenerate_texture();
        }
    }

    fn set_layer(&mut self, layer: String) {
        if self.layer != layer {
            self.layer = layer;
            self.regenerate_texture();
        }
    }

    /// Loads an image file.
    fn load_image(&mut self, path: PathBuf) {
        self.log(&format!("Loading image: {}", path.display()));

        // Try reading as layered first, fall back to simple
        let layered = match vfx_io::exr::read_layers(&path) {
            Ok(l) => l,
            Err(_) => {
                // Fall back to simple read
                match vfx_io::read(&path) {
                    Ok(img) => img.to_layered("default"),
                    Err(e) => {
                        self.send(ViewerEvent::Error(format!("Failed to load image: {e}")));
                        return;
                    }
                }
            }
        };

        let layers: Vec<String> = layered.layers.iter().map(|l| l.name.clone()).collect();
        let first_layer = layers.first().cloned().unwrap_or_default();
        
        // Get dimensions from first layer
        let dims = layered
            .layers
            .first()
            .map(|l| (l.width, l.height))
            .unwrap_or((0, 0));

        // Get colorspace from metadata
        let colorspace = layered.metadata.colorspace.clone();

        self.image = Some(layered);
        self.image_path = Some(path.clone());
        
        // Set layer if not set
        if self.layer.is_empty() || !layers.contains(&self.layer) {
            self.layer = first_layer;
        }

        // Set input colorspace from metadata if not already set
        if let Some(ref cs) = colorspace
            && self.input_colorspace.is_empty()
        {
            self.input_colorspace.clone_from(cs);
        }

        self.send(ViewerEvent::ImageLoaded {
            path,
            dims,
            layers,
            colorspace,
        });

        // Generate initial texture
        self.regenerate_texture();
    }

    /// Regenerates display texture with current settings.
    fn regenerate_texture(&mut self) {
        let Some(image) = &self.image else { return };
        let Some(config) = &self.ocio_config else { return };

        // Find the layer
        let layer = image
            .layers
            .iter()
            .find(|l| l.name == self.layer)
            .or_else(|| image.layers.first());

        let Some(layer) = layer else {
            self.send(ViewerEvent::Error("No layer found".into()));
            return;
        };

        // Convert layer to ImageData
        let img_data = match layer.to_image_data() {
            Ok(d) => d,
            Err(e) => {
                self.send(ViewerEvent::Error(format!("Failed to convert layer: {e}")));
                return;
            }
        };

        // Apply channel mode
        let processed = self.apply_channel_mode(&img_data);

        // Build display processor with exposure
        let pixels = self.apply_ocio_pipeline(config, &processed);

        self.send(ViewerEvent::TextureReady {
            generation: self.generation,
            width: processed.width,
            height: processed.height,
            pixels,
        });
    }

    /// Applies channel isolation.
    fn apply_channel_mode(&self, img: &ImageData) -> ImageData {
        match self.channel_mode {
            ChannelMode::Color => img.clone(),
            ChannelMode::Red | ChannelMode::Green | ChannelMode::Blue | ChannelMode::Alpha => {
                let channel_idx = match self.channel_mode {
                    ChannelMode::Red => 0,
                    ChannelMode::Green => 1,
                    ChannelMode::Blue => 2,
                    ChannelMode::Alpha => 3,
                    _ => 0,
                };
                
                let src = img.to_f32();
                let channels = img.channels as usize;
                let pixel_count = img.pixel_count();
                
                let mut out = vec![0.0f32; pixel_count * 3];
                for i in 0..pixel_count {
                    let val = if channel_idx < channels {
                        src[i * channels + channel_idx]
                    } else {
                        1.0 // Alpha default if not present
                    };
                    out[i * 3] = val;
                    out[i * 3 + 1] = val;
                    out[i * 3 + 2] = val;
                }
                
                ImageData::from_f32(img.width, img.height, 3, out)
            }
            ChannelMode::Luminance => {
                let src = img.to_f32();
                let channels = img.channels as usize;
                let pixel_count = img.pixel_count();
                
                let mut out = vec![0.0f32; pixel_count * 3];
                for i in 0..pixel_count {
                    let r = src[i * channels];
                    let g = if channels > 1 { src[i * channels + 1] } else { r };
                    let b = if channels > 2 { src[i * channels + 2] } else { r };
                    // Rec.709 luminance
                    let luma = LUMA_R * r + LUMA_G * g + LUMA_B * b;
                    out[i * 3] = luma;
                    out[i * 3 + 1] = luma;
                    out[i * 3 + 2] = luma;
                }
                
                ImageData::from_f32(img.width, img.height, 3, out)
            }
        }
    }

    /// Applies OCIO display pipeline.
    fn apply_ocio_pipeline(&self, config: &Config, img: &ImageData) -> Vec<Color32> {
        let mut pixels = img.to_f32();
        let channels = img.channels as usize;
        let pixel_count = img.pixel_count();

        // Apply exposure (simple 2^EV multiplier)
        if self.exposure.abs() > 0.001 {
            let mult = self.exposure.exp2();
            for p in &mut pixels {
                *p *= mult;
            }
        }

        // Determine input colorspace
        let input_cs = if self.input_colorspace.is_empty() {
            // Default to scene_linear role
            config
                .roles()
                .get("scene_linear")
                .or_else(|| config.roles().get("default"))
                .unwrap_or("ACEScg")
        } else {
            &self.input_colorspace
        };

        // Create display processor
        let processor = if !self.display.is_empty() && !self.view.is_empty() {
            config.display_processor(input_cs, &self.display, &self.view).ok()
        } else {
            None
        };

        // Apply OCIO transform
        if let Some(proc) = processor {
            // Process as RGB triplets
            if channels >= 3 {
                let mut rgb_pixels: Vec<[f32; 3]> = (0..pixel_count)
                    .map(|i| [
                        pixels[i * channels],
                        pixels[i * channels + 1],
                        pixels[i * channels + 2],
                    ])
                    .collect();
                
                proc.apply_rgb(&mut rgb_pixels);
                
                // Write back
                for (i, rgb) in rgb_pixels.iter().enumerate() {
                    pixels[i * channels] = rgb[0];
                    pixels[i * channels + 1] = rgb[1];
                    pixels[i * channels + 2] = rgb[2];
                }
            }
        }

        // Convert to Color32 for egui
        (0..pixel_count)
            .map(|i| {
                let r = (pixels[i * channels].clamp(0.0, 1.0) * 255.0) as u8;
                let g = if channels > 1 {
                    (pixels[i * channels + 1].clamp(0.0, 1.0) * 255.0) as u8
                } else {
                    r
                };
                let b = if channels > 2 {
                    (pixels[i * channels + 2].clamp(0.0, 1.0) * 255.0) as u8
                } else {
                    r
                };
                let a = if channels > 3 {
                    (pixels[i * channels + 3].clamp(0.0, 1.0) * 255.0) as u8
                } else {
                    255
                };
                Color32::from_rgba_unmultiplied(r, g, b, a)
            })
            .collect()
    }

    /// Query pixel value at image coordinates.
    fn query_pixel(&self, x: u32, y: u32) {
        let Some(image) = &self.image else { return };

        // Find the layer
        let layer = image
            .layers
            .iter()
            .find(|l| l.name == self.layer)
            .or_else(|| image.layers.first());

        let Some(layer) = layer else { return };

        // Bounds check
        if x >= layer.width || y >= layer.height {
            return;
        }

        // Get pixel data
        let img_data = match layer.to_image_data() {
            Ok(d) => d,
            Err(_) => return,
        };

        let pixels = img_data.to_f32();
        let channels = img_data.channels as usize;
        let idx = (y as usize * layer.width as usize + x as usize) * channels;

        let r = pixels.get(idx).copied().unwrap_or(0.0);
        let g = pixels.get(idx + 1).copied().unwrap_or(r);
        let b = pixels.get(idx + 2).copied().unwrap_or(r);
        let a = pixels.get(idx + 3).copied().unwrap_or(1.0);

        self.send(ViewerEvent::PixelValue {
            x,
            y,
            rgba: [r, g, b, a],
        });
    }

    fn zoom(&mut self, factor: f32, center: [f32; 2]) {
        let old_zoom = self.zoom;
        let new_zoom = (old_zoom * (1.0 + factor)).clamp(ZOOM_MIN, ZOOM_MAX);
        
        // Zoom-to-point: adjust pan so point under cursor stays fixed
        // center is offset from viewport center in screen pixels
        if center[0].abs() > 0.001 || center[1].abs() > 0.001 {
            // Convert screen offset to pan adjustment
            // Formula: new_pan = old_pan + center * (1/new_zoom - 1/old_zoom)
            let scale_diff = 1.0 / new_zoom - 1.0 / old_zoom;
            self.pan[0] += center[0] * scale_diff;
            self.pan[1] += center[1] * scale_diff;
        }
        
        self.zoom = new_zoom;
        self.sync_view_state();
    }

    fn pan(&mut self, delta: [f32; 2]) {
        self.pan[0] += delta[0] / self.zoom;
        self.pan[1] += delta[1] / self.zoom;
        self.sync_view_state();
    }

    fn fit_to_window(&mut self) {
        if let Some(img) = &self.image
            && let Some(layer) = img.layers.first()
        {
            let img_w = layer.width as f32;
            let img_h = layer.height as f32;
            let vp_w = self.viewport[0];
            let vp_h = self.viewport[1];
            
            if img_w > 0.0 && img_h > 0.0 {
                self.zoom = (vp_w / img_w).min(vp_h / img_h) * FIT_MARGIN;
                self.pan = [0.0, 0.0];
                self.sync_view_state();
            }
        }
    }

    fn home(&mut self) {
        self.zoom = 1.0;
        self.pan = [0.0, 0.0];
        self.sync_view_state();
    }
}
