//! Tests for ClientState in state module

use std::ops::ControlFlow;

use async_lsp::LanguageClient;
use async_lsp::lsp_types::{
    LogMessageParams, MessageType, ProgressParams, ProgressParamsValue, ProgressToken,
    PublishDiagnosticsParams, ShowMessageParams, WorkDoneProgress, WorkDoneProgressBegin,
    WorkDoneProgressCreateParams, WorkDoneProgressEnd,
};
use futures::channel::oneshot;

use crate::lsp::state::ClientState;

#[test]
fn test_progress_with_indexing_token_end() {
    // Test that progress with indexing token and End state triggers the oneshot
    let (tx, mut rx) = oneshot::channel();
    let mut state = ClientState::new_for_test(Some(tx));

    let params = ProgressParams {
        token: ProgressToken::String("rustAnalyzer/Indexing".to_string()),
        value: ProgressParamsValue::WorkDone(WorkDoneProgress::End(WorkDoneProgressEnd {
            message: Some("Indexing complete".to_string()),
        })),
    };

    let result = state.progress(params);
    assert!(matches!(result, ControlFlow::Continue(())));

    // Verify oneshot was triggered
    match rx.try_recv() {
        Ok(Some(())) => (),
        Ok(None) => panic!("Channel was not triggered"),
        Err(_) => panic!("Channel error"),
    }
}

#[test]
fn test_progress_with_cache_priming_token_end() {
    // Test that progress with cache priming token and End state triggers the oneshot
    let (tx, mut rx) = oneshot::channel();
    let mut state = ClientState::new_for_test(Some(tx));

    let params = ProgressParams {
        token: ProgressToken::String("rustAnalyzer/cachePriming".to_string()),
        value: ProgressParamsValue::WorkDone(WorkDoneProgress::End(WorkDoneProgressEnd {
            message: None,
        })),
    };

    let result = state.progress(params);
    assert!(matches!(result, ControlFlow::Continue(())));

    // Verify oneshot was triggered
    match rx.try_recv() {
        Ok(Some(())) => (),
        Ok(None) => panic!("Channel was not triggered"),
        Err(_) => panic!("Channel error"),
    }
}

#[test]
fn test_progress_with_non_indexing_token() {
    // Test that progress with non-indexing token does not trigger oneshot
    let (tx, mut rx) = oneshot::channel();
    let mut state = ClientState::new_for_test(Some(tx));

    let params = ProgressParams {
        token: ProgressToken::String("some/other/token".to_string()),
        value: ProgressParamsValue::WorkDone(WorkDoneProgress::End(WorkDoneProgressEnd {
            message: None,
        })),
    };

    let result = state.progress(params);
    assert!(matches!(result, ControlFlow::Continue(())));

    // Verify oneshot was NOT triggered
    match rx.try_recv() {
        Ok(None) => (),
        Ok(Some(())) => panic!("Channel should not have been triggered"),
        Err(e) => panic!("Unexpected channel error: {:?}", e),
    }
}

#[test]
fn test_progress_with_indexing_token_begin() {
    // Test that progress with indexing token and Begin state does not trigger oneshot
    let (tx, mut rx) = oneshot::channel();
    let mut state = ClientState::new_for_test(Some(tx));

    let params = ProgressParams {
        token: ProgressToken::String("rustAnalyzer/Indexing".to_string()),
        value: ProgressParamsValue::WorkDone(WorkDoneProgress::Begin(WorkDoneProgressBegin {
            title: "Indexing".to_string(),
            cancellable: None,
            message: None,
            percentage: None,
        })),
    };

    let result = state.progress(params);
    assert!(matches!(result, ControlFlow::Continue(())));

    // Verify oneshot was NOT triggered (Begin state doesn't trigger)
    match rx.try_recv() {
        Ok(None) => (),
        Ok(Some(())) => panic!("Channel should not have been triggered for Begin state"),
        Err(e) => panic!("Unexpected channel error: {:?}", e),
    }
}

#[test]
fn test_progress_with_number_token() {
    // Test that progress with number token does not trigger oneshot
    let (tx, mut rx) = oneshot::channel();
    let mut state = ClientState::new_for_test(Some(tx));

    let params = ProgressParams {
        token: ProgressToken::Number(123),
        value: ProgressParamsValue::WorkDone(WorkDoneProgress::End(WorkDoneProgressEnd {
            message: None,
        })),
    };

    let result = state.progress(params);
    assert!(matches!(result, ControlFlow::Continue(())));

    // Verify oneshot was NOT triggered (number tokens don't match)
    match rx.try_recv() {
        Ok(None) => (),
        Ok(Some(())) => panic!("Channel should not have been triggered for number token"),
        Err(e) => panic!("Unexpected channel error: {:?}", e),
    }
}

#[test]
fn test_progress_multiple_calls_only_first_triggers() {
    // Test that only the first matching progress triggers the oneshot
    let (tx, mut rx) = oneshot::channel();
    let mut state = ClientState::new_for_test(Some(tx));

    // First call should trigger
    let params1 = ProgressParams {
        token: ProgressToken::String("rustAnalyzer/Indexing".to_string()),
        value: ProgressParamsValue::WorkDone(WorkDoneProgress::End(WorkDoneProgressEnd {
            message: None,
        })),
    };
    let result1 = state.progress(params1);
    assert!(matches!(result1, ControlFlow::Continue(())));

    // Second call should not trigger (oneshot already consumed)
    let params2 = ProgressParams {
        token: ProgressToken::String("rustAnalyzer/Indexing".to_string()),
        value: ProgressParamsValue::WorkDone(WorkDoneProgress::End(WorkDoneProgressEnd {
            message: None,
        })),
    };
    let result2 = state.progress(params2);
    assert!(matches!(result2, ControlFlow::Continue(())));

    // Verify only one signal was sent
    match rx.try_recv() {
        Ok(Some(())) => (),
        Ok(None) => panic!("Channel was not triggered"),
        Err(_) => panic!("Channel error"),
    }
}

#[test]
fn test_progress_with_no_sender() {
    // Test that progress works correctly when indexed_tx is None
    let mut state = ClientState::new_for_test(None);

    let params = ProgressParams {
        token: ProgressToken::String("rustAnalyzer/Indexing".to_string()),
        value: ProgressParamsValue::WorkDone(WorkDoneProgress::End(WorkDoneProgressEnd {
            message: None,
        })),
    };

    // Should not panic even without a sender
    let result = state.progress(params);
    assert!(matches!(result, ControlFlow::Continue(())));
}

#[test]
fn test_publish_diagnostics_returns_continue() {
    // Test that publish_diagnostics returns Continue
    let (tx, _rx) = oneshot::channel();
    let mut state = ClientState::new_for_test(Some(tx));

    let params = PublishDiagnosticsParams {
        uri: "file:///test.rs".parse().unwrap(),
        diagnostics: vec![],
        version: None,
    };

    let result = state.publish_diagnostics(params);
    assert!(matches!(result, ControlFlow::Continue(())));
}

#[test]
fn test_show_message_returns_continue() {
    // Test that show_message returns Continue
    let (tx, _rx) = oneshot::channel();
    let mut state = ClientState::new_for_test(Some(tx));

    let params = ShowMessageParams {
        typ: MessageType::INFO,
        message: "Test message".to_string(),
    };

    let result = state.show_message(params);
    assert!(matches!(result, ControlFlow::Continue(())));
}

#[test]
fn test_show_message_with_different_types() {
    // Test show_message with different message types
    let (tx, _rx) = oneshot::channel();
    let mut state = ClientState::new_for_test(Some(tx));

    // Test ERROR type
    let params = ShowMessageParams {
        typ: MessageType::ERROR,
        message: "Error message".to_string(),
    };
    let result = state.show_message(params);
    assert!(matches!(result, ControlFlow::Continue(())));

    // Test WARNING type
    let params = ShowMessageParams {
        typ: MessageType::WARNING,
        message: "Warning message".to_string(),
    };
    let result = state.show_message(params);
    assert!(matches!(result, ControlFlow::Continue(())));

    // Test LOG type
    let params = ShowMessageParams {
        typ: MessageType::LOG,
        message: "Log message".to_string(),
    };
    let result = state.show_message(params);
    assert!(matches!(result, ControlFlow::Continue(())));
}

#[test]
fn test_log_message_returns_continue() {
    // Test that log_message returns Continue
    let (tx, _rx) = oneshot::channel();
    let mut state = ClientState::new_for_test(Some(tx));

    let params = LogMessageParams {
        typ: MessageType::INFO,
        message: "Test log".to_string(),
    };

    let result = state.log_message(params);
    assert!(matches!(result, ControlFlow::Continue(())));
}

#[test]
fn test_log_message_with_different_types() {
    // Test log_message with different message types
    let (tx, _rx) = oneshot::channel();
    let mut state = ClientState::new_for_test(Some(tx));

    // Test ERROR type
    let params = LogMessageParams {
        typ: MessageType::ERROR,
        message: "Error log".to_string(),
    };
    let result = state.log_message(params);
    assert!(matches!(result, ControlFlow::Continue(())));

    // Test WARNING type
    let params = LogMessageParams {
        typ: MessageType::WARNING,
        message: "Warning log".to_string(),
    };
    let result = state.log_message(params);
    assert!(matches!(result, ControlFlow::Continue(())));
}

#[tokio::test]
async fn test_work_done_progress_create_returns_ok() {
    // Test that work_done_progress_create returns Ok
    let (tx, _rx) = oneshot::channel();
    let mut state = ClientState::new_for_test(Some(tx));

    let params = WorkDoneProgressCreateParams {
        token: ProgressToken::String("test-token".to_string()),
    };

    let result = state.work_done_progress_create(params).await;
    assert!(result.is_ok());
}

#[test]
fn test_new_router_creates_router() {
    // Test that new_router creates a router with the correct state
    let (tx, _rx) = oneshot::channel();
    let _router = ClientState::new_router(tx);

    // Router should be created successfully
    // We can't directly inspect the router internals, but we can verify it compiles
}

#[test]
fn test_on_stop_through_router_creation() {
    // Test that on_stop is properly registered in the router
    // Note: on_stop is private, so we test it through the router's creation
    // This test verifies the function exists and has the correct signature
    // The actual behavior is tested through integration testing with the router

    // Create a router to ensure on_stop is properly registered
    let (tx, _rx) = oneshot::channel();
    let _router = ClientState::new_router(tx);

    // The on_stop handler is registered in new_router via router.event(Self::on_stop)
    // This test ensures the code compiles and the router can be created with the handler
}
