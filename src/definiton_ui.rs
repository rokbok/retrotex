use std::hash::{DefaultHasher, Hasher};

use egui::Button;

use crate::definition::{self, Color, GeneratorOption, SolidColorGenerator, TextureDefinition, TexturePass};

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
                if ui.button("X").clicked() {
                    pass.name = None;
                }
            });

            show_dropdown(ui, &mut pass.generator, "Generator", pass_idx);
            generate_ui_for_generator_option(&mut pass.generator, ui);
            ui.separator();
            show_dropdown(ui, &mut pass.blend_mode, "Blend Mode", pass_idx);
        });
    }


    if add_full_width(ui, Button::new("Add Pass")).clicked() {
        def.passes.push(TexturePass::default());
    }
}
