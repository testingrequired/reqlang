use std::collections::HashMap;
use std::ops::Deref;

use anyhow::{Context, Result};
use reqlang::diagnostics::{
    Diagnoser, Diagnosis, DiagnosisPosition, DiagnosisRange, DiagnosisSeverity,
};
use reqlang::errors::ReqlangError;
use reqlang::http::{HttpResponse, HttpVersion};
use reqlang::{http::HttpRequest, parse, template, Spanned, UnresolvedRequestFile};
use reqwest::{Method, Url, Version};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::Mutex;
use tower_lsp::jsonrpc::Result as RpcResult;
use tower_lsp::lsp_types::notification::Notification;
use tower_lsp::lsp_types::{
    Diagnostic, DiagnosticSeverity, DidChangeTextDocumentParams, DidOpenTextDocumentParams,
    DidSaveTextDocumentParams, ExecuteCommandOptions, ExecuteCommandParams, InitializeParams,
    InitializeResult, MessageType, Position, Range, SaveOptions, ServerCapabilities, ServerInfo,
    TextDocumentSyncKind, TextDocumentSyncOptions,
};
use tower_lsp::{Client, LanguageServer, LspService, Server};

struct Backend {
    client: Client,
    file_texts: Mutex<HashMap<Url, String>>,
}

impl Backend {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            file_texts: Mutex::new(HashMap::new()),
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> RpcResult<InitializeResult> {
        let version = env!("CARGO_PKG_VERSION").to_string();

        self.client
            .log_message(
                MessageType::INFO,
                format!("Reqlang Language Server (v{}) running...", version),
            )
            .await;

        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                execute_command_provider: Some(ExecuteCommandOptions {
                    commands: vec![
                        "reqlang.executeRequest".to_string(),
                        "reqlang.exportRequest".to_string(),
                    ],
                    work_done_progress_options: Default::default(),
                }),
                text_document_sync: Some(
                    tower_lsp::lsp_types::TextDocumentSyncCapability::Options(
                        TextDocumentSyncOptions {
                            open_close: Some(true),
                            change: TextDocumentSyncKind::FULL.into(),
                            save: Some(
                                SaveOptions {
                                    include_text: Some(true),
                                }
                                .into(),
                            ),
                            ..Default::default()
                        },
                    ),
                ),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: env!("CARGO_PKG_NAME").to_string(),
                version: Some(version),
            }),
        })
    }

    async fn shutdown(&self) -> RpcResult<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let source = params.text_document.text;

        let mut file_texts = self.file_texts.lock().await;
        file_texts.insert(uri.clone(), source.clone());

        let result: Result<ParseResult, Vec<Spanned<ReqlangError>>> =
            parse(&source).map(|unresolved_reqfile| {
                let result: ParseResult = unresolved_reqfile.into();

                result
            });

        self.client
            .send_notification::<ParseNotification>(ParseNotificationParams::new(
                uri.clone(),
                result,
            ))
            .await;

        let version = Some(params.text_document.version);
        let diagnostics = Diagnoser::get_diagnostics(&source);
        self.client
            .publish_diagnostics(
                uri.clone(),
                diagnostics
                    .iter()
                    .map(|x| (LspDiagnosis((*x).clone())).into())
                    .collect(),
                version,
            )
            .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        let source = &params.content_changes.first().unwrap().text;

        let version = Some(params.text_document.version);
        let diagnostics = Diagnoser::get_diagnostics(source);
        self.client
            .publish_diagnostics(
                uri,
                diagnostics
                    .iter()
                    .map(|x| LspDiagnosis((*x).clone()).into())
                    .collect(),
                version,
            )
            .await;
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.text.unwrap_or_default();

        let mut file_texts = self.file_texts.lock().await;
        file_texts.insert(uri.clone(), text.clone());

        let result: Result<ParseResult, Vec<Spanned<ReqlangError>>> =
            parse(&text).map(|unresolved_reqfile| {
                let result: ParseResult = unresolved_reqfile.into();

                result
            });

        self.client
            .send_notification::<ParseNotification>(ParseNotificationParams::new(uri, result))
            .await;
    }

    async fn execute_command(&self, params: ExecuteCommandParams) -> RpcResult<Option<Value>> {
        if params.command.as_str() == "reqlang.executeRequest" {
            let from_client_params_value = params.arguments.first().expect("Should be present");

            self.client
                .log_message(MessageType::INFO, format!("{:?}", from_client_params_value))
                .await;

            // Get parsed params from JSON `Value`
            let from_client_params: FromClientExecuteRequestParams =
                from_client_params_value.clone().into();

            // Setup provider values
            let env = from_client_params.env.as_str();
            let mut provider = HashMap::new();
            provider.insert("env".to_string(), env.to_string());

            // Get reqfile text content
            let url = Url::parse(&from_client_params.uri).expect("Should be a valid url");
            let file_texts = self.file_texts.lock().await;
            let text = file_texts.get(&url).expect("Should be present");

            // Template the reqfile
            let templated_reqfile = template(
                text,
                env,
                &from_client_params.prompts,
                &from_client_params.secrets,
                provider,
            )
            .expect("Should have templated");

            let http_client = reqwest::Client::new();

            let method: Method = match templated_reqfile.request.verb.to_string().as_str() {
                "GET" => Method::GET,
                "POST" => Method::POST,
                _ => todo!(),
            };

            let mut request = http_client.request(method, templated_reqfile.request.target);

            for (key, value) in &templated_reqfile.request.headers {
                request = request.header(key, value);
            }

            if let Some(body) = templated_reqfile.request.body {
                if !body.is_empty() {
                    request = request.body(body);
                }
            }

            let response = request.send().await.expect("Should not error");
            let http_version = match response.version() {
                Version::HTTP_11 => HttpVersion::one_point_one(),
                _ => todo!(),
            };
            let mut headers = HashMap::new();
            for (key, value) in response.headers() {
                headers.insert(
                    key.to_string(),
                    value.to_str().expect("Shoud work").to_string(),
                );
            }
            let status = response.status().to_string();
            let mut status_split = status.splitn(2, ' ');
            let status_code = status_split.next().unwrap().to_string();
            let status_text = status_split.next().unwrap().to_string();
            let body = Some(response.text().await.expect("Should have body"));

            let reqlang_response = HttpResponse {
                http_version,
                status_code,
                status_text,
                headers,
                body,
            };

            return Ok(Some(
                serde_json::to_string(&reqlang_response)
                    .expect("Should serialize to json")
                    .into(),
            ));
        };

        if params.command.as_str() == "reqlang.exportRequest" {
            let from_client_params_value = params.arguments.first().expect("Should be present");

            self.client
                .log_message(MessageType::INFO, format!("{:?}", from_client_params_value))
                .await;

            // Get parsed params from JSON `Value`
            let from_client_params: FromClientExportRequestParams =
                from_client_params_value.clone().into();

            // Setup provider values
            let env = from_client_params.env.as_str();
            let mut provider = HashMap::new();
            provider.insert("env".to_string(), env.to_string());

            // Get reqfile text content
            let url = Url::parse(&from_client_params.uri).expect("Should be a valid url");
            let file_texts = self.file_texts.lock().await;
            let text = file_texts.get(&url).expect("Should be present");

            // Template the reqfile
            let templated_reqfile = template(
                text,
                env,
                &from_client_params.prompts,
                &from_client_params.secrets,
                provider,
            )
            .expect("Should have templated");

            let exported = export::export(&templated_reqfile.request, from_client_params.format);

            return Ok(Some(exported.into()));
        }

        self.client
            .log_message(MessageType::INFO, "command executed!")
            .await;

        Ok(Some(Value::String("DONE!".to_string())))
    }
}

impl From<UnresolvedRequestFile> for ParseResult {
    fn from(value: UnresolvedRequestFile) -> Self {
        let vars = value
            .config
            .clone()
            .unwrap_or_default()
            .0
            .vars
            .unwrap_or_default();

        let envs: Vec<String> = value
            .config
            .clone()
            .unwrap_or_default()
            .0
            .envs
            .unwrap_or_default()
            .keys()
            .cloned()
            .collect();

        let prompts: Vec<String> = value
            .config
            .clone()
            .unwrap_or_default()
            .0
            .prompts
            .unwrap_or_default()
            .keys()
            .cloned()
            .collect();

        let secrets = value
            .config
            .clone()
            .unwrap_or_default()
            .0
            .secrets
            .unwrap_or_default();

        Self {
            vars,
            envs,
            prompts,
            secrets,
            request: value.clone().request.0,
            full: value,
        }
    }
}

/// A simplified version of the parsed file
///
/// This is useful for language server clients
#[derive(Debug, Deserialize, Serialize)]
struct ParseResult {
    vars: Vec<String>,
    envs: Vec<String>,
    prompts: Vec<String>,
    secrets: Vec<String>,
    request: HttpRequest,
    full: UnresolvedRequestFile,
}

/// Command parameters from client to execute request
///
/// This is useful for language server clients
#[derive(Debug, Deserialize, Serialize, Default)]
struct FromClientExecuteRequestParams {
    uri: String,
    env: String,
    vars: HashMap<String, String>,
    prompts: HashMap<String, String>,
    secrets: HashMap<String, String>,
}

impl From<Value> for FromClientExecuteRequestParams {
    fn from(params_value: Value) -> Self {
        let uri = params_value
            .get("uri")
            .expect("Should be present")
            .as_str()
            .expect("Should be a string")
            .to_string();

        let env = params_value
            .get("env")
            .expect("Should be present")
            .as_str()
            .expect("Should be a string")
            .to_string();

        let vars_from_params = params_value
            .get("vars")
            .map(|v| v.as_object().expect("Should be there"))
            .expect("Should be there");

        let mut vars: HashMap<String, String> = HashMap::default();

        for (key, value) in vars_from_params {
            vars.insert(
                key.to_string(),
                value.as_str().expect("Should be a string").to_string(),
            );
        }

        let prompts_from_params = params_value
            .get("prompts")
            .map(|v| v.as_object().expect("Should be there"))
            .expect("Should be there");

        let mut prompts: HashMap<String, String> = HashMap::default();

        for (key, value) in prompts_from_params {
            prompts.insert(
                key.to_string(),
                value.as_str().expect("Should be a string").to_string(),
            );
        }

        let secrets_from_params = params_value
            .get("secrets")
            .map(|v| v.as_object().expect("Should be there"))
            .expect("Should be there");

        let mut secrets: HashMap<String, String> = HashMap::default();

        for (key, value) in secrets_from_params {
            secrets.insert(
                key.to_string(),
                value.as_str().expect("Should be a string").to_string(),
            );
        }

        FromClientExecuteRequestParams {
            uri,
            env,
            vars,
            prompts,
            secrets,
        }
    }
}

/// Command parameters from client to export request
///
/// This is useful for language server clients
#[derive(Debug, Deserialize, Serialize, Default)]
struct FromClientExportRequestParams {
    uri: String,
    env: String,
    vars: HashMap<String, String>,
    prompts: HashMap<String, String>,
    secrets: HashMap<String, String>,
    format: export::Format,
}

impl From<Value> for FromClientExportRequestParams {
    fn from(params_value: Value) -> Self {
        let uri = params_value
            .get("uri")
            .expect("Should be present")
            .as_str()
            .expect("Should be a string")
            .to_string();

        let env = params_value
            .get("env")
            .expect("Should be present")
            .as_str()
            .expect("Should be a string")
            .to_string();

        let vars_from_params = params_value
            .get("vars")
            .map(|v| v.as_object().expect("Should be there"))
            .expect("Should be there");

        let mut vars: HashMap<String, String> = HashMap::default();

        for (key, value) in vars_from_params {
            vars.insert(
                key.to_string(),
                value.as_str().expect("Should be a string").to_string(),
            );
        }

        let prompts_from_params = params_value
            .get("prompts")
            .map(|v| v.as_object().expect("Should be there"))
            .expect("Should be there");

        let mut prompts: HashMap<String, String> = HashMap::default();

        for (key, value) in prompts_from_params {
            prompts.insert(
                key.to_string(),
                value.as_str().expect("Should be a string").to_string(),
            );
        }

        let secrets_from_params = params_value
            .get("secrets")
            .map(|v| v.as_object().expect("Should be there"))
            .expect("Should be there");

        let mut secrets: HashMap<String, String> = HashMap::default();

        for (key, value) in secrets_from_params {
            secrets.insert(
                key.to_string(),
                value.as_str().expect("Should be a string").to_string(),
            );
        }

        FromClientExportRequestParams {
            uri,
            env,
            vars,
            prompts,
            secrets,
            format: Default::default(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct ParseNotificationParams {
    file_id: String,
    result: Result<ParseResult, Vec<Spanned<ReqlangError>>>,
}

impl ParseNotificationParams {
    fn new(
        file_id: impl Into<String>,
        result: Result<ParseResult, Vec<Spanned<ReqlangError>>>,
    ) -> Self {
        ParseNotificationParams {
            file_id: file_id.into(),
            result,
        }
    }
}

enum ParseNotification {}

impl Notification for ParseNotification {
    type Params = ParseNotificationParams;

    const METHOD: &'static str = "reqlang/parse";
}

struct LspDiagnosis(Diagnosis);
impl Deref for LspDiagnosis {
    type Target = Diagnosis;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl From<LspDiagnosis> for Diagnostic {
    fn from(value: LspDiagnosis) -> Self {
        let range: LspDiagnosisRange = value.range.into();

        Diagnostic {
            range: range.into(),
            severity: value.severity.map(|x| LspDiagnosisSeverity(x).into()),
            message: value.message.clone(),
            ..Default::default()
        }
    }
}

struct LspDiagnosisPosition(DiagnosisPosition);
impl Deref for LspDiagnosisPosition {
    type Target = DiagnosisPosition;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl From<LspDiagnosisPosition> for Position {
    fn from(value: LspDiagnosisPosition) -> Self {
        Position {
            line: value.line,
            character: value.character,
        }
    }
}
impl From<DiagnosisPosition> for LspDiagnosisPosition {
    fn from(value: DiagnosisPosition) -> Self {
        LspDiagnosisPosition(value)
    }
}

struct LspDiagnosisRange(DiagnosisRange);
impl Deref for LspDiagnosisRange {
    type Target = DiagnosisRange;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl From<LspDiagnosisRange> for Range {
    fn from(value: LspDiagnosisRange) -> Self {
        let start: LspDiagnosisPosition = value.start.into();
        let end: LspDiagnosisPosition = value.end.into();

        Range {
            start: start.into(),
            end: end.into(),
        }
    }
}
impl From<DiagnosisRange> for LspDiagnosisRange {
    fn from(value: DiagnosisRange) -> Self {
        LspDiagnosisRange(value)
    }
}

struct LspDiagnosisSeverity(DiagnosisSeverity);
impl Deref for LspDiagnosisSeverity {
    type Target = DiagnosisSeverity;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl From<LspDiagnosisSeverity> for DiagnosticSeverity {
    fn from(value: LspDiagnosisSeverity) -> Self {
        match value.0 {
            DiagnosisSeverity::ERROR => DiagnosticSeverity::ERROR,
            DiagnosisSeverity::HINT => DiagnosticSeverity::HINT,
            DiagnosisSeverity::INFORMATION => DiagnosticSeverity::INFORMATION,
            DiagnosisSeverity::WARNING => DiagnosticSeverity::WARNING,
            _ => panic!("Invalid diagnosis severity {:?}", value.0),
        }
    }
}
impl From<DiagnosisSeverity> for LspDiagnosisSeverity {
    fn from(value: DiagnosisSeverity) -> Self {
        LspDiagnosisSeverity(value)
    }
}

fn serve() -> Result<()> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("failed to start async runtime")?
        .block_on(serve_async());

    Ok(())
}

async fn serve_async() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(Backend::new);

    Server::new(stdin, stdout, socket).serve(service).await;
}

fn main() {
    let _ = serve();
}
