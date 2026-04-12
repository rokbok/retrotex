use std::{fs::File, io::{BufRead as _, BufReader, BufWriter, Read, Write as _}, path::{Path, PathBuf}};
use egui::{Color32, ColorImage};
use glam::FloatExt;
use rayon::prelude::*;

use crate::{prelude::*, storage::FileRegistry};
use crate::{IMG_PIXEL_COUNT, TextureHandleSet, color::{self, Color}, definition::TextureDefinition, processing::TextureLayers};

pub const DEFAULT_NAME: &str = "unnamed";
pub const FILE_LOCATION: &str = "textures";
pub const FILE_EXTENSION: &str = "rtex";
const UNDO_LIMIT: usize = 1000;

const PREVIEW_TEX_OPTIONS: egui::TextureOptions = egui::TextureOptions {
    magnification: egui::TextureFilter::Nearest,
    minification: egui::TextureFilter::Nearest,
    wrap_mode: egui::TextureWrapMode::ClampToEdge,
    mipmap_mode: None,
};

pub type FileId = u128;

#[derive(Debug)]
pub struct UndoStack {
    undo_stack: Vec<String>,
    redo_index: usize,
}

impl UndoStack {
    pub fn new() -> Self {
        Self { undo_stack: Vec::new(), redo_index: 0 }
    }

    pub fn push(&mut self, json_content: String) {
        if self.redo_index < self.undo_stack.len() {
            self.undo_stack.truncate(self.redo_index);
        }
        self.undo_stack.push(json_content);
        self.redo_index = self.undo_stack.len();
        while self.undo_stack.len() > UNDO_LIMIT {
            self.undo_stack.remove(0);
            self.redo_index -= 1;
        }
    }
    
    pub fn undo(&mut self) -> Option<TextureDefinition> {
        if self.redo_index <= 1 {
            return None;
        }
        self.redo_index -= 1;
        let last_content = self.undo_stack[self.redo_index - 1].as_str();
        let d: TextureDefinition = serde_json::from_str(last_content).ok().expect("How did we get invalid JSON in the undo stack?");
        Some(d)
    }
    
    pub fn redo(&mut self) -> Option<TextureDefinition> {
        if self.redo_index >= self.undo_stack.len() {
            return None;
        }
        let last_content = self.undo_stack[self.redo_index].as_str();
        let d: TextureDefinition = serde_json::from_str(last_content).ok().expect("How did we get invalid JSON in the undo stack?");
        self.redo_index += 1;
        Some(d)
    }
}

pub struct DefinitionFile {
    id: FileId,
    name: String,
    def: TextureDefinition,
    hash: u64,
    saved_hash: u64,
    layers: TextureLayers,
    layers_hash: u64,
    images: Option<TextureHandleSet>,
    image_hash: u64,
    undo: UndoStack,
}

impl DefinitionFile {
    fn path_for_name(name: &str) -> PathBuf {
        Path::new(FILE_LOCATION).join(format!("{}.{}", name, FILE_EXTENSION))
    }

    pub fn new(name: String) -> Self {
        Self::new_with_def(name, TextureDefinition::default())
    }

    pub fn new_with_def(name: String, def: TextureDefinition) -> Self {
        Self::new_with_def_and_id(name, def, rand::random(), false)
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn id(&self) -> u128 {
        self.id
    }

    fn new_with_def_and_id(name: String, def: TextureDefinition, id: u128, is_saved: bool) -> Self {
        let hash = crate::util::single_hash(&def);
        Self {
            id,
            name,
            def,
            hash,
            saved_hash: if is_saved { hash } else { 0 },
            layers: TextureLayers::default(),
            layers_hash: 0,
            images: None,
            image_hash: 0,
            undo: UndoStack::new(),
        }
    }

    pub fn def(&self) -> &TextureDefinition {
        &self.def
    }

    pub(crate) fn modify_definition<F: FnOnce(&mut TextureDefinition, &TextureHandleSet, &TextureLayers, &str)>(
        &mut self,
        ctx: &egui::Context,
        updating: &mut Vec<FileId>,
        reg: &FileRegistry,
        change_fn: F
    ) -> bool {
        if self.images.is_none() {
            self.update_images(ctx, updating, reg);
        }
        change_fn(&mut self.def, &self.images.as_ref().unwrap(), &self.layers, &self.name);
        let prev_hash = self.hash;
        self.hash = crate::util::single_hash(&self.def);
        prev_hash != self.hash
    }

    pub fn get_layers(&self) -> &TextureLayers {
        &self.layers
    }

    pub fn is_dirty(&self) -> bool {
        self.hash != self.saved_hash
    }

    pub fn invalidate_images(&mut self) {
        self.layers_hash = 0;
        self.image_hash = 0;
    }

    pub fn load_by_name_or_create(name: &str) -> Self {
        match Self::load_by_name(name) {
            Ok(def) => {
                info!("Loaded texture definition: {}", def.name);
                def
            },
            Err(e) => {
                warn!("Failed to load texture definition, creating default: {}", e);
                let def = TextureDefinition::demo();
                let mut ret = Self::new_with_def(name.to_string(), def);
                ret.save().expect("Failed to save default texture definition");
                ret
            },
        }
    }

    pub fn load_by_name(name: &str) -> Result<Self, String> {
        let path = Self::path_for_name(name);
        let mut reader = BufReader::new(File::open(&path).map_err(|e| format!("Failed to open file: {}", e))?);
        let mut buffer = String::new();
        reader.read_line(&mut buffer).map_err(|e| format!("Failed to read file: {}", e))?;
        let magic = buffer.trim();
        if magic != "RETROTEX" {
            return Err(format!("Invalid file format: missing magic header"));
        }

        buffer.clear();
        reader.read_line(&mut buffer).map_err(|e| format!("Failed to read file: {}", e))?;
        let version = buffer.trim().parse::<u32>().map_err(|e| format!("Failed to parse version: {}", e))?;
        if version != TextureDefinition::VERSION {
            return Err(format!("Unsupported version: {}, expected {}", version, TextureDefinition::VERSION));
        }

        buffer.clear();
        reader.read_line(&mut buffer).map_err(|e| format!("Failed to read file: {}", e))?;
        let id = buffer.trim().parse::<u128>().map_err(|e| format!("Failed to parse hash: {}", e))?;
        
        buffer.clear();
        reader.read_to_string(&mut buffer).map_err(|e| format!("Failed to read file: {}", e))?;
        let def: TextureDefinition = serde_json::from_str(&buffer).map_err(|e| format!("Failed to parse JSON: {}", e))?;
        let mut ret = Self::new_with_def_and_id(name.to_string(), def, id, true);
        ret.saved_hash = ret.hash;
        Ok(ret)
    }

    pub fn save(&mut self) -> Result<(), String> {

        let json_content = serde_json::to_string(&self.def).map_err(|e| format!("Failed to serialize to JSON: {}", e))?;

        let path = Self::path_for_name(&self.name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("Failed to create directory: {}", e))?;
        }
        let mut writer = BufWriter::new(File::create(&path).map_err(|e| format!("Failed to create file: {}", e))?);
        writer.write_all(b"RETROTEX\n").map_err(|e| format!("Failed to write file: {}", e))?;
        writer.write_all(format!("{}\n", TextureDefinition::VERSION).as_bytes()).map_err(|e| format!("Failed to write file: {}", e))?;
        writer.write_all(format!("{}\n", self.id).as_bytes()).map_err(|e| format!("Failed to write file: {}", e))?;
        writer.write_all(json_content.as_bytes()).map_err(|e| format!("Failed to write file: {}", e))?;

        self.undo.push(json_content);

        self.saved_hash = self.hash;

        Ok(())
    }

    pub fn rename(&mut self, new_name: &str) -> Result<(), String> {
        if new_name.is_empty() {
            return Err("File name cannot be empty".to_string());
        }

        if self.name == new_name {
            return Ok(());
        }

        let old_path = Self::path_for_name(&self.name);
        let new_path = Self::path_for_name(new_name);

        if new_path.exists() {
            return Err(format!("Cannot rename: destination already exists: {}", new_path.display()));
        }

        if old_path.exists() {
            std::fs::rename(&old_path, &new_path)
                .map_err(|e| format!("Failed to rename file from {} to {}: {}", old_path.display(), new_path.display(), e))?;
        }

        self.name = new_name.to_string();
        Ok(())
    }

    pub fn redo(&mut self) {
        if let Some(def) = self.undo.redo() {
            self.def = def;
            self.hash = crate::util::single_hash(&self.def);
        }
    }

    pub fn undo(&mut self) {
        if let Some(def) = self.undo.undo() {
            self.def = def;
            self.hash = crate::util::single_hash(&self.def);
        }
    }

    pub fn update_layers(&mut self, updating: &mut Vec<FileId>, reg: &FileRegistry) -> Result<(), String> {
        if self.layers_hash != self.hash {
            updating.push(self.id);
                for pass in self.def.passes.iter() {
                    if let Some(rid) = pass.tex_ref.as_ref() {          
                        if updating.contains(rid) {
                            return Err(format!("Circular references: '{}' is already being updated", self.name));
                        }

                        let res = reg.file_by_id(*rid)
                            .map(|file_ref| file_ref.write().unwrap().update_layers(updating, reg));
                        if let Some(r) = res {
                            r?
                        }
                    }
                }
            updating.pop();
            
            debug!("Regenerating layers for texture '{}'", self.name);
            self.layers.albedo.par_iter_mut()
                .zip(self.layers.depth.par_iter_mut())
                .enumerate()
                .for_each(|(i, (albedo_layer, depth_layer))| {
                    let (x, y) = idx2coords(i);
                    let s = self.def.generate_pixel(x, y, reg);
                    *albedo_layer = s.albedo;
                    
                    *depth_layer = s.depth;
                });
            self.layers.recalculate_derived(&self.def.ao_settings, &self.def.lighting_settings);
            self.layers_hash = self.hash;
        }
        Ok(())
    }

    pub(crate) fn update_images(&mut self, ctx: &egui::Context, updating: &mut Vec<FileId>, reg: &FileRegistry) -> &TextureHandleSet {
        if self.images.is_none() || self.image_hash != self.hash {
            self.update_layers(updating, reg).unwrap_or_else(|e| error!("Failed to update layers for '{}': {}", self.name, e));

            let mut albedo_img = ColorImage::filled([IMG_SIZE as usize, IMG_SIZE as usize], Color32::MAGENTA);
            let mut depth_img = ColorImage::filled([IMG_SIZE as usize, IMG_SIZE as usize], Color32::MAGENTA);
            let mut normal_img = ColorImage::filled([IMG_SIZE as usize, IMG_SIZE as usize], Color32::MAGENTA);
            let mut ao_img = ColorImage::filled([IMG_SIZE as usize, IMG_SIZE as usize], Color32::MAGENTA);
            let mut lit_img = ColorImage::filled([IMG_SIZE as usize, IMG_SIZE as usize], Color32::MAGENTA);

            albedo_img.pixels.par_iter_mut()
                .zip(depth_img.pixels.par_iter_mut())
                .zip(normal_img.pixels.par_iter_mut())
                .zip(ao_img.pixels.par_iter_mut())
                .zip(lit_img.pixels.par_iter_mut())
                .enumerate()
                .for_each(|(index, ((((albedo_px, depth_px), normal_px), ao_px), lit_px))| {
                    let a = self.layers.albedo[index];
                    *albedo_px = color::Color::from_linear(a.extend(1.0)).into();
                    
                    let d = (self.layers.depth[index] + 128.0).round().clamp(0.0, 255.0) as u8;
                    *depth_px = egui::Rgba::from_srgba_unmultiplied(d, d, d, 255).into();

                    let n = self.layers.normal[index];
                    *normal_px = egui::Rgba::from_rgba_unmultiplied(n.x.mul_add(0.5, 0.5).saturate(), n.y.mul_add(0.5, 0.5).saturate(), n.z.saturate(), 1.0).into();

                    let ao = self.layers.ao[index];
                    *ao_px = egui::Rgba::from_srgba_unmultiplied((ao * 255.0) as u8, (ao * 255.0) as u8, (ao * 255.0) as u8, 255).into();

                    let lit = self.layers.lit[index];
                    *lit_px = color::Color::from_linear(lit.extend(1.0)).into();
                });

            if let Some(tex) = &mut self.images {
                tex.albedo.set(albedo_img, PREVIEW_TEX_OPTIONS);
                tex.depth.set(depth_img, PREVIEW_TEX_OPTIONS);
                tex.normal.set(normal_img, PREVIEW_TEX_OPTIONS);
                tex.ao.set(ao_img, PREVIEW_TEX_OPTIONS);
                tex.lit.set(lit_img, PREVIEW_TEX_OPTIONS);
                // Data already was written into the correct place
            } else {
                self.images = Some(TextureHandleSet {
                    albedo: ctx.load_texture("preview_albedo", albedo_img, PREVIEW_TEX_OPTIONS),
                    depth: ctx.load_texture("preview_depth", depth_img, PREVIEW_TEX_OPTIONS),
                    normal: ctx.load_texture("preview_normal", normal_img, PREVIEW_TEX_OPTIONS),
                    ao: ctx.load_texture("preview_ao", ao_img, PREVIEW_TEX_OPTIONS),
                    lit: ctx.load_texture("preview_lit", lit_img, PREVIEW_TEX_OPTIONS),
                });
            }
            self.image_hash = self.hash;
        }

        self.images.as_ref().unwrap()
    }

    pub fn write_images(&self, out_dir: &str) -> Result<(), String> {
        let dir = Path::new(out_dir);
        std::fs::create_dir_all(dir).map_err(|e| format!("Failed to create output directory: {}", e))?;

        let mut buf = vec![0u8; crate::IMG_PIXEL_COUNT * 3];

        // Albedo
        {
            let albedo_path = dir.join(format!("{}_albedo.png", self.name));
            info!("Writing albedo image to {}", albedo_path.display());
            let file = File::create(albedo_path).map_err(|e| format!("Failed to create albedo image file: {}", e))?;
            let mut encoder = png::Encoder::new(BufWriter::new(file), crate::IMG_SIZE as u32, crate::IMG_SIZE as u32);
            encoder.set_color(png::ColorType::Rgb);
            encoder.set_depth(png::BitDepth::Eight);
            encoder.set_source_srgb(png::SrgbRenderingIntent::Perceptual);

            let mut writer = encoder.write_header().map_err(|e| format!("Failed to write PNG header: {}", e))?;
            for (i, sample) in self.layers.albedo.iter().enumerate() {
                let c = Color::from_linear(sample.extend(1.0));
                buf[i * 3..i *  3 + 3].copy_from_slice(c.rgba[..3].as_ref());
            }
            writer.write_image_data(&buf[..(3 * IMG_PIXEL_COUNT)]).map_err(|e| format!("Failed to write PNG data: {}", e))?;
            writer.finish().map_err(|e| format!("Failed to finish PNG writing: {}", e))?;
        }

        // Depth
        {
            let depth_path = dir.join(format!("{}_depth.png", self.name));
            info!("Writing depth image to {}", depth_path.display());
            let file = File::create(depth_path).map_err(|e| format!("Failed to create depth image file: {}", e))?;
            let mut encoder = png::Encoder::new(BufWriter::new(file), crate::IMG_SIZE as u32, crate::IMG_SIZE as u32);
            encoder.set_color(png::ColorType::Grayscale);
            encoder.set_depth(png::BitDepth::Sixteen);
            // encoder.set_source_gamma(ScaledFloat::new(1.0));

            let mut writer = encoder.write_header().map_err(|e| format!("Failed to write PNG header: {}", e))?;
            for (i, sample) in self.layers.depth.iter().enumerate() {
                let enc = (sample + 64.0).mul_add(512.0, 0.5) as u16;
                buf[(i * 2)..(i * 2 + 2)].copy_from_slice(&enc.to_be_bytes());
            }
            writer.write_image_data(&buf[..(IMG_PIXEL_COUNT * 2)]).map_err(|e| format!("Failed to write PNG data: {}", e))?;
            writer.finish().map_err(|e| format!("Failed to finish PNG writing: {}", e))?;
        }

        // Normal
        {
            let normal_path = dir.join(format!("{}_normal.png", self.name));
            info!("Writing normal image to {}", normal_path.display());
            let file = File::create(normal_path).map_err(|e| format!("Failed to create normal image file: {}", e))?;
            let mut encoder = png::Encoder::new(BufWriter::new(file), crate::IMG_SIZE as u32, crate::IMG_SIZE as u32);
            encoder.set_color(png::ColorType::Rgb);
            encoder.set_depth(png::BitDepth::Eight);

            let mut writer = encoder.write_header().map_err(|e| format!("Failed to write PNG header: {}", e))?;
            for (i, sample) in self.layers.normal.iter().enumerate() {
                buf[i * 3 + 0] = sample.x.mul_add(0.5, 0.5).saturate().mul_add(255.0, 0.5) as u8;
                buf[i * 3 + 1] = sample.y.mul_add(0.5, 0.5).saturate().mul_add(255.0, 0.5) as u8;
                buf[i * 3 + 2] = sample.z.mul_add(0.5, 0.5).saturate().mul_add(255.0, 0.5) as u8;
            }
            writer.write_image_data(&buf[..(3 * IMG_PIXEL_COUNT)]).map_err(|e| format!("Failed to write PNG data: {}", e))?;
            writer.finish().map_err(|e| format!("Failed to finish PNG writing: {}", e))?;
        }

        // AO
        {
            let ao_path = dir.join(format!("{}_ao.png", self.name));
            info!("Writing AO image to {}", ao_path.display());
            let file = File::create(ao_path).map_err(|e| format!("Failed to create AO image file: {}", e))?;
            let mut encoder = png::Encoder::new(BufWriter::new(file), crate::IMG_SIZE as u32, crate::IMG_SIZE as u32);
            encoder.set_color(png::ColorType::Grayscale);
            encoder.set_depth(png::BitDepth::Eight);

            let mut writer = encoder.write_header().map_err(|e| format!("Failed to write PNG header: {}", e))?;
            for (i, sample) in self.layers.ao.iter().enumerate() {
                buf[i] = sample.mul_add(255.0, 0.5) as u8;
            }
            writer.write_image_data(&buf[..IMG_PIXEL_COUNT]).map_err(|e| format!("Failed to write PNG data: {}", e))?;
            writer.finish().map_err(|e| format!("Failed to finish PNG writing: {}", e))?;
        }

        // Lit
        {
            let lit_path = dir.join(format!("{}.png", self.name));
            info!("Writing lit image to {}", lit_path.display());
            let file = File::create(lit_path).map_err(|e| format!("Failed to create lit image file: {}", e))?;
            let mut encoder = png::Encoder::new(BufWriter::new(file), crate::IMG_SIZE as u32, crate::IMG_SIZE as u32);
            encoder.set_color(png::ColorType::Rgb);
            encoder.set_depth(png::BitDepth::Eight);
            encoder.set_source_srgb(png::SrgbRenderingIntent::Perceptual);

            let mut writer = encoder.write_header().map_err(|e| format!("Failed to write PNG header: {}", e))?;
            for (i, sample) in self.layers.lit.iter().enumerate() {
                let c = Color::from_linear(sample.extend(1.0));
                buf[i * 3..i *  3 + 3].copy_from_slice(c.rgba[..3].as_ref());
            }
            writer.write_image_data(&buf[..(3 * IMG_PIXEL_COUNT)]).map_err(|e| format!("Failed to write PNG data: {}", e))?;
            writer.finish().map_err(|e| format!("Failed to finish PNG writing: {}", e))?;
        }

        Ok(())
    }
}
