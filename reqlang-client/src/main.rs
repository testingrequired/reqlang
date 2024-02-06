use std::{
    collections::HashMap,
    fs,
    ops::ControlFlow,
    str::FromStr,
    sync::{Arc, Mutex},
};

use eframe::egui;
use parser::split;
use types::{ResolvedRequestFile, UnresolvedRequestFile};

#[allow(dead_code)]
enum ClientState {
    Init(InitState),
    View(ViewState),
    Edit(EditState),
    Resolving(ResolvingState),
    Resolved(ResolvedState),
    RequestResponse(RequestResponseState),
    Demo(DemoState),
}

struct InitState {
    picked_path: Option<String>,
}

impl Default for InitState {
    fn default() -> Self {
        Self { picked_path: None }
    }
}

impl InitState {
    pub fn ui(&mut self, egui_ctx: &egui::Context) -> Result<ClientState, &str> {
        let mut next_state: ClientState = ClientState::Init(InitState { picked_path: None });

        egui::CentralPanel::default().show(egui_ctx, |ui| {
            if ui.button("Open file…").clicked() {
                if let Some(path) = rfd::FileDialog::new().pick_file() {
                    self.picked_path = Some(path.display().to_string());
                }
            }

            if let Some(picked_path) = &self.picked_path {
                let source = fs::read_to_string(&picked_path)
                    .expect("Should have been able to read the file");

                let split = split(&source).unwrap();

                next_state = ClientState::View(ViewState {
                    path: picked_path.to_owned(),
                    source: source,
                    request: split.request.0,
                    response: split
                        .response
                        .map(|(response, _)| response)
                        .unwrap_or("".to_owned()),
                    config: split
                        .config
                        .map(|(config, _)| config)
                        .unwrap_or("".to_owned()),
                });
            } else {
                next_state = ClientState::Init(InitState { picked_path: None });
            }
        });

        Ok(next_state)
    }
}

#[derive(Clone)]
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
    pub fn ui(&mut self, egui_ctx: &egui::Context) -> Result<ClientState, &str> {
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

        Ok(ClientState::Demo(self.clone()))
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
    source: String,
    request: String,
    response: String,
    config: String,
}

impl ViewState {
    pub fn ui(&mut self, egui_ctx: &egui::Context) -> Result<ClientState, &str> {
        let mut next_state: ClientState = ClientState::View(self.clone());

        egui::CentralPanel::default().show(egui_ctx, |ui| {
            ui.heading("Request");

            selectable_text(ui, &format!("{:#?}", &self.request));

            ui.heading("Response");

            selectable_text(ui, &format!("{:#?}", &self.response));

            ui.heading("Config");

            selectable_text(ui, &format!("{:#?}", &self.config));

            if ui.button("Resolve").clicked() {
                next_state = ClientState::Resolving(ResolvingState::new(
                    self.path.clone(),
                    parser::parse(&self.source).unwrap(),
                    "".to_owned(),
                    HashMap::new(),
                    HashMap::new(),
                ))
            }
        });

        Ok(next_state)
    }
}

#[derive(Debug, PartialEq, Clone)]
struct EditState {
    path: String,
    request: String,
    response: String,
    config: String,
}

impl EditState {
    pub fn ui(&mut self, egui_ctx: &egui::Context) -> Result<ClientState, &str> {
        egui::CentralPanel::default().show(egui_ctx, |_| {});

        Ok(ClientState::Edit(self.clone()))
    }
}

#[derive(Debug, PartialEq, Clone)]
struct ResolvingState {
    path: String,
    reqfile: UnresolvedRequestFile,
    env: String,
    prompts: HashMap<String, String>,
    secrets: HashMap<String, String>,
    resolved: Option<ResolvedRequestFile>,
}

impl ResolvingState {
    fn new(
        path: String,
        reqfile: UnresolvedRequestFile,
        env: String,
        mut prompts: HashMap<String, String>,
        mut secrets: HashMap<String, String>,
    ) -> Self {
        let prompt_names = reqfile.prompt_names();
        let secret_names = reqfile.secret_names();

        for prompt_name in prompt_names.clone() {
            prompts.insert(prompt_name.to_string(), String::new());
        }

        for secret_name in secret_names.clone() {
            secrets.insert(secret_name.to_string(), String::new());
        }

        Self {
            path,
            reqfile,
            env,
            prompts,
            secrets,
            resolved: None,
        }
    }
}

impl ResolvingState {
    pub fn ui(&mut self, egui_ctx: &egui::Context) -> Result<ClientState, &str> {
        egui::CentralPanel::default().show(egui_ctx, |ui| {
            let env_names = self.reqfile.env_names();
            let prompt_names = self.reqfile.prompt_names();
            let secret_names = self.reqfile.secret_names();

            if !env_names.is_empty() {
                ui.heading("Environment");

                ui.horizontal(|ui| {
                    for env_name in env_names {
                        ui.radio_value(&mut self.env, env_name.to_owned(), env_name);
                    }
                });

                ui.end_row();
            } else {
                ui.heading("No Environments Found!");
            }

            if !prompt_names.is_empty() {
                ui.heading("Prompts");

                ui.horizontal(|ui| {
                    for prompt_name in prompt_names {
                        let input = self.prompts.get_mut(prompt_name).unwrap();

                        ui.horizontal(|ui| {
                            ui.label(prompt_name);
                            ui.text_edit_singleline(input);
                        });

                        ui.end_row();
                    }
                });

                ui.end_row();
            } else {
                ui.heading("No Prompts Found!");
            }

            if !secret_names.is_empty() {
                ui.heading("Secrets");

                ui.horizontal(|ui| {
                    for secret_name in secret_names {
                        let input = self.secrets.get_mut(secret_name).unwrap();

                        ui.horizontal(|ui| {
                            ui.label(secret_name);
                            ui.text_edit_singleline(input);
                        });

                        ui.end_row();
                    }
                });

                ui.end_row();
            } else {
                ui.heading("No Secrets Found!");
            }

            ui.separator();
        });

        Ok(ClientState::Resolving(self.clone()))
    }
}

#[derive(Debug, PartialEq, Clone)]
struct ResolvedState {
    path: String,
    reqfile: ResolvedRequestFile,
}

impl ResolvedState {
    pub fn ui(&mut self, egui_ctx: &egui::Context) -> Result<ClientState, &str> {
        egui::CentralPanel::default().show(egui_ctx, |_| {});

        Ok(ClientState::Resolved(self.clone()))
    }
}

#[allow(dead_code)]
#[derive(Clone)]
struct RequestResponseState {
    path: String,
    reqfile: ResolvedRequestFile,
    download: Arc<Mutex<Download>>,
}

impl RequestResponseState {
    pub fn ui(&mut self, egui_ctx: &egui::Context) -> Result<ClientState, &str> {
        egui::CentralPanel::default().show(egui_ctx, |_| {});

        Ok(ClientState::RequestResponse(self.clone()))
    }
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

#[derive(Clone)]
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
    state: Box<ClientState>,
}

impl Default for Client {
    fn default() -> Self {
        Self {
            streaming: true,
            state: Box::new(ClientState::Init(InitState::default())),
        }
    }
}

impl eframe::App for Client {
    fn update(&mut self, egui_ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let next_state = match &mut *self.state {
            ClientState::Init(state) => state.ui(egui_ctx),
            ClientState::View(state) => state.ui(egui_ctx),
            ClientState::Edit(state) => state.ui(egui_ctx),
            ClientState::Resolving(state) => state.ui(egui_ctx),
            ClientState::Resolved(state) => state.ui(egui_ctx),
            ClientState::RequestResponse(state) => state.ui(egui_ctx),
            ClientState::Demo(state) => state.ui(egui_ctx),
        };

        self.state = match next_state {
            Ok(next_state) => Box::new(next_state),
            Err(err) => panic!("{err}"),
        };
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
