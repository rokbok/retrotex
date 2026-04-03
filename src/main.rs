use std::{hash::Hash, time::Instant};

use clap::{Parser as _};
use eframe::egui;
use egui::TextureHandle;
use strum_macros::{AsRefStr, EnumString, VariantNames};

use crate::prelude::*;
use crate::file_ui::show_file_list_panel;
use crate::logs::{LogQueue, LogOverlay};
use crate::settings::Settings;
use crate::storage::FileRegistry;
use crate::{definition::TextureDefinition, preview_ui::OngoingDrag};

pub mod prelude;
pub mod definition;
pub mod definition_ui;
pub mod preview_ui;
pub mod file;
pub mod util;
pub mod noise;
pub mod color;
pub mod processing;
pub mod storage;
pub mod file_ui;
pub mod settings;
pub mod logs;

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
    rename_dialog_open: bool,
    rename_just_opened: bool,
    rename_input: String,
    rename_pending: Option<String>,
}

struct RetroTexApp {
    tmp_str: String,
    file_registry: FileRegistry,
    settings: Settings,
    file_id: u128,
    last_unsaved_change: Instant,
    output_dir: String,
    ui_data: UiData,
    log_overlay: LogOverlay,
}

impl RetroTexApp {
    fn new(output_dir: String, log_entries: LogQueue) -> Self {
        let mut file_registry = FileRegistry::read();
        let mut settings = Settings::load();
        let saved_file_id = settings.last_opened_id;
        if !file_registry.file_by_id(saved_file_id).is_some() {
            let id = file_registry
                .id_by_name(file::DEFAULT_NAME)
                .unwrap_or_else(|| file_registry.create(file::DEFAULT_NAME, TextureDefinition::demo()));
            settings.last_opened_id = id;
        }

        let file_id = settings.last_opened_id;

        let ret = Self {
            tmp_str: String::new(),
            file_registry,
            settings,
            file_id,
            last_unsaved_change: Instant::now(),
            output_dir,
            ui_data: UiData {
                drag: OngoingDrag::None,
                preview_editing: None,
                display_mode: DisplayMode::Lit,
                rename_dialog_open: false,
                rename_just_opened: false,
                rename_input: String::new(),
                rename_pending: None,
            },
            log_overlay: LogOverlay::new(log_entries),
        };

        ret
    }

    fn name() -> &'static str {
        "retrotex"
    }
}

impl eframe::App for RetroTexApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        show_file_list_panel(ctx, &self.file_registry, &mut self.file_id);

        let file_ref = self.file_registry
            .file_by_id(self.file_id)
            .expect("Active file id not found in registry");

        let closing = ctx.input(|i| {
            if i.key_pressed(egui::Key::F10) {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            if i.key_pressed(egui::Key::Z) && i.modifiers.ctrl {
                file_ref.borrow_mut().undo();
            }
            if i.key_pressed(egui::Key::Y) && i.modifiers.ctrl {
                file_ref.borrow_mut().redo();
            }
            i.viewport().close_requested()
        });

        ctx.set_pixels_per_point(1.5);

        let changed = {
            let (mut file, ui_data) = (file_ref.borrow_mut(), &mut self.ui_data);
            file.update_images(ctx);
            file.update_layers();

            file.modify_definition(ctx, | def, images, layers, name | {
                egui::SidePanel::right("right_panel")
                    .default_width(400.0)
                    .show(ctx, |ui| {
                        def.definition_ui(ui, ui_data, name, &mut self.tmp_str);
                    });

                egui::CentralPanel::default().show(ctx, |ui| {
                    def.add_preview(ui, ui_data, images, layers, &mut self.tmp_str);
                });
            })
        };

        self.log_overlay.show(ctx);        

        let mut file_name_changed = false;
        if let Some(new_name) = self.ui_data.rename_pending.take() {
            if let Err(e) = file_ref.borrow_mut().rename(&new_name) {
                error!("Failed to rename file: {}", e);
            } else {
                file_name_changed = true;
            }
        }

        if changed {
            let mut file = file_ref.borrow_mut();
            self.last_unsaved_change = Instant::now();
            file.update_images(ctx);
            file.update_layers();
        }

        if file_ref.borrow().is_dirty() || file_name_changed {
            let mut file = file_ref.borrow_mut();
            ctx.request_repaint(); // Keep updating until we save
            if closing || self.last_unsaved_change.elapsed().as_millis() >= AUTO_SAVE_DELAY_MILLIS as u128 {
                file.save().unwrap_or_else(|e| error!("Failed to save texture {}: {}", file.name(), e));
                file.write_images(&self.output_dir).unwrap_or_else(|e| error!("Failed to write images for texture {}: {}", file.name(), e));
                assert!(!file.is_dirty(), "File should not be dirty after saving");
            }
        }
        
        self.settings.last_opened_id = self.file_id;
        self.settings.save_if_changed();

        if self.log_overlay.num_entries() > 0 {
            ctx.request_repaint();
        }
    }
}

#[derive(clap::Parser)]
struct CommandLineArgs {
    #[arg(short, long)]
    output: Option<String>,
}



fn main() {
    let log_entries = logs::init();

    let args = CommandLineArgs::parse();
    let output = args.output.unwrap_or_else(|| "output".to_string());
    debug!("Using output directory: {}", output);

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size((1600.0, 900.0)),
        ..eframe::NativeOptions::default()
    };

    eframe::run_native(
        RetroTexApp::name(),
        native_options,
        Box::new(| cc |  {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(RetroTexApp::new(output, log_entries)))
        }),
    ).expect("Error running app")
}
