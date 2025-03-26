use eframe::{egui, App, Frame};

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "CnF-Infinity",
        options,
        Box::new(|_cc| Ok(Box::new(MyApp::default()))),
    )
}

struct NoteNode {
    id: usize,
    position: egui::Pos2,
    size: egui::Vec2,
    text: String,
    is_dragging: bool,
}

#[derive(Clone)]
struct Stroke {
    points: Vec<egui::Pos2>,
    color: egui::Color32,
    thickness: f32,
}

struct MyApp {
    zoom: f32,
    offset: egui::Vec2,
    dragging: bool,
    drag_start: egui::Pos2,
    tools_open: bool,
    next_note_id: usize,
    note_nodes: Vec<NoteNode>,
    selected_node: Option<usize>,
    marker_active: bool,
    current_stroke: Option<Stroke>,
    strokes: Vec<Stroke>,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            zoom: 2.0,
            offset: egui::Vec2::ZERO,
            dragging: false,
            drag_start: egui::Pos2::ZERO,
            tools_open: false,
            next_note_id: 0,
            note_nodes: Vec::new(),
            selected_node: None,
            marker_active: false,
            current_stroke: None,
            strokes: Vec::new(),
        }
    }
}

impl App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        ctx.set_visuals(egui::Visuals {
            code_bg_color: egui::Color32::from_rgb(32, 37, 43),
            panel_fill: egui::Color32::from_rgb(40, 44, 52),
            window_fill: egui::Color32::from_rgb(40, 44, 52),
            faint_bg_color: egui::Color32::from_rgb(40, 44, 52),
            extreme_bg_color: egui::Color32::from_rgb(40, 44, 52),
            ..Default::default()
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            let response = ui.interact(ui.max_rect(), ui.id(), egui::Sense::drag());

            let spacing = (25.0 * self.zoom).max(1.0); // 2x denser grid
            let grid_color = egui::Color32::from_gray(60);
            let stroke = egui::Stroke::new(1.0, grid_color);

            let bounds = ui.clip_rect();
            let top_left = bounds.left_top() - self.offset;
            let bottom_right = bounds.right_bottom() - self.offset;

            let start_x = (top_left.x / spacing).floor() * spacing;
            let end_x = (bottom_right.x / spacing).ceil() * spacing;
            let start_y = (top_left.y / spacing).floor() * spacing;
            let end_y = (bottom_right.y / spacing).ceil() * spacing;

            let painter = ui.painter_at(bounds);

            for x in (start_x as i32..=end_x as i32).step_by(spacing as usize) {
                let x = x as f32;
                painter.line_segment(
                    [
                        egui::pos2(x, top_left.y) + self.offset,
                        egui::pos2(x, bottom_right.y) + self.offset,
                    ],
                    stroke,
                );
            }

            for y in (start_y as i32..=end_y as i32).step_by(spacing as usize) {
                let y = y as f32;
                painter.line_segment(
                    [
                        egui::pos2(top_left.x, y) + self.offset,
                        egui::pos2(bottom_right.x, y) + self.offset,
                    ],
                    stroke,
                );
            }

            // Draw finalized strokes
            for stroke in &self.strokes {
                for w in stroke.points.windows(2) {
                    if let [a, b] = w {
                        let a = (*a) * self.zoom + self.offset;
                        let b = (*b) * self.zoom + self.offset;
                        painter.line_segment([a, b], egui::Stroke::new(stroke.thickness * self.zoom, stroke.color));
                    }
                }
            }

            // Draw current stroke in progress
            if let Some(stroke) = &self.current_stroke {
                for w in stroke.points.windows(2) {
                    if let [a, b] = w {
                        let a = (*a) * self.zoom + self.offset;
                        let b = (*b) * self.zoom + self.offset;
                        painter.line_segment([a, b], egui::Stroke::new(stroke.thickness * self.zoom, stroke.color));
                    }
                }
            }

            let pointer = ctx.input(|i| i.pointer.clone());
            if self.marker_active {
                if pointer.primary_down() {
                    let pos = pointer.interact_pos();
                    if let Some(pos) = pos {
                        let canvas_pos = (pos - self.offset) / self.zoom;
                        if let Some(stroke) = self.current_stroke.as_mut() {
                            stroke.points.push(canvas_pos);
                        } else {
                            self.current_stroke = Some(Stroke {
                                points: vec![canvas_pos],
                                color: egui::Color32::from_rgb(187, 192, 206), // #BBC0CE
                                thickness: 2.0,
                            });
                        }
                    }
                } else if let Some(stroke) = self.current_stroke.take() {
                    self.strokes.push(stroke);
                }
            }

            if !self.marker_active {
                if response.drag_started() {
                    self.drag_start = response.interact_pointer_pos().unwrap_or(self.drag_start);
                    self.dragging = true;
                }

                if response.drag_stopped() {
                    self.dragging = false;
                }

                if self.dragging {
                    let current_pos = response.interact_pointer_pos().unwrap();
                    let delta = current_pos - self.drag_start;
                    self.offset += delta;
                    self.drag_start = current_pos;
                }
            }

            let scroll = ctx.input(|i| i.raw_scroll_delta.y);
            if scroll != 0.0 {
                self.zoom *= 1.0 + scroll * 0.001;
                self.zoom = self.zoom.clamp(0.4, 4.0);
            }

            let mut i = 0;
            while i < self.note_nodes.len() {
                let note = &mut self.note_nodes[i];

                let scaled_size = (note.size * self.zoom).max(egui::vec2(1.0, 1.0));
                let scaled_position = (note.position * self.zoom) + self.offset;
                let rect = egui::Rect::from_min_size(scaled_position, scaled_size);

                let id = ui.make_persistent_id(note.id);
                let interact = ui.interact(rect, id, egui::Sense::click_and_drag());
                if interact.drag_started() {
                    note.is_dragging = true;
                }
                if interact.drag_stopped() {
                    note.is_dragging = false;
                }
                if note.is_dragging {
                    note.position += interact.drag_delta() / self.zoom;
                }

                ui.allocate_ui_at_rect(rect, |ui| {
                    egui::Frame::none()
                        .fill(egui::Color32::from_rgb(32, 37, 43))
                        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(80, 80, 80)))
                        .shadow(egui::epaint::Shadow::NONE)
                        .show(ui, |ui| {
                            let font_id = egui::FontId::monospace(6.0 * self.zoom);
                            ui.vertical_centered(|ui| {
                                if ui.button("⋮").on_hover_text("Options").clicked() {
                                    if self.selected_node == Some(i) {
                                        self.selected_node = None;
                                    } else {
                                        self.selected_node = Some(i);
                                    }
                                }
                            });
                            ui.add_sized(
                                scaled_size,
                                egui::TextEdit::multiline(&mut note.text)
                                    .font(font_id.clone())
                                    .frame(false)
                                    .background_color(egui::Color32::from_rgb(32, 37, 43))
                                    .text_color(egui::Color32::WHITE),
                            );
                            ui.add(egui::DragValue::new(&mut note.size.x).clamp_range(1.0..=400.0));
                            ui.add(egui::DragValue::new(&mut note.size.y).clamp_range(1.0..=400.0));
                        });
                });

                if Some(i) == self.selected_node {
                    let menu_pos = scaled_position + egui::vec2(0.0, -25.0);
                    egui::Area::new(format!("note_menu_{}", note.id).into())
                        .fixed_pos(menu_pos)
                        .show(ctx, |ui| {
                            let mut to_remove = false;
                            ui.horizontal(|ui| {
                                if ui.button("⬆ Backward").clicked() && i > 0 {
                                    self.note_nodes.swap(i, i - 1);
                                    self.selected_node = Some(i - 1);
                                }
                                if ui.button("⬇ Forward").clicked() && i < self.note_nodes.len() - 1 {
                                    self.note_nodes.swap(i, i + 1);
                                    self.selected_node = Some(i + 1);
                                }
                                if ui.button("❌ Delete").clicked() {
                                    to_remove = true;
                                }
                            });
                            if to_remove {
                                self.note_nodes.remove(i);
                                self.selected_node = None;
                                return;
                            }
                        });
                }
                i += 1;
            }

            painter.text(
                egui::pos2(10.0, 10.0),
                egui::Align2::LEFT_TOP,
                format!("Zoom: {:.2} | Offset: {:?}", self.zoom, self.offset),
                egui::TextStyle::Monospace.resolve(ui.style()),
                ui.visuals().text_color(),
            );

            egui::Area::new("tool_overlay".into())
                .fixed_pos(egui::pos2(30.0, 30.0))
                .show(ctx, |ui| {
                    egui::Frame::popup(ui.style()).show(ui, |ui| {
                        ui.horizontal(|ui| {
                            if ui.button("🛠 Tools").clicked() {
                                self.tools_open = !self.tools_open;
                            }

                            if self.tools_open {
                                if ui.button("+ Code Node").clicked() {
                                    // future logic to add a code node
                                }
                                if ui.button("+ Note Node").clicked() {
                                    self.note_nodes.push(NoteNode {
                                        id: self.next_note_id,
                                        position: egui::pos2(100.0, 100.0),
                                        size: egui::vec2(200.0, 150.0),
                                        text: String::new(),
                                        is_dragging: false,
                                    });
                                    self.next_note_id += 1;
                                }
                                if ui.button("✏ Marker").clicked() {
                                    self.marker_active = !self.marker_active;
                                }
                                if ui.button("Reset Zoom").clicked() {
                                    self.zoom = 2.0;
                                }
                            }
                        });
                    });
                });
        });
    }
}
