//! LSP Client: Core struct and lifecycle management

use std::process::Stdio;
use std::time::Duration;

use anyhow::Result;
use async_lsp::concurrency::ConcurrencyLayer;
use async_lsp::lsp_types::{
    ClientCapabilities, DidOpenTextDocumentParams, InitializeParams, InitializedParams,
    TextDocumentItem, Url, WindowClientCapabilities, WorkspaceFolder,
};
use async_lsp::panic::CatchUnwindLayer;
use async_lsp::tracing::TracingLayer;
use async_lsp::{LanguageServer, ServerSocket};
use futures::channel::oneshot;
use tower::ServiceBuilder;

use super::state::{ClientState, Stop};
use super::types::LspServerConfig;

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

    /// Get mutable access to the server socket (for requests module)
    pub(super) fn server(&mut self) -> &mut ServerSocket {
        &mut self.server
    }
}
