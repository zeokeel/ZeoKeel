//! Bounded ralph loop over subagents.
#![forbid(unsafe_code)]
use zk_contract_tool::{Tool, ToolError, ToolSpec, TurnContext};
pub struct RalphTool;
#[must_use] pub fn tool() -> RalphTool { RalphTool }
impl Tool for RalphTool {
    fn spec(&self) -> ToolSpec { ToolSpec { name: "ralph".to_owned(), description: "Run up to max_rounds fresh subagent rounds toward one objective.".to_owned(), parameters: serde_json::json!({"type":"object","properties":{"objective":{"type":"string"},"max_rounds":{"type":"integer"}},"required":["objective"],"additionalProperties":false}), mutating: false, requires_approval: false } }
    fn call(&self, _args: &str) -> Result<String, ToolError> { Err(ToolError::InvalidArgs("ralph requires JSON".to_owned())) }
    fn call_json(&self, context: &TurnContext, args: serde_json::Value) -> Result<String, ToolError> {
        let runner = zk_kit_subagent::runner().ok_or_else(|| ToolError::Denied("subagent runner is not installed".to_owned()))?;
        let objective = args.get("objective").and_then(serde_json::Value::as_str).ok_or_else(|| ToolError::InvalidArgs("objective".to_owned()))?;
        let rounds = args.get("max_rounds").and_then(serde_json::Value::as_u64).unwrap_or(16).clamp(1, 64);
        let depth = zk_kit_subagent::current_depth();
        let mut results = Vec::new();
        for round in 1..=rounds {
            let prompt = format!("Ralph round {round}/{rounds}. Objective:\n{objective}\nWork toward it. Report concrete progress and blockers.");
            let text = zk_kit_subagent::with_depth(depth.saturating_add(1), || runner.run(&prompt, context, depth)).map_err(ToolError::Io)?;
            results.push(serde_json::json!({"round":round,"result":text}));
        }
        Ok(serde_json::json!({"rounds":results}).to_string())
    }
}
