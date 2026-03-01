
use eframe::egui;
use egui::{Color32, ColorImage, TextureHandle};

use crate::{definition::TextureDefinition, util::quick_hash};

#[allow(unused_imports)]
use log::{debug, error, log_enabled, info, warn, trace};

pub mod definition;
pub mod definiton_ui;
pub mod files;
pub mod util;

pub const IMG_SIZE: i32 = 256;

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
    img_t: Option<TextureHandle>,
}

impl ExampleApp {
    fn new() -> Self {
        let loaded_def = files::load_by_name(definition::DEFAULT_NAME);
        let mut save = false;
        match &loaded_def {
            Ok(def) => info!("Loaded texture definition: {}", def.name),
            Err(e) => {
                save = true;
                warn!("Failed to load texture definition, creating default: {}", e)
            },
        }

        let ret = Self {
            def: loaded_def.unwrap_or_else(| _ | TextureDefinition::default()),
            tmp_str: String::new(),
            img_t: None,
        };

        if save {
            ret.save_current();
        }

        ret
    }

    fn regenerate(&mut self, ctx: Option<&egui::Context>) {
        let tex_available = self.img_t.is_some() || ctx.is_some();
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
        if let Some(tex) = &mut self.img_t {
            tex.set(img, PREVIEW_TEX_OPTIONS);
        } else if let Some(ctx) = ctx {
            self.img_t = Some(ctx.load_texture("preview", img, PREVIEW_TEX_OPTIONS));
        }
    }

    fn name() -> &'static str {
        "retrotex"
    }

    fn save_current(&self) {
        if let Err(e) = files::save(&self.def) {
            error!("Failed to save texture {}: {}", self.def.name, e);
        } else {
            info!("Texture {} saved successfully", self.def.name);
        }
    }
}

impl eframe::App for ExampleApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {

        if ctx.input(|i| i.key_pressed(egui::Key::F10)) {
            self.save_current();
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }

        ctx.set_pixels_per_point(1.5);

        egui::SidePanel::right("right_panel").show(ctx, |ui| {
            let old_hash = quick_hash(&self.def);
            definiton_ui::definition_ui(&mut self.def, &mut self.tmp_str, ui);
            let new_hash = quick_hash(&self.def);
            let changed = old_hash != new_hash;
            if changed {
                self.save_current();
                trace!("Saved {}", self.def.name);
            }
            if changed || self.img_t.is_none() {
                self.regenerate(Some(ctx));
            }
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            self.img_t.as_ref().expect("Texture not initialized yet?");
            if let Some(tex) = &self.img_t {
                let available = ui.available_size();
                let mnsz = available.x.min(available.y);
                let iscale = (mnsz / (IMG_SIZE as f32)).floor().max(1.0) as i32;
                let display_size = IMG_SIZE as f32 * iscale as f32;
                ui.add_sized(available, egui::Image::new(tex).fit_to_exact_size(egui::Vec2::new(display_size, display_size)));
            }
        });
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
