use std::fmt::Write as _;

use egui::{Color32, Stroke};

use crate::prelude::*;
use crate::{DisplayMode, TextureHandleSet, UiData, definition::{Coverage, Pattern, TextureDefinition, TexturePass}, processing::TextureLayers, util::add_enum_dropdown};


#[allow(unused)]
fn with_alpha(alpha: f32, mut stroke: egui::Stroke) -> egui::Stroke {
    let [r, g, b, _] = stroke.color.to_srgba_unmultiplied();
    stroke.color = egui::Color32::from_rgba_unmultiplied(r, g, b, (alpha * 255.0).round().clamp(0.0, 255.0) as u8);
    stroke
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum DragTarget {
    Rect,
    Edge(usize),
    CornerHandle(usize),
    RoundHandle,
    Tile(usize),
}

impl DragTarget {
    fn start_values(&self, pass: &TexturePass) -> (i32, i32, i32, i32) {
        match self {
            DragTarget::RoundHandle => (pass.rect.round.radius, 0, 0, 0),
            DragTarget::Tile(0) => (pass.tile.x_gap, 0, 0, 0),
            DragTarget::Tile(1) => (pass.tile.y_gap, 0, 0, 0),
            _ => (pass.feature_x, pass.feature_y, pass.rect.width, pass.rect.height),
        }
    }

    fn update_pass(&self, pass: &mut TexturePass, pointer: egui::Pos2, drag: &RectParamDrag, image_scale: i32) {
        let delta = (pointer - drag.pointer_start) / image_scale as f32;
        let dx = delta.x.round() as i32;
        let dy = delta.y.round() as i32;
        match self {
            DragTarget::Rect => {
                pass.feature_x = drag.value_start.0 + dx;
                pass.feature_y = drag.value_start.1 + dy;
            }
            DragTarget::Edge(0) => {
                pass.feature_x = drag.value_start.0 + dx;
                pass.rect.width = (drag.value_start.2 - dx).max(1);
            },
            DragTarget::Edge(1) => {
                pass.rect.width = (drag.value_start.2 + dx).max(1);
            },
            DragTarget::Edge(2) => {
                pass.feature_y = drag.value_start.1 + dy;
                pass.rect.height = (drag.value_start.3 - dy).max(1);
            },
            DragTarget::Edge(3) => {
                pass.rect.height = (drag.value_start.3 + dy).max(1);
            },
            DragTarget::CornerHandle(0) => {
                pass.feature_x = drag.value_start.0 + dx;
                pass.feature_y = drag.value_start.1 + dy;
                pass.rect.width = (drag.value_start.2 - dx).max(1);
                pass.rect.height = (drag.value_start.3 - dy).max(1);
            },
            DragTarget::CornerHandle(1) => {
                pass.feature_y = drag.value_start.1 + dy;
                pass.rect.width = (drag.value_start.2 + dx).max(1);
                pass.rect.height = (drag.value_start.3 - dy).max(1);
            },
            DragTarget::CornerHandle(2) => {
                pass.feature_x = drag.value_start.0 + dx;
                pass.rect.width = (drag.value_start.2 - dx).max(1);
                pass.rect.height = (drag.value_start.3 + dy).max(1);
            },
            DragTarget::CornerHandle(3) => {
                pass.rect.width = (drag.value_start.2 + dx).max(1);
                pass.rect.height = (drag.value_start.3 + dy).max(1);
            },
            DragTarget::RoundHandle => {
                pass.rect.round.radius = (drag.value_start.0 - dy).clamp(0, pass.rect.width.min(pass.rect.height) / 2);
            },
            DragTarget::Tile(0) => {
                pass.tile.x_gap = (drag.value_start.0 + dx).max(0);
            },
            DragTarget::Tile(1) => {
                pass.tile.y_gap = (drag.value_start.0 + dy).max(0);
            },
            _ => unreachable!(),
        }
    }
}

#[inline]
fn choose<T: Copy>(target: Option<DragTarget>, target_type: DragTarget, normal: T, active: T) -> T {
    if target == Some(target_type) {
        active
    } else {
        normal
    }
}

fn calculate_screen_rect(x: i32, y: i32, width: i32, height: i32, within: egui::Rect, scale: i32) -> egui::Rect {
    let mn = within.min + egui::Vec2::new(x as f32 * scale as f32, y as f32 * scale as f32);
    let sz = egui::Vec2::new(width as f32 * scale as f32, height as f32 * scale as f32);
    egui::Rect::from_min_size(mn, sz)
}

struct GrabCandidate {
    target: Option<DragTarget>,
    distance: f32,
}

impl GrabCandidate {
    const GRAB_TOLERANCE: f32 = 6.0;

    fn new() -> Self {
        Self { target: None, distance: f32::MAX }
    }

    fn update_grab_target(&mut self, candidate: DragTarget, distance: f32) {
        let distance = distance.max(0.0); // When inside a handle, the latter one takes priority
        if distance > Self::GRAB_TOLERANCE {
            return;
        }
        if self.target.is_some() {
            if distance <= self.distance {
                self.target = Some(candidate);
                self.distance = distance;
            }
        } else {
            self.target = Some(candidate);
            self.distance = distance;
        }
    }

    fn is(&self, target_type: DragTarget) -> bool {
        match self.target {
            Some(t) => t == target_type,
            None => false,
        }
    }

    fn adjust_edges_for_round(&mut self, hp: egui::Pos2, round_sz: f32, edit_rect: egui::Rect){
        // Special case the edges in the rounded part -- being able to grab them here feels wrong, so we re-direct to the corner instead
        match self.target {
            Some(DragTarget::Edge(0)) =>
                if hp.y < edit_rect.top() + round_sz - GrabCandidate::GRAB_TOLERANCE {
                    self.target = Some(DragTarget::CornerHandle(0));
                } else if hp.y > edit_rect.bottom() - round_sz + GrabCandidate::GRAB_TOLERANCE {
                    self.target = Some(DragTarget::CornerHandle(2));
                },
            Some(DragTarget::Edge(1)) =>
                if hp.y < edit_rect.top() + round_sz - GrabCandidate::GRAB_TOLERANCE {
                    self.target = Some(DragTarget::CornerHandle(1));
                } else if hp.y > edit_rect.bottom() - round_sz + GrabCandidate::GRAB_TOLERANCE {
                    self.target = Some(DragTarget::CornerHandle(3));
                },
            Some(DragTarget::Edge(2)) =>
                if hp.x < edit_rect.left() + round_sz - GrabCandidate::GRAB_TOLERANCE {
                    self.target = Some(DragTarget::CornerHandle(0));
                } else if hp.x > edit_rect.right() - round_sz + GrabCandidate::GRAB_TOLERANCE {
                    self.target = Some(DragTarget::CornerHandle(1));
                },
            Some(DragTarget::Edge(3)) =>
                if hp.x < edit_rect.left() + round_sz - GrabCandidate::GRAB_TOLERANCE {
                    self.target = Some(DragTarget::CornerHandle(2));
                } else if hp.x > edit_rect.right() - round_sz + GrabCandidate::GRAB_TOLERANCE {
                    self.target = Some(DragTarget::CornerHandle(3));
                },
            _ => {},
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct RectParamDrag {
    target: DragTarget,
    pointer_start: egui::Pos2,
    value_start: (i32, i32, i32, i32),
}

#[derive(Debug, Clone)]
pub(crate) struct PatternDrawDrag {
    last_pointer: egui::Pos2,
}

#[derive(Debug, Clone)]
pub(crate) struct PatternRepositionDrag {
    pointer_start: egui::Pos2,
    position_start: (i32, i32),
}

#[derive(Debug, Clone)]
pub(crate) struct PatternScaleDrag {
    offset: egui::Vec2,
}

#[derive(Debug, Clone, Default)]
pub(crate) enum OngoingDrag {
    #[default] None,
    RectParam(RectParamDrag),
    PatternDraw(PatternDrawDrag),
    PatternReposition(PatternRepositionDrag),
    PatternScale(PatternScaleDrag),
}


impl TexturePass {
    fn rect_gizmos(&mut self, drag: &mut OngoingDrag, ui: &mut egui::Ui, pointer_response: &egui::Response, image_rect: egui::Rect, image_scale: i32) {
        if let OngoingDrag::RectParam(drag) = drag {
            // Update ongoing drag
            if pointer_response.dragged_by(egui::PointerButton::Primary) {
                if let Some(hp) = pointer_response.interact_pointer_pos() {
                    drag.target.update_pass(self, hp, drag, image_scale);
                }
            }
        } else {
            *drag = OngoingDrag::None;
        }

        // Calculate gizmo positions
        let edit_rect = calculate_screen_rect(self.feature_x, self.feature_y, self.rect.width, self.rect.height, image_rect, image_scale);
        let round_sz = if self.rect.round.enabled { self.rect.round.radius as f32 * image_scale as f32 } else { 0.0 };
        
        let corner_handle_size = 8.0;
        let corner_handle_size_vec = egui::Vec2::splat(corner_handle_size);
        let handles = [
            egui::Rect::from_center_size(edit_rect.left_top(), corner_handle_size_vec),
            egui::Rect::from_center_size(edit_rect.right_top(), corner_handle_size_vec),
            egui::Rect::from_center_size(edit_rect.left_bottom(), corner_handle_size_vec),
            egui::Rect::from_center_size(edit_rect.right_bottom(), corner_handle_size_vec),
        ];
        let round_handle_center = if self.rect.round.enabled {
            Some(edit_rect.right_bottom() - egui::Vec2::new(0.0, round_sz))
        } else {
            None
        };
        let round_handle_rad = corner_handle_size / 2.0;
        let line_width: f32 = 2.0;

        let htile_rect = if self.tile.enabled && self.tile.x_count > 1 {
            let offset = egui::Vec2::new(((self.rect.width + self.tile.x_gap) * image_scale) as f32, 0.0);
            Some(egui::Rect::from_min_max(edit_rect.left_top() + offset, edit_rect.right_bottom() + offset))
        } else {
            None
        };

        let vtile_rect = if self.tile.enabled && self.tile.y_count > 1 {
            let offset = egui::Vec2::new(0.0, ((self.rect.height + self.tile.y_gap) * image_scale) as f32);
            Some(egui::Rect::from_min_max(edit_rect.left_top() + offset, edit_rect.right_bottom() + offset))
        } else {
            None
        };

        // Check grab target
        let drag_target = if let OngoingDrag::RectParam(drag) = drag {
            Some(drag.target)
        } else if let Some(hp) = pointer_response.hover_pos() {
            let mut grab_target = GrabCandidate::new();
            grab_target.update_grab_target(DragTarget::Rect, edit_rect.distance_to_pos(hp).max(GrabCandidate::GRAB_TOLERANCE));
            if grab_target.is(DragTarget::Rect) { // Check edges
                let lwh = 0.5 * line_width;
                grab_target.update_grab_target(DragTarget::Edge(0), (hp.x - (edit_rect.left() - lwh)).abs() - lwh);
                grab_target.update_grab_target(DragTarget::Edge(1), (hp.x - (edit_rect.right() + lwh)).abs() - lwh);
                grab_target.update_grab_target(DragTarget::Edge(2), (hp.y - (edit_rect.top() - lwh)).abs() - lwh);
                grab_target.update_grab_target(DragTarget::Edge(3), (hp.y - (edit_rect.bottom() + lwh)).abs() - lwh);
            }
            
            for (i, handle) in handles.iter().enumerate() {
                grab_target.update_grab_target(DragTarget::CornerHandle(i), handle.distance_to_pos(hp));
            }

            if let Some(round_center) = round_handle_center {
                grab_target.update_grab_target(DragTarget::RoundHandle, (round_center - hp).length() - round_handle_rad);
            }

            if let Some(htr) = htile_rect {
                grab_target.update_grab_target(DragTarget::Tile(0), htr.distance_to_pos(hp));
            }
            if let Some(vtr) = vtile_rect {
                grab_target.update_grab_target(DragTarget::Tile(1), vtr.distance_to_pos(hp));
            }

            grab_target.adjust_edges_for_round(hp, round_sz, edit_rect);
            
            if let Some(target) = grab_target.target {
                if pointer_response.drag_started_by(egui::PointerButton::Primary) {
                    *drag = OngoingDrag::RectParam(RectParamDrag { target, pointer_start: hp, value_start: target.start_values(self) });
                }
            }

            grab_target.target
        } else {
            None
        };

        // Draw gizmos
        let color = Color32::GREEN;
        let active_color = Color32::WHITE;
        let line_stroke = Stroke::new(line_width, color);
        let active_line_stroke = Stroke::new(2.0 * line_width, active_color);
        let handle_color = Color32::BLACK;
        let active_handle_color = Color32::WHITE;
        let tile_color = Color32::from_rgba_unmultiplied(0, 255, 255, 85);
        let tile_stroke = Stroke::new(line_width, tile_color);

        let painter  = ui.painter();

        if let Some(htr) = htile_rect {
            painter.rect_stroke(htr, round_sz,
                choose(drag_target, DragTarget::Tile(0), tile_stroke, active_line_stroke), egui::StrokeKind::Outside);
        }
        if let Some(vtr) = vtile_rect {
            painter.rect_stroke(vtr, round_sz,
                choose(drag_target, DragTarget::Tile(1), tile_stroke, active_line_stroke), egui::StrokeKind::Outside);
        }

        painter.rect_stroke(edit_rect, round_sz, choose(drag_target, DragTarget::Rect, line_stroke, active_line_stroke), egui::StrokeKind::Outside);
        if let Some(DragTarget::Edge(edge_index)) = drag_target {
            let lwh = 0.5 * line_width;
            let (p1, p2, offset, round_dir) = match edge_index {
                0 => (edit_rect.left_top(),    edit_rect.left_bottom(),  egui::Vec2::new(-lwh, 0.0), egui::Vec2::new(0.0, 1.0)),
                1 => (edit_rect.right_top(),   edit_rect.right_bottom(), egui::Vec2::new(lwh, 0.0),  egui::Vec2::new(0.0, 1.0)),
                2 => (edit_rect.left_top(),    edit_rect.right_top(),    egui::Vec2::new(0.0, -lwh), egui::Vec2::new(1.0, 0.0)),
                3 => (edit_rect.left_bottom(), edit_rect.right_bottom(), egui::Vec2::new(0.0, lwh),  egui::Vec2::new(1.0, 0.0)),
                _ => unreachable!(),
            };
            let start = p1 + offset + round_dir * round_sz;
            let end = p2 + offset - round_dir * round_sz;
            painter.line_segment([start, end], active_line_stroke);
        }

        for (i, h) in handles.iter().enumerate() {
            painter.rect_filled(*h, 0.0, choose(drag_target, DragTarget::CornerHandle(i), handle_color, active_handle_color));
            painter.rect_stroke(*h, 0.0, choose(drag_target, DragTarget::CornerHandle(i), line_stroke, active_line_stroke), egui::StrokeKind::Outside);
        }

        if let Some(round_center) = round_handle_center {
            painter.circle_filled(round_center, round_handle_rad, choose(drag_target, DragTarget::RoundHandle, handle_color, active_handle_color));
            painter.circle_stroke(round_center, round_handle_rad, choose(drag_target, DragTarget::RoundHandle, line_stroke, active_line_stroke));
        }
    }

    fn pattern_gizmos(&mut self, drag: &mut OngoingDrag, ui: &mut egui::Ui, pointer_response: &egui::Response, image_rect: egui::Rect, image_scale: i32) {
        // Drag updates
        match drag {
            OngoingDrag::PatternDraw(d) => {
                let draw = pointer_response.dragged_by(egui::PointerButton::Primary);
                let erase = pointer_response.dragged_by(egui::PointerButton::Secondary);
                if draw || erase {
                    if let Some(hp) = pointer_response.interact_pointer_pos() {
                        let last_x = ((d.last_pointer.x - image_rect.min.x) / image_scale as f32 - self.feature_x as f32) / self.pattern.scale as f32;
                        let last_y = ((d.last_pointer.y - image_rect.min.y) / image_scale as f32 - self.feature_y as f32) / self.pattern.scale as f32;
                        let new_x = ((pointer_response.interact_pointer_pos().unwrap().x - image_rect.min.x) / image_scale as f32 - self.feature_x as f32) / self.pattern.scale as f32;
                        let new_y = ((pointer_response.interact_pointer_pos().unwrap().y - image_rect.min.y) / image_scale as f32 - self.feature_y as f32) / self.pattern.scale as f32;
                        self.pattern.set_line(last_x, last_y, new_x, new_y, draw);
                        d.last_pointer = hp;
                    }
                } else {
                    *drag = OngoingDrag::None;
                }
            },
            OngoingDrag::PatternReposition(d) => {
                if pointer_response.dragged_by(egui::PointerButton::Primary) {
                    if let Some(hp) = pointer_response.interact_pointer_pos() {
                        let delta = hp - d.pointer_start;
                        let delta_px = (delta / image_scale as f32).round();
                        self.feature_x = (d.position_start.0 as f32 + delta_px.x).round() as i32;
                        self.feature_y = (d.position_start.1 as f32 + delta_px.y).round() as i32;
                    }
                } else {
                    *drag = OngoingDrag::None;
                }
            },
            OngoingDrag::PatternScale(scl) => {
                if pointer_response.dragged_by(egui::PointerButton::Primary) {
                    if let Some(hp) = pointer_response.interact_pointer_pos() {
                        let rel_pos = hp - scl.offset - (image_rect.left_top() + egui::Vec2::new(self.feature_x as f32, self.feature_y as f32) * image_scale as f32);
                        let new_scale = rel_pos.y / (image_scale * Pattern::SIZE) as f32;
                        self.pattern.scale = new_scale.round().max(1.0) as i32;
                    }
                } else {
                    *drag = OngoingDrag::None;
                }
            },
            _ => *drag = OngoingDrag::None,
        }

        let pattern_size = self.pattern.scale * Pattern::SIZE;
        let pattern_rect = calculate_screen_rect(self.feature_x, self.feature_y, pattern_size, pattern_size, image_rect, image_scale);

        // Calculate positions
        let grab_handle_size = image_scale as f32 * egui::Vec2::new(4.0, 6.0);
        let grab_handle_dist: f32 = image_scale as f32;
        let grab_handle_pos = pattern_rect.left_top() + egui::Vec2::new(-0.5 * grab_handle_size.x - grab_handle_dist, 0.5 * pattern_rect.height());
        let grab_handle_rect = egui::Rect::from_center_size(grab_handle_pos, grab_handle_size);
        let corner_handle_size = 10.0;
        let br = pattern_rect.right_bottom();
        let corner_handle_rect = egui::Rect::from_min_max(br, br + egui::Vec2::splat(corner_handle_size));

        // Drag start
        if pointer_response.drag_started_by(egui::PointerButton::Primary) || pointer_response.drag_started_by(egui::PointerButton::Secondary) {
            if let Some(hp) = pointer_response.interact_pointer_pos() {
                if grab_handle_rect.contains(hp) {
                    *drag = OngoingDrag::PatternReposition(PatternRepositionDrag { pointer_start: hp, position_start: (self.feature_x, self.feature_y) });
                } else if pattern_rect.contains(hp) {
                    *drag = OngoingDrag::PatternDraw(PatternDrawDrag { last_pointer: hp });
                } else if corner_handle_rect.contains(hp) {
                    *drag = OngoingDrag::PatternScale(PatternScaleDrag { offset: hp - pattern_rect.right_bottom() });
                }
            }
        }
  
        
        // Paint
        let painter = ui.painter();
        painter.rect_stroke(pattern_rect, 0.0, Stroke::new(2.0, Color32::WHITE), egui::StrokeKind::Outside);
        painter.rect_filled(grab_handle_rect, image_scale as f32 * 0.5, Color32::WHITE);

        for i in 0..6 { 
            let pos = grab_handle_rect.center() + egui::Vec2::new(
                -0.75 * image_scale as f32 + image_scale as f32 * 1.5 * (i % 2) as f32,
                -1.5 * image_scale as f32 + image_scale as f32 * 1.5 * (i / 2) as f32
            );
            painter.circle_filled(pos, image_scale as f32 * 0.5, egui::Visuals::dark().widgets.active.bg_fill);
        }

        painter.rect_filled(corner_handle_rect, 0.0, Color32::BLACK);
        painter.rect_stroke(corner_handle_rect, 0.0, Stroke::new(2.0, Color32::WHITE), egui::StrokeKind::Inside);
    }
}

impl TextureDefinition {
    pub(crate) fn add_preview(&mut self, ui: &mut egui::Ui, ui_data: &mut UiData, textures: &TextureHandleSet, layers: &TextureLayers, tmp_str: &mut String) {
        ui.horizontal(| ui | {
            ui.label("Display Mode:");
            add_enum_dropdown(ui, &mut ui_data.display_mode, "display_mode", 0, false);
        });
        
        ui.centered_and_justified(| ui | {
            let available = ui.available_rect_before_wrap();
            let sense = egui::Sense::hover() | egui::Sense::drag();
            let pointer_response = ui.interact(available, ui.id().with("preview_drag"), sense);
            if pointer_response.drag_stopped_by(egui::PointerButton::Primary) {
                ui_data.drag = OngoingDrag::None;
            }

            let image_scale = (available.width().min(available.height()) / IMG_SIZE as f32).floor().max(1.0) as i32;
            let image_size_sc = IMG_SIZE as f32 * image_scale as f32;
            let image_size = egui::Vec2::new(image_size_sc, image_size_sc);
            let image_rect = egui::Rect::from_center_size(available.center(), image_size);
            
            let tex = match ui_data.display_mode {
                DisplayMode::Lit => &textures.lit,
                DisplayMode::Albedo => &textures.albedo,
                DisplayMode::Depth => &textures.depth,
                DisplayMode::Normal => &textures.normal,
                DisplayMode::AmbientOcclusion => &textures.ao,
            };

            let img = egui::Image::new(tex)
                .fit_to_exact_size(image_size)
                .sense(egui::Sense::hover());
            img.paint_at(ui, image_rect);

            if let Some(pass_idx) = ui_data.preview_editing {
                if pass_idx >= self.passes.len() {
                    ui_data.preview_editing = None;
                } else {
                    match self.passes[pass_idx].coverage {
                        Coverage::Rectangle => self.passes[pass_idx].rect_gizmos(&mut ui_data.drag, ui, &pointer_response, image_rect, image_scale),
                        Coverage::Pattern => self.passes[pass_idx].pattern_gizmos(&mut ui_data.drag, ui, &pointer_response, image_rect, image_scale),
                        _ => {},
                    }
                }
            };

            if pointer_response.hovered() {
                if let Some(hover_pos) = pointer_response.hover_pos() {
                    let x = ((hover_pos.x - image_rect.min.x) / image_scale as f32).floor() as i32;
                    let y = ((hover_pos.y - image_rect.min.y) / image_scale as f32).floor() as i32;
                    if x >= 0 && x < IMG_SIZE && y >= 0 && y < IMG_SIZE {
                        pointer_response.on_hover_ui_at_pointer(| ui | {
                            tmp_str.clear();
                            let index = idx(x, y);
                            write!(tmp_str, "Pixel ({}, {})", x, y).unwrap();

                            let albedo = layers.albedo[index];
                            write!(tmp_str, "\nAlbedo: ({:.3}, {:.3}, {:.3})", albedo.x, albedo.y, albedo.z).unwrap();

                            let depth = layers.depth[index];
                            write!(tmp_str, "\nDepth: {:.3}", depth).unwrap();

                            let normal = layers.normal[index];
                            write!(tmp_str, "\nNormal: ({:.3}, {:.3}, {:.3})", normal.x, normal.y, normal.z).unwrap();

                            ui.label(&*tmp_str);
                        });
                    }
                }
            }
        });
    }
}
