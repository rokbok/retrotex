use std::{fmt::{Debug, Display, Write}, hash::Hash};

use glam::{FloatExt, IVec3, Vec2, Vec3, Vec4};
use serde::{Deserialize, Serialize};
use strum_macros::{AsRefStr, EnumCount, EnumIter, EnumString, VariantNames};

use crate::prelude::*;

use crate::{color::{Color, EditableColor}, noise::{self, gaussian}, util};

pub const DEFAULT_NAME: &str = "unnamed";

#[allow(dead_code)]
const SQRT2HALF: f32 = 0.70710678118654752440084436210485;

#[derive(Clone, Copy, Hash, Default, Eq, PartialEq)]
pub struct FloatAsInt<const F: i32> {
    pub v: i32,
}

impl<const F: i32> From<FloatAsInt<F>> for f32 {
    fn from(value: FloatAsInt<F>) -> Self {
        value.v as f32 / F as f32
}
}

impl<const F: i32> From<f32> for FloatAsInt<F> {
    fn from(value: f32) -> Self {
        Self { v: (value * F as f32).round() as i32 }
    }
}

impl<const F: i32> Serialize for FloatAsInt<F> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.v.serialize(serializer)
    }
}

impl<'de, const F: i32> Deserialize<'de> for FloatAsInt<F> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let v = i32::deserialize(deserializer)?;
        Ok(Self { v })
    }
}

impl<const F: i32> Debug for FloatAsInt<F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.2}", f32::from(*self))
    }
}

#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy, Serialize, Deserialize, AsRefStr, EnumString, VariantNames, Default, EnumIter, EnumCount)]
pub enum Coverage {
    #[default] Full,
    Rectangle,
    Pattern
}

impl Coverage {
    pub fn is_gizmo_editable(&self) -> bool {
        match &self {
            Coverage::Rectangle => true,
            _ => false,
        }
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

#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy, Serialize, Deserialize, AsRefStr, EnumString, VariantNames, Default, EnumCount, EnumIter)]

pub enum NoiseType {
    #[default] None,
    Perlin,
    Gaussian,
    White,
}


#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy, Serialize, Deserialize, AsRefStr, EnumString, VariantNames, Default)]
pub enum NoiseMode {
    #[default] Color,
    Alpha
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
#[serde(default)]
pub struct NoiseSettings {
    pub noise_type: NoiseType,
    pub mode: NoiseMode,
    pub pixel_scale: i32,
    pub perlin_scale: FloatAsInt<500>,
    pub perlin_octaves: i32,
    pub perlin_strength: FloatAsInt<100>,
    pub seed: u32,
    pub std: FloatAsInt<400>,
    pub use_threshold: bool,
    pub threshold: FloatAsInt<100>,
}

impl NoiseSettings {
    pub const PERLIN_SEED_MASK: u32 = 0xff_ffff; // Avoid too large seeds to avoid floating point issues
}

impl Default for NoiseSettings {
    fn default() -> Self {
        Self {
            noise_type: NoiseType::None,
            mode: NoiseMode::Color,
            pixel_scale: 1,
            perlin_scale: FloatAsInt::from(0.02),
            perlin_octaves: 4,
            perlin_strength: FloatAsInt::from(1.0),
            seed: 0,
            std: FloatAsInt::from(0.1),
            use_threshold: false,
            threshold: FloatAsInt::from(0.5),
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
    pub width: i32,
    pub height: i32,
    pub round: RoundOptions,
    pub bevel: BevelOptions,
}

impl Default for RectSettings {
    fn default() -> Self {
        Self {
            width: IMG_SIZE / 2,
            height: IMG_SIZE / 2,
            round: RoundOptions::default(),
            bevel: BevelOptions::default(),
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
    pub x_gap: i32,
    pub y_gap: i32,
    pub x_count: i32,
    pub y_count: i32,
    pub shift: i32,
    pub shift_direction: TileShiftDirection,
    pub variation_enabled: bool,
    pub variation: FloatAsInt<400>,
    pub variation_seed: u32,
}

impl Default for TileOptions {
    fn default() -> Self {
        Self {
            enabled: false,
            x_gap: 4,
            y_gap: 4,
            x_count: 3,
            y_count: 3,
            shift: 0,
            shift_direction: TileShiftDirection::Horizontal,
            variation_enabled: false,
            variation: 0.05.into(),
            variation_seed: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct Pattern {
    pub rows: [u16; 16],
    pub scale: i32,
    pub mirror_x: bool,
}

impl Pattern {
    pub const SIZE: i32 = 16;

    #[inline]
    pub fn sample(&self, x: i32, y: i32) -> bool {
        let mut x = x / self.scale;
        let y = y / self.scale;
        if self.mirror_x && x >= 8 {
            x = 15 - x;
        }
        self.rows[y as usize] & (1 << (x as u16)) != 0
    }

    #[inline]
    pub fn sample_safe(&self, x: i32, y: i32) -> bool {
        if x < 0 || x >= 16 * self.scale || y < 0 || y >= 16 * self.scale {
            return false;
        }
        self.sample(x, y)
    }

    #[inline]
    pub fn sample_clamp(&self, x: i32, y: i32) -> bool {
        self.sample(x.clamp(0, 15), y.clamp(0, 15))
    }

    #[inline]
    pub fn sample_wrap(&self, x: i32, y: i32) -> bool {
        self.sample(x.rem_euclid(16), y.rem_euclid(16))
    }

    pub fn set(&mut self, x: i32, y: i32, value: bool) {
        if self.mirror_x && x >= 8 {
            self.set(15 - x, y, value);
            return;
        }
        if value {
            self.rows[y as usize] |= 1 << (x as u16);
        } else {
            self.rows[y as usize] &= !(1 << (x as u16));
        }
    }

    pub fn set_safe(&mut self, x: i32, y: i32, value: bool) {
        if x < 0 || x >= 16 || y < 0 || y >= 16 {
            return;
        }
        self.set(x, y, value);
    }

    pub fn set_line(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, value: bool) {
        let mut x = x1.floor() as i32;
        let mut y = y1.floor() as i32;
        let x2 = x2.floor() as i32;
        let y2 = y2.floor() as i32;

        let dx = (x2 - x).abs();
        let dy = (y2 - y).abs();
        let sx = if x < x2 { 1 } else { -1 };
        let sy = if y < y2 { 1 } else { -1 };
        let mut err = dx - dy;

        loop {
            self.set_safe(x, y, value);
            if x == x2 && y == y2 {
                break;
            }           

            let err2 = err * 2;
            if err2 > -dy {
                err -= dy;
                x += sx;
            }
            if err2 < dx {
                err += dx;
                y += sy;
            }
        }
    }
    
    pub(crate) fn fill(&mut self) {
        for row in &mut self.rows {
            *row = 0xFFFF;
        }
    }

    pub(crate) fn clear(&mut self) {
        for row in &mut self.rows {
            *row = 0;
        }
    }

    pub(crate) fn invert(&mut self) {
        let mask: u16 = if self.mirror_x { 0x00FF } else { 0xFFFF };
        for row in &mut self.rows {
            *row ^= mask;
        }
    }

    pub(crate) fn randomize(&mut self) {        
        for row in &mut self.rows {
            *row = rand::random::<u16>();
        }
    }
}

impl Default for Pattern {
    fn default() -> Self {
        Self {
            rows: [6168,15420,26214,50115,33153,33153,32769,49155,24582,12300,6168,3120,1632,960,384,0],
            scale: 4,
            mirror_x: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
#[serde(default)]
pub struct TexturePass {
    pub name: Option<String>,
    pub coverage: Coverage,
    pub enabled: bool,
    pub color: EditableColor<true>,
    pub noise: NoiseSettings,
    pub blend_mode: BlendMode,
    pub feature_x: i32,
    pub feature_y: i32,
    pub rect: RectSettings,
    pub tile: TileOptions,
    pub pattern: Pattern,
}

impl TexturePass {
    pub fn new() -> Self {
        let l = rand::random_range(0..=IMG_SIZE - 40);
        let r = rand::random_range(l + 20..=IMG_SIZE);
        let t = rand::random_range(0..=IMG_SIZE - 40);
        let b = rand::random_range(t + 20..=IMG_SIZE);
        Self {
            color: Color::random().into(),
            noise: NoiseSettings {
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

    pub fn write_name<T: Write>(&self, w: &mut T, idx: usize) -> std::fmt::Result {
        match &self.name {
            Some(name) => write!(w, "{}", name),
            None => write!(w, "Pass {}", idx),
        }
    }
    
    pub fn is_rect(&self) -> bool {
        self.coverage == Coverage::Rectangle
    }

    pub fn is_pattern(&self) -> bool {
        self.coverage == Coverage::Pattern
    }

    pub fn uses_noise(&self) -> bool {
        self.noise.noise_type != NoiseType::None
    }

    pub fn can_use_tilinging(&self) -> bool {
        match self.coverage {
            Coverage::Rectangle | Coverage::Pattern => true,
            _ => false,
        }
    }

    fn noise(&self, x: i32, y: i32, seed: u32) -> f32 {
        let noise_val = match self.noise.noise_type {
            NoiseType::Perlin => {
                let noise_scale= f32::from(self.noise.perlin_scale);
                f32::from(self.noise.perlin_strength)
                    * noise::fbm2(noise_scale * x as f32, noise_scale * y as f32, 
                        self.noise.perlin_octaves.max(1) as u32, 2.0, 0.5,
                        (seed & NoiseSettings::PERLIN_SEED_MASK) as f32) 
            },
            NoiseType::White => {
                let uniform_std = (1.0f32 / 12.0).sqrt(); // Standard deviation of uniform distribution in [0, 1]
                let fact = f32::from(self.noise.std) / uniform_std;
                fact * noise::white_noise(x, y, self.noise.pixel_scale, seed).mul_add(2.0, -1.0)
            },
            NoiseType::Gaussian => {
                let fact = f32::from(self.noise.std);
                fact * gaussian(x, y, self.noise.pixel_scale, seed)
            },
            NoiseType::None => 0.0,
        };

        if self.noise.use_threshold {
            if noise_val >= f32::from(self.noise.threshold).mul_add(2.0, -1.0) { 1.0 } else { -1.0 }
        } else {
            noise_val
        }
    }

    fn tile_dim(&self) -> (i32, i32) {
        match self.coverage {
            Coverage::Rectangle => (self.rect.width + self.tile.x_gap, self.rect.height + self.tile.y_gap),
            Coverage::Pattern => (16 * self.pattern.scale + self.tile.x_gap, 16 * self.pattern.scale + self.tile.y_gap),
            _ => (IMG_SIZE, IMG_SIZE),
        }
    }
    
    fn apply(&self, dest: &mut Vec3, dest_d: &mut f32, x: i32, y: i32) {
        let mut gen_x = x;
        let mut gen_y = y;
        let mut tile_x = 0;
        let mut tile_y = 0;
        
        if self.can_use_tilinging() {
            gen_x -= self.feature_x;
            gen_y -= self.feature_y;

            if !self.tile.enabled {
                if gen_x < 0 || gen_y < 0 {
                    return;
                }
            } else {
                let (tile_width, tile_height) = self.tile_dim();

                match self.tile.shift_direction {
                    TileShiftDirection::Horizontal => gen_x -= (gen_y / tile_height) * self.tile.shift,
                    TileShiftDirection::Vertical => gen_y -= (gen_x / tile_width) * self.tile.shift,
                }
                if gen_x < 0 || gen_y < 0 || gen_x >= tile_width * self.tile.x_count || gen_y >= tile_height * self.tile.y_count {
                    return;
                }

                tile_x = gen_x / tile_width;
                gen_x %= tile_width;
                tile_y = gen_y / tile_height;
                gen_y %= tile_height;
            }
        }

        match self.coverage {
            Coverage::Full => {},
            Coverage::Rectangle => {
                if gen_x < 0 || gen_y < 0 || gen_x >= self.rect.width || gen_y >= self.rect.height {
                    return;
                }
            },
            Coverage::Pattern => {
                if !self.pattern.sample_safe(gen_x, gen_y) {
                    return;
                }
            },
        }

        let mut src = self.color.color().to_linear();
        // We don't want noise to "continue" across tiles
        let seed = self.noise.seed ^ (tile_x as u32).wrapping_mul(0x1f1f1f1f) ^ (tile_y as u32).wrapping_mul(0x1e1e1e1e);
        match self.noise.mode {
            NoiseMode::Color => src += Vec4::new(
                self.noise(x, y, seed),
                self.noise(x, y , seed ^ 0xA5A5A5A5),
                self.noise(x, y, seed ^ 0x5A5A5A5A),
                0.0
            ),
            NoiseMode::Alpha => src.w *= self.noise(x, y, seed).remap(-1.0, 1.0, 0.0, 1.0).saturate(),
        }

        if self.tile.variation_enabled && self.can_use_tilinging() {
            src += f32::from(self.tile.variation) * Vec4::new(
                noise::gaussian(tile_x, tile_y, 1, self.tile.variation_seed),
                noise::gaussian(tile_x, tile_y, 1, self.tile.variation_seed ^ 0xA5A5A5A5),
                noise::gaussian(tile_x, tile_y, 1, self.tile.variation_seed ^ 0x5A5A5A5A),
                0.0
            );
        }

        src = src.saturate();

        let mut bevel_dist = if self.rect.bevel.enabled {
            let from_boundary = Vec4::new(gen_x as f32 + 0.5, gen_y as f32 + 0.5, (self.rect.width - gen_x) as f32 - 0.5, (self.rect.height - gen_y) as f32 - 0.5);
            from_boundary.min_element()
        } else {
            0.0
        };

        if self.is_rect() && self.rect.round.enabled {
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

        if self.is_rect() && self.rect.bevel.enabled  {
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
            coverage: Coverage::default(),
            feature_x: IMG_SIZE / 4,
            feature_y: IMG_SIZE / 4,
            blend_mode: BlendMode::Alpha,
            noise: Default::default(),
            rect: RectSettings::default(),
            tile: TileOptions::default(),
            pattern: Pattern::default(),
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
    pub ao_settings: AOSettings,
    pub lighting_settings: LightingSettings,
    pub passes: Vec<TexturePass>,
}

impl TextureDefinition {
    pub const VERSION: u32 = 1;

    pub fn demo() -> Self {
        Self {
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
                    name: Some("Background".to_string()),
                    color: Color::from_hex("#3E3E3EFF").unwrap().into(),
                    ..Default::default()
                },
                TexturePass {
                    name: Some("Rust".to_string()),
                    color: Color::from_hex("#70310054").unwrap().into(),
                    noise: NoiseSettings {
                        noise_type: NoiseType::Perlin,
                        mode: NoiseMode::Alpha,
                        perlin_scale: FloatAsInt::from(0.03),
                        perlin_octaves: 4,
                        seed: rand::random(),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                TexturePass {
                    name: Some("Frame".to_string()),
                    color: Color::from_hex("#00000022").unwrap().into(),
                    feature_x: 37,
                    feature_y: 25,
                    coverage: Coverage::Rectangle,
                    rect: RectSettings {
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
                    coverage: Coverage::Rectangle,
                    rect: RectSettings {
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
                    coverage: Coverage::Rectangle,
                    rect: RectSettings {
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
            albedo: Vec3::ZERO,
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
            ao_settings: AOSettings::default(),
            lighting_settings: LightingSettings::default(),
            passes: vec![],
        }
    }
}


