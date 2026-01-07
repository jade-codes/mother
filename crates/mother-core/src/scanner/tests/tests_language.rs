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

#[test]
fn test_language_extensions() {
    // Test that each language returns the correct extensions
    assert_eq!(Language::Rust.extensions(), &["rs"]);
    assert_eq!(Language::Python.extensions(), &["py"]);
    assert_eq!(Language::TypeScript.extensions(), &["ts", "tsx"]);
    assert_eq!(
        Language::JavaScript.extensions(),
        &["js", "jsx", "mjs", "cjs"]
    );
    assert_eq!(Language::Go.extensions(), &["go"]);
    assert_eq!(Language::SysML.extensions(), &["sysml"]);
    assert_eq!(Language::KerML.extensions(), &["kerml"]);
}

#[test]
fn test_extensions_consistency_with_from_extension() {
    // Verify that all extensions returned by extensions() can be parsed by from_extension()
    for language in [
        Language::Rust,
        Language::Python,
        Language::TypeScript,
        Language::JavaScript,
        Language::Go,
        Language::SysML,
        Language::KerML,
    ] {
        for ext in language.extensions() {
            assert_eq!(
                Language::from_extension(ext),
                Some(language),
                "Extension '{}' should map to {:?}",
                ext,
                language
            );
        }
    }
}

#[test]
fn test_extensions_are_not_empty() {
    // Ensure every language has at least one extension
    for language in [
        Language::Rust,
        Language::Python,
        Language::TypeScript,
        Language::JavaScript,
        Language::Go,
        Language::SysML,
        Language::KerML,
    ] {
        assert!(
            !language.extensions().is_empty(),
            "{:?} should have at least one extension",
            language
        );
    }
}

#[test]
fn test_extensions_multiple_for_typescript() {
    // TypeScript should support both .ts and .tsx files
    let extensions = Language::TypeScript.extensions();
    assert_eq!(extensions.len(), 2);
    assert!(extensions.contains(&"ts"));
    assert!(extensions.contains(&"tsx"));
}

#[test]
fn test_extensions_multiple_for_javascript() {
    // JavaScript should support .js, .jsx, .mjs, and .cjs files
    let extensions = Language::JavaScript.extensions();
    assert_eq!(extensions.len(), 4);
    assert!(extensions.contains(&"js"));
    assert!(extensions.contains(&"jsx"));
    assert!(extensions.contains(&"mjs"));
    assert!(extensions.contains(&"cjs"));
}
