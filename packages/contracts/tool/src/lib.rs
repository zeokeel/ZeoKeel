//! Tool contract (minimal M0 subset).
//!
//! @capability contract.tool
#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Human/model readable description of a tool.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolSpec {
    /// Stable tool name, e.g. `fs.read`.
    pub name: String,
    /// One-line description shown to the model and UI.
    pub description: String,
}

/// Closed error set for tool calls.
#[derive(Debug, Clone, PartialEq, Eq, Error, Serialize, Deserialize)]
pub enum ToolError {
    /// Tool name is not registered.
    #[error("tool not found: {0}")]
    NotFound(String),
    /// Underlying IO failure.
    #[error("tool io error: {0}")]
    Io(String),
    /// Path/argument denied by policy.
    #[error("tool denied: {0}")]
    Denied(String),
    /// Arguments are not valid for this tool.
    #[error("tool invalid arguments: {0}")]
    InvalidArgs(String),
}

/// A replaceable capability executed by the local agent loop.
pub trait Tool: Send + Sync {
    /// Static description.
    fn spec(&self) -> ToolSpec;
    /// Execute with a raw argument string; implementations validate and parse.
    fn call(&self, args: &str) -> Result<String, ToolError>;
}
