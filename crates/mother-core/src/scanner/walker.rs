//! File walker: Discovers files in a directory tree

use std::path::{Path, PathBuf};

use ignore::WalkBuilder;

use super::Language;

/// A file discovered during scanning
#[derive(Debug, Clone)]
pub struct DiscoveredFile {
    pub path: PathBuf,
    pub language: Language,
}

/// Scanner for discovering source files in a directory
#[derive(Debug)]
pub struct Scanner {
    root: PathBuf,
    languages: Vec<Language>,
}

impl Scanner {
    /// Create a new scanner for the given root directory
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            languages: vec![
                Language::Rust,
                Language::Python,
                Language::TypeScript,
                Language::JavaScript,
                Language::SysML,
                Language::KerML,
            ],
        }
    }

    /// Filter to only scan specific languages
    #[must_use]
    pub fn with_languages(mut self, languages: Vec<Language>) -> Self {
        self.languages = languages;
        self
    }

    /// Scan the directory and return discovered files
    pub fn scan(&self) -> impl Iterator<Item = DiscoveredFile> + '_ {
        WalkBuilder::new(&self.root)
            .hidden(false)
            .git_ignore(true)
            .git_global(true)
            .git_exclude(true)
            .build()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_type().is_some_and(|ft| ft.is_file()))
            .filter_map(|entry| {
                let path = entry.into_path();
                Language::from_path(&path)
                    .filter(|lang| self.languages.contains(lang))
                    .map(|language| DiscoveredFile { path, language })
            })
    }

    /// Get the root directory being scanned
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }
}
