use eframe::{egui, App, Frame};

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "CnF-Infinity",
        options,
        Box::new(|_cc| Ok(Box::new(MyApp::default()))),
    )
}

struct MyApp {
    zoom: f32,
    offset: egui::Vec2,
    dragging: bool,
    drag_start: egui::Pos2,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            zoom: 0.5,
            offset: egui::Vec2::ZERO,
            dragging: false,
            drag_start: egui::Pos2::ZERO,
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
            let painter = ui.painter();
            let response = ui.interact(ui.max_rect(), ui.id(), egui::Sense::drag());

            let spacing = (50.0 * self.zoom).max(1.0);
            let grid_color = egui::Color32::from_gray(60);
            let stroke = egui::Stroke::new(1.0, grid_color);

            let bounds = ui.clip_rect();
            let top_left = bounds.left_top() - self.offset;
            let bottom_right = bounds.right_bottom() - self.offset;

            let start_x = (top_left.x / spacing).floor() * spacing;
            let end_x = (bottom_right.x / spacing).ceil() * spacing;
            let start_y = (top_left.y / spacing).floor() * spacing;
            let end_y = (bottom_right.y / spacing).ceil() * spacing;

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

            let scroll = ctx.input(|i| i.raw_scroll_delta.y);
            if scroll != 0.0 {
                self.zoom *= 1.0 + scroll * 0.001;
                self.zoom = self.zoom.clamp(0.1, 4.0);
            }

            painter.text(
                egui::pos2(10.0, 10.0),
                egui::Align2::LEFT_TOP,
                format!("Zoom: {:.2} | Offset: {:?}", self.zoom, self.offset),
                egui::TextStyle::Monospace.resolve(ui.style()),
                ui.visuals().text_color(),
            );
        });
    }
}
