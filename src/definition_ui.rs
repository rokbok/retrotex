use std::{fmt::Write, mem::swap};

use egui::{Button, TextEdit, text::{CCursor, CCursorRange}};
use crate::{IMG_SIZE, color::{Color, EditableColor}, definition::{TextureDefinition, TexturePass}, util::add_enum_dropdown};

#[allow(unused_imports)]
use log::{debug, error, log_enabled, info, warn, trace};

pub enum PassOperation { Remove(usize), MoveUp(usize), MoveDown(usize) }

fn add_full_width<T: egui::Widget>(ui: &mut egui::Ui, widget: T) -> egui::Response {
    let available_width = ui.available_width();
    ui.add_sized([available_width, 0.0], widget)
}

fn reseed_button(ui: &mut egui::Ui, seed: &mut u32) {
    if ui.button("Re-seed").clicked() || *seed == 0 {
        *seed = rand::random::<u32>();
    }
}

pub fn add_color_edit<const ALPHA: bool>(ui: &mut egui::Ui, editable: &mut EditableColor<ALPHA>, monospace_width: f32) {
    let mut color = editable.color();
    let change = if ALPHA {
        ui.color_edit_button_srgba_unmultiplied(&mut color.rgba).changed()
    } else {
        let mut rgb = [color.rgba[0], color.rgba[1], color.rgba[2]];
        if ui.color_edit_button_srgb(&mut rgb).changed() {
            color.rgba[0] = rgb[0];
            color.rgba[1] = rgb[1];
            color.rgba[2] = rgb[2];
            true
        } else {
            false
        }
    };
    if change {
        editable.set_color(color);
    }

    let max_count = if ALPHA { 9 } else { 7 };
    let output = TextEdit::singleline(&mut editable.edit_str)
        .font(egui::TextStyle::Monospace)
        .desired_width(monospace_width * max_count as f32)
        .show(ui);
    if output.response.gained_focus() {
        let mut state = output.state;
        state.cursor.set_char_range(Some(CCursorRange::two(CCursor::new(0), CCursor::new(editable.edit_str.len()))));
        state.store(ui.ctx(), output.response.id);
    }
    if output.response.changed() {
        if editable.edit_str.len() > max_count {
            editable.edit_str.truncate(max_count);
            ui.ctx().request_repaint();
        }
        if let Ok(new_color) = Color::from_hex(&editable.edit_str) {
            editable.set_color_while_editing(new_color);
            ui.ctx().request_repaint();
        }
    }
    if output.response.lost_focus() {
        editable.update_edit_str();
        ui.ctx().request_repaint();
    }
}

pub fn definition_ui(def: &mut TextureDefinition, tmp_str: &mut String, ui: &mut egui::Ui) {
    let monospace_id = egui::TextStyle::Monospace.resolve(ui.style());

    // Estimate width of one character (monospace assumption works best)
    let monospace_width = ui.fonts_mut(|f| {
        f.glyph_width(&monospace_id, 'W') // use a wide character as baseline
    });

    ui.heading(&def.name);
    ui.horizontal(| ui | {
        ui.label("Background:");
        add_color_edit(ui, &mut def.background, monospace_width);
    });
    ui.separator();
    ui.horizontal(| ui | {
        ui.label("Light direction:");
        ui.add(egui::DragValue::new(&mut def.lighting_settings.direction[0]).range(-100..=100));
        ui.add(egui::DragValue::new(&mut def.lighting_settings.direction[1]).range(-100..=100));
        ui.add(egui::DragValue::new(&mut def.lighting_settings.direction[2]).range(1..=100));
        ui.label("Impact:");
        ui.add(egui::DragValue::new(&mut def.lighting_settings.impact).range(0..=100)).on_hover_text("0 = unlit; 100 = maximum contrast");
    });
    ui.horizontal( | ui | {
        ui.label("Ambient occlusion:");
        ui.add(egui::DragValue::new(&mut def.ao_settings.strength).range(0..=100)).on_hover_text("Ambient occlusion strength");
        ui.label("Radius:");
        ui.add(egui::DragValue::new(&mut def.ao_settings.radius).range(1..=(IMG_SIZE - 1))).on_hover_text("Higher = more distant occluders will contribute to AO");
        ui.label("Bias:");
        ui.add(egui::DragValue::new(&mut def.ao_settings.bias).range(0..=200)).on_hover_text("Bias ambient occlusion based on light direction");
        ui.checkbox(&mut def.ao_settings.ignore_surface_normal, "Ignore Surface Normal").on_hover_text("Experimental; Probably not something you want to use");
    });

    egui::ScrollArea::vertical().show(ui, | ui | {
        let mut pass_op = Option::<PassOperation>::None;
        let pass_count = def.passes.len();
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
                    add_color_edit(ui, &mut pass.color, monospace_width);
                    if pass.uses_both_colors() {
                            add_color_edit(ui, &mut pass.color2, monospace_width);
                    }
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
                        reseed_button(ui, &mut pass.perlin.seed);
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
                        reseed_button(ui, &mut pass.white_noise.seed);
                    }
                });

                ui.separator();
                ui.horizontal_wrapped(| ui | {
                    ui.checkbox(&mut pass.rect.enabled, "Rect");
                    if pass.rect.enabled {
                                                let wh = pass.rect.width / 2;
                        let hh = pass.rect.height / 2;
                        ui.label("Center X:");
                        let mut cx = pass.feature_x + wh;
                        if ui.add(egui::DragValue::new(&mut cx).range((-pass.rect.width + 1 + wh)..=(IMG_SIZE - 1 + wh))).changed() {
                            pass.feature_x = cx - wh;
                        }
                        ui.label("Center Y:");
                        let mut cy = pass.feature_y + hh;
                        if ui.add(egui::DragValue::new(&mut cy).range((-pass.rect.height + 1 + hh)..=(IMG_SIZE - 1 + hh))).changed() {
                            pass.feature_y = cy - hh;
                        }
                        ui.label("Width:");
                        let old_width = pass.rect.width;
                        if ui.add(egui::DragValue::new(&mut pass.rect.width).range(1..=(2 * IMG_SIZE - 1))).changed() {
                            let delta = pass.rect.width - old_width;
    
                            if delta % 2 == 0 {
                                pass.feature_x -= delta / 2;
                            } else {
                                let old_center = 2 * pass.feature_x + old_width;
                                let bias = match old_center.rem_euclid(4) {
                                    0 | 1 => -1,
                                    2 | 3 => 1,
                                    _ => unreachable!(),
                                };
                                pass.feature_x += (-delta + bias) / 2;
                            }
                        }
                        ui.label("Height:");
                        let old_height = pass.rect.height;
                        if ui.add(egui::DragValue::new(&mut pass.rect.height).range(1..=(2 * IMG_SIZE - 1))).changed() {
                            let delta = pass.rect.height - old_height;
                            if delta % 2 == 0 {
                                pass.feature_y -= delta / 2;
                            } else {
                                let old_center = 2 * pass.feature_y + old_height;
                                let bias = match old_center.rem_euclid(4) {
                                    0 | 1 => -1,
                                    2 | 3 => 1,
                                    _ => unreachable!(),
                                };
                                pass.feature_y += (-delta + bias) / 2;
                            }
                        }
                    }
                });

                if pass.rect.enabled {
                    ui.horizontal_wrapped(| ui | {
                        let mut l = pass.feature_x;
                        ui.label("Left:");
                        if ui.add(egui::DragValue::new(&mut l).range((-IMG_SIZE + 1)..=(pass.feature_x + pass.rect.width - 1))).changed() {
                            pass.rect.width += pass.feature_x - l;
                            pass.feature_x = l;
                        }
                        let mut t = pass.feature_y;
                        ui.label("Top:");
                        if ui.add(egui::DragValue::new(&mut t).range((-IMG_SIZE + 1)..=(pass.feature_y + pass.rect.height - 1))).changed() {
                            pass.rect.height += pass.feature_y - t;
                            pass.feature_y = t;
                        }
                        let mut r = pass.feature_x + pass.rect.width;
                        ui.label("Right:");
                        if ui.add(egui::DragValue::new(&mut r).range((pass.feature_x + 1)..=(2 * IMG_SIZE - 1))).changed() {
                            pass.rect.width = r - pass.feature_x;
                        }
                        let mut b = pass.feature_y + pass.rect.height;
                        ui.label("Bottom:");
                        if ui.add(egui::DragValue::new(&mut b).range((pass.feature_y + 1)..=(2 * IMG_SIZE - 1))).changed() {
                            pass.rect.height = b - pass.feature_y;
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
                            pass.feature_x += mv;
                            pass.feature_y -= mv;
                            swap(&mut pass.rect.width, &mut pass.rect.height);
                        }
                        if ui.button("Clip").clicked() {
                            pass.feature_x = pass.feature_x.max(0);
                            pass.feature_y = pass.feature_y.max(0);
                            pass.rect.width = pass.rect.width.min(IMG_SIZE - pass.feature_x);
                            pass.rect.height = pass.rect.height.min(IMG_SIZE - pass.feature_y);
                        }
                        if ui.button("Full").clicked() {
                            pass.feature_x = 0;
                            pass.feature_y = 0;
                            pass.rect.width = IMG_SIZE;
                            pass.rect.height = IMG_SIZE;
                        }
                    });

                    ui.horizontal_wrapped(| ui | {
                        ui.label("Align:");
                        if ui.button("Left").on_hover_text("Align to the left").clicked() {
                            pass.feature_x = 0;
                        }
                        if ui.button("HCenter").on_hover_text("Center horizontally").clicked() {
                            pass.feature_x = (IMG_SIZE - pass.rect.width) / 2;
                        }
                        if ui.button("Right").on_hover_text("Align to the right").clicked() {
                            pass.feature_x = IMG_SIZE - pass.rect.width;
                        }
                        if ui.button("Top").on_hover_text("Align to the top").clicked() {
                            pass.feature_y = 0;
                        }
                        if ui.button("VCenter").on_hover_text("Center vertically").clicked() {
                            pass.feature_y = (IMG_SIZE - pass.rect.height) / 2;
                        }
                        if ui.button("Bottom").on_hover_text("Align to the bottom").clicked() {
                            pass.feature_y = IMG_SIZE - pass.rect.height;
                        }
                    });
                    
                    ui.horizontal_wrapped(| ui | {
                        ui.checkbox(&mut pass.rect.round.enabled, "Round").on_hover_text("Round rect corners");
                        if pass.rect.round.enabled {
                            ui.label("Radius:");
                            ui.add(egui::DragValue::new(&mut pass.rect.round.radius).range(1..=(pass.rect.width.min(pass.rect.height))));
                            ui.checkbox(&mut pass.rect.round.anti_alias, "Anti-Alias").on_hover_text("Enable anti-aliasing for rounded corners (albedo only)");
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

                    ui.horizontal_wrapped(| ui | {
                        ui.checkbox(&mut pass.rect.tile.enabled, "Tile");
                        if pass.rect.tile.enabled {
                            ui.label("Offset:");
                            ui.add(egui::DragValue::new(&mut pass.rect.tile.x_offset).range(2..=IMG_SIZE));
                            ui.add(egui::DragValue::new(&mut pass.rect.tile.y_offset).range(2..=IMG_SIZE));
                            ui.label("Count:");
                            ui.add(egui::DragValue::new(&mut pass.rect.tile.x_count).range(1..=IMG_SIZE/2));
                            ui.add(egui::DragValue::new(&mut pass.rect.tile.y_count).range(1..=IMG_SIZE/2));
                            ui.label("Shift:");
                            ui.add(egui::DragValue::new(&mut pass.rect.tile.shift).range(-IMG_SIZE/2..=IMG_SIZE/2));
                            if pass.rect.tile.shift > 0 {
                                ui.label("Direction:");
                                add_enum_dropdown(ui, &mut pass.rect.tile.shift_direction, "tile_shift_direction", pass_idx, false);
                            }

                            ui.label("Variation:");
                            ui.add(egui::DragValue::new(&mut pass.rect.tile.variation).range(0..=400))
                                .on_hover_text("Per-tile variation, between the two colors selected");
                            if pass.rect.tile.variation > 0 {
                                reseed_button(ui, &mut pass.rect.tile.variation_seed);
                            }
                        }
                    });
                }

                ui.separator();

                ui.horizontal(| ui | {
                    if pass_idx > 0 {
                        let resp = ui.button("Up");
                        if resp.clicked() {
                            pass_op = Some(PassOperation::MoveUp(pass_idx));
                        }
                    }
                    if pass_idx < pass_count - 1 {
                        let resp = ui.button("Down");
                        if resp.clicked() {
                            pass_op = Some(PassOperation::MoveDown(pass_idx));
                        }
                    }
                    if ui.add_sized([ui.available_width(), ui.spacing().interact_size.y], Button::new("Remove")).clicked() {
                        pass_op = Some(PassOperation::Remove(pass_idx));
                    }
                });
            });
        }

        if let Some(op) = pass_op {
            match op {
                PassOperation::Remove(idx) => {
                    def.passes.remove(idx);
                }
                PassOperation::MoveUp(idx) => {
                    if idx > 0 {
                        def.passes.swap(idx, idx - 1);
                    }
                }
                PassOperation::MoveDown(idx) => {
                    if idx < def.passes.len() - 1 {
                        def.passes.swap(idx, idx + 1);
                    }
                }
            }
        }

        if add_full_width(ui, Button::new("Add Pass")).clicked() {
            def.passes.push(TexturePass::new());
        }
    });
}
