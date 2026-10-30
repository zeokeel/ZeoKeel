//! Present-a-deliverable tool.
#![forbid(unsafe_code)]
use zk_contract_tool::{Tool, ToolError, ToolSpec, TurnContext};

pub struct PresentTool;
#[must_use]
pub fn tool() -> PresentTool {
    PresentTool
}
impl Tool for PresentTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "present".to_owned(),
            description: "Declare a final deliverable for the user (path + description)."
                .to_owned(),
            parameters: serde_json::json!({"type":"object","properties":{"path":{"type":"string"},"description":{"type":"string"}},"required":["path"],"additionalProperties":false}),
            mutating: false,
            requires_approval: false,
        }
    }
    fn call(&self, _args: &str) -> Result<String, ToolError> {
        Err(ToolError::InvalidArgs(
            "present requires JSON arguments".to_owned(),
        ))
    }
    fn call_json(
        &self,
        context: &TurnContext,
        args: serde_json::Value,
    ) -> Result<String, ToolError> {
        let path = args
            .get("path")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| ToolError::InvalidArgs("path".to_owned()))?;
        let target = zk_kit_tools::safe_path(context, path)?;
        if !target.exists() {
            return Err(ToolError::InvalidArgs(format!(
                "deliverable not found: {path}"
            )));
        }
        Ok(serde_json::json!({"presented": path, "description": args.get("description").and_then(serde_json::Value::as_str).unwrap_or("")}).to_string())
    }
}
