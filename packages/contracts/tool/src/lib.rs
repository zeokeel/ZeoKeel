//! Tool contract (minimal M0 subset).
//!
//! @capability contract.tool
#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Human/model readable description of a tool.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
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

/// One approval request's declared impact surface (tool-side only).
///
/// The tool that requires approval declares this via
/// [`Tool::approval_metadata`]; the agent runtime only aggregates it. All
/// fields are `#[serde(default)]` so older consumers can ignore the struct.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ApprovalMetadata {
    /// One-line action summary, e.g. `"write notes.txt"`.
    #[serde(default)]
    pub summary: String,
    /// Absolute working directory of the call.
    #[serde(default)]
    pub cwd: String,
    /// Paths the call will read (workspace-relative when possible).
    #[serde(default)]
    pub read_paths: Vec<String>,
    /// Paths the call will write (workspace-relative when possible).
    #[serde(default)]
    pub write_paths: Vec<String>,
    /// Network targets as `host[:port]` or `<mcp:server-id>`.
    #[serde(default)]
    pub network_targets: Vec<String>,
    /// `true` when the tool cannot statically determine its network targets
    /// (e.g. a free-form shell command).
    #[serde(default)]
    pub network_undetermined: bool,
    /// Risk level declared by the tool.
    #[serde(default)]
    pub risk: ApprovalRisk,
    /// Optional unified diff for write tools (tool-side, bounded by the tool).
    #[serde(default)]
    pub diff: Option<String>,
}

/// Closed risk level for one approval card.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalRisk {
    /// Read-only or trivially reversible action.
    Low,
    /// Mutates workspace state.
    #[default]
    Medium,
    /// Shell / network / external side effects.
    High,
}

/// User decision for one approval request.
///
/// Defined in the tool contract because [`ApprovalGate`] returns it; the agent
/// contract re-exports it to keep `zk_contract_agent::ApprovalDecision` working.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalDecision {
    /// Allow exactly this call.
    #[serde(alias = "approved")]
    AllowOnce,
    /// Allow this tool for the rest of the conversation (memory only).
    AllowSession,
    /// Reject this call (fail-closed default).
    #[serde(alias = "rejected")]
    Reject,
    /// Cancel the whole turn.
    Cancel,
}

impl ApprovalDecision {
    /// Whether this decision lets the tool run.
    #[must_use]
    pub fn is_allowed(self) -> bool {
        matches!(self, Self::AllowOnce | Self::AllowSession)
    }
}

/// Per-turn workspace context passed to tools.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct TurnContext {
    /// Conversation id (empty for legacy calls).
    pub conversation_id: String,
    /// Workspace id.
    pub workspace_id: String,
    /// Canonical workspace root; tools resolve relative paths against it.
    pub workspace_root: String,
}

/// Closed error set for tool calls.
#[derive(Debug, Clone, PartialEq, Eq, Error, Serialize, Deserialize, schemars::JsonSchema)]
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

    /// Typed variant of [`ApprovalGate::decide`] used by the agent runtime.
    ///
    /// Defaults to mapping the legacy boolean onto `AllowOnce`/`Reject`, so
    /// existing third-party gates keep working unchanged.
    fn decide_scoped(
        &self,
        approval_id: &str,
        tool: &str,
        args: &serde_json::Value,
        reason: &str,
    ) -> ApprovalDecision {
        if self.decide(approval_id, tool, args, reason) {
            ApprovalDecision::AllowOnce
        } else {
            ApprovalDecision::Reject
        }
    }
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
    /// Argument keys whose values must be masked in approval cards and persisted
    /// tool rows (recursively matched by key name). Defaults to none.
    fn sensitive_arg_keys(&self) -> &'static [&'static str] {
        &[]
    }
    /// Declared impact surface for an approval card. Defaults to `None` (the UI
    /// then shows "no declared impact" instead of guessing).
    fn approval_metadata(
        &self,
        _args: &serde_json::Value,
        _context: &TurnContext,
    ) -> Option<ApprovalMetadata> {
        None
    }
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
