use std::{fmt::Display, hash::Hash};

use glam::{FloatExt, Vec2, Vec3, Vec4};
use serde::{Deserialize, Serialize};
use strum_macros::{AsRefStr, EnumString, VariantNames};

#[allow(unused_imports)]
use log::{debug, error, log_enabled, info, warn, trace};

use crate::{IMG_SIZE, noise, util};
use crate::color::Color;

pub const DEFAULT_NAME: &str = "unnamed";

#[allow(dead_code)]
const SQRT2HALF: f32 = 0.70710678118654752440084436210485;

#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy, Serialize, Deserialize, AsRefStr, EnumString, VariantNames)]
pub enum BlendMode {
    Alpha,
    Additive,
    Multiply,
}

impl BlendMode {
    pub fn all() -> &'static [BlendMode] {
        &[BlendMode::Alpha, BlendMode::Additive, BlendMode::Multiply]
    }
    
    fn apply(&self, bot: Vec3, top: Vec4) -> Vec3 {
        match self {
            BlendMode::Alpha => top.truncate() * top.w + bot * (1.0 - top.w),
            BlendMode::Additive => bot + top.truncate() * top.w,
            BlendMode::Multiply => bot * (top.truncate() * top.w + Vec3::splat(1.0 - top.w)),
        }
    }
}

impl Display for BlendMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BlendMode::Alpha => write!(f, "Normal"),
            BlendMode::Additive => write!(f, "Additive"),
            BlendMode::Multiply => write!(f, "Multiply"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct LightingSettings {
    pub light_dir: [i32; 3],
    pub ambient: i32,
}

impl Default for LightingSettings {
    fn default() -> Self {
        Self {
            light_dir: [10, -50, 20],
            ambient: 50,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct AOSettings {
    pub radius: i32,
    pub strength: i32,
}

impl Default for AOSettings {
    fn default() -> Self {
        Self {
            radius: 4,
            strength: 25,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    pub fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self { x, y, w, h }
    }
    pub fn contains(&self, px: i32, py: i32) -> bool {
        px >= self.x && px < self.x + self.w && py >= self.y && py < self.y + self.h
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct TexturePass {
    pub name: Option<String>,
    pub enabled: bool,
    pub color: Color,
    pub perlin: bool,
    pub perlin_scale: i32,
    pub perlin_octaves: i32,
    pub perlin_seed: u32,
    pub perlin_use_threshold: bool,
    pub perlin_threshold: i32,
    pub white_noise: bool,
    pub white_noise_scale: i32,
    pub white_noise_seed: u32,
    pub white_noise_use_threshold: bool,
    pub white_noise_threshold: i32,
    pub blend_mode: BlendMode,
    pub rect: Rect,
    pub round_rect: bool,
    pub round_rect_radius: i32,
    pub round_rect_aa: bool,
    pub bevel_size: i32,
    pub bevel_steepness: i32,
    pub bevel_shadow: bool,
    pub bevel_ease_in: bool,
    pub bevel_ease_out: bool,
}

impl TexturePass {
    pub fn new() -> Self {
        let l = rand::random_range(0..=IMG_SIZE - 40);
        let r = rand::random_range(l + 20..=IMG_SIZE);
        let t = rand::random_range(0..=IMG_SIZE - 40);
        let b = rand::random_range(t + 20..=IMG_SIZE);
        Self {
            color: Color::random(),
            perlin_seed: rand::random(),
            white_noise_seed: rand::random(),
            rect: Rect { x: l, y: t, w: r - l, h: b - t },
            ..Default::default()
        }
    }
    
    fn apply(&self, dest: &mut Vec3, dest_d: &mut f32, x: i32, y: i32) {
        if !self.rect.contains(x, y) {
            return;
        }

        let gen_x = x - self.rect.x;
        let gen_y = y - self.rect.y;

        let mut src: Vec4 = self.color.to_linear();

        if self.perlin {
            let noise_scale = 0.002 * self.perlin_scale as f32;
            let mut noise = noise::fbm2(noise_scale * gen_x as f32, noise_scale * gen_y as f32, self.perlin_octaves as u32, 2.0, 0.5, self.perlin_seed as f32);
            noise = noise.remap(-1.0, 1.0, 0.0, 1.0);
            if self.perlin_use_threshold {
                noise = if noise >= (self.perlin_threshold as f32 / 100.0) { 1.0 } else { 0.0 };
            }
            src.w *= noise.saturate();
        }

        if self.white_noise {
            let mut noise = noise::white_noise(gen_x, gen_y, self.white_noise_scale, self.white_noise_seed);
            if self.white_noise_use_threshold {
                noise = if noise >= (self.white_noise_threshold as f32 / 100.0) { 1.0 } else { 0.0 };
            }
            src.w *= noise.saturate();
        }

        let mut bevel_dist = if self.bevel_size != 0 {
            let from_boundary = Vec4::new(gen_x as f32 + 0.5, gen_y as f32 + 0.5, (self.rect.w - gen_x) as f32 - 0.5, (self.rect.h - gen_y) as f32 - 0.5);
            from_boundary.min_element()
        } else {
            0.0
        };

        if self.round_rect {
            let rad = (self.round_rect_radius + 2) as f32;
            let half_size = Vec2::new(0.5 * self.rect.w as f32, 0.5 * self.rect.h as f32);
            let rel = Vec2::new(gen_x as f32, gen_y as f32) + 0.5 - half_size;
            let d = util::box_sdf(rel, half_size - rad) - rad;
            let fact = if self.round_rect_aa {
                d.remap(0.5, -0.5, 0.0, 1.0).saturate()
            } else {
                if d > 0.0 { 0.0 } else { 1.0 }
            };
            src.w *= fact;

            if self.bevel_size != 0 {
                let from_corner = rel.abs() - (half_size - Vec2::splat(rad));
                if from_corner.cmpgt(Vec2::ZERO).all() {
                    bevel_dist = rad - from_corner.length();
                }
            }
        }

        if self.bevel_size != 0 && self.bevel_steepness != 0 {
            let bdepth = if self.bevel_steepness > 0 {
                (-self.bevel_size * self.bevel_steepness) as f32
            } else {
                -self.bevel_size as f32 / (-self.bevel_steepness as f32)
            };
            let bt_lin = ((bevel_dist + 0.5) / self.bevel_size.abs() as f32).saturate();
            let bt = match (self.bevel_ease_in, self.bevel_ease_out) {
                (true, true) => {
                    if bt_lin < 0.5 {
                        0.5 * (bt_lin * 2.0).powi(2)
                    } else {
                        1.0 - 0.5 * ((1.0 - bt_lin) * 2.0).powi(2)
                    }
                },
                (true, false) => bt_lin.powi(2),
                (false, true) => 1.0 - (1.0 - bt_lin).powi(2),
                (false, false) => bt_lin,
            };
            *dest_d += bt * bdepth;
        }

        *dest = self.blend_mode.apply(*dest, src);
    }
}

impl Default for TexturePass {
    fn default() -> Self {
        Self {
            name: None,
            enabled: true,
            color: Color::from_hex("#f48a71").unwrap(),
            perlin: false,
            perlin_seed: 0,
            perlin_scale: 10,
            perlin_octaves: 4,
            perlin_use_threshold: false,
            perlin_threshold: 50,
            white_noise: false,
            white_noise_scale: 1,
            white_noise_seed: 0,
            white_noise_use_threshold: false,
            white_noise_threshold: 50,
            blend_mode: BlendMode::Alpha,
            rect: Rect { x: 0, y: 0, w: IMG_SIZE, h: IMG_SIZE },
            round_rect: false,
            round_rect_radius: 10,
            round_rect_aa: false,
            bevel_size: 0,
            bevel_steepness: -1,
            bevel_shadow: false,
            bevel_ease_in: false,
            bevel_ease_out: false,
        }
    }
}


#[derive(Debug, Clone, Copy, Default)]
pub struct GeneratedSample {
    pub albedo: Vec3,
    pub depth: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct TextureDefinition {
    #[serde(skip)] // This will be the filename
    pub name: String,
    pub background: Color,
    pub ao_settings: AOSettings,
    pub lighting_settings: LightingSettings,
    pub passes: Vec<TexturePass>,
}

impl TextureDefinition {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            background: Color::new(0, 0, 0, 255),
            ao_settings: AOSettings::default(),
            lighting_settings: LightingSettings::default(),
            passes: vec![TexturePass::new()],
        }
    }

    pub fn generate_pixel(&self, x: i32, y: i32) -> GeneratedSample {
        let mut ret = GeneratedSample {
            albedo: self.background.to_linear().truncate(),
            depth: 0.0,
        };
        for pass in &self.passes{
            if pass.enabled {
                pass.apply(&mut ret.albedo, &mut ret.depth, x, y);
            }
        }
        ret
    }
}

impl Default for TextureDefinition {
    fn default() -> Self {
        Self {
            name: DEFAULT_NAME.to_string(),
            background: Color::new(0, 0, 0, 255),
            ao_settings: AOSettings::default(),
            lighting_settings: LightingSettings::default(),
            passes: vec![],
        }
    }
}


