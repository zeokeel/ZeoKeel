//! Regex content grep.
#![forbid(unsafe_code)]
use zk_contract_tool::{Tool, ToolError, ToolSpec, TurnContext};
pub struct GrepTool;
#[must_use]
pub fn tool() -> GrepTool {
    GrepTool
}
impl Tool for GrepTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "grep".to_owned(),
            description: "Search file contents with a regex (max 250 matches).".to_owned(),
            parameters: serde_json::json!({"type":"object","properties":{"pattern":{"type":"string"},"path":{"type":"string"},"include":{"type":"string"}},"required":["pattern"],"additionalProperties":false}),
            mutating: false,
            requires_approval: false,
        }
    }
    fn call(&self, _args: &str) -> Result<String, ToolError> {
        Err(ToolError::InvalidArgs("grep requires JSON".to_owned()))
    }
    fn call_json(
        &self,
        context: &TurnContext,
        args: serde_json::Value,
    ) -> Result<String, ToolError> {
        let pattern = args
            .get("pattern")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| ToolError::InvalidArgs("pattern".to_owned()))?;
        zk_kit_tools::grep_files(
            context,
            pattern,
            args.get("path").and_then(serde_json::Value::as_str),
            args.get("include").and_then(serde_json::Value::as_str),
        )
    }
}
