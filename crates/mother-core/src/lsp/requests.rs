//! LSP request methods (document_symbols, references, definition, hover)

use std::path::Path;

use anyhow::Result;
use async_lsp::lsp_types::{
    DocumentSymbolParams, DocumentSymbolResponse, GotoDefinitionParams, GotoDefinitionResponse,
    HoverContents, HoverParams, Position, ReferenceContext, ReferenceParams,
    TextDocumentIdentifier, TextDocumentPositionParams, Url,
};
use async_lsp::LanguageServer;

use super::client::LspClient;
use super::convert::{convert_symbol_response, marked_string_to_string};
use super::types::{LspReference, LspSymbol};

impl LspClient {
    /// Get document symbols for a file
    ///
    /// # Errors
    /// Returns an error if the request fails.
    pub async fn document_symbols(&mut self, file_uri: &str) -> Result<Vec<LspSymbol>> {
        let url = Url::parse(file_uri)?;
        let symbols = self.fetch_document_symbols(&url).await?;
        Ok(convert_symbol_response(symbols))
    }

    async fn fetch_document_symbols(
        &mut self,
        url: &Url,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let params = DocumentSymbolParams {
            text_document: TextDocumentIdentifier { uri: url.clone() },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };

        tracing::debug!("Requesting document symbols for: {}", url);
        let response = self.server().document_symbol(params).await?;
        tracing::debug!("Got response for {}: {:?}", url, response.is_some());
        Ok(response)
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

        let response = self.server().references(params).await?;

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

        let response = self.server().definition(params).await?;

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

        let response = self.server().hover(params).await?;

        let content = response.and_then(|hover| match hover.contents {
            HoverContents::Scalar(marked) => Some(marked_string_to_string(marked)),
            HoverContents::Array(items) => {
                let text: Vec<String> = items.into_iter().map(marked_string_to_string).collect();
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
}
