use std::{fmt::Display, hash::{Hash, Hasher}};

use glam::{Vec3, Vec4};
use serde::{Deserialize, Serialize};
use strum_macros::{AsRefStr, EnumString, VariantNames};

#[allow(unused_imports)]
use log::{debug, error, log_enabled, info, warn, trace};
use twox_hash::XxHash32;

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
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash, AsRefStr, EnumString, VariantNames, PartialEq, Eq)]
pub enum WhiteNoiseSeparation {
    AllCombined,
    SeparateAlpha,
    EachChannel,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct WhiteNoiseGenerator {
    pub seed: u32,
    pub separation: WhiteNoiseSeparation,
    pub scale: i32,
}

impl WhiteNoiseGenerator {
    #[inline]
    fn hash_to_unit_f32(h: u64) -> f32 {
        let x = h as u32 | (h >> 32) as u32;
        let mant = x >> 8;
        (mant as f32) * (1.0 / 16_777_216.0)
    }

    fn generate(&self, mut x: i32, mut y: i32, c0: Vec4, c1: Vec4) -> Vec4 {
        let mut hasher = XxHash32::with_seed(self.seed);
        if self.scale > 1 {
            x = x / self.scale;
            y = y / self.scale;
        }
        hasher.write_i32(x);
        hasher.write_i32(y);
        let r = Self::hash_to_unit_f32(hasher.finish());
        let a = if self.separation != WhiteNoiseSeparation::AllCombined {
            hasher.write_u32(3);
            Self::hash_to_unit_f32(hasher.finish())
        } else {
            r
        };
        let g = if self.separation == WhiteNoiseSeparation::EachChannel {
            hasher.write_u32(1);
            Self::hash_to_unit_f32(hasher.finish())
        } else {
            r
        };
        let b = if self.separation == WhiteNoiseSeparation::EachChannel {
            hasher.write_u32(2);
            Self::hash_to_unit_f32(hasher.finish())
        } else {
            r
        };
        let t = Vec4::new(r, g, b, a);
        c0 + t * (c1 - c0)
    }
}

impl Default for WhiteNoiseGenerator {
    fn default() -> Self {
        Self { seed: rand::random(), separation: WhiteNoiseSeparation::AllCombined, scale: 1 }
    }
}


#[derive(Debug, Clone, Serialize, Deserialize, Hash, AsRefStr, EnumString, VariantNames)]
pub enum GeneratorOption {
    SolidColor,
    WhiteNoise(WhiteNoiseGenerator),
}

impl GeneratorOption {
    fn generate(&self, x: i32, y: i32, c0: Vec4, c1: Vec4) -> Vec4 {
        match self {
            GeneratorOption::SolidColor => c0,
            GeneratorOption::WhiteNoise(opts) => opts.generate(x, y, c0, c1),
        }
    }
}

impl std::default::Default for GeneratorOption {
    fn default() -> Self {
        Self::SolidColor
    }
}

#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy, Serialize, Deserialize, AsRefStr, EnumString, VariantNames)]
pub enum BlendMode {
    Normal,
    Additive,
    Multiply,
}

impl BlendMode {
    pub fn all() -> &'static [BlendMode] {
        &[BlendMode::Normal, BlendMode::Additive, BlendMode::Multiply]
    }
    
    fn apply(&self, bot: Vec3, top: Vec4) -> Vec3 {
        match self {
            BlendMode::Normal => top.truncate() * top.w + bot * (1.0 - top.w),
            BlendMode::Additive => bot + top.truncate() * top.w,
            BlendMode::Multiply => bot * (top.truncate() * top.w + Vec3::splat(1.0 - top.w)),
        }
    }
}

impl Display for BlendMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BlendMode::Normal => write!(f, "Normal"),
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
    pub color0: Color,
    pub color1: Color,
    pub blend_mode: BlendMode,
    pub rect: Option<Rect>,
    pub generator: GeneratorOption,
}
impl TexturePass {
    fn apply(&self, dest: Vec3, x: i32, y: i32) -> Vec3 {
        let inside_rect = match &self.rect {
            Some(rect) => rect.contains(x, y),
            None => true,
        };
        if !inside_rect{
            return dest;
        }

        let (gen_x, gen_y) = if let Some(rect) = &self.rect {
            (x - rect.x, y - rect.y)
        } else {
            (x, y)
        };

        let src = self.generator.generate(gen_x, gen_y, self.color0.into(), self.color1.into());
        self.blend_mode.apply(dest, src)
    }
}

impl Default for TexturePass {
    fn default() -> Self {
        Self {
            name: None,
            color0: Color::from_hex("#f48a71").unwrap(),
            color1: Color::from_hex("#c3edd9").unwrap(),
            blend_mode: BlendMode::Normal,
            rect: None,
            generator: GeneratorOption::default(),
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
            passes: vec![TexturePass::default()],
        }
    }
}
