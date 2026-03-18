use std::{fmt::Display, hash::Hash};

use glam::{FloatExt, IVec3, Vec2, Vec3, Vec4};
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

impl LightingSettings {
    pub fn light_dir_vec3(&self) -> Vec3 {
        IVec3::from_array(self.light_dir).as_vec3().normalize_or_zero()
    }
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
pub struct PerlinSettings {
    pub enabled: bool,
    pub scale: i32,
    pub octaves: i32,
    pub seed: u32,
    pub use_threshold: bool,
    pub threshold: i32,
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
pub struct RoundOptions {
    pub enabled: bool,
    pub radius: i32,
    pub anti_alias: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct BevelOptions {
    pub enabled: bool,
    pub convex: bool,
    pub size: i32,
    pub steepness: i32,
    pub ease_in: bool,
    pub ease_out: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct RectSettings {
    pub enabled: bool,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub round: RoundOptions,
    pub bevel: BevelOptions,
}

impl Default for RectSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            x: IMG_SIZE / 4,
            y: IMG_SIZE / 4,
            width: IMG_SIZE / 2,
            height: IMG_SIZE / 2,
            round: RoundOptions {
                enabled: false,
                radius: 4,
                anti_alias: false,
            },
            bevel: BevelOptions {
                enabled: false,
                convex: false,
                size: 3,
                steepness: 1,
                ease_in: false,
                ease_out: false,
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct TexturePass {
    pub name: Option<String>,
    pub enabled: bool,
    pub color: Color,
    pub perlin: PerlinSettings,
    pub white_noise: WhiteNoiseSettings,
    pub blend_mode: BlendMode,
    pub rect: RectSettings,
}

impl TexturePass {
    pub fn new() -> Self {
        let l = rand::random_range(0..=IMG_SIZE - 40);
        let r = rand::random_range(l + 20..=IMG_SIZE);
        let t = rand::random_range(0..=IMG_SIZE - 40);
        let b = rand::random_range(t + 20..=IMG_SIZE);
        Self {
            color: Color::random(),
            perlin: PerlinSettings { 
                seed: rand::random(),
                ..Default::default()
            },
            white_noise: WhiteNoiseSettings { 
                seed: rand::random(),
                ..Default::default()
            },
            rect: RectSettings {
                x: l,
                y: t,
                width: r - l,
                height: b - t,
                ..Default::default()
            },
            ..Default::default()
        }
    }
    
    fn apply(&self, dest: &mut Vec3, dest_d: &mut f32, x: i32, y: i32) {
        let (gen_x, gen_y) = if self.rect.enabled {
            (x - self.rect.x, y - self.rect.y)
        } else {
            (x, y)
        };

        if gen_x < 0 || gen_y < 0 {
            return;
        }

        if self.rect.enabled && (gen_x >= self.rect.width || gen_y >= self.rect.height) {
            return;
        }

        let mut src: Vec4 = self.color.to_linear();

        if self.perlin.enabled {
            let noise_scale = 0.002 * self.perlin.scale as f32;
            let mut noise = noise::fbm2(noise_scale * gen_x as f32, noise_scale * gen_y as f32, self.perlin.octaves.max(1) as u32, 2.0, 0.5, self.perlin.seed as f32);
            noise = noise.remap(-1.0, 1.0, 0.0, 1.0);
            if self.perlin.use_threshold {
                noise = if noise >= (self.perlin.threshold as f32 / 100.0) { 1.0 } else { 0.0 };
            }
            src.w *= noise.saturate();
        }

        if self.white_noise.enabled {
            let mut noise = noise::white_noise(gen_x, gen_y, self.white_noise.scale, self.white_noise.seed);
            if self.white_noise.use_threshold {
                noise = if noise >= (self.white_noise.threshold as f32 / 100.0) { 1.0 } else { 0.0 };
            }
            src.w *= noise.saturate();
        }

        let mut bevel_dist = if self.rect.bevel.enabled {
            let from_boundary = Vec4::new(gen_x as f32 + 0.5, gen_y as f32 + 0.5, (self.rect.width - gen_x) as f32 - 0.5, (self.rect.height - gen_y) as f32 - 0.5);
            from_boundary.min_element()
        } else {
            0.0
        };

        if self.rect.round.enabled {
            let rad = (self.rect.round.radius + 2) as f32;
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
            color: Color::from_hex("#f48a71").unwrap(),
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


