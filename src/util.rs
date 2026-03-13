use std::{hash::{DefaultHasher, Hash, Hasher}, str::FromStr};

use glam::{Vec2, Vec4};
use strum::VariantNames;

pub fn single_hash<T: Hash>(t: &T) -> u64 {
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

