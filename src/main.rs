use std::collections::HashMap;
use std::{hash::Hash, time::Instant};

use clap::{Parser as _};
use eframe::egui;
use egui::TextureHandle;
use strum_macros::{AsRefStr, EnumString, VariantNames};

use crate::file::FileId;
use crate::prelude::*;
use crate::file_ui::show_file_list_panel;
use crate::logs::{LogQueue, LogOverlay};
use crate::palettes::PaletteManager;
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
pub mod palettes;

pub const IMG_SIZE: i32 = 128;
pub const IMG_PIXEL_COUNT: usize = IMG_SIZE as usize * IMG_SIZE as usize;
const AUTO_SAVE_DELAY_MILLIS: u64 = 200;


#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Default, AsRefStr, EnumString, VariantNames)]
enum DisplayMode { 
    #[default]
    Final,
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
    fin: TextureHandle
}

#[derive(Clone)]
pub(crate) enum FileNameDialogMode {
    Rename(String),
    Create,
}

pub(crate) struct UiData {
    drag: OngoingDrag,
    preview_editing: Option<usize>,
    display_mode: DisplayMode,
    file_name_dialog: Option<FileNameDialogMode>,
    file_name_dialog_just_opened: bool,
    tex_ref_dialog_pass: Option<usize>,
    palette_dialog_open: bool,
    file_name_input: String,
    rename_pending: Option<String>,
    create_pending: Option<String>,
    palette_textures: Option<HashMap<String, TextureHandle>>,
}

struct RetroTexApp {
    tmp_str: String,
    tmp_id_stack: Vec<FileId>,
    file_registry: FileRegistry,
    palettes: PaletteManager,
    settings: Settings,
    file_id: FileId,
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
        let palette_manager = PaletteManager::initialize();

        let ret = Self {
            tmp_str: String::new(),
            tmp_id_stack: Vec::new(),
            file_registry,
            palettes: palette_manager,
            settings,
            file_id,
            last_unsaved_change: Instant::now(),
            output_dir,
            ui_data: UiData {
                drag: OngoingDrag::None,
                preview_editing: None,
                display_mode: DisplayMode::Lit,
                file_name_dialog: None,
                file_name_dialog_just_opened: false,
                tex_ref_dialog_pass: None,
                palette_dialog_open: false,
                file_name_input: String::new(),
                rename_pending: None,
                create_pending: None,
                palette_textures: None,
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
        if self.ui_data.palette_textures.is_none() {
            self.ui_data.palette_textures = Some(self.palettes.load_textures(ctx));
        }

        let available_files = self.file_registry.files_sorted();
        let new_file = show_file_list_panel(ctx, &available_files, &self.file_id, &mut self.ui_data);
        let selected_new_file = if let Some(fid) = new_file {
            self.file_id = fid;
            true
        } else {
            false
        };

        let file_ref = self.file_registry
            .file_by_id(self.file_id)
            .expect("Active file id not found in registry");

        if selected_new_file {
            file_ref.write().unwrap().invalidate_images();
        }

        let closing = ctx.input(|i| {
            if i.key_pressed(egui::Key::F10) {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            if i.key_pressed(egui::Key::Z) && i.modifiers.ctrl {
                file_ref.write().unwrap().undo();
            }
            if i.key_pressed(egui::Key::Y) && i.modifiers.ctrl {
                file_ref.write().unwrap().redo();
            }
            i.viewport().close_requested()
        });

        ctx.set_pixels_per_point(1.5);

        let changed = {
            let (mut file, ui_data) = (file_ref.write().unwrap(), &mut self.ui_data);
            self.tmp_id_stack.clear();
            file.update_layers(&mut self.tmp_id_stack, &self.file_registry, &self.palettes).unwrap_or_else(|e| error!("Failed to update layers for '{}': {}", file.name(), e));
            file.update_images(ctx, &mut self.tmp_id_stack, &self.file_registry, &self.palettes);

            file.modify_definition(ctx, &mut self.tmp_id_stack, &self.file_registry, &self.palettes, | def, images, layers, name | {
                egui::SidePanel::right("right_panel")
                    .default_width(400.0)
                    .show(ctx, |ui| {
                        def.definition_ui(ui, ui_data, name, self.file_id, &available_files, self.palettes.names(), &mut self.tmp_str);
                    });

                egui::CentralPanel::default().show(ctx, |ui| {
                    def.add_preview(ui, ui_data, images, layers, &mut self.tmp_str);
                });
            })
        };

        self.log_overlay.show(ctx);

        let mut file_name_changed = false;
        if let Some(new_name) = self.ui_data.rename_pending.take() {
            if let Err(e) = file_ref.write().unwrap().rename(&new_name) {
                error!("Failed to rename file: {}", e);
            } else {
                file_name_changed = true;
            }
        }

        if changed {
            let mut file = file_ref.write().unwrap();
            self.last_unsaved_change = Instant::now();
            self.tmp_id_stack.clear();
            file.update_layers(&mut self.tmp_id_stack, &self.file_registry, &self.palettes).unwrap_or_else(|e| error!("Failed to update layers for '{}': {}", file.name(), e));
            file.update_images(ctx, &mut self.tmp_id_stack, &self.file_registry, &self.palettes);
        }

        if file_ref.read().unwrap().is_dirty() || file_name_changed {
            let mut file = file_ref.write().unwrap();
            ctx.request_repaint(); // Keep updating until we save
            if closing || self.last_unsaved_change.elapsed().as_millis() >= AUTO_SAVE_DELAY_MILLIS as u128 {
                file.save().unwrap_or_else(|e| error!("Failed to save texture {}: {}", file.name(), e));
                file.write_images(&self.output_dir).unwrap_or_else(|e| error!("Failed to write images for texture {}: {}", file.name(), e));
                assert!(!file.is_dirty(), "File should not be dirty after saving");
            }
        }

        if let Some(new_name) = self.ui_data.create_pending.take() {
            let trimmed_name = new_name.trim();
            if trimmed_name.is_empty() {
                error!("Cannot create file: name cannot be empty");
            } else if self.file_registry.id_by_name(trimmed_name).is_some() {
                error!("Cannot create file: a file named '{}' already exists", trimmed_name);
            } else {
                let id = self.file_registry.create(trimmed_name, TextureDefinition::demo());
                if let Some(created_file) = self.file_registry.file_by_id(id) {
                    let mut created_file = created_file.write().unwrap();
                    if let Err(e) = created_file.save() {
                        error!("Failed to create file '{}': {}", trimmed_name, e);
                    }
                    if let Err(e) = created_file.write_images(&self.output_dir) {
                        error!("Failed to write images for new texture '{}': {}", trimmed_name, e);
                    }
                }
                self.file_id = id;
                self.last_unsaved_change = Instant::now();
                ctx.request_repaint();
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
