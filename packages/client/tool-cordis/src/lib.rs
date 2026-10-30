//! Disabled cordis inspection tool.
#![forbid(unsafe_code)]
use zk_contract_tool::{Tool, ToolError, ToolSpec, TurnContext};
pub struct CordisTool;
#[must_use]
pub fn tool() -> CordisTool {
    CordisTool
}
impl Tool for CordisTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "cordis_inspect_list".to_owned(),
            description: "Disabled by composition policy.".to_owned(),
            parameters: serde_json::json!({"type":"object","properties":{},"additionalProperties":false}),
            mutating: false,
            requires_approval: false,
        }
    }
    fn call(&self, _args: &str) -> Result<String, ToolError> {
        Err(ToolError::Denied(
            "TOOL_DISABLED_BY_POLICY: runtime plugin loading is forbidden".to_owned(),
        ))
    }
    fn call_json(&self, _c: &TurnContext, _a: serde_json::Value) -> Result<String, ToolError> {
        self.call("")
    }
}
