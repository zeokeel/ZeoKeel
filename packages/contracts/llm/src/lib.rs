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
    /// Complete a single prompt (non-streaming minimum; streaming added in M1).
    fn complete(&self, prompt: &str) -> Result<String, LlmError>;
}
