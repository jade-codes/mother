//! Symbol-related Neo4j queries

use neo4rs::Query;

use super::Neo4jClient;
use crate::graph::model::{Edge, SymbolNode};
use crate::graph::neo4j::Neo4jError;

impl Neo4jClient {
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

        self.graph().run(query).await?;
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

        self.graph().run(query).await?;
        Ok(())
    }
}
