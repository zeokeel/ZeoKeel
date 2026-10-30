//! Linear workflow over subagents.
#![forbid(unsafe_code)]
use zk_contract_tool::{Tool, ToolError, ToolSpec, TurnContext};
pub struct WorkflowTool;
#[must_use] pub fn tool() -> WorkflowTool { WorkflowTool }
impl Tool for WorkflowTool {
    fn spec(&self) -> ToolSpec { ToolSpec { name: "workflow".to_owned(), description: "Run a linear workflow: each step prompt is delegated to a child agent in order.".to_owned(), parameters: serde_json::json!({"type":"object","properties":{"steps":{"type":"array","items":{"type":"object","properties":{"id":{"type":"string"},"prompt":{"type":"string"}},"required":["prompt"]}}},"required":["steps"],"additionalProperties":false}), mutating: false, requires_approval: false } }
    fn call(&self, _args: &str) -> Result<String, ToolError> { Err(ToolError::InvalidArgs("workflow requires JSON".to_owned())) }
    fn call_json(&self, context: &TurnContext, args: serde_json::Value) -> Result<String, ToolError> {
        let runner = zk_kit_subagent::runner().ok_or_else(|| ToolError::Denied("subagent runner is not installed".to_owned()))?;
        let steps = args.get("steps").and_then(serde_json::Value::as_array).ok_or_else(|| ToolError::InvalidArgs("steps".to_owned()))?;
        if steps.is_empty() { return Err(ToolError::InvalidArgs("workflow has no steps".to_owned())); }
        if steps.len() > 20 { return Err(ToolError::InvalidArgs("workflow too long (max 20)".to_owned())); }
        let depth = zk_kit_subagent::current_depth();
        let mut results = Vec::new();
        for (index, step) in steps.iter().enumerate() {
            let prompt = step.get("prompt").and_then(serde_json::Value::as_str).ok_or_else(|| ToolError::InvalidArgs("step prompt".to_owned()))?;
            let id = step.get("id").and_then(serde_json::Value::as_str).map(str::to_owned).unwrap_or_else(|| format!("step-{}", index.saturating_add(1)));
            let text = zk_kit_subagent::with_depth(depth.saturating_add(1), || runner.run(prompt, context, depth)).map_err(ToolError::Io)?;
            results.push(serde_json::json!({"id":id,"result":text}));
        }
        Ok(serde_json::json!({"steps":results}).to_string())
    }
}
