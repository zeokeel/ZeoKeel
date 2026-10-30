//! Persistent workspace goal.
#![forbid(unsafe_code)]
use zk_contract_tool::{Tool, ToolError, ToolSpec, TurnContext};

pub struct GoalTool;
#[must_use]
pub fn tool() -> GoalTool {
    GoalTool
}

fn path(context: &TurnContext) -> Result<std::path::PathBuf, ToolError> {
    zk_kit_tools::safe_path(context, ".zeokeel/goal.json")
}
fn load(context: &TurnContext) -> Result<serde_json::Value, ToolError> {
    let path = path(context)?;
    if !path.is_file() {
        return Ok(serde_json::Value::Null);
    }
    serde_json::from_str(&zk_kit_tools::read_text(&path)?)
        .map_err(|error| ToolError::InvalidArgs(error.to_string()))
}
fn save(context: &TurnContext, value: &serde_json::Value) -> Result<(), ToolError> {
    zk_kit_tools::write_text(&path(context)?, &value.to_string())
}

impl Tool for GoalTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "goal".to_owned(),
            description: "Get/create/update the persistent workspace goal.".to_owned(),
            parameters: serde_json::json!({"type":"object","properties":{"action":{"type":"string","enum":["get","create","update"]},"objective":{"type":"string"},"status":{"type":"string","enum":["active","complete","blocked"]},"reason":{"type":"string"},"rounds":{"type":"integer"}},"required":["action"],"additionalProperties":false}),
            mutating: true,
            requires_approval: false,
        }
    }
    fn call(&self, _args: &str) -> Result<String, ToolError> {
        Err(ToolError::InvalidArgs("goal requires JSON".to_owned()))
    }
    fn call_json(
        &self,
        context: &TurnContext,
        args: serde_json::Value,
    ) -> Result<String, ToolError> {
        let action = args
            .get("action")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| ToolError::InvalidArgs("action".to_owned()))?;
        match action {
            "get" => Ok(load(context)?.to_string()),
            "create" => {
                let objective = args
                    .get("objective")
                    .and_then(serde_json::Value::as_str)
                    .ok_or_else(|| ToolError::InvalidArgs("objective".to_owned()))?;
                let goal = serde_json::json!({"objective": objective, "status": "active", "rounds": 0, "created_ms": now_ms()});
                save(context, &goal)?;
                Ok(goal.to_string())
            }
            "update" => {
                let mut goal = load(context)?;
                if goal.is_null() {
                    return Err(ToolError::InvalidArgs("no goal; create first".to_owned()));
                }
                let object = goal
                    .as_object_mut()
                    .ok_or_else(|| ToolError::InvalidArgs("goal store corrupted".to_owned()))?;
                if let Some(status) = args.get("status").and_then(serde_json::Value::as_str) {
                    object.insert("status".to_owned(), serde_json::json!(status));
                }
                if let Some(reason) = args.get("reason").and_then(serde_json::Value::as_str) {
                    object.insert("reason".to_owned(), serde_json::json!(reason));
                }
                if let Some(rounds) = args.get("rounds").and_then(serde_json::Value::as_u64) {
                    object.insert("rounds".to_owned(), serde_json::json!(rounds));
                }
                save(context, &goal)?;
                Ok(goal.to_string())
            }
            other => Err(ToolError::InvalidArgs(format!("unknown action {other}"))),
        }
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| u64::try_from(duration.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0)
}
