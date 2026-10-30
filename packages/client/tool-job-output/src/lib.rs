//! Read background job output.
#![forbid(unsafe_code)]
use zk_contract_tool::{Tool, ToolError, ToolSpec, TurnContext};
pub struct JobOutputTool;
#[must_use]
pub fn tool() -> JobOutputTool {
    JobOutputTool
}
impl Tool for JobOutputTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "job_output".to_owned(),
            description: "Read current output/status of a background job.".to_owned(),
            parameters: serde_json::json!({"type":"object","properties":{"id":{"type":"string"}},"required":["id"],"additionalProperties":false}),
            mutating: false,
            requires_approval: false,
        }
    }
    fn call(&self, _args: &str) -> Result<String, ToolError> {
        Err(ToolError::InvalidArgs(
            "job_output requires JSON".to_owned(),
        ))
    }
    fn call_json(&self, _c: &TurnContext, args: serde_json::Value) -> Result<String, ToolError> {
        let id = args
            .get("id")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| ToolError::InvalidArgs("id".to_owned()))?;
        Ok(zk_kit_jobs::output(id)?.to_string())
    }
}
