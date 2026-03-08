
use std::time::{Duration, Instant};

use eframe::egui;
use egui::{Color32, ColorImage, TextureHandle};

use crate::{load_save_undo::LoadSaveUndo, util::quick_hash};

#[allow(unused_imports)]
use log::{debug, error, log_enabled, info, warn, trace};

pub mod definition;
pub mod definition_ui;
pub mod load_save_undo;
pub mod util;
pub mod noise;
pub mod color;

pub const IMG_SIZE: i32 = 256;
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

struct ExampleApp {
    def: definition::TextureDefinition,
    tmp_str: String,
    img_texture: Option<TextureHandle>,
    auto_save_at: Option<Instant>,
    load_save_undo: LoadSaveUndo,
    clipboard: arboard::Clipboard,
}

impl ExampleApp {
    fn new() -> Self {
        let mut load_save_undo = LoadSaveUndo::new();
        let def = load_save_undo.load_by_name_or_create(definition::DEFAULT_NAME);

        let ret = Self {
            def,
            tmp_str: String::new(),
            img_texture: None,
            auto_save_at: None,
            load_save_undo,
            clipboard: arboard::Clipboard::new().expect("Failed to initialize clipboard"),
        };

        ret
    }

    fn regenerate(&mut self, ctx: Option<&egui::Context>) {
        let tex_available = self.img_texture.is_some() || ctx.is_some();
        if !tex_available {
            return;

        }
        let mut img = ColorImage::filled([IMG_SIZE as usize, IMG_SIZE as usize], Color32::BLACK);
        for y in 0..IMG_SIZE {
            for x in 0..IMG_SIZE {
                let c = self.def.generate_pixel(x, y);
                img.pixels[idx(x, y)] = egui::Rgba::from_rgba_unmultiplied(c.x.clamp(0.0, 1.0), c.y.clamp(0.0, 1.0), c.z.clamp(0.0, 1.0), 1.0).into();
            }
        }
        if let Some(tex) = &mut self.img_texture {
            tex.set(img, PREVIEW_TEX_OPTIONS);
        } else if let Some(ctx) = ctx {
            self.img_texture = Some(ctx.load_texture("preview", img, PREVIEW_TEX_OPTIONS));
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
    }
}

impl eframe::App for ExampleApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut regen_needed = self.img_texture.is_none();
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
            self.img_texture.as_ref().expect("Texture not initialized yet?");
            if let Some(tex) = &self.img_texture {
                let available = ui.available_size();
                let mnsz = available.x.min(available.y);
                let iscale = (mnsz / (IMG_SIZE as f32)).floor().max(1.0) as i32;
                let display_size = IMG_SIZE as f32 * iscale as f32;
                ui.add_sized(available, egui::Image::new(tex).fit_to_exact_size(egui::Vec2::new(display_size, display_size)));
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

fn main() {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size((1600.0, 900.0)),
        ..eframe::NativeOptions::default()
    };

    eframe::run_native(
        ExampleApp::name(),
        native_options,
        Box::new(|_| Ok(Box::new(ExampleApp::new()))),
    ).expect("Error running app")
}
