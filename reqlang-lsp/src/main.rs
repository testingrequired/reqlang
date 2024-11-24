use std::ops::Deref;

use anyhow::{Context, Result};
use reqlang::diagnostics::{
    Diagnoser, Diagnosis, DiagnosisPosition, DiagnosisRange, DiagnosisSeverity,
};
use reqlang::errors::ReqlangError;
use reqlang::{parse, Request, Spanned, UnresolvedRequestFile};
use serde::{Deserialize, Serialize};
use tower_lsp::lsp_types::notification::Notification;
use tower_lsp::lsp_types::{
    Diagnostic, DiagnosticSeverity, DidChangeTextDocumentParams, DidOpenTextDocumentParams,
    DidSaveTextDocumentParams, InitializeParams, InitializeResult, MessageType, Position, Range,
    SaveOptions, ServerCapabilities, ServerInfo, TextDocumentSyncKind, TextDocumentSyncOptions,
};
use tower_lsp::{jsonrpc, Client, LanguageServer, LspService, Server};

struct Backend {
    client: Client,
}

impl Backend {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> jsonrpc::Result<InitializeResult> {
        let version = env!("CARGO_PKG_VERSION").to_string();

        self.client
            .log_message(
                MessageType::INFO,
                format!("Reqlang Language Server (v{}) running...", version),
            )
            .await;

        Ok(InitializeResult {
            capabilities: ServerCapabilities {
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

    async fn shutdown(&self) -> jsonrpc::Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let source = params.text_document.text;

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

        let result: Result<ParseResult, Vec<Spanned<ReqlangError>>> =
            parse(&text).map(|unresolved_reqfile| {
                let result: ParseResult = unresolved_reqfile.into();

                result
            });

        self.client
            .send_notification::<ParseNotification>(ParseNotificationParams::new(uri, result))
            .await;
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
            request: value.request.0,
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
    request: Request,
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
