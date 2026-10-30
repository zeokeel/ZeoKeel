//! Contract conformance: provider trait object + error variants.
#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only

use std::sync::Arc;
use zk_contract_llm::{LlmError, LlmProvider};

struct Fixed;

impl LlmProvider for Fixed {
    fn complete(&self, prompt: &str) -> Result<String, LlmError> {
        if prompt.is_empty() {
            return Err(LlmError::InvalidRequest("empty".to_owned()));
        }
        Ok(format!("echo:{prompt}"))
    }
}

#[test]
fn provider_is_object_safe() {
    let p: Arc<dyn LlmProvider> = Arc::new(Fixed);
    assert_eq!(p.complete("hi").unwrap(), "echo:hi");
    assert!(matches!(p.complete(""), Err(LlmError::InvalidRequest(_))));
}
