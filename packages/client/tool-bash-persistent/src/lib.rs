//! Persistent bash shell.
#![forbid(unsafe_code)]
use zk_contract_tool::{Tool, ToolError, ToolSpec, TurnContext};
pub struct BashPersistentTool;
#[must_use] pub fn tool() -> BashPersistentTool { BashPersistentTool }
impl Tool for BashPersistentTool {
    fn spec(&self) -> ToolSpec { ToolSpec { name: "bash_persistent".to_owned(), description: "Run commands in a persistent bash shell; cwd/env persist across calls.".to_owned(), parameters: serde_json::json!({"type":"object","properties":{"command":{"type":"string"},"timeout_ms":{"type":"integer"}},"required":["command"],"additionalProperties":false}), mutating: true, requires_approval: true } }
    fn call(&self, _args: &str) -> Result<String, ToolError> { Err(ToolError::InvalidArgs("bash_persistent requires JSON".to_owned())) }
    fn call_json(&self, context: &TurnContext, args: serde_json::Value) -> Result<String, ToolError> {
        let command = args.get("command").and_then(serde_json::Value::as_str).ok_or_else(|| ToolError::InvalidArgs("command".to_owned()))?;
        let timeout = args.get("timeout_ms").and_then(serde_json::Value::as_u64).unwrap_or(30_000);
        zk_kit_tools::persistent::run(context, command, timeout)
    }
}
