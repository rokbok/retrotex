
use std::{fmt::Write as _, time::{Duration, Instant}};

use clap::Parser as _;
use eframe::egui;
use egui::{Color32, ColorImage, TextureHandle};
use glam::FloatExt as _;
use strum_macros::{AsRefStr, EnumString, VariantNames};

use crate::{load_save_undo::LoadSaveUndo, processing::TextureLayers, util::{add_enum_dropdown, quick_hash}};

#[allow(unused_imports)]
use log::{debug, error, log_enabled, info, warn, trace};

pub mod definition;
pub mod definition_ui;
pub mod load_save_undo;
pub mod util;
pub mod noise;
pub mod color;
pub mod processing;

pub const IMG_SIZE: i32 = 128;
pub const IMG_PIXEL_COUNT: usize = IMG_SIZE as usize * IMG_SIZE as usize;
const AUTO_SAVE_DELAY_MILLIS: u64 = 200;

const PREVIEW_TEX_OPTIONS: egui::TextureOptions = egui::TextureOptions {
    magnification: egui::TextureFilter::Nearest,
    minification: egui::TextureFilter::Nearest,
    wrap_mode: egui::TextureWrapMode::ClampToEdge,
    mipmap_mode: None,
};

pub fn idx(x: i32, y: i32) -> usize {
    (y * IMG_SIZE + x) as usize
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Default, AsRefStr, EnumString, VariantNames)]
enum DisplayMode { 
    #[default]
    Albedo,
    Depth,
    Normal,
}

#[derive(Debug, Default)]
struct DisplaySettings {
    mode: DisplayMode,
}


struct TextureHandleSet {
    albedo: TextureHandle,
    depth: TextureHandle,
    normal: TextureHandle,
}

struct ExampleApp {
    def: definition::TextureDefinition,
    tmp_str: String,
    textures: Option<TextureHandleSet>,
    layers: TextureLayers,
    auto_save_at: Option<Instant>,
    load_save_undo: LoadSaveUndo,
    clipboard: arboard::Clipboard,
    display_settings: DisplaySettings,
    output_dir: String,
    initial_generation_done: bool,
}

impl ExampleApp {
    fn new(output_dir: String) -> Self {
        let mut load_save_undo = LoadSaveUndo::new();
        let def = load_save_undo.load_by_name_or_create(definition::DEFAULT_NAME);

        let ret = Self {
            def,
            tmp_str: String::new(),
            textures: None,
            layers: TextureLayers::default(),
            auto_save_at: None,
            load_save_undo,
            clipboard: arboard::Clipboard::new().expect("Failed to initialize clipboard"),
            display_settings: DisplaySettings::default(),
            output_dir,
            initial_generation_done: false,
        };

        ret
    }

    fn regenerate(&mut self, ctx: Option<&egui::Context>) {
        let tex_available = self.textures.is_some() || ctx.is_some();
        if !tex_available {
            return;

        }
        let mut albedo_img = ColorImage::filled([IMG_SIZE as usize, IMG_SIZE as usize], Color32::MAGENTA);
        let mut depth_img = ColorImage::filled([IMG_SIZE as usize, IMG_SIZE as usize], Color32::MAGENTA);
        let mut normal_img = ColorImage::filled([IMG_SIZE as usize, IMG_SIZE as usize], Color32::MAGENTA);

        for y in 0..IMG_SIZE {
            for x in 0..IMG_SIZE {
                let s = self.def.generate_pixel(x, y);

                albedo_img.pixels[idx(x, y)] = egui::Rgba::from_rgba_unmultiplied(s.albedo.x.clamp(0.0, 1.0), s.albedo.y.clamp(0.0, 1.0), s.albedo.z.clamp(0.0, 1.0), 1.0).into();

                let d = (s.depth + 128.0).round().clamp(0.0, 255.0) as u8;
                depth_img.pixels[idx(x, y)] = egui::Rgba::from_srgba_unmultiplied(d, d, d, 255).into();

                self.layers.albedo[idx(x, y)] = s.albedo;
                self.layers.depth[idx(x, y)] = s.depth;
            }
        }

        self.layers.recalculate();

        for y in 0..IMG_SIZE {
            for x in 0..IMG_SIZE {
                let n = self.layers.normal[idx(x, y)];
                normal_img.pixels[idx(x, y)] = egui::Rgba::from_rgba_unmultiplied(n.x.mul_add(0.5, 0.5).saturate(), n.y.mul_add(0.5, 0.5).saturate(), n.z.saturate(), 1.0).into();
            }
        }

        if !self.initial_generation_done {
            info!("Writing initial output images for texture {}...", self.def.name);
            load_save_undo::write_images(&self.layers, &self.output_dir, &self.def.name).unwrap_or_else(|e| error!("Failed to write initial output images: {}", e));
            self.initial_generation_done = true;
        }

        if let Some(tex) = &mut self.textures {
            tex.albedo.set(albedo_img, PREVIEW_TEX_OPTIONS);
            tex.depth.set(depth_img, PREVIEW_TEX_OPTIONS);
            tex.normal.set(normal_img, PREVIEW_TEX_OPTIONS);

            // Data already was written into the correct place
        } else if let Some(ctx) = ctx {
            self.textures = Some(TextureHandleSet {
                albedo: ctx.load_texture("preview_albedo", albedo_img, PREVIEW_TEX_OPTIONS),
                depth: ctx.load_texture("preview_depth", depth_img, PREVIEW_TEX_OPTIONS),
                normal: ctx.load_texture("preview_normal", normal_img, PREVIEW_TEX_OPTIONS),
            });
        }
    }

    fn name() -> &'static str {
        "retrotex"
    }

    fn save_current(&mut self) {
        if let Err(e) = self.load_save_undo.save(&self.def) {
            error!("Failed to save texture {}: {}", self.def.name, e);
        } else {
            info!("Texture {} saved successfully", self.def.name);
        }

        if let Err(e) = load_save_undo::write_images(&self.layers, &self.output_dir, &self.def.name) {
            error!("Failed to write output images for texture {}: {}", self.def.name, e);
        } else {
            info!("Output images for texture {} written successfully", self.def.name);
        }
    }
}

impl eframe::App for ExampleApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut regen_needed = self.textures.is_none();
        let closing = ctx.input(|i| {
            if i.key_pressed(egui::Key::F10) {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            if i.key_pressed(egui::Key::Z) && i.modifiers.ctrl {
                if let Some(undo_def) = self.load_save_undo.undo() {
                    self.def = undo_def;
                    regen_needed = true;
                }
            }
            if i.key_pressed(egui::Key::Y) && i.modifiers.ctrl {
                if let Some(redo_def) = self.load_save_undo.redo() {
                    self.def = redo_def;
                    regen_needed = true;
                }
            }
            i.viewport().close_requested()
        });

        ctx.set_pixels_per_point(1.5);

 
        egui::SidePanel::right("right_panel")
            .default_width(400.0)
            .show(ctx, |ui| {
                let old_hash = quick_hash(&self.def);
                definition_ui::definition_ui(&mut self.def, &mut self.tmp_str, ui, &mut self.clipboard);
                let new_hash = quick_hash(&self.def);
                let changed = old_hash != new_hash;
                if changed {
                    self.auto_save_at = Some(Instant::now() + Duration::from_millis(AUTO_SAVE_DELAY_MILLIS));
                }

                regen_needed |= changed;
            });

        if regen_needed {
            self.regenerate(Some(ctx));
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(| ui | {
                ui.label("Display Mode:");
                add_enum_dropdown(ui, &mut self.display_settings.mode, "display_mode", 0, false);
            });
            self.textures.as_ref().expect("Texture not initialized yet?");
            if let Some(tex) = &self.textures {
                ui.centered_and_justified(| ui | {
                    let available = ui.available_rect_before_wrap();
                    let mnsz = available.width().min(available.height());
                    let iscale = (mnsz / (IMG_SIZE as f32)).floor().max(1.0) as i32;
                    let display_size = IMG_SIZE as f32 * iscale as f32;
                    let tex = match self.display_settings.mode {
                        DisplayMode::Albedo => &tex.albedo,
                        DisplayMode::Depth => &tex.depth,
                        DisplayMode::Normal => &tex.normal,
                    };
                    let sz = egui::Vec2::new(display_size, display_size);
                    let rect = egui::Rect::from_center_size(available.center(), sz);
                    let resp = ui.allocate_rect(rect, egui::Sense::hover());
                    let img = egui::Image::new(tex)
                        .fit_to_exact_size(sz)
                        .sense(egui::Sense::hover());
                    img.paint_at(ui, rect);
                    if let Some(hover_pos) = resp.hover_pos() {
                        resp.on_hover_ui_at_pointer(| ui | {
                            let x = ((hover_pos.x - rect.min.x) / iscale as f32).floor() as i32;
                            let y = ((hover_pos.y - rect.min.y) / iscale as f32).floor() as i32;
                            write!(self.tmp_str, "Hovering at ({:.1}, {:.1})", x, y).unwrap();
                            if x >= 0 && x < IMG_SIZE && y >= 0 && y < IMG_SIZE {
                                self.tmp_str.clear();
                                let index = idx(x, y);
                                write!(self.tmp_str, "Pixel ({}, {})", x, y).unwrap();

                                let albedo = self.layers.albedo[index];
                                write!(self.tmp_str, "\nAlbedo: ({:.3}, {:.3}, {:.3})", albedo.x, albedo.y, albedo.z).unwrap();

                                let depth = self.layers.depth[index];
                                write!(self.tmp_str, "\nDepth: {:.3}", depth).unwrap();

                                let normal = self.layers.normal[index];
                                write!(self.tmp_str, "\nNormal: ({:.3}, {:.3}, {:.3})", normal.x, normal.y, normal.z).unwrap();

                                ui.label(&self.tmp_str);
                            } else {
                                ui.label("Outside");
                            }
                        });
                    }
                });
            }
        });

        if let Some(inst) = self.auto_save_at {
            if closing || Instant::now() >= inst {
                self.save_current();
                self.auto_save_at = None;
            } else {
                ctx.request_repaint(); // Keep updating until we save
            }
        }
    }
}

#[derive(clap::Parser)]
struct CommandLineArgs {
    #[arg(short, long)]
    output: Option<String>,
}



fn main() {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    let args = CommandLineArgs::parse();
    let output = args.output.unwrap_or_else(|| "output".to_string());
    info!("Using output directory: {}", output);

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size((1600.0, 900.0)),
        ..eframe::NativeOptions::default()
    };

    eframe::run_native(
        ExampleApp::name(),
        native_options,
        Box::new(|_| Ok(Box::new(ExampleApp::new(output)))),
    ).expect("Error running app")
}
