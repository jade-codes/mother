//! Scan-related Neo4j queries

use neo4rs::Query;

use super::Neo4jClient;
use crate::graph::model::ScanRun;
use crate::graph::neo4j::Neo4jError;

impl Neo4jClient {
    /// Create a new scan run and link it to a commit
    ///
    /// Returns `true` if this is a new commit (needs file processing),
    /// or `false` if the commit already exists (can skip file processing).
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

            let mut result = self.graph().execute(check_query).await?;

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

                self.graph().run(query).await?;
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

        self.graph().run(query).await?;
        Ok(true) // New commit, needs file processing
    }
}
