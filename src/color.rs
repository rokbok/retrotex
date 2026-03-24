use std::hash::Hash;
use glam::Vec4;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize, Hash, Copy)]
pub struct Color {
    pub rgba: [u8; 4],
}

impl Color {
    pub fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { rgba: [r, g, b, a] }
    }

    pub fn newi(r: i32, g: i32, b: i32, a: i32) -> Self {
        Self {
            rgba: [
                r.clamp(0, 255) as u8,
                g.clamp(0, 255) as u8,
                b.clamp(0, 255) as u8,
                a.clamp(0, 255) as u8,
            ],
        }
    }

    pub fn to_linear(&self) -> Vec4 {
        Vec4::new(
            Self::srgb_channel_to_linear(self.rgba[0] as f32 / 255.0),
            Self::srgb_channel_to_linear(self.rgba[1] as f32 / 255.0),
            Self::srgb_channel_to_linear(self.rgba[2] as f32 / 255.0),
            self.rgba[3] as f32 / 255.0,
        )
    }

    pub fn from_linear(linear: Vec4) -> Self {
        let r = (Self::linear_channel_to_srgb(linear.x) * 255.0).round().clamp(0.0, 255.0) as u8;
        let g = (Self::linear_channel_to_srgb(linear.y) * 255.0).round().clamp(0.0, 255.0) as u8;
        let b = (Self::linear_channel_to_srgb(linear.z) * 255.0).round().clamp(0.0, 255.0) as u8;
        let a = (linear.w.clamp(0.0, 1.0) * 255.0).round() as u8;
        Self { rgba: [r, g, b, a] }
    }

    pub fn write_hex<W: std::fmt::Write>(&self, out: &mut W) -> std::fmt::Result {
        write!(out, "#{:02x}{:02x}{:02x}{:02x}", self.rgba[0], self.rgba[1], self.rgba[2], self.rgba[3])
    }

    pub fn to_hex(&self) -> String {
        let mut s = String::new();
        self.write_hex(&mut s).unwrap();
        s
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
        Ok(Self { rgba: [r, g, b, a] })
    }

    pub fn random() -> Self {
        Self {
            rgba: [rand::random(), rand::random(), rand::random(), 255],
        }
    }

    pub fn with_alpha(&self, alpha: u8) -> Self {
        let mut ret = *self;
        ret.rgba[3] = alpha;
        ret
    }
    
    /// Convert a single sRGB channel in [0,1] to linear space.
    /// Uses the standard IEC 61966-2-1 / sRGB transfer function.
    pub fn srgb_channel_to_linear(c: f32) -> f32 {
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    }

    /// Convert a single linear channel in [0,1] to sRGB space.
    /// Inverse of `srgb_channel_to_linear` using the standard sRGB transfer function.
    pub fn linear_channel_to_srgb(c: f32) -> f32 {
        if c <= 0.0031308 {
            c * 12.92
        } else {
            1.055 * c.powf(1.0 / 2.4) - 0.055
        }
    }
}

impl From<[u8; 4]> for Color {
    fn from(c: [u8; 4]) -> Self {
        Color::new(c[0], c[1], c[2], c[3])
    }
}

impl From<Color> for [u8; 4] {
    fn from(c: Color) -> Self {
        c.rgba
    }
}

impl From<Color> for egui::Color32 {
    fn from(c: Color) -> Self {
        egui::Color32::from_rgba_unmultiplied(c.rgba[0], c.rgba[1], c.rgba[2], c.rgba[3])
    }
}

#[derive(Debug, Clone)]
pub struct EditableColor<const ALPHA: bool> {
    c: Color,
    pub edit_str: String,
}

impl<const ALPHA: bool> Serialize for EditableColor<ALPHA> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.c.serialize(serializer)
    }
}

impl<'de, const ALPHA: bool> Deserialize<'de> for EditableColor<ALPHA> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let mut c = Color::deserialize(deserializer)?;
        if !ALPHA {
            c.rgba[3] = 255;
        }
        Ok(Self::new(c))
    }
}

impl<const ALPHA: bool> EditableColor<ALPHA> {
    pub fn new(c: Color) -> Self {
        let mut strn = c.to_hex();
        if !ALPHA {
            strn.truncate(7);
        }
        Self { c, edit_str: strn }
    }

    pub fn color(&self) -> Color {
        self.c
    }

    pub fn set_color(&mut self, new_color: Color) {
        self.c = new_color;
        self.edit_str.clear();
        new_color.write_hex(&mut self.edit_str).unwrap();
        if !ALPHA {
            self.edit_str.truncate(7);
        }
    }

    pub fn set_color_while_editing(&mut self, new_color: Color) {
        self.c = new_color;
    }
    
    pub(crate) fn update_edit_str(&mut self) {
        self.edit_str.clear();
        self.c.write_hex(&mut self.edit_str).unwrap();
        if !ALPHA {
            self.edit_str.truncate(7);
        }
    }
}

impl<const ALPHA: bool> Hash for EditableColor<ALPHA> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.c.hash(state);
    }
}

impl<const ALPHA: bool> From<Color> for EditableColor<ALPHA> {
    fn from(c: Color) -> Self {
        Self { c, edit_str: c.to_hex() }
    }
}

#[cfg(test)]
mod tests {
    use super::Color;
use glam::Vec4;

    #[test]
    fn test_new() {
        let c = Color::new(10, 20, 30, 40);
        assert_eq!(c.rgba[0], 10);
        assert_eq!(c.rgba[1], 20);
        assert_eq!(c.rgba[2], 30);
        assert_eq!(c.rgba[3], 40);
    }

    #[test]
    fn test_newi_normal() {
        let c = Color::newi(100, 150, 200, 255);
        assert_eq!(c.rgba[0], 100);
        assert_eq!(c.rgba[1], 150);
        assert_eq!(c.rgba[2], 200);
        assert_eq!(c.rgba[3], 255);
    }

    #[test]
    fn test_newi_clamp_low() {
        let c = Color::newi(-10, -1, -100, -255);
        assert_eq!(c.rgba[0], 0);
        assert_eq!(c.rgba[1], 0);
        assert_eq!(c.rgba[2], 0);
        assert_eq!(c.rgba[3], 0);
    }

    #[test]
    fn test_newi_clamp_high() {
        let c = Color::newi(300, 256, 1000, 999);
        assert_eq!(c.rgba[0], 255);
        assert_eq!(c.rgba[1], 255);
        assert_eq!(c.rgba[2], 255);
        assert_eq!(c.rgba[3], 255);
    }

    #[test]
    fn test_to_linear_black() {
        let c = Color::new(0, 0, 0, 0);
        let v = c.to_linear();
        assert!((v.x - 0.0).abs() < 1e-6);
        assert!((v.y - 0.0).abs() < 1e-6);
        assert!((v.z - 0.0).abs() < 1e-6);
        assert!((v.w - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_to_linear_white() {
        let c = Color::new(255, 255, 255, 255);
        let v = c.to_linear();
        assert!((v.x - 1.0).abs() < 1e-5);
        assert!((v.y - 1.0).abs() < 1e-5);
        assert!((v.z - 1.0).abs() < 1e-5);
        assert!((v.w - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_to_linear_alpha_unchanged() {
        let c = Color::new(0, 0, 0, 128);
        let v = c.to_linear();
        assert!((v.w - 128.0 / 255.0).abs() < 1e-6);
    }

    #[test]
    fn test_from_linear_white() {
        let c = Color::from_linear(Vec4::new(1.0, 1.0, 1.0, 1.0));
        assert_eq!(c.rgba[0], 255);
        assert_eq!(c.rgba[1], 255);
        assert_eq!(c.rgba[2], 255);
        assert_eq!(c.rgba[3], 255);
    }

    #[test]
    fn test_from_linear_black() {
        let c = Color::from_linear(Vec4::new(0.0, 0.0, 0.0, 0.0));
        assert_eq!(c.rgba[0], 0);
        assert_eq!(c.rgba[1], 0);
        assert_eq!(c.rgba[2], 0);
        assert_eq!(c.rgba[3], 0);
    }

    #[test]
    fn test_from_linear_clamp_over() {
        let c = Color::from_linear(Vec4::new(2.0, 2.0, 2.0, 2.0));
        assert_eq!(c.rgba[0], 255);
        assert_eq!(c.rgba[1], 255);
        assert_eq!(c.rgba[2], 255);
        assert_eq!(c.rgba[3], 255);
    }

    #[test]
    fn test_from_linear_clamp_under() {
        let c = Color::from_linear(Vec4::new(-1.0, -1.0, -1.0, -1.0));
        assert_eq!(c.rgba[0], 0);
        assert_eq!(c.rgba[1], 0);
        assert_eq!(c.rgba[2], 0);
        assert_eq!(c.rgba[3], 0);
    }

    #[test]
    fn test_to_linear_from_linear_roundtrip() {
        let original = Color::new(100, 150, 200, 220);
        let linear = original.to_linear();
        let recovered = Color::from_linear(linear);
        assert_eq!(recovered.rgba[0], original.rgba[0]);
        assert_eq!(recovered.rgba[1], original.rgba[1]);
        assert_eq!(recovered.rgba[2], original.rgba[2]);
        assert_eq!(recovered.rgba[3], original.rgba[3]);
    }

    #[test]
    fn test_to_hex_with_alpha() {
        let c = Color::new(255, 0, 128, 255);
        assert_eq!(c.to_hex(), "#ff0080ff");
    }

    #[test]
    fn test_to_hex_black() {
        let c = Color::new(0, 0, 0, 0);
        assert_eq!(c.to_hex(), "#00000000");
    }

    #[test]
    fn test_to_hex_white() {
        let c = Color::new(255, 255, 255, 255);
        assert_eq!(c.to_hex(), "#ffffffff");
    }

    #[test]
    fn test_write_hex() {
        let c = Color::new(16, 32, 48, 64);
        let mut s = String::new();
        c.write_hex(&mut s).unwrap();
        assert_eq!(s, "#10203040");
    }

    #[test]
    fn test_from_hex_6_chars() {
        let c = Color::from_hex("#FF8000").unwrap();
        assert_eq!(c.rgba[0], 255);
        assert_eq!(c.rgba[1], 128);
        assert_eq!(c.rgba[2], 0);
        assert_eq!(c.rgba[3], 255);
    }

    #[test]
    fn test_from_hex_8_chars() {
        let c = Color::from_hex("#FF800080").unwrap();
        assert_eq!(c.rgba[0], 255);
        assert_eq!(c.rgba[1], 128);
        assert_eq!(c.rgba[2], 0);
        assert_eq!(c.rgba[3], 128);
    }

    #[test]
    fn test_from_hex_no_hash() {
        let c = Color::from_hex("FF8000").unwrap();
        assert_eq!(c.rgba[0], 255);
        assert_eq!(c.rgba[1], 128);
        assert_eq!(c.rgba[2], 0);
        assert_eq!(c.rgba[3], 255);
    }

    #[test]
    fn test_from_hex_lowercase() {
        let c = Color::from_hex("#ff8000").unwrap();
        assert_eq!(c.rgba[0], 255);
        assert_eq!(c.rgba[1], 128);
        assert_eq!(c.rgba[2], 0);
        assert_eq!(c.rgba[3], 255);
    }

    #[test]
    fn test_from_hex_invalid_length() {
        assert!(Color::from_hex("#FFF").is_err());
        assert!(Color::from_hex("#FFFFF").is_err());
        assert!(Color::from_hex("#FFFFFFF").is_err());
    }

    #[test]
    fn test_from_hex_invalid_chars() {
        assert!(Color::from_hex("#GGGGGG").is_err());
        assert!(Color::from_hex("#ZZZZZZZZ").is_err());
    }

    #[test]
    fn test_hex_roundtrip() {
        let original = Color::new(12, 34, 56, 78);
        let hex = original.to_hex();
        let recovered = Color::from_hex(&hex).unwrap();
        assert_eq!(recovered.rgba[0], original.rgba[0]);
        assert_eq!(recovered.rgba[1], original.rgba[1]);
        assert_eq!(recovered.rgba[2], original.rgba[2]);
        assert_eq!(recovered.rgba[3], original.rgba[3]);
    }

    #[test]
    fn test_srgb_channel_to_linear_zero() {
        assert!((Color::srgb_channel_to_linear(0.0) - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_srgb_channel_to_linear_one() {
        assert!((Color::srgb_channel_to_linear(1.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_srgb_channel_to_linear_low_value() {
        // Below threshold 0.04045 => c / 12.92
        let c = 0.01;
        let expected = 0.01 / 12.92;
        assert!((Color::srgb_channel_to_linear(c) - expected).abs() < 1e-10);
    }

    #[test]
    fn test_srgb_channel_to_linear_high_value() {
        // Above threshold
        let c = 0.5;
        let expected = ((0.5 + 0.055) / 1.055_f32).powf(2.4);
        assert!((Color::srgb_channel_to_linear(c) - expected).abs() < 1e-6);
    }

    #[test]
    fn test_linear_channel_to_srgb_zero() {
        assert!((Color::linear_channel_to_srgb(0.0) - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_linear_channel_to_srgb_one() {
        assert!((Color::linear_channel_to_srgb(1.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_linear_channel_to_srgb_low_value() {
        // Below threshold 0.0031308 => c * 12.92
        let c = 0.001;
        let expected = 0.001 * 12.92;
        assert!((Color::linear_channel_to_srgb(c) - expected).abs() < 1e-10);
    }

    #[test]
    fn test_linear_channel_to_srgb_high_value() {
        let c = 0.5;
        let expected = 1.055 * (0.5_f32).powf(1.0 / 2.4) - 0.055;
        assert!((Color::linear_channel_to_srgb(c) - expected).abs() < 1e-6);
    }

    #[test]
    fn test_srgb_linear_channel_roundtrip() {
        for val in [0.0, 0.01, 0.1, 0.5, 0.9, 1.0_f32] {
            let linear = Color::srgb_channel_to_linear(val);
            let back = Color::linear_channel_to_srgb(linear);
            assert!((back - val).abs() < 1e-6, "roundtrip failed for {val}: got {back}");
        }
    }

    #[test]
    fn test_from_u8_array() {
        let c: Color = [10u8, 20, 30, 40].into();
        assert_eq!(c.rgba[0], 10);
        assert_eq!(c.rgba[1], 20);
        assert_eq!(c.rgba[2], 30);
        assert_eq!(c.rgba[3], 40);
    }

    #[test]
    fn test_into_u8_array() {
        let c = Color::new(10, 20, 30, 40);
        let arr: [u8; 4] = c.into();
        assert_eq!(arr, [10, 20, 30, 40]);
    }

    #[test]
    fn test_u8_array_roundtrip() {
        let original = [100u8, 150, 200, 255];
        let c: Color = original.into();
        let back: [u8; 4] = c.into();
        assert_eq!(back, original);
    }

}


