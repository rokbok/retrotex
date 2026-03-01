use std::{fmt::Display, hash::{Hash, Hasher}};

use glam::{Vec3, Vec4};
use serde::{Deserialize, Serialize};
use strum_macros::{AsRefStr, EnumString, VariantNames};

#[allow(unused_imports)]
use log::{debug, error, log_enabled, info, warn, trace};

pub const DEFAULT_NAME: &str = "unnamed";

#[derive(Debug, Clone, Serialize, Deserialize)]
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

impl Color {
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { v: [r, g, b, a] }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct SolidColorGenerator {
    pub color: Color,
}

impl Default for SolidColorGenerator {
    fn default() -> Self {
        Self { color: Color::new(1.0, 0.0, 0.0, 1.0) }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub enum WhiteNoiseSeparation {
    Combined,
    Alpha,
    PerChannel,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct WhiteNoiseGenerator {
    pub seed: u32,
    pub color1: Color,
    pub color2: Color,
    pub per_channel: bool,
    pub scale: i32,
}

//fn hash_u32_2(x: u32, y: u32) -> u32 {
//     let mut h = x.wrapping_mul(374761393)
//         .wrapping_add(y.wrapping_mul(668265263));

//     h ^= h >> 13;
//     h = h.wrapping_mul(1274126177);
//     h ^= h >> 16;

//     h
// }


#[derive(Debug, Clone, Serialize, Deserialize, Hash, AsRefStr, EnumString, VariantNames)]
pub enum GeneratorOption {
    SolidColor(SolidColorGenerator),
}

impl GeneratorOption {
    fn generate(&self, _x: i32, _y: i32) -> Vec4 {
        match self {
            GeneratorOption::SolidColor(opts) => Vec4::from(opts.color.v),
        }
    }
}

impl std::default::Default for GeneratorOption {
    fn default() -> Self {
        let opts = SolidColorGenerator { color: Color::new(1.0, 0.0, 0.0, 1.0) };
        Self::SolidColor(opts)
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

        let src = self.generator.generate(x, y);
        self.blend_mode.apply(dest, src)
    }
}

impl Default for TexturePass {
    fn default() -> Self {
        Self {
            name: None,
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
            passes: vec![ TexturePass {
                name: None,
                blend_mode: BlendMode::Normal,
                rect: None,
                generator: GeneratorOption::SolidColor(SolidColorGenerator { color: Color::new(0.1, 0.1, 0.1, 1.0) })
            }],
        }
    }
}
