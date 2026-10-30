//! Subagent control (limited).
#![forbid(unsafe_code)]
use zk_contract_tool::{Tool, ToolError, ToolSpec, TurnContext};
pub struct SubagentControlTool;
#[must_use]
pub fn tool() -> SubagentControlTool {
    SubagentControlTool
}
impl Tool for SubagentControlTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec { name: "list_agents".to_owned(), description: "List current subagent depth/limits; send/interrupt are unavailable in synchronous runners.".to_owned(), parameters: serde_json::json!({"type":"object","properties":{},"additionalProperties":false}), mutating: false, requires_approval: false }
    }
    fn call(&self, _args: &str) -> Result<String, ToolError> {
        Err(ToolError::InvalidArgs(
            "list_agents takes no arguments".to_owned(),
        ))
    }
    fn call_json(&self, _c: &TurnContext, _a: serde_json::Value) -> Result<String, ToolError> {
        let agents = zk_kit_subagent::list();
        Ok(serde_json::json!({"agents":agents,"max_depth":zk_kit_subagent::MAX_DEPTH,"note":"send_message/interrupt unavailable: synchronous runners"}).to_string())
    }
}
