//! Test script to debug SysML LSP references
//! Run with: cargo run --example debug_sysml_references -p mother-core

#![allow(clippy::print_stdout)]

use mother_core::lsp::{LspClient, LspServerConfig, LspSymbol};
use mother_core::scanner::Language;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Simple logging
    println!("=== SysML LSP References Debug Test ===\n");

    let root_path = PathBuf::from("/tmp/syster");
    let root_uri = format!("file://{}", root_path.display());

    // Configure the syster LSP with stdlib path
    let stdlib_path = root_path.join("crates/syster-base/sysml.library");
    let init_options = Some(serde_json::json!({
        "stdlibEnabled": true,
        "stdlibPath": stdlib_path.display().to_string()
    }));

    let config = LspServerConfig {
        language: Language::SysML,
        command: "/tmp/syster/target/release/syster-lsp".to_string(),
        args: vec![],
        root_path: root_path.clone(),
        init_options,
    };

    println!("Starting syster LSP with stdlib path: {:?}", stdlib_path);
    let mut client = LspClient::start(config).await?;
    client.initialize(&root_uri).await?;

    // Open a test file with internal references
    let test_file = root_path.join("test_refs.sysml");
    let file_uri = format!("file://{}", test_file.display());
    let content = std::fs::read_to_string(&test_file)?;

    // Also hardcode the correct path for symbols_by_file later
    let test_file_path = test_file.display().to_string();

    println!("Opening file: {}", file_uri);
    client.did_open(&file_uri, "sysml", &content).await?;

    // Get document symbols first
    println!("\n=== Document Symbols ===");
    let symbols = client.document_symbols(&file_uri).await?;
    for sym in &symbols {
        println!(
            "Symbol: {} (kind: {:?}) at line {}-{}, col {}-{}",
            sym.name, sym.kind, sym.start_line, sym.end_line, sym.start_col, sym.end_col
        );
        for child in &sym.children {
            println!(
                "  Child: {} (kind: {:?}) at line {}-{}",
                child.name, child.kind, child.start_line, child.end_line
            );
        }
    }

    // Flatten symbols for reference testing
    fn flatten(symbols: &[LspSymbol]) -> Vec<&LspSymbol> {
        let mut result = Vec::new();
        for s in symbols {
            result.push(s);
            result.extend(flatten(&s.children));
        }
        result
    }

    let flat_symbols = flatten(&symbols);

    // Try to get references for each symbol
    println!("\n=== References ===");

    // Build a symbols_by_file lookup like the scanner does
    use std::collections::HashMap;
    let mut symbols_by_file: HashMap<String, Vec<(String, u32, u32)>> = HashMap::new();
    for sym in &flat_symbols {
        // Use the actual file path
        symbols_by_file
            .entry(test_file_path.clone())
            .or_default()
            .push((sym.name.clone(), sym.start_line, sym.end_line));
    }

    println!("\nSymbols by file lookup:");
    for (file, syms) in &symbols_by_file {
        println!("  {}", file);
        for (name, start, end) in syms {
            println!("    {} @ lines {}-{}", name, start, end);
        }
    }

    for sym in &flat_symbols {
        println!(
            "\nGetting references for '{}' at line {}, col {}...",
            sym.name, sym.start_line, sym.start_col
        );

        match client
            .references(&file_uri, sym.start_line, sym.start_col, true)
            .await
        {
            Ok(refs) => {
                println!("  Found {} references:", refs.len());
                for r in &refs {
                    println!(
                        "    - {}:{}:{}-{}",
                        r.file.display(),
                        r.line,
                        r.start_col,
                        r.end_col
                    );

                    // Simulate what the scanner does
                    let ref_file = r.file.display().to_string();
                    let ref_line = r.line;

                    if let Some(symbols_in_file) = symbols_by_file.get(&ref_file) {
                        let containing_symbol = symbols_in_file
                            .iter()
                            .filter(|(_, start, end)| ref_line >= *start && ref_line <= *end)
                            .min_by_key(|(_, start, end)| end - start);

                        if let Some((from_name, start, end)) = containing_symbol {
                            println!(
                                "      -> Would create edge: {} (lines {}-{}) -[:REFERENCES]-> {}",
                                from_name, start, end, sym.name
                            );
                        } else {
                            println!("      -> No containing symbol found for line {}", ref_line);
                            println!("         Available symbols in file:");
                            for (name, start, end) in symbols_in_file {
                                println!(
                                    "           {} @ {}-{} (contains line {}? {})",
                                    name,
                                    start,
                                    end,
                                    ref_line,
                                    ref_line >= *start && ref_line <= *end
                                );
                            }
                        }
                    } else {
                        println!("      -> File not found in symbols_by_file: {}", ref_file);
                    }
                }
            }
            Err(e) => {
                println!("  Error: {}", e);
            }
        }
    }

    client.shutdown().await?;
    println!("\nDone!");
    Ok(())
}
