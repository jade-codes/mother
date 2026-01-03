//! Neo4j client for graph storage

use std::sync::Arc;

use neo4rs::{ConfigBuilder, Graph, Query};
use thiserror::Error;

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
    /// Connect to Neo4j and ensure indexes exist
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

        let client = Self {
            graph: Arc::new(graph),
        };

        // Ensure indexes exist for performant queries
        client.ensure_indexes().await?;

        Ok(client)
    }

    /// Create indexes if they don't exist
    async fn ensure_indexes(&self) -> Result<(), Neo4jError> {
        let indexes = [
            "CREATE INDEX commit_sha IF NOT EXISTS FOR (c:Commit) ON (c.sha)",
            "CREATE INDEX file_path_hash IF NOT EXISTS FOR (f:File) ON (f.path, f.content_hash)",
            "CREATE INDEX symbol_name IF NOT EXISTS FOR (s:Symbol) ON (s.name)",
            "CREATE INDEX symbol_id IF NOT EXISTS FOR (s:Symbol) ON (s.id)",
            "CREATE INDEX symbol_file_path IF NOT EXISTS FOR (s:Symbol) ON (s.file_path)",
        ];

        for index_stmt in indexes {
            self.graph.run(Query::new(index_stmt.to_string())).await?;
        }

        Ok(())
    }

    /// Get access to the graph for query modules
    pub(super) fn graph(&self) -> &Graph {
        &self.graph
    }
}
