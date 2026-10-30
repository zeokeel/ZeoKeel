//! Experimental agent team placeholder.
#![forbid(unsafe_code)]
use zk_contract_tool::{Tool, ToolError, ToolSpec, TurnContext};
pub struct AgentTeamTool;
#[must_use]
pub fn tool() -> AgentTeamTool {
    AgentTeamTool
}
impl Tool for AgentTeamTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "agent_team".to_owned(),
            description: "Experimental agent team (not enabled).".to_owned(),
            parameters: serde_json::json!({"type":"object","properties":{},"additionalProperties":false}),
            mutating: false,
            requires_approval: false,
        }
    }
    fn call(&self, _args: &str) -> Result<String, ToolError> {
        Err(ToolError::Denied(
            "TOOL_UNSUPPORTED_EXPERIMENTAL: agent team is not enabled".to_owned(),
        ))
    }
    fn call_json(&self, _c: &TurnContext, _a: serde_json::Value) -> Result<String, ToolError> {
        self.call("")
    }
}
