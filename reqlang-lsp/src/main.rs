use anyhow::{Context, Result};
use tower_lsp::{
    jsonrpc,
    lsp_types::{
        DidChangeTextDocumentParams, DidOpenTextDocumentParams, InitializeParams, InitializeResult,
        MessageType, ServerCapabilities, ServerInfo, TextDocumentSyncKind,
    },
    Client, LanguageServer, LspService, Server,
};

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
        let _source = &params.text_document.text;
        let _uri = params.text_document.uri;
        let _version = Some(params.text_document.version);
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let _source = &params.content_changes.first().unwrap().text;
        let _uri = params.text_document.uri;
        let _version = Some(params.text_document.version);
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
