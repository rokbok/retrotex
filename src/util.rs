use std::hash::{DefaultHasher, Hash, Hasher};

use glam::{Vec2, Vec4};

pub fn quick_hash<T: Hash>(t: &T) -> u64 {
    let mut hasher = DefaultHasher::new();
    t.hash(&mut hasher);
    hasher.finish()
}

pub fn box_sdf(p: Vec2, b: Vec2) -> f32 {
    let d = p.abs() - b;
    d.max(Vec2::ZERO).length() + d.x.max(d.y).min(0.0)
}

#[inline]
pub fn retain_min_abs(v: Vec4) -> Vec4 {
    let vabs = v.abs();
    let mnel = vabs.min_element();
    Vec4::select(vabs.cmpeq(Vec4::splat(mnel)), v, Vec4::ZERO)
}
