use std::{fmt::Display, hash::{Hash, Hasher}};

use glam::{Vec3, Vec3A, Vec4};
use serde::{Deserialize, Serialize};

use crate::IMG_SIZE;

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

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub enum GeneratorOption {
    SolidColor(SolidColorGenerator),
}
impl GeneratorOption {
    fn generate(&self, x: usize, y: usize) -> Vec4 {
        match self {
            GeneratorOption::SolidColor(opts) => Vec4::from(opts.color.v),
        }
    }
}

impl Default for GeneratorOption {
    fn default() -> Self {
        let opts = SolidColorGenerator { color: Color::new(1.0, 0.0, 0.0, 1.0) };
        Self::SolidColor(opts)
    }
}

#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy, Serialize, Deserialize)]
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
pub struct TexturePass {
    pub name: Option<String>,
    pub blend_mode: BlendMode,
    pub generator: GeneratorOption,
}
impl TexturePass {
    fn apply(&self, dest: Vec3, x: usize, y: usize) -> Vec3 {
        let src = self.generator.generate(x, y);
        self.blend_mode.apply(dest, src)
    }
}

impl Default for TexturePass {
    fn default() -> Self {
        Self {
            name: None,
            blend_mode: BlendMode::Normal,
            generator: GeneratorOption::default(),
        }
    }
}


#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct TextureDefinition {
    #[serde(skip)] // This will be the filename
    pub name: String,
    pub passes: Vec<TexturePass>,
}
impl TextureDefinition {
    pub fn generate_pixel(&self, x: usize, y: usize) -> Vec3 {
        let mut ret = Vec3::new(1.0, 0.0, 0.0);
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
            passes: vec![ TexturePass {
                name: None,
                blend_mode: BlendMode::Normal,
                generator: GeneratorOption::SolidColor(SolidColorGenerator { color: Color::new(0.1, 0.1, 0.1, 1.0) })
            }],
        }
    }
}
