//! Read-only query operations for Neo4j

use neo4rs::Query;

use super::Neo4jClient;
use crate::graph::neo4j::Neo4jError;

/// A symbol result from a query
#[derive(Debug, Clone)]
pub struct SymbolResult {
    pub id: String,
    pub name: String,
    pub qualified_name: String,
    pub kind: String,
    pub file_path: String,
    pub start_line: i64,
    pub end_line: i64,
}

/// A reference result from a query
#[derive(Debug, Clone)]
pub struct ReferenceResult {
    pub source_name: String,
    pub source_file: String,
    pub source_line: i64,
    pub target_name: String,
    pub target_file: String,
    pub target_line: i64,
}

/// A file result from a query
#[derive(Debug, Clone)]
pub struct FileResult {
    pub path: String,
    pub language: String,
    pub symbol_count: i64,
}

impl Neo4jClient {
    /// Find symbols by name pattern (case-insensitive contains)
    ///
    /// # Errors
    /// Returns an error if the query fails.
    pub async fn find_symbols(&self, pattern: &str) -> Result<Vec<SymbolResult>, Neo4jError> {
        let query = Query::new(
            r#"
            MATCH (s:Symbol)
            WHERE toLower(s.name) CONTAINS toLower($pattern)
            RETURN s.id, s.name, s.qualified_name, s.kind, s.file_path, s.start_line, s.end_line
            ORDER BY s.name
            LIMIT 100
            "#
            .to_string(),
        )
        .param("pattern", pattern);

        let mut result = self.graph().execute(query).await?;
        let mut symbols = Vec::new();

        while let Some(row) = result.next().await? {
            symbols.push(SymbolResult {
                id: row.get("s.id").unwrap_or_default(),
                name: row.get("s.name").unwrap_or_default(),
                qualified_name: row.get("s.qualified_name").unwrap_or_default(),
                kind: row.get("s.kind").unwrap_or_default(),
                file_path: row.get("s.file_path").unwrap_or_default(),
                start_line: row.get("s.start_line").unwrap_or(0),
                end_line: row.get("s.end_line").unwrap_or(0),
            });
        }

        Ok(symbols)
    }

    /// Find symbols in a specific file
    ///
    /// # Errors
    /// Returns an error if the query fails.
    pub async fn symbols_in_file(&self, file_path: &str) -> Result<Vec<SymbolResult>, Neo4jError> {
        let query = Query::new(
            r#"
            MATCH (s:Symbol)
            WHERE s.file_path CONTAINS $file_path
            RETURN s.id, s.name, s.qualified_name, s.kind, s.file_path, s.start_line, s.end_line
            ORDER BY s.start_line
            "#
            .to_string(),
        )
        .param("file_path", file_path);

        let mut result = self.graph().execute(query).await?;
        let mut symbols = Vec::new();

        while let Some(row) = result.next().await? {
            symbols.push(SymbolResult {
                id: row.get("s.id").unwrap_or_default(),
                name: row.get("s.name").unwrap_or_default(),
                qualified_name: row.get("s.qualified_name").unwrap_or_default(),
                kind: row.get("s.kind").unwrap_or_default(),
                file_path: row.get("s.file_path").unwrap_or_default(),
                start_line: row.get("s.start_line").unwrap_or(0),
                end_line: row.get("s.end_line").unwrap_or(0),
            });
        }

        Ok(symbols)
    }

    /// Find what references a given symbol (by name)
    ///
    /// # Errors
    /// Returns an error if the query fails.
    pub async fn find_references_to(
        &self,
        symbol_name: &str,
    ) -> Result<Vec<ReferenceResult>, Neo4jError> {
        let query = Query::new(
            r#"
            MATCH (source:Symbol)-[r:REFERENCES]->(target:Symbol)
            WHERE target.name = $symbol_name
            RETURN source.name, source.file_path, r.line, target.name, target.file_path, target.start_line
            ORDER BY source.file_path, r.line
            LIMIT 100
            "#
            .to_string(),
        )
        .param("symbol_name", symbol_name);

        let mut result = self.graph().execute(query).await?;
        let mut refs = Vec::new();

        while let Some(row) = result.next().await? {
            refs.push(ReferenceResult {
                source_name: row.get("source.name").unwrap_or_default(),
                source_file: row.get("source.file_path").unwrap_or_default(),
                source_line: row.get("r.line").unwrap_or(0),
                target_name: row.get("target.name").unwrap_or_default(),
                target_file: row.get("target.file_path").unwrap_or_default(),
                target_line: row.get("target.start_line").unwrap_or(0),
            });
        }

        Ok(refs)
    }

    /// Find what a symbol references (outgoing references)
    ///
    /// # Errors
    /// Returns an error if the query fails.
    pub async fn find_references_from(
        &self,
        symbol_name: &str,
    ) -> Result<Vec<ReferenceResult>, Neo4jError> {
        let query = Query::new(
            r#"
            MATCH (source:Symbol)-[r:REFERENCES]->(target:Symbol)
            WHERE source.name = $symbol_name
            RETURN source.name, source.file_path, r.line, target.name, target.file_path, target.start_line
            ORDER BY target.file_path, target.start_line
            LIMIT 100
            "#
            .to_string(),
        )
        .param("symbol_name", symbol_name);

        let mut result = self.graph().execute(query).await?;
        let mut refs = Vec::new();

        while let Some(row) = result.next().await? {
            refs.push(ReferenceResult {
                source_name: row.get("source.name").unwrap_or_default(),
                source_file: row.get("source.file_path").unwrap_or_default(),
                source_line: row.get("r.line").unwrap_or(0),
                target_name: row.get("target.name").unwrap_or_default(),
                target_file: row.get("target.file_path").unwrap_or_default(),
                target_line: row.get("target.start_line").unwrap_or(0),
            });
        }

        Ok(refs)
    }

    /// List files with symbol counts
    ///
    /// # Errors
    /// Returns an error if the query fails.
    pub async fn list_files(&self, pattern: Option<&str>) -> Result<Vec<FileResult>, Neo4jError> {
        let query_str = if pattern.is_some() {
            r#"
            MATCH (f:File)
            WHERE f.path CONTAINS $pattern
            OPTIONAL MATCH (s:Symbol)-[:DEFINED_IN]->(f)
            RETURN f.path, f.language, count(s) as symbol_count
            ORDER BY f.path
            LIMIT 100
            "#
        } else {
            r#"
            MATCH (f:File)
            OPTIONAL MATCH (s:Symbol)-[:DEFINED_IN]->(f)
            RETURN f.path, f.language, count(s) as symbol_count
            ORDER BY f.path
            LIMIT 100
            "#
        };

        let mut query = Query::new(query_str.to_string());
        if let Some(p) = pattern {
            query = query.param("pattern", p);
        }

        let mut result = self.graph().execute(query).await?;
        let mut files = Vec::new();

        while let Some(row) = result.next().await? {
            files.push(FileResult {
                path: row.get("f.path").unwrap_or_default(),
                language: row.get("f.language").unwrap_or_default(),
                symbol_count: row.get("symbol_count").unwrap_or(0),
            });
        }

        Ok(files)
    }

    /// Execute a raw Cypher query and return the number of rows affected
    ///
    /// For queries that return data, use specific query methods instead.
    ///
    /// # Errors
    /// Returns an error if the query fails.
    pub async fn execute_raw(&self, cypher: &str) -> Result<usize, Neo4jError> {
        let query = Query::new(cypher.to_string());
        let mut result = self.graph().execute(query).await?;
        let mut count = 0;

        while let Some(_row) = result.next().await? {
            count += 1;
        }

        Ok(count)
    }

    /// Get graph statistics
    ///
    /// # Errors
    /// Returns an error if the query fails.
    pub async fn stats(&self) -> Result<GraphStats, Neo4jError> {
        let query = Query::new(
            r#"
            MATCH (n)
            WITH labels(n)[0] as label, count(n) as cnt
            RETURN label, cnt
            ORDER BY label
            "#
            .to_string(),
        );

        let mut result = self.graph().execute(query).await?;
        let mut stats = GraphStats::default();

        while let Some(row) = result.next().await? {
            let label: String = row.get("label").unwrap_or_default();
            let count: i64 = row.get("cnt").unwrap_or(0);

            match label.as_str() {
                "Commit" => stats.commits = count,
                "File" => stats.files = count,
                "Symbol" => stats.symbols = count,
                "ScanRun" => stats.scan_runs = count,
                _ => {}
            }
        }

        // Get relationship counts
        let rel_query = Query::new(
            r#"
            MATCH ()-[r]->()
            WITH type(r) as rel_type, count(r) as cnt
            RETURN rel_type, cnt
            ORDER BY cnt DESC
            "#
            .to_string(),
        );

        let mut rel_result = self.graph().execute(rel_query).await?;
        while let Some(row) = rel_result.next().await? {
            let rel_type: String = row.get("rel_type").unwrap_or_default();
            let count: i64 = row.get("cnt").unwrap_or(0);

            match rel_type.as_str() {
                "REFERENCES" => stats.references = count,
                "DEFINED_IN" => stats.defined_in = count,
                "CONTAINS" => stats.contains = count,
                _ => {}
            }
        }

        Ok(stats)
    }
}

/// Graph statistics
#[derive(Debug, Default, Clone)]
pub struct GraphStats {
    pub commits: i64,
    pub files: i64,
    pub symbols: i64,
    pub scan_runs: i64,
    pub references: i64,
    pub defined_in: i64,
    pub contains: i64,
}
