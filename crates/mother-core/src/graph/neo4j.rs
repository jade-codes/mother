//! Neo4j client for graph storage

use std::sync::Arc;

use neo4rs::{ConfigBuilder, Graph, Query};
use thiserror::Error;

use super::model::{Edge, SymbolNode};
use crate::version::ScanRun;

/// Errors that can occur during Neo4j operations
#[derive(Debug, Error)]
pub enum Neo4jError {
    #[error("Connection error: {0}")]
    Connection(String),

    #[error("Query error: {0}")]
    Query(String),

    #[error("Neo4j error: {0}")]
    Neo4j(#[from] neo4rs::Error),
}

/// Configuration for Neo4j connection
#[derive(Debug, Clone)]
pub struct Neo4jConfig {
    pub uri: String,
    pub user: String,
    pub password: String,
    pub database: Option<String>,
}

impl Neo4jConfig {
    /// Create a new Neo4j configuration
    #[must_use]
    pub fn new(
        uri: impl Into<String>,
        user: impl Into<String>,
        password: impl Into<String>,
    ) -> Self {
        Self {
            uri: uri.into(),
            user: user.into(),
            password: password.into(),
            database: None,
        }
    }

    /// Set the database name
    #[must_use]
    pub fn with_database(mut self, database: impl Into<String>) -> Self {
        self.database = Some(database.into());
        self
    }
}

/// Client for interacting with Neo4j
pub struct Neo4jClient {
    graph: Arc<Graph>,
}

impl Neo4jClient {
    /// Connect to Neo4j
    ///
    /// # Errors
    /// Returns an error if the connection fails.
    pub async fn connect(config: &Neo4jConfig) -> Result<Self, Neo4jError> {
        let mut builder = ConfigBuilder::default()
            .uri(&config.uri)
            .user(&config.user)
            .password(&config.password);

        if let Some(db) = &config.database {
            builder = builder.db(db.as_str());
        }

        let neo_config = builder
            .build()
            .map_err(|e| Neo4jError::Connection(e.to_string()))?;
        let graph = Graph::connect(neo_config).await?;

        Ok(Self {
            graph: Arc::new(graph),
        })
    }

    /// Create a new scan run and link it to a commit
    ///
    /// Returns \`true\` if this is a new commit (needs file processing),
    /// or \`false\` if the commit already exists (can skip file processing).
    ///
    /// # Errors
    /// Returns an error if the query fails.
    pub async fn create_scan_run(&self, scan_run: &ScanRun) -> Result<bool, Neo4jError> {
        let commit_sha = scan_run.commit_sha.clone().unwrap_or_default();

        // Check if commit already exists
        if !commit_sha.is_empty() {
            let check_query = Query::new(
                r#"
                MATCH (c:Commit {sha: $commit_sha})
                RETURN c.sha as sha
                LIMIT 1
                "#
                .to_string(),
            )
            .param("commit_sha", commit_sha.clone());

            let mut result = self.graph.execute(check_query).await?;

            if result.next().await?.is_some() {
                // Commit exists - create ScanRun and link to existing commit
                let query = Query::new(
                    r#"
                    MATCH (c:Commit {sha: $commit_sha})
                    CREATE (r:ScanRun {
                        id: $id,
                        repo_path: $repo_path,
                        scanned_at: datetime($scanned_at),
                        version: $version
                    })
                    CREATE (r)-[:FOR_COMMIT]->(c)
                    "#
                    .to_string(),
                )
                .param("id", scan_run.id.clone())
                .param("repo_path", scan_run.repo_path.clone())
                .param("scanned_at", scan_run.scanned_at.to_rfc3339())
                .param("version", scan_run.version.clone().unwrap_or_default())
                .param("commit_sha", commit_sha);

                self.graph.run(query).await?;
                return Ok(false); // Commit already exists, skip file processing
            }
        }

        // Create new commit and scan run
        let query = Query::new(
            r#"
            CREATE (c:Commit {
                sha: $commit_sha,
                branch: $branch
            })
            CREATE (r:ScanRun {
                id: $id,
                repo_path: $repo_path,
                scanned_at: datetime($scanned_at),
                version: $version
            })
            CREATE (r)-[:FOR_COMMIT]->(c)
            "#
            .to_string(),
        )
        .param("id", scan_run.id.clone())
        .param("repo_path", scan_run.repo_path.clone())
        .param("commit_sha", commit_sha)
        .param("branch", scan_run.branch.clone().unwrap_or_default())
        .param("scanned_at", scan_run.scanned_at.to_rfc3339())
        .param("version", scan_run.version.clone().unwrap_or_default());

        self.graph.run(query).await?;
        Ok(true) // New commit, needs file processing
    }

    /// Create or link a file to a commit
    ///
    /// Returns \`Some(content_hash)\` if this is a new file (needs symbol extraction),
    /// or \`None\` if the file already exists (symbols already extracted).
    ///
    /// # Errors
    /// Returns an error if the query fails.
    pub async fn create_file_if_new(
        &self,
        file_path: &str,
        content_hash: &str,
        language: &str,
        commit_sha: &str,
    ) -> Result<Option<String>, Neo4jError> {
        // Check if file with this hash already exists
        let check_query = Query::new(
            r#"
            MATCH (f:File {content_hash: $content_hash})
            RETURN f.content_hash as hash
            LIMIT 1
            "#
            .to_string(),
        )
        .param("content_hash", content_hash);

        let mut result = self.graph.execute(check_query).await?;

        if result.next().await?.is_some() {
            // File exists - just link to commit
            let link_query = Query::new(
                r#"
                MATCH (f:File {content_hash: $content_hash})
                MATCH (c:Commit {sha: $commit_sha})
                MERGE (c)-[:CONTAINS]->(f)
                "#
                .to_string(),
            )
            .param("content_hash", content_hash)
            .param("commit_sha", commit_sha);

            self.graph.run(link_query).await?;
            return Ok(None); // File exists, skip symbol extraction
        }

        // Create new file and link to commit
        let create_query = Query::new(
            r#"
            MATCH (c:Commit {sha: $commit_sha})
            CREATE (f:File {
                content_hash: $content_hash,
                path: $file_path,
                language: $language
            })
            CREATE (c)-[:CONTAINS]->(f)
            "#
            .to_string(),
        )
        .param("commit_sha", commit_sha)
        .param("content_hash", content_hash)
        .param("file_path", file_path)
        .param("language", language);

        self.graph.run(create_query).await?;
        Ok(Some(content_hash.to_string())) // New file, needs symbol extraction
    }

    /// Create a symbol linked to a file
    ///
    /// # Errors
    /// Returns an error if the query fails.
    pub async fn create_symbol(
        &self,
        symbol: &SymbolNode,
        content_hash: &str,
    ) -> Result<(), Neo4jError> {
        let query = Query::new(
            r#"
            MATCH (f:File {content_hash: $content_hash})
            CREATE (s:Symbol {
                id: $id,
                name: $name,
                qualified_name: $qualified_name,
                kind: $kind,
                visibility: $visibility,
                file_path: $file_path,
                start_line: $start_line,
                end_line: $end_line,
                signature: $signature,
                doc_comment: $doc_comment
            })
            CREATE (s)-[:DEFINED_IN]->(f)
            "#
            .to_string(),
        )
        .param("content_hash", content_hash)
        .param("id", symbol.id.clone())
        .param("name", symbol.name.clone())
        .param("qualified_name", symbol.qualified_name.clone())
        .param("kind", symbol.kind.to_string())
        .param("visibility", symbol.visibility.clone().unwrap_or_default())
        .param("file_path", symbol.file_path.clone())
        .param("start_line", symbol.start_line as i64)
        .param("end_line", symbol.end_line as i64)
        .param("signature", symbol.signature.clone().unwrap_or_default())
        .param(
            "doc_comment",
            symbol.doc_comment.clone().unwrap_or_default(),
        );

        self.graph.run(query).await?;
        Ok(())
    }

    /// Create multiple symbols for a file
    ///
    /// # Errors
    /// Returns an error if the query fails.
    pub async fn create_symbols_batch(
        &self,
        symbols: &[SymbolNode],
        content_hash: &str,
    ) -> Result<(), Neo4jError> {
        for symbol in symbols {
            self.create_symbol(symbol, content_hash).await?;
        }
        Ok(())
    }

    /// Create an edge between symbols
    ///
    /// # Errors
    /// Returns an error if the query fails.
    pub async fn create_edge(&self, edge: &Edge) -> Result<(), Neo4jError> {
        let rel_type = edge.kind.to_string();
        let query_str = format!(
            r#"
            MATCH (source:Symbol {{id: $source_id}})
            MATCH (target:Symbol {{id: $target_id}})
            CREATE (source)-[:{rel_type} {{line: $line, column: $column}}]->(target)
            "#
        );

        let query = Query::new(query_str)
            .param("source_id", edge.source_id.clone())
            .param("target_id", edge.target_id.clone())
            .param("line", edge.line.map(|l| l as i64).unwrap_or(0))
            .param("column", edge.column.map(|c| c as i64).unwrap_or(0));

        self.graph.run(query).await?;
        Ok(())
    }

    /// Create a REFERENCES edge from one symbol to another
    ///
    /// This represents that `from_symbol_id` references/uses `to_symbol_id` at the given location.
    ///
    /// # Errors
    /// Returns an error if the query fails.
    pub async fn create_symbol_reference(
        &self,
        from_symbol_id: &str,
        to_symbol_id: &str,
        line: u32,
        column: u32,
    ) -> Result<(), Neo4jError> {
        let query = Query::new(
            r#"
            MATCH (from:Symbol {id: $from_id})
            MATCH (to:Symbol {id: $to_id})
            CREATE (from)-[:REFERENCES {line: $line, column: $column}]->(to)
            "#
            .to_string(),
        )
        .param("from_id", from_symbol_id)
        .param("to_id", to_symbol_id)
        .param("line", line as i64)
        .param("column", column as i64);

        self.graph.run(query).await?;
        Ok(())
    }
}
