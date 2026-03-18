use std::iter::zip;

use glam::{FloatExt, IVec2, IVec3, Vec2, Vec3};

use crate::{IMG_PIXEL_COUNT, IMG_SIZE, definition::{AOSettings, LightingSettings}, idx, idx_safe};

#[allow(unused_imports)]
use log::{debug, error, log_enabled, info, warn, trace};


fn calculate_normals(depth: &[f32; IMG_PIXEL_COUNT], normals: &mut Box<[Vec3; IMG_PIXEL_COUNT]>) {
    // Y points up, Unity-style
    for y in 0..IMG_SIZE {
        for x in 0..IMG_SIZE {
            let index = idx(x, y);
            let left  = if x > 0 {            depth[idx_safe(x - 1, y)] } else { depth[index] };
            let right = if x < IMG_SIZE - 1 { depth[idx_safe(x + 1, y)] } else { depth[index] };
            let up    = if y > 0 {            depth[idx_safe(x, y - 1)] } else { depth[index] };
            let down  = if y < IMG_SIZE - 1 { depth[idx_safe(x, y + 1)] } else { depth[index] };

            let normal = Vec3::new(left - right, down - up, 1.0).normalize();
            normals[index] = normal;
        }
    }
}

fn calculate_ao(depth: &[f32; IMG_PIXEL_COUNT], ao: &mut Box<[f32; IMG_PIXEL_COUNT]>, light_dir: Vec3, settings: &AOSettings) {
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
    for y in 0..IMG_SIZE {
        for x in 0..IMG_SIZE {
            let l = depth[idx_safe(x - 1, y)];
            let r = depth[idx_safe(x + 1, y)];
            let u = depth[idx_safe(x, y - 1)];
            let d = depth[idx_safe(x, y + 1)];
            let surface_slope = Vec2::new(r - l, d - u) * 0.5;
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
            ao[idx(x, y)] = 1.0 - (slope_sum / weight_sum * strength).saturate();
        }
    }
}

fn calculate_light(
    albedo: &[Vec3; IMG_PIXEL_COUNT],
    normal: &[Vec3; IMG_PIXEL_COUNT],
    ao: &[f32; IMG_PIXEL_COUNT],
    lit: &mut Box<[Vec3; IMG_PIXEL_COUNT]>,
    light: &LightingSettings,
) {
    let mut light_dir: Vec3 = light.light_dir_vec3();
    if light_dir.length_squared() < 0.001 {
        light_dir = Vec3::new(1.0, -3.0, 2.0).normalize();
    }

    let lfact = 1.0 / light_dir.z.abs().max(0.1); // Make sure flat surface has the assigned color exactly -- within reason

    let lvec = Vec3::new(-light_dir.x, -light_dir.y, light_dir.z);
    for i in 0..IMG_PIXEL_COUNT {
        let col = albedo[i];
        let normal = normal[i];
        let l = lvec.dot(normal).max(0.0) * lfact;
        let amb = ao[i];
        let f = l.lerp(1.0, (light.ambient as f32 / 100.0).saturate()) * amb;
        lit[i] = col * f;
    }
}

pub struct TextureLayers {
    pub albedo: Box<[Vec3; IMG_PIXEL_COUNT]>,
    pub depth: Box<[f32; IMG_PIXEL_COUNT]>,
    pub normal: Box<[Vec3; IMG_PIXEL_COUNT]>,
    pub ao: Box<[f32; IMG_PIXEL_COUNT]>,
    pub lit: Box<[Vec3; IMG_PIXEL_COUNT]>,
}

impl Default for TextureLayers {
    fn default() -> Self {
        Self {
            albedo: Box::new([Vec3::ZERO; IMG_PIXEL_COUNT]),
            depth: Box::new([0.0; IMG_PIXEL_COUNT]),
            normal: Box::new([Vec3::new(0.0, 0.0, 1.0); IMG_PIXEL_COUNT]),
            ao: Box::new([0.0; IMG_PIXEL_COUNT]),
            lit: Box::new([Vec3::ZERO; IMG_PIXEL_COUNT]),
        }
    }
}

impl TextureLayers {
    pub fn recalculate(&mut self, ao_settings: &AOSettings, light: &LightingSettings) {
        calculate_normals(&self.depth, &mut self.normal);
        calculate_ao(&self.depth, &mut self.ao, light.light_dir_vec3(), ao_settings);
        calculate_light(&self.albedo, &self.normal, &self.ao, &mut self.lit, light);
    }
}
