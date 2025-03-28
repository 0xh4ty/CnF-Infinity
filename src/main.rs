use eframe::{egui, App, Frame};
use serde::{Deserialize, Serialize};
use std::fs;
use std::fs::File;
use std::io::{self, Write};

mod ser_de {
    use egui::{Color32, Pos2, Vec2};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    // Serialize a Color32 as (r, g, b, a)
    pub fn serialize_color<S>(color: &Color32, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let tup = (color.r(), color.g(), color.b(), color.a());
        tup.serialize(serializer)
    }

    pub fn deserialize_color<'de, D>(deserializer: D) -> Result<Color32, D::Error>
    where
        D: Deserializer<'de>,
    {
        let (r, g, b, a) = <(u8, u8, u8, u8)>::deserialize(deserializer)?;
        Ok(Color32::from_rgba_premultiplied(r, g, b, a))
    }

    // Serialize a Pos2 as (x, y)
    pub fn serialize_pos2<S>(pos: &Pos2, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let tup = (pos.x, pos.y);
        tup.serialize(serializer)
    }

    pub fn deserialize_pos2<'de, D>(deserializer: D) -> Result<Pos2, D::Error>
    where
        D: Deserializer<'de>,
    {
        let (x, y) = <(f32, f32)>::deserialize(deserializer)?;
        Ok(Pos2::new(x, y))
    }

    // Serialize a Vec2 as (x, y)
    pub fn serialize_vec2<S>(vec: &Vec2, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let tup = (vec.x, vec.y);
        tup.serialize(serializer)
    }

    pub fn deserialize_vec2<'de, D>(deserializer: D) -> Result<Vec2, D::Error>
    where
        D: Deserializer<'de>,
    {
        let (x, y) = <(f32, f32)>::deserialize(deserializer)?;
        Ok(Vec2::new(x, y))
    }

    // Serialize a Vec<Pos2> as a Vec of (x, y) tuples.
    pub fn serialize_pos2_vec<S>(vec: &Vec<Pos2>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let tuples: Vec<(f32, f32)> = vec.iter().map(|p| (p.x, p.y)).collect();
        tuples.serialize(serializer)
    }

    pub fn deserialize_pos2_vec<'de, D>(deserializer: D) -> Result<Vec<Pos2>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let tuples: Vec<(f32, f32)> = Vec::deserialize(deserializer)?;
        Ok(tuples.into_iter().map(|(x, y)| Pos2::new(x, y)).collect())
    }

    // Serialize Option<(Pos2, Pos2)> as an option of two (x, y) tuples.
    pub fn serialize_pos2_tuple<S>(
        tuple: &Option<(Pos2, Pos2)>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if let Some((p1, p2)) = tuple {
            let tup = ((p1.x, p1.y), (p2.x, p2.y));
            tup.serialize(serializer)
        } else {
            serializer.serialize_none()
        }
    }

    pub fn deserialize_pos2_tuple<'de, D>(deserializer: D) -> Result<Option<(Pos2, Pos2)>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let opt: Option<((f32, f32), (f32, f32))> = Option::deserialize(deserializer)?;
        Ok(opt.map(|((x1, y1), (x2, y2))| (Pos2::new(x1, y1), Pos2::new(x2, y2))))
    }
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
enum NodeType {
    Note,
    Code,
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
enum Side {
    Top,
    Bottom,
    Left,
    Right,
}

#[derive(Clone, Serialize, Deserialize)]
struct NodeConnection {
    start_node_id: usize,
    start_node_type: NodeType,
    start_side: Side,
    end_node_id: usize,
    end_node_type: NodeType,
    end_side: Side,
    #[serde(
        serialize_with = "ser_de::serialize_pos2_tuple",
        deserialize_with = "ser_de::deserialize_pos2_tuple"
    )]
    control_points: Option<(egui::Pos2, egui::Pos2)>,
    #[serde(
        serialize_with = "ser_de::serialize_color",
        deserialize_with = "ser_de::deserialize_color"
    )]
    color: egui::Color32,
}

#[derive(Clone, Serialize, Deserialize)]
struct NoteNode {
    id: usize,
    #[serde(
        serialize_with = "ser_de::serialize_pos2",
        deserialize_with = "ser_de::deserialize_pos2"
    )]
    position: egui::Pos2,
    #[serde(
        serialize_with = "ser_de::serialize_vec2",
        deserialize_with = "ser_de::deserialize_vec2"
    )]
    size: egui::Vec2,
    text: String,
    is_dragging: bool,
    locked: bool,
}

#[derive(Clone, Serialize, Deserialize)]
struct CodeNode {
    id: usize,
    #[serde(
        serialize_with = "ser_de::serialize_pos2",
        deserialize_with = "ser_de::deserialize_pos2"
    )]
    position: egui::Pos2,
    #[serde(
        serialize_with = "ser_de::serialize_vec2",
        deserialize_with = "ser_de::deserialize_vec2"
    )]
    size: egui::Vec2,
    file_path: String,
    code: String,
    is_dragging: bool,
    locked: bool,
    line_offset: Option<usize>,
}

#[derive(Clone, Serialize, Deserialize)]
struct Stroke {
    #[serde(
        serialize_with = "ser_de::serialize_pos2_vec",
        deserialize_with = "ser_de::deserialize_pos2_vec"
    )]
    points: Vec<egui::Pos2>,
    #[serde(
        serialize_with = "ser_de::serialize_color",
        deserialize_with = "ser_de::deserialize_color"
    )]
    color: egui::Color32,
    thickness: f32,
}

#[derive(Clone, Serialize, Deserialize)]
struct ProjectSnapshot {
    note_nodes: Vec<NoteNode>,
    code_nodes: Vec<CodeNode>,
    connections: Vec<NodeConnection>,
    strokes: Vec<Stroke>,
    zoom: f32,
    #[serde(
        serialize_with = "ser_de::serialize_vec2",
        deserialize_with = "ser_de::deserialize_vec2"
    )]
    offset: egui::Vec2,
}

#[derive(Serialize, Deserialize)]
struct ProjectHistory {
    undo_stack: Vec<ProjectSnapshot>,
    redo_stack: Vec<ProjectSnapshot>,
    current: ProjectSnapshot,
}

struct MyApp {
    zoom: f32,
    offset: egui::Vec2,
    dragging: bool,
    drag_start: egui::Pos2,
    tools_open: bool,
    next_note_id: usize,
    note_nodes: Vec<NoteNode>,
    code_nodes: Vec<CodeNode>,
    connections: Vec<NodeConnection>,
    marker_active: bool,
    eraser_active: bool,
    current_stroke: Option<Stroke>,
    strokes: Vec<Stroke>,
    project_root: Option<std::path::PathBuf>,
    // Connection-related fields
    arrow_connection_active: bool,
    connection_start: Option<(usize, NodeType, Side)>,
    // Undo/Redo stacks
    undo_stack: Vec<ProjectSnapshot>,
    redo_stack: Vec<ProjectSnapshot>,
    // Node selection (for floating menus)
    selected_node: Option<usize>,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            zoom: 2.0,
            offset: egui::Vec2::ZERO,
            dragging: false,
            drag_start: egui::Pos2::ZERO,
            tools_open: false,
            next_note_id: 1,
            note_nodes: Vec::new(),
            code_nodes: Vec::new(),
            connections: Vec::new(),
            marker_active: false,
            eraser_active: false,
            current_stroke: None,
            strokes: Vec::new(),
            project_root: None,
            arrow_connection_active: false,
            connection_start: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            selected_node: None,
        }
    }
}

impl MyApp {
    // Save entire project history (if desired)
    fn save_project(&self, file_path: &str) -> io::Result<()> {
        let history = self.project_history();
        let json = serde_json::to_string_pretty(&history)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        let mut file = File::create(file_path)?;
        file.write_all(json.as_bytes())?;
        Ok(())
    }

    // Load project history and restore state.
    fn load_project(&mut self, file_path: &str) -> io::Result<()> {
        let json = std::fs::read_to_string(file_path)?;
        let history: ProjectHistory =
            serde_json::from_str(&json).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        self.undo_stack = history.undo_stack;
        self.redo_stack = history.redo_stack;
        self.restore_snapshot(history.current);
        Ok(())
    }
    fn project_history(&self) -> ProjectHistory {
        ProjectHistory {
            undo_stack: self.undo_stack.clone(),
            redo_stack: self.redo_stack.clone(),
            current: self.take_snapshot(),
        }
    }
    fn take_snapshot(&self) -> ProjectSnapshot {
        ProjectSnapshot {
            note_nodes: self.note_nodes.clone(),
            code_nodes: self.code_nodes.clone(),
            connections: self.connections.clone(),
            strokes: self.strokes.clone(),
            zoom: self.zoom,
            offset: self.offset,
        }
    }

    fn restore_snapshot(&mut self, snapshot: ProjectSnapshot) {
        self.note_nodes = snapshot.note_nodes;
        self.code_nodes = snapshot.code_nodes;
        self.connections = snapshot.connections;
        self.strokes = snapshot.strokes;
        self.zoom = snapshot.zoom;
        self.offset = snapshot.offset;
    }

    fn record_state(&mut self) {
        self.undo_stack.push(self.take_snapshot());
        self.redo_stack.clear();
    }

    fn undo(&mut self) {
        if let Some(snapshot) = self.undo_stack.pop() {
            self.redo_stack.push(self.take_snapshot());
            self.restore_snapshot(snapshot);
        }
    }

    fn redo(&mut self) {
        if let Some(snapshot) = self.redo_stack.pop() {
            self.undo_stack.push(self.take_snapshot());
            self.restore_snapshot(snapshot);
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
            if std::ptr::eq(conn, current) {
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
        // Canvas View
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

            // Render Connections (same as before).
            for connection in &self.connections {
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

                let (start_pos, start_size) = if connection.start_node_type == NodeType::Note {
                    let node = self
                        .note_nodes
                        .iter()
                        .find(|n| n.id == connection.start_node_id)
                        .unwrap_or(&fallback_note);
                    (
                        ((node.position * self.zoom) + self.offset),
                        node.size * self.zoom,
                    )
                } else {
                    let node = self
                        .code_nodes
                        .iter()
                        .find(|n| n.id == connection.start_node_id)
                        .unwrap_or(&fallback_code);
                    (
                        ((node.position * self.zoom) + self.offset),
                        node.size * self.zoom,
                    )
                };

                let (end_pos, end_size) = if connection.end_node_type == NodeType::Note {
                    let node = self
                        .note_nodes
                        .iter()
                        .find(|n| n.id == connection.end_node_id)
                        .unwrap_or(&fallback_note);
                    (
                        ((node.position * self.zoom) + self.offset),
                        node.size * self.zoom,
                    )
                } else {
                    let node = self
                        .code_nodes
                        .iter()
                        .find(|n| n.id == connection.end_node_id)
                        .unwrap_or(&fallback_code);
                    (
                        ((node.position * self.zoom) + self.offset),
                        node.size * self.zoom,
                    )
                };

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

                let d = end_connection_point - start_connection_point;
                let normal_start = side_normal(connection.start_side);
                let normal_end = side_normal(connection.end_side);
                let offset_distance = 50.0;
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

            // Temporary Arrow (in progress)
            if self.arrow_connection_active {
                if let Some((start_id, start_type, start_side)) = self.connection_start {
                    let (start_pos, start_size) = if start_type == NodeType::Note {
                        let node = self.note_nodes.iter().find(|n| n.id == start_id).unwrap();
                        (
                            ((node.position * self.zoom) + self.offset),
                            node.size * self.zoom,
                        )
                    } else {
                        let node = self.code_nodes.iter().find(|n| n.id == start_id).unwrap();
                        (
                            ((node.position * self.zoom) + self.offset),
                            node.size * self.zoom,
                        )
                    };
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

            // Use flags to record only once after the operation.
            static mut MARKER_STATE_RECORDED: bool = false;
            static mut ERASER_STATE_RECORDED: bool = false;

            if self.marker_active {
                if pointer.primary_down() {
                    // Reset the flag while drawing.
                    unsafe {
                        MARKER_STATE_RECORDED = false;
                    }
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
                    // Only record state once when the pointer is released.
                    unsafe {
                        if !MARKER_STATE_RECORDED {
                            self.record_state();
                            MARKER_STATE_RECORDED = true;
                        }
                    }
                }
            }

            if self.eraser_active {
                if pointer.primary_down() {
                    // Reset the flag while erasing.
                    unsafe {
                        ERASER_STATE_RECORDED = false;
                    }
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
                } else {
                    // When pointer is released, record state if it hasn't been recorded yet.
                    unsafe {
                        if !ERASER_STATE_RECORDED {
                            self.record_state();
                            ERASER_STATE_RECORDED = true;
                        }
                    }
                }
            }

            // Draw Strokes.
            for stroke in &self.strokes {
                for window in stroke.points.windows(2) {
                    if let [a, b] = window {
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
                for window in stroke.points.windows(2) {
                    if let [a, b] = window {
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
                    let left = node_pos.x;
                    let right = node_pos.x + node_size.x;
                    let top = node_pos.y;
                    let bottom = node_pos.y + node_size.y;

                    // Compute the absolute distances from the point to each side.
                    let dist_top = (point.y - top).abs();
                    let dist_bottom = (point.y - bottom).abs();
                    let dist_left = (point.x - left).abs();
                    let dist_right = (point.x - right).abs();

                    // Choose the side with the smallest distance.
                    if dist_top <= dist_bottom && dist_top <= dist_left && dist_top <= dist_right {
                        Side::Top
                    } else if dist_bottom <= dist_top
                        && dist_bottom <= dist_left
                        && dist_bottom <= dist_right
                    {
                        Side::Bottom
                    } else if dist_left <= dist_top
                        && dist_left <= dist_bottom
                        && dist_left <= dist_right
                    {
                        Side::Left
                    } else {
                        Side::Right
                    }
                }

                // Connection logic for note nodes.
                for i in 0..self.note_nodes.len() {
                    let note = &self.note_nodes[i]; // immutable borrow
                    let scaled_position = (note.position * self.zoom) + self.offset;
                    let scaled_size = note.size * self.zoom;
                    let rect = egui::Rect::from_min_size(scaled_position, scaled_size);
                    let response =
                        ui.interact(rect, ui.make_persistent_id(note.id), egui::Sense::click());
                    if response.clicked() {
                        // Capture local values.
                        let pointer_pos = response.interact_pointer_pos().unwrap();
                        if let Some((start_id, start_type, start_side)) = self.connection_start {
                            let end_side =
                                determine_closest_side(scaled_position, scaled_size, pointer_pos);
                            self.connections.push(NodeConnection {
                                start_node_id: start_id,
                                start_node_type: start_type,
                                start_side,
                                end_node_id: note.id,
                                end_node_type: NodeType::Note,
                                end_side,
                                control_points: None,
                                color: egui::Color32::from_rgb(187, 192, 206),
                            });
                            self.connection_start = None;
                            self.record_state(); // Record state after creating a connection.
                        } else {
                            let closest_side =
                                determine_closest_side(scaled_position, scaled_size, pointer_pos);
                            self.connection_start = Some((note.id, NodeType::Note, closest_side));
                        }
                    }
                }
                // Connection logic for code nodes.
                for i in 0..self.code_nodes.len() {
                    let node = &self.code_nodes[i]; // immutable borrow
                    let scaled_position = (node.position * self.zoom) + self.offset;
                    let scaled_size = node.size * self.zoom;
                    let rect = egui::Rect::from_min_size(scaled_position, scaled_size);
                    let response = ui.interact(
                        rect,
                        ui.make_persistent_id(node.id + 10_000),
                        egui::Sense::click(),
                    );
                    if response.clicked() {
                        let pointer_pos = response.interact_pointer_pos().unwrap();
                        if let Some((start_id, start_type, start_side)) = self.connection_start {
                            let end_side =
                                determine_closest_side(scaled_position, scaled_size, pointer_pos);
                            self.connections.push(NodeConnection {
                                start_node_id: start_id,
                                start_node_type: start_type,
                                start_side,
                                end_node_id: node.id,
                                end_node_type: NodeType::Code,
                                end_side,
                                control_points: None,
                                color: egui::Color32::from_rgb(187, 192, 206),
                            });
                            self.connection_start = None;
                            self.record_state(); // Record state after connection creation.
                        } else {
                            let closest_side =
                                determine_closest_side(scaled_position, scaled_size, pointer_pos);
                            self.connection_start = Some((node.id, NodeType::Code, closest_side));
                        }
                    }
                }
            }

            // Dragging and Scrolling Logic (disabled when arrow connection is active).
            if !self.marker_active && !self.eraser_active && !self.arrow_connection_active {
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
                // Extract local copies before mutable borrow.
                let note_id = self.note_nodes[i].id;
                let scaled_size = (self.note_nodes[i].size * self.zoom).max(egui::vec2(1.0, 1.0));
                let scaled_position = (self.note_nodes[i].position * self.zoom) + self.offset;
                let rect = egui::Rect::from_min_size(scaled_position, scaled_size);

                // Local flags to track state changes.
                let mut lock_changed = false;
                let mut drag_ended = false;

                {
                    // Inner block: mutable borrow of self.note_nodes[i].
                    let note = &mut self.note_nodes[i];
                    let id = ui.make_persistent_id(note.id);
                    let interact = ui.interact(rect, id, egui::Sense::click_and_drag());
                    if interact.drag_started() {
                        note.is_dragging = true;
                    }
                    if interact.drag_stopped() {
                        note.is_dragging = false;
                        drag_ended = true;
                    }
                    if note.is_dragging {
                        note.position += interact.drag_delta() / self.zoom;
                    }
                    ui.allocate_ui_at_rect(rect, |ui| {
                        egui::Frame::NONE
                            .fill(egui::Color32::from_rgb(32, 37, 43))
                            .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(80, 80, 80)))
                            .show(ui, |ui| {
                                let font_id = egui::FontId::monospace(6.0 * self.zoom);
                                // Options button at the top right.
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::TOP),
                                    |ui| {
                                        if ui.button("o").on_hover_text("Options").clicked() {
                                            if self.selected_node == Some(i) {
                                                self.selected_node = None;
                                            } else {
                                                self.selected_node = Some(i);
                                            }
                                        }
                                    },
                                );
                                if note.locked {
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
                                                .background_color(egui::Color32::from_rgb(
                                                    32, 37, 43,
                                                ))
                                                .text_color(egui::Color32::from_rgb(187, 192, 206)),
                                        );
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                if ui
                                                    .button("Lock")
                                                    .on_hover_text("Lock Note")
                                                    .clicked()
                                                {
                                                    note.locked = true;
                                                    lock_changed = true;
                                                }
                                            },
                                        );
                                    });
                                }
                                ui.add(egui::DragValue::new(&mut note.size.x).range(1.0..=400.0));
                                ui.add(egui::DragValue::new(&mut note.size.y).range(1.0..=400.0));
                            });
                    });
                } // End inner block: mutable borrow of self.note_nodes[i] is dropped.

                // If a drag ended or the node was locked, record state.
                if drag_ended || lock_changed {
                    self.record_state();
                }
                // Render floating menu using local copies.
                if Some(i) == self.selected_node {
                    let menu_pos = scaled_position + egui::vec2(0.0, -25.0);
                    egui::Area::new(format!("note_menu_{}", note_id).into())
                        .fixed_pos(menu_pos)
                        .show(ctx, |ui| {
                            let mut to_remove = false;
                            ui.horizontal(|ui| {
                                if ui.button("Backward").clicked() && i > 0 {
                                    self.record_state();
                                    self.note_nodes.swap(i, i - 1);
                                    self.selected_node = Some(i - 1);
                                }
                                if ui.button("Forward").clicked() && i < self.note_nodes.len() - 1 {
                                    self.record_state();
                                    self.note_nodes.swap(i, i + 1);
                                    self.selected_node = Some(i + 1);
                                }
                                if ui.button("Delete").clicked() {
                                    to_remove = true;
                                }
                            });
                            if to_remove {
                                self.record_state();
                                self.note_nodes.remove(i);
                                self.selected_node = None;
                            }
                        });
                }
                i += 1;
            }

            // Code Nodes Rendering using an index loop.
            for i in 0..self.code_nodes.len() {
                // Extract local copies before mutable borrow.
                let node_id = self.code_nodes[i].id;
                let scaled_size = (self.code_nodes[i].size * self.zoom).max(egui::vec2(1.0, 1.0));
                let scaled_position = (self.code_nodes[i].position * self.zoom) + self.offset;
                let rect = egui::Rect::from_min_size(scaled_position, scaled_size);
                // Flags to track changes.
                let mut lock_changed = false;
                let mut drag_ended = false;

                {
                    // Inner block: mutable borrow of self.code_nodes[i].
                    let node = &mut self.code_nodes[i];
                    let id = ui.make_persistent_id(node.id + 10_000);
                    let interact = ui.interact(rect, id, egui::Sense::click_and_drag());
                    if interact.drag_started() {
                        node.is_dragging = true;
                    }
                    if interact.drag_stopped() {
                        node.is_dragging = false;
                        drag_ended = true;
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
                                let row_count = (scaled_size.y / (5.0 * self.zoom)).ceil() as usize;
                                // Options button at top right.
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::TOP),
                                    |ui| {
                                        if ui.button("o").on_hover_text("Options").clicked() {
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
                                    // Locked state: show file path in a frame and a read-only code text edit.
                                    egui::Frame::NONE
                                        .fill(egui::Color32::from_rgb(187, 192, 206))
                                        .show(ui, |ui| {
                                            ui.label(
                                                egui::RichText::new(&node.file_path)
                                                    .font(font_id.clone())
                                                    .color(egui::Color32::BLACK),
                                            );
                                        });
                                    let offset_val = node.line_offset.unwrap_or(1);
                                    let display_code = node
                                        .code
                                        .lines()
                                        .enumerate()
                                        .map(|(i, line)| format!("{:>4}: {}", i + offset_val, line))
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
                                    // Unlocked state: allow editing.
                                    ui.vertical(|ui| {
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
                                        let (text_edit_rect, _resp) = ui
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
                                                    lock_changed = true;
                                                }
                                            },
                                        );
                                    });
                                }
                                ui.add(egui::DragValue::new(&mut node.size.x).range(1.0..=400.0));
                                ui.add(egui::DragValue::new(&mut node.size.y).range(1.0..=400.0));
                            });
                    });
                } // End inner block; mutable borrow of self.code_nodes[i] is dropped.

                // If dragging ended or the node was locked, record state.
                if drag_ended || lock_changed {
                    self.record_state();
                }
                // Render floating menu using the local copy of the scaled position.
                if Some(i + self.note_nodes.len()) == self.selected_node {
                    let menu_pos = scaled_position + egui::vec2(0.0, -25.0);
                    egui::Area::new(format!("code_menu_{}", node_id).into())
                        .fixed_pos(menu_pos)
                        .show(ctx, |ui| {
                            let mut to_remove = false;
                            ui.horizontal(|ui| {
                                if ui.button("Backward").clicked() && i > 0 {
                                    self.record_state();
                                    self.code_nodes.swap(i, i - 1);
                                    self.selected_node = Some(i - 1 + self.note_nodes.len());
                                }
                                if ui.button("Forward").clicked() && i < self.code_nodes.len() - 1 {
                                    self.record_state();
                                    self.code_nodes.swap(i, i + 1);
                                    self.selected_node = Some(i + 1 + self.note_nodes.len());
                                }
                                if ui.button("Delete").clicked() {
                                    to_remove = true;
                                }
                            });
                            if to_remove {
                                self.record_state();
                                self.code_nodes.remove(i);
                                self.selected_node = None;
                            }
                        });
                }
            }

            // Zoom and Offset Display.
            painter.text(
                egui::pos2(40.0, 10.0),
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
                                if ui.button("New").clicked() {
                                    // Clear any previous state.
                                    self.note_nodes.clear();
                                    self.code_nodes.clear();
                                    self.connections.clear();
                                    self.strokes.clear();
                                    self.marker_active = false;
                                    self.eraser_active = false;
                                    self.arrow_connection_active = false;
                                    self.connection_start = None;
                                    self.selected_node = None;
                                    self.zoom = 2.0;
                                    self.offset = egui::Vec2::ZERO;
                                    self.undo_stack.clear();
                                    self.redo_stack.clear();
                                    self.record_state();
                                }
                                if ui.button("Open").clicked() {
                                    if let Some(path) = rfd::FileDialog::new().pick_file() {
                                        if let Err(e) = self.load_project(path.to_str().unwrap()) {
                                            eprintln!("Load error: {}", e);
                                        }
                                    }
                                }
                                if ui.button("Undo").clicked() {
                                    self.undo();
                                }
                                if ui.button("Redo").clicked() {
                                    self.redo();
                                }
                                if ui.button("Code Node").clicked() {
                                    if self.project_root.is_none() {
                                        if let Some(path) = rfd::FileDialog::new().pick_folder() {
                                            self.project_root = Some(path);
                                        }
                                        if self.project_root.is_none() {
                                            return;
                                        }
                                    }
                                    // Get the center of the visible area (screen coordinates)
                                    let visible_center = ctx.input(|i| i.screen_rect().center());
                                    // Convert visible center to canvas (logical) coordinates.
                                    let canvas_center = (visible_center - self.offset) / self.zoom;
                                    // Use next_note_id (or self.code_nodes.len()) to compute an angle.
                                    let angle = (self.next_note_id as f32) * 45.0_f32.to_radians();
                                    // Choose a radius in canvas coordinates.
                                    let radius = 100.0 / self.zoom;
                                    // Compute new node position relative to the canvas center.
                                    let new_pos = egui::pos2(
                                        canvas_center.x + radius * angle.cos(),
                                        canvas_center.y + radius * angle.sin(),
                                    );
                                    self.code_nodes.push(CodeNode {
                                        id: self.next_note_id,
                                        position: new_pos,
                                        size: egui::vec2(300.0, 40.0),
                                        file_path: String::new(),
                                        code: String::new(),
                                        is_dragging: false,
                                        locked: false,
                                        line_offset: None,
                                    });
                                    self.record_state();
                                    self.next_note_id += 1;
                                }
                                if ui.button("Note Node").clicked() {
                                    // Get the center of the visible area (in screen coordinates).
                                    let visible_center = ctx.input(|i| i.screen_rect().center());
                                    // Convert to canvas coordinates.
                                    let canvas_center = (visible_center - self.offset) / self.zoom;
                                    // Use the current count of note nodes to compute an angle.
                                    let angle =
                                        (self.note_nodes.len() as f32) * 45.0_f32.to_radians();
                                    // Choose a radius (in canvas coordinates). Adjust as needed.
                                    let radius = 100.0 / self.zoom;
                                    // Compute the new node position.
                                    let new_pos = egui::pos2(
                                        canvas_center.x + radius * angle.cos(),
                                        canvas_center.y + radius * angle.sin(),
                                    );
                                    self.note_nodes.push(NoteNode {
                                        id: self.next_note_id,
                                        position: new_pos,
                                        size: egui::vec2(200.0, 40.0),
                                        text: String::new(),
                                        is_dragging: false,
                                        locked: false,
                                    });
                                    self.record_state();
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
                                if ui.button("Save Project").clicked() {
                                    if let Some(path) = rfd::FileDialog::new().save_file() {
                                        if let Err(e) = self.save_project(path.to_str().unwrap()) {
                                            eprintln!("Save error: {}", e);
                                        }
                                    }
                                }
                            }
                        });
                    });
                });
        });
    }
}
