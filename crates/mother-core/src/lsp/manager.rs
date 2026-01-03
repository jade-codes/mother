//! LSP Server Manager: Manages multiple LSP servers

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::Result;

use super::client::LspClient;
use super::types::LspServerConfig;
use crate::scanner::Language;

/// Default LSP server commands for each language
pub struct LspServerDefaults;

impl LspServerDefaults {
    /// Get the default server config for a language
    #[must_use]
    pub fn for_language(language: Language, root_path: &Path) -> LspServerConfig {
        let root = root_path.to_path_buf();

        match language {
            Language::Rust => LspServerConfig {
                language,
                command: "rust-analyzer".to_string(),
                args: vec![],
                root_path: root,
                init_options: None,
            },
            Language::Python => LspServerConfig {
                language,
                command: "pyright-langserver".to_string(),
                args: vec!["--stdio".to_string()],
                root_path: root,
                init_options: None,
            },
            Language::TypeScript | Language::JavaScript => LspServerConfig {
                language,
                command: "typescript-language-server".to_string(),
                args: vec!["--stdio".to_string()],
                root_path: root,
                init_options: None,
            },
            Language::SysML | Language::KerML => {
                // Find sysml.library in the project or use system default
                let stdlib_path = root
                    .join("crates/syster-base/sysml.library")
                    .canonicalize()
                    .ok()
                    .or_else(|| {
                        // Try relative to cargo install location
                        std::env::current_exe()
                            .ok()
                            .and_then(|exe| exe.parent().map(|p| p.to_path_buf()))
                            .and_then(|bin| bin.parent().map(|p| p.join("lib/sysml.library")))
                    });

                let init_options = if let Some(path) = stdlib_path {
                    Some(serde_json::json!({
                        "stdlibEnabled": true,
                        "stdlibPath": path.display().to_string()
                    }))
                } else {
                    Some(serde_json::json!({
                        "stdlibEnabled": true
                    }))
                };

                LspServerConfig {
                    language,
                    command: "syster-lsp".to_string(),
                    args: vec![],
                    root_path: root,
                    init_options,
                }
            }
        }
    }
}

/// Manages multiple LSP server instances
pub struct LspServerManager {
    root_path: PathBuf,
    clients: HashMap<Language, LspClient>,
    custom_configs: HashMap<Language, LspServerConfig>,
}

impl LspServerManager {
    /// Create a new server manager
    #[must_use]
    pub fn new(root_path: impl Into<PathBuf>) -> Self {
        Self {
            root_path: root_path.into(),
            clients: HashMap::new(),
            custom_configs: HashMap::new(),
        }
    }

    /// Register a custom server config for a language
    pub fn register_server(&mut self, config: LspServerConfig) {
        self.custom_configs.insert(config.language, config);
    }

    /// Get or start an LSP client for a language
    ///
    /// # Errors
    /// Returns an error if the server cannot be started.
    pub async fn get_client(&mut self, language: Language) -> Result<&mut LspClient> {
        if !self.clients.contains_key(&language) {
            let config = self
                .custom_configs
                .get(&language)
                .cloned()
                .unwrap_or_else(|| LspServerDefaults::for_language(language, &self.root_path));

            let mut client = LspClient::start(config).await?;

            let root_uri = format!("file://{}", self.root_path.display());
            client.initialize(&root_uri).await?;

            // Wait for the LSP server to finish initial indexing
            // This uses async-lsp's proper notification handling
            client.wait_for_indexing(Duration::from_secs(30)).await?;

            self.clients.insert(language, client);
        }

        self.clients
            .get_mut(&language)
            .ok_or_else(|| anyhow::anyhow!("Failed to get LSP client for {:?}", language))
    }

    /// Shutdown all LSP servers
    ///
    /// # Errors
    /// Returns an error if any server fails to shutdown.
    pub async fn shutdown_all(&mut self) -> Result<()> {
        for (_, mut client) in self.clients.drain() {
            let _ = client.shutdown().await;
        }
        Ok(())
    }
}
