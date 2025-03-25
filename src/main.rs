use eframe::{egui, App, Frame};

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "CnF-Infinity",
        options,
        Box::new(|_cc| Ok(Box::new(MyApp::default()))),
    )
}

#[derive(Default)]
struct MyApp {
    zoom: f32,
    offset: egui::Vec2,
    dragging: bool,
    drag_start: egui::Pos2,
}

impl App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let painter = ui.painter();
            let response = ui.interact(ui.max_rect(), ui.id(), egui::Sense::drag());

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
