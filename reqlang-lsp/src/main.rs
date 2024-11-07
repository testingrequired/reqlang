use std::ops::Deref;

use anyhow::{Context, Result};
use reqlang::diagnostics::{
    Diagnoser, Diagnosis, DiagnosisPosition, DiagnosisRange, DiagnosisSeverity,
};
use tower_lsp::lsp_types::{
    Diagnostic, DiagnosticSeverity, DidChangeTextDocumentParams, DidOpenTextDocumentParams,
    InitializeParams, InitializeResult, MessageType, Position, Range, ServerCapabilities,
    ServerInfo, TextDocumentSyncKind,
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
                text_document_sync: Some(TextDocumentSyncKind::FULL.into()),
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
        let source = &params.text_document.text;
        let uri = params.text_document.uri;
        let version = Some(params.text_document.version);
        let diagnostics = Diagnoser::get_diagnostics(source);
        self.client
            .publish_diagnostics(
                uri,
                diagnostics
                    .iter()
                    .map(|x| (LspDiagnosis((*x).clone())).into())
                    .collect(),
                version,
            )
            .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let source = &params.content_changes.first().unwrap().text;
        let uri = params.text_document.uri;
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
