use glam::Vec3;

use crate::{IMG_PIXEL_COUNT, IMG_SIZE, idx};


pub struct TextureLayers {
    pub albedo: Box<[Vec3; IMG_PIXEL_COUNT]>,
    pub depth: Box<[f32; IMG_PIXEL_COUNT]>,
    pub normal: Box<[Vec3; IMG_PIXEL_COUNT]>,
}

fn calculate_normals(depth: &[f32; IMG_PIXEL_COUNT]) -> Box<[Vec3; IMG_PIXEL_COUNT]> {
    // Y points up, Unity-style
    let mut normals = Box::new([Vec3::ZERO; IMG_PIXEL_COUNT]);
    for y in 0..IMG_SIZE {
        for x in 0..IMG_SIZE {
            let index = idx(x, y);
            let left  = if x > 0 {            depth[idx(x - 1, y)] } else { depth[index] };
            let right = if x < IMG_SIZE - 1 { depth[idx(x + 1, y)] } else { depth[index] };
            let up    = if y > 0 {            depth[idx(x, y - 1)] } else { depth[index] };
            let down  = if y < IMG_SIZE - 1 { depth[idx(x, y + 1)] } else { depth[index] };

            let normal = Vec3::new(right - left, up - down, 1.0).normalize();
            normals[index] = normal;
        }
    }
    normals
}

impl Default for TextureLayers {
    fn default() -> Self {
        Self {
            albedo: Box::new([Vec3::ZERO; IMG_PIXEL_COUNT]),
            depth: Box::new([0.0; IMG_PIXEL_COUNT]),
            normal: Box::new([Vec3::new(0.0, 0.0, 1.0); IMG_PIXEL_COUNT]),
        }
    }
}

impl TextureLayers {
    pub fn recalculate(&mut self) {
        self.normal = calculate_normals(&self.depth);
    }
}
