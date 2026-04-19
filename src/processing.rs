use std::iter::zip;

use egui::ColorImage;
use egui::ahash::HashMap;
use glam::{FloatExt, IVec2, Vec2, Vec3};
use rayon::prelude::*;

use crate::noise::gaussian;
use crate::palettes::{Palette, PaletteManager};
use crate::{TextureHandleSet, color, prelude::*};
use crate::storage::FileRegistry;
use crate::util::{RayIterator};
use crate::{IMG_PIXEL_COUNT, definition::{AOSettings, LightingSettings}};

const IMAGE_TEX_OPTIONS: egui::TextureOptions = egui::TextureOptions {
    magnification: egui::TextureFilter::Nearest,
    minification: egui::TextureFilter::Nearest,
    wrap_mode: egui::TextureWrapMode::ClampToEdge,
    mipmap_mode: None,
};

fn calculate_normals(depth: &[f32], normals: &mut Box<[Vec3]>) {
    assert!(depth.len() == IMG_PIXEL_COUNT);
    assert!(normals.len() == IMG_PIXEL_COUNT);
    // Y points up, Unity-style
    normals.par_iter_mut().enumerate().for_each(|(i, normal)| {
        let (x, y) = idx2coords(i);
        let index = idx(x, y);
        let left  = if x > 0 {            depth[idx_safe(x - 1, y)] } else { depth[index] };
        let right = if x < IMG_SIZE - 1 { depth[idx_safe(x + 1, y)] } else { depth[index] };
        let up    = if y > 0 {            depth[idx_safe(x, y - 1)] } else { depth[index] };
        let down  = if y < IMG_SIZE - 1 { depth[idx_safe(x, y + 1)] } else { depth[index] };

        *normal = Vec3::new(left - right, down - up, 1.0).normalize();
    });
}

fn calculate_ao(depth: &[f32], ao: &mut Box<[f32]>, light_dir: Vec3, settings: &AOSettings) {
    assert!(depth.len() == IMG_PIXEL_COUNT);
    assert!(ao.len() == IMG_PIXEL_COUNT);
    let bias_dir = Vec2::new(-light_dir.x, light_dir.y).normalize_or_zero();
    let light_dir_fact = 1.0 - light_dir.z.abs();
    let bias_strength = light_dir_fact * if bias_dir.length_squared() < 0.1 { 0.0 } else { settings.bias as f32 / 100.0 };

    if settings.radius <= 0 || settings.strength <= 0 {
        for i in 0..IMG_PIXEL_COUNT {
            ao[i] = 1.0;
        }
        return;
    }
    let dirs = [
        IVec2::new(-1,  0),
        IVec2::new(-1, -1),
        IVec2::new( 0, -1),
        IVec2::new( 1, -1),
        IVec2::new( 1,  0),
        IVec2::new( 1,  1),
        IVec2::new( 0,  1),
        IVec2::new(-1,  1),
    ];
    let lengths = dirs.map(| d | d.as_vec2().length() );

    let strength = settings.strength as f32 / 100.0;
    ao.par_iter_mut().enumerate().for_each(|(i, ao)| {
        let (x, y) = idx2coords(i);
        let l = depth[idx_safe(x - 1, y)];
        let r = depth[idx_safe(x + 1, y)];
        let u = depth[idx_safe(x, y - 1)];
        let d = depth[idx_safe(x, y + 1)];
        let surface_slope = if settings.ignore_surface_normal { Vec2::ZERO } else { Vec2::new(r - l, d - u) * 0.5 };
        let dd = depth[idx(x, y)];
        let mut slope_sum = 0.0;
        let pos = IVec2::new(x as i32, y as i32);
        let mut weight_sum = 0.0;
        for (dir, length) in zip(&dirs, &lengths) {
            let dir_slope = surface_slope.dot(dir.as_vec2()) / length;
            let mut slope: f32 = 0.0;
            for i in 1..=settings.radius {
                let sample_pos = pos + *dir * i;
                if sample_pos.x < 0 || sample_pos.x >= IMG_SIZE as i32 || sample_pos.y < 0 || sample_pos.y >= IMG_SIZE as i32 {
                    break;
                }
                let sample_depth: f32 = depth[idx(sample_pos.x as i32, sample_pos.y as i32)];
                slope = slope.max((sample_depth - dd) / (i as f32 * length) - dir_slope);
            }
            let bias_w = bias_dir.dot(dir.as_vec2()).mul_add(0.5, 0.5);
            let weight = 1.0.lerp(bias_w, bias_strength);
            slope_sum += slope * weight;
            weight_sum += weight;
        }
        *ao = 1.0 - (slope_sum / weight_sum * strength).saturate();
    });
}

fn trace_shadow_ray(depth: &[f32], start: IVec2, light_dir: Vec3, light: &LightingSettings) -> f32 {
    assert!(depth.len() == IMG_PIXEL_COUNT);
    let xy_dir = Vec2::new(-light_dir.x, light_dir.y);
    let xy_len = xy_dir.length();
    if xy_len < 0.001 {
        return 1.0; // Light is directly overhead, no lateral shadowing
    }

    let start_depth = depth[idx(start.x, start.y)];
    let dz_per_xy = light_dir.z / xy_len;

    for pos in RayIterator::new(start, xy_dir) {
        if pos == start {
            continue; // Skip the origin pixel
        }
        let xy_dist = (pos - start).as_vec2().length();
        let ray_depth = start_depth + xy_dist * dz_per_xy;
        let excess = depth[idx(pos.x, pos.y)] - ray_depth;
        if excess > 0.0 {
            let fade_min = if light.shadow_fade {
                (xy_dist / light.shadow_fade_distance as f32).min(1.0)
            } else {
                0.0
            };
            return fade_min;
        }
    }

    1.0
}

fn trace_shadows(depth: &[f32], shadow_raw: &mut Box<[f32]>, light: &LightingSettings) {
    assert!(depth.len() == IMG_PIXEL_COUNT);
    assert!(shadow_raw.len() == IMG_PIXEL_COUNT);

    if !light.shadows {
        shadow_raw.par_iter_mut().for_each(|s| *s = 1.0);
        return;
    }

    let use_soft_shadows = light.shadow_rays > 1 && light.shadow_ray_spread.v > 0;
    let light_dir = light.light_dir_vec3();
    let (ld_right, ld_up) = if use_soft_shadows && light_dir.z.abs() < 0.999 {
        let right = light_dir.cross(Vec3::Y).normalize_or_zero();
        let up = light_dir.cross(right).normalize_or_zero();
        (right, up)
    } else {
        (Vec3::ZERO, Vec3::ZERO)
    };
    let spread = f32::from(light.shadow_ray_spread);

    shadow_raw.par_iter_mut().enumerate().for_each(|(i, s)| {
        let (x, y) = idx2coords(i);
        let mut accum = 0.0_f32;
        for r in 0..light.shadow_rays {
            let mut my_dir = light_dir;
            if use_soft_shadows {
                let s1_64 = single_hash(&[x, y, r]);
                let s1 = ((s1_64 & 0xFFFFFFFF) ^ (s1_64 >> 32)) as u32;
                let s2 = s1 ^ 0xDEADBEEF;
                my_dir = light_dir
                    + spread * gaussian(x, y, 1, s1) * ld_right
                    + spread * gaussian(x, y, 1, s2) * ld_up;
                my_dir = my_dir.normalize_or_zero();
            }
            accum +=  trace_shadow_ray(depth, IVec2::new(x, y), my_dir, light);
        }
        *s = accum / light.shadow_rays as f32;
    });
}

fn calculate_light(albedo: &[Vec3], normal: &[Vec3], ao: &[f32], shadow: &[f32], lit: &mut Box<[Vec3]>, light: &LightingSettings) {
    assert!(albedo.len() == IMG_PIXEL_COUNT);
    assert!(normal.len() == IMG_PIXEL_COUNT);
    assert!(ao.len() == IMG_PIXEL_COUNT);
    assert!(shadow.len() == IMG_PIXEL_COUNT);
    assert!(lit.len() == IMG_PIXEL_COUNT);

    let light_dir: Vec3 = light.light_dir_vec3();

    let lfact = 1.0 / light_dir.z.abs().max(0.1); // Make sure flat surface has the assigned color exactly -- within reason

    let lvec = Vec3::new(-light_dir.x, -light_dir.y, light_dir.z);
    lit.par_iter_mut().enumerate().for_each(|(i, lit)| {
        let col = albedo[i];
        let normal = normal[i];
        let shadow_fact = shadow[i];
        let l = lvec.dot(normal).max(0.0) * shadow_fact * lfact;
        let amb = ao[i];
        let f = 1.0.lerp(l, (light.impact as f32 / 100.0).saturate()) * amb;
        *lit = col * f;
    });
}

fn apply_palette(lit: &[Vec3], fin: &mut Box<[Vec3]>, palette: Option<&Palette>) {
    assert!(lit.len() == IMG_PIXEL_COUNT);
    assert!(fin.len() == IMG_PIXEL_COUNT);

    if let Some(palette) = palette {
        // Floyd-Steinberg dithering
        for i in 0..IMG_PIXEL_COUNT {
            fin[i] = lit[i];
        }
        
        for y in 0..IMG_SIZE {
            for x in 0..IMG_SIZE {
                let i = idx(x, y);
                let color = fin[i];
                let quantized = palette.sample(color);
                fin[i] = quantized;
                
                // Calculate quantization error
                let error = color - quantized;
                
                // Distribute error using Floyd-Steinberg weights:
                // (x+1, y): 7/16
                if x + 1 < IMG_SIZE {
                    let j = idx(x + 1, y);
                    fin[j] = fin[j] + error * (7.0 / 16.0);
                }
                // (x-1, y+1): 3/16, (x, y+1): 5/16, (x+1, y+1): 1/16
                if y + 1 < IMG_SIZE {
                    if x > 0 {
                        let j = idx(x - 1, y + 1);
                        fin[j] = fin[j] + error * (3.0 / 16.0);
                    }
                    let j = idx(x, y + 1);
                    fin[j] = fin[j] + error * (5.0 / 16.0);
                    if x + 1 < IMG_SIZE {
                        let j = idx(x + 1, y + 1);
                        fin[j] = fin[j] + error * (1.0 / 16.0);
                    }
                }
            }
        }
    } else {
        fin.par_iter_mut().enumerate().for_each(|(i, fin)| {
            *fin = lit[i];
        });
    }
}

pub struct TextureLayers {
    pub albedo: Box<[Vec3]>,
    pub depth: Box<[f32]>,
    pub normal: Box<[Vec3]>,
    pub ao: Box<[f32]>,
    pub shadow: Box<[f32]>,
    pub lit: Box<[Vec3]>,
    pub fin: Box<[Vec3]>,
}
impl TextureLayers {
    pub fn new() -> Self {
        Self {
            albedo: vec![Vec3::ZERO; IMG_PIXEL_COUNT].into_boxed_slice(),
            depth: vec![0.0; IMG_PIXEL_COUNT].into_boxed_slice(),
            normal: vec![Vec3::new(0.0, 0.0, 1.0); IMG_PIXEL_COUNT].into_boxed_slice(),
            ao: vec![0.0; IMG_PIXEL_COUNT].into_boxed_slice(),
            shadow: vec![1.0; IMG_PIXEL_COUNT].into_boxed_slice(),
            lit: vec![Vec3::ZERO; IMG_PIXEL_COUNT].into_boxed_slice(),
            fin: vec![Vec3::ZERO; IMG_PIXEL_COUNT].into_boxed_slice(),
        }
    }

    pub fn recalculate_derived(&mut self, ao_settings: &AOSettings, light: &LightingSettings, palette: Option<&Palette>) {
        calculate_normals(&self.depth, &mut self.normal);
        calculate_ao(&self.depth, &mut self.ao, light.light_dir_vec3(), ao_settings);
        trace_shadows(&self.depth, &mut self.shadow, light);
        calculate_light(&self.albedo, &self.normal, &self.ao, &self.shadow, &mut self.lit, light);
        apply_palette(&self.lit, &mut self.fin, palette);
    }
}

struct LayerCacheEntry {
    layers: TextureLayers,
    hash: u64,
}

struct ImageCacheEntry {
    tex: TextureHandleSet,
    dirty: bool,
}

pub struct LayerCache {
    layers: HashMap<FileId, LayerCacheEntry>,
    images: HashMap<FileId, ImageCacheEntry>,
    tmp_dep_vec: Vec<FileId>,
}

impl LayerCache {
    pub fn new() -> Self {
        Self {
            layers: HashMap::default(),
            images: HashMap::default(),
            tmp_dep_vec: Vec::new(),
        }
    }

    pub fn get_layers(&self, file_id: FileId) -> Option<&TextureLayers> {
        self.layers.get(&file_id).map(| entry | &entry.layers)
    }

    pub(crate) fn get_images(&self, file_id: FileId) -> Option<&TextureHandleSet> {
        self.images.get(&file_id).map(| entry | &entry.tex)
    }

    pub fn invalidate(&mut self, file_id: FileId) {
        if let Some(entry) = self.layers.get_mut(&file_id) {
            entry.hash = 0;
        }
        if let Some(image_entry) = self.images.get_mut(&file_id) {
            image_entry.dirty = true;
        }
    }
    
    pub fn update_layers_for(&mut self, id: FileId, files: &FileRegistry, pal: &PaletteManager) -> Result<bool, String> {
        self.tmp_dep_vec.clear();
        self.tmp_dep_vec.push(id);
        let mut idx = 0;
        while idx < self.tmp_dep_vec.len() {
            let file_id = self.tmp_dep_vec[idx];
            for other in self.tmp_dep_vec[0..idx].iter() {
                if *other == file_id {
                    return Err(format!("Circular dependency detected involving file id {}", file_id));
                }
            }

            if let Some(file) = files.get(file_id) {
                for dep in file.def().dependencies() {
                    if files.get(dep).is_none() {
                        warn!("File with id {} depends on missing file id {}, skipping dependency", file_id, dep);
                    } else {
                        self.tmp_dep_vec.push(dep);
                    }
                }
            } else {
                return Err(format!("File with id {} not found in registry", file_id));
            }
            idx += 1;
        }

        let needs_update = self.tmp_dep_vec.iter().copied().any(| file_id | {
            let file = files.get(file_id).unwrap();
            if let Some(entry) = self.layers.get(&file_id) {
                let hash = file.definition_hash();
                entry.hash != hash
            } else {
                true
            }
        });

        if !needs_update {
            return Ok(false);
        }

        for file_id in self.tmp_dep_vec.iter().rev().copied() {
            // Temporarily move out of cache to free mutable borrow for recursive dependencies; will be put back at the end of this loop iteration
            let mut dest = if let Some(entry) = self.layers.remove(&file_id) { entry.layers } else { TextureLayers::new() };
            let file = files.get(file_id).unwrap();
            debug!("Regenerating layers for texture '{}'", file.name());
            let def = file.def();
            dest.albedo.par_iter_mut()
                .zip(dest.depth.par_iter_mut())
                .enumerate()
                .for_each(|(i, (albedo_layer, depth_layer))| {
                    let (x, y) = idx2coords(i);
                    let s = def.generate_pixel(x, y, &self);
                    *albedo_layer = s.albedo;    
                    *depth_layer = s.depth;
                });
            dest.recalculate_derived(
                &def.ao_settings,
                &def.lighting_settings,
                def.palette.as_ref().and_then(|name| pal.get(name)),
            );

            self.layers.insert(file_id, LayerCacheEntry { layers: dest, hash: file.definition_hash() });
            if let Some(image_entry) = self.images.get_mut(&file_id) {
                image_entry.dirty = true;
            }
        }
        Ok(true)
    }

    pub fn update_images_for(&mut self, file_id: FileId, ctx: &egui::Context) -> Result<bool, String> {
        if let Some(image_entry) = self.images.get_mut(&file_id) {
            if !image_entry.dirty {
                return Ok(false);
            }
        }

        debug!("Updating images for file id {}", file_id);

        let mut albedo_img = ColorImage::filled([IMG_SIZE as usize, IMG_SIZE as usize], egui::Color32::MAGENTA);
        let mut depth_img = ColorImage::filled([IMG_SIZE as usize, IMG_SIZE as usize], egui::Color32::MAGENTA);
        let mut normal_img = ColorImage::filled([IMG_SIZE as usize, IMG_SIZE as usize], egui::Color32::MAGENTA);
        let mut ao_img = ColorImage::filled([IMG_SIZE as usize, IMG_SIZE as usize], egui::Color32::MAGENTA);
        let mut lit_img = ColorImage::filled([IMG_SIZE as usize, IMG_SIZE as usize], egui::Color32::MAGENTA);
        let mut fin_img = ColorImage::filled([IMG_SIZE as usize, IMG_SIZE as usize], egui::Color32::MAGENTA);

        let layers = if let Some(entry) = self.layers.get(&file_id) {
            &entry.layers
        } else {
            return Err(format!("Cannot update images for file id {} because layers are missing", file_id));
        };

        albedo_img.pixels.par_iter_mut()
            .zip(depth_img.pixels.par_iter_mut())
            .zip(normal_img.pixels.par_iter_mut())
            .zip(ao_img.pixels.par_iter_mut())
            .zip(lit_img.pixels.par_iter_mut())
            .zip(fin_img.pixels.par_iter_mut())
            .enumerate()
            .for_each(|(index, (((((albedo_px, depth_px), normal_px), ao_px), lit_px), fin_px))| {
                let a = layers.albedo[index];
                *albedo_px = color::Color::from_linear(a.extend(1.0)).into();
                
                let d = (layers.depth[index] + 128.0).round().clamp(0.0, 255.0) as u8;
                *depth_px = egui::Rgba::from_srgba_unmultiplied(d, d, d, 255).into();

                let n = layers.normal[index];
                *normal_px = egui::Rgba::from_rgba_unmultiplied(n.x.mul_add(0.5, 0.5).saturate(), n.y.mul_add(0.5, 0.5).saturate(), n.z.saturate(), 1.0).into();

                let ao = layers.ao[index];
                *ao_px = egui::Rgba::from_srgba_unmultiplied((ao * 255.0) as u8, (ao * 255.0) as u8, (ao * 255.0) as u8, 255).into();

                let lit = layers.lit[index];
                *lit_px = color::Color::from_linear(lit.extend(1.0)).into();

                let fin = layers.fin[index];
                *fin_px = color::Color::from_linear(fin.extend(1.0)).into();
            });

        if let Some(entry) = self.images.get_mut(&file_id) {
            entry.tex.albedo.set(albedo_img, IMAGE_TEX_OPTIONS);
            entry.tex.depth.set(depth_img, IMAGE_TEX_OPTIONS);
            entry.tex.normal.set(normal_img, IMAGE_TEX_OPTIONS);
            entry.tex.ao.set(ao_img, IMAGE_TEX_OPTIONS);
            entry.tex.lit.set(lit_img, IMAGE_TEX_OPTIONS);
            entry.tex.fin.set(fin_img, IMAGE_TEX_OPTIONS);
            entry.dirty = false;
        } else {
            self.images.insert(file_id, ImageCacheEntry {
                tex: TextureHandleSet {
                    albedo: ctx.load_texture(format!("preview_albedo_{}", file_id), albedo_img, IMAGE_TEX_OPTIONS),
                    depth: ctx.load_texture(format!("preview_depth_{}", file_id), depth_img, IMAGE_TEX_OPTIONS),
                    normal: ctx.load_texture(format!("preview_normal_{}", file_id), normal_img, IMAGE_TEX_OPTIONS),
                    ao: ctx.load_texture(format!("preview_ao_{}", file_id), ao_img, IMAGE_TEX_OPTIONS),
                    lit: ctx.load_texture(format!("preview_lit_{}", file_id), lit_img, IMAGE_TEX_OPTIONS),
                    fin: ctx.load_texture(format!("preview_fin_{}", file_id), fin_img, IMAGE_TEX_OPTIONS),
                },
                dirty: false,
            });
        }

        Ok(true)
    }
}
