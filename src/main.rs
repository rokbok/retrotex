
use eframe::egui;

use crate::{definition::TextureDefinition, util::quick_hash};

#[allow(unused_imports)]
use log::{debug, error, log_enabled, info, warn, trace};

pub mod definition;
pub mod definiton_ui;
pub mod files;
pub mod util;

struct ExampleApp {
    def: definition::TextureDefinition,
    tmp_str: String,
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
        };

        if save {
            ret.save_current();
        }
        ret
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

        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }

        ctx.set_pixels_per_point(1.5);

        egui::CentralPanel::default().show(ctx, |ui| {
            let old_hash = quick_hash(&self.def);
            definiton_ui::definition_ui(&mut self.def, &mut self.tmp_str, ui);
            let new_hash = quick_hash(&self.def);
            if old_hash != new_hash {
                self.save_current();
                trace!("Saved {}", self.def.name);
            }
        });
    }
}

fn main() {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size((800.0, 600.0)),
        ..eframe::NativeOptions::default()
    };

    eframe::run_native(
        ExampleApp::name(),
        native_options,
        Box::new(|_| Ok(Box::new(ExampleApp::new()))),
    ).expect("Error running app")
}
