//! Structured extraction tool.
#![forbid(unsafe_code)]
use zk_contract_tool::{Tool, ToolError, ToolSpec, TurnContext};
pub struct ExtractTool;
#[must_use] pub fn tool() -> ExtractTool { ExtractTool }
impl Tool for ExtractTool {
    fn spec(&self) -> ToolSpec { ToolSpec { name: "extract_json".to_owned(), description: "Extract structured JSON from text using the model.".to_owned(), parameters: serde_json::json!({"type":"object","properties":{"text":{"type":"string"}},"required":["text"],"additionalProperties":false}), mutating: false, requires_approval: false } }
    fn call(&self, _args: &str) -> Result<String, ToolError> { Err(ToolError::InvalidArgs("extract_json requires JSON".to_owned())) }
    fn call_json(&self, context: &TurnContext, args: serde_json::Value) -> Result<String, ToolError> {
        let text = args.get("text").and_then(serde_json::Value::as_str).ok_or_else(|| ToolError::InvalidArgs("text".to_owned()))?;
        let runner = zk_kit_extractor::runner().ok_or_else(|| ToolError::Denied("extractor runner unavailable".to_owned()))?;
        runner.extract(text, context).map(|value| value.to_string()).map_err(ToolError::Io)
    }
}
