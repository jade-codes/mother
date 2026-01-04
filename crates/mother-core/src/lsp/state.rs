//! LSP client state and notification handling

use std::ops::ControlFlow;

use async_lsp::lsp_types::{
    LogMessageParams, NumberOrString, ProgressParams, ProgressParamsValue,
    PublishDiagnosticsParams, ShowMessageParams, WorkDoneProgress, WorkDoneProgressCreateParams,
};
use async_lsp::router::Router;
use async_lsp::{LanguageClient, ResponseError};
use futures::channel::oneshot;

/// Known rust-analyzer indexing progress tokens
const RA_INDEXING_TOKENS: &[&str] = &["rustAnalyzer/Indexing", "rustAnalyzer/cachePriming"];

/// Client state for handling LSP notifications
pub(super) struct ClientState {
    indexed_tx: Option<oneshot::Sender<()>>,
}

impl ClientState {
    /// Create a new ClientState for testing
    #[cfg(test)]
    pub(super) fn new_for_test(indexed_tx: Option<oneshot::Sender<()>>) -> Self {
        ClientState { indexed_tx }
    }
}

/// Event to signal stopping the client
pub(super) struct Stop;

impl LanguageClient for ClientState {
    type Error = ResponseError;
    type NotifyResult = ControlFlow<async_lsp::Result<()>>;

    fn progress(&mut self, params: ProgressParams) -> Self::NotifyResult {
        // Check if indexing is complete
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
    pub fn new_router(indexed_tx: oneshot::Sender<()>) -> Router<Self> {
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
