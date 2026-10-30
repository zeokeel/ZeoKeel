//! List background jobs.
#![forbid(unsafe_code)]
use zk_contract_tool::{Tool, ToolError, ToolSpec, TurnContext};
pub struct JobListTool;
#[must_use]
pub fn tool() -> JobListTool {
    JobListTool
}
impl Tool for JobListTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "job_list".to_owned(),
            description: "List background jobs with status.".to_owned(),
            parameters: serde_json::json!({"type":"object","properties":{},"additionalProperties":false}),
            mutating: false,
            requires_approval: false,
        }
    }
    fn call(&self, _args: &str) -> Result<String, ToolError> {
        Err(ToolError::InvalidArgs(
            "job_list takes no arguments".to_owned(),
        ))
    }
    fn call_json(&self, _c: &TurnContext, _a: serde_json::Value) -> Result<String, ToolError> {
        Ok(zk_kit_jobs::list()?.to_string())
    }
}
