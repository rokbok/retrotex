use std::{collections::HashMap, fs::File, io::BufReader, path::{Path, PathBuf}};

use egui::{ColorImage, TextureHandle};
use glam::Vec3;

use crate::prelude::*;

const PALETTES_DIR: &str = "palettes";

const PALETTE_TEX_OPTIONS: egui::TextureOptions = egui::TextureOptions {
    magnification: egui::TextureFilter::Nearest,
    minification: egui::TextureFilter::Nearest,
    wrap_mode: egui::TextureWrapMode::ClampToEdge,
    mipmap_mode: None,
};

#[derive(Clone)]
pub struct Palette {
    pub name: String,
    pub image: Option<ColorImage>,
}

impl Palette {
    pub fn sample(&self, color: Vec3) -> Vec3 {
        color
    }

    pub fn single_time_load(&mut self, ctx: &egui::Context) -> TextureHandle {
        if let Some(image) = self.image.take() {
            ctx.load_texture(&self.name, image, PALETTE_TEX_OPTIONS)
        } else {
            panic!("Palette '{}' was already loaded, cannot load again", self.name);
        }
    }
}

#[derive(Default)]
pub struct PaletteManager {
    palettes: HashMap<String, Palette>,
    names: Vec<String>,
}

impl PaletteManager {
    pub fn initialize() -> Self {
        let folder = Path::new(PALETTES_DIR);
        if let Err(e) = std::fs::create_dir_all(folder) {
            error!("Failed to create palettes directory '{}': {}", PALETTES_DIR, e);
            return Self::default();
        }

        let mut palettes = HashMap::new();
        let mut names = Vec::new();
        let entries = match std::fs::read_dir(folder) {
            Ok(entries) => entries,
            Err(e) => {
                error!("Failed to read palettes directory '{}': {}", PALETTES_DIR, e);
                return Self::default();
            }
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if !is_png_file(&path) {
                continue;
            }

            match load_palette_from_png(&path) {
                Ok(palette) => {
                    names.push(palette.name.clone());
                    palettes.insert(palette.name.clone(), palette);
                }
                Err(e) => warn!("Skipping invalid palette '{}': {}", path.display(), e),
            }
        }

        names.sort_unstable();
        info!("Loaded {} palette file(s) from '{}'", palettes.len(), PALETTES_DIR);

        Self { palettes, names }
    }

    pub fn names(&self) -> &[String] {
        &self.names
    }

    pub fn get(&self, name: &str) -> Option<&Palette> {
        self.palettes.get(name)
    }

    pub fn load_textures(&mut self, ctx: &egui::Context) -> HashMap<String, TextureHandle> {
        self.palettes
            .values_mut()
            .map(|palette| (palette.name.clone(), palette.single_time_load(ctx)))
            .collect()
    }

    pub fn is_empty(&self) -> bool {
        assert!(self.palettes.len() == self.names.len(), "PaletteManager invariant violated: names and palettes length mismatch");
        self.palettes.is_empty()
    }
}

fn is_png_file(path: &Path) -> bool {
    path.is_file()
        && path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("png"))
}

fn load_palette_from_png(path: &Path) -> Result<Palette, String> {
    let file = File::open(path).map_err(|e| format!("Failed to open file: {}", e))?;
    let mut decoder = png::Decoder::new(BufReader::new(file));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().map_err(|e| format!("Failed to decode PNG info: {}", e))?;

    let output_buffer_size = reader
        .output_buffer_size()
        .ok_or_else(|| "Unable to determine PNG output buffer size".to_string())?;
    let mut buf = vec![0; output_buffer_size];
    let info = reader
        .next_frame(&mut buf)
        .map_err(|e| format!("Failed to read PNG frame: {}", e))?;
    let bytes = &buf[..info.buffer_size()];

    let expected_pixels = (info.width as usize)
        .checked_mul(info.height as usize)
        .ok_or_else(|| "Palette image dimensions overflowed".to_string())?;

    let rgba_bytes = match info.color_type {
        png::ColorType::Rgb => {
            if bytes.len() != expected_pixels * 3 {
                return Err(format!(
                    "Decoded RGB byte count mismatch: expected {}, got {}",
                    expected_pixels * 3,
                    bytes.len()
                ));
            }

            let mut out = Vec::with_capacity(expected_pixels * 4);
            for px in bytes.chunks_exact(3) {
                out.extend_from_slice(&[px[0], px[1], px[2], 255]);
            }
            out
        }
        png::ColorType::Rgba => {
            if bytes.len() != expected_pixels * 4 {
                return Err(format!(
                    "Decoded RGBA byte count mismatch: expected {}, got {}",
                    expected_pixels * 4,
                    bytes.len()
                ));
            }
            bytes.to_vec()
        }
        png::ColorType::Grayscale => {
            if bytes.len() != expected_pixels {
                return Err(format!(
                    "Decoded grayscale byte count mismatch: expected {}, got {}",
                    expected_pixels,
                    bytes.len()
                ));
            }

            let mut out = Vec::with_capacity(expected_pixels * 4);
            for &v in bytes {
                out.extend_from_slice(&[v, v, v, 255]);
            }
            out
        }
        png::ColorType::GrayscaleAlpha => {
            if bytes.len() != expected_pixels * 2 {
                return Err(format!(
                    "Decoded grayscale-alpha byte count mismatch: expected {}, got {}",
                    expected_pixels * 2,
                    bytes.len()
                ));
            }

            let mut out = Vec::with_capacity(expected_pixels * 4);
            for px in bytes.chunks_exact(2) {
                out.extend_from_slice(&[px[0], px[0], px[0], px[1]]);
            }
            out
        }
        png::ColorType::Indexed => {
            return Err("Indexed PNG output was not expanded as expected".to_string());
        }
    };

    let image = ColorImage::from_rgba_unmultiplied([info.width as usize, info.height as usize], &rgba_bytes);

    let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .map(str::to_string)
        .ok_or_else(|| "Invalid UTF-8 file name".to_string())?;

    Ok(Palette {
        name,
        image: Some(image),
    })
}

pub fn palettes_dir() -> PathBuf {
    PathBuf::from(PALETTES_DIR)
}
