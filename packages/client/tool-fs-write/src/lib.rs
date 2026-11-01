//! Workspace file write tool.
#![forbid(unsafe_code)]
use zk_contract_tool::{ApprovalMetadata, ApprovalRisk, Tool, ToolError, ToolSpec, TurnContext};

pub struct FsWriteTool;
#[must_use]
pub fn tool() -> FsWriteTool {
    FsWriteTool
}
impl Tool for FsWriteTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "fs.write".to_owned(),
            description: "Write a UTF-8 file inside the workspace (creates parent directories)."
                .to_owned(),
            parameters: serde_json::json!({"type":"object","properties":{"path":{"type":"string"},"content":{"type":"string"}},"required":["path","content"],"additionalProperties":false}),
            mutating: true,
            requires_approval: true,
        }
    }
    fn approval_metadata(
        &self,
        args: &serde_json::Value,
        context: &TurnContext,
    ) -> Option<ApprovalMetadata> {
        let path = args.get("path")?.as_str()?;
        let content = args.get("content")?.as_str()?;
        let target = zk_kit_tools::safe_path(context, path).ok()?;
        let relative = zk_kit_tools::workspace_relative(context, &target);
        let previous = zk_kit_tools::read_for_diff(&target);
        Some(ApprovalMetadata {
            summary: format!("write {relative}"),
            cwd: context.workspace_root.clone(),
            write_paths: vec![relative],
            risk: ApprovalRisk::Medium,
            diff: zk_kit_tools::approval_diff(previous.as_deref(), content),
            ..ApprovalMetadata::default()
        })
    }
    fn call(&self, _args: &str) -> Result<String, ToolError> {
        Err(ToolError::InvalidArgs(
            "fs.write requires JSON arguments".to_owned(),
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
        let content = args
            .get("content")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| ToolError::InvalidArgs("content".to_owned()))?;
        let target = zk_kit_tools::safe_path(context, path)?;
        zk_kit_tools::write_text(&target, content)?;
        Ok(format!("wrote {} bytes to {path}", content.len()))
    }
}
