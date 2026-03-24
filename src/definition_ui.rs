use std::{fmt::Write as _};

use egui::{Button, TextEdit, Vec2, text::{CCursor, CCursorRange}};
use strum::{EnumCount, IntoEnumIterator};
use crate::{IMG_SIZE, RetroTexApp, color::{Color, EditableColor}, definition::{NoiseType, TexturePass}, util::add_enum_dropdown};

#[allow(unused_imports)]
use log::{debug, error, log_enabled, info, warn, trace};

const SECTION_SPACING: f32 = 10.0;

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

fn mode_selector<T: IntoEnumIterator + AsRef<str> + Eq + EnumCount + Copy>(ui: &mut egui::Ui, val: &mut T, label: &str) -> Option<T> {
    let mut ret = None;
    ui.horizontal(| ui | {
        ui.label(label);
        let aw = ui.available_width();
        let btn_w = (aw - ui.spacing().item_spacing.x * (T::COUNT as f32 - 1.0)) /  T::COUNT as f32;
        for cur in T::iter() {
            let btn = egui::Button::selectable(*val == cur, cur.as_ref());
            if ui.add_sized([btn_w, ui.spacing().interact_size.y], btn).clicked() {
                *val = cur;
                ret = Some(cur);        }
        }
    });
    ui.add_space(SECTION_SPACING);
    ret
}


impl RetroTexApp {
    pub fn definition_ui(&mut self, ui: &mut egui::Ui) {
        let monospace_id = egui::TextStyle::Monospace.resolve(ui.style());

        // Estimate width of one character (monospace assumption works best)
        let monospace_width = ui.fonts_mut(|f| {
            f.glyph_width(&monospace_id, 'W') // use a wide character as baseline
        });

        ui.heading(&self.def.name);
        ui.horizontal(| ui | {
            ui.label("Background:");
            add_color_edit(ui, &mut self.def.background, monospace_width);
        });
        ui.separator();
        ui.horizontal(| ui | {
            ui.label("Light direction:");
            ui.add(egui::DragValue::new(&mut self.def.lighting_settings.direction[0]).range(-100..=100));
            ui.add(egui::DragValue::new(&mut self.def.lighting_settings.direction[1]).range(-100..=100));
            ui.add(egui::DragValue::new(&mut self.def.lighting_settings.direction[2]).range(1..=100));
            ui.label("Impact:");
            ui.add(egui::DragValue::new(&mut self.def.lighting_settings.impact).range(0..=100)).on_hover_text("0 = unlit; 100 = maximum contrast");
        });
        ui.horizontal_wrapped( | ui | {
            ui.label("Ambient occlusion:");
            ui.add(egui::DragValue::new(&mut self.def.ao_settings.strength).range(0..=100)).on_hover_text("Ambient occlusion strength");
            ui.add(egui::DragValue::new(&mut self.def.ao_settings.radius).range(1..=(IMG_SIZE - 1)).prefix("Radius:")).on_hover_text("Higher = more distant occluders will contribute to AO");
            ui.add(egui::DragValue::new(&mut self.def.ao_settings.bias).range(0..=200).prefix("Bias:")).on_hover_text("Bias ambient occlusion based on light direction");
        //    ui.checkbox(&mut self.def.ao_settings.ignore_surface_normal, "Ignore Surface Normal").on_hover_text("Experimental; Probably not something you want to use");
        });

        egui::ScrollArea::vertical().show(ui, | ui | {
            let mut pass_op = Option::<PassOperation>::None;
            let pass_count = self.def.passes.len();
            for (pass_idx, pass) in self.def.passes.iter_mut().enumerate() {
                ui.group(| ui | {
                    self.tmp_str.clear();
                    match &pass.name {
                        Some(name) => self.tmp_str.push_str(name),
                        None => {
                            self.tmp_str.clear();
                            write!(self.tmp_str, "Pass {}", pass_idx).unwrap();
                        },
                    }

                    let name_response = add_full_width(ui, egui::TextEdit::singleline(&mut self.tmp_str).hint_text("Pass Name"));
                    if name_response.changed() {
                        match &mut pass.name {
                            Some(name) => {
                                name.clear();
                                name.push_str(&self.tmp_str);
                            },
                            None => pass.name = Some(self.tmp_str.clone()),
                        }
                    }

                    if self.tmp_str.is_empty() && !name_response.has_focus() {
                        pass.name = None;
                    }
                    
                    ui.checkbox(&mut pass.enabled, "Enabled");

                    ui.horizontal_wrapped(| ui | {
                        add_color_edit(ui, &mut pass.color, monospace_width);
                        ui.label("Blend:");
                        add_enum_dropdown(ui, &mut pass.blend_mode, "blend_mode", pass_idx, false);
                    });

                    ui.separator();
                    mode_selector(ui, &mut pass.noise.noise_type, "Noise");

                    if pass.uses_noise() {
                        ui.horizontal_wrapped(| ui | {
                            ui.label("Noise Mode:");
                            add_enum_dropdown(ui, &mut pass.noise.mode, "noise_mode", pass_idx, false);
                            match pass.noise.noise_type {
                                NoiseType::Perlin => {
                                    ui.label("Strength:");
                                    ui.add(egui::DragValue::new(&mut pass.noise.perlin_strength.v).range(0..=400));
                                    ui.label("Scale:");
                                    ui.add(egui::DragValue::new(&mut pass.noise.perlin_scale.v).range(1..=IMG_SIZE));
                                    ui.label("Octaves:");
                                    ui.add(egui::DragValue::new(&mut pass.noise.perlin_octaves).range(1..=4));
                                },
                                NoiseType::White | NoiseType::Gaussian => {
                                    ui.label("STD:");
                                    ui.add(egui::DragValue::new(&mut pass.noise.std.v).range(0..=100));
                                    ui.label("Scale:");
                                    ui.add(egui::DragValue::new(&mut pass.noise.pixel_scale).range(1..=IMG_SIZE/2));
                                },
                                NoiseType::None => {},
                            }
                            ui.checkbox(&mut pass.noise.use_threshold, "Threshold");
                            if pass.noise.use_threshold {
                                ui.add(egui::DragValue::new(&mut pass.noise.threshold.v).range(0..=100));
                            }
                            reseed_button(ui, &mut pass.noise.seed);
                        });
                    }   

                    ui.separator();
                    let new_coverage = mode_selector(ui, &mut pass.coverage, "Shape:");
                    if let Some(cov) = new_coverage {
                        if cov.is_gizmo_editable() {
                            self.preview_editing = Some(pass_idx);
                        } else if self.preview_editing == Some(pass_idx) {
                            self.preview_editing = None;
                        }
                    }

                    if pass.is_rect() {
                        ui.horizontal(| ui | {
                            ui.label("Rect:");
                            ui.add(egui::DragValue::new(&mut pass.feature_x).range(-IMG_SIZE..=(IMG_SIZE - 1)).prefix("X:"));
                            ui.add(egui::DragValue::new(&mut pass.feature_y).range(-IMG_SIZE..=(IMG_SIZE - 1)).prefix("Y:"));
                            ui.add(egui::DragValue::new(&mut pass.rect.width).range(1..=(2 * IMG_SIZE - 1)).prefix("W:"));
                            ui.add(egui::DragValue::new(&mut pass.rect.height).range(1..=(2 * IMG_SIZE - 1)).prefix("H:"));
                            let is_editing = self.preview_editing == Some(pass_idx);
                            let edit_button = egui::Button::selectable(is_editing, if is_editing { "Editing rect..." } else { "Edit rect" });
                            if ui.add_sized([ui.available_width(), ui.spacing().interact_size.y], edit_button).clicked() {
                                if is_editing {
                                    self.preview_editing = None;
                                } else {
                                    self.preview_editing = Some(pass_idx);
                                }
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

                        let mut bevel_enabled_width = 0.0;
                        ui.horizontal(| ui | {
                            let resp = ui.checkbox(&mut pass.rect.bevel.enabled, "Bevel");
                            bevel_enabled_width = resp.rect.width();
                            if pass.rect.bevel.enabled {
                                ui.add(egui::DragValue::new(&mut pass.rect.bevel.size).range(1..=IMG_SIZE));
                                ui.label("Steepness:");
                                ui.add(egui::DragValue::new(&mut pass.rect.bevel.steepness).range(-10..=10));
                                ui.checkbox(&mut pass.rect.bevel.convex, "Convex");
                            }
                        });
                        if pass.rect.bevel.enabled {
                            ui.horizontal(| ui | {
                                ui.add_space(bevel_enabled_width + ui.spacing().item_spacing.x);
                                ui.label("Ease:");
                                ui.checkbox(&mut pass.rect.bevel.ease_in, "In");
                                ui.checkbox(&mut pass.rect.bevel.ease_out, "Out");
                            });
                            ui.add_space(SECTION_SPACING);
                        }

                        let mut tile_enabled_width = 0.0;
                        ui.horizontal(| ui | {
                            let resp = ui.checkbox(&mut pass.tile.enabled, "Tile");
                            tile_enabled_width = resp.rect.width();
                            if pass.tile.enabled {
                                ui.label("Gap:");
                                ui.add(egui::DragValue::new(&mut pass.tile.x_gap).range(0..=IMG_SIZE-2).prefix("X:"));
                                ui.add(egui::DragValue::new(&mut pass.tile.y_gap).range(0..=IMG_SIZE-2).prefix("Y:"));
                                ui.label("Count:");
                                ui.add(egui::DragValue::new(&mut pass.tile.x_count).range(1..=IMG_SIZE/2).prefix("X:"));
                                ui.add(egui::DragValue::new(&mut pass.tile.y_count).range(1..=IMG_SIZE/2).prefix("Y:"));
                            }
                        });
                        if pass.tile.enabled {
                            ui.horizontal(| ui | {
                                ui.add_space(tile_enabled_width + ui.spacing().item_spacing.x);
                                ui.label("Shift:");
                                ui.add(egui::DragValue::new(&mut pass.tile.shift).range(-IMG_SIZE/2..=IMG_SIZE/2).prefix(if pass.tile.shift_direction == crate::definition::TileShiftDirection::Horizontal { "X:" } else { "Y:" }));
                                if pass.tile.shift > 0 {
                                    ui.label("Direction:");
                                    add_enum_dropdown(ui, &mut pass.tile.shift_direction, "tile_shift_direction", pass_idx, false);
                                }
                            });

                            ui.horizontal(| ui | {
                                ui.add_space(tile_enabled_width + ui.spacing().item_spacing.x);
                                ui.checkbox(&mut pass.tile.variation_enabled, "Variation");
                                if pass.tile.variation_enabled {
                                    ui.label("Strength:");
                                    ui.add(egui::DragValue::new(&mut pass.tile.variation.v).range(1..=400));
                                    reseed_button(ui, &mut pass.tile.variation_seed);
                                }
                            });
                            ui.add_space(SECTION_SPACING);
                        };
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
                        self.def.passes.remove(idx);
                    }
                    PassOperation::MoveUp(idx) => {
                        if idx > 0 {
                            self.def.passes.swap(idx, idx - 1);
                        }
                    }
                    PassOperation::MoveDown(idx) => {
                        if idx < self.def.passes.len() - 1 {
                            self.def.passes.swap(idx, idx + 1);
                        }
                    }
                }
            }

            if add_full_width(ui, Button::new("Add Pass")).clicked() {
                self.def.passes.push(TexturePass::new());
            }
        });
    }
}
