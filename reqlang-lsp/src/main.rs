use std::collections::HashMap;
use std::ops::Deref;

use anyhow::{Context, Result};
use reqlang::{
    assert_response::assert_response,
    diagnostics::{
        get_diagnostics, Diagnosis, DiagnosisPosition, DiagnosisRange, DiagnosisSeverity,
    },
    export, parse, template, Ast, Fetch, HttpRequestFetcher, ParseResult, ReqlangError,
    RequestFormat, RequestParamsFromClient, Spanned,
};
use reqlang_lsp::{generate_semantic_tokens, LEGEND_TYPE};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::Mutex;
use tower_lsp::lsp_types::{
    notification::Notification, SemanticTokens, SemanticTokensFullOptions, SemanticTokensLegend,
    SemanticTokensOptions, SemanticTokensParams, SemanticTokensResult, WorkDoneProgressOptions,
};
use tower_lsp::lsp_types::{
    Diagnostic, DiagnosticSeverity, DidChangeTextDocumentParams, DidOpenTextDocumentParams,
    DidSaveTextDocumentParams, ExecuteCommandOptions, ExecuteCommandParams, InitializeParams,
    InitializeResult, MessageType, Position, Range, SaveOptions, ServerCapabilities, ServerInfo,
    TextDocumentSyncKind, TextDocumentSyncOptions,
};
use tower_lsp::{jsonrpc::Result as RpcResult, lsp_types::SemanticTokensServerCapabilities};
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

    async fn parse_file_for_client(&self, uri: &Url, source: &str) {
        let ast = Ast::new(source);

        let result = match parse(&ast) {
            Ok(parsed_request_file) => {
                // Clear diagnostics on the client
                self.client
                    .publish_diagnostics(uri.clone(), vec![], None)
                    .await;

                Ok(parsed_request_file.into())
            }
            Err(errs) => {
                let diagnostics = get_diagnostics(&errs, source);

                // Log error diagnostics to client
                // This is mostly for debugging
                self.client
                    .log_message(
                        MessageType::ERROR,
                        format!(
                            "{} errors parsing file '{uri}':\n{diagnostics:#?}",
                            diagnostics.len()
                        ),
                    )
                    .await;

                // Send error diagnostics to client
                self.client
                    .publish_diagnostics(
                        uri.clone(),
                        diagnostics
                            .iter()
                            .map(|x| (LspDiagnosis((*x).clone())).into())
                            .collect(),
                        None,
                    )
                    .await;

                Err(errs)
            }
        };

        // Send a notification to the client with the results of the parse
        self.client
            .send_notification::<ParseNotification>(ParseNotificationParams::new(
                uri.clone(),
                result,
            ))
            .await;
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> RpcResult<InitializeResult> {
        let version = env!("CARGO_PKG_VERSION").to_string();

        let initial_log = format!("Reqlang Language Server (v{}) running...", version);

        eprintln!("{initial_log}");

        self.client
            .log_message(MessageType::INFO, initial_log)
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
                semantic_tokens_provider: Some(
                    SemanticTokensServerCapabilities::SemanticTokensOptions(
                        SemanticTokensOptions {
                            work_done_progress_options: WorkDoneProgressOptions {
                                work_done_progress: Some(true),
                            },
                            legend: SemanticTokensLegend {
                                token_types: LEGEND_TYPE.into(),
                                token_modifiers: vec![],
                            },
                            range: Some(true),
                            full: Some(SemanticTokensFullOptions::Delta { delta: Some(true) }),
                        },
                    ),
                ),
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

        self.parse_file_for_client(&uri, &source).await;

        let mut file_texts = self.file_texts.lock().await;
        file_texts.insert(uri.clone(), source.clone());
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        let source = &params.content_changes.first().unwrap().text;

        self.parse_file_for_client(&uri, source).await;
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        let uri = params.text_document.uri;
        let source = params.text.unwrap_or_default();

        self.parse_file_for_client(&uri, &source).await;

        let mut file_texts = self.file_texts.lock().await;
        file_texts.insert(uri.clone(), source.clone());
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> RpcResult<Option<SemanticTokensResult>> {
        let uri = params.text_document.uri;

        let file_texts = self.file_texts.lock().await;
        let source = file_texts.get(&uri).expect("Should be present");
        let ast = Ast::new(source);

        let semantic_tokens = generate_semantic_tokens(&ast, source);

        self.client
            .log_message(
                MessageType::INFO,
                format!("SEMANTIC TOKENS: {:#?}", semantic_tokens),
            )
            .await;

        Ok(Some(SemanticTokensResult::Tokens(SemanticTokens {
            result_id: None,
            data: semantic_tokens,
        })))
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
                from_client_params.env.as_deref(),
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

            if let Some(expected_response) = reqfile.response {
                if let Err(diffs) = assert_response(&expected_response, &response) {
                    self.client
                        .log_message(
                            MessageType::WARNING,
                            format!("Differences found in the response:\n{diffs}"),
                        )
                        .await;
                }
            };

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
            let env = from_client_params.env.as_deref();
            let mut provider = HashMap::new();

            provider.insert("env".to_string(), env.unwrap_or_default().to_string());

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
    env: Option<String>,
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
            .map(|x| x.to_string());

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

/// Notification sent to the client when a file has been parsed
///
/// Uses the `reqlang/parse` event
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
