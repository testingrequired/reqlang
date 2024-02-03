use std::fs;

use eframe::{run_native, App};
use parser::parse;
use types::UnresolvedRequestFile;

struct Client {
    current_path: Option<String>,
    current_source: Option<String>,
    current_reqfile: Option<UnresolvedRequestFile>,
}

impl Client {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        #[allow(unused_mut)]
        let mut slf = Self {
            current_path: None,
            current_source: None,
            current_reqfile: None,
        };

        slf
    }
}

impl App for Client {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Open file…").clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_file() {
                        let path = path.display().to_string();

                        let source = fs::read_to_string(&path)
                            .expect("Should have been able to read the file");

                        self.current_reqfile = Some(parse(&source).unwrap());

                        self.current_source = Some(
                            fs::read_to_string(&path)
                                .expect("Should have been able to read the file"),
                        );

                        self.current_path = Some(path);
                    }
                }

                if ui.button("Clear").clicked() {
                    self.current_path = None;
                    self.current_source = None;
                    self.current_reqfile = None;
                }
            });

            if let Some(picked_path) = &self.current_path {
                ui.horizontal(|ui| {
                    ui.label("Picked file:");
                    ui.monospace(picked_path);
                });

                ui.separator();
            }

            if let Some(reqfile) = &self.current_reqfile {
                eframe::egui::ScrollArea::vertical()
                    .auto_shrink(false)
                    .show(ui, |ui| {
                        ui.code(format!("{:#?}", reqfile));
                    });
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
