use std::{fmt::Display, hash::Hash};

use glam::{FloatExt, IVec3, Vec2, Vec3, Vec4};
use serde::{Deserialize, Serialize};
use strum_macros::{AsRefStr, EnumString, VariantNames};

#[allow(unused_imports)]
use log::{debug, error, log_enabled, info, warn, trace};

use crate::{IMG_SIZE, color::{Color, EditableColor}, noise, util};

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
#[serde(default)]
pub struct LightingSettings {
    pub direction: [i32; 3],
    pub impact: i32,
}

impl LightingSettings {
    pub fn light_dir_vec3(&self) -> Vec3 {
        IVec3::from_array(self.direction).as_vec3().normalize_or_zero()
    }
}

impl Default for LightingSettings {
    fn default() -> Self {
        Self {
            direction: [10, -50, 20],
            impact: 50,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
#[serde(default)]
pub struct AOSettings {
    pub radius: i32,
    pub strength: i32,
    pub bias: i32,
    pub ignore_surface_normal: bool,
}

impl Default for AOSettings {
    fn default() -> Self {
        Self {
            radius: 4,
            strength: 50,
            bias: 50,
            ignore_surface_normal: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
#[serde(default)]
pub struct PerlinSettings {
    pub enabled: bool,
    pub scale: i32,
    pub octaves: i32,
    pub seed: u32,
    pub use_threshold: bool,
    pub threshold: i32,
}

impl PerlinSettings {
    pub fn random_seed() -> u32 {
        rand::random::<u32>() % 0x1_00_00_00 // Avoid too large seeds to avoid floating point issues
    }
}

impl Default for PerlinSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            scale: 10,
            octaves: 4,
            seed: 0,
            use_threshold: false,
            threshold: 50,
        }
    }
}


#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
#[serde(default)]
pub struct WhiteNoiseSettings {
    pub enabled: bool,
    pub scale: i32,
    pub seed: u32,
    pub use_threshold: bool,
    pub threshold: i32,
}

impl Default for WhiteNoiseSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            scale: 1,
            seed: 0,
            use_threshold: false,
            threshold: 50,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
#[serde(default)]
pub struct RoundOptions {
    pub enabled: bool,
    pub radius: i32,
    pub anti_alias: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
#[serde(default)]
pub struct BevelOptions {
    pub enabled: bool,
    pub convex: bool,
    pub size: i32,
    pub steepness: i32,
    pub ease_in: bool,
    pub ease_out: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
#[serde(default)]
pub struct RectSettings {
    pub enabled: bool,
    pub width: i32,
    pub height: i32,
    pub round: RoundOptions,
    pub bevel: BevelOptions,
    pub tile: TileOptions,
}

impl Default for RectSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            width: IMG_SIZE / 2,
            height: IMG_SIZE / 2,
            round: RoundOptions::default(),
            bevel: BevelOptions::default(),
            tile: TileOptions::default(),
        }
    }
}

impl Default for RoundOptions {
    fn default() -> Self {
        Self {
            enabled: false,
            radius: 4,
            anti_alias: false,
        }
    }
}

impl Default for BevelOptions {
    fn default() -> Self {
        Self {
            enabled: false,
            convex: false,
            size: 3,
            steepness: 1,
            ease_in: false,
            ease_out: false,
        }
    }
}

#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy, Serialize, Deserialize, AsRefStr, EnumString, VariantNames)]
pub enum TileShiftDirection { Horizontal, Vertical }

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct TileOptions {
    pub enabled: bool,
    pub x_offset: i32,
    pub y_offset: i32,
    pub x_count: i32,
    pub y_count: i32,
    pub shift: i32,
    pub shift_direction: TileShiftDirection,
    pub variation: i32,
}

impl Default for TileOptions {
    fn default() -> Self {
        Self {
            enabled: false,
            x_offset: 32,
            y_offset: 32,
            x_count: 3,
            y_count: 3,
            shift: 0,
            shift_direction: TileShiftDirection::Horizontal,
            variation: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
#[serde(default)]
pub struct TexturePass {
    pub name: Option<String>,
    pub enabled: bool,
    pub color: EditableColor<true>,
    pub color2: EditableColor<true>,
    pub perlin: PerlinSettings,
    pub white_noise: WhiteNoiseSettings,
    pub blend_mode: BlendMode,
    pub feature_x: i32,
    pub feature_y: i32,
    pub rect: RectSettings,
}

impl TexturePass {
    pub fn new() -> Self {
        let l = rand::random_range(0..=IMG_SIZE - 40);
        let r = rand::random_range(l + 20..=IMG_SIZE);
        let t = rand::random_range(0..=IMG_SIZE - 40);
        let b = rand::random_range(t + 20..=IMG_SIZE);
        Self {
            color: Color::random().into(),
            perlin: PerlinSettings { 
                seed: rand::random(),
                ..Default::default()
            },
            white_noise: WhiteNoiseSettings { 
                seed: rand::random(),
                ..Default::default()
            },
            feature_x: l,
            feature_y: t,
            rect: RectSettings {
                width: r - l,
                height: b - t,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    pub fn uses_both_colors(&self) -> bool {
        self.perlin.enabled
        || self.white_noise.enabled
        || (self.rect.enabled && self.rect.tile.enabled && self.rect.tile.variation > 0)
    }
    
    fn apply(&self, dest: &mut Vec3, dest_d: &mut f32, x: i32, y: i32) {
        let mut gen_x = x;
        let mut gen_y = y;
        
        if self.rect.enabled {
            gen_x -= self.feature_x;
            gen_y -= self.feature_y;

            if !self.rect.tile.enabled {
                if gen_x < 0 || gen_y < 0 {
                    return;
                }
            } else {
                match self.rect.tile.shift_direction {
                    TileShiftDirection::Horizontal => gen_x -= (gen_y / self.rect.tile.y_offset) * self.rect.tile.shift,
                    TileShiftDirection::Vertical => gen_y -= (gen_x / self.rect.tile.x_offset) * self.rect.tile.shift,
                }

                if gen_x < 0 || gen_y < 0 || gen_x >= self.rect.tile.x_offset * self.rect.tile.x_count || gen_y >= self.rect.tile.y_offset * self.rect.tile.y_count {
                    return;
                }

                gen_x %= self.rect.tile.x_offset;
                gen_y %= self.rect.tile.y_offset;
            }

            if gen_x >= self.rect.width || gen_y >= self.rect.height {
                return;
            }
        }

        let mut src = if self.uses_both_colors() {
            let mut color_t = -0.0;

            if self.perlin.enabled {
                let noise_scale = 0.002 * self.perlin.scale as f32;
                let mut noise = noise::fbm2(noise_scale * x as f32, noise_scale * y as f32, self.perlin.octaves.max(1) as u32, 2.0, 0.5, self.perlin.seed as f32);
                noise = noise.remap(-1.0, 1.0, 0.0, 1.0);
                if self.perlin.use_threshold {
                    noise = if noise >= (self.perlin.threshold as f32 / 100.0) { 1.0 } else { 0.0 };
                }
                color_t += noise.mul_add(2.0, -1.0);
            }

            if self.white_noise.enabled {
                let mut noise = noise::white_noise(x, y, self.white_noise.scale, self.white_noise.seed);
                if self.white_noise.use_threshold {
                    noise = if noise >= (self.white_noise.threshold as f32 / 100.0) { 1.0 } else { 0.0 };
                }
                color_t += noise.mul_add(2.0, -1.0);
            }

            self.color.color().to_linear().lerp(self.color2.color().to_linear(), color_t.mul_add(0.5, 0.5).saturate())
        } else {
            self.color.color().to_linear()
        };

        let mut bevel_dist = if self.rect.bevel.enabled {
            let from_boundary = Vec4::new(gen_x as f32 + 0.5, gen_y as f32 + 0.5, (self.rect.width - gen_x) as f32 - 0.5, (self.rect.height - gen_y) as f32 - 0.5);
            from_boundary.min_element()
        } else {
            0.0
        };

        if self.rect.round.enabled {
            let rad = self.rect.round.radius as f32;
            let half_size = Vec2::new(0.5 * self.rect.width as f32, 0.5 * self.rect.height as f32);
            let rel = Vec2::new(gen_x as f32, gen_y as f32) + 0.5 - half_size;
            let d = util::box_sdf(rel, half_size - rad) - rad;
            let fact = if self.rect.round.anti_alias {

                d.remap(0.5, -0.5, 0.0, 1.0).saturate()
            } else {
                if d > 0.0 { 0.0 } else { 1.0 }
            };
            src.w *= fact;

            if self.rect.bevel.enabled {
                let from_corner = rel.abs() - (half_size - Vec2::splat(rad));
                if from_corner.cmpgt(Vec2::ZERO).all() {
                    bevel_dist = rad - from_corner.length();
                }
            }
        }

        if self.rect.bevel.enabled  {
            let bdepth_abs = if self.rect.bevel.steepness > 0 {
                (self.rect.bevel.size * self.rect.bevel.steepness) as f32
            } else if self.rect.bevel.steepness < 0 {
                self.rect.bevel.size as f32 / (-self.rect.bevel.steepness as f32)
            } else {
                self.rect.bevel.size as f32
            };
            let bdepth = if self.rect.bevel.convex { bdepth_abs } else { -bdepth_abs };
            let bt_lin = ((bevel_dist + 0.5) / self.rect.bevel.size.abs() as f32).saturate();
            let bt = match (self.rect.bevel.ease_in, self.rect.bevel.ease_out) {
                (true, true) => if bt_lin < 0.5 { 0.5 * (bt_lin * 2.0).powi(2) } else { 1.0 - 0.5 * ((1.0 - bt_lin) * 2.0).powi(2) },
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
            color: Color::from_hex("#f48a71").unwrap().into(),
            color2: Color::from_hex("#71c8f4").unwrap().into(),

            feature_x: IMG_SIZE / 4,
            feature_y: IMG_SIZE / 4,
            blend_mode: BlendMode::Alpha,
            perlin: Default::default(),
            white_noise: Default::default(),
            rect: RectSettings::default(),
        }
    }
}


#[derive(Debug, Clone, Copy, Default)]
pub struct GeneratedSample {
    pub albedo: Vec3,
    pub depth: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
#[serde(default)]
pub struct TextureDefinition {
    #[serde(skip)] // This will be the filename
    pub name: String,
    pub background: EditableColor<false>,
    pub ao_settings: AOSettings,
    pub lighting_settings: LightingSettings,
    pub passes: Vec<TexturePass>,
}

impl TextureDefinition {
    pub const VERSION: u32 = 1;

    pub fn demo(name: &str) -> Self {
        Self {
            name: name.to_string(),
            background: Color::from_hex("#3E3E3EFF").unwrap().into(),
            ao_settings: AOSettings {
                radius: 4,
                strength: 50,
                bias: 50,
                ..Default::default()
            },
            lighting_settings: LightingSettings {
                direction: [20, -50, 20],
                impact: 50,
            },
            passes: vec![
                TexturePass {
                    name: Some("Rust".to_string()),
                    color: Color::from_hex("#70310054").unwrap().into(),
                    perlin: PerlinSettings {
                        enabled: true,
                        scale: 15,
                        octaves: 4,
                        seed: PerlinSettings::random_seed(),
                        ..Default::default()
                    },
                    white_noise: Default::default(),
                    ..Default::default()
                },
                TexturePass {
                    name: Some("Frame".to_string()),
                    color: Color::from_hex("#00000022").unwrap().into(),
                    feature_x: 37,
                    feature_y: 25,
                    rect: RectSettings {
                        enabled: true,
                        width: 53,
                        height: 98,
                        round: RoundOptions {
                            enabled: true,
                            radius: 4,
                            ..Default::default()
                        },
                        bevel: BevelOptions {
                            enabled: true,
                            convex: false,
                            size: 3,
                            steepness: 1,
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    ..Default::default()
                },
                TexturePass {
                    name: Some("Handle".to_string()),
                    color: Color::from_hex("#00000000").unwrap().into(),
                    feature_x: 79,
                    feature_y: 70,
                    rect: RectSettings {
                        enabled: true,
                        width: 6,
                        height: 6,
                        round: RoundOptions {
                            enabled: true,
                            radius: 6,
                            ..Default::default()
                        },
                        bevel: BevelOptions {
                            enabled: true,
                            convex: true,
                            size: 3,
                            steepness: 1,
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    ..Default::default()
                },
                TexturePass {
                    name: Some("Window".to_string()),
                    color: Color::from_hex("#304A4FFF").unwrap().into(),
                    feature_x: 51,
                    feature_y: 36,
                    rect: RectSettings {
                        enabled: true,
                        width: 26,
                        height: 17,
                        bevel: BevelOptions {
                            enabled: true,
                            convex: false,
                            size: 1,
                            steepness: -8,
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    ..Default::default()
                },
            ],
        }
    }

    pub fn generate_pixel(&self, x: i32, y: i32) -> GeneratedSample {
        let mut ret = GeneratedSample {
            albedo: self.background.color().to_linear().truncate(),
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
            background: Color::new(0, 0, 0, 255).into(),
            ao_settings: AOSettings::default(),
            lighting_settings: LightingSettings::default(),
            passes: vec![],
        }
    }
}


