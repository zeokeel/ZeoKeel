//! Persistent PowerShell placeholder (explicitly unsupported).
#![forbid(unsafe_code)]
use zk_contract_tool::{Tool, ToolError, ToolSpec, TurnContext};
pub struct PwshPersistentTool;
#[must_use] pub fn tool() -> PwshPersistentTool { PwshPersistentTool }
impl Tool for PwshPersistentTool {
    fn spec(&self) -> ToolSpec { ToolSpec { name: "pwsh_persistent".to_owned(), description: "Persistent PowerShell (unsupported in this build).".to_owned(), parameters: serde_json::json!({"type":"object","properties":{"command":{"type":"string"}},"required":["command"],"additionalProperties":false}), mutating: true, requires_approval: true } }
    fn call(&self, _args: &str) -> Result<String, ToolError> { Err(ToolError::Denied("TOOL_UNSUPPORTED_PLATFORM: pwsh persistent is not implemented".to_owned())) }
    fn call_json(&self, _c: &TurnContext, _a: serde_json::Value) -> Result<String, ToolError> { self.call("") }
}
