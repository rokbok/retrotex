use std::{fmt::Write, mem::swap};

use egui::Button;
use crate::{IMG_SIZE, color::Color, definition::{TextureDefinition, TexturePass}, util::add_enum_dropdown};

#[allow(unused_imports)]
use log::{debug, error, log_enabled, info, warn, trace};

pub enum PassOperation { Remove(usize) }

fn add_full_width<T: egui::Widget>(ui: &mut egui::Ui, widget: T) -> egui::Response {
    let available_width = ui.available_width();
    ui.add_sized([available_width, 0.0], widget)
}

pub fn color_with_copy_paste(ui: &mut egui::Ui, color: &mut Color, clipboard: &mut arboard::Clipboard, tmp_str: &mut String) {
    ui.color_edit_button_srgba_unmultiplied(&mut color.rgba);
    tmp_str.clear();
    color.write_hex(tmp_str).expect("Color string conversion failed");
    ui.label(&*tmp_str);
    if ui.button("Copy").clicked() {
        ui.ctx().copy_text(tmp_str.clone());
    }
    if ui.button("Paste").clicked() {
        if let Ok(clipboard_str) = clipboard.get_text() {
            if let Ok(new_color) = Color::from_hex(&clipboard_str) {
                *color = new_color;
            }
        }
    }
}

pub fn definition_ui(def: &mut TextureDefinition, tmp_str: &mut String, ui: &mut egui::Ui, clipboard: &mut arboard::Clipboard) {
    ui.heading(&def.name);
    ui.horizontal(| ui | {
        ui.label("Background:");
        color_with_copy_paste(ui, &mut def.background, clipboard, tmp_str);
    });
    ui.horizontal(| ui | {
        ui.label("Light:");
        ui.add(egui::DragValue::new(&mut def.lighting_settings.light_dir[0]).range(-100..=100));
        ui.add(egui::DragValue::new(&mut def.lighting_settings.light_dir[1]).range(-100..=100));
        ui.add(egui::DragValue::new(&mut def.lighting_settings.light_dir[2]).range(1..=100));
        ui.label("Ambient:");
        ui.add(egui::DragValue::new(&mut def.lighting_settings.ambient).range(0..=100));
    });
    ui.horizontal( | ui | {
        ui.label("AO:");
        ui.add(egui::DragValue::new(&mut def.ao_settings.strength).range(0..=100));
        ui.label("Radius:");
        ui.add(egui::DragValue::new(&mut def.ao_settings.radius).range(1..=(IMG_SIZE - 1)));
    });

    egui::ScrollArea::vertical().show(ui, | ui | {
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

                ui.checkbox(&mut pass.enabled, "Enabled");

                ui.horizontal_wrapped(| ui | {
                    color_with_copy_paste(ui, &mut pass.color, clipboard, tmp_str);
                });

                ui.horizontal( | ui | {
                    ui.label("Blend:");
                    add_enum_dropdown(ui, &mut pass.blend_mode, "blend_mode", pass_idx, false);
                });


                ui.horizontal_wrapped(| ui | { 
                    ui.checkbox(&mut pass.perlin.enabled, "Perlin");
                    if pass.perlin.enabled {
                        ui.label("Scale:");
                        ui.add(egui::DragValue::new(&mut pass.perlin.scale).range(1..=IMG_SIZE));
                        ui.label("Octaves:");
                        ui.add(egui::DragValue::new(&mut pass.perlin.octaves).range(1..=4));
                        ui.checkbox(&mut pass.perlin.use_threshold, "Threshold");
                        if pass.perlin.use_threshold {
                            ui.add(egui::DragValue::new(&mut pass.perlin.threshold).range(0..=100));
                        }
                        if ui.button("Re-seed").clicked() {
                            pass.perlin.seed = rand::random();
                        }
                    }
                });
                
                ui.horizontal_wrapped(| ui | {
                    ui.checkbox(&mut pass.white_noise.enabled, "White Noise");
                    if pass.white_noise.enabled {
                        ui.label("Scale:");
                        ui.add(egui::DragValue::new(&mut pass.white_noise.scale).range(1..=(IMG_SIZE/2)));
                        ui.checkbox(&mut pass.white_noise.use_threshold, "Threshold");
                        if pass.white_noise.use_threshold {
                            ui.add(egui::DragValue::new(&mut pass.white_noise.threshold).range(0..=100));
                        }
                        if ui.button("Re-seed").clicked() {
                            pass.white_noise.seed = rand::random();
                        }
                    }
                });

                ui.separator();
                ui.horizontal_wrapped(| ui | {
                    ui.checkbox(&mut pass.rect.enabled, "Rect");
                    if pass.rect.enabled {
                        let mut l = pass.rect.x;
                        ui.label("L:");
                        if ui.add(egui::DragValue::new(&mut l).range((-IMG_SIZE + 1)..=(pass.rect.x + pass.rect.width - 1))).changed() {
                            pass.rect.width += pass.rect.x - l;
                            pass.rect.x = l;
                        }
                        let mut t = pass.rect.y;
                        ui.label("T:");
                        if ui.add(egui::DragValue::new(&mut t).range((-IMG_SIZE + 1)..=(pass.rect.y + pass.rect.height - 1))).changed() {
                            pass.rect.height += pass.rect.y - t;
                            pass.rect.y = t;
                        }
                        let mut r = pass.rect.x + pass.rect.width;
                        ui.label("R:");
                        if ui.add(egui::DragValue::new(&mut r).range((pass.rect.x + 1)..=(2 * IMG_SIZE - 1))).changed() {
                            pass.rect.width = r - pass.rect.x;
                        }
                        let mut b = pass.rect.y + pass.rect.height;
                        ui.label("B:");
                        if ui.add(egui::DragValue::new(&mut b).range((pass.rect.y + 1)..=(2 * IMG_SIZE - 1))).changed() {
                            pass.rect.height = b - pass.rect.y;
                        }
                    }
                });

                if pass.rect.enabled {
                    ui.horizontal_wrapped(| ui | {
                        let wh = pass.rect.width / 2;
                        let hh = pass.rect.height / 2;
                        ui.label("CX:");
                        let mut cx = pass.rect.x + wh;
                        if ui.add(egui::DragValue::new(&mut cx).range((-pass.rect.width + 1 + wh)..=(IMG_SIZE - 1 + wh))).changed() {
                            pass.rect.x = cx - wh;
                        }
                        ui.label("CY:");
                        let mut cy = pass.rect.y + hh;
                        if ui.add(egui::DragValue::new(&mut cy).range((-pass.rect.height + 1 + hh)..=(IMG_SIZE - 1 + hh))).changed() {
                            pass.rect.y = cy - hh;
                        }
                        ui.label("W:");
                        let old_width = pass.rect.width;
                        if ui.add(egui::DragValue::new(&mut pass.rect.width).range(1..=(2 * IMG_SIZE - 1))).changed() {
                            let delta = pass.rect.width - old_width;
    
                            if delta % 2 == 0 {
                                pass.rect.x -= delta / 2;
                            } else {
                                let old_center = 2 * pass.rect.x + old_width;
                                let bias = match old_center.rem_euclid(4) {
                                    0 | 1 => -1,
                                    2 | 3 => 1,
                                    _ => unreachable!(),
                                };
                                pass.rect.x += (-delta + bias) / 2;
                            }
                        }
                        ui.label("H:");
                        let old_height = pass.rect.height;
                        if ui.add(egui::DragValue::new(&mut pass.rect.height).range(1..=(2 * IMG_SIZE - 1))).changed() {
                            let delta = pass.rect.height - old_height;
                            if delta % 2 == 0 {
                                pass.rect.y -= delta / 2;
                            } else {
                                let old_center = 2 * pass.rect.y + old_height;
                                let bias = match old_center.rem_euclid(4) {
                                    0 | 1 => -1,
                                    2 | 3 => 1,
                                    _ => unreachable!(),
                                };
                                pass.rect.y += (-delta + bias) / 2;
                            }
                        }
                    });

                    ui.horizontal_wrapped(| ui | {
                        ui.label("Ratio:");
                        if ui.button("Square").clicked() {
                            pass.rect.height = pass.rect.width;
                        }
                        if ui.button("Golden").clicked() {
                            let ratio = (1.0 + 5.0_f32.sqrt()) / 2.0;
                            pass.rect.height = ((pass.rect.width as f32 / ratio).round() as i32).min(IMG_SIZE);
                        }
                        if ui.button("SQRT2").clicked() {
                            let ratio = 2.0_f32.sqrt();
                            pass.rect.height = ((pass.rect.width as f32 / ratio).round() as i32).min(IMG_SIZE);
                        }
                        if ui.button("16:9").clicked() {
                            let ratio = 16.0 / 9.0;
                            pass.rect.height = ((pass.rect.width as f32 / ratio).round() as i32).min(IMG_SIZE);
                        }
                        if ui.button("Flip").clicked() {
                            let mv = (pass.rect.width - pass.rect.height) / 2;
                            pass.rect.x += mv;
                            pass.rect.y -= mv;
                            swap(&mut pass.rect.width, &mut pass.rect.height);
                        }
                        if ui.button("Clip").clicked() {
                            pass.rect.x = pass.rect.x.max(0);
                            pass.rect.y = pass.rect.y.max(0);
                            pass.rect.width = pass.rect.width.min(IMG_SIZE - pass.rect.x);
                            pass.rect.height = pass.rect.height.min(IMG_SIZE - pass.rect.y);
                        }
                        if ui.button("Full").clicked() {
                            pass.rect.x = 0;
                            pass.rect.y = 0;
                            pass.rect.width = IMG_SIZE;
                            pass.rect.height = IMG_SIZE;
                        }
                    });

                    ui.horizontal_wrapped(| ui | {
                        ui.label("Align:");
                        if ui.button("Left").clicked() {
                            pass.rect.x = 0;
                        }
                        if ui.button("HCenter").clicked() {
                            pass.rect.x = (IMG_SIZE - pass.rect.width) / 2;
                        }
                        if ui.button("Right").clicked() {
                            pass.rect.x = IMG_SIZE - pass.rect.width;
                        }
                        if ui.button("Top").clicked() {
                            pass.rect.y = 0;
                        }
                        if ui.button("VCenter").clicked() {
                            pass.rect.y = (IMG_SIZE - pass.rect.height) / 2;
                        }
                        if ui.button("Bottom").clicked() {
                            pass.rect.y = IMG_SIZE - pass.rect.height;
                        }
                    });
                    
                    ui.horizontal_wrapped(| ui | {
                        ui.checkbox(&mut pass.rect.round.enabled, "Round");
                        if pass.rect.round.enabled {
                            ui.label("Radius:");
                            ui.add(egui::DragValue::new(&mut pass.rect.round.radius).range(0..=(pass.rect.width.min(pass.rect.height) / 2)));
                            ui.checkbox(&mut pass.rect.round.anti_alias, "Anti-Alias");
                        }
                    });

                    ui.horizontal_wrapped(| ui | {
                        ui.checkbox(&mut pass.rect.bevel.enabled, "Bevel");
                        if pass.rect.bevel.enabled {
                            ui.add(egui::DragValue::new(&mut pass.rect.bevel.size).range(1..=IMG_SIZE));
                            ui.label("Steepness:");
                            ui.add(egui::DragValue::new(&mut pass.rect.bevel.steepness).range(-10..=10));
                            ui.checkbox(&mut pass.rect.bevel.convex, "Convex");
                            ui.label("Ease:");
                            ui.checkbox(&mut pass.rect.bevel.ease_in, "In");
                            ui.checkbox(&mut pass.rect.bevel.ease_out, "Out");
                        }
                    });
                }

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
    });
}
