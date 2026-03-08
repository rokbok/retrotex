use std::{convert::AsRef, fmt::Write, hash::{DefaultHasher, Hasher}, mem::swap, str::FromStr};

use egui::Button;
use strum::VariantNames;

use crate::{IMG_SIZE, definition::{Rect, TextureDefinition, TexturePass}, color::Color};

pub enum PassOperation { Remove(usize) }

fn add_full_width<T: egui::Widget>(ui: &mut egui::Ui, widget: T) -> egui::Response {
    let available_width = ui.available_width();
    ui.add_sized([available_width, 0.0], widget)
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

pub fn definition_ui(def: &mut TextureDefinition, tmp_str: &mut String, ui: &mut egui::Ui, clipboard: &mut arboard::Clipboard) {
    ui.heading(&def.name);
    ui.horizontal(| ui | {
        ui.label("Background:");
        ui.color_edit_button_rgba_unmultiplied(&mut def.background.v);
    });

    let mut pass_op = Option::<PassOperation>::None;
    for (pass_idx, pass) in def.passes.iter_mut().enumerate() {
        ui.group(| ui | {
            tmp_str.clear();
            match &pass.name {
                Some(name) => tmp_str.push_str(name),
                None => {
                    tmp_str.clear();
                    write!(tmp_str, "Pass {}", pass_idx).unwrap();
                },
            }

            let name_response = add_full_width(ui, egui::TextEdit::singleline(tmp_str).hint_text("Pass Name"));
            if name_response.changed() {
                match &mut pass.name {
                    Some(name) => {
                        name.clear();
                        name.push_str(tmp_str);
                    },
                    None => pass.name = Some(tmp_str.clone()),
                }
            }

            if tmp_str.is_empty() && !name_response.has_focus() {
                pass.name = None;
            }

            ui.horizontal_wrapped(| ui | {
                ui.color_edit_button_rgba_unmultiplied(&mut pass.color.v);
                tmp_str.clear();
                pass.color.write_hex(tmp_str).expect("Color string conversion failed");
                ui.label(&*tmp_str);
                if ui.button("Copy").clicked() {
                    ui.ctx().copy_text(tmp_str.clone());
                }
                if ui.button("Paste").clicked() {
                    if let Ok(clipboard_str) = clipboard.get_text() {
                        if let Ok(color) = Color::from_hex(&clipboard_str) {
                            pass.color.v[..3].copy_from_slice(&color.v[..3]);
                        }
                    }
                }
            });

            ui.horizontal( | ui | {
                ui.label("Blend:");
                add_enum_dropdown(ui, &mut pass.blend_mode, "blend_mode", pass_idx, false);
            });


            ui.horizontal_wrapped(| ui | { 
                ui.checkbox(&mut pass.perlin, "Perlin");
                if pass.perlin {
                    ui.label("Scale:");
                    ui.add(egui::DragValue::new(&mut pass.perlin_scale).range(1..=400));
                    ui.label("Octaves:");
                    ui.add(egui::DragValue::new(&mut pass.perlin_octaves));
                    ui.checkbox(&mut pass.perlin_use_threshold, "Threshold");
                    if pass.perlin_use_threshold {
                        ui.add(egui::DragValue::new(&mut pass.perlin_threshold).range(0..=100));
                    }
                    if ui.button("Re-seed").clicked() {
                        pass.perlin_seed = rand::random();
                    }
                }
            });
            ui.horizontal_wrapped(| ui | {
                ui.checkbox(&mut pass.white_noise, "White Noise");
                if pass.white_noise {
                    ui.label("Scale:");
                    ui.add(egui::DragValue::new(&mut pass.white_noise_scale).range(1..=(IMG_SIZE/2)));
                    ui.checkbox(&mut pass.white_noise_use_threshold, "Threshold");
                    if pass.white_noise_use_threshold {
                        ui.add(egui::DragValue::new(&mut pass.white_noise_threshold).range(0..=100));
                    }
                    if ui.button("Re-seed").clicked() {
                        pass.white_noise_seed = rand::random();
                    }
                }
            });

            ui.separator();
            ui.horizontal_wrapped(| ui | {
                ui.label("Rect:");
                let mut l = pass.rect.x;
                ui.label("L:");
                if ui.add(egui::DragValue::new(&mut l).range((-IMG_SIZE + 1)..=(pass.rect.x + pass.rect.w - 1))).changed() {
                    pass.rect.w += pass.rect.x - l;
                    pass.rect.x = l;
                }
                let mut t = pass.rect.y;
                ui.label("T:");
                if ui.add(egui::DragValue::new(&mut t).range((-IMG_SIZE + 1)..=(pass.rect.y + pass.rect.h - 1))).changed() {
                    pass.rect.h += pass.rect.y - t;
                    pass.rect.y = t;
                }
                let mut r = pass.rect.x + pass.rect.w;
                ui.label("R:");
                if ui.add(egui::DragValue::new(&mut r).range((pass.rect.x + 1)..=(2 * IMG_SIZE - 1))).changed() {
                    pass.rect.w = r - pass.rect.x;
                }
                let mut b = pass.rect.y + pass.rect.h;
                ui.label("B:");
                if ui.add(egui::DragValue::new(&mut b).range((pass.rect.y + 1)..=(2 * IMG_SIZE - 1))).changed() {
                    pass.rect.h = b - pass.rect.y;
                }
            });

            ui.horizontal_wrapped(| ui | {
                ui.label("Dim:");
                ui.label("X:");
                ui.add(egui::DragValue::new(&mut pass.rect.x).range((-pass.rect.w + 1)..=(IMG_SIZE - 1)));
                ui.label("Y:");
                ui.add(egui::DragValue::new(&mut pass.rect.y).range((-pass.rect.h + 1)..=(IMG_SIZE - 1)));
                ui.label("W:");
                ui.add(egui::DragValue::new(&mut pass.rect.w).range(1..=(2 * IMG_SIZE - 1)));
                ui.label("H:");
                ui.add(egui::DragValue::new(&mut pass.rect.h).range(1..=(2 * IMG_SIZE - 1)));
            });

            ui.horizontal_wrapped(| ui | {
                ui.label("Ratio:");
                if ui.button("Square").clicked() {
                    pass.rect.h = pass.rect.w;
                }
                if ui.button("Golden").clicked() {
                    let ratio = (1.0 + 5.0_f32.sqrt()) / 2.0;
                    pass.rect.h = ((pass.rect.w as f32 / ratio).round() as i32).min(IMG_SIZE);
                }
                if ui.button("SQRT2").clicked() {
                    let ratio = 2.0_f32.sqrt();
                    pass.rect.h = ((pass.rect.w as f32 / ratio).round() as i32).min(IMG_SIZE);
                }
                if ui.button("16:9").clicked() {
                    let ratio = 16.0 / 9.0;
                    pass.rect.h = ((pass.rect.w as f32 / ratio).round() as i32).min(IMG_SIZE);
                }
                if ui.button("Flip").clicked() {
                    let mv = (pass.rect.w - pass.rect.h) / 2;
                    pass.rect.x += mv;
                    pass.rect.y -= mv;
                    swap(&mut pass.rect.w, &mut pass.rect.h);
                }
                if ui.button("Clip").clicked() {
                    pass.rect = Rect {
                        x: pass.rect.x.max(0),
                        y: pass.rect.y.max(0),
                        w: pass.rect.w.min(IMG_SIZE - pass.rect.x),
                        h: pass.rect.h.min(IMG_SIZE - pass.rect.y),
                    };
                }
                if ui.button("Full").clicked() {
                    pass.rect = Rect { x: 0, y: 0, w: IMG_SIZE, h: IMG_SIZE };
                }
            });

            ui.horizontal_wrapped(| ui | {
                ui.label("Align:");
                if ui.button("Left").clicked() {
                    pass.rect.x = 0;
                }
                if ui.button("HCenter").clicked() {
                    pass.rect.x = (IMG_SIZE - pass.rect.w) / 2;
                }
                if ui.button("Right").clicked() {
                    pass.rect.x = IMG_SIZE - pass.rect.w;
                }
                if ui.button("Top").clicked() {
                    pass.rect.y = 0;
                }
                if ui.button("VCenter").clicked() {
                    pass.rect.y = (IMG_SIZE - pass.rect.h) / 2;
                }
                if ui.button("Bottom").clicked() {
                    pass.rect.y = IMG_SIZE - pass.rect.h;
                }
            });
            
            ui.horizontal_wrapped(| ui | {
                ui.checkbox(&mut pass.round_rect, "Round");
                if pass.round_rect {
                    ui.label("Radius:");
                    ui.add(egui::DragValue::new(&mut pass.round_rect_radius).range(0..=(pass.rect.w.min(pass.rect.h) / 2)));
                    ui.checkbox(&mut pass.round_rect_aa, "Anti-Alias");
                }
            });

            ui.horizontal_wrapped(| ui | {
                ui.add(egui::DragValue::new(&mut pass.bevel_depth).range(-IMG_SIZE..=IMG_SIZE));
                if pass.bevel_depth != 0 {
                    ui.checkbox(&mut pass.bevel_ease_in, "Ease In");
                    ui.checkbox(&mut pass.bevel_ease_out, "Ease Out");
                    ui.checkbox(&mut pass.bevel_shadow, "Shadow Only");
                }
            });

            ui.separator();

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
        def.passes.push(TexturePass::new());
    }
}
