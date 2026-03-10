use std::{fs::File, io::BufWriter, path::Path};

use crate::{IMG_PIXEL_COUNT, color::Color, definition::{self, TextureDefinition}};

const FILE_LOCATION: &str = "textures";
const UNDO_LIMIT: usize = 1000;

use glam::FloatExt;
#[allow(unused_imports)]
use log::{debug, error, log_enabled, info, warn, trace};

pub struct LoadSaveUndo {
    loaded: Option<String>,
    undo_stack: Vec<String>,
    redo_index: usize,
}

impl LoadSaveUndo {
    pub fn new() -> Self {
        Self {
            loaded: None,
            undo_stack: Vec::new(),
            redo_index: 0,
        }
    }

    pub fn load_by_name_or_create(&mut self, name: &str) -> TextureDefinition {
        match self.load_by_name(name) {
            Ok(def) => {
                info!("Loaded texture definition: {}", def.name);
                def
            },
            Err(e) => {
                warn!("Failed to load texture definition, creating default: {}", e);
                let def = TextureDefinition::new(name);
                self.loaded = Some(name.to_string());
                self.save(&def).expect("Failed to save default texture definition");
                def
            },
        }
    }

    pub fn load_by_name(&mut self, name: &str) -> Result<definition::TextureDefinition, String> {
        let path = Path::new(FILE_LOCATION).join(format!("{}.json", name));
        let file_content = std::fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;
        let mut ret:  TextureDefinition = serde_json::from_str(&file_content).map_err(|e| format!("Failed to parse JSON: {}", e))?;
        ret.name = name.to_string();
        self.loaded = Some(name.to_string());
        self.undo_stack.clear();
        self.undo_stack.push(file_content);
        self.redo_index = 1;
        Ok(ret)
    }

    pub fn save(&mut self, def: &TextureDefinition) -> Result<(), String> {
        assert!(self.loaded.as_ref().map(| ld | *ld == def.name).unwrap_or(false));

        let json_content = serde_json::to_string(def).map_err(|e| format!("Failed to serialize to JSON: {}", e))?;
        if self.redo_index < self.undo_stack.len() {
            self.undo_stack.truncate(self.redo_index);
        }
        self.undo_stack.push(json_content);
        self.redo_index = self.undo_stack.len();
        while self.undo_stack.len() > UNDO_LIMIT {
            self.undo_stack.remove(0);
            self.redo_index -= 1;
        }

        let path = Path::new(FILE_LOCATION).join(format!("{}.json", def.name));
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("Failed to create directory: {}", e))?;
        }
        std::fs::write(path, self.undo_stack.last().unwrap()).map_err(|e| format!("Failed to write file: {}", e))?;
        Ok(())
    }

    pub fn undo(&mut self) -> Option<TextureDefinition> {
        if self.redo_index <= 1 {
            return None;
        }
        self.redo_index -= 1;
        let last_content = self.undo_stack[self.redo_index - 1].as_str();
        let mut ret: TextureDefinition = serde_json::from_str(last_content).ok().expect("How did we get invalid JSON in the undo stack?");
        ret.name = self.loaded.as_ref().unwrap().to_string();
        Some(ret)
    }
    
    pub fn redo(&mut self) -> Option<TextureDefinition> {
        if self.redo_index >= self.undo_stack.len() {
            return None;
        }
        let last_content = self.undo_stack[self.redo_index].as_str();
        let mut ret: TextureDefinition = serde_json::from_str(last_content).ok().expect("How did we get invalid JSON in the undo stack?");
        ret.name = self.loaded.as_ref().unwrap().to_string();
        self.redo_index += 1;
        Some(ret)
    }
}


pub fn write_images(data: &[definition::GeneratedSample], out_dir: &str, name: &str) -> Result<(), String> {
    let dir = Path::new(out_dir);
    std::fs::create_dir_all(dir).map_err(|e| format!("Failed to create output directory: {}", e))?;

    let mut buf = vec![0u8; crate::IMG_PIXEL_COUNT * 3];

    // Albedo
    {
        let albedo_path = dir.join(format!("{}_albedo.png", name));
        info!("Writing albedo image to {}", albedo_path.display());
        let file = File::create(albedo_path).map_err(|e| format!("Failed to create albedo image file: {}", e))?;
        let mut encoder = png::Encoder::new(BufWriter::new(file), crate::IMG_SIZE as u32, crate::IMG_SIZE as u32);
        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.set_source_srgb(png::SrgbRenderingIntent::Perceptual);

        let mut writer = encoder.write_header().map_err(|e| format!("Failed to write PNG header: {}", e))?;
        for (i, sample) in data.iter().enumerate() {
            buf[i * 3 + 0] = Color::linear_channel_to_srgb(sample.albedo.x).saturate().mul_add(255.0, 0.5) as u8;
            buf[i * 3 + 1] = Color::linear_channel_to_srgb(sample.albedo.y).saturate().mul_add(255.0, 0.5) as u8;
            buf[i * 3 + 2] = Color::linear_channel_to_srgb(sample.albedo.z).saturate().mul_add(255.0, 0.5) as u8;
        }
        writer.write_image_data(&buf[..(3 * IMG_PIXEL_COUNT)]).map_err(|e| format!("Failed to write PNG data: {}", e))?;
        writer.finish().map_err(|e| format!("Failed to finish PNG writing: {}", e))?;
    }

    // Depth
    {
        let depth_path = dir.join(format!("{}_depth.png", name));
        let file = File::create(depth_path).map_err(|e| format!("Failed to create depth image file: {}", e))?;
        let mut encoder = png::Encoder::new(BufWriter::new(file), crate::IMG_SIZE as u32, crate::IMG_SIZE as u32);
        encoder.set_color(png::ColorType::Grayscale);
        encoder.set_depth(png::BitDepth::Sixteen);
        // encoder.set_source_gamma(ScaledFloat::new(1.0));

        let mut writer = encoder.write_header().map_err(|e| format!("Failed to write PNG header: {}", e))?;
        for (i, sample) in data.iter().enumerate() {
            let enc = (sample.depth + 64.0).mul_add(512.0, 0.5) as u16;
            if i == 2000 || i == 0 {
                info!("Encoding depth value {} to {}", sample.depth, enc);
            }
            buf[(i * 2)..(i * 2 + 2)].copy_from_slice(&enc.to_be_bytes());
        }
        writer.write_image_data(&buf[..(IMG_PIXEL_COUNT * 2)]).map_err(|e| format!("Failed to write PNG data: {}", e))?;
        writer.finish().map_err(|e| format!("Failed to finish PNG writing: {}", e))?;
    }

    Ok(())
}
