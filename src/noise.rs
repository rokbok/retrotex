use std::hash::Hasher as _;

use twox_hash::XxHash32;

const PERMUTATION: [u32; 256] = [ 151, 160, 137,  91,  90,  15, 131,  13, 201,  95,  96,  53, 194, 233,   7, 225,
                      140,  36, 103,  30,  69, 142,   8,  99,  37, 240,  21,  10,  23, 190,   6, 148,
                      247, 120, 234,  75,   0,  26, 197,  62,  94, 252, 219, 203, 117,  35,  11,  32,
                       57, 177,  33,  88, 237, 149,  56,  87, 174,  20, 125, 136, 171, 168,  68, 175,
                       74, 165,  71, 134, 139,  48,  27, 166,  77, 146, 158, 231,  83, 111, 229, 122,
                       60, 211, 133, 230, 220, 105,  92,  41,  55,  46, 245,  40, 244, 102, 143,  54,
                       65,  25,  63, 161,   1, 216,  80,  73, 209,  76, 132, 187, 208,  89,  18, 169,
                      200, 196, 135, 130, 116, 188, 159,  86, 164, 100, 109, 198, 173, 186,   3,  64,
                       52, 217, 226, 250, 124, 123,   5, 202,  38, 147, 118, 126, 255,  82,  85, 212,
                      207, 206,  59, 227,  47,  16,  58,  17, 182, 189,  28,  42, 223, 183, 170, 213,
                      119, 248, 152,   2,  44, 154, 163,  70, 221, 153, 101, 155, 167,  43, 172,   9,
                      129,  22,  39, 253,  19,  98, 108, 110,  79, 113, 224, 232, 178, 185, 112, 104,
                      218, 246,  97, 228, 251,  34, 242, 193, 238, 210, 144,  12, 191, 179, 162, 241,
                       81,  51, 145, 235, 249,  14, 239, 107,  49, 192, 214,  31, 181, 199, 106, 157,
                      184,  84, 204, 176, 115, 121,  50,  45, 127,   4, 150, 254, 138, 236, 205,  93,
                      222, 114,  67,  29,  24,  72, 243, 141, 128, 195,  78,  66, 215,  61, 156, 180 ];

#[inline]
fn fade(t: f32) -> f32 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

#[inline]
fn lerp(t: f32, a: f32, b: f32) -> f32 {
    a + t * (b - a)
}

#[inline]
fn grad(hash: u32, x: f32, y: f32, z: f32) -> f32 {
    let h = (hash & 15) as i32;
    let u = if h < 8 { x } else { y };
    let v = if h < 4 {
        y
    } else if h == 12 || h == 14 {
        x
    } else {
        z
    };

    let u = if (h & 1) == 0 { u } else { -u };
    let v = if (h & 2) == 0 { v } else { -v };
    u + v
}

#[inline]
fn perm(i: i32) -> u32 {
    PERMUTATION[(i as usize) & 255]
}

/// 3D Improved Perlin noise in range approximately [-1, 1].
pub fn noise3(x: f32, y: f32, z: f32) -> f32 {
    let xi = (x.floor() as i32) & 255;
    let yi = (y.floor() as i32) & 255;
    let zi = (z.floor() as i32) & 255;

    let xf = x - x.floor();
    let yf = y - y.floor();
    let zf = z - z.floor();

    let u = fade(xf);
    let v = fade(yf);
    let w = fade(zf);

    let a = perm(xi) as i32 + yi;
    let aa = perm(a) as i32 + zi;
    let ab = perm(a + 1) as i32 + zi;
    let b = perm(xi + 1) as i32 + yi;
    let ba = perm(b) as i32 + zi;
    let bb = perm(b + 1) as i32 + zi;

    let x1 = lerp(
        u,
        grad(perm(aa), xf, yf, zf),
        grad(perm(ba), xf - 1.0, yf, zf),
    );
    let x2 = lerp(
        u,
        grad(perm(ab), xf, yf - 1.0, zf),
        grad(perm(bb), xf - 1.0, yf - 1.0, zf),
    );
    let y1 = lerp(v, x1, x2);

    let x3 = lerp(
        u,
        grad(perm(aa + 1), xf, yf, zf - 1.0),
        grad(perm(ba + 1), xf - 1.0, yf, zf - 1.0),
    );
    let x4 = lerp(
        u,
        grad(perm(ab + 1), xf, yf - 1.0, zf - 1.0),
        grad(perm(bb + 1), xf - 1.0, yf - 1.0, zf - 1.0),
    );
    let y2 = lerp(v, x3, x4);

    lerp(w, y1, y2)
}

/// 2D Perlin noise helper (z fixed at 0.0).
pub fn noise2(x: f32, y: f32) -> f32 {
    noise3(x, y, 0.0)
}

/// Fractal Brownian Motion using Perlin noise.
/// Output is normalized to roughly [-1, 1].
pub fn fbm2(x: f32, y: f32, octaves: u32, lacunarity: f32, gain: f32, seed: f32) -> f32 {
    let mut sum = 0.0;
    let mut amp = 1.0;
    let mut freq = 1.0;
    let mut norm = 0.0;

    for octave in 0..octaves {
        sum += noise3(x * freq, y * freq, seed + octave as f32) * amp;
        norm += amp;
        amp *= gain;
        freq *= lacunarity;
    }

    if norm > 0.0 { sum / norm } else { 0.0 }
}

#[inline]
fn hash_to_unit_f32(h: u64) -> f32 {
    let x = h as u32 | (h >> 32) as u32;
    let mant = x >> 8;
    (mant as f32) * (1.0 / 16_777_216.0)
}

pub fn white_noise(mut x: i32, mut y: i32, scale: i32, seed: u32) -> f32 {
    if scale > 1 {
        x = x / scale;
        y = y / scale;
    }

    let mut hasher = XxHash32::with_seed(seed);
    hasher.write_i32(x);
    hasher.write_i32(y);
    hash_to_unit_f32(hasher.finish())
}

pub fn gaussian(x: i32, y: i32, scale: i32, seed: u32) -> f32 {
    // Box-Muller transform
    let u1 = white_noise(x, y, scale, seed);
    let u2 = white_noise(x, y, scale, seed ^ 0xC99DFD3A);
    let r = (-2.0 * u1.ln()).sqrt();
    let theta = 2.0 * std::f32::consts::PI * u2;
    r * theta.cos() // Standard normal distribution
}
