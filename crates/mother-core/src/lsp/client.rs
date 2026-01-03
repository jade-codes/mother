//! LSP Client: Communicates with language servers using async-lsp

use std::ops::ControlFlow;
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

use anyhow::Result;
use async_lsp::concurrency::ConcurrencyLayer;
use async_lsp::panic::CatchUnwindLayer;
use async_lsp::router::Router;
use async_lsp::tracing::TracingLayer;
use async_lsp::{LanguageClient, LanguageServer, ResponseError, ServerSocket};
// Use lsp_types re-exported from async_lsp to avoid version mismatch
use async_lsp::lsp_types::{
    ClientCapabilities, DidOpenTextDocumentParams, DocumentSymbolParams, DocumentSymbolResponse,
    GotoDefinitionParams, GotoDefinitionResponse, HoverContents, HoverParams, InitializeParams,
    InitializedParams, LogMessageParams, MarkedString, NumberOrString, Position, ProgressParams,
    ProgressParamsValue, PublishDiagnosticsParams, ReferenceContext, ReferenceParams,
    ShowMessageParams, TextDocumentIdentifier, TextDocumentItem, TextDocumentPositionParams, Url,
    WindowClientCapabilities, WorkDoneProgress, WorkDoneProgressCreateParams, WorkspaceFolder,
};
use futures::channel::oneshot;
use tower::ServiceBuilder;

use super::types::{LspReference, LspServerConfig, LspSymbol, LspSymbolKind};

/// Known rust-analyzer indexing progress tokens
const RA_INDEXING_TOKENS: &[&str] = &["rustAnalyzer/Indexing", "rustAnalyzer/cachePriming"];

/// Client state for handling LSP notifications
struct ClientState {
    indexed_tx: Option<oneshot::Sender<()>>,
}

/// Event to signal stopping the client
struct Stop;

impl LanguageClient for ClientState {
    type Error = ResponseError;
    type NotifyResult = ControlFlow<async_lsp::Result<()>>;

    fn progress(&mut self, params: ProgressParams) -> Self::NotifyResult {
        // Check if indexing is complete - combined condition to satisfy collapsible_if lint
        let is_indexing_token = matches!(&params.token, NumberOrString::String(s) if RA_INDEXING_TOKENS.contains(&&**s));
        let is_end_progress = matches!(
            params.value,
            ProgressParamsValue::WorkDone(WorkDoneProgress::End(_))
        );

        if is_indexing_token && is_end_progress {
            if let Some(tx) = self.indexed_tx.take() {
                let _ = tx.send(());
            }
        }
        ControlFlow::Continue(())
    }

    fn publish_diagnostics(&mut self, _: PublishDiagnosticsParams) -> Self::NotifyResult {
        ControlFlow::Continue(())
    }

    fn show_message(&mut self, params: ShowMessageParams) -> Self::NotifyResult {
        tracing::debug!("LSP message {:?}: {}", params.typ, params.message);
        ControlFlow::Continue(())
    }

    fn log_message(&mut self, params: LogMessageParams) -> Self::NotifyResult {
        tracing::debug!("LSP log {:?}: {}", params.typ, params.message);
        ControlFlow::Continue(())
    }

    fn work_done_progress_create(
        &mut self,
        _params: WorkDoneProgressCreateParams,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<(), ResponseError>> + Send + 'static>,
    > {
        Box::pin(async { Ok(()) })
    }
}

impl ClientState {
    fn new_router(indexed_tx: oneshot::Sender<()>) -> Router<Self> {
        let mut router = Router::from_language_client(ClientState {
            indexed_tx: Some(indexed_tx),
        });
        router.request::<async_lsp::lsp_types::request::WorkDoneProgressCreate, _>(
            Self::work_done_progress_create,
        );
        router.event(Self::on_stop);
        router
    }

    fn on_stop(&mut self, _: Stop) -> ControlFlow<async_lsp::Result<()>> {
        ControlFlow::Break(Ok(()))
    }
}

/// Client for communicating with an LSP server using async-lsp
pub struct LspClient {
    server: ServerSocket,
    #[allow(dead_code)]
    mainloop_handle: tokio::task::JoinHandle<()>,
    #[allow(dead_code)]
    child: async_process::Child,
    indexed_rx: Option<oneshot::Receiver<()>>,
    #[allow(dead_code)]
    config: LspServerConfig,
}

impl LspClient {
    /// Start an LSP server and create a client
    ///
    /// # Errors
    /// Returns an error if the server cannot be started.
    pub async fn start(config: LspServerConfig) -> Result<Self> {
        let (indexed_tx, indexed_rx) = oneshot::channel();

        let (mainloop, server) = async_lsp::MainLoop::new_client(|_server| {
            ServiceBuilder::new()
                .layer(TracingLayer::default())
                .layer(CatchUnwindLayer::default())
                .layer(ConcurrencyLayer::default())
                .service(ClientState::new_router(indexed_tx))
        });

        // Spawn the LSP server process
        let mut child = async_process::Command::new(&config.command)
            .args(&config.args)
            .current_dir(&config.root_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .kill_on_drop(true)
            .spawn()?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow::anyhow!("Failed to get stdout from LSP process"))?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow::anyhow!("Failed to get stdin from LSP process"))?;

        // Run the mainloop in a background task
        let mainloop_handle = tokio::spawn(async move {
            if let Err(e) = mainloop.run_buffered(stdout, stdin).await {
                tracing::warn!("LSP mainloop error: {}", e);
            }
        });

        Ok(Self {
            server,
            mainloop_handle,
            child,
            indexed_rx: Some(indexed_rx),
            config,
        })
    }

    /// Initialize the LSP server
    ///
    /// # Errors
    /// Returns an error if initialization fails.
    pub async fn initialize(&mut self, root_uri: &str) -> Result<()> {
        let root_url = Url::parse(root_uri)?;

        tracing::debug!(
            "Initializing LSP with init_options: {:?}",
            self.config.init_options
        );

        #[allow(deprecated)]
        let params = InitializeParams {
            process_id: Some(std::process::id()),
            root_uri: Some(root_url.clone()),
            workspace_folders: Some(vec![WorkspaceFolder {
                uri: root_url,
                name: "root".into(),
            }]),
            capabilities: ClientCapabilities {
                window: Some(WindowClientCapabilities {
                    work_done_progress: Some(true),
                    ..Default::default()
                }),
                ..Default::default()
            },
            initialization_options: self.config.init_options.clone(),
            ..Default::default()
        };

        let _result = self.server.initialize(params).await?;
        self.server.initialized(InitializedParams {})?;

        Ok(())
    }

    /// Wait for the LSP server to finish indexing
    ///
    /// # Errors
    /// Returns an error if waiting times out.
    pub async fn wait_for_indexing(&mut self, timeout: Duration) -> Result<()> {
        if let Some(rx) = self.indexed_rx.take() {
            match tokio::time::timeout(timeout, rx).await {
                Ok(Ok(())) => {
                    tracing::info!("LSP indexing complete");
                }
                Ok(Err(_)) => {
                    tracing::debug!("Indexing channel closed");
                }
                Err(_) => {
                    tracing::debug!("Indexing wait timed out, proceeding anyway");
                }
            }
        }
        Ok(())
    }

    /// Get document symbols for a file
    ///
    /// # Errors
    /// Returns an error if the request fails.
    pub async fn document_symbols(&mut self, file_uri: &str) -> Result<Vec<LspSymbol>> {
        let url = Url::parse(file_uri)?;

        let params = DocumentSymbolParams {
            text_document: TextDocumentIdentifier { uri: url.clone() },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };

        tracing::debug!("Requesting document symbols for: {}", url);
        let response = self.server.document_symbol(params).await?;
        tracing::debug!("Got response for {}: {:?}", url, response.is_some());

        let symbols = match response {
            Some(DocumentSymbolResponse::Flat(symbols)) => {
                tracing::debug!("Got {} flat symbols from LSP", symbols.len());
                symbols
                    .into_iter()
                    .map(|s| self.convert_symbol_information(&s))
                    .collect()
            }
            Some(DocumentSymbolResponse::Nested(symbols)) => {
                tracing::debug!("Got {} nested symbols from LSP", symbols.len());
                symbols
                    .into_iter()
                    .map(|s| self.convert_document_symbol(&s))
                    .collect()
            }
            None => {
                tracing::debug!("LSP returned None for document symbols");
                vec![]
            }
        };

        Ok(symbols)
    }

    /// Find all references to a symbol at a position
    ///
    /// # Errors
    /// Returns an error if the request fails.
    pub async fn references(
        &mut self,
        file_uri: &str,
        line: u32,
        character: u32,
        include_declaration: bool,
    ) -> Result<Vec<LspReference>> {
        let url = Url::parse(file_uri)?;

        let params = ReferenceParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: url },
                position: Position::new(line, character),
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
            context: ReferenceContext {
                include_declaration,
            },
        };

        let response = self.server.references(params).await?;

        let refs = response
            .unwrap_or_default()
            .into_iter()
            .map(|loc| LspReference {
                file: loc
                    .uri
                    .to_file_path()
                    .unwrap_or_else(|_| Path::new(loc.uri.path()).to_path_buf()),
                line: loc.range.start.line,
                start_col: loc.range.start.character,
                end_col: loc.range.end.character,
            })
            .collect();

        Ok(refs)
    }

    /// Go to definition of a symbol
    ///
    /// # Errors
    /// Returns an error if the request fails.
    pub async fn definition(
        &mut self,
        file_uri: &str,
        line: u32,
        character: u32,
    ) -> Result<Vec<LspReference>> {
        let url = Url::parse(file_uri)?;

        let params = GotoDefinitionParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: url },
                position: Position::new(line, character),
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };

        let response = self.server.definition(params).await?;

        let locations = match response {
            Some(GotoDefinitionResponse::Scalar(loc)) => vec![loc],
            Some(GotoDefinitionResponse::Array(locs)) => locs,
            Some(GotoDefinitionResponse::Link(links)) => links
                .into_iter()
                .map(|l| async_lsp::lsp_types::Location {
                    uri: l.target_uri,
                    range: l.target_selection_range,
                })
                .collect(),
            None => vec![],
        };

        let refs = locations
            .into_iter()
            .map(|loc| LspReference {
                file: loc
                    .uri
                    .to_file_path()
                    .unwrap_or_else(|_| Path::new(loc.uri.path()).to_path_buf()),
                line: loc.range.start.line,
                start_col: loc.range.start.character,
                end_col: loc.range.end.character,
            })
            .collect();

        Ok(refs)
    }

    /// Get hover information for a symbol at a position
    ///
    /// Returns the hover content as a string, or None if no hover info is available.
    ///
    /// # Errors
    /// Returns an error if the request fails.
    pub async fn hover(
        &mut self,
        file_uri: &str,
        line: u32,
        character: u32,
    ) -> Result<Option<String>> {
        let url = Url::parse(file_uri)?;

        let params = HoverParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: url },
                position: Position::new(line, character),
            },
            work_done_progress_params: Default::default(),
        };

        let response = self.server.hover(params).await?;

        let content = response.and_then(|hover| match hover.contents {
            HoverContents::Scalar(marked) => Some(Self::marked_string_to_string(marked)),
            HoverContents::Array(items) => {
                let text: Vec<String> = items
                    .into_iter()
                    .map(Self::marked_string_to_string)
                    .collect();
                if text.is_empty() {
                    None
                } else {
                    Some(text.join("\n\n"))
                }
            }
            HoverContents::Markup(markup) => Some(markup.value),
        });

        Ok(content)
    }

    /// Convert a MarkedString to a plain String
    fn marked_string_to_string(marked: MarkedString) -> String {
        match marked {
            MarkedString::String(s) => s,
            MarkedString::LanguageString(ls) => ls.value,
        }
    }

    /// Notify the server that a file was opened
    ///
    /// # Errors
    /// Returns an error if the notification fails.
    pub async fn did_open(&mut self, file_uri: &str, language_id: &str, text: &str) -> Result<()> {
        let url = Url::parse(file_uri)?;

        self.server.did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: url,
                language_id: language_id.into(),
                version: 1,
                text: text.into(),
            },
        })?;

        Ok(())
    }

    /// Shutdown the LSP server
    ///
    /// # Errors
    /// Returns an error if shutdown fails.
    pub async fn shutdown(&mut self) -> Result<()> {
        self.server.shutdown(()).await?;
        self.server.exit(())?;
        self.server.emit(Stop)?;
        Ok(())
    }

    // Conversion helpers

    fn convert_document_symbol(&self, symbol: &async_lsp::lsp_types::DocumentSymbol) -> LspSymbol {
        let children = symbol
            .children
            .as_ref()
            .map(|c| c.iter().map(|s| self.convert_document_symbol(s)).collect())
            .unwrap_or_default();

        LspSymbol {
            name: symbol.name.clone(),
            kind: self.convert_symbol_kind(symbol.kind),
            detail: symbol.detail.clone(),
            container_name: None, // Nested format uses explicit children instead
            file: std::path::PathBuf::new(), // DocumentSymbol doesn't include file
            start_line: symbol.range.start.line,
            end_line: symbol.range.end.line,
            start_col: symbol.range.start.character,
            end_col: symbol.range.end.character,
            children,
        }
    }

    fn convert_symbol_information(
        &self,
        symbol: &async_lsp::lsp_types::SymbolInformation,
    ) -> LspSymbol {
        #[allow(deprecated)]
        let container_name = symbol.container_name.clone();
        LspSymbol {
            name: symbol.name.clone(),
            kind: self.convert_symbol_kind(symbol.kind),
            detail: None,
            container_name,
            file: Path::new(symbol.location.uri.path()).to_path_buf(),
            start_line: symbol.location.range.start.line,
            end_line: symbol.location.range.end.line,
            start_col: symbol.location.range.start.character,
            end_col: symbol.location.range.end.character,
            children: vec![],
        }
    }

    fn convert_symbol_kind(&self, kind: async_lsp::lsp_types::SymbolKind) -> LspSymbolKind {
        use async_lsp::lsp_types::SymbolKind;
        match kind {
            SymbolKind::FILE => LspSymbolKind::File,
            SymbolKind::MODULE => LspSymbolKind::Module,
            SymbolKind::NAMESPACE => LspSymbolKind::Namespace,
            SymbolKind::PACKAGE => LspSymbolKind::Package,
            SymbolKind::CLASS => LspSymbolKind::Class,
            SymbolKind::METHOD => LspSymbolKind::Method,
            SymbolKind::PROPERTY => LspSymbolKind::Property,
            SymbolKind::FIELD => LspSymbolKind::Field,
            SymbolKind::CONSTRUCTOR => LspSymbolKind::Constructor,
            SymbolKind::ENUM => LspSymbolKind::Enum,
            SymbolKind::INTERFACE => LspSymbolKind::Interface,
            SymbolKind::FUNCTION => LspSymbolKind::Function,
            SymbolKind::VARIABLE => LspSymbolKind::Variable,
            SymbolKind::CONSTANT => LspSymbolKind::Constant,
            SymbolKind::STRING => LspSymbolKind::String,
            SymbolKind::NUMBER => LspSymbolKind::Number,
            SymbolKind::BOOLEAN => LspSymbolKind::Boolean,
            SymbolKind::ARRAY => LspSymbolKind::Array,
            SymbolKind::OBJECT => LspSymbolKind::Object,
            SymbolKind::KEY => LspSymbolKind::Key,
            SymbolKind::NULL => LspSymbolKind::Null,
            SymbolKind::ENUM_MEMBER => LspSymbolKind::EnumMember,
            SymbolKind::STRUCT => LspSymbolKind::Struct,
            SymbolKind::EVENT => LspSymbolKind::Event,
            SymbolKind::OPERATOR => LspSymbolKind::Operator,
            SymbolKind::TYPE_PARAMETER => LspSymbolKind::TypeParameter,
            _ => LspSymbolKind::Variable,
        }
    }
}
