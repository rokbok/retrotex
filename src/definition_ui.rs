
use std::fmt::Write as _;

use egui::{Button, Checkbox, Image, TextEdit, include_image, text::{CCursor, CCursorRange}};
use glam::FloatExt;
use strum::{EnumCount, IntoEnumIterator};

use crate::prelude::*;
use crate::{FileNameDialogMode, UiData, color::{Color, EditableColor}, definition::{NoiseType, TextureDefinition, TexturePass}, util::add_enum_dropdown};

const SECTION_SPACING: f32 = 10.0;

const DRAG_SCROLL_PART: f32 = 0.15;
const DRAG_SCROLL_SPEED_MAX: f32 = 256.0;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum PassDrag { StartedOrDragging(usize), Stopped(usize) }

fn add_full_width<T: egui::Widget>(ui: &mut egui::Ui, widget: T) -> egui::Response {
    let available_width = ui.available_width();
    ui.add_sized([available_width, ui.spacing().interact_size.y], widget)
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
        ui.label(egui::RichText::new(label).strong());
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

fn find_closest_y_center(y: f32, rects: &[egui::Response]) -> usize {
    if rects.len() <= 1 || y <= rects[0].rect.center().y {
        return 0;
    }
    if y >= rects[rects.len() - 1].rect.center().y {
        return rects.len() - 1;
    }

    let mut first = 0_usize;
    let mut last = rects.len();
    while first + 1 < last {
        let mid = (first + last) / 2;
        if y < rects[mid].rect.center().y {
            last = mid;
        } else {
            first = mid;
        }
    }
    assert!(first + 1 == last);
    assert!(y >= rects[first].rect.center().y && y <= rects[last].rect.center().y);
    if (y - rects[first].rect.center().y) < (rects[last].rect.center().y - y) {
        first
    } else {
        last
    }
}

fn show_file_name_dialog(ctx: &egui::Context, ui_data: &mut UiData, tmp_str: &mut String) {
    let Some(dialog_mode) = &ui_data.file_name_dialog else {
        return;
    };

    let just_opened = ui_data.file_name_dialog_just_opened;
    if just_opened {
        ui_data.file_name_input.clear();
        match &dialog_mode {
            FileNameDialogMode::Rename(source_name) => ui_data.file_name_input.push_str(source_name),
            FileNameDialogMode::Create => {}
        }
        ui_data.file_name_dialog_just_opened = false;
    }

    let mut open = true;
    let mut close_after_action = false;
    let mut do_submit = false;

    let (window_title, prompt_text, submit_button_text, empty_error) = match &dialog_mode {
        FileNameDialogMode::Rename(source_name) => {
            tmp_str.clear();
            let _ = write!(tmp_str, "Rename file {} to", source_name);
            ("Rename File", tmp_str.as_str(), "Save", "Cannot rename file: name cannot be empty")
        }
        FileNameDialogMode::Create => (
            "Create File",
            "Enter a new file name",
            "Create",
            "Cannot create file: name cannot be empty",
        ),
    };

    egui::Window::new(window_title)
        .id(egui::Id::new("file_name_modal"))
        .collapsible(false)
        .resizable(false)
        .fixed_size(egui::vec2(360.0, 120.0))
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .open(&mut open)
        .show(ctx, |ui| {
            ui.label(prompt_text);

            let te_output = TextEdit::singleline(&mut ui_data.file_name_input).show(ui);
            if just_opened {
                te_output.response.request_focus();
            }
            if te_output.response.gained_focus() {
                let mut state = te_output.state;
                let len = ui_data.file_name_input.chars().count();
                state.cursor.set_char_range(Some(CCursorRange::two(CCursor::new(0), CCursor::new(len))));
                state.store(ctx, te_output.response.id);
            }
            if te_output.response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                do_submit = true;
            }
            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                close_after_action = true;
            }

            ui.separator();

            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(submit_button_text).clicked() || do_submit {
                        let new_name = ui_data.file_name_input.trim();
                        if new_name.is_empty() {
                            error!("{}", empty_error);
                        } else {
                            match dialog_mode {
                                FileNameDialogMode::Rename(_) => ui_data.rename_pending = Some(new_name.to_string()),
                                FileNameDialogMode::Create => ui_data.create_pending = Some(new_name.to_string()),
                            }
                            close_after_action = true;
                        }
                    }
                    if ui.button("Cancel").clicked() {
                        close_after_action = true;
                    }
                });
            });
        });

    if !open || close_after_action {
        ui_data.file_name_dialog = None;
    }
}

fn show_tex_ref_dialog(
    ctx: &egui::Context,
    def: &mut TextureDefinition,
    ui_data: &mut UiData,
    current_file_id: u128,
    available_files: &[(u128, String)],
) {
    let Some(pass_idx) = ui_data.tex_ref_dialog_pass else {
        return;
    };

    if pass_idx >= def.passes.len() {
        ui_data.tex_ref_dialog_pass = None;
        return;
    }

    let pass = &mut def.passes[pass_idx];
    let mut open = true;
    let mut close_after_action = false;

    egui::Window::new("Select Referenced Texture")
        .id(egui::Id::new("tex_ref_modal"))
        .collapsible(false)
        .resizable(false)
        .fixed_size(egui::vec2(360.0, 420.0))
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .open(&mut open)
        .show(ctx, |ui| {
            let mut has_choice = false;
            egui::ScrollArea::vertical()
                .max_height(280.0)
                .show(ui, |ui| {
                    for (id, name) in available_files {
                        if *id == current_file_id {
                            continue;
                        }
                        has_choice = true;
                        let selected = pass.tex_ref == Some(*id);
                        if ui.selectable_label(selected, name).clicked() {
                            pass.tex_ref = Some(*id);
                            close_after_action = true;
                        }
                    }
                });

            if !has_choice {
                ui.label("No other textures are available.");
            }

            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                close_after_action = true;
            }
        });

    if !open || close_after_action {
        ui_data.tex_ref_dialog_pass = None;
    }
}

impl TextureDefinition {
    fn pass_ui(&mut self, ui: &mut egui::Ui, pass_idx: usize, ui_data: &mut UiData, available_files: &[(u128, String)], monospace_width: f32, tmp_str: &mut String) -> (bool, Option<PassDrag>) {
        let pass = &mut self.passes[pass_idx];
        let group = ui.group(| ui | {
            let mut remove = false;
            let mut pass_drag = Option::<PassDrag>::None;
            ui.horizontal(| ui | {
                let isy = ui.spacing().interact_size.y;
                let drag = ui.add_sized([isy, isy],Image::new(include_image!("../assets/ui/reorder.svg"))
                    .tint(ui.visuals().text_color())
                    .sense(egui::Sense::drag()));
                if drag.drag_started() {
                    pass_drag = Some(PassDrag::StartedOrDragging(pass_idx));
                }
                if drag.dragged() {
                    pass_drag = Some(PassDrag::StartedOrDragging(pass_idx));
                }
                if drag.drag_stopped() {
                    pass_drag = Some(PassDrag::Stopped(pass_idx));
                }
                ui.add(Checkbox::without_text(&mut pass.enabled)).on_hover_text("Enable or disable this pass");
                tmp_str.clear();
                pass.write_name(tmp_str, pass_idx).expect("Writing to a string should never fail");

                let aw = ui.available_width();
                let tw = aw - isy - ui.spacing().item_spacing.x;
                let name_response = ui.add_sized([tw, isy], egui::TextEdit::singleline(tmp_str).hint_text("Pass Name"));
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

                if ui.add_sized([isy, isy], egui::Button::new("X")).clicked() {
                    remove = true;
                }
            });

            if pass.enabled {
                ui.horizontal_wrapped(| ui | {
                    add_color_edit(ui, &mut pass.color, monospace_width);
                    ui.label("Blend:");
                    add_enum_dropdown(ui, &mut pass.blend_mode, "blend_mode", pass_idx, false);
                });

                let reference_name = available_files
                    .iter()
                    .find(|(id, _)| Some(*id) == pass.tex_ref)
                    .map(|(_, name)| name.as_str())
                    .unwrap_or_else(|| {
                        match pass.tex_ref {
                            Some(_) => "Missing texture",
                            None => "None",
                        }
                    });
                ui.horizontal_wrapped(|ui| {
                    ui.label("Texture Ref:");
                    if ui.button(reference_name).clicked() {
                        ui_data.tex_ref_dialog_pass = Some(pass_idx);
                    }
                    if pass.tex_ref.is_some() && ui.button("Clear").clicked() {
                        pass.tex_ref = None;
                    }
                });

                ui.separator();
                mode_selector(ui, &mut pass.noise.noise_type, "Noise");

                if pass.uses_noise() {
                    ui.horizontal_wrapped(| ui | {
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
                let new_coverage = mode_selector(ui, &mut pass.coverage, "Shape");
                if let Some(cov) = new_coverage {
                    if cov.is_gizmo_editable() {
                        ui_data.preview_editing = Some(pass_idx);
                    } else if ui_data.preview_editing == Some(pass_idx) {
                        ui_data.preview_editing = None;
                    }
                }

                if pass.is_pattern() {
                    let is_editing = ui_data.preview_editing == Some(pass_idx);
                    if ui.add_sized([ui.available_width(), ui.spacing().interact_size.y],
                        Button::selectable(is_editing, if is_editing { "Editing Pattern..." } else { "Edit Pattern" })).clicked() {
                        if is_editing {
                            ui_data.preview_editing = None;
                        } else {
                            ui_data.preview_editing = Some(pass_idx);
                        }
                    }

                    ui.horizontal(| ui | {
                        ui.label("Position:");
                        ui.add(egui::DragValue::new(&mut pass.feature_x).range(-IMG_SIZE..=(IMG_SIZE - 1)).prefix("X:"));
                        ui.add(egui::DragValue::new(&mut pass.feature_y).range(-IMG_SIZE..=(IMG_SIZE - 1)).prefix("Y:"));
                        ui.label("Scale:");
                        ui.add(egui::DragValue::new(&mut pass.pattern.scale).range(1..=IMG_SIZE));
                        ui.checkbox(&mut pass.pattern.mirror_x, "Mirror");
                    });

                    ui.horizontal(| ui | {
                        let w = (ui.available_width() - 3.0 * ui.spacing().item_spacing.x) / 4.0;
                        if ui.add_sized([w, ui.spacing().interact_size.y], egui::Button::new("Fill")).clicked() {
                            pass.pattern.fill();
                        }
                        if ui.add_sized([w, ui.spacing().interact_size.y], egui::Button::new("Clear")).clicked() {
                            pass.pattern.clear();
                        }
                        if ui.add_sized([w, ui.spacing().interact_size.y], egui::Button::new("Invert")).clicked() {
                            pass.pattern.invert();
                        }
                        if ui.add_sized([w, ui.spacing().interact_size.y], egui::Button::new("Randomize")).clicked() {
                            pass.pattern.randomize();
                        }
                    });
                }

                if pass.is_rect() {
                    ui.horizontal(| ui | {
                        ui.label("Rect:");
                        ui.add(egui::DragValue::new(&mut pass.feature_x).range(-IMG_SIZE..=(IMG_SIZE - 1)).prefix("X:"));
                        ui.add(egui::DragValue::new(&mut pass.feature_y).range(-IMG_SIZE..=(IMG_SIZE - 1)).prefix("Y:"));
                        ui.add(egui::DragValue::new(&mut pass.rect.width).range(1..=(2 * IMG_SIZE - 1)).prefix("W:"));
                        ui.add(egui::DragValue::new(&mut pass.rect.height).range(1..=(2 * IMG_SIZE - 1)).prefix("H:"));
                        let is_editing = ui_data.preview_editing == Some(pass_idx);
                        let edit_button = egui::Button::selectable(is_editing, if is_editing { "Editing rect..." } else { "Edit rect" });
                        if ui.add_sized([ui.available_width(), ui.spacing().interact_size.y], edit_button).clicked() {
                            if is_editing {
                                ui_data.preview_editing = None;
                            } else {
                                ui_data.preview_editing = Some(pass_idx);
                            }
                        }
                    });
                    ui.horizontal_wrapped(| ui | {
                        ui.checkbox(&mut pass.rect.round.enabled, "Round").on_hover_text("Round rect corners");
                        if pass.rect.round.enabled {
                            ui.label("Radius:");
                            ui.add(egui::DragValue::new(&mut pass.rect.round.radius).range(1..=(pass.rect.width.min(pass.rect.height) / 2)));
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
            }
            (remove, pass_drag)
        });

        group.inner
    }
        
    pub(crate) fn definition_ui(&mut self, ui: &mut egui::Ui, ui_data: &mut UiData, name: &str, current_file_id: u128, available_files: &[(u128, String)], tmp_str: &mut String) {
        let monospace_id = egui::TextStyle::Monospace.resolve(ui.style());

        // Estimate width of one character (monospace assumption works best)
        let monospace_width = ui.fonts_mut(|f| {
            f.glyph_width(&monospace_id, 'W') // use a wide character as baseline
        });

        egui::ScrollArea::vertical().show(ui, | ui | {
            tmp_str.clear();
            write!(tmp_str, "File: {}", name).expect("Writing to a string should never fail");
            ui.horizontal(|ui| {
                ui.heading(&*tmp_str);
                if ui.button("Rename").clicked() {
                    ui_data.file_name_dialog = Some(FileNameDialogMode::Rename(name.to_string()));
                    ui_data.file_name_dialog_just_opened = true;
                }
            });

            ui.separator();

            ui.horizontal_wrapped(| ui | {
                ui.label("Light direction:");
                ui.add(egui::DragValue::new(&mut self.lighting_settings.direction[0]).range(-100..=100));
                ui.add(egui::DragValue::new(&mut self.lighting_settings.direction[1]).range(-100..=100));
                ui.add(egui::DragValue::new(&mut self.lighting_settings.direction[2]).range(1..=100));
                ui.label("Impact:");
                ui.add(egui::DragValue::new(&mut self.lighting_settings.impact).range(0..=100)).on_hover_text("0 = unlit; 100 = maximum contrast");
                ui.checkbox(&mut self.lighting_settings.shadows, "Shadows");
                if self.lighting_settings.shadows {
                    ui.checkbox(&mut self.lighting_settings.smooth_shadows, "Smooth");
                    if self.lighting_settings.smooth_shadows {
                        ui.add(egui::DragValue::new(&mut self.lighting_settings.smooth_kernel_size).range(1..=10).prefix("Kernel:")).on_hover_text("Gaussian kernel radius in pixels");
                    }
                    ui.checkbox(&mut self.lighting_settings.shadow_fade, "Fade");
                    if self.lighting_settings.shadow_fade {
                        ui.add(egui::DragValue::new(&mut self.lighting_settings.shadow_fade_distance).range(1..=(IMG_SIZE * 2)).prefix("Dist:")).on_hover_text("Distance in pixels over which shadows fade to nothing");
                    }
                }
            });
            ui.horizontal_wrapped( | ui | {
                ui.label("Ambient occlusion:");
                ui.add(egui::DragValue::new(&mut self.ao_settings.strength).range(0..=100)).on_hover_text("Ambient occlusion strength");
                ui.add(egui::DragValue::new(&mut self.ao_settings.radius).range(1..=(IMG_SIZE - 1)).prefix("Radius:")).on_hover_text("Higher = more distant occluders will contribute to AO");
                ui.add(egui::DragValue::new(&mut self.ao_settings.bias).range(0..=200).prefix("Directional:")).on_hover_text("Bias ambient occlusion based on light direction");
            //    ui.checkbox(&mut self.ao_settings.ignore_surface_normal, "Ignore Surface Normal").on_hover_text("Experimental; Probably not something you want to use");
            });
            
        
            let mut drag_op = Option::<PassDrag>::None;
            let mut drop_targets = Vec::<egui::Response>::with_capacity(self.passes.len() + 1);
            let mut remove = Option::<usize>::None;
            let pass_gap = 4.0_f32;
            let (_, resp) = ui.allocate_exact_size(egui::Vec2::new(ui.available_width(), pass_gap), egui::Sense::empty());
            drop_targets.push(resp);
            ui.scope(| ui | {
                for pass_idx in 0..self.passes.len() {
                    let (remove_this, new_drag) = self.pass_ui(ui, pass_idx, ui_data, available_files, monospace_width, tmp_str);
                    let (_, resp) = ui.allocate_exact_size(egui::Vec2::new(ui.available_width(), pass_gap), egui::Sense::empty());
                    drop_targets.push(resp);
                    if remove_this {
                        remove = Some(pass_idx);
                    }
                    drag_op = new_drag.or(drag_op);
                }
            });

            // Drag and drop re-order
            match drag_op {
                Some(PassDrag::StartedOrDragging(pass_idx)) => {
                    if let Some(hp) = ui.ctx().pointer_interact_pos() {
                        let insertion_idx = find_closest_y_center(hp.y, &drop_targets);
                        let insertion_rect = drop_targets[insertion_idx].rect;
                        ui.ctx().layer_painter(egui::LayerId::new(egui::Order::Background, egui::Id::new("pass_drag_line")))
                            .rect_filled(insertion_rect, 0.0, ui.visuals().selection.bg_fill);

                        egui::Area::new(egui::Id::new(&"pass_drag").with(pass_idx))
                            .order(egui::Order::Foreground)
                            .movable(false)
                            .interactable(false)
                            .fixed_pos(hp)
                            .show(ui.ctx(), | ui | {
                                egui::Frame::new()
                                    .fill(ui.visuals().window_fill())
                                    .inner_margin(4.0)
                                    .outer_margin(4.0)
                                    .stroke(ui.visuals().window_stroke())
                                    .show(ui, | ui | {
                                        tmp_str.clear();
                                        self.passes[pass_idx].write_name(tmp_str, pass_idx).expect("Writing to a string should never fail");
                                        ui.label(egui::RichText::new(&*tmp_str).strong());
                                    });
                            });
                        
                        let clip = ui.clip_rect();
                        let y_rel = (hp.y - clip.top()) / clip.height() - 0.5;
                        let scroll_dir = -y_rel.signum();
                        let scroll = ((y_rel.abs() + DRAG_SCROLL_PART - 0.5) / DRAG_SCROLL_PART).saturate() * scroll_dir;
                        let scroll_speed = scroll * DRAG_SCROLL_SPEED_MAX;
                        let dt = ui.ctx().input(| i | i.unstable_dt).max(0.05);
                        ui.scroll_with_delta_animation(
                            egui::Vec2::new(0.0, scroll_speed * dt),
                            egui::style::ScrollAnimation::none()
                        );
                    }
                },
                Some(PassDrag::Stopped(pass_idx)) => {
                    if let Some(hp) = ui.ctx().pointer_interact_pos() {
                        let mut insertion_idx = find_closest_y_center(hp.y, &drop_targets);
                        if insertion_idx > pass_idx {
                            insertion_idx -= 1;
                        }
                        if insertion_idx != pass_idx {
                            self.passes.swap(insertion_idx, pass_idx);
                        }
                    }
                },
                None => {}
            }

            // Deletion
            if let Some(idx) = remove {
                self.passes.remove(idx);
            }

            if add_full_width(ui, Button::new("Add Pass")).clicked() {
                self.passes.push(TexturePass::new());
            }
        });

        show_file_name_dialog(ui.ctx(), ui_data, tmp_str);
        show_tex_ref_dialog(ui.ctx(), self, ui_data, current_file_id, available_files);
    }
}
