//! Kill a background job.
#![forbid(unsafe_code)]
use zk_contract_tool::{Tool, ToolError, ToolSpec, TurnContext};
pub struct JobKillTool;
#[must_use]
pub fn tool() -> JobKillTool {
    JobKillTool
}
impl Tool for JobKillTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "job_kill".to_owned(),
            description: "Kill a background job.".to_owned(),
            parameters: serde_json::json!({"type":"object","properties":{"id":{"type":"string"}},"required":["id"],"additionalProperties":false}),
            mutating: true,
            requires_approval: false,
        }
    }
    fn call(&self, _args: &str) -> Result<String, ToolError> {
        Err(ToolError::InvalidArgs("job_kill requires JSON".to_owned()))
    }
    fn call_json(&self, _c: &TurnContext, args: serde_json::Value) -> Result<String, ToolError> {
        let id = args
            .get("id")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| ToolError::InvalidArgs("id".to_owned()))?;
        Ok(zk_kit_jobs::kill(id)?.to_string())
    }
}
