#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only

use zk_contract_llm::{LlmError, LlmProvider};
use zk_llm_openai_compatible::OpenAiCompatible;

#[test]
fn missing_base_url_is_invalid_request() {
    let provider = OpenAiCompatible::new(String::new(), String::new(), "m".to_owned());
    assert!(matches!(provider.complete("hi"), Err(LlmError::InvalidRequest(_))));
}

#[test]
fn missing_model_is_invalid_request() {
    let provider = OpenAiCompatible::new("http://127.0.0.1:1/v1".to_owned(), String::new(), String::new());
    assert!(matches!(provider.complete("hi"), Err(LlmError::InvalidRequest(_))));
}

/// Live smoke test; run explicitly with env configured:
/// `cargo test -p zk-llm-openai-compatible -- --ignored live_completion`
#[test]
#[ignore = "requires a live OpenAI-compatible endpoint (local dev env)"]
fn live_completion() {
    let provider = zk_llm_openai_compatible::provider();
    let text = provider.complete("只回复两个字：你好").unwrap();
    assert!(!text.trim().is_empty());
}
