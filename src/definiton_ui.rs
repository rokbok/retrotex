use std::hash::{DefaultHasher, Hasher};

use egui::Button;

use crate::{IMG_SIZE, definition::{self, Color, GeneratorOption, SolidColorGenerator, TextureDefinition, TexturePass}};

pub enum PassOperation { Remove(usize) }

fn add_full_width<T: egui::Widget>(ui: &mut egui::Ui, widget: T) -> egui::Response {
    let available_width = ui.available_width();
    ui.add_sized([available_width, 0.0], widget)
}

pub trait DropdownableEnum {
    fn type_index(&self) -> i32;
    fn default_for_type_index(type_index: i32) -> Self where Self: Sized;
    fn name_for_type_index(type_index: i32) -> &'static str;
    fn option_count() -> i32 where Self: Sized;

    fn type_name(&self) -> &'static str {
        Self::name_for_type_index(self.type_index())
    }
}

impl DropdownableEnum for GeneratorOption {
    fn type_index(&self) -> i32 {
        match self {
            GeneratorOption::SolidColor(_) => 0,
        }
    }

    fn default_for_type_index(type_index: i32) -> Self {
        match type_index {
            0 => GeneratorOption::SolidColor(SolidColorGenerator { color: Color::new(1.0, 0.0, 0.0, 1.0) }),
            _ => panic!("Invalid type index for GeneratorOption: {}", type_index),
        }
    }

    fn name_for_type_index(type_index: i32) -> &'static str {
        match type_index {
            0 => "Solid Color",
            _ => "Unknown",
        }
    }

    fn option_count() -> i32 {
        1
    }
}

impl DropdownableEnum for definition::BlendMode {
    fn type_index(&self) -> i32 {
        match self {
            definition::BlendMode::Normal => 0,
            definition::BlendMode::Additive => 1,
            definition::BlendMode::Multiply => 2,
        }
    }

    fn default_for_type_index(type_index: i32) -> Self {
        match type_index {
            0 => definition::BlendMode::Normal,
            1 => definition::BlendMode::Additive,
            2 => definition::BlendMode::Multiply,
            _ => panic!("Invalid type index for BlendMode: {}", type_index),
        }
    }

    fn name_for_type_index(type_index: i32) -> &'static str {
        match type_index {
            0 => "Normal",
            1 => "Additive",
            2 => "Multiply",
            _ => "Unknown",
        }
    }

    fn option_count() -> i32 {
        3
    }
}

pub fn show_dropdown<T: DropdownableEnum>(ui: &mut egui::Ui, value: &mut T, label: &str, idx: usize) {
    let mut salt_hasher = DefaultHasher::new();
    salt_hasher.write(label.as_bytes());
    salt_hasher.write("dropdown".as_bytes());
    salt_hasher.write_u64(idx as u64);
    let combo_box_id = salt_hasher.finish();

    egui::ComboBox::from_id_salt(combo_box_id)
        .selected_text(value.type_name())
        .show_ui(ui, |ui| {
            let before_idx = value.type_index();
            let mut selected_idx = before_idx;
            for i in 0..T::option_count() {
                let name = T::name_for_type_index(i);
                ui.selectable_value(&mut selected_idx, i, name);
            }
            if selected_idx != before_idx {
                *value = T::default_for_type_index(selected_idx);
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

    let mut pass_op = Option::<PassOperation>::None;
    for (pass_idx, pass) in def.passes.iter_mut().enumerate() {
        ui.group(| ui | {
            ui.horizontal(| ui | {
                tmp_str.clear();
                match &pass.name {
                    Some(name) => tmp_str.push_str(name),
                    None => tmp_str.push_str(pass.generator.type_name()),
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

            show_dropdown(ui, &mut pass.generator, "Generator", pass_idx);
            generate_ui_for_generator_option(&mut pass.generator, ui);
            ui.separator();
            ui.horizontal(| ui | {
                ui.label("Blend:");
                show_dropdown(ui, &mut pass.blend_mode, "Blend Mode", pass_idx);
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
