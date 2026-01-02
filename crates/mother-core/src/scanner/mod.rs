//! Scanner module: File discovery and language detection
//!
//! Responsible for walking directories, respecting .gitignore,
//! and detecting the programming language of each file.

mod language;
mod walker;

pub use language::Language;
pub use walker::{DiscoveredFile, Scanner};

use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

/// Compute SHA-256 hash of a file's contents
///
/// # Errors
/// Returns an error if the file cannot be read.
pub fn compute_file_hash(path: &Path) -> std::io::Result<String> {
    let contents = fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&contents);
    let result = hasher.finalize();
    Ok(format!("{:x}", result))
}

#[cfg(test)]
mod tests;
