use std::fs;

use eframe::{run_native, App};

struct Client {
    current_path: Option<String>,
    current_source: Option<String>,
}

impl Client {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        #[allow(unused_mut)]
        let mut slf = Self {
            current_path: None,
            current_source: None,
        };

        slf
    }
}

impl App for Client {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            if ui.button("Open file…").clicked() {
                if let Some(path) = rfd::FileDialog::new().pick_file() {
                    let path = path.display().to_string();

                    self.current_source = Some(
                        fs::read_to_string(&path).expect("Should have been able to read the file"),
                    );

                    self.current_path = Some(path);
                }
            }

            if ui.button("Clear").clicked() {
                self.current_path = None;
                self.current_source = None;
            }

            if let Some(picked_path) = &self.current_path {
                ui.horizontal(|ui| {
                    ui.label("Picked file:");
                    ui.monospace(picked_path);
                });
            }

            if let Some(current_source) = &self.current_source {
                ui.code(current_source);
            }
        });
    }
}

fn main() {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
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
