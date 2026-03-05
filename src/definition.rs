use std::{fmt::Display, hash::{Hash, Hasher}};

use glam::{FloatExt, Vec2, Vec3, Vec4};
use serde::{Deserialize, Serialize};
use strum_macros::{AsRefStr, EnumString, VariantNames};

#[allow(unused_imports)]
use log::{debug, error, log_enabled, info, warn, trace};

use crate::{IMG_SIZE, noise, util};

pub const DEFAULT_NAME: &str = "unnamed";

#[derive(Debug, Clone, Serialize, Deserialize, Copy)]
pub struct Color {
    pub v: [f32; 4],
}

impl Hash for Color {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for &component in self.v.iter() {
            let bits = component.to_bits();
            bits.hash(state);
        }
    }
}

impl From<[f32; 4]> for Color {
    fn from(v: [f32; 4]) -> Self {
        Self { v }
    }
}

impl From<Color> for [f32; 4] {
    fn from(color: Color) -> Self {
        color.v
    }
}

impl From<Vec4> for Color {
    fn from(vec: Vec4) -> Self {
        Self { v: [vec.x, vec.y, vec.z, vec.w] }
    }
}

impl From<Color> for Vec4 {
    fn from(color: Color) -> Self {
        Vec4::new(color.v[0], color.v[1], color.v[2], color.v[3])
    }
}

impl Color {
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { v: [r, g, b, a] }
    }

    /// Convert a single sRGB channel in [0,1] to linear space.
    /// Uses the standard IEC 61966-2-1 / sRGB transfer function.
    fn srgb_channel_to_linear(c: f32) -> f32 {
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    }

    pub fn from_hex(hex: &str) -> Result<Self, &'static str> {
        let hex = hex.trim_start_matches('#');
        if hex.len() != 6 && hex.len() != 8 {
            return Err("Hex color must be 6 or 8 characters long");
        }
        let r = u8::from_str_radix(&hex[0..2], 16).map_err(|_| "Invalid hex color")?;
        let g = u8::from_str_radix(&hex[2..4], 16).map_err(|_| "Invalid hex color")?;
        let b = u8::from_str_radix(&hex[4..6], 16).map_err(|_| "Invalid hex color")?;
        let a = if hex.len() == 8 {
            u8::from_str_radix(&hex[6..8], 16).map_err(|_| "Invalid hex color")?
        } else {
            255
        };

        let r_lin = Self::srgb_channel_to_linear(r as f32 / 255.0);
        let g_lin = Self::srgb_channel_to_linear(g as f32 / 255.0);
        let b_lin = Self::srgb_channel_to_linear(b as f32 / 255.0);
        let a_f = a as f32 / 255.0;

        Ok(Self::new(r_lin, g_lin, b_lin, a_f))
    }

    pub fn random() -> Color {
        Color { v: [rand::random(), rand::random(), rand::random(), 1.0] }
    }

    pub fn with_alpha(&self, alpha: f32) -> Color {
        let mut new_color = *self;
        new_color.v[3] = alpha;
        new_color
    }
}

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
    pub use_rect: bool,
    pub rect: Rect,
    pub round_rect: bool,
    pub round_rect_radius: i32,
    pub round_rect_aa: bool,
    pub bevel: bool,
    pub bevel_depth: i32,
    pub bevel_shadow: bool,
    pub bevel_ease_in: bool,
    pub bevel_ease_out: bool,
}

impl TexturePass {
    pub fn new() -> Self {
        Self {
            color: Color::random().with_alpha(0.5),
            perlin_seed: rand::random(),
            white_noise_seed: rand::random(),
            ..Default::default()
        }
    }
    
    fn apply(&self, dest: Vec3, x: i32, y: i32) -> Vec3 {
        if self.use_rect && !self.rect.contains(x, y) {
            return dest;
        }

        let (gen_x, gen_y) = if self.use_rect {
            (x - self.rect.x, y - self.rect.y)
        } else {
            (x, y)
        };

        let mut src: Vec4 = self.color.into();

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

        if self.use_rect {
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
                if fact <= 0.0 {
                    return dest;
                }
                src.w *= fact;
            }
        }

        self.blend_mode.apply(dest, src)
    }
}

impl Default for TexturePass {
    fn default() -> Self {
        Self {
            name: None,
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
            use_rect: false,
            rect: Rect { x: IMG_SIZE / 4, y: IMG_SIZE / 4, w: IMG_SIZE / 2, h: IMG_SIZE / 2 },
            round_rect: false,
            round_rect_radius: 10,
            round_rect_aa: false,
            bevel: false,
            bevel_depth: 5,
            bevel_shadow: false,
            bevel_ease_in: false,
            bevel_ease_out: false,
        }
    }
}


#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct TextureDefinition {
    #[serde(skip)] // This will be the filename
    pub name: String,
    pub background: Color,
    pub passes: Vec<TexturePass>,
}
impl TextureDefinition {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            background: Color::new(0.0, 0.0, 0.0, 1.0),
            passes: vec![TexturePass::new()],
        }
    }

    pub fn generate_pixel(&self, x: i32, y: i32) -> Vec3 {
        let mut ret = Vec3::new(self.background.v[0], self.background.v[1], self.background.v[2]);
        for pass in &self.passes{
            ret = pass.apply(ret, x, y);
        }
        ret
    }
}

impl Default for TextureDefinition {
    fn default() -> Self {
        Self {
            name: DEFAULT_NAME.to_string(),
            background: Color::new(0.0, 0.0, 0.0, 1.0),
            passes: vec![],
        }
    }
}
