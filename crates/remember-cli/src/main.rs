//! remember-cli: CLI for AST graph ingestion

use clap::{Parser, Subcommand};
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

mod commands;

#[derive(Parser)]
#[command(name = "remember")]
#[command(author, version, about = "AST graph ingestion tool", long_about = None)]
struct Cli {
    /// Enable verbose output
    #[arg(short, long, global = true)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Scan a repository and store AST in Neo4j
    Scan {
        /// Path to the repository to scan
        path: std::path::PathBuf,

        /// Neo4j connection URI
        #[arg(long, default_value = "bolt://localhost:7687")]
        neo4j_uri: String,

        /// Neo4j username
        #[arg(long, default_value = "neo4j")]
        neo4j_user: String,

        /// Neo4j password
        #[arg(long)]
        neo4j_password: String,

        /// Version tag for this scan
        #[arg(long)]
        version: Option<String>,
    },

    /// Query the Neo4j graph
    Query {
        /// Cypher query to execute
        query: String,

        /// Neo4j connection URI
        #[arg(long, default_value = "bolt://localhost:7687")]
        neo4j_uri: String,

        /// Neo4j username
        #[arg(long, default_value = "neo4j")]
        neo4j_user: String,

        /// Neo4j password
        #[arg(long)]
        neo4j_password: String,
    },

    /// Compare two scan versions
    Diff {
        /// First version to compare
        #[arg(long)]
        from: String,

        /// Second version to compare
        #[arg(long)]
        to: String,

        /// Neo4j connection URI
        #[arg(long, default_value = "bolt://localhost:7687")]
        neo4j_uri: String,

        /// Neo4j username
        #[arg(long, default_value = "neo4j")]
        neo4j_user: String,

        /// Neo4j password
        #[arg(long)]
        neo4j_password: String,
    },
}

fn setup_logging(verbose: bool) {
    let filter = if verbose {
        EnvFilter::new("debug")
    } else {
        EnvFilter::new("info")
    };

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(filter)
        .init();
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    setup_logging(cli.verbose);

    match cli.command {
        Commands::Scan {
            path,
            neo4j_uri,
            neo4j_user,
            neo4j_password,
            version,
        } => {
            commands::scan::run(&path, &neo4j_uri, &neo4j_user, &neo4j_password, version.as_deref())
                .await?;
        }
        Commands::Query {
            query,
            neo4j_uri,
            neo4j_user,
            neo4j_password,
        } => {
            commands::query::run(&query, &neo4j_uri, &neo4j_user, &neo4j_password).await?;
        }
        Commands::Diff {
            from,
            to,
            neo4j_uri,
            neo4j_user,
            neo4j_password,
        } => {
            commands::diff::run(&from, &to, &neo4j_uri, &neo4j_user, &neo4j_password).await?;
        }
    }

    Ok(())
}
