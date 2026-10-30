//! Forked subagent tool.
#![forbid(unsafe_code)]
use zk_contract_tool::{Tool, ToolError, ToolSpec, TurnContext};
pub struct SubagentForkTool;
#[must_use] pub fn tool() -> SubagentForkTool { SubagentForkTool }
impl Tool for SubagentForkTool {
    fn spec(&self) -> ToolSpec { ToolSpec { name: "subagent_fork".to_owned(), description: "Fork a child agent over the current conversation context and return its answer.".to_owned(), parameters: serde_json::json!({"type":"object","properties":{"prompt":{"type":"string"}},"required":["prompt"],"additionalProperties":false}), mutating: false, requires_approval: false } }
    fn call(&self, _args: &str) -> Result<String, ToolError> { Err(ToolError::InvalidArgs("subagent_fork requires JSON".to_owned())) }
    fn call_json(&self, context: &TurnContext, args: serde_json::Value) -> Result<String, ToolError> {
        let prompt = args.get("prompt").and_then(serde_json::Value::as_str).ok_or_else(|| ToolError::InvalidArgs("prompt".to_owned()))?;
        let runner = zk_kit_subagent::runner().ok_or_else(|| ToolError::Denied("subagent runner is not installed".to_owned()))?;
        let depth = zk_kit_subagent::current_depth();
        if depth >= zk_kit_subagent::MAX_DEPTH { return Err(ToolError::Denied("subagent depth limit reached".to_owned())); }
        zk_kit_subagent::with_depth(depth.saturating_add(1), || runner.fork(prompt, context, depth)).map_err(ToolError::Io)
    }
}
