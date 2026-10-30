//! Workspace bash tool.
#![forbid(unsafe_code)]
use zk_contract_tool::{Tool, ToolError, ToolSpec, TurnContext};

pub struct BashTool;
#[must_use] pub fn tool() -> BashTool { BashTool }
impl Tool for BashTool {
    fn spec(&self) -> ToolSpec { ToolSpec { name: "bash".to_owned(), description: "Run a bash command in the workspace (bounded output, timeout). Requires approval.".to_owned(), parameters: serde_json::json!({"type":"object","properties":{"command":{"type":"string"},"cwd":{"type":"string"},"timeout_ms":{"type":"integer"},"run_in_background":{"type":"boolean"}},"required":["command"],"additionalProperties":false}), mutating: true, requires_approval: true } }
    fn call(&self, _args: &str) -> Result<String, ToolError> { Err(ToolError::InvalidArgs("bash requires JSON arguments".to_owned())) }
    fn call_json(&self, context: &TurnContext, args: serde_json::Value) -> Result<String, ToolError> {
        let command = args.get("command").and_then(serde_json::Value::as_str).ok_or_else(|| ToolError::InvalidArgs("command".to_owned()))?;
        let cwd = args.get("cwd").and_then(serde_json::Value::as_str);
        let timeout = args.get("timeout_ms").and_then(serde_json::Value::as_u64);
        if args.get("run_in_background").and_then(serde_json::Value::as_bool) == Some(true) {
            let id = zk_kit_jobs::start(context, command, cwd)?;
            return Ok(serde_json::json!({"kind":"background","job_id":id,"command":command}).to_string());
        }
        zk_kit_tools::run_bash(context, command, cwd, timeout)
    }
}
