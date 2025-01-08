use std::{
    collections::HashMap,
    fs,
    ops::ControlFlow,
    str::FromStr,
    sync::{Arc, Mutex},
};

use eframe::egui;
use reqlang::{
    export::{export, Format},
    http::HttpRequest,
    parse, resolve, template, ParsedRequestFile, ResolvedRequestFile,
};

#[allow(dead_code)]
#[derive(Clone)]
enum ClientState {
    Init(InitState),
    View(ViewState),
    Resolving(ResolvingState),
    Resolved(ResolvedState),
    ExecutingRequest(ExecutingRequestState),
}

#[derive(Debug, Default, Clone)]
struct InitState {}

impl InitState {
    pub fn ui(
        &mut self,
        egui_ctx: &egui::Context,
        client_ctx: &mut ClientContext,
    ) -> Result<StateTransition, &str> {
        let mut next_state: StateTransition = StateTransition::None;

        egui::CentralPanel::default().show(egui_ctx, |ui| {
            if ui.button("Open file…").clicked() {
                if let Some(path) = rfd::FileDialog::new().pick_file() {
                    client_ctx.path = Some(path.display().to_string());
                }
            }

            client_ctx.source = if let Some(path) = &client_ctx.path {
                let source =
                    fs::read_to_string(path).expect("Should have been able to read the file");

                Some(source)
            } else {
                None
            };

            client_ctx.reqfile = client_ctx
                .source
                .as_ref()
                .map(|source| Box::new(parse(source).unwrap()));

            if client_ctx.reqfile.is_some() {
                next_state = StateTransition::New(ClientState::View(ViewState {}));
            }
        });

        Ok(next_state)
    }
}

#[derive(Debug, PartialEq, Clone)]
struct ViewState {}

impl ViewState {
    pub fn ui(
        &mut self,
        egui_ctx: &egui::Context,
        client_ctx: &mut ClientContext,
    ) -> Result<StateTransition, &str> {
        let mut next_state: StateTransition = StateTransition::None;

        let request: Option<&HttpRequest> = match &client_ctx.reqfile {
            Some(reqfile) => Some(&reqfile.request.0),
            None => None,
        };

        let request_string = match request {
            Some(request) => export(request, Format::HttpMessage),
            None => String::new(),
        };

        let env_names: Vec<String> = match &client_ctx.reqfile {
            Some(reqfile) => reqfile.envs(),
            None => vec![],
        };

        let var_names: Vec<String> = match &client_ctx.reqfile {
            Some(reqfile) => reqfile.vars(),
            None => vec![],
        };

        let prompt_names: Vec<String> = match &client_ctx.reqfile {
            Some(reqfile) => reqfile.prompts(),
            None => vec![],
        };

        let secret_names: Vec<String> = match &client_ctx.reqfile {
            Some(reqfile) => reqfile.secrets(),
            None => vec![],
        };

        egui::CentralPanel::default().show(egui_ctx, |ui| {
            if ui.button("Back").clicked() {
                client_ctx.path = None;
                next_state = StateTransition::Back;
                return;
            }

            if ui.button("Resolve").clicked() {
                match &client_ctx.source {
                    Some(_) => {
                        next_state =
                            StateTransition::New(ClientState::Resolving(ResolvingState::new(
                                "".to_owned(),
                                prompt_names.clone(),
                                secret_names.clone(),
                                HashMap::new(),
                                HashMap::new(),
                            )));
                    }
                    None => todo!(),
                }
            }

            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading("Request");

                selectable_text(ui, &request_string);

                ui.separator();

                ui.heading("Config");

                ui.label(format!("Environments: {:?}", &env_names));
                ui.label(format!("Variables: {:?}", &var_names));
                ui.label(format!("Prompts: {:?}", &prompt_names));
                ui.label(format!("Secrets: {:?}", &secret_names));
            });
        });

        Ok(next_state)
    }
}

#[derive(Debug, PartialEq, Clone)]
struct ResolvingState {
    env: String,
    prompts: HashMap<String, String>,
    secrets: HashMap<String, String>,
}

impl ResolvingState {
    fn new(
        env: String,
        prompt_names: Vec<String>,
        secret_names: Vec<String>,
        mut prompts: HashMap<String, String>,
        mut secrets: HashMap<String, String>,
    ) -> Self {
        for prompt_name in prompt_names.clone() {
            prompts.insert(prompt_name.to_string(), String::new());
        }

        for secret_name in secret_names.clone() {
            secrets.insert(secret_name.to_string(), String::new());
        }

        Self {
            env,
            prompts,
            secrets,
        }
    }
}

impl ResolvingState {
    pub fn ui(
        &mut self,
        egui_ctx: &egui::Context,
        client_ctx: &ClientContext,
    ) -> Result<StateTransition, &str> {
        let mut next_state: StateTransition = StateTransition::None;

        // let reqfile = &client_ctx.reqfile.unwrap();

        let env_names: Vec<String> = match &client_ctx.reqfile {
            Some(reqfile) => reqfile.envs(),
            None => vec![],
        };

        let prompt_names: Vec<String> = match &client_ctx.reqfile {
            Some(reqfile) => reqfile.prompts(),
            None => vec![],
        };

        let secret_names: Vec<String> = match &client_ctx.reqfile {
            Some(reqfile) => reqfile.secrets(),
            None => vec![],
        };

        egui::CentralPanel::default().show(egui_ctx, |ui| {
            if ui.button("Back").clicked() {
                next_state = StateTransition::Back;
                return;
            }

            if !env_names.is_empty() {
                ui.heading("Environment");

                ui.horizontal(|ui| {
                    for env_name in env_names {
                        ui.radio_value(&mut self.env, env_name.to_string(), env_name.to_string());
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
                        let input = self.prompts.get_mut(prompt_name.as_str()).unwrap();

                        ui.horizontal(|ui| {
                            ui.label(prompt_name.as_str());
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
                        let input = self.secrets.get_mut(secret_name.as_str()).unwrap();

                        ui.horizontal(|ui| {
                            ui.label(secret_name.as_str());
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

            if ui.button("Resolve").clicked() {
                let reqfile = resolve(
                    &client_ctx.source.clone().unwrap(),
                    &self.env,
                    &self.prompts,
                    &self.secrets,
                )
                .unwrap();

                next_state = StateTransition::New(ClientState::Resolved(ResolvedState::new(
                    Box::new(reqfile),
                )));
            }
        });

        Ok(next_state)
    }
}

#[derive(Clone)]
struct ResolvedState {
    reqfile: Box<ResolvedRequestFile>,
}

impl ResolvedState {
    pub fn new(reqfile: Box<ResolvedRequestFile>) -> Self {
        Self { reqfile }
    }

    pub fn ui(
        &mut self,
        egui_ctx: &egui::Context,
        _client_ctx: &ClientContext,
    ) -> Result<StateTransition, &str> {
        let mut next_state: StateTransition = StateTransition::None;

        let request = &self.reqfile.request.0;

        let request_string = export(request, Format::HttpMessage);

        egui::CentralPanel::default().show(egui_ctx, |ui| {
            if ui.button("Back").clicked() {
                next_state = StateTransition::Back;
                return;
            }

            ui.heading("Request");

            selectable_text(ui, &request_string);

            if ui.button("Send Request").clicked() {
                next_state = StateTransition::New(ClientState::ExecutingRequest(
                    ExecutingRequestState::new(self.reqfile.clone()),
                ));
            }
        });

        Ok(next_state)
    }
}

#[derive(Clone)]
struct ExecutingRequestState {
    reqfile: Box<ResolvedRequestFile>,
    download: Arc<Mutex<Download>>,
}

impl ExecutingRequestState {
    pub fn new(reqfile: Box<ResolvedRequestFile>) -> Self {
        Self {
            reqfile,
            download: Arc::new(Mutex::new(Download::None)),
        }
    }

    pub fn ui(
        &mut self,
        egui_ctx: &egui::Context,
        client_ctx: &ClientContext,
    ) -> Result<StateTransition, &str> {
        let mut next_state: StateTransition = StateTransition::None;

        let env = self.reqfile.config.0.env.as_str();
        let prompts = &self.reqfile.config.0.prompts;
        let secrets = &self.reqfile.config.0.secrets;

        let provider_values = {
            let mut values = HashMap::new();

            values.insert("env".to_string(), env.to_string());

            values
        };

        let input = client_ctx.source.as_ref().unwrap().as_str();

        let templated_reqfile = template(input, env, prompts, secrets, provider_values)
            .expect("Should be a valid reqfile");

        let request = &templated_reqfile.request;
        let request_string = export(request, Format::HttpMessage);

        egui::CentralPanel::default().show(egui_ctx, |ui| {
            ui.heading("Request");

            selectable_text(ui, &request_string);

            if ui.button("Execute").clicked() {
                eprintln!("CLICKED EXECUTE");
                let verb = &request.verb;
                let target = request.target.as_str();
                let download_store = self.download.clone();
                *download_store.lock().unwrap() = Download::InProgress;
                let egui_ctx = egui_ctx.clone();

                let request = match verb.to_string().as_str() {
                    "GET" => ehttp::Request::get(target),
                    "HEAD" => ehttp::Request::head(target),
                    "POST" => ehttp::Request::post(
                        target,
                        request.body.clone().unwrap_or_default().as_bytes().to_vec(),
                    ),
                    _ => unreachable!(),
                };

                let streaming = true;

                if streaming {
                    eprintln!("STREAMING REQUEST");
                    // The more complicated streaming API:
                    ehttp::streaming::fetch(request, move |part| {
                        egui_ctx.request_repaint(); // Wake up UI thread
                        on_fetch_part(part, &mut download_store.lock().unwrap())
                    });
                } else {
                    eprintln!("NON STREAMING REQUEST");
                    // The simple non-streaming API:
                    ehttp::fetch(request, move |response| {
                        *download_store.lock().unwrap() = Download::Done(response);
                        egui_ctx.request_repaint(); // Wake up UI thread
                    });
                }

                ui.separator();
            }

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
                Download::Done(response) => {
                    if ui.button("Back").clicked() {
                        next_state = StateTransition::Back;
                        return;
                    }

                    ui.separator();

                    match response {
                        Err(err) => {
                            ui.label(err);
                        }
                        Ok(response) => {
                            response_ui(ui, response);
                        }
                    }
                }
            }
        });

        Ok(next_state)
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

#[derive(Default, Debug)]
struct ClientContext {
    path: Option<String>,
    source: Option<String>,
    reqfile: Option<Box<ParsedRequestFile>>,
}

#[allow(dead_code)]
pub struct Client {
    streaming: bool,
    context: Box<ClientContext>,
    states: Vec<ClientState>,
}

impl Client {}

impl Default for Client {
    fn default() -> Self {
        Self {
            streaming: true,
            context: Box::<ClientContext>::default(),
            states: vec![ClientState::Init(InitState::default())],
        }
    }
}

enum StateTransition {
    None,
    Back,
    New(ClientState),
}

impl eframe::App for Client {
    fn update(&mut self, egui_ctx: &egui::Context, _frame: &mut eframe::Frame) {
        match self.states.last_mut() {
            Some(latest_state) => {
                let next_state = match &mut *latest_state {
                    ClientState::Init(state) => state.ui(egui_ctx, &mut self.context),
                    ClientState::View(state) => state.ui(egui_ctx, &mut self.context),
                    ClientState::Resolving(state) => state.ui(egui_ctx, &self.context),
                    ClientState::Resolved(state) => state.ui(egui_ctx, &self.context),
                    ClientState::ExecutingRequest(state) => state.ui(egui_ctx, &self.context),
                };

                match next_state {
                    Ok(next_state) => match next_state {
                        StateTransition::None => {}
                        StateTransition::Back => {
                            self.states.pop();
                        }
                        StateTransition::New(next_state) => self.states.push(next_state),
                    },
                    Err(err) => panic!("{err}"),
                };
            }
            None => panic!("Trying to pop non existant state from client"),
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
                ui.output_mut(|o| text.clone_into(&mut o.copied_text));
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
