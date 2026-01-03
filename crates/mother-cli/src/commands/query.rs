//! Query command: Execute queries against Neo4j graph

use anyhow::Result;
use mother_core::graph::neo4j::{Neo4jClient, Neo4jConfig};
use tracing::info;

use crate::QueryCommands;

/// Run the query command
///
/// # Errors
/// Returns an error if the query fails.
pub async fn run(
    cmd: QueryCommands,
    neo4j_uri: &str,
    neo4j_user: &str,
    neo4j_password: &str,
) -> Result<()> {
    let config = Neo4jConfig::new(neo4j_uri, neo4j_user, neo4j_password);
    let client = Neo4jClient::connect(&config).await?;

    match cmd {
        QueryCommands::Symbols { pattern } => {
            run_find_symbols(&client, &pattern).await?;
        }
        QueryCommands::File { path } => {
            run_symbols_in_file(&client, &path).await?;
        }
        QueryCommands::RefsTo { symbol } => {
            run_refs_to(&client, &symbol).await?;
        }
        QueryCommands::RefsFrom { symbol } => {
            run_refs_from(&client, &symbol).await?;
        }
        QueryCommands::Files { pattern } => {
            run_list_files(&client, pattern.as_deref()).await?;
        }
        QueryCommands::Stats => {
            run_stats(&client).await?;
        }
        QueryCommands::Raw { query } => {
            run_raw(&client, &query).await?;
        }
    }

    Ok(())
}

async fn run_find_symbols(client: &Neo4jClient, pattern: &str) -> Result<()> {
    info!("Finding symbols matching '{}'...", pattern);
    let symbols = client.find_symbols(pattern).await?;

    if symbols.is_empty() {
        println!("No symbols found matching '{}'", pattern);
        return Ok(());
    }

    println!("\n{:<40} {:<15} {:<50} LINES", "NAME", "KIND", "FILE");
    println!("{}", "-".repeat(110));

    for s in &symbols {
        let file = truncate_path(&s.file_path, 50);
        println!(
            "{:<40} {:<15} {:<50} {}-{}",
            truncate_str(&s.name, 40),
            truncate_str(&s.kind, 15),
            file,
            s.start_line,
            s.end_line
        );
    }

    println!("\nFound {} symbols", symbols.len());
    Ok(())
}

async fn run_symbols_in_file(client: &Neo4jClient, path: &str) -> Result<()> {
    info!("Finding symbols in file matching '{}'...", path);
    let symbols = client.symbols_in_file(path).await?;

    if symbols.is_empty() {
        println!("No symbols found in files matching '{}'", path);
        return Ok(());
    }

    println!(
        "\n{:<6} {:<40} {:<15} QUALIFIED NAME",
        "LINE", "NAME", "KIND"
    );
    println!("{}", "-".repeat(100));

    for s in &symbols {
        println!(
            "{:<6} {:<40} {:<15} {}",
            s.start_line,
            truncate_str(&s.name, 40),
            truncate_str(&s.kind, 15),
            truncate_str(&s.qualified_name, 60),
        );
    }

    println!("\nFound {} symbols", symbols.len());
    Ok(())
}

async fn run_refs_to(client: &Neo4jClient, symbol: &str) -> Result<()> {
    info!("Finding references to '{}'...", symbol);
    let refs = client.find_references_to(symbol).await?;

    if refs.is_empty() {
        println!("No references found to '{}'", symbol);
        return Ok(());
    }

    println!("\n{:<40} {:<50} {:<6}", "FROM SYMBOL", "FILE", "LINE");
    println!("{}", "-".repeat(100));

    for r in &refs {
        println!(
            "{:<40} {:<50} {:<6}",
            truncate_str(&r.source_name, 40),
            truncate_path(&r.source_file, 50),
            r.source_line,
        );
    }

    println!("\nFound {} references to '{}'", refs.len(), symbol);
    Ok(())
}

async fn run_refs_from(client: &Neo4jClient, symbol: &str) -> Result<()> {
    info!("Finding references from '{}'...", symbol);
    let refs = client.find_references_from(symbol).await?;

    if refs.is_empty() {
        println!("'{}' doesn't reference any symbols", symbol);
        return Ok(());
    }

    println!("\n{:<40} {:<50} {:<6}", "TO SYMBOL", "FILE", "LINE");
    println!("{}", "-".repeat(100));

    for r in &refs {
        println!(
            "{:<40} {:<50} {:<6}",
            truncate_str(&r.target_name, 40),
            truncate_path(&r.target_file, 50),
            r.target_line,
        );
    }

    println!("\n'{}' references {} symbols", symbol, refs.len());
    Ok(())
}

async fn run_list_files(client: &Neo4jClient, pattern: Option<&str>) -> Result<()> {
    info!("Listing files...");
    let files = client.list_files(pattern).await?;

    if files.is_empty() {
        println!("No files found");
        return Ok(());
    }

    println!("\n{:<60} {:<15} SYMBOLS", "PATH", "LANGUAGE");
    println!("{}", "-".repeat(85));

    for f in &files {
        println!(
            "{:<60} {:<15} {}",
            truncate_path(&f.path, 60),
            f.language,
            f.symbol_count,
        );
    }

    println!("\nFound {} files", files.len());
    Ok(())
}

async fn run_stats(client: &Neo4jClient) -> Result<()> {
    info!("Getting graph statistics...");
    let stats = client.stats().await?;

    println!("\n=== Graph Statistics ===\n");
    println!("Nodes:");
    println!("  Commits:   {}", stats.commits);
    println!("  Files:     {}", stats.files);
    println!("  Symbols:   {}", stats.symbols);
    println!("  ScanRuns:  {}", stats.scan_runs);
    println!("\nRelationships:");
    println!("  REFERENCES: {}", stats.references);
    println!("  DEFINED_IN: {}", stats.defined_in);
    println!("  CONTAINS:   {}", stats.contains);
    Ok(())
}

async fn run_raw(client: &Neo4jClient, query: &str) -> Result<()> {
    info!("Executing raw query...");
    let count = client.execute_raw(query).await?;
    println!("Query executed successfully. {} rows returned.", count);
    Ok(())
}

fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

fn truncate_path(path: &str, max_len: usize) -> String {
    if path.len() <= max_len {
        path.to_string()
    } else {
        // Show the end of the path (more useful)
        format!("...{}", &path[path.len() - max_len + 3..])
    }
}
