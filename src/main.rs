// TODO:
// - Transition serialized Colors in u8

use std::{hash::Hash, time::{Duration, Instant}};

use clap::{Parser as _};
use eframe::egui;
use egui::{Color32, ColorImage, TextureHandle};
use glam::FloatExt as _;
use rayon::prelude::*;
use strum_macros::{AsRefStr, EnumString, VariantNames};

use crate::{load_save_undo::LoadSaveUndo, preview_ui::OngoingDrag, processing::TextureLayers, util::single_hash};

#[allow(unused_imports)]
use log::{debug, error, log_enabled, info, warn, trace};

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

const PREVIEW_TEX_OPTIONS: egui::TextureOptions = egui::TextureOptions {
    magnification: egui::TextureFilter::Nearest,
    minification: egui::TextureFilter::Nearest,
    wrap_mode: egui::TextureWrapMode::ClampToEdge,
    mipmap_mode: None,
};

pub fn idx(x: i32, y: i32) -> usize {
    (y * IMG_SIZE + x) as usize
}

pub fn idx_safe(x: i32, y: i32) -> usize {
    let x = x.clamp(0, IMG_SIZE - 1);
    let y = y.clamp(0, IMG_SIZE - 1);
    idx(x, y)
}

pub fn reverse_idx(index: usize) -> (i32, i32) {
    let x = (index as i32) % IMG_SIZE;
    let y = (index as i32) / IMG_SIZE;
    (x, y)
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Default, AsRefStr, EnumString, VariantNames)]
enum DisplayMode { 
    #[default]
    Lit,
    Albedo,
    Depth,
    Normal,
    AmbientOcclusion,
}

#[derive(Debug, Default)]
struct DisplaySettings {
    mode: DisplayMode,
}


struct TextureHandleSet {
    albedo: TextureHandle,
    depth: TextureHandle,
    normal: TextureHandle,
    ao: TextureHandle,
    lit: TextureHandle,
}

struct RetroTexApp {
    def: definition::TextureDefinition,
    tmp_str: String,
    textures: Option<TextureHandleSet>,
    layers: TextureLayers,
    auto_save_at: Option<Instant>,
    load_save_undo: LoadSaveUndo,
    display_settings: DisplaySettings,
    output_dir: String,
    initial_generation_done: bool,
    drag: Option<OngoingDrag>,
    preview_editing: Option<usize>,
}

impl RetroTexApp {
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
            display_settings: DisplaySettings::default(),
            output_dir,
            initial_generation_done: false,
            drag: None,
            preview_editing: None,
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
        let mut ao_img = ColorImage::filled([IMG_SIZE as usize, IMG_SIZE as usize], Color32::MAGENTA);
        let mut lit_img = ColorImage::filled([IMG_SIZE as usize, IMG_SIZE as usize], Color32::MAGENTA);

        albedo_img.pixels.par_iter_mut()
            .zip(depth_img.pixels.par_iter_mut())
            .zip(self.layers.albedo.par_iter_mut())
            .zip(self.layers.depth.par_iter_mut())
            .enumerate()
            .for_each(|(i, (((albedo_px, depth_px), albedo_layer), depth_layer))| {
                let (x, y) = reverse_idx(i);
                let s = self.def.generate_pixel(x, y);
                *albedo_px = color::Color::from_linear(s.albedo.extend(1.0)).into();
                *albedo_layer = s.albedo;
                
                let d = (s.depth + 128.0).round().clamp(0.0, 255.0) as u8;
                *depth_px = egui::Rgba::from_srgba_unmultiplied(d, d, d, 255).into();
                *depth_layer = s.depth;
            });

        self.layers.recalculate(&self.def.ao_settings, & self.def.lighting_settings);

        normal_img.pixels.par_iter_mut()
            .zip(ao_img.pixels.par_iter_mut())
            .zip(lit_img.pixels.par_iter_mut())
            .enumerate()
            .for_each(|(index, ((normal_px, ao_px), lit_px))| {
                let n = self.layers.normal[index];
                *normal_px = egui::Rgba::from_rgba_unmultiplied(n.x.mul_add(0.5, 0.5).saturate(), n.y.mul_add(0.5, 0.5).saturate(), n.z.saturate(), 1.0).into();

                let ao = self.layers.ao[index];
                *ao_px = egui::Rgba::from_srgba_unmultiplied((ao * 255.0) as u8, (ao * 255.0) as u8, (ao * 255.0) as u8, 255).into();

                let lit = self.layers.lit[index];
                *lit_px = color::Color::from_linear(lit.extend(1.0)).into();
            });

        if !self.initial_generation_done {
            info!("Writing initial output images for texture {}...", self.def.name);
            load_save_undo::write_images(&self.layers, &self.output_dir, &self.def.name).unwrap_or_else(|e| error!("Failed to write initial output images: {}", e));
            self.initial_generation_done = true;
        }

        if let Some(tex) = &mut self.textures {
            tex.albedo.set(albedo_img, PREVIEW_TEX_OPTIONS);
            tex.depth.set(depth_img, PREVIEW_TEX_OPTIONS);
            tex.normal.set(normal_img, PREVIEW_TEX_OPTIONS);
            tex.ao.set(ao_img, PREVIEW_TEX_OPTIONS);
            tex.lit.set(lit_img, PREVIEW_TEX_OPTIONS);

            // Data already was written into the correct place
        } else if let Some(ctx) = ctx {
            self.textures = Some(TextureHandleSet {
                albedo: ctx.load_texture("preview_albedo", albedo_img, PREVIEW_TEX_OPTIONS),
                depth: ctx.load_texture("preview_depth", depth_img, PREVIEW_TEX_OPTIONS),
                normal: ctx.load_texture("preview_normal", normal_img, PREVIEW_TEX_OPTIONS),
                ao: ctx.load_texture("preview_ao", ao_img, PREVIEW_TEX_OPTIONS),
                lit: ctx.load_texture("preview_lit", lit_img, PREVIEW_TEX_OPTIONS),
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

impl eframe::App for RetroTexApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.textures.is_none() {
             self.regenerate(Some(ctx));
        }

        let old_hash = single_hash(&self.def);
        let closing = ctx.input(|i| {
            if i.key_pressed(egui::Key::F10) {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            if i.key_pressed(egui::Key::Z) && i.modifiers.ctrl {
                if let Some(undo_def) = self.load_save_undo.undo() {
                    self.def = undo_def;
                }
            }
            if i.key_pressed(egui::Key::Y) && i.modifiers.ctrl {
                if let Some(redo_def) = self.load_save_undo.redo() {
                    self.def = redo_def;
                }
            }
            i.viewport().close_requested()
        });

        ctx.set_pixels_per_point(1.5);
 
        egui::SidePanel::right("right_panel")
            .default_width(400.0)
            .show(ctx, |ui| {
                self.definition_ui(ui);
            });
        
        egui::CentralPanel::default().show(ctx, |ui| {
            self.add_preview(ui);
        });

        let new_hash = single_hash(&self.def);
        if old_hash != new_hash {
            self.regenerate(Some(ctx));
            self.auto_save_at = Some(Instant::now() + Duration::from_millis(AUTO_SAVE_DELAY_MILLIS));
            ctx.request_repaint();
        }

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
        RetroTexApp::name(),
        native_options,
        Box::new(|_| Ok(Box::new(RetroTexApp::new(output)))),
    ).expect("Error running app")
}
