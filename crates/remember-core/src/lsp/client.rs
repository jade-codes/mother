//! LSP Client: Communicates with language servers

use std::collections::HashMap;
use std::path::Path;
use std::process::Stdio;

use anyhow::Result;
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

use super::types::{LspReference, LspServerConfig, LspSymbol, LspSymbolKind};

/// Client for communicating with an LSP server
pub struct LspClient {
    process: Child,
    request_id: Mutex<i64>,
    #[allow(dead_code)]
    config: LspServerConfig,
}

impl LspClient {
    /// Start an LSP server and create a client
    ///
    /// # Errors
    /// Returns an error if the server cannot be started.
    pub async fn start(config: LspServerConfig) -> Result<Self> {
        let mut cmd = Command::new(&config.command);
        cmd.args(&config.args)
            .current_dir(&config.root_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let process = cmd.spawn()?;

        let client = Self {
            process,
            request_id: Mutex::new(0),
            config,
        };

        Ok(client)
    }

    /// Initialize the LSP server
    ///
    /// # Errors
    /// Returns an error if initialization fails.
    pub async fn initialize(&mut self, root_uri: &str) -> Result<()> {
        let params = json!({
            "processId": std::process::id(),
            "rootUri": root_uri,
            "capabilities": {
                "textDocument": {
                    "documentSymbol": {
                        "hierarchicalDocumentSymbolSupport": true
                    },
                    "references": {},
                    "definition": {},
                    "implementation": {}
                },
                "workspace": {
                    "workspaceFolders": true
                }
            }
        });

        self.send_request("initialize", params).await?;
        self.send_notification("initialized", json!({})).await?;

        Ok(())
    }

    /// Get document symbols for a file
    ///
    /// # Errors
    /// Returns an error if the request fails.
    pub async fn document_symbols(&mut self, file_uri: &str) -> Result<Vec<LspSymbol>> {
        let params = json!({
            "textDocument": {
                "uri": file_uri
            }
        });

        let response = self.send_request("textDocument/documentSymbol", params).await?;
        
        // Parse the response into LspSymbol
        let symbols = self.parse_document_symbols(&response)?;
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
        let params = json!({
            "textDocument": {
                "uri": file_uri
            },
            "position": {
                "line": line,
                "character": character
            },
            "context": {
                "includeDeclaration": include_declaration
            }
        });

        let response = self.send_request("textDocument/references", params).await?;
        let references = self.parse_references(&response)?;
        Ok(references)
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
        let params = json!({
            "textDocument": {
                "uri": file_uri
            },
            "position": {
                "line": line,
                "character": character
            }
        });

        let response = self.send_request("textDocument/definition", params).await?;
        let locations = self.parse_references(&response)?;
        Ok(locations)
    }

    /// Notify the server that a file was opened
    ///
    /// # Errors
    /// Returns an error if the notification fails.
    pub async fn did_open(&mut self, file_uri: &str, language_id: &str, text: &str) -> Result<()> {
        let params = json!({
            "textDocument": {
                "uri": file_uri,
                "languageId": language_id,
                "version": 1,
                "text": text
            }
        });

        self.send_notification("textDocument/didOpen", params).await
    }

    /// Shutdown the LSP server
    ///
    /// # Errors
    /// Returns an error if shutdown fails.
    pub async fn shutdown(&mut self) -> Result<()> {
        self.send_request("shutdown", json!(null)).await?;
        self.send_notification("exit", json!(null)).await?;
        Ok(())
    }

    // Internal methods

    async fn send_request(&mut self, method: &str, params: Value) -> Result<Value> {
        let id = {
            let mut id_guard = self.request_id.lock().await;
            *id_guard += 1;
            *id_guard
        };

        let request = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        });

        self.send_message(&request).await?;
        self.read_response(id).await
    }

    async fn send_notification(&mut self, method: &str, params: Value) -> Result<()> {
        let notification = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        });

        self.send_message(&notification).await
    }

    async fn send_message(&mut self, message: &Value) -> Result<()> {
        let content = serde_json::to_string(message)?;
        let header = format!("Content-Length: {}\r\n\r\n", content.len());

        if let Some(stdin) = self.process.stdin.as_mut() {
            stdin.write_all(header.as_bytes()).await?;
            stdin.write_all(content.as_bytes()).await?;
            stdin.flush().await?;
        }

        Ok(())
    }

    async fn read_response(&mut self, expected_id: i64) -> Result<Value> {
        let stdout = self.process.stdout.as_mut()
            .ok_or_else(|| anyhow::anyhow!("No stdout"))?;
        let mut reader = BufReader::new(stdout);

        // Read headers
        let mut content_length = 0;
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).await?;
            
            if line == "\r\n" || line == "\n" {
                break;
            }
            
            if let Some(len_str) = line.strip_prefix("Content-Length: ") {
                content_length = len_str.trim().parse()?;
            }
        }

        // Read content
        let mut content = vec![0u8; content_length];
        tokio::io::AsyncReadExt::read_exact(&mut reader, &mut content).await?;

        let response: Value = serde_json::from_slice(&content)?;

        // Check if this is our response
        if response.get("id").and_then(|v| v.as_i64()) == Some(expected_id) {
            if let Some(result) = response.get("result") {
                return Ok(result.clone());
            }
            if let Some(error) = response.get("error") {
                anyhow::bail!("LSP error: {}", error);
            }
        }

        Ok(response)
    }

    fn parse_document_symbols(&self, response: &Value) -> Result<Vec<LspSymbol>> {
        let mut symbols = Vec::new();

        if let Some(array) = response.as_array() {
            for item in array {
                if let Some(symbol) = self.parse_symbol(item) {
                    symbols.push(symbol);
                }
            }
        }

        Ok(symbols)
    }

    fn parse_symbol(&self, value: &Value) -> Option<LspSymbol> {
        let name = value.get("name")?.as_str()?.to_string();
        let kind_num = value.get("kind")?.as_u64()?;
        let kind = self.symbol_kind_from_number(kind_num as u32);

        let detail = value.get("detail").and_then(|d| d.as_str()).map(String::from);

        // Handle both DocumentSymbol (with range) and SymbolInformation (with location)
        let (file, start_line, end_line, start_col, end_col) = if let Some(range) = value.get("range") {
            let start = range.get("start")?;
            let end = range.get("end")?;
            (
                std::path::PathBuf::new(), // DocumentSymbol doesn't have file
                start.get("line")?.as_u64()? as u32,
                end.get("line")?.as_u64()? as u32,
                start.get("character")?.as_u64()? as u32,
                end.get("character")?.as_u64()? as u32,
            )
        } else if let Some(location) = value.get("location") {
            let uri = location.get("uri")?.as_str()?;
            let range = location.get("range")?;
            let start = range.get("start")?;
            let end = range.get("end")?;
            (
                Path::new(uri.strip_prefix("file://").unwrap_or(uri)).to_path_buf(),
                start.get("line")?.as_u64()? as u32,
                end.get("line")?.as_u64()? as u32,
                start.get("character")?.as_u64()? as u32,
                end.get("character")?.as_u64()? as u32,
            )
        } else {
            return None;
        };

        // Parse children recursively
        let children = value
            .get("children")
            .and_then(|c| c.as_array())
            .map(|arr| arr.iter().filter_map(|v| self.parse_symbol(v)).collect())
            .unwrap_or_default();

        Some(LspSymbol {
            name,
            kind,
            detail,
            file,
            start_line,
            end_line,
            start_col,
            end_col,
            children,
        })
    }

    fn parse_references(&self, response: &Value) -> Result<Vec<LspReference>> {
        let mut refs = Vec::new();

        if let Some(array) = response.as_array() {
            for item in array {
                if let Some(r) = self.parse_location(item) {
                    refs.push(r);
                }
            }
        } else if response.is_object() {
            // Single location
            if let Some(r) = self.parse_location(response) {
                refs.push(r);
            }
        }

        Ok(refs)
    }

    fn parse_location(&self, value: &Value) -> Option<LspReference> {
        let uri = value.get("uri")?.as_str()?;
        let range = value.get("range")?;
        let start = range.get("start")?;
        let end = range.get("end")?;

        Some(LspReference {
            file: Path::new(uri.strip_prefix("file://").unwrap_or(uri)).to_path_buf(),
            line: start.get("line")?.as_u64()? as u32,
            start_col: start.get("character")?.as_u64()? as u32,
            end_col: end.get("character")?.as_u64()? as u32,
            is_definition: false,
        })
    }

    fn symbol_kind_from_number(&self, kind: u32) -> LspSymbolKind {
        match kind {
            1 => LspSymbolKind::File,
            2 => LspSymbolKind::Module,
            3 => LspSymbolKind::Namespace,
            4 => LspSymbolKind::Package,
            5 => LspSymbolKind::Class,
            6 => LspSymbolKind::Method,
            7 => LspSymbolKind::Property,
            8 => LspSymbolKind::Field,
            9 => LspSymbolKind::Constructor,
            10 => LspSymbolKind::Enum,
            11 => LspSymbolKind::Interface,
            12 => LspSymbolKind::Function,
            13 => LspSymbolKind::Variable,
            14 => LspSymbolKind::Constant,
            15 => LspSymbolKind::String,
            16 => LspSymbolKind::Number,
            17 => LspSymbolKind::Boolean,
            18 => LspSymbolKind::Array,
            19 => LspSymbolKind::Object,
            20 => LspSymbolKind::Key,
            21 => LspSymbolKind::Null,
            22 => LspSymbolKind::EnumMember,
            23 => LspSymbolKind::Struct,
            24 => LspSymbolKind::Event,
            25 => LspSymbolKind::Operator,
            26 => LspSymbolKind::TypeParameter,
            _ => LspSymbolKind::Variable,
        }
    }
}
