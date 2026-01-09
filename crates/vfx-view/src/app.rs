//! Main viewer application with eframe/egui integration.
//!
//! Handles UI rendering and user interaction.

use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread::{self, JoinHandle};

use egui::{Color32, ColorImage, TextureHandle, TextureOptions, Vec2};

use crate::handler::ViewerHandler;
use crate::messages::{Generation, ViewerEvent, ViewerMsg};
use crate::state::{ChannelMode, ViewerPersistence, ViewerState, DEFAULT_EXPOSURE};

/// Main viewer application.
pub struct ViewerApp {
    /// Sender for commands to worker thread.
    tx: Sender<ViewerMsg>,
    /// Receiver for results from worker thread.
    rx: Receiver<ViewerEvent>,
    /// Worker thread handle (Option for Drop).
    worker: Option<JoinHandle<()>>,
    
    /// Current display texture.
    texture: Option<TextureHandle>,
    /// Error message to display.
    error: Option<String>,
    
    /// Runtime state.
    state: ViewerState,
    
    /// Generation counter for stale result rejection.
    generation: Generation,
    
    /// CLI overrides.
    cli_ocio: Option<PathBuf>,
    cli_display: Option<String>,
    cli_view: Option<String>,
    cli_colorspace: Option<String>,
}

/// Configuration for launching the viewer.
#[derive(Debug, Clone, Default)]
pub struct ViewerConfig {
    /// OCIO config path override.
    pub ocio: Option<PathBuf>,
    /// Display override.
    pub display: Option<String>,
    /// View override.
    pub view: Option<String>,
    /// Input colorspace override.
    pub colorspace: Option<String>,
    /// Verbosity level.
    pub verbose: u8,
}

impl ViewerApp {
    /// Creates a new viewer application.
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        image_path: Option<PathBuf>,
        config: ViewerConfig,
    ) -> Self {
        // Create bidirectional channels
        let (tx_to_worker, rx_in_worker) = channel();
        let (tx_to_ui, rx_from_worker) = channel();

        // Spawn worker thread
        let verbose = config.verbose;
        let worker = thread::spawn(move || {
            let handler = ViewerHandler::new(rx_in_worker, tx_to_ui, verbose);
            handler.run();
        });

        // Load persisted settings
        let persistence: ViewerPersistence = cc
            .storage
            .and_then(|s| eframe::get_value(s, "vfx_viewer_state"))
            .unwrap_or_default();

        let state = ViewerState::from_persistence(
            &persistence,
            config.ocio.clone(),
            config.display.as_deref(),
            config.view.as_deref(),
        );

        let app = Self {
            tx: tx_to_worker,
            rx: rx_from_worker,
            worker: Some(worker),
            texture: None,
            error: None,
            state,
            generation: 0,
            cli_ocio: config.ocio,
            cli_display: config.display,
            cli_view: config.view,
            cli_colorspace: config.colorspace.clone(),
        };

        // Send initial config
        if let Some(ref ocio) = app.cli_ocio {
            app.send(ViewerMsg::SetOcioConfig(Some(ocio.clone())));
        } else {
            app.send(ViewerMsg::SetOcioConfig(None));
        }

        // Override colorspace if specified
        if let Some(ref cs) = config.colorspace {
            app.send(ViewerMsg::SetInputColorspace(cs.clone()));
        }

        // Load the image if provided
        if let Some(path) = image_path {
            app.send(ViewerMsg::LoadImage(path));
        }

        app
    }

    /// Open file dialog and load selected image.
    fn open_file_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Images", &["exr", "hdr", "png", "jpg", "jpeg", "tif", "tiff", "dpx"])
            .add_filter("All files", &["*"])
            .pick_file()
        {
            self.send(ViewerMsg::LoadImage(path));
        }
    }

    fn send(&self, msg: ViewerMsg) {
        let _ = self.tx.send(msg);
    }

    fn send_regen(&mut self, msg: ViewerMsg) {
        self.generation += 1;
        self.send(ViewerMsg::SyncGeneration(self.generation));
        self.send(msg);
    }

    /// Process all pending events from worker. Returns true if any events were processed.
    fn process_events(&mut self, ctx: &egui::Context) -> bool {
        let mut had_events = false;
        while let Ok(event) = self.rx.try_recv() {
            had_events = true;
            match event {
                ViewerEvent::ImageLoaded { dims, layers, colorspace, path } => {
                    self.state.image_dims = Some(dims);
                    self.state.image_path = Some(path.clone());
                    self.state.layers = layers;
                    
                    // Update window title
                    let title = format!("vfx view - {}", path.file_name().and_then(|n| n.to_str()).unwrap_or("Image"));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Title(title));
                    if let Some(cs) = colorspace
                        && self.cli_colorspace.is_none()
                    {
                        self.state.input_colorspace = cs;
                    }
                    self.error = None;
                }
                ViewerEvent::OcioConfigLoaded { displays, default_display, colorspaces } => {
                    self.state.displays = displays;
                    self.state.colorspaces = colorspaces;
                    
                    // Apply CLI override or use default
                    if let Some(ref d) = self.cli_display {
                        self.state.display = d.clone();
                    } else if self.state.display.is_empty() {
                        self.state.display = default_display;
                    }
                }
                ViewerEvent::DisplayChanged { views, default_view } => {
                    self.state.views = views;
                    
                    // Apply CLI override or use default
                    if let Some(ref v) = self.cli_view {
                        self.state.view = v.clone();
                    } else if self.state.view.is_empty() {
                        self.state.view = default_view;
                    }
                }
                ViewerEvent::TextureReady { generation, width, height, pixels } => {
                    if generation < self.generation {
                        continue; // Stale result
                    }
                    let image = ColorImage {
                        size: [width as usize, height as usize],
                        pixels,
                    };
                    self.texture = Some(ctx.load_texture(
                        "viewer_image",
                        image,
                        TextureOptions::LINEAR,
                    ));
                }
                ViewerEvent::StateSync { zoom, pan } => {
                    self.state.zoom = zoom;
                    self.state.pan = pan;
                }
                ViewerEvent::Error(msg) => {
                    self.error = Some(msg);
                }
                ViewerEvent::PixelValue { x, y, rgba } => {
                    self.state.cursor_pixel = Some((x, y));
                    self.state.cursor_color = Some(rgba);
                }
            }
        }
        had_events
    }

    /// Handle keyboard input. Returns true if should exit.
    fn handle_input(&mut self, ctx: &egui::Context) -> bool {
        let mut exit = false;
        let mut open_file = false;

        ctx.input(|i| {
            if i.key_pressed(egui::Key::Escape) {
                exit = true;
            }

            // File operations
            if i.key_pressed(egui::Key::O) && !i.modifiers.ctrl {
                open_file = true;
            }

            // View controls
            if i.key_pressed(egui::Key::F) {
                self.send(ViewerMsg::FitToWindow);
            }
            if i.key_pressed(egui::Key::H) || i.key_pressed(egui::Key::Num0) {
                self.send(ViewerMsg::Home);
            }

            // Zoom
            if i.key_pressed(egui::Key::Plus) || i.key_pressed(egui::Key::Equals) {
                self.send(ViewerMsg::Zoom { factor: 0.2, center: [0.0, 0.0] });
            }
            if i.key_pressed(egui::Key::Minus) {
                self.send(ViewerMsg::Zoom { factor: -0.2, center: [0.0, 0.0] });
            }

            // Channel modes (avoid Ctrl+key)
            if i.key_pressed(egui::Key::R) && !i.modifiers.ctrl {
                self.state.channel_mode = ChannelMode::Red;
                self.send_regen(ViewerMsg::SetChannelMode(ChannelMode::Red));
            }
            if i.key_pressed(egui::Key::G) && !i.modifiers.ctrl {
                self.state.channel_mode = ChannelMode::Green;
                self.send_regen(ViewerMsg::SetChannelMode(ChannelMode::Green));
            }
            if i.key_pressed(egui::Key::B) && !i.modifiers.ctrl {
                self.state.channel_mode = ChannelMode::Blue;
                self.send_regen(ViewerMsg::SetChannelMode(ChannelMode::Blue));
            }
            if i.key_pressed(egui::Key::A) && !i.modifiers.ctrl {
                self.state.channel_mode = ChannelMode::Alpha;
                self.send_regen(ViewerMsg::SetChannelMode(ChannelMode::Alpha));
            }
            if i.key_pressed(egui::Key::C) && !i.modifiers.ctrl {
                self.state.channel_mode = ChannelMode::Color;
                self.send_regen(ViewerMsg::SetChannelMode(ChannelMode::Color));
            }
            if i.key_pressed(egui::Key::L) {
                self.state.channel_mode = ChannelMode::Luminance;
                self.send_regen(ViewerMsg::SetChannelMode(ChannelMode::Luminance));
            }

            // Copy pixel value to clipboard
            if i.key_pressed(egui::Key::P) {
                if let Some(rgba) = self.state.cursor_color {
                    let text = format!("{:.4}, {:.4}, {:.4}, {:.3}", rgba[0], rgba[1], rgba[2], rgba[3]);
                    ctx.output_mut(|o| o.copied_text = text);
                }
            }

            // Scroll zoom - use mouse position for zoom-to-cursor
            if i.raw_scroll_delta.y != 0.0 {
                // Get mouse position relative to viewport center
                let mouse_pos = i.pointer.hover_pos().unwrap_or_default();
                let vp_center = egui::pos2(
                    self.state.viewport_size[0] / 2.0,
                    self.state.viewport_size[1] / 2.0,
                );
                // Offset from center in screen pixels
                let center = [
                    mouse_pos.x - vp_center.x,
                    mouse_pos.y - vp_center.y,
                ];
                self.send(ViewerMsg::Zoom {
                    factor: i.raw_scroll_delta.y * 0.002,
                    center,
                });
            }
        });

        // Handle actions outside input closure
        if open_file {
            self.open_file_dialog();
        }

        exit
    }

    /// Draw top control panel.
    fn draw_controls(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("controls").show(ctx, |ui| {
            ui.horizontal(|ui| {
                // Src: Input colorspace (what colorspace the file is in)
                ui.label("Src:").on_hover_text(
                    "Source Colorspace\n\n\
                    The colorspace your image file is encoded in.\n\
                    Examples: ACEScg, Linear sRGB, sRGB, Raw...\n\n\
                    'Auto' tries to detect from file metadata.\n\
                    Wrong setting = wrong colors!"
                );
                egui::ComboBox::from_id_salt("src_cs")
                    .width(120.0)
                    .selected_text(if self.state.input_colorspace.is_empty() {
                        "Auto"
                    } else {
                        &self.state.input_colorspace
                    })
                    .show_ui(ui, |ui| {
                        for cs in self.state.colorspaces.clone() {
                            if ui.selectable_value(&mut self.state.input_colorspace, cs.clone(), &cs).changed() {
                                self.send_regen(ViewerMsg::SetInputColorspace(cs));
                            }
                        }
                    });

                ui.separator();

                // RRT: View transform (Reference Rendering Transform)
                ui.label("RRT:").on_hover_text(
                    "View Transform (RRT)\n\n\
                    Converts scene-linear data to display-ready image.\n\
                    This is the 'look' or 'tone mapping'.\n\n\
                    Examples:\n\
                    - ACES 1.0 SDR: Film-like, industry standard\n\
                    - Filmic: Softer highlights\n\
                    - Raw: No transform (for technical checks)"
                );
                egui::ComboBox::from_id_salt("rrt_view")
                    .width(140.0)
                    .selected_text(&self.state.view)
                    .show_ui(ui, |ui| {
                        for view in self.state.views.clone() {
                            if ui.selectable_value(&mut self.state.view, view.clone(), &view).changed() {
                                self.send_regen(ViewerMsg::SetView(view));
                            }
                        }
                    });

                // ODT: Output Device Transform (display/monitor)
                ui.label("ODT:").on_hover_text(
                    "Output Device Transform (ODT)\n\n\
                    Target display/monitor type.\n\
                    Adapts image for your screen's capabilities.\n\n\
                    Examples:\n\
                    - sRGB: Standard monitors\n\
                    - Rec.709: TV/broadcast\n\
                    - P3-D65: Wide gamut (Mac, cinema)\n\
                    - Rec.2020: HDR displays"
                );
                egui::ComboBox::from_id_salt("odt_display")
                    .width(100.0)
                    .selected_text(&self.state.display)
                    .show_ui(ui, |ui| {
                        for display in self.state.displays.clone() {
                            if ui.selectable_value(&mut self.state.display, display.clone(), &display).changed() {
                                self.send_regen(ViewerMsg::SetDisplay(display));
                            }
                        }
                    });

                ui.separator();

                // Exposure slider with Ctrl+click reset
                ui.label("EV:").on_hover_text(
                    "Exposure Value\n\n\
                    Adjusts image brightness in stops.\n\
                    +1 = 2x brighter, -1 = 2x darker\n\n\
                    Ctrl+Click to reset to 0"
                );
                let exp_slider = ui.add(
                    egui::Slider::new(&mut self.state.exposure, -10.0..=10.0)
                        .step_by(0.1)
                        .fixed_decimals(1)
                );
                // Ctrl+click resets to default
                if exp_slider.clicked() && ui.input(|i| i.modifiers.ctrl) {
                    self.state.exposure = DEFAULT_EXPOSURE;
                    self.send_regen(ViewerMsg::SetExposure(DEFAULT_EXPOSURE));
                } else if exp_slider.changed() {
                    self.send_regen(ViewerMsg::SetExposure(self.state.exposure));
                }
            });

            // Second row
            ui.horizontal(|ui| {
                // Layer selector (if multiple)
                if self.state.layers.len() > 1 {
                    ui.label("Layer:").on_hover_text(
                        "EXR Layer\n\n\
                        Multi-layer EXR files contain separate\n\
                        render passes (diffuse, specular, etc.)\n\n\
                        Select which layer to view."
                    );
                    egui::ComboBox::from_id_salt("layer_sel")
                        .width(120.0)
                        .selected_text(&self.state.layer)
                        .show_ui(ui, |ui| {
                            for layer in self.state.layers.clone() {
                                if ui.selectable_value(&mut self.state.layer, layer.clone(), &layer).changed() {
                                    self.send_regen(ViewerMsg::SetLayer(layer));
                                }
                            }
                        });
                    ui.separator();
                }

                // Channel mode selector
                ui.label("Ch:").on_hover_text(
                    "Channel Display\n\n\
                    View individual color channels:\n\
                    - Color (C): Full RGB\n\
                    - Red (R), Green (G), Blue (B): Single channel\n\
                    - Alpha (A): Transparency mask\n\
                    - Luma (L): Brightness only\n\n\
                    Hotkeys: R, G, B, A, C, L"
                );
                egui::ComboBox::from_id_salt("channel_sel")
                    .width(90.0)
                    .selected_text(self.state.channel_mode.label())
                    .show_ui(ui, |ui| {
                        for &mode in ChannelMode::all() {
                            let label = format!("{} ({})", mode.label(), mode.shortcut());
                            if ui.selectable_value(&mut self.state.channel_mode, mode, label).changed() {
                                self.send_regen(ViewerMsg::SetChannelMode(mode));
                            }
                        }
                    });

                ui.separator();

                // Zoom percentage
                ui.label(format!("{}%", (self.state.zoom * 100.0) as i32));

                // Image dimensions
                if let Some((w, h)) = self.state.image_dims {
                    ui.separator();
                    ui.label(format!("{w}x{h}"));
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Refresh").clicked() {
                        self.send(ViewerMsg::Regenerate);
                    }
                    if ui.button("Open").clicked() {
                        self.open_file_dialog();
                    }
                });
            });
        });
    }

    /// Draw bottom hints panel.
    fn draw_hints(&self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("hints").show(ctx, |ui| {
            ui.horizontal(|ui| {
                // Pixel inspector info
                if let (Some((x, y)), Some(rgba)) = (self.state.cursor_pixel, self.state.cursor_color) {
                    ui.monospace(format!(
                        "[{:4},{:4}] R:{:7.4} G:{:7.4} B:{:7.4} A:{:5.3}",
                        x, y, rgba[0], rgba[1], rgba[2], rgba[3]
                    ));
                    ui.separator();
                }

                ui.label("O: Open | F: Fit | H: Home | +/-: Zoom | R/G/B/A/C/L: Channels | P: Pick | Esc: Exit");
            });
        });
    }

    /// Draw main canvas with image.
    fn draw_canvas(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let available = ui.available_size();

            // Track viewport size changes
            if (self.state.viewport_size[0] - available.x).abs() > 1.0
                || (self.state.viewport_size[1] - available.y).abs() > 1.0
            {
                self.state.viewport_size = [available.x, available.y];
                self.send(ViewerMsg::SetViewport(self.state.viewport_size));
            }

            // Show error if any
            if let Some(ref err) = self.error {
                ui.centered_and_justified(|ui| {
                    ui.colored_label(Color32::RED, err);
                });
                return;
            }

            // Draw image if texture available
            if let Some(ref texture) = self.texture {
                let tex_size = texture.size_vec2();
                let scaled_size = tex_size * self.state.zoom;

                // Center image with pan offset
                let center = available / 2.0;
                let pan_offset = Vec2::new(
                    self.state.pan[0] * self.state.zoom,
                    self.state.pan[1] * self.state.zoom,
                );
                let top_left = center - scaled_size / 2.0 + pan_offset;

                // Allocate space and handle drag
                let (rect, response) = ui.allocate_exact_size(available, egui::Sense::click_and_drag());

                if response.dragged() {
                    let delta = response.drag_delta();
                    self.send(ViewerMsg::Pan { delta: [delta.x, delta.y] });
                }

                if response.double_clicked() {
                    self.send(ViewerMsg::FitToWindow);
                }

                // Track hover for pixel inspector
                if let Some(hover_pos) = response.hover_pos() {
                    // Convert screen position to image coordinates
                    let img_pos = hover_pos - rect.min - top_left.to_pos2().to_vec2();
                    let img_x = (img_pos.x / self.state.zoom) as i32;
                    let img_y = (img_pos.y / self.state.zoom) as i32;

                    if let Some((w, h)) = self.state.image_dims {
                        if img_x >= 0 && img_y >= 0 && (img_x as u32) < w && (img_y as u32) < h {
                            self.send(ViewerMsg::QueryPixel {
                                x: img_x as u32,
                                y: img_y as u32,
                            });
                        } else {
                            self.state.cursor_pixel = None;
                            self.state.cursor_color = None;
                        }
                    }
                } else {
                    self.state.cursor_pixel = None;
                    self.state.cursor_color = None;
                }

                // Paint the image
                let painter = ui.painter_at(rect);
                let image_rect = egui::Rect::from_min_size(
                    rect.min + top_left.to_pos2().to_vec2(),
                    scaled_size,
                );
                painter.image(
                    texture.id(),
                    image_rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    Color32::WHITE,
                );
            } else {
                // No image - allocate clickable area first, then draw hint text
                let (rect, response) = ui.allocate_exact_size(available, egui::Sense::click());
                
                // Draw centered hint text
                let painter = ui.painter_at(rect);
                painter.text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "Double-click or drag file to open",
                    egui::FontId::default(),
                    ui.visuals().text_color(),
                );

                if response.double_clicked() {
                    self.open_file_dialog();
                }
            }
        });
    }

    /// Check for dropped files.
    fn handle_dropped_files(&mut self, ctx: &egui::Context) {
        ctx.input(|i| {
            if !i.raw.dropped_files.is_empty()
                && let Some(path) = i.raw.dropped_files.first().and_then(|f| f.path.clone())
            {
                self.send(ViewerMsg::LoadImage(path));
            }
        });
    }
}

impl eframe::App for ViewerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Process worker events - request repaint if any events received
        let had_events = self.process_events(ctx);

        // Handle dropped files
        self.handle_dropped_files(ctx);

        // Handle input
        if self.handle_input(ctx) {
            self.send(ViewerMsg::Close);
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }

        // Draw UI
        self.draw_controls(ctx);
        self.draw_hints(ctx);
        self.draw_canvas(ctx);

        // Repaint when needed (events pending, dragging, or hovering image for pixel inspector)
        if had_events || self.state.cursor_pixel.is_some() {
            ctx.request_repaint();
        }
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        let persistence = self.state.to_persistence();
        eframe::set_value(storage, "vfx_viewer_state", &persistence);
    }
}

impl Drop for ViewerApp {
    fn drop(&mut self) {
        // Signal worker to stop
        let _ = self.tx.send(ViewerMsg::Close);
        
        // Wait for worker thread to finish
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
