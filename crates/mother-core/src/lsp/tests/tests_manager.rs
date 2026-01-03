//! Tests for LSP manager module

use crate::lsp::manager::LspServerDefaults;
use crate::scanner::Language;
use std::path::PathBuf;

#[test]
fn test_for_language_rust() {
    let root = PathBuf::from("/test/root");
    let config = LspServerDefaults::for_language(Language::Rust, &root);

    assert_eq!(config.language, Language::Rust);
    assert_eq!(config.command, "rust-analyzer");
    assert!(config.args.is_empty());
    assert_eq!(config.root_path, root);
    assert!(config.init_options.is_none());
}

#[test]
fn test_for_language_python() {
    let root = PathBuf::from("/test/root");
    let config = LspServerDefaults::for_language(Language::Python, &root);

    assert_eq!(config.language, Language::Python);
    assert_eq!(config.command, "pyright-langserver");
    assert_eq!(config.args, vec!["--stdio"]);
    assert_eq!(config.root_path, root);
    assert!(config.init_options.is_none());
}

#[test]
fn test_for_language_typescript() {
    let root = PathBuf::from("/test/root");
    let config = LspServerDefaults::for_language(Language::TypeScript, &root);

    assert_eq!(config.language, Language::TypeScript);
    assert_eq!(config.command, "typescript-language-server");
    assert_eq!(config.args, vec!["--stdio"]);
    assert_eq!(config.root_path, root);
    assert!(config.init_options.is_none());
}

#[test]
fn test_for_language_javascript() {
    let root = PathBuf::from("/test/root");
    let config = LspServerDefaults::for_language(Language::JavaScript, &root);

    assert_eq!(config.language, Language::JavaScript);
    assert_eq!(config.command, "typescript-language-server");
    assert_eq!(config.args, vec!["--stdio"]);
    assert_eq!(config.root_path, root);
    assert!(config.init_options.is_none());
}

#[test]
fn test_for_language_sysml() {
    let root = PathBuf::from("/test/root");
    let config = LspServerDefaults::for_language(Language::SysML, &root);

    assert_eq!(config.language, Language::SysML);
    assert_eq!(config.command, "syster-lsp");
    assert!(config.args.is_empty());
    assert_eq!(config.root_path, root);

    // Should have init_options with stdlibEnabled
    assert!(config.init_options.is_some());
    let init_opts = config.init_options.unwrap();
    assert_eq!(
        init_opts.get("stdlibEnabled"),
        Some(&serde_json::json!(true))
    );
}

#[test]
fn test_for_language_kerml() {
    let root = PathBuf::from("/test/root");
    let config = LspServerDefaults::for_language(Language::KerML, &root);

    assert_eq!(config.language, Language::KerML);
    assert_eq!(config.command, "syster-lsp");
    assert!(config.args.is_empty());
    assert_eq!(config.root_path, root);

    // Should have init_options with stdlibEnabled
    assert!(config.init_options.is_some());
    let init_opts = config.init_options.unwrap();
    assert_eq!(
        init_opts.get("stdlibEnabled"),
        Some(&serde_json::json!(true))
    );
}

#[test]
fn test_for_language_sysml_with_stdlib_path() {
    // Test with a root path that might have the stdlib
    let root = PathBuf::from("/home/runner/work/mother/mother");
    let config = LspServerDefaults::for_language(Language::SysML, &root);

    assert_eq!(config.language, Language::SysML);
    assert_eq!(config.command, "syster-lsp");

    // Should have init_options
    assert!(config.init_options.is_some());
    let init_opts = config.init_options.unwrap();
    assert_eq!(
        init_opts.get("stdlibEnabled"),
        Some(&serde_json::json!(true))
    );

    // stdlibPath may or may not be present depending on filesystem
    // We just verify the structure is correct
}

#[test]
fn test_for_language_kerml_with_stdlib_path() {
    // Test with a root path that might have the stdlib
    let root = PathBuf::from("/home/runner/work/mother/mother");
    let config = LspServerDefaults::for_language(Language::KerML, &root);

    assert_eq!(config.language, Language::KerML);
    assert_eq!(config.command, "syster-lsp");

    // Should have init_options
    assert!(config.init_options.is_some());
    let init_opts = config.init_options.unwrap();
    assert_eq!(
        init_opts.get("stdlibEnabled"),
        Some(&serde_json::json!(true))
    );

    // stdlibPath may or may not be present depending on filesystem
    // We just verify the structure is correct
}

#[test]
fn test_for_language_with_empty_path() {
    let root = PathBuf::from("");
    let config = LspServerDefaults::for_language(Language::Rust, &root);

    assert_eq!(config.language, Language::Rust);
    assert_eq!(config.root_path, root);
}

#[test]
fn test_for_language_with_relative_path() {
    let root = PathBuf::from("./relative/path");
    let config = LspServerDefaults::for_language(Language::Python, &root);

    assert_eq!(config.language, Language::Python);
    assert_eq!(config.root_path, root);
}

#[test]
fn test_for_language_typescript_and_javascript_share_server() {
    let root = PathBuf::from("/test/root");
    let ts_config = LspServerDefaults::for_language(Language::TypeScript, &root);
    let js_config = LspServerDefaults::for_language(Language::JavaScript, &root);

    // Both should use the same server command
    assert_eq!(ts_config.command, js_config.command);
    assert_eq!(ts_config.args, js_config.args);

    // But maintain their own language identity
    assert_ne!(ts_config.language, js_config.language);
}

#[test]
fn test_for_language_sysml_and_kerml_share_server() {
    let root = PathBuf::from("/test/root");
    let sysml_config = LspServerDefaults::for_language(Language::SysML, &root);
    let kerml_config = LspServerDefaults::for_language(Language::KerML, &root);

    // Both should use the same server command
    assert_eq!(sysml_config.command, kerml_config.command);
    assert_eq!(sysml_config.args, kerml_config.args);

    // But maintain their own language identity
    assert_ne!(sysml_config.language, kerml_config.language);
}

#[test]
fn test_all_languages_have_config() {
    // Ensure every Language variant has a configuration
    let root = PathBuf::from("/test");

    let languages = [
        Language::Rust,
        Language::Python,
        Language::TypeScript,
        Language::JavaScript,
        Language::SysML,
        Language::KerML,
    ];

    for lang in languages {
        let config = LspServerDefaults::for_language(lang, &root);
        assert_eq!(config.language, lang);
        assert!(
            !config.command.is_empty(),
            "Command should not be empty for {:?}",
            lang
        );
    }
}

#[test]
fn test_for_language_preserves_root_path() {
    let root = PathBuf::from("/some/test/path/with/multiple/segments");

    let config = LspServerDefaults::for_language(Language::Rust, &root);
    assert_eq!(config.root_path, root);

    let config = LspServerDefaults::for_language(Language::Python, &root);
    assert_eq!(config.root_path, root);

    let config = LspServerDefaults::for_language(Language::SysML, &root);
    assert_eq!(config.root_path, root);
}
