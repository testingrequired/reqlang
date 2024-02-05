use std::{
    ops::ControlFlow,
    str::FromStr,
    sync::{Arc, Mutex},
};

use eframe::egui;
use types::{ResolvedRequestFile, UnresolvedRequestFile};

#[allow(dead_code)]
enum ClientState {
    Init,
    View(ViewState),
    Edit(EditState),
    Resolving(ResolvingState),
    Resolved(ResolvedState),
    RequestResponse(RequestResponseState),
    Demo(DemoState),
}

struct DemoState {
    url: String,
    method: Method,
    request_body: String,
    download: Arc<Mutex<Download>>,
    streaming: bool,
}

impl Default for DemoState {
    fn default() -> Self {
        Self {
            url: "https://raw.githubusercontent.com/emilk/ehttp/master/README.md".to_owned(),
            method: Method::Get,
            request_body: r#"["posting some json"]"#.to_owned(),
            download: Arc::new(Mutex::new(Download::None)),
            streaming: true,
        }
    }
}

impl DemoState {
    pub fn ui(&mut self, egui_ctx: &egui::Context) -> Result<(), &str> {
        egui::CentralPanel::default().show(egui_ctx, |ui| {
            let trigger_fetch = self.ui_url(ui);

            if trigger_fetch {
                let request = match self.method {
                    Method::Get => ehttp::Request::get(&self.url),
                    Method::Head => ehttp::Request::head(&self.url),
                    Method::Post => {
                        ehttp::Request::post(&self.url, self.request_body.as_bytes().to_vec())
                    }
                };
                let download_store = self.download.clone();
                *download_store.lock().unwrap() = Download::InProgress;
                let egui_ctx = egui_ctx.clone();

                if self.streaming {
                    // The more complicated streaming API:
                    ehttp::streaming::fetch(request, move |part| {
                        egui_ctx.request_repaint(); // Wake up UI thread
                        on_fetch_part(part, &mut download_store.lock().unwrap())
                    });
                } else {
                    // The simple non-streaming API:
                    ehttp::fetch(request, move |response| {
                        *download_store.lock().unwrap() = Download::Done(response);
                        egui_ctx.request_repaint(); // Wake up UI thread
                    });
                }
            }

            ui.separator();

            let download: &Download = &self.download.lock().unwrap();
            match download {
                Download::None => {}
                Download::InProgress => {
                    ui.label("Wait for it…");
                }
                Download::StreamingInProgress { body, .. } => {
                    let num_bytes = body.len();
                    if num_bytes < 1_000_000 {
                        ui.label(format!("{:.1} kB", num_bytes as f32 / 1e3));
                    } else {
                        ui.label(format!("{:.1} MB", num_bytes as f32 / 1e6));
                    }
                }
                Download::Done(response) => match response {
                    Err(err) => {
                        ui.label(err);
                    }
                    Ok(response) => {
                        response_ui(ui, response);
                    }
                },
            }
        });

        Ok(())
    }

    fn ui_url(&mut self, ui: &mut egui::Ui) -> bool {
        let mut trigger_fetch = self.ui_examples(ui);

        egui::Grid::new("request_parameters")
            .spacing(egui::Vec2::splat(4.0))
            .min_col_width(70.0)
            .num_columns(2)
            .show(ui, |ui| {
                ui.label("URL:");
                trigger_fetch |= ui.text_edit_singleline(&mut self.url).lost_focus();
                ui.end_row();

                ui.label("Method:");
                ui.horizontal(|ui| {
                    ui.radio_value(&mut self.method, Method::Get, "GET")
                        .clicked();
                    ui.radio_value(&mut self.method, Method::Head, "HEAD")
                        .clicked();
                    ui.radio_value(&mut self.method, Method::Post, "POST")
                        .clicked();
                });
                ui.end_row();

                if self.method == Method::Post {
                    ui.label("POST Body:");
                    ui.add(
                        egui::TextEdit::multiline(&mut self.request_body)
                            .code_editor()
                            .desired_rows(1),
                    );
                    ui.end_row();
                }

                ui.checkbox(&mut self.streaming, "Stream HTTP Response")
                    .on_hover_text("Read the HTTP response in chunks");
                ui.end_row();
            });

        trigger_fetch |= ui.button("fetch ▶").clicked();

        trigger_fetch
    }

    fn ui_examples(&mut self, ui: &mut egui::Ui) -> bool {
        let mut trigger_fetch = false;

        ui.horizontal(|ui| {
            ui.label("Examples:");

            let self_url = format!(
                "https://raw.githubusercontent.com/emilk/ehttp/master/{}",
                file!()
            );
            if ui
                .selectable_label(
                    (&self.url, self.method) == (&self_url, Method::Get),
                    "GET source code",
                )
                .clicked()
            {
                self.url = self_url;
                self.method = Method::Get;
                trigger_fetch = true;
            }

            let wasm_file = "https://emilk.github.io/ehttp/example_eframe_bg.wasm".to_owned();
            if ui
                .selectable_label(
                    (&self.url, self.method) == (&wasm_file, Method::Get),
                    "GET .wasm",
                )
                .clicked()
            {
                self.url = wasm_file;
                self.method = Method::Get;
                trigger_fetch = true;
            }

            let pastebin_url = "https://httpbin.org/post".to_owned();
            if ui
                .selectable_label(
                    (&self.url, self.method) == (&pastebin_url, Method::Post),
                    "POST to httpbin.org",
                )
                .clicked()
            {
                self.url = pastebin_url;
                self.method = Method::Post;
                trigger_fetch = true;
            }
        });

        trigger_fetch
    }
}

#[derive(Debug, PartialEq, Clone)]
struct ViewState {
    path: String,
    reqfile: UnresolvedRequestFile,
}

#[derive(Debug, PartialEq, Clone)]
struct EditState {
    path: String,
    request: String,
    response: String,
    config: String,
}

#[derive(Debug, PartialEq, Clone)]
struct ResolvingState {
    path: String,
    reqfile: UnresolvedRequestFile,
    env: Option<String>,
    prompts: Vec<(String, String)>,
    secrets: Vec<(String, String)>,
}

#[derive(Debug, PartialEq, Clone)]
struct ResolvedState {
    path: String,
    reqfile: ResolvedRequestFile,
}

#[allow(dead_code)]
#[derive(Clone)]
struct RequestResponseState {
    path: String,
    reqfile: ResolvedRequestFile,
    download: Arc<Mutex<Download>>,
}

#[derive(Debug, PartialEq, Copy, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
enum Method {
    Get,
    Head,
    Post,
}

impl FromStr for Method {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "GET" => Ok(Self::Get),
            "HEAD" => Ok(Self::Head),
            "POST" => Ok(Self::Post),
            _ => Err(format!("Unsupported HTTP Verb: {s}")),
        }
    }
}

enum Download {
    None,
    InProgress,
    StreamingInProgress {
        response: ehttp::PartialResponse,
        body: Vec<u8>,
    },
    Done(ehttp::Result<ehttp::Response>),
}

#[allow(dead_code)]
pub struct Client {
    streaming: bool,
    state: ClientState,
}

impl Default for Client {
    fn default() -> Self {
        Self {
            streaming: true,
            state: ClientState::Demo(DemoState::default()),
        }
    }
}

impl eframe::App for Client {
    fn update(&mut self, egui_ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Err(err) = match &mut self.state {
            ClientState::Init => todo!(),
            ClientState::View(_) => todo!(),
            ClientState::Edit(_) => todo!(),
            ClientState::Resolving(_) => todo!(),
            ClientState::Resolved(_) => todo!(),
            ClientState::RequestResponse(_) => todo!(),
            ClientState::Demo(demo) => demo.ui(egui_ctx),
        } {
            panic!("{err}");
        }
    }
}

fn on_fetch_part(
    part: Result<ehttp::streaming::Part, String>,
    download_store: &mut Download,
) -> ControlFlow<()> {
    let part = match part {
        Err(err) => {
            *download_store = Download::Done(Result::Err(err));
            return ControlFlow::Break(());
        }
        Ok(part) => part,
    };

    match part {
        ehttp::streaming::Part::Response(response) => {
            *download_store = Download::StreamingInProgress {
                response,
                body: Vec::new(),
            };
            ControlFlow::Continue(())
        }
        ehttp::streaming::Part::Chunk(chunk) => {
            if let Download::StreamingInProgress { response, mut body } =
                std::mem::replace(download_store, Download::None)
            {
                body.extend_from_slice(&chunk);

                if chunk.is_empty() {
                    // This was the last chunk.
                    *download_store = Download::Done(Ok(response.complete(body)));
                    ControlFlow::Break(())
                } else {
                    // More to come.
                    *download_store = Download::StreamingInProgress { response, body };
                    ControlFlow::Continue(())
                }
            } else {
                ControlFlow::Break(()) // some data race - abort download.
            }
        }
    }
}

impl Client {}

fn response_ui(ui: &mut egui::Ui, response: &ehttp::Response) {
    ui.monospace(format!("url:          {}", response.url));
    ui.monospace(format!(
        "status:       {} ({})",
        response.status, response.status_text
    ));
    ui.monospace(format!(
        "content-type: {}",
        response.content_type().unwrap_or_default()
    ));
    ui.monospace(format!(
        "size:         {:.1} kB",
        response.bytes.len() as f32 / 1000.0
    ));

    ui.separator();

    egui::ScrollArea::vertical().show(ui, |ui| {
        egui::CollapsingHeader::new("Response headers")
            .default_open(false)
            .show(ui, |ui| {
                egui::Grid::new("response_headers")
                    .spacing(egui::vec2(ui.spacing().item_spacing.x * 2.0, 0.0))
                    .show(ui, |ui| {
                        for (k, v) in &response.headers {
                            ui.label(k);
                            ui.label(v);
                            ui.end_row();
                        }
                    })
            });

        ui.separator();

        if let Some(text) = response.text() {
            let tooltip = "Click to copy the response body";
            if ui.button("📋").on_hover_text(tooltip).clicked() {
                ui.output_mut(|o| o.copied_text = text.to_owned());
            }
            ui.separator();
        }

        if let Some(text) = response.text() {
            selectable_text(ui, text);
        } else {
            ui.monospace("[binary]");
        }
    });
}

fn selectable_text(ui: &mut egui::Ui, mut text: &str) {
    ui.add(
        egui::TextEdit::multiline(&mut text)
            .desired_width(f32::INFINITY)
            .font(egui::TextStyle::Monospace.resolve(ui.style())),
    );
}

fn main() -> eframe::Result<()> {
    eframe::run_native(
        "Reqlang Client",
        Default::default(),
        Box::new(|_cc| Box::<Client>::default()),
    )
}
