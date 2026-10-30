//! Working directory tool.
#![forbid(unsafe_code)]
use zk_contract_tool::{Tool, ToolError, ToolSpec, TurnContext};
pub struct WorkingDirectoryTool;
#[must_use] pub fn tool() -> WorkingDirectoryTool { WorkingDirectoryTool }
impl Tool for WorkingDirectoryTool {
    fn spec(&self) -> ToolSpec { ToolSpec { name: "working_directory".to_owned(), description: "Return the current workspace root.".to_owned(), parameters: serde_json::json!({"type":"object","properties":{},"additionalProperties":false}), mutating: false, requires_approval: false } }
    fn call(&self, _args: &str) -> Result<String, ToolError> { Err(ToolError::InvalidArgs("working_directory takes no arguments".to_owned())) }
    fn call_json(&self, context: &TurnContext, _args: serde_json::Value) -> Result<String, ToolError> { Ok(zk_kit_tools::workspace_root(context)?.display().to_string()) }
}
