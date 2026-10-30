//! Workspace search tool.
#![forbid(unsafe_code)]
use zk_contract_tool::{Tool, ToolError, ToolSpec, TurnContext};

pub struct FsSearchTool;
#[must_use]
pub fn tool() -> FsSearchTool {
    FsSearchTool
}
impl Tool for FsSearchTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "fs.search".to_owned(),
            description: "Search the workspace by file name or line content (bounded).".to_owned(),
            parameters: serde_json::json!({"type":"object","properties":{"query":{"type":"string"},"path":{"type":"string"}},"required":["query"],"additionalProperties":false}),
            mutating: false,
            requires_approval: false,
        }
    }
    fn call(&self, _args: &str) -> Result<String, ToolError> {
        Err(ToolError::InvalidArgs(
            "fs.search requires JSON arguments".to_owned(),
        ))
    }
    fn call_json(
        &self,
        context: &TurnContext,
        args: serde_json::Value,
    ) -> Result<String, ToolError> {
        let query = args
            .get("query")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| ToolError::InvalidArgs("query".to_owned()))?;
        let path = args.get("path").and_then(serde_json::Value::as_str);
        zk_kit_tools::search(context, query, path)
    }
}
