use std::hash::{Hash, Hasher};
use glam::Vec4;
use serde::{Serialize, Deserialize};

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

    /// Convert a single linear channel in [0,1] to sRGB space.
    /// Inverse of `srgb_channel_to_linear` using the standard sRGB transfer function.
    fn linear_channel_to_srgb(c: f32) -> f32 {
        if c <= 0.0031308 {
            c * 12.92
        } else {
            1.055 * c.powf(1.0 / 2.4) - 0.055
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

    pub fn to_hex(&self) -> String {
        let mut s = String::new();
        // write_hex returns a fmt::Result; unwrap here is fine for an in-memory String
        self.write_hex(&mut s).unwrap();
        s
    }

    /// Write the hex representation (sRGB) into any `std::fmt::Write`.
    /// Useful to append into an existing `String` without allocating a new one.
    pub fn write_hex<W: std::fmt::Write>(&self, out: &mut W) -> std::fmt::Result {
        let r_lin = self.v[0].clamp(0.0, 1.0);
        let g_lin = self.v[1].clamp(0.0, 1.0);
        let b_lin = self.v[2].clamp(0.0, 1.0);
        let a = (self.v[3].clamp(0.0, 1.0) * 255.0).round() as u8;

        let r = (Self::linear_channel_to_srgb(r_lin) * 255.0).round().clamp(0.0, 255.0) as u8;
        let g = (Self::linear_channel_to_srgb(g_lin) * 255.0).round().clamp(0.0, 255.0) as u8;
        let b = (Self::linear_channel_to_srgb(b_lin) * 255.0).round().clamp(0.0, 255.0) as u8;

        write!(out, "#{:02X}{:02X}{:02X}{:02X}", r, g, b, a)
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

#[cfg(test)]
mod tests {
    use super::Color;

    #[test]
    fn srgb_channel_roundtrip() {
        let samples = [0.0_f32, 0.001, 0.0031308, 0.02, 0.18, 0.5, 1.0];
        let eps = 1e-6_f32;

        for &s in &samples {
            let lin = Color::srgb_channel_to_linear(s);
            let back = Color::linear_channel_to_srgb(lin);
            let diff = (s - back).abs();
            assert!(diff <= eps, "srgb->linear->srgb mismatch: s={} back={} diff={}", s, back, diff);
        }

        // also test some linear-starting values -> srgb -> linear
        let lin_samples = [0.0_f32, 1e-6, 0.01, 0.1, 0.5, 1.0];
        for &l in &lin_samples {
            let s = Color::linear_channel_to_srgb(l);
            let back = Color::srgb_channel_to_linear(s);
            let diff = (l - back).abs();
            assert!(diff <= eps, "linear->srgb->linear mismatch: l={} back={} diff={}", l, back, diff);
        }
    }

    #[test]
    fn hex_roundtrip() {
        let samples = [
            Color::new(0.0, 0.0, 0.0, 1.0),
            Color::new(1.0, 1.0, 1.0, 1.0),
            Color::new(0.18, 0.2, 0.5, 0.75),
            Color::new(0.003, 0.001, 0.5, 0.0),
        ];

        // allow for 8-bit quantization + small numeric error
        let eps = 0.006_f32;

        for &c in &samples {
            let hex = c.to_hex();
            let parsed = Color::from_hex(&hex).expect("from_hex failed");
            for i in 0..4 {
                let a = c.v[i];
                let b = parsed.v[i];
                let diff = (a - b).abs();
                assert!(diff <= eps, "to_hex/from_hex roundtrip mismatch idx={} a={} b={} diff={} hex={}", i, a, b, diff, hex);
            }
        }
    }
}
