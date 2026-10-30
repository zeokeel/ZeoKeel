//! Glob file finder.
#![forbid(unsafe_code)]
use zk_contract_tool::{Tool, ToolError, ToolSpec, TurnContext};
pub struct GlobTool;
#[must_use] pub fn tool() -> GlobTool { GlobTool }
impl Tool for GlobTool {
    fn spec(&self) -> ToolSpec { ToolSpec { name: "glob".to_owned(), description: "Find files whose paths match a glob pattern (max 100).".to_owned(), parameters: serde_json::json!({"type":"object","properties":{"pattern":{"type":"string"},"path":{"type":"string"}},"required":["pattern"],"additionalProperties":false}), mutating: false, requires_approval: false } }
    fn call(&self, _args: &str) -> Result<String, ToolError> { Err(ToolError::InvalidArgs("glob requires JSON".to_owned())) }
    fn call_json(&self, context: &TurnContext, args: serde_json::Value) -> Result<String, ToolError> { let pattern = args.get("pattern").and_then(serde_json::Value::as_str).ok_or_else(|| ToolError::InvalidArgs("pattern".to_owned()))?; zk_kit_tools::glob_files(context, pattern, args.get("path").and_then(serde_json::Value::as_str)) }
}
