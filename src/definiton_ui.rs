use std::{hash::{DefaultHasher, Hasher}, str::FromStr, convert::AsRef};

use egui::Button;
use strum::VariantNames;

use crate::{IMG_SIZE, definition::{self, TextureDefinition, TexturePass}};

pub enum PassOperation { Remove(usize) }

fn add_full_width<T: egui::Widget>(ui: &mut egui::Ui, widget: T) -> egui::Response {
    let available_width = ui.available_width();
    ui.add_sized([available_width, 0.0], widget)
}

pub fn add_enum_dropdown<T: AsRef<str> + FromStr + VariantNames>(ui: &mut egui::Ui, value: &mut T, hash_str: &str, hash_idx: usize)
where <T as FromStr>::Err: std::fmt::Debug
{
    let mut salt_hasher = DefaultHasher::new();
    salt_hasher.write(hash_str.as_bytes());
    salt_hasher.write("dropdown".as_bytes());
    salt_hasher.write_u64(hash_idx as u64);
    let combo_box_id = salt_hasher.finish();

    egui::ComboBox::from_id_salt(combo_box_id)
        .selected_text(value.as_ref())
        .show_ui(ui, |ui| {
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

pub fn generate_ui_for_generator_option(generator: &mut definition::GeneratorOption, ui: &mut egui::Ui) {
    match generator {
        definition::GeneratorOption::SolidColor(solid_color_gen) => {
            ui.horizontal(|ui| {
                ui.label("Color:");
                ui.color_edit_button_rgba_unmultiplied(&mut solid_color_gen.color.v).changed();
            });
        }
    }
}

pub fn definition_ui(def: &mut TextureDefinition, tmp_str: &mut String, ui: &mut egui::Ui) {
    ui.heading(&def.name);
    ui.horizontal(| ui | {
        ui.label("Background:");
        ui.color_edit_button_rgba_unmultiplied(&mut def.background.v);
    });

    let mut pass_op = Option::<PassOperation>::None;
    for (pass_idx, pass) in def.passes.iter_mut().enumerate() {
        ui.group(| ui | {
            ui.horizontal(| ui | {
                tmp_str.clear();
                match &pass.name {
                    Some(name) => tmp_str.push_str(name),
                    None => tmp_str.push_str(pass.generator.as_ref()),
                }
                if ui.text_edit_singleline(tmp_str).changed() {
                    match &mut pass.name {
                        Some(name) => {
                            name.clear();
                            name.push_str(tmp_str);
                        },
                        None => pass.name = Some(tmp_str.clone()),
                    }
                }
                if ui.button("Reset").clicked() {
                    pass.name = None;
                }
            });

            add_enum_dropdown(ui, &mut pass.generator, "Generator", pass_idx);
            generate_ui_for_generator_option(&mut pass.generator, ui);
            ui.separator();
            ui.horizontal(| ui | {
                ui.label("Blend:");
                add_enum_dropdown(ui, &mut pass.blend_mode, "blend_mode", pass_idx);
            });

            let mut use_rect = pass.rect.is_some();
            if ui.checkbox(&mut use_rect, "Use Rect").changed() {
                if use_rect {
                    pass.rect = Some(definition::Rect::new(IMG_SIZE / 4, IMG_SIZE / 4, IMG_SIZE / 2, IMG_SIZE / 2));
                } else {
                    pass.rect = None;
                }
            }
            ui.horizontal(| ui | {
                if let Some(rect) = &mut pass.rect {
                    ui.label("X:");
                    ui.add(egui::DragValue::new(&mut rect.x).range(0..=(IMG_SIZE - 1)));
                    ui.label("Y:");
                    ui.add(egui::DragValue::new(&mut rect.y).range(0..=(IMG_SIZE - 1)));
                    ui.label("W:");
                    ui.add(egui::DragValue::new(&mut rect.w).range(1..=IMG_SIZE));
                    ui.label("H:");
                    ui.add(egui::DragValue::new(&mut rect.h).range(1..=IMG_SIZE));

                    let mut r = rect.x + rect.w;
                    ui.label("R:");
                    if ui.add(egui::DragValue::new(&mut r).range(1..=(IMG_SIZE))).changed() {
                        rect.x = r - rect.w;
                    }
                    let mut b = rect.y + rect.h;
                    ui.label("B:");
                    if ui.add(egui::DragValue::new(&mut b).range(1..=(IMG_SIZE))).changed() {
                        rect.y = b - rect.h;
                    }
                }
            });
            if add_full_width(ui, Button::new("Remove")).clicked() {
                pass_op = Some(PassOperation::Remove(pass_idx));
            }
        });
    }

    if let Some(op) = pass_op {
        match op {
            PassOperation::Remove(idx) => {
                def.passes.remove(idx);
            }
        }
    }


    if add_full_width(ui, Button::new("Add Pass")).clicked() {
        def.passes.push(TexturePass::default());
    }
}
