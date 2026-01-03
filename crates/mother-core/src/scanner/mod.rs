//! Scanner module: File discovery and language detection
//!
//! Responsible for walking directories, respecting .gitignore,
//! and detecting the programming language of each file.

mod language;
mod run;
mod walker;

pub use language::Language;
pub use walker::{DiscoveredFile, Scanner};

#[cfg(test)]
mod tests;
