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
    /// JSON Schema for the tool arguments (rig/OpenAI tools).
    #[serde(default)]
    pub parameters: serde_json::Value,
    /// Whether the tool may mutate the workspace/system.
    #[serde(default)]
    pub mutating: bool,
    /// Whether the tool needs explicit user approval before execution.
    #[serde(default)]
    pub requires_approval: bool,
}

/// Per-turn workspace context passed to tools.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TurnContext {
    /// Conversation id (empty for legacy calls).
    pub conversation_id: String,
    /// Workspace id.
    pub workspace_id: String,
    /// Canonical workspace root; tools resolve relative paths against it.
    pub workspace_root: String,
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

/// Approval gate for mutating tool calls. The desktop/UI implements it.
pub trait ApprovalGate: Send + Sync {
    /// Ask the user; implementations may block. Fail closed on timeout/error.
    fn decide(&self, approval_id: &str, tool: &str, args: &serde_json::Value, reason: &str)
        -> bool;
}

/// Deny everything (safe default).
pub struct DenyAll;

impl ApprovalGate for DenyAll {
    fn decide(
        &self,
        _approval_id: &str,
        _tool: &str,
        _args: &serde_json::Value,
        _reason: &str,
    ) -> bool {
        false
    }
}

/// A replaceable capability executed by the local agent loop.
pub trait Tool: Send + Sync {
    /// Static description.
    fn spec(&self) -> ToolSpec;
    /// Execute with a raw argument string; implementations validate and parse.
    fn call(&self, args: &str) -> Result<String, ToolError>;

    /// Execute within a workspace context. Defaults to `call` for tools that do not care.
    fn call_in(&self, _context: &TurnContext, args: &str) -> Result<String, ToolError> {
        self.call(args)
    }

    /// Execute with JSON arguments (agent tool calling). Tools that implement a
    /// real schema should override this; the default keeps legacy string tools usable.
    fn call_json(
        &self,
        context: &TurnContext,
        args: serde_json::Value,
    ) -> Result<String, ToolError> {
        self.call_in(context, &args.to_string())
    }
}
