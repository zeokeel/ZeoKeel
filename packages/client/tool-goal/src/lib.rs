//! Conversation goal tool with a bounded round driver (spec R9.1 / F3).
//!
//! `set`/`create` creates or replaces the goal (objective + `max_rounds`), `status`/`get` reads it,
//! `run` executes exactly one agent round through [`driver::GoalTurnRunner`] (or the shared
//! subagent runner) and persists the round counter, `cancel` stops it. See [`driver`] for the
//! restart-safe persisted semantics (default `max_rounds` 10, clamp 1..=100).
#![forbid(unsafe_code)]
pub mod driver;

use zk_contract_tool::{Tool, ToolError, ToolSpec, TurnContext};

pub struct GoalTool;
#[must_use]
pub fn tool() -> GoalTool {
    GoalTool
}

fn required_str<'a>(args: &'a serde_json::Value, key: &str) -> Result<&'a str, ToolError> {
    args.get(key)
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| ToolError::InvalidArgs(key.to_owned()))
}

fn optional_str<'a>(args: &'a serde_json::Value, key: &str) -> Option<&'a str> {
    args.get(key).and_then(serde_json::Value::as_str)
}

fn optional_u32(args: &serde_json::Value, key: &str) -> Option<u32> {
    args.get(key)
        .and_then(serde_json::Value::as_u64)
        .and_then(|value| u32::try_from(value).ok())
}

impl Tool for GoalTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "goal".to_owned(),
            description: "Conversation goal: set/status/run/cancel. `run` executes exactly one \
                bounded agent round (max_rounds default 10, clamp 1..=100), persists the round \
                counter, and stops on achieved/cancelled/max_rounds; restart resumes safely."
                .to_owned(),
            parameters: serde_json::json!({"type":"object","properties":{"action":{"type":"string","enum":["set","create","get","status","update","run","cancel"]},"objective":{"type":"string"},"max_rounds":{"type":"integer"},"status":{"type":"string","enum":["active","achieved","complete","cancelled","blocked"]},"reason":{"type":"string"},"rounds":{"type":"integer"}},"required":["action"],"additionalProperties":false}),
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
        let action = required_str(&args, "action")?;
        match action {
            "set" | "create" => {
                let objective = required_str(&args, "objective")?;
                let max_rounds = optional_u32(&args, "max_rounds");
                driver::set_goal(context, objective, max_rounds).map(|goal| goal.to_string())
            }
            "get" | "status" => driver::goal(context)?
                .map_or_else(|| Ok("null".to_owned()), |goal| Ok(goal.to_string())),
            "update" => driver::update_goal(
                context,
                optional_str(&args, "status"),
                optional_str(&args, "reason"),
                optional_u32(&args, "rounds"),
            )
            .map(|goal| goal.to_string()),
            "cancel" => driver::cancel_goal(context).map(|goal| goal.to_string()),
            "run" => driver::run_round(context).map(|report| report.to_json().to_string()),
            other => Err(ToolError::InvalidArgs(format!("unknown action {other}"))),
        }
    }
}
