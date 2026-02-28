use std::{fmt::Display, hash::{Hash, Hasher}};

use serde::{Deserialize, Serialize};

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
    None,
    SolidColor(SolidColorGenerator),
}

impl Default for GeneratorOption {
    fn default() -> Self {
        Self::None
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

impl Default for TexturePass {
    fn default() -> Self {
        Self {
            name: None,
            blend_mode: BlendMode::Normal,
            generator: GeneratorOption::None,
        }
    }
}


#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct TextureDefinition {
    #[serde(skip)] // This will be the filename
    pub name: String,
    pub passes: Vec<TexturePass>,
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
