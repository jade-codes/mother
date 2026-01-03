//! File-related Neo4j queries

use neo4rs::Query;

use super::Neo4jClient;
use crate::graph::neo4j::Neo4jError;

impl Neo4jClient {
    /// Create or link a file to a commit
    ///
    /// Returns `Some(content_hash)` if this is a new file (needs symbol extraction),
    /// or `None` if the file already exists (symbols already extracted).
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

        let mut result = self.graph().execute(check_query).await?;

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

            self.graph().run(link_query).await?;
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

        self.graph().run(create_query).await?;
        Ok(Some(content_hash.to_string())) // New file, needs symbol extraction
    }
}
