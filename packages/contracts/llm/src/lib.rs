//! LLM provider contract (minimal M0 subset).
//!
//! @capability contract.llm
#![forbid(unsafe_code)]

use thiserror::Error;

/// Closed error set for provider calls.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum LlmError {
    /// Upstream/network failure.
    #[error("llm upstream error: {0}")]
    Upstream(String),
    /// Request rejected locally (bad config/input).
    #[error("llm invalid request: {0}")]
    InvalidRequest(String),
}

/// A replaceable model provider. Real providers (Responses API/Ollama) land in M1.
pub trait LlmProvider: Send + Sync {
    /// Complete a single prompt (non-streaming minimum).
    fn complete(&self, prompt: &str) -> Result<String, LlmError>;

    /// Complete while delivering text deltas as they arrive.
    ///
    /// Default implementation falls back to `complete` and emits the full text once;
    /// providers that support SSE should override it.
    fn complete_stream(
        &self,
        prompt: &str,
        on_delta: &mut dyn FnMut(&str),
    ) -> Result<String, LlmError> {
        let text = self.complete(prompt)?;
        on_delta(&text);
        Ok(text)
    }
}
