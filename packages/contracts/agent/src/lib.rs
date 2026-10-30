//! Local agent loop contract (minimal M0 subset).
//!
//! @capability contract.agent
#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// One executed tool call, recorded for UI and diagnostics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolCallRecord {
    /// Tool name, e.g. `fs.read`.
    pub name: String,
    /// Raw arguments passed to the tool.
    pub args: String,
    /// Tool output (or error text).
    pub result: String,
}

/// Result of one agent turn.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentReply {
    /// Final assistant text.
    pub text: String,
    /// Tool calls executed during the turn (empty when none).
    pub tool_calls: Vec<ToolCallRecord>,
}

/// Closed error set for the agent loop.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum AgentError {
    /// The provider failed.
    #[error("agent llm error: {0}")]
    Llm(String),
    /// A tool failed.
    #[error("agent tool error: {0}")]
    Tool(String),
    /// Input could not be interpreted.
    #[error("agent invalid input: {0}")]
    Invalid(String),
}

/// A replaceable local agent loop (state machine grows in M1).
pub trait Agent: Send + Sync {
    /// Run one turn for the given user input.
    fn run(&self, input: &str) -> Result<AgentReply, AgentError>;
}
