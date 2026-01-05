//! Tests for shutdown_lsp function

use mother_core::lsp::LspServerManager;
use std::path::PathBuf;
use tempfile::TempDir;

// ============================================================================
// Tests for shutdown_lsp basic functionality
// ============================================================================

#[tokio::test]
async fn test_shutdown_lsp_with_no_clients() -> anyhow::Result<()> {
    let temp = TempDir::new()?;
    let mut manager = LspServerManager::new(temp.path());

    // shutdown_lsp should succeed even with no active clients
    crate::commands::scan::shutdown_lsp(&mut manager).await;

    // Function completes without panicking or returning error
    Ok(())
}

#[tokio::test]
async fn test_shutdown_lsp_is_idempotent() -> anyhow::Result<()> {
    let temp = TempDir::new()?;
    let mut manager = LspServerManager::new(temp.path());

    // Call shutdown_lsp multiple times
    crate::commands::scan::shutdown_lsp(&mut manager).await;
    crate::commands::scan::shutdown_lsp(&mut manager).await;
    crate::commands::scan::shutdown_lsp(&mut manager).await;

    // All calls should succeed without panicking
    Ok(())
}

#[tokio::test]
async fn test_shutdown_lsp_with_different_manager_instances() -> anyhow::Result<()> {
    let temp1 = TempDir::new()?;
    let temp2 = TempDir::new()?;
    let temp3 = TempDir::new()?;

    let mut manager1 = LspServerManager::new(temp1.path());
    let mut manager2 = LspServerManager::new(temp2.path());
    let mut manager3 = LspServerManager::new(temp3.path());

    // Shutdown different manager instances
    crate::commands::scan::shutdown_lsp(&mut manager1).await;
    crate::commands::scan::shutdown_lsp(&mut manager2).await;
    crate::commands::scan::shutdown_lsp(&mut manager3).await;

    // All should succeed independently
    Ok(())
}

#[tokio::test]
async fn test_shutdown_lsp_with_empty_path() {
    let mut manager = LspServerManager::new(PathBuf::from(""));

    // shutdown_lsp should handle manager with empty path
    crate::commands::scan::shutdown_lsp(&mut manager).await;
}

#[tokio::test]
async fn test_shutdown_lsp_with_nonexistent_path() {
    let mut manager = LspServerManager::new(PathBuf::from("/nonexistent/path/to/project"));

    // shutdown_lsp should handle manager with nonexistent path
    crate::commands::scan::shutdown_lsp(&mut manager).await;
}

// ============================================================================
// Tests for shutdown_lsp with valid paths
// ============================================================================

#[tokio::test]
async fn test_shutdown_lsp_with_temp_directory() -> anyhow::Result<()> {
    let temp = TempDir::new()?;
    let mut manager = LspServerManager::new(temp.path());

    // Shutdown with valid temporary directory
    crate::commands::scan::shutdown_lsp(&mut manager).await;
    Ok(())
}

#[tokio::test]
async fn test_shutdown_lsp_with_absolute_path() -> anyhow::Result<()> {
    let temp = TempDir::new()?;
    let abs_path = temp.path().canonicalize()?;
    let mut manager = LspServerManager::new(&abs_path);

    // Shutdown with absolute path
    crate::commands::scan::shutdown_lsp(&mut manager).await;
    Ok(())
}

#[tokio::test]
async fn test_shutdown_lsp_with_relative_path() {
    let mut manager = LspServerManager::new(PathBuf::from("."));

    // Shutdown with relative path (current directory)
    crate::commands::scan::shutdown_lsp(&mut manager).await;
}

// ============================================================================
// Tests for shutdown_lsp error handling behavior
// ============================================================================

#[tokio::test]
async fn test_shutdown_lsp_never_panics_on_shutdown_errors() -> anyhow::Result<()> {
    // Test that shutdown_lsp handles any errors from shutdown_all gracefully
    // Even if the underlying shutdown_all were to return an error,
    // shutdown_lsp should log it and continue without panicking

    let temp = TempDir::new()?;
    let mut manager = LspServerManager::new(temp.path());

    // This should never panic, even if there are internal errors
    crate::commands::scan::shutdown_lsp(&mut manager).await;
    Ok(())
}

#[tokio::test]
async fn test_shutdown_lsp_sequential_calls_after_first_shutdown() -> anyhow::Result<()> {
    let temp = TempDir::new()?;
    let mut manager = LspServerManager::new(temp.path());

    // First shutdown
    crate::commands::scan::shutdown_lsp(&mut manager).await;

    // Subsequent shutdowns should also succeed
    for _ in 0..10 {
        crate::commands::scan::shutdown_lsp(&mut manager).await;
    }
    Ok(())
}

// ============================================================================
// Tests for shutdown_lsp with various manager states
// ============================================================================

#[tokio::test]
async fn test_shutdown_lsp_with_freshly_created_manager() -> anyhow::Result<()> {
    // Test with a manager that was just created and never used
    let temp = TempDir::new()?;
    let mut manager = LspServerManager::new(temp.path());

    crate::commands::scan::shutdown_lsp(&mut manager).await;
    Ok(())
}

#[tokio::test]
async fn test_shutdown_lsp_doesnt_affect_manager_after_shutdown() -> anyhow::Result<()> {
    let temp = TempDir::new()?;
    let mut manager = LspServerManager::new(temp.path());

    // Shutdown
    crate::commands::scan::shutdown_lsp(&mut manager).await;

    // Manager should still be in a valid state for subsequent operations
    // We can verify by shutting down again
    crate::commands::scan::shutdown_lsp(&mut manager).await;
    Ok(())
}

// ============================================================================
// Concurrency tests
// ============================================================================

#[tokio::test]
async fn test_shutdown_lsp_with_multiple_managers_concurrently() -> anyhow::Result<()> {
    use tokio::task;

    let handles: Vec<_> = (0..5)
        .map(|_i| {
            task::spawn(async move {
                let temp = TempDir::new()?;
                let mut manager = LspServerManager::new(temp.path());
                crate::commands::scan::shutdown_lsp(&mut manager).await;
                drop(temp); // Explicitly ensure temp is owned by this task
                Ok::<(), anyhow::Error>(())
            })
        })
        .collect();

    // Wait for all tasks to complete
    for handle in handles {
        handle.await??;
    }
    Ok(())
}

#[tokio::test]
async fn test_shutdown_lsp_sequential_with_different_paths() -> anyhow::Result<()> {
    // Create multiple temporary directories and managers
    let temps: Vec<TempDir> = (0..5).map(|_| TempDir::new()).collect::<Result<_, _>>()?;

    let mut managers: Vec<LspServerManager> = temps
        .iter()
        .map(|temp| LspServerManager::new(temp.path()))
        .collect();

    // Shutdown all managers sequentially
    for manager in &mut managers {
        crate::commands::scan::shutdown_lsp(manager).await;
    }

    // All should have succeeded
    Ok(())
}

// ============================================================================
// Edge case tests
// ============================================================================

#[tokio::test]
async fn test_shutdown_lsp_with_root_path() {
    // Test with root path (may not have write permissions, but should handle gracefully)
    let mut manager = LspServerManager::new(PathBuf::from("/"));

    crate::commands::scan::shutdown_lsp(&mut manager).await;
}

#[tokio::test]
async fn test_shutdown_lsp_with_special_characters_in_path() -> anyhow::Result<()> {
    let temp = TempDir::new()?;
    let special_path = temp.path().join("test dir with spaces & special!chars");
    let mut manager = LspServerManager::new(&special_path);

    crate::commands::scan::shutdown_lsp(&mut manager).await;
    Ok(())
}

#[tokio::test]
async fn test_shutdown_lsp_with_unicode_path() -> anyhow::Result<()> {
    let temp = TempDir::new()?;
    let unicode_path = temp.path().join("テスト_测试_🦀");
    let mut manager = LspServerManager::new(&unicode_path);

    crate::commands::scan::shutdown_lsp(&mut manager).await;
    Ok(())
}

#[tokio::test]
async fn test_shutdown_lsp_with_very_long_path() -> anyhow::Result<()> {
    let temp = TempDir::new()?;
    // Create a very long path
    let long_segment = "very_long_directory_name_".repeat(10);
    let long_path = temp.path().join(long_segment);
    let mut manager = LspServerManager::new(&long_path);

    crate::commands::scan::shutdown_lsp(&mut manager).await;
    Ok(())
}

// ============================================================================
// State verification tests
// ============================================================================

#[tokio::test]
async fn test_shutdown_lsp_completes_quickly() -> anyhow::Result<()> {
    use std::time::Instant;

    let temp = TempDir::new()?;
    let mut manager = LspServerManager::new(temp.path());

    let start = Instant::now();
    crate::commands::scan::shutdown_lsp(&mut manager).await;
    let duration = start.elapsed();

    // Shutdown with no clients should be very fast (< 1 second)
    assert!(
        duration.as_secs() < 1,
        "shutdown_lsp took too long: {:?}",
        duration
    );
    Ok(())
}

#[tokio::test]
async fn test_shutdown_lsp_multiple_times_stays_fast() -> anyhow::Result<()> {
    use std::time::Instant;

    let temp = TempDir::new()?;
    let mut manager = LspServerManager::new(temp.path());

    // First shutdown
    crate::commands::scan::shutdown_lsp(&mut manager).await;

    // Measure subsequent shutdowns
    let start = Instant::now();
    for _ in 0..100 {
        crate::commands::scan::shutdown_lsp(&mut manager).await;
    }
    let duration = start.elapsed();

    // 100 shutdowns should complete in reasonable time
    assert!(
        duration.as_secs() < 1,
        "100 shutdown_lsp calls took too long: {:?}",
        duration
    );
    Ok(())
}
