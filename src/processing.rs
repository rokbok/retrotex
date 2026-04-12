use std::iter::zip;

use glam::{FloatExt, IVec2, Vec2, Vec3};
use rayon::prelude::*;

use crate::prelude::*;
use crate::util::{RayIterator, gaussian_kernel_weight};
use crate::{IMG_PIXEL_COUNT, definition::{AOSettings, LightingSettings}};


fn calculate_normals(depth: &[f32; IMG_PIXEL_COUNT], normals: &mut Box<[Vec3; IMG_PIXEL_COUNT]>) {
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

fn trace_shadow_ray(depth: &[f32; IMG_PIXEL_COUNT], start: IVec2, light: &LightingSettings) -> Vec2 {
    let light_dir: Vec3 = light.light_dir_vec3();

    let xy_dir = Vec2::new(-light_dir.x, light_dir.y);
    let xy_len = xy_dir.length();
    if xy_len < 0.001 {
        return Vec2::new(1.0, f32::INFINITY); // Light is directly overhead, no lateral shadowing
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
            return Vec2::new(fade_min, xy_dist);
        }
    }

    Vec2::new(1.0, f32::INFINITY) // No occluder found
}

fn trace_shadows(
    depth: &[f32; IMG_PIXEL_COUNT],
    shadow_raw: &mut Box<[Vec2; IMG_PIXEL_COUNT]>,
    light: &LightingSettings,
) {
    if !light.shadows {
        shadow_raw.par_iter_mut().for_each(|s| *s = Vec2::new(1.0, f32::INFINITY));
        return;
    }

    shadow_raw.par_iter_mut().enumerate().for_each(|(i, s)| {
        let (x, y) = idx2coords(i);
        *s = trace_shadow_ray(depth, IVec2::new(x, y), light);
    });
}

fn smooth_shadow(
    shadow_raw: &[Vec2; IMG_PIXEL_COUNT],
    shadow_smooth: &mut Box<[f32; IMG_PIXEL_COUNT]>,
    light: &LightingSettings,
) {
    if !light.smooth_shadows {
        shadow_smooth.par_iter_mut().enumerate().for_each(|(i, s)| {
            *s = shadow_raw[i].x;
        });
        return;
    }

    let kernel_size = light.smooth_kernel_size as f32;
    let kernel_radius = kernel_size.ceil() as i32;

    shadow_smooth.par_iter_mut().enumerate().for_each(|(i, s)| {
        let (x, y) = idx2coords(i);
        let x = x as i32;
        let y = y as i32;
        let center_dist = shadow_raw[i].y;

        let sigma = if center_dist.is_finite() && center_dist > 0.0 {
            // Clamp distance influence: very close = 0.3x, very far = 1.0x
            let distance_factor = (center_dist / 50.0).min(1.0).max(0.3);
            kernel_size * distance_factor
        } else {
            kernel_size
        };

        let mut weighted_sum = 0.0;
        let mut weight_sum = 0.0;

        for dy in -kernel_radius..=kernel_radius {
            for dx in -kernel_radius..=kernel_radius {
                let nx = x + dx;
                let ny = y + dy;

                if nx >= 0 && nx < IMG_SIZE as i32 && ny >= 0 && ny < IMG_SIZE as i32 {
                    let neighbor_idx = idx(nx, ny);
                    let pixel_distance = ((dx * dx + dy * dy) as f32).sqrt();
                    let weight = gaussian_kernel_weight(pixel_distance, sigma);

                    weighted_sum += shadow_raw[neighbor_idx].x * weight;
                    weight_sum += weight;
                }
            }
        }

        *s = if weight_sum > 0.0 {
            weighted_sum / weight_sum
        } else {
            shadow_raw[i].x
        };
    });
}

fn calculate_light(
    albedo: &[Vec3; IMG_PIXEL_COUNT],
    normal: &[Vec3; IMG_PIXEL_COUNT],
    ao: &[f32; IMG_PIXEL_COUNT],
    shadow_smooth: &[f32; IMG_PIXEL_COUNT],
    lit: &mut Box<[Vec3; IMG_PIXEL_COUNT]>,
    light: &LightingSettings,
) {
    let light_dir: Vec3 = light.light_dir_vec3();

    let lfact = 1.0 / light_dir.z.abs().max(0.1); // Make sure flat surface has the assigned color exactly -- within reason

    let lvec = Vec3::new(-light_dir.x, -light_dir.y, light_dir.z);
    lit.par_iter_mut().enumerate().for_each(|(i, lit)| {
        let col = albedo[i];
        let normal = normal[i];
        let shadow_fact = shadow_smooth[i];
        let l = lvec.dot(normal).max(0.0) * shadow_fact * lfact;
        let amb = ao[i];
        let f = 1.0.lerp(l, (light.impact as f32 / 100.0).saturate()) * amb;
        *lit = col * f;
    });
}

pub struct TextureLayers {
    pub albedo: Box<[Vec3; IMG_PIXEL_COUNT]>,
    pub depth: Box<[f32; IMG_PIXEL_COUNT]>,
    pub normal: Box<[Vec3; IMG_PIXEL_COUNT]>,
    pub ao: Box<[f32; IMG_PIXEL_COUNT]>,
    pub shadow_raw: Box<[Vec2; IMG_PIXEL_COUNT]>,
    pub shadow_smooth: Box<[f32; IMG_PIXEL_COUNT]>,
    pub lit: Box<[Vec3; IMG_PIXEL_COUNT]>,
}

impl Default for TextureLayers {
    fn default() -> Self {
        Self {
            albedo: Box::new([Vec3::ZERO; IMG_PIXEL_COUNT]),
            depth: Box::new([0.0; IMG_PIXEL_COUNT]),
            normal: Box::new([Vec3::new(0.0, 0.0, 1.0); IMG_PIXEL_COUNT]),
            ao: Box::new([0.0; IMG_PIXEL_COUNT]),
            shadow_raw: Box::new([Vec2::new(1.0, f32::INFINITY); IMG_PIXEL_COUNT]),
            shadow_smooth: Box::new([1.0; IMG_PIXEL_COUNT]),
            lit: Box::new([Vec3::ZERO; IMG_PIXEL_COUNT]),
        }
    }
}

impl TextureLayers {
    pub fn recalculate_derived(&mut self, ao_settings: &AOSettings, light: &LightingSettings) {
        calculate_normals(&self.depth, &mut self.normal);
        calculate_ao(&self.depth, &mut self.ao, light.light_dir_vec3(), ao_settings);
        trace_shadows(&self.depth, &mut self.shadow_raw, light);
        smooth_shadow(&self.shadow_raw, &mut self.shadow_smooth, light);
        calculate_light(&self.albedo, &self.normal, &self.ao, &self.shadow_smooth, &mut self.lit, light);
    }
}
