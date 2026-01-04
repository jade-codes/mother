//! Tests for language detection

use crate::scanner::Language;
use std::path::Path;

#[test]
fn test_language_from_extension() {
    assert_eq!(Language::from_extension("rs"), Some(Language::Rust));
    assert_eq!(Language::from_extension("py"), Some(Language::Python));
    assert_eq!(Language::from_extension("ts"), Some(Language::TypeScript));
    assert_eq!(Language::from_extension("tsx"), Some(Language::TypeScript));
    assert_eq!(Language::from_extension("js"), Some(Language::JavaScript));
    assert_eq!(Language::from_extension("jsx"), Some(Language::JavaScript));
    assert_eq!(Language::from_extension("go"), Some(Language::Go));
    assert_eq!(Language::from_extension("sysml"), Some(Language::SysML));
    assert_eq!(Language::from_extension("kerml"), Some(Language::KerML));
    assert_eq!(Language::from_extension("txt"), None);
}

#[test]
fn test_language_from_path() {
    assert_eq!(
        Language::from_path(Path::new("src/main.rs")),
        Some(Language::Rust)
    );
    assert_eq!(
        Language::from_path(Path::new("app.py")),
        Some(Language::Python)
    );
    assert_eq!(
        Language::from_path(Path::new("index.ts")),
        Some(Language::TypeScript)
    );
    assert_eq!(
        Language::from_path(Path::new("model.sysml")),
        Some(Language::SysML)
    );
    assert_eq!(
        Language::from_path(Path::new("main.go")),
        Some(Language::Go)
    );
    assert_eq!(
        Language::from_path(Path::new("kernel.kerml")),
        Some(Language::KerML)
    );
    assert_eq!(Language::from_path(Path::new("README.md")), None);
}

#[test]
fn test_language_display() {
    assert_eq!(format!("{}", Language::Rust), "rust");
    assert_eq!(format!("{}", Language::Python), "python");
    assert_eq!(format!("{}", Language::TypeScript), "typescript");
    assert_eq!(format!("{}", Language::JavaScript), "javascript");
    assert_eq!(format!("{}", Language::Go), "go");
    assert_eq!(format!("{}", Language::SysML), "sysml");
    assert_eq!(format!("{}", Language::KerML), "kerml");
}
