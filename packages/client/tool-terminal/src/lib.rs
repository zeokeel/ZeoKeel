//! Persistent terminal session (thin wrapper over kit persistent shell).
#![forbid(unsafe_code)]
use zk_contract_tool::{Tool, ToolError, ToolSpec, TurnContext};
pub struct TerminalTool;
#[must_use] pub fn tool() -> TerminalTool { TerminalTool }
impl Tool for TerminalTool {
    fn spec(&self) -> ToolSpec { ToolSpec { name: "terminal".to_owned(), description: "Open/send/close a persistent terminal session for this conversation.".to_owned(), parameters: serde_json::json!({"type":"object","properties":{"action":{"type":"string","enum":["open","status","send","close"]},"command":{"type":"string"}},"required":["action"],"additionalProperties":false}), mutating: true, requires_approval: true } }
    fn call(&self, _args: &str) -> Result<String, ToolError> { Err(ToolError::InvalidArgs("terminal requires JSON".to_owned())) }
    fn call_json(&self, context: &TurnContext, args: serde_json::Value) -> Result<String, ToolError> {
        let action = args.get("action").and_then(serde_json::Value::as_str).ok_or_else(|| ToolError::InvalidArgs("action".to_owned()))?;
        match action {
            "open" => { let out = zk_kit_tools::persistent::run(context, "pwd", 5000)?; Ok(format!("terminal open: {out}")) }
            "status" => Ok(format!("open={}", zk_kit_tools::persistent::is_open(context))),
            "send" => { let command = args.get("command").and_then(serde_json::Value::as_str).ok_or_else(|| ToolError::InvalidArgs("command".to_owned()))?; zk_kit_tools::persistent::run(context, command, 30_000) }
            "close" => Ok(format!("closed={}", zk_kit_tools::persistent::close(context)?)),
            other => Err(ToolError::InvalidArgs(format!("unknown action {other}"))),
        }
    }
}
