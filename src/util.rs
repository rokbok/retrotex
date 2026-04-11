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
}

