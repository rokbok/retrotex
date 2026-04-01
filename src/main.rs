// TODO:
// - Transition serialized Colors in u8

use std::{cell::RefCell, hash::Hash, time::Instant};

use clap::{Parser as _};
use eframe::egui;
use egui::TextureHandle;
use strum_macros::{AsRefStr, EnumString, VariantNames};

use crate::prelude::*;
use crate::{load_save_undo::DefinitionFile, preview_ui::OngoingDrag, util::idx2coords};

pub mod prelude;
pub mod definition;
pub mod definition_ui;
pub mod preview_ui;
pub mod load_save_undo;
pub mod util;
pub mod noise;
pub mod color;
pub mod processing;

pub const IMG_SIZE: i32 = 128;
pub const IMG_PIXEL_COUNT: usize = IMG_SIZE as usize * IMG_SIZE as usize;
const AUTO_SAVE_DELAY_MILLIS: u64 = 200;


#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Default, AsRefStr, EnumString, VariantNames)]
enum DisplayMode { 
    #[default]
    Lit,
    Albedo,
    Depth,
    Normal,
    AmbientOcclusion,
}

pub(crate) struct TextureHandleSet {
    albedo: TextureHandle,
    depth: TextureHandle,
    normal: TextureHandle,
    ao: TextureHandle,
    lit: TextureHandle,
}

pub(crate) struct UiData {
    drag: OngoingDrag,
    preview_editing: Option<usize>,
    display_mode: DisplayMode,
}

struct RetroTexApp {
    tmp_str: String,
    file: RefCell<DefinitionFile>,
    last_unsaved_change: Instant,
    output_dir: String,
    ui_data: UiData,
}

impl RetroTexApp {
    fn new(output_dir: String) -> Self {
        let file = DefinitionFile::load_by_name_or_create(definition::DEFAULT_NAME);

        let ret = Self {
            tmp_str: String::new(),
            last_unsaved_change: Instant::now(),
            file: RefCell::new(file),
            output_dir,
            ui_data: UiData {
                drag: OngoingDrag::None,
                preview_editing: None,
                display_mode: DisplayMode::Lit,
            },
        };

        ret
    }

    fn name() -> &'static str {
        "retrotex"
    }
}

impl eframe::App for RetroTexApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let closing = ctx.input(|i| {
            if i.key_pressed(egui::Key::F10) {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            if i.key_pressed(egui::Key::Z) && i.modifiers.ctrl {
                self.file.borrow_mut().undo();
            }
            if i.key_pressed(egui::Key::Y) && i.modifiers.ctrl {
                self.file.borrow_mut().redo();
            }
            i.viewport().close_requested()
        });

        ctx.set_pixels_per_point(1.5);

        let (mut file, ui_data) = (self.file.borrow_mut(), &mut self.ui_data);
        file.update_images(ctx);
        file.update_layers();

        let changed = file.modify_definition(ctx, | def, images, layers, name | {
            egui::SidePanel::right("right_panel")
                .default_width(400.0)
                .show(ctx, |ui| {
                    def.definition_ui(ui, ui_data, name, &mut self.tmp_str);
                });

            egui::CentralPanel::default().show(ctx, |ui| {
                def.add_preview(ui, ui_data, images, layers, &mut self.tmp_str);
            });
        });

        if changed {
            self.last_unsaved_change = Instant::now();
            file.update_images(ctx);
            file.update_layers();
        }

        if file.is_dirty() {
            ctx.request_repaint(); // Keep updating until we save
            if closing || self.last_unsaved_change.elapsed().as_millis() >= AUTO_SAVE_DELAY_MILLIS as u128 {
                file.save().unwrap_or_else(|e| error!("Failed to save texture {}: {}", file.name(), e));
                file.write_images(&self.output_dir).unwrap_or_else(|e| error!("Failed to write images for texture {}: {}", file.name(), e));
                assert!(!file.is_dirty(), "File should not be dirty after saving");
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
        RetroTexApp::name(),
        native_options,
        Box::new(| cc |  {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(RetroTexApp::new(output)))
        }),
    ).expect("Error running app")
}
