use eframe::{egui, App, Frame};
use rfd::FileDialog;
use std::fs;

// New enums for connections
#[derive(Clone, Copy, PartialEq)]
enum NodeType {
    Note,
    Code,
}

#[derive(Clone, Copy, PartialEq)]
enum Side {
    Top,
    Bottom,
    Left,
    Right,
}

#[derive(Clone)]
struct NodeConnection {
    start_node_id: usize,
    start_node_type: NodeType,
    start_side: Side,
    end_node_id: usize,
    end_node_type: NodeType,
    end_side: Side,
    control_points: Option<(egui::Pos2, egui::Pos2)>,
    color: egui::Color32,
}

#[derive(Clone)]
struct NoteNode {
    id: usize,
    position: egui::Pos2,
    size: egui::Vec2,
    text: String,
    is_dragging: bool,
    locked: bool,
}

#[derive(Clone)]
struct CodeNode {
    id: usize,
    position: egui::Pos2,
    size: egui::Vec2,
    file_path: String,
    code: String,
    is_dragging: bool,
    locked: bool,
    line_offset: Option<usize>,
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
    eraser_active: bool,
    current_stroke: Option<Stroke>,
    strokes: Vec<Stroke>,
    code_nodes: Vec<CodeNode>,
    project_root: Option<std::path::PathBuf>,

    // Connection-related fields
    connections: Vec<NodeConnection>,
    arrow_connection_active: bool,
    connection_start: Option<(usize, NodeType, Side)>,
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
            eraser_active: false,
            current_stroke: None,
            strokes: Vec::new(),
            code_nodes: Vec::new(),
            project_root: None,
            connections: Vec::new(),
            arrow_connection_active: false,
            connection_start: None,
        }
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "CnF-Infinity",
        options,
        Box::new(|_cc| Ok(Box::new(MyApp::default()))),
    )
}

fn compute_cubic_bezier_points(
    p0: egui::Pos2,
    p1: egui::Pos2,
    p2: egui::Pos2,
    p3: egui::Pos2,
    segments: usize,
) -> Vec<egui::Pos2> {
    let mut points = Vec::with_capacity(segments + 1);
    for i in 0..=segments {
        let t = i as f32 / segments as f32;
        let one_minus_t = 1.0 - t;
        let a = one_minus_t.powi(3);
        let b = 3.0 * t * one_minus_t.powi(2);
        let c = 3.0 * t.powi(2) * one_minus_t;
        let d = t.powi(3);
        let x = a * p0.x + b * p1.x + c * p2.x + d * p3.x;
        let y = a * p0.y + b * p1.y + c * p2.y + d * p3.y;
        points.push(egui::pos2(x, y));
    }
    points
}

// Helper function: returns the outward normal for a given side.
fn side_normal(side: Side) -> egui::Vec2 {
    match side {
        Side::Top => egui::vec2(0.0, -1.0),
        Side::Bottom => egui::vec2(0.0, 1.0),
        Side::Left => egui::vec2(-1.0, 0.0),
        Side::Right => egui::vec2(1.0, 0.0),
    }
}

// Helper function: compute a connection point along a node's side.
// If multiple arrows come from the same side, they are evenly distributed.
fn connection_point(
    node_pos: egui::Pos2,
    node_size: egui::Vec2,
    side: Side,
    arrow_index: usize,
    total: usize,
) -> egui::Pos2 {
    match side {
        Side::Top => {
            let fraction = (arrow_index + 1) as f32 / (total as f32 + 1.0);
            egui::pos2(node_pos.x + node_size.x * fraction, node_pos.y)
        }
        Side::Bottom => {
            let fraction = (arrow_index + 1) as f32 / (total as f32 + 1.0);
            egui::pos2(
                node_pos.x + node_size.x * fraction,
                node_pos.y + node_size.y,
            )
        }
        Side::Left => {
            let fraction = (arrow_index + 1) as f32 / (total as f32 + 1.0);
            egui::pos2(node_pos.x, node_pos.y + node_size.y * fraction)
        }
        Side::Right => {
            let fraction = (arrow_index + 1) as f32 / (total as f32 + 1.0);
            egui::pos2(
                node_pos.x + node_size.x,
                node_pos.y + node_size.y * fraction,
            )
        }
    }
}

// Helper function: given the list of connections, determine the index of the current connection
// (i.e. its order among all arrows originating from the same node and side).
fn get_arrow_index(
    connections: &[NodeConnection],
    node_id: usize,
    side: Side,
    current: &NodeConnection,
) -> (usize, usize) {
    let mut count = 0;
    let mut index = 0;
    for conn in connections {
        if conn.start_node_id == node_id && conn.start_side == side {
            if (conn as *const _) == (current as *const _) {
                index = count;
            }
            count += 1;
        }
    }
    (index, count)
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
            let response = ui.interact(
                ui.max_rect(),
                ui.id(),
                if !self.arrow_connection_active {
                    egui::Sense::drag()
                } else {
                    egui::Sense::empty()
                },
            );

            // Grid Drawing
            let spacing = (25.0 * self.zoom).max(1.0);
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

            for connection in &self.connections {
                // Create fallback nodes that live long enough.
                let fallback_note = NoteNode {
                    id: 0,
                    position: egui::pos2(0.0, 0.0),
                    size: egui::vec2(1.0, 1.0),
                    text: String::new(),
                    is_dragging: false,
                    locked: false,
                };
                let fallback_code = CodeNode {
                    id: 0,
                    position: egui::pos2(0.0, 0.0),
                    size: egui::vec2(1.0, 1.0),
                    file_path: String::new(),
                    code: String::new(),
                    is_dragging: false,
                    locked: false,
                    line_offset: None,
                };

                // Get start node's position and size.
                let (start_pos, start_size) = if connection.start_node_type == NodeType::Note {
                    let node = self
                        .note_nodes
                        .iter()
                        .find(|n| n.id == connection.start_node_id)
                        .unwrap_or(&fallback_note);
                    (
                        (node.position * self.zoom) + self.offset,
                        node.size * self.zoom,
                    )
                } else {
                    let node = self
                        .code_nodes
                        .iter()
                        .find(|n| n.id == connection.start_node_id)
                        .unwrap_or(&fallback_code);
                    (
                        (node.position * self.zoom) + self.offset,
                        node.size * self.zoom,
                    )
                };

                // Get end node's position and size.
                let (end_pos, end_size) = if connection.end_node_type == NodeType::Note {
                    let node = self
                        .note_nodes
                        .iter()
                        .find(|n| n.id == connection.end_node_id)
                        .unwrap_or(&fallback_note);
                    (
                        (node.position * self.zoom) + self.offset,
                        node.size * self.zoom,
                    )
                } else {
                    let node = self
                        .code_nodes
                        .iter()
                        .find(|n| n.id == connection.end_node_id)
                        .unwrap_or(&fallback_code);
                    (
                        (node.position * self.zoom) + self.offset,
                        node.size * self.zoom,
                    )
                };

                // Compute distributed connection points.
                let (start_index, total_start) = get_arrow_index(
                    &self.connections,
                    connection.start_node_id,
                    connection.start_side,
                    connection,
                );
                let start_connection_point = connection_point(
                    start_pos,
                    start_size,
                    connection.start_side,
                    start_index,
                    total_start,
                );
                let (end_index, total_end) = get_arrow_index(
                    &self.connections,
                    connection.end_node_id,
                    connection.end_side,
                    connection,
                );
                let end_connection_point =
                    connection_point(end_pos, end_size, connection.end_side, end_index, total_end);

                // Compute control points with perpendicular offsets so the curve bends away from the nodes.
                let d = end_connection_point - start_connection_point;
                let normal_start = side_normal(connection.start_side);
                let normal_end = side_normal(connection.end_side);
                let offset_distance = 50.0; // Adjust for desired curvature.
                let control1 = start_connection_point + d * 0.3 + normal_start * offset_distance;
                let control2 = start_connection_point + d * 0.7 + normal_end * offset_distance;

                let bezier_points = compute_cubic_bezier_points(
                    start_connection_point,
                    control1,
                    control2,
                    end_connection_point,
                    30,
                );
                for window in bezier_points.windows(2) {
                    if let [p1, p2] = window {
                        painter.line_segment([*p1, *p2], egui::Stroke::new(2.0, connection.color));
                    }
                }

                // Draw arrowhead at the end.
                let arrow_head_size = 10.0;
                let last_segment_dir = (end_connection_point - control2).normalized();
                let perp = egui::vec2(-last_segment_dir.y, last_segment_dir.x);
                let arrow_left = end_connection_point - last_segment_dir * arrow_head_size
                    + perp * arrow_head_size * 0.5;
                let arrow_right = end_connection_point
                    - last_segment_dir * arrow_head_size
                    - perp * arrow_head_size * 0.5;
                painter.line_segment(
                    [end_connection_point, arrow_left],
                    egui::Stroke::new(2.0, connection.color),
                );
                painter.line_segment(
                    [end_connection_point, arrow_right],
                    egui::Stroke::new(2.0, connection.color),
                );
            }

            // -------------------- Temporary Arrow (Arrow in Progress) --------------------
            if self.arrow_connection_active {
                if let Some((start_id, start_type, start_side)) = self.connection_start {
                    // Get start node data.
                    let (start_pos, start_size) = if start_type == NodeType::Note {
                        let node = self.note_nodes.iter().find(|n| n.id == start_id).unwrap();
                        (
                            (node.position * self.zoom) + self.offset,
                            node.size * self.zoom,
                        )
                    } else {
                        let node = self.code_nodes.iter().find(|n| n.id == start_id).unwrap();
                        (
                            (node.position * self.zoom) + self.offset,
                            node.size * self.zoom,
                        )
                    };
                    // For temporary arrow, use the center of the selected side.
                    let start_connection_point =
                        connection_point(start_pos, start_size, start_side, 0, 1);
                    if let Some(pointer_pos) = ctx.input(|i| i.pointer.interact_pos()) {
                        let d = pointer_pos - start_connection_point;
                        let normal_start = side_normal(start_side);
                        let offset_distance = 50.0;
                        let control1 =
                            start_connection_point + d * 0.3 + normal_start * offset_distance;
                        let control2 =
                            start_connection_point + d * 0.7 + normal_start * offset_distance;
                        let temp_points = compute_cubic_bezier_points(
                            start_connection_point,
                            control1,
                            control2,
                            pointer_pos,
                            30,
                        );
                        for window in temp_points.windows(2) {
                            if let [p1, p2] = window {
                                painter.line_segment(
                                    [*p1, *p2],
                                    egui::Stroke::new(2.0, egui::Color32::LIGHT_BLUE),
                                );
                            }
                        }
                    }
                }
            }

            // Marker and Eraser Drawing.
            let pointer = ctx.input(|i| i.pointer.clone());
            if self.marker_active {
                if pointer.primary_down() {
                    if let Some(pos) = pointer.interact_pos() {
                        let canvas_pos = (pos - self.offset) / self.zoom;
                        if let Some(stroke) = self.current_stroke.as_mut() {
                            stroke.points.push(canvas_pos);
                        } else {
                            self.current_stroke = Some(Stroke {
                                points: vec![canvas_pos],
                                color: egui::Color32::from_rgb(187, 192, 206),
                                thickness: 2.0,
                            });
                        }
                    }
                } else if let Some(stroke) = self.current_stroke.take() {
                    self.strokes.push(stroke);
                }
            }
            if self.eraser_active {
                if pointer.primary_down() {
                    if let Some(pos) = pointer.interact_pos() {
                        let canvas_pos = (pos - self.offset) / self.zoom;
                        let threshold = 10.0 / self.zoom;
                        for stroke in &mut self.strokes {
                            stroke
                                .points
                                .retain(|&p| p.distance(canvas_pos) >= threshold);
                        }
                        self.strokes.retain(|s| s.points.len() > 1);
                    }
                }
            }

            // Draw Strokes.
            for stroke in &self.strokes {
                for w in stroke.points.windows(2) {
                    if let [a, b] = w {
                        let a = (*a) * self.zoom + self.offset;
                        let b = (*b) * self.zoom + self.offset;
                        painter.line_segment(
                            [a, b],
                            egui::Stroke::new(stroke.thickness * self.zoom, stroke.color),
                        );
                    }
                }
            }
            if let Some(stroke) = &self.current_stroke {
                for w in stroke.points.windows(2) {
                    if let [a, b] = w {
                        let a = (*a) * self.zoom + self.offset;
                        let b = (*b) * self.zoom + self.offset;
                        painter.line_segment(
                            [a, b],
                            egui::Stroke::new(stroke.thickness * self.zoom, stroke.color),
                        );
                    }
                }
            }

            // Arrow Connection Logic.
            if self.arrow_connection_active {
                // Helper function to determine closest side of a node.
                fn determine_closest_side(
                    node_pos: egui::Pos2,
                    node_size: egui::Vec2,
                    point: egui::Pos2,
                ) -> Side {
                    let top = node_pos + egui::vec2(node_size.x / 2.0, 0.0);
                    let bottom = node_pos + egui::vec2(node_size.x / 2.0, node_size.y);
                    let left = node_pos + egui::vec2(0.0, node_size.y / 2.0);
                    let right = node_pos + egui::vec2(node_size.x, node_size.y / 2.0);
                    let sides = [
                        (Side::Top, top),
                        (Side::Bottom, bottom),
                        (Side::Left, left),
                        (Side::Right, right),
                    ];
                    sides
                        .iter()
                        .min_by(|a, b| {
                            a.1.distance(point)
                                .partial_cmp(&b.1.distance(point))
                                .unwrap()
                        })
                        .map(|x| x.0)
                        .unwrap()
                }

                // Connection logic for note nodes.
                for note in &self.note_nodes {
                    let scaled_position = (note.position * self.zoom) + self.offset;
                    let scaled_size = note.size * self.zoom;
                    let rect = egui::Rect::from_min_size(scaled_position, scaled_size);
                    let response =
                        ui.interact(rect, ui.make_persistent_id(note.id), egui::Sense::click());
                    if response.clicked() {
                        if let Some((start_id, start_type, start_side)) = self.connection_start {
                            self.connections.push(NodeConnection {
                                start_node_id: start_id,
                                start_node_type: start_type,
                                start_side,
                                end_node_id: note.id,
                                end_node_type: NodeType::Note,
                                end_side: determine_closest_side(
                                    note.position * self.zoom + self.offset,
                                    note.size * self.zoom,
                                    response.interact_pointer_pos().unwrap(),
                                ),
                                control_points: None,
                                color: egui::Color32::from_rgb(187, 192, 206),
                            });
                            self.connection_start = None;
                        } else {
                            let closest_side = determine_closest_side(
                                note.position * self.zoom + self.offset,
                                note.size * self.zoom,
                                response.interact_pointer_pos().unwrap(),
                            );
                            self.connection_start = Some((note.id, NodeType::Note, closest_side));
                        }
                    }
                }

                // Connection logic for code nodes.
                for node in &self.code_nodes {
                    let scaled_position = (node.position * self.zoom) + self.offset;
                    let scaled_size = node.size * self.zoom;
                    let rect = egui::Rect::from_min_size(scaled_position, scaled_size);
                    let response = ui.interact(
                        rect,
                        ui.make_persistent_id(node.id + 10_000),
                        egui::Sense::click(),
                    );
                    if response.clicked() {
                        if let Some((start_id, start_type, start_side)) = self.connection_start {
                            self.connections.push(NodeConnection {
                                start_node_id: start_id,
                                start_node_type: start_type,
                                start_side,
                                end_node_id: node.id,
                                end_node_type: NodeType::Code,
                                end_side: determine_closest_side(
                                    node.position * self.zoom + self.offset,
                                    node.size * self.zoom,
                                    response.interact_pointer_pos().unwrap(),
                                ),
                                control_points: None,
                                color: egui::Color32::from_rgb(187, 192, 206),
                            });
                            self.connection_start = None;
                        } else {
                            let closest_side = determine_closest_side(
                                node.position * self.zoom + self.offset,
                                node.size * self.zoom,
                                response.interact_pointer_pos().unwrap(),
                            );
                            self.connection_start = Some((node.id, NodeType::Code, closest_side));
                        }
                    }
                }
            }

            // Dragging and Scrolling Logic (disabled when arrow connection is active).
            if !self.arrow_connection_active {
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

            // Zoom Logic.
            let scroll = ctx.input(|i| i.raw_scroll_delta.y);
            if scroll != 0.0 {
                self.zoom *= 1.0 + scroll * 0.001;
                self.zoom = self.zoom.clamp(0.4, 4.0);
            }

            // Note Nodes Rendering.
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

                // In the note node rendering block:
                ui.allocate_ui_at_rect(rect, |ui| {
                    egui::Frame::NONE
                        .fill(egui::Color32::from_rgb(32, 37, 43))
                        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(80, 80, 80)))
                        .show(ui, |ui| {
                            let font_id = egui::FontId::monospace(6.0 * self.zoom);
                            // Options button at the top right.
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                                if ui.button("⋮").on_hover_text("Options").clicked() {
                                    if self.selected_node == Some(i) {
                                        self.selected_node = None;
                                    } else {
                                        self.selected_node = Some(i);
                                    }
                                }
                            });
                            if note.locked {
                                // Render a read-only text area.
                                // Note: interactive(false) disables editing. Depending on egui's version,
                                // this may disable text selection; you may need a custom solution if selection is required.
                                ui.add(
                                    egui::TextEdit::multiline(&mut note.text)
                                        .font(font_id.clone())
                                        .frame(false)
                                        .interactive(false)
                                        .text_color(egui::Color32::from_rgb(187, 192, 206)),
                                );
                            } else {
                                ui.vertical(|ui| {
                                    ui.add_sized(
                                        scaled_size,
                                        egui::TextEdit::multiline(&mut note.text)
                                            .font(font_id.clone())
                                            .frame(false)
                                            .background_color(egui::Color32::from_rgb(32, 37, 43))
                                            .text_color(egui::Color32::from_rgb(187, 192, 206)),
                                    );
                                    // Lock button at the bottom right.
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            if ui
                                                .button("Lock")
                                                .on_hover_text("Lock Note")
                                                .clicked()
                                            {
                                                note.locked = true;
                                            }
                                        },
                                    );
                                });
                            }
                            // Optional: Allow resizing via drag values.
                            ui.add(egui::DragValue::new(&mut note.size.x).range(1.0..=400.0));
                            ui.add(egui::DragValue::new(&mut note.size.y).range(1.0..=400.0));
                        });
                });
                if Some(i) == self.selected_node {
                    let menu_pos = scaled_position + egui::vec2(0.0, -25.0);
                    egui::Area::new(format!("note_menu_{}", note.id).into())
                        .fixed_pos(menu_pos)
                        .show(ctx, |ui| {
                            let mut to_remove = false;
                            ui.horizontal(|ui| {
                                if ui.button("Backward").clicked() && i > 0 {
                                    self.note_nodes.swap(i, i - 1);
                                    self.selected_node = Some(i - 1);
                                }
                                if ui.button("Forward").clicked() && i < self.note_nodes.len() - 1 {
                                    self.note_nodes.swap(i, i + 1);
                                    self.selected_node = Some(i + 1);
                                }
                                if ui.button("Delete").clicked() {
                                    to_remove = true;
                                }
                            });
                            if to_remove {
                                self.note_nodes.remove(i);
                                self.selected_node = None;
                            }
                        });
                }
                i += 1;
            }

            // Code Nodes Rendering using an index loop.
            for i in 0..self.code_nodes.len() {
                {
                    // Inner scope: mutable borrow of self.code_nodes[i].
                    let node = &mut self.code_nodes[i];
                    let scaled_size = (node.size * self.zoom).max(egui::vec2(1.0, 1.0));
                    let scaled_position = (node.position * self.zoom) + self.offset;
                    let rect = egui::Rect::from_min_size(scaled_position, scaled_size);
                    let id = ui.make_persistent_id(node.id + 10_000);
                    let interact = ui.interact(rect, id, egui::Sense::click_and_drag());
                    if interact.drag_started() {
                        node.is_dragging = true;
                    }
                    if interact.drag_stopped() {
                        node.is_dragging = false;
                    }
                    if node.is_dragging {
                        node.position += interact.drag_delta() / self.zoom;
                    }
                    ui.allocate_ui_at_rect(rect, |ui| {
                        egui::Frame::NONE
                            .fill(egui::Color32::from_rgb(30, 35, 40))
                            .stroke(egui::Stroke::new(
                                1.0,
                                egui::Color32::from_rgb(100, 100, 100),
                            ))
                            .show(ui, |ui| {
                                let font_id = egui::FontId::monospace(5.0 * self.zoom);
                                // Compute desired rows based on the fixed height.
                                let row_count = (scaled_size.y / (5.0 * self.zoom)).ceil() as usize;
                                // Options button at top right.
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::TOP),
                                    |ui| {
                                        if ui.button("⋮").on_hover_text("Options").clicked() {
                                            let code_index = i + self.note_nodes.len();
                                            if self.selected_node == Some(code_index) {
                                                self.selected_node = None;
                                            } else {
                                                self.selected_node = Some(code_index);
                                            }
                                        }
                                    },
                                );
                                if node.locked {
                                    // Locked: show file path in a frame and read-only code.
                                    egui::Frame::NONE
                                        .fill(egui::Color32::from_rgb(187, 192, 206))
                                        .show(ui, |ui| {
                                            ui.label(
                                                egui::RichText::new(&node.file_path)
                                                    .font(font_id.clone())
                                                    .color(egui::Color32::BLACK),
                                            );
                                        });
                                    let offset = node.line_offset.unwrap_or(1);
                                    let display_code = node
                                        .code
                                        .lines()
                                        .enumerate()
                                        .map(|(i, line)| format!("{:>4}: {}", i + offset, line))
                                        .collect::<Vec<_>>()
                                        .join("\n");
                                    ui.add_sized(
                                        scaled_size,
                                        egui::TextEdit::multiline(&mut display_code.clone())
                                            .font(font_id.clone())
                                            .frame(false)
                                            .desired_rows(row_count)
                                            .text_color(egui::Color32::from_rgb(187, 192, 206))
                                            .interactive(false),
                                    );
                                } else {
                                    // Unlocked: allow editing of file path and code.
                                    ui.vertical(|ui| {
                                        // File path input area.
                                        egui::Frame::NONE
                                            .fill(egui::Color32::from_rgb(187, 192, 206))
                                            .show(ui, |ui| {
                                                ui.label(
                                                    egui::RichText::new(
                                                        "Enter file path relative to project root:",
                                                    )
                                                    .font(font_id.clone())
                                                    .color(egui::Color32::BLACK),
                                                );
                                            });
                                        ui.add(
                                            egui::TextEdit::singleline(&mut node.file_path)
                                                .font(font_id.clone()),
                                        );

                                        // Reserve an exact area for the code text edit.
                                        // Reserve an exact area for the code text edit.
                                        let (text_edit_rect, _response) = ui
                                            .allocate_exact_size(scaled_size, egui::Sense::hover());
                                        ui.put(text_edit_rect, |ui: &mut egui::Ui| {
                                            ui.add(
                                                egui::TextEdit::multiline(&mut node.code)
                                                    .font(font_id.clone())
                                                    .frame(false)
                                                    .text_color(egui::Color32::from_rgb(
                                                        187, 192, 206,
                                                    )),
                                            )
                                        });

                                        // Lock button at the bottom right.
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                if ui
                                                    .button("Lock")
                                                    .on_hover_text("Lock Code Node")
                                                    .clicked()
                                                {
                                                    node.locked = true;
                                                    if let Some(project_root) = &self.project_root {
                                                        let full_path =
                                                            project_root.join(&node.file_path);
                                                        if let Ok(contents) =
                                                            fs::read_to_string(&full_path)
                                                        {
                                                            let snippet_raw =
                                                                node.code.replace("\r\n", "\n");
                                                            let snippet = snippet_raw.trim_end();
                                                            let file =
                                                                contents.replace("\r\n", "\n");
                                                            node.line_offset = file
                                                                .lines()
                                                                .collect::<Vec<_>>()
                                                                .windows(snippet.lines().count())
                                                                .position(|window| {
                                                                    window.join("\n").trim_end()
                                                                        == snippet
                                                                })
                                                                .map(|i| i + 1);
                                                        }
                                                    }
                                                    // (Additional logic to update node.line_offset if desired.)
                                                }
                                            },
                                        );
                                    }); // Lock button at the bottom right.
                                }
                                // Always allow resizing via drag values.
                                ui.add(egui::DragValue::new(&mut node.size.x).range(1.0..=400.0));
                                ui.add(egui::DragValue::new(&mut node.size.y).range(1.0..=400.0));
                            });
                    });
                } // End inner scope.
                  // Now the mutable borrow is dropped; we can render the floating menu.
                let scaled_position = (self.code_nodes[i].position * self.zoom) + self.offset;
                let code_index = i + self.note_nodes.len();
                if Some(code_index) == self.selected_node {
                    let menu_pos = scaled_position + egui::vec2(0.0, -25.0);
                    egui::Area::new(format!("code_menu_{}", self.code_nodes[i].id).into())
                        .fixed_pos(menu_pos)
                        .show(ctx, |ui| {
                            let mut to_remove = false;
                            ui.horizontal(|ui| {
                                if ui.button("Backward").clicked() && i > 0 {
                                    self.code_nodes.swap(i, i - 1);
                                    self.selected_node = Some(i - 1 + self.note_nodes.len());
                                }
                                if ui.button("Forward").clicked() && i < self.code_nodes.len() - 1 {
                                    self.code_nodes.swap(i, i + 1);
                                    self.selected_node = Some(i + 1 + self.note_nodes.len());
                                }
                                if ui.button("Delete").clicked() {
                                    to_remove = true;
                                }
                            });
                            if to_remove {
                                self.code_nodes.remove(i);
                                self.selected_node = None;
                            }
                        });
                }
            }

            // Zoom and Offset Display.
            painter.text(
                egui::pos2(10.0, 10.0),
                egui::Align2::LEFT_TOP,
                format!("Zoom: {:.2} | Offset: {:?}", self.zoom, self.offset),
                egui::TextStyle::Monospace.resolve(ui.style()),
                ui.visuals().text_color(),
            );

            // Tools Overlay.
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
                        if self.project_root.is_none() {
                            if let Some(path) = rfd::FileDialog::new().pick_folder() {
                                self.project_root = Some(path);
                            }
                            if self.project_root.is_none() {
                                return;
                            }
                        }
                        // Use next_note_id to calculate a stable offset.
                        let pos_offset = self.next_note_id as f32 * 20.0;
                        self.code_nodes.push(CodeNode {
                            id: self.next_note_id,
                            position: egui::pos2(120.0 + pos_offset, 120.0 + pos_offset),
                            size: egui::vec2(300.0, 40.0),
                            file_path: String::new(),
                            code: String::new(),
                            is_dragging: false,
                            locked: false,
                            line_offset: None,
                        });
                        self.next_note_id += 1;
                    }
                    if ui.button("+ Note Node").clicked() {
                        // Use next_note_id to calculate a stable offset.
                        let pos_offset = self.next_note_id as f32 * 20.0;
                        self.note_nodes.push(NoteNode {
                            id: self.next_note_id,
                            position: egui::pos2(100.0 + pos_offset, 100.0 + pos_offset),
                            size: egui::vec2(200.0, 40.0),
                            text: String::new(),
                            is_dragging: false,
                            locked: false,
                        });
                        self.next_note_id += 1;
                    }
                    if ui.button("Marker").clicked() {
                        self.marker_active = !self.marker_active;
                        self.eraser_active = false;
                    }
                    if ui.button("Eraser").clicked() {
                        self.eraser_active = !self.eraser_active;
                        self.marker_active = false;
                    }
                    if ui.button("Arrow").clicked() {
                        self.arrow_connection_active = !self.arrow_connection_active;
                        if !self.arrow_connection_active {
                            self.connection_start = None;
                        }
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
