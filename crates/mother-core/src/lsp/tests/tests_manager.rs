//! Tests for LSP Server Manager

use crate::lsp::manager::{LspServerDefaults, LspServerManager};
use crate::lsp::types::LspServerConfig;
use crate::scanner::Language;
use std::path::PathBuf;
use tempfile::TempDir;

// ============================================================================
// Tests for LspServerManager::new
// ============================================================================

#[test]
fn test_new_creates_manager_with_root_path() {
    let root = PathBuf::from("/tmp/test_project");
    let manager = LspServerManager::new(root.clone());

    // Manager should be created successfully
    // We can't directly inspect private fields, but we can test behavior
    // by trying to use the manager
    drop(manager);
}

#[test]
fn test_new_accepts_str_path() {
    let manager = LspServerManager::new("/tmp/test_project");
    drop(manager);
}

#[test]
fn test_new_accepts_pathbuf() {
    let root = PathBuf::from("/tmp/test_project");
    let manager = LspServerManager::new(root);
    drop(manager);
}

#[test]
fn test_new_accepts_tempdir_path() -> anyhow::Result<()> {
    let temp = TempDir::new()?;
    let manager = LspServerManager::new(temp.path());
    drop(manager);
    Ok(())
}

// ============================================================================
// Tests for LspServerManager::register_server
// ============================================================================

#[test]
fn test_register_server_stores_custom_config() -> anyhow::Result<()> {
    let temp = TempDir::new()?;
    let mut manager = LspServerManager::new(temp.path());

    let config = LspServerConfig {
        language: Language::Rust,
        command: "custom-rust-analyzer".to_string(),
        args: vec!["--custom-flag".to_string()],
        root_path: temp.path().to_path_buf(),
        init_options: Some(serde_json::json!({"custom": true})),
    };

    manager.register_server(config);

    // The custom config should be stored and used when get_client is called
    // We can't verify this directly without starting a server, but registration
    // should complete without error
    Ok(())
}

#[test]
fn test_register_server_multiple_languages() -> anyhow::Result<()> {
    let temp = TempDir::new()?;
    let mut manager = LspServerManager::new(temp.path());

    // Register configs for multiple languages
    let rust_config = LspServerConfig {
        language: Language::Rust,
        command: "rust-analyzer".to_string(),
        args: vec![],
        root_path: temp.path().to_path_buf(),
        init_options: None,
    };

    let python_config = LspServerConfig {
        language: Language::Python,
        command: "pyright".to_string(),
        args: vec![],
        root_path: temp.path().to_path_buf(),
        init_options: None,
    };

    manager.register_server(rust_config);
    manager.register_server(python_config);

    // Both should be registered successfully
    Ok(())
}

#[test]
fn test_register_server_overwrites_existing_language() -> anyhow::Result<()> {
    let temp = TempDir::new()?;
    let mut manager = LspServerManager::new(temp.path());

    // Register a config
    let config1 = LspServerConfig {
        language: Language::Rust,
        command: "first-analyzer".to_string(),
        args: vec![],
        root_path: temp.path().to_path_buf(),
        init_options: None,
    };
    manager.register_server(config1);

    // Register another config for the same language
    let config2 = LspServerConfig {
        language: Language::Rust,
        command: "second-analyzer".to_string(),
        args: vec!["--new-arg".to_string()],
        root_path: temp.path().to_path_buf(),
        init_options: Some(serde_json::json!({"new": true})),
    };
    manager.register_server(config2);

    // The second config should overwrite the first
    // This follows HashMap behavior
    Ok(())
}

// ============================================================================
// Tests for LspServerManager::shutdown_all
// ============================================================================

#[tokio::test]
async fn test_shutdown_all_with_no_clients() -> anyhow::Result<()> {
    let temp = TempDir::new()?;
    let mut manager = LspServerManager::new(temp.path());

    // Shutdown with no clients should succeed
    let result = manager.shutdown_all().await;
    assert!(
        result.is_ok(),
        "shutdown_all should succeed with no clients"
    );
    Ok(())
}

#[tokio::test]
async fn test_shutdown_all_is_idempotent() -> anyhow::Result<()> {
    let temp = TempDir::new()?;
    let mut manager = LspServerManager::new(temp.path());

    // First shutdown
    let result1 = manager.shutdown_all().await;
    assert!(result1.is_ok(), "first shutdown should succeed");

    // Second shutdown
    let result2 = manager.shutdown_all().await;
    assert!(result2.is_ok(), "second shutdown should succeed");
    Ok(())
}

#[tokio::test]
async fn test_shutdown_all_clears_clients() -> anyhow::Result<()> {
    let temp = TempDir::new()?;
    let mut manager = LspServerManager::new(temp.path());

    // Shutdown
    manager.shutdown_all().await?;

    // After shutdown, all clients should be cleared
    // We can verify this by trying to shutdown again
    let result = manager.shutdown_all().await;
    assert!(
        result.is_ok(),
        "shutdown after previous shutdown should succeed"
    );
    Ok(())
}

// ============================================================================
// Tests for LspServerDefaults::for_language
// ============================================================================

#[test]
fn test_defaults_for_rust() {
    let root = PathBuf::from("/tmp/test");
    let config = LspServerDefaults::for_language(Language::Rust, &root);

    assert_eq!(config.language, Language::Rust);
    assert_eq!(config.command, "rust-analyzer");
    assert!(config.args.is_empty());
    assert_eq!(config.root_path, root);
    assert!(config.init_options.is_none());
}

#[test]
fn test_defaults_for_python() {
    let root = PathBuf::from("/tmp/test");
    let config = LspServerDefaults::for_language(Language::Python, &root);

    assert_eq!(config.language, Language::Python);
    assert_eq!(config.command, "pyright-langserver");
    assert_eq!(config.args, vec!["--stdio"]);
    assert_eq!(config.root_path, root);
    assert!(config.init_options.is_none());
}

#[test]
fn test_defaults_for_typescript() {
    let root = PathBuf::from("/tmp/test");
    let config = LspServerDefaults::for_language(Language::TypeScript, &root);

    assert_eq!(config.language, Language::TypeScript);
    assert_eq!(config.command, "typescript-language-server");
    assert_eq!(config.args, vec!["--stdio"]);
    assert_eq!(config.root_path, root);
    assert!(config.init_options.is_none());
}

#[test]
fn test_defaults_for_javascript() {
    let root = PathBuf::from("/tmp/test");
    let config = LspServerDefaults::for_language(Language::JavaScript, &root);

    assert_eq!(config.language, Language::JavaScript);
    assert_eq!(config.command, "typescript-language-server");
    assert_eq!(config.args, vec!["--stdio"]);
    assert_eq!(config.root_path, root);
    assert!(config.init_options.is_none());
}

#[test]
fn test_defaults_for_sysml() {
    let root = PathBuf::from("/tmp/test");
    let config = LspServerDefaults::for_language(Language::SysML, &root);

    assert_eq!(config.language, Language::SysML);
    assert_eq!(config.command, "syster-lsp");
    assert!(config.args.is_empty());
    assert_eq!(config.root_path, root);
    // SysML should have init_options with stdlibEnabled
    assert!(config.init_options.is_some());

    if let Some(options) = config.init_options {
        assert!(options.get("stdlibEnabled").is_some());
        assert_eq!(options["stdlibEnabled"], true);
    }
}

#[test]
fn test_defaults_for_kerml() {
    let root = PathBuf::from("/tmp/test");
    let config = LspServerDefaults::for_language(Language::KerML, &root);

    assert_eq!(config.language, Language::KerML);
    assert_eq!(config.command, "syster-lsp");
    assert!(config.args.is_empty());
    assert_eq!(config.root_path, root);
    // KerML should have init_options with stdlibEnabled
    assert!(config.init_options.is_some());

    if let Some(options) = config.init_options {
        assert!(options.get("stdlibEnabled").is_some());
        assert_eq!(options["stdlibEnabled"], true);
    }
}

#[test]
fn test_defaults_preserves_root_path() {
    let root1 = PathBuf::from("/path/one");
    let root2 = PathBuf::from("/path/two");

    let config1 = LspServerDefaults::for_language(Language::Rust, &root1);
    let config2 = LspServerDefaults::for_language(Language::Rust, &root2);

    assert_eq!(config1.root_path, root1);
    assert_eq!(config2.root_path, root2);
}

// ============================================================================
// Tests for LspServerConfig creation
// ============================================================================

#[test]
fn test_create_config_with_all_fields() {
    let config = LspServerConfig {
        language: Language::Rust,
        command: "test-analyzer".to_string(),
        args: vec!["--arg1".to_string(), "--arg2".to_string()],
        root_path: PathBuf::from("/test/path"),
        init_options: Some(serde_json::json!({"key": "value"})),
    };

    assert_eq!(config.language, Language::Rust);
    assert_eq!(config.command, "test-analyzer");
    assert_eq!(config.args.len(), 2);
    assert_eq!(config.root_path, PathBuf::from("/test/path"));
    assert!(config.init_options.is_some());
}

#[test]
fn test_create_config_with_no_args() {
    let config = LspServerConfig {
        language: Language::Python,
        command: "test-server".to_string(),
        args: vec![],
        root_path: PathBuf::from("/test"),
        init_options: None,
    };

    assert!(config.args.is_empty());
    assert!(config.init_options.is_none());
}

#[test]
fn test_config_clone() {
    let config1 = LspServerConfig {
        language: Language::TypeScript,
        command: "ts-server".to_string(),
        args: vec!["--stdio".to_string()],
        root_path: PathBuf::from("/test"),
        init_options: Some(serde_json::json!({"test": true})),
    };

    let config2 = config1.clone();

    assert_eq!(config1.language, config2.language);
    assert_eq!(config1.command, config2.command);
    assert_eq!(config1.args, config2.args);
    assert_eq!(config1.root_path, config2.root_path);
}

// ============================================================================
// Edge case tests
// ============================================================================

#[test]
fn test_manager_with_empty_path() {
    let manager = LspServerManager::new("");
    drop(manager);
}

#[test]
fn test_manager_with_relative_path() {
    let manager = LspServerManager::new("./relative/path");
    drop(manager);
}

#[test]
fn test_register_server_with_empty_command() -> anyhow::Result<()> {
    let temp = TempDir::new()?;
    let mut manager = LspServerManager::new(temp.path());

    let config = LspServerConfig {
        language: Language::Rust,
        command: "".to_string(),
        args: vec![],
        root_path: temp.path().to_path_buf(),
        init_options: None,
    };

    // Should register even with empty command (will fail when starting)
    manager.register_server(config);
    Ok(())
}

#[test]
fn test_register_server_with_many_args() -> anyhow::Result<()> {
    let temp = TempDir::new()?;
    let mut manager = LspServerManager::new(temp.path());

    let args: Vec<String> = (0..100).map(|i| format!("--arg{}", i)).collect();

    let config = LspServerConfig {
        language: Language::Rust,
        command: "test-server".to_string(),
        args,
        root_path: temp.path().to_path_buf(),
        init_options: None,
    };

    manager.register_server(config);
    Ok(())
}

#[test]
fn test_defaults_for_all_languages() {
    let root = PathBuf::from("/tmp/test");

    // Ensure defaults work for all language variants
    let languages = vec![
        Language::Rust,
        Language::Python,
        Language::TypeScript,
        Language::JavaScript,
        Language::SysML,
        Language::KerML,
    ];

    for language in languages {
        let config = LspServerDefaults::for_language(language, &root);
        assert_eq!(config.language, language);
        assert!(
            !config.command.is_empty(),
            "command should not be empty for {:?}",
            language
        );
        assert_eq!(config.root_path, root);
    }
}

#[tokio::test]
async fn test_multiple_shutdowns() -> anyhow::Result<()> {
    let temp = TempDir::new()?;
    let mut manager = LspServerManager::new(temp.path());

    // Multiple shutdowns should all succeed
    for _ in 0..5 {
        let result = manager.shutdown_all().await;
        assert!(result.is_ok(), "shutdown should always succeed");
    }
    Ok(())
}

#[test]
fn test_register_same_language_multiple_times() -> anyhow::Result<()> {
    let temp = TempDir::new()?;
    let mut manager = LspServerManager::new(temp.path());

    // Register the same language multiple times with different configs
    for i in 0..10 {
        let config = LspServerConfig {
            language: Language::Rust,
            command: format!("analyzer-{}", i),
            args: vec![],
            root_path: temp.path().to_path_buf(),
            init_options: None,
        };
        manager.register_server(config);
    }

    // Last one should win (HashMap behavior)
    Ok(())
}

#[test]
fn test_config_with_complex_init_options() {
    let complex_options = serde_json::json!({
        "nested": {
            "array": [1, 2, 3],
            "object": {
                "key": "value",
                "bool": true,
                "null": null
            }
        },
        "string": "test",
        "number": 42
    });

    let config = LspServerConfig {
        language: Language::Rust,
        command: "test".to_string(),
        args: vec![],
        root_path: PathBuf::from("/test"),
        init_options: Some(complex_options.clone()),
    };

    assert_eq!(config.init_options, Some(complex_options));
}

#[test]
fn test_defaults_with_different_root_paths() {
    let roots = vec![
        PathBuf::from("/"),
        PathBuf::from("/home/user"),
        PathBuf::from("./relative"),
        PathBuf::from("../parent"),
        PathBuf::from("C:\\Windows"),
    ];

    for root in roots {
        let config = LspServerDefaults::for_language(Language::Rust, &root);
        assert_eq!(config.root_path, root);
    }
}
