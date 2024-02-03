use eframe::{run_native, App};

struct Client {
    picked_path: Option<String>,
}

impl Client {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        #[allow(unused_mut)]
        let mut slf = Self { picked_path: None };

        slf
    }
}

impl App for Client {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            if ui.button("Open file…").clicked() {
                if let Some(path) = rfd::FileDialog::new().pick_file() {
                    self.picked_path = Some(path.display().to_string());
                }
            }

            if let Some(picked_path) = &self.picked_path {
                ui.horizontal(|ui| {
                    ui.label("Picked file:");
                    ui.monospace(picked_path);
                });
            }
        });
    }
}

fn main() {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([500.0, 281.0])
            .with_drag_and_drop(true),

        #[cfg(feature = "wgpu")]
        renderer: eframe::Renderer::Wgpu,

        ..Default::default()
    };

    let _ = run_native(
        "Reqlang Client",
        options,
        Box::new(|cc| Box::new(Client::new(cc))),
    );
}
