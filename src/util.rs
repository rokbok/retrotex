use std::{hash::{DefaultHasher, Hash, Hasher}, str::FromStr};

use glam::{IVec2, Vec2};
use strum::VariantNames;

use crate::prelude::*;


#[derive(Debug, Clone)]
pub struct LineIterator {
    current: IVec2,
    end: IVec2,
    step: IVec2,
    dx: i32,
    dy: i32,
    err: i32,
    done: bool,
}

impl LineIterator {
    pub fn new(start: IVec2, end: IVec2) -> Self {
        let current = start;
        let end = end;
        let dx = (end.x - current.x).abs();
        let dy = (end.y - current.y).abs();
        let step = IVec2::new(
            if current.x < end.x { 1 } else { -1 },
            if current.y < end.y { 1 } else { -1 },
        );

        Self {
            current,
            end,
            step,
            dx,
            dy,
            err: dx - dy,
            done: false,
        }
    }
}

impl Iterator for LineIterator {
    type Item = IVec2;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        let point = self.current;
        if self.current == self.end {
            self.done = true;
            return Some(point);
        }

        let err2 = self.err * 2;
        if err2 > -self.dy {
            self.err -= self.dy;
            self.current.x += self.step.x;
        }
        if err2 < self.dx {
            self.err += self.dx;
            self.current.y += self.step.y;
        }

        Some(point)
    }
}

#[derive(Debug, Clone)]
pub struct RayIterator {
    current: IVec2,
    step: IVec2,
    t_max: Vec2,
    t_delta: Vec2,
    done: bool,
}

impl RayIterator {
    pub fn new(start: IVec2, direction: Vec2) -> Self {
        let step = IVec2::new(
            if direction.x > 0.0 { 1 } else if direction.x < 0.0 { -1 } else { 0 },
            if direction.y > 0.0 { 1 } else if direction.y < 0.0 { -1 } else { 0 },
        );

        let t_delta = Vec2::new(
            if step.x == 0 { f32::INFINITY } else { 1.0 / direction.x.abs() },
            if step.y == 0 { f32::INFINITY } else { 1.0 / direction.y.abs() },
        );

        let t_max = Vec2::new(
            if step.x == 0 { f32::INFINITY } else { 0.5 / direction.x.abs() },
            if step.y == 0 { f32::INFINITY } else { 0.5 / direction.y.abs() },
        );

        Self {
            current: start,
            step,
            t_max,
            t_delta,
            done: !in_bounds(start),
        }
    }
}

impl Iterator for RayIterator {
    type Item = IVec2;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        let point = self.current;

        if self.step.x == 0 && self.step.y == 0 {
            self.done = true;
            return Some(point);
        }

        if self.t_max.x < self.t_max.y {
            self.current.x += self.step.x;
            self.t_max.x += self.t_delta.x;
        } else if self.t_max.y < self.t_max.x {
            self.current.y += self.step.y;
            self.t_max.y += self.t_delta.y;
        } else {
            self.current.x += self.step.x;
            self.current.y += self.step.y;
            self.t_max.x += self.t_delta.x;
            self.t_max.y += self.t_delta.y;
        }

        if !in_bounds(self.current) {
            self.done = true;
        }

        Some(point)
    }
}

#[inline]
fn in_bounds(p: IVec2) -> bool {
    p.x >= 0 && p.y >= 0 && p.x < IMG_SIZE && p.y < IMG_SIZE
}



pub fn idx(x: i32, y: i32) -> usize {
    (y * IMG_SIZE + x) as usize
}

pub fn idx_safe(x: i32, y: i32) -> usize {
    let x = x.clamp(0, IMG_SIZE - 1);
    let y = y.clamp(0, IMG_SIZE - 1);
    idx(x, y)
}

pub fn idx2coords(index: usize) -> (i32, i32) {
    let x = (index as i32) % IMG_SIZE;
    let y = (index as i32) / IMG_SIZE;
    (x, y)
}

pub fn single_hash<T: Hash>(t: &T) -> u64 {
    let mut hasher = DefaultHasher::new();
    t.hash(&mut hasher);
    hasher.finish()
}

pub fn box_sdf(p: Vec2, b: Vec2) -> f32 {
    let d = p.abs() - b;
    d.max(Vec2::ZERO).length() + d.x.max(d.y).min(0.0)
}

pub fn add_enum_dropdown<T: AsRef<str> + FromStr + VariantNames>(ui: &mut egui::Ui, value: &mut T, hash_str: &str, hash_idx: usize, full_width: bool)
where <T as FromStr>::Err: std::fmt::Debug
{
    let mut salt_hasher = DefaultHasher::new();
    salt_hasher.write(hash_str.as_bytes());
    salt_hasher.write("dropdown".as_bytes());
    salt_hasher.write_u64(hash_idx as u64);
    let combo_box_id = salt_hasher.finish();

    let mut combo_box = egui::ComboBox::from_id_salt(combo_box_id)
        .selected_text(value.as_ref());
    if full_width {
        combo_box = combo_box.width(ui.available_width());
    }
    combo_box.show_ui(ui, |ui| {
            let mut selected = value.as_ref();
            let mut changed = false;
            for name in T::VARIANTS {
                changed |= ui.selectable_value(&mut selected, name, *name).changed();
            }
            if changed {
                *value = T::from_str(selected).expect("Selected value should always be valid");
            }
        });
}

#[inline]
pub fn gaussian_kernel_weight(distance: f32, sigma: f32) -> f32 {
    if sigma <= 0.0 {
        return if distance == 0.0 { 1.0 } else { 0.0 };
    }
    let exponent = -(distance * distance) / (2.0 * sigma * sigma);
    exponent.exp()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pts(points: &[(i32, i32)]) -> Vec<IVec2> {
        points.iter().map(|&(x, y)| IVec2::new(x, y)).collect()
    }

    #[test]
    fn line_iterator_horizontal() {
        let got: Vec<IVec2> = LineIterator::new(IVec2::new(1, 2), IVec2::new(4, 2)).collect();
        let expected = pts(&[(1, 2), (2, 2), (3, 2), (4, 2)]);
        assert_eq!(got, expected);
    }

    #[test]
    fn line_iterator_vertical() {
        let got: Vec<IVec2> = LineIterator::new(IVec2::new(3, 1), IVec2::new(3, 4)).collect();
        let expected = pts(&[(3, 1), (3, 2), (3, 3), (3, 4)]);
        assert_eq!(got, expected);
    }

    #[test]
    fn line_iterator_diagonal() {
        let got: Vec<IVec2> = LineIterator::new(IVec2::new(1, 1), IVec2::new(4, 4)).collect();
        let expected = pts(&[(1, 1), (2, 2), (3, 3), (4, 4)]);
        assert_eq!(got, expected);
    }

    #[test]
    fn line_iterator_steep_line() {
        let got: Vec<IVec2> = LineIterator::new(IVec2::new(1, 1), IVec2::new(3, 6)).collect();
        let expected = pts(&[(1, 1), (1, 2), (2, 3), (2, 4), (3, 5), (3, 6)]);
        assert_eq!(got, expected);
    }

    #[test]
    fn line_iterator_reverse_direction() {
        let got: Vec<IVec2> = LineIterator::new(IVec2::new(4, 2), IVec2::new(1, 2)).collect();
        let expected = pts(&[(4, 2), (3, 2), (2, 2), (1, 2)]);
        assert_eq!(got, expected);
    }

    #[test]
    fn line_iterator_single_point() {
        let got: Vec<IVec2> = LineIterator::new(IVec2::new(2, 2), IVec2::new(2, 2)).collect();
        let expected = pts(&[(2, 2)]);
        assert_eq!(got, expected);
    }

    #[test]
    fn ray_iterator_right_to_edge() {
        let got: Vec<IVec2> = RayIterator::new(IVec2::new(0, 0), Vec2::new(1.0, 0.0)).collect();
        assert_eq!(got.len(), IMG_SIZE as usize);
        assert_eq!(got.first(), Some(&IVec2::new(0, 0)));
        assert_eq!(got.last(), Some(&IVec2::new(IMG_SIZE - 1, 0)));
    }

    #[test]
    fn ray_iterator_diagonal_to_corner() {
        let got: Vec<IVec2> = RayIterator::new(IVec2::new(0, 0), Vec2::new(1.0, 1.0)).collect();
        assert_eq!(got.len(), IMG_SIZE as usize);
        assert_eq!(got.first(), Some(&IVec2::new(0, 0)));
        assert_eq!(got.last(), Some(&IVec2::new(IMG_SIZE - 1, IMG_SIZE - 1)));
        assert!(got.iter().all(|p| p.x == p.y));
    }

    #[test]
    fn ray_iterator_up_to_top_edge() {
        let start = IVec2::new(8, 10);
        let got: Vec<IVec2> = RayIterator::new(start, Vec2::new(0.0, -1.0)).collect();
        assert_eq!(got.len(), (start.y + 1) as usize);
        assert_eq!(got.first(), Some(&start));
        assert_eq!(got.last(), Some(&IVec2::new(8, 0)));
    }

    #[test]
    fn ray_iterator_zero_direction_single_point() {
        let start = IVec2::new(5, 6);
        let got: Vec<IVec2> = RayIterator::new(start, Vec2::ZERO).collect();
        assert_eq!(got, vec![start]);
    }

    #[test]
    fn ray_iterator_start_out_of_bounds_is_empty() {
        let got: Vec<IVec2> = RayIterator::new(IVec2::new(-1, 0), Vec2::new(1.0, 0.0)).collect();
        assert!(got.is_empty());
    }

    #[test]
    fn gaussian_kernel_weight_center_is_one() {
        let w = gaussian_kernel_weight(0.0, 2.0);
        assert!((w - 1.0).abs() < 1e-6);
    }

    #[test]
    fn gaussian_kernel_weight_matches_known_value() {
        let w = gaussian_kernel_weight(1.0, 1.0);
        let expected = (-0.5_f32).exp();
        assert!((w - expected).abs() < 1e-6);
    }

    #[test]
    fn gaussian_kernel_weight_decreases_with_distance() {
        let sigma = 2.0;
        let w0 = gaussian_kernel_weight(0.0, sigma);
        let w1 = gaussian_kernel_weight(1.0, sigma);
        let w2 = gaussian_kernel_weight(2.0, sigma);
        assert!(w0 > w1 && w1 > w2);
    }

    #[test]
    fn gaussian_kernel_weight_is_symmetric() {
        let sigma = 3.0;
        let wp = gaussian_kernel_weight(2.5, sigma);
        let wn = gaussian_kernel_weight(-2.5, sigma);
        assert!((wp - wn).abs() < 1e-6);
    }

    #[test]
    fn gaussian_kernel_weight_non_positive_sigma_behavior() {
        assert_eq!(gaussian_kernel_weight(0.0, 0.0), 1.0);
        assert_eq!(gaussian_kernel_weight(1.0, 0.0), 0.0);
        assert_eq!(gaussian_kernel_weight(0.0, -1.0), 1.0);
        assert_eq!(gaussian_kernel_weight(2.0, -1.0), 0.0);
    }
}

