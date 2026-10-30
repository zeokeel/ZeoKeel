//! Git worktree helper (runs git in the workspace).
#![forbid(unsafe_code)]
use zk_contract_tool::{Tool, ToolError, ToolSpec, TurnContext};
pub struct WorktreeTool;
#[must_use] pub fn tool() -> WorktreeTool { WorktreeTool }
impl Tool for WorktreeTool {
    fn spec(&self) -> ToolSpec { ToolSpec { name: "create_worktree".to_owned(), description: "Create/list/remove a git worktree inside the workspace (action + path/branch).".to_owned(), parameters: serde_json::json!({"type":"object","properties":{"action":{"type":"string","enum":["create","list","remove"]},"path":{"type":"string"},"branch":{"type":"string"}},"required":["action"],"additionalProperties":false}), mutating: true, requires_approval: true } }
    fn call(&self, _args: &str) -> Result<String, ToolError> { Err(ToolError::InvalidArgs("create_worktree requires JSON".to_owned())) }
    fn call_json(&self, context: &TurnContext, args: serde_json::Value) -> Result<String, ToolError> {
        let action = args.get("action").and_then(serde_json::Value::as_str).ok_or_else(|| ToolError::InvalidArgs("action".to_owned()))?;
        let command = match action {
            "list" => "git worktree list".to_owned(),
            "create" => { let path = args.get("path").and_then(serde_json::Value::as_str).ok_or_else(|| ToolError::InvalidArgs("path".to_owned()))?; let branch = args.get("branch").and_then(serde_json::Value::as_str).ok_or_else(|| ToolError::InvalidArgs("branch".to_owned()))?; format!("git worktree add -- {} {}", path, branch) }
            "remove" => { let path = args.get("path").and_then(serde_json::Value::as_str).ok_or_else(|| ToolError::InvalidArgs("path".to_owned()))?; format!("git worktree remove -- {}", path) }
            other => return Err(ToolError::InvalidArgs(format!("unknown action {other}"))),
        };
        zk_kit_tools::run_bash(context, &command, None, Some(30_000))
    }
}
