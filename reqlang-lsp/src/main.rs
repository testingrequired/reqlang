use std::collections::HashMap;
use std::ops::Deref;

use anyhow::{Context, Result};
use reqlang::{
    assert_response::assert_response,
    diagnostics::{
        get_diagnostics, Diagnosis, DiagnosisPosition, DiagnosisRange, DiagnosisSeverity,
    },
    export, parse, template, Fetch, HttpRequestFetcher, ParseResult, ReqlangError, RequestFormat,
    RequestParamsFromClient, Spanned,
};
use reqwest::Url;
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

        let result: Result<ParseResult, _> = parse(&source).map(|reqfile| reqfile.into());

        if let Err(errs) = &result {
            self.client
                .publish_diagnostics(
                    uri.clone(),
                    get_diagnostics(errs, &source)
                        .iter()
                        .map(|x| (LspDiagnosis((*x).clone())).into())
                        .collect(),
                    Some(params.text_document.version),
                )
                .await;
        }

        self.client
            .send_notification::<ParseNotification>(ParseNotificationParams::new(
                uri.clone(),
                result,
            ))
            .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        let source = &params.content_changes.first().unwrap().text;

        let result: Result<ParseResult, _> = parse(source).map(|reqfile| reqfile.into());

        if let Err(errs) = &result {
            self.client
                .publish_diagnostics(
                    uri.clone(),
                    get_diagnostics(errs, source)
                        .iter()
                        .map(|x| (LspDiagnosis((*x).clone())).into())
                        .collect(),
                    Some(params.text_document.version),
                )
                .await;
        }
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
        self.client
            .log_message(
                MessageType::INFO,
                format!("Executing command: {}", params.command.as_str()),
            )
            .await;

        if params.command.as_str() == "reqlang.executeRequest" {
            let from_client_params_value = params.arguments.first().expect("Should be present");

            self.client
                .log_message(MessageType::INFO, format!("{:?}", from_client_params_value))
                .await;

            // Get parsed params from JSON `Value`
            let from_client_params =
                Into::<RequestParamsFromClient>::into(from_client_params_value.clone());

            let reqfile = template(
                &from_client_params.reqfile,
                &from_client_params.env,
                &from_client_params.prompts,
                &from_client_params.secrets,
                &Default::default(),
            )
            .expect("Should get templated request file");

            let response = Into::<HttpRequestFetcher>::into(from_client_params)
                .fetch()
                .await
                .expect("Request should have succeeded");

            self.client
                .log_message(
                    MessageType::WARNING,
                    format!("Expected response:\n{:?}", reqfile.response),
                )
                .await;

            self.client
                .log_message(
                    MessageType::WARNING,
                    format!("Actual response:\n{:?}", response),
                )
                .await;

            let diff = reqfile
                .response
                .and_then(|expected_response| assert_response(&expected_response, &response).err())
                .unwrap_or_default();

            self.client
                .log_message(
                    MessageType::WARNING,
                    format!("Differences found in the response:\n{:?}", diff),
                )
                .await;

            return Ok(Some(
                serde_json::to_string(&response)
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
                &provider,
            )
            .expect("Should have templated");

            let exported = export(&templated_reqfile.request, from_client_params.format);

            return Ok(Some(exported.into()));
        }

        self.client
            .log_message(MessageType::INFO, "command executed!")
            .await;

        Ok(Some(Value::String("DONE!".to_string())))
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
    format: RequestFormat,
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
