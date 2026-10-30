//! Workspace string-replace editor.
#![forbid(unsafe_code)]
use zk_contract_tool::{Tool, ToolError, ToolSpec, TurnContext};

pub struct FsEditTool;
#[must_use]
pub fn tool() -> FsEditTool {
    FsEditTool
}
impl Tool for FsEditTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "fs.edit".to_owned(),
            description:
                "Replace exactly one occurrence of old_string with new_string in a workspace file."
                    .to_owned(),
            parameters: serde_json::json!({"type":"object","properties":{"path":{"type":"string"},"old_string":{"type":"string"},"new_string":{"type":"string"}},"required":["path","old_string","new_string"],"additionalProperties":false}),
            mutating: true,
            requires_approval: true,
        }
    }
    fn call(&self, _args: &str) -> Result<String, ToolError> {
        Err(ToolError::InvalidArgs(
            "fs.edit requires JSON arguments".to_owned(),
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
        let old = args
            .get("old_string")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| ToolError::InvalidArgs("old_string".to_owned()))?;
        let new = args
            .get("new_string")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| ToolError::InvalidArgs("new_string".to_owned()))?;
        let target = zk_kit_tools::safe_path(context, path)?;
        let text = zk_kit_tools::read_text(&target)?;
        let count = text.matches(old).count();
        if count != 1 {
            return Err(ToolError::InvalidArgs(format!(
                "old_string must match exactly once (found {count})"
            )));
        }
        zk_kit_tools::write_text(&target, &text.replacen(old, new, 1))?;
        Ok(format!("edited {path}"))
    }
}
