# mother: Semantic Graph Ingestion System

A tool that uses Language Server Protocol to extract rich semantic information from codebases, storing them in a Neo4j graph database with versioned runs for meaningful analysis.

## Features

- **LSP-based extraction** - Uses existing language servers for accurate semantic info
- **Multi-language support** - Rust, Python, TypeScript, JavaScript, SysML, KerML
- **Neo4j graph storage** - Versioned scan runs with full relationship tracking
- **Diff queries** - Track changes between versions
- **Cross-file analysis** - Fully resolved references and types

## Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                           mother                                   │
├─────────────────────────────────────────────────────────────────────┤
│  ┌──────────────┐    ┌─────────────────────────────────────────────┐│
│  │   Scanner    │───▶│              LSP Client                     ││
│  │  (walkdir)   │    │  textDocument/documentSymbol                ││
│  └──────────────┘    │  textDocument/references                    ││
│                      │  textDocument/definition                    ││
│                      └──────────────────────┬──────────────────────┘│
│                                             │                        │
│  ┌──────────────────────────────────────────▼──────────────────────┐│
│  │                   LSP Server Manager                            ││
│  │  ┌────────────┐ ┌────────────┐ ┌────────────┐ ┌────────────┐   ││
│  │  │rust-       │ │pyright     │ │typescript- │ │syster-lsp  │   ││
│  │  │analyzer    │ │            │ │language-   │ │(SysML/     │   ││
│  │  │            │ │            │ │server      │ │KerML)      │   ││
│  │  └────────────┘ └────────────┘ └────────────┘ └────────────┘   ││
│  └─────────────────────────────────────────────────────────────────┘│
│                                                                      │
│  ┌─────────────────────────────────────────────────────────────────┐│
│  │                    Neo4j (Versioned Graph)                      ││
│  │  ScanRun → File → Symbol → [CALLS|REFERENCES|INHERITS] → Symbol ││
│  └─────────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────────┘
```

## Project Structure

```
mother/
├── crates/
│   ├── mother-core/          # Core library
│   │   └── src/
│   │       ├── scanner/        # File discovery
│   │       ├── lsp/            # LSP client & server manager
│   │       ├── graph/          # Graph model & Neo4j storage
│   │       └── version/        # Versioning logic
│   └── mother-cli/           # CLI application
│       └── src/
│           └── main.rs
├── .github/workflows/          # CI/CD
└── Makefile                    # Development commands
```

## Usage

```bash
# Scan a repository and store in Neo4j
mother scan /path/to/repo \
  --neo4j-uri bolt://localhost:7687 \
  --neo4j-user neo4j \
  --neo4j-password secret

# Scan with explicit version tag
mother scan /path/to/repo --version "v1.2.0"

# Compare two versions
mother diff --from v1.0.0 --to v1.2.0

# Query the graph
mother query "MATCH (s:Symbol {kind: 'function'}) RETURN s.name LIMIT 10"
```

## Development

### Prerequisites

- Rust 1.85+ (edition 2024)
- Neo4j 5.x (for graph storage)
- Git

### Commands

```bash
# Build the project
make build

# Run tests
make test

# Format code
make fmt

# Run linter
make lint

# Run complete validation pipeline
make run-guidelines

# Install the binary
make install
```

## Graph Schema

```cypher
// Versioned scan runs
(:ScanRun {id, repo_path, commit_sha, branch, scanned_at, version})

// Files scanned in each run
(:File {path, content_hash, language, lines})-[:SCANNED_IN]->(:ScanRun)

// Symbols with semantics
(:Symbol {
  id, name, qualified_name, kind, visibility,
  start_line, end_line, signature, doc_comment
})-[:DEFINED_IN]->(:File)

// Relationships
(:Symbol)-[:CALLS {line, column}]->(:Symbol)
(:Symbol)-[:REFERENCES {line}]->(:Symbol)
(:Symbol)-[:INHERITS]->(:Symbol)
(:Symbol)-[:IMPLEMENTS]->(:Symbol)
(:Symbol)-[:IMPORTS]->(:Symbol)
```

## License

MIT
