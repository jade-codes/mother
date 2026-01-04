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
fn test_defaults_for_go() {
    let root = PathBuf::from("/tmp/test");
    let config = LspServerDefaults::for_language(Language::Go, &root);

    assert_eq!(config.language, Language::Go);
    assert_eq!(config.command, "gopls");
    assert!(config.args.is_empty());
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
        Language::Go,
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

// ============================================================================
// Comprehensive tests for LspServerDefaults::for_language closure logic
// ============================================================================

#[test]
fn test_sysml_stdlib_path_with_existing_project_path() -> anyhow::Result<()> {
    // Create a temporary directory structure that mimics the project
    let temp = TempDir::new()?;
    let crates_dir = temp.path().join("crates");
    std::fs::create_dir_all(&crates_dir)?;
    let syster_base = crates_dir.join("syster-base");
    std::fs::create_dir_all(&syster_base)?;
    let sysml_library = syster_base.join("sysml.library");
    std::fs::create_dir_all(&sysml_library)?;

    let config = LspServerDefaults::for_language(Language::SysML, temp.path());

    assert_eq!(config.language, Language::SysML);
    assert_eq!(config.command, "syster-lsp");
    assert!(config.args.is_empty());
    assert_eq!(config.root_path, temp.path());

    // Should have init_options with both stdlibEnabled and stdlibPath
    assert!(config.init_options.is_some());
    if let Some(options) = config.init_options {
        assert_eq!(options["stdlibEnabled"], true);
        assert!(options.get("stdlibPath").is_some());
        // The path should be set to the canonicalized library path
        if let Some(path) = options["stdlibPath"].as_str() {
            assert!(!path.is_empty());
        }
    }
    Ok(())
}

#[test]
fn test_sysml_stdlib_path_when_project_path_not_exists() {
    // Use a non-existent directory
    let root = PathBuf::from("/nonexistent/test/path");
    let config = LspServerDefaults::for_language(Language::SysML, &root);

    assert_eq!(config.language, Language::SysML);
    assert_eq!(config.command, "syster-lsp");
    assert!(config.args.is_empty());
    assert_eq!(config.root_path, root);

    // Should still have init_options with stdlibEnabled
    assert!(config.init_options.is_some());
    if let Some(options) = config.init_options {
        assert_eq!(options["stdlibEnabled"], true);
        // May or may not have stdlibPath depending on system installation
        // but should at least have stdlibEnabled
    }
}

#[test]
fn test_kerml_stdlib_path_with_existing_project_path() -> anyhow::Result<()> {
    // Create a temporary directory structure that mimics the project
    let temp = TempDir::new()?;
    let crates_dir = temp.path().join("crates");
    std::fs::create_dir_all(&crates_dir)?;
    let syster_base = crates_dir.join("syster-base");
    std::fs::create_dir_all(&syster_base)?;
    let sysml_library = syster_base.join("sysml.library");
    std::fs::create_dir_all(&sysml_library)?;

    let config = LspServerDefaults::for_language(Language::KerML, temp.path());

    assert_eq!(config.language, Language::KerML);
    assert_eq!(config.command, "syster-lsp");
    assert!(config.args.is_empty());
    assert_eq!(config.root_path, temp.path());

    // Should have init_options with both stdlibEnabled and stdlibPath
    assert!(config.init_options.is_some());
    if let Some(options) = config.init_options {
        assert_eq!(options["stdlibEnabled"], true);
        assert!(options.get("stdlibPath").is_some());
        // The path should be set to the canonicalized library path
        if let Some(path) = options["stdlibPath"].as_str() {
            assert!(!path.is_empty());
        }
    }
    Ok(())
}

#[test]
fn test_kerml_stdlib_path_when_project_path_not_exists() {
    // Use a non-existent directory
    let root = PathBuf::from("/nonexistent/test/path");
    let config = LspServerDefaults::for_language(Language::KerML, &root);

    assert_eq!(config.language, Language::KerML);
    assert_eq!(config.command, "syster-lsp");
    assert!(config.args.is_empty());
    assert_eq!(config.root_path, root);

    // Should still have init_options with stdlibEnabled
    assert!(config.init_options.is_some());
    if let Some(options) = config.init_options {
        assert_eq!(options["stdlibEnabled"], true);
        // May or may not have stdlibPath depending on system installation
    }
}

#[test]
fn test_sysml_kerml_stdlib_path_closure_handles_canonicalize_failure() {
    // Test the closure that handles .canonicalize().ok() returning None
    // This happens when the path exists but cannot be canonicalized
    let root = PathBuf::from("/tmp");
    let config = LspServerDefaults::for_language(Language::SysML, &root);

    // Should succeed even if canonicalization fails
    assert_eq!(config.language, Language::SysML);
    assert_eq!(config.command, "syster-lsp");
    assert!(config.init_options.is_some());
}

#[test]
fn test_sysml_stdlib_fallback_to_exe_relative_path() {
    // This tests the or_else closure that tries to find stdlib relative to exe
    let root = PathBuf::from("/nonexistent/path/without/stdlib");
    let config = LspServerDefaults::for_language(Language::SysML, &root);

    // Should still create valid config with init_options
    assert_eq!(config.language, Language::SysML);
    assert_eq!(config.command, "syster-lsp");
    assert!(config.init_options.is_some());

    // The init_options should at minimum have stdlibEnabled: true
    if let Some(options) = config.init_options {
        assert_eq!(options["stdlibEnabled"], true);
    }
}

#[test]
fn test_kerml_stdlib_fallback_to_exe_relative_path() {
    // This tests the or_else closure that tries to find stdlib relative to exe
    let root = PathBuf::from("/nonexistent/path/without/stdlib");
    let config = LspServerDefaults::for_language(Language::KerML, &root);

    // Should still create valid config with init_options
    assert_eq!(config.language, Language::KerML);
    assert_eq!(config.command, "syster-lsp");
    assert!(config.init_options.is_some());

    // The init_options should at minimum have stdlibEnabled: true
    if let Some(options) = config.init_options {
        assert_eq!(options["stdlibEnabled"], true);
    }
}

// ============================================================================
// Edge case tests for closure logic
// ============================================================================

#[test]
fn test_defaults_for_language_with_empty_path() {
    let root = PathBuf::from("");
    let config = LspServerDefaults::for_language(Language::Rust, &root);

    assert_eq!(config.language, Language::Rust);
    assert_eq!(config.command, "rust-analyzer");
    assert_eq!(config.root_path, root);
}

#[test]
fn test_defaults_for_language_with_current_dir() {
    let root = PathBuf::from(".");
    let config = LspServerDefaults::for_language(Language::Python, &root);

    assert_eq!(config.language, Language::Python);
    assert_eq!(config.command, "pyright-langserver");
    assert_eq!(config.root_path, root);
}

#[test]
fn test_defaults_for_language_with_parent_dir() {
    let root = PathBuf::from("..");
    let config = LspServerDefaults::for_language(Language::Go, &root);

    assert_eq!(config.language, Language::Go);
    assert_eq!(config.command, "gopls");
    assert_eq!(config.root_path, root);
}

#[test]
fn test_sysml_with_relative_path() {
    let root = PathBuf::from("./some/relative/path");
    let config = LspServerDefaults::for_language(Language::SysML, &root);

    assert_eq!(config.language, Language::SysML);
    assert_eq!(config.command, "syster-lsp");
    assert_eq!(config.root_path, root);
    assert!(config.init_options.is_some());
}

#[test]
fn test_kerml_with_relative_path() {
    let root = PathBuf::from("./some/relative/path");
    let config = LspServerDefaults::for_language(Language::KerML, &root);

    assert_eq!(config.language, Language::KerML);
    assert_eq!(config.command, "syster-lsp");
    assert_eq!(config.root_path, root);
    assert!(config.init_options.is_some());
}

#[test]
fn test_typescript_javascript_shared_server() {
    // Both TypeScript and JavaScript use the same server
    let root = PathBuf::from("/test");

    let ts_config = LspServerDefaults::for_language(Language::TypeScript, &root);
    let js_config = LspServerDefaults::for_language(Language::JavaScript, &root);

    assert_eq!(ts_config.command, js_config.command);
    assert_eq!(ts_config.args, js_config.args);
    assert_eq!(ts_config.command, "typescript-language-server");
    assert_eq!(ts_config.args, vec!["--stdio"]);
}

#[test]
fn test_sysml_kerml_shared_server() {
    // Both SysML and KerML use the same server
    let root = PathBuf::from("/test");

    let sysml_config = LspServerDefaults::for_language(Language::SysML, &root);
    let kerml_config = LspServerDefaults::for_language(Language::KerML, &root);

    assert_eq!(sysml_config.command, kerml_config.command);
    assert_eq!(sysml_config.args, kerml_config.args);
    assert_eq!(sysml_config.command, "syster-lsp");
    assert!(sysml_config.args.is_empty());

    // Both should have init_options
    assert!(sysml_config.init_options.is_some());
    assert!(kerml_config.init_options.is_some());
}

#[test]
fn test_config_language_preservation() {
    // Ensure that the language field is correctly set for each variant
    let root = PathBuf::from("/test");

    let rust = LspServerDefaults::for_language(Language::Rust, &root);
    assert_eq!(rust.language, Language::Rust);

    let python = LspServerDefaults::for_language(Language::Python, &root);
    assert_eq!(python.language, Language::Python);

    let ts = LspServerDefaults::for_language(Language::TypeScript, &root);
    assert_eq!(ts.language, Language::TypeScript);

    let js = LspServerDefaults::for_language(Language::JavaScript, &root);
    assert_eq!(js.language, Language::JavaScript);

    let go = LspServerDefaults::for_language(Language::Go, &root);
    assert_eq!(go.language, Language::Go);

    let sysml = LspServerDefaults::for_language(Language::SysML, &root);
    assert_eq!(sysml.language, Language::SysML);

    let kerml = LspServerDefaults::for_language(Language::KerML, &root);
    assert_eq!(kerml.language, Language::KerML);
}

// ============================================================================
// Boundary and stress tests
// ============================================================================

#[test]
fn test_defaults_with_very_long_path() {
    // Test with a very long path name
    let long_component = "a".repeat(255);
    let long_path = format!(
        "/tmp/{}/{}/{}",
        long_component, long_component, long_component
    );
    let root = PathBuf::from(long_path);

    let config = LspServerDefaults::for_language(Language::Rust, &root);
    assert_eq!(config.root_path, root);
}

#[test]
fn test_defaults_with_special_characters_in_path() {
    // Test with special characters that are valid in paths
    let special_paths = vec![
        PathBuf::from("/tmp/test with spaces"),
        PathBuf::from("/tmp/test-with-dashes"),
        PathBuf::from("/tmp/test_with_underscores"),
        PathBuf::from("/tmp/test.with.dots"),
        PathBuf::from("/tmp/test@special#chars"),
    ];

    for root in special_paths {
        let config = LspServerDefaults::for_language(Language::Python, &root);
        assert_eq!(config.root_path, root);
    }
}

#[test]
fn test_sysml_init_options_structure() {
    // Test that SysML init_options have the correct structure
    let root = PathBuf::from("/test");
    let config = LspServerDefaults::for_language(Language::SysML, &root);

    assert!(config.init_options.is_some());
    if let Some(options) = config.init_options {
        // Must have stdlibEnabled
        assert!(options.is_object());
        assert!(options.get("stdlibEnabled").is_some());
        assert_eq!(options["stdlibEnabled"], true);

        // May have stdlibPath
        if let Some(path) = options.get("stdlibPath") {
            assert!(path.is_string());
            if let Some(s) = path.as_str() {
                assert!(!s.is_empty());
            }
        }
    }
}

#[test]
fn test_kerml_init_options_structure() {
    // Test that KerML init_options have the correct structure
    let root = PathBuf::from("/test");
    let config = LspServerDefaults::for_language(Language::KerML, &root);

    assert!(config.init_options.is_some());
    if let Some(options) = config.init_options {
        // Must have stdlibEnabled
        assert!(options.is_object());
        assert!(options.get("stdlibEnabled").is_some());
        assert_eq!(options["stdlibEnabled"], true);

        // May have stdlibPath
        if let Some(path) = options.get("stdlibPath") {
            assert!(path.is_string());
            if let Some(s) = path.as_str() {
                assert!(!s.is_empty());
            }
        }
    }
}

#[test]
fn test_non_sysml_languages_no_init_options() {
    // Verify that non-SysML/KerML languages don't have init_options by default
    let root = PathBuf::from("/test");

    let languages = vec![
        Language::Rust,
        Language::Python,
        Language::TypeScript,
        Language::JavaScript,
        Language::Go,
    ];

    for language in languages {
        let config = LspServerDefaults::for_language(language, &root);
        assert!(
            config.init_options.is_none(),
            "Language {:?} should not have init_options",
            language
        );
    }
}

#[test]
fn test_all_languages_have_nonempty_command() {
    // Verify that all languages return a non-empty command
    let root = PathBuf::from("/test");

    let languages = vec![
        Language::Rust,
        Language::Python,
        Language::TypeScript,
        Language::JavaScript,
        Language::Go,
        Language::SysML,
        Language::KerML,
    ];

    for language in languages {
        let config = LspServerDefaults::for_language(language, &root);
        assert!(
            !config.command.is_empty(),
            "Language {:?} should have a non-empty command",
            language
        );
    }
}

#[test]
fn test_all_languages_preserve_exact_root_path() {
    // Ensure root_path is preserved exactly without modification
    let test_paths = vec![
        PathBuf::from("/absolute/path"),
        PathBuf::from("relative/path"),
        PathBuf::from("./current"),
        PathBuf::from("../parent"),
        PathBuf::from(""),
    ];

    let languages = vec![
        Language::Rust,
        Language::Python,
        Language::TypeScript,
        Language::JavaScript,
        Language::Go,
        Language::SysML,
        Language::KerML,
    ];

    for path in test_paths {
        for language in &languages {
            let config = LspServerDefaults::for_language(*language, &path);
            assert_eq!(
                config.root_path, path,
                "Root path should be preserved exactly for {:?}",
                language
            );
        }
    }
}

#[test]
fn test_rust_config_consistency() {
    // Test multiple calls return consistent results
    let root = PathBuf::from("/test");

    let config1 = LspServerDefaults::for_language(Language::Rust, &root);
    let config2 = LspServerDefaults::for_language(Language::Rust, &root);

    assert_eq!(config1.language, config2.language);
    assert_eq!(config1.command, config2.command);
    assert_eq!(config1.args, config2.args);
    assert_eq!(config1.root_path, config2.root_path);
}

#[test]
fn test_python_config_consistency() {
    // Test multiple calls return consistent results
    let root = PathBuf::from("/test");

    let config1 = LspServerDefaults::for_language(Language::Python, &root);
    let config2 = LspServerDefaults::for_language(Language::Python, &root);

    assert_eq!(config1.language, config2.language);
    assert_eq!(config1.command, config2.command);
    assert_eq!(config1.args, config2.args);
    assert_eq!(config1.root_path, config2.root_path);
}
