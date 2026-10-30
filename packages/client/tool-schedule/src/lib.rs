//! Persistent schedule definitions.
#![forbid(unsafe_code)]
use zk_contract_tool::{Tool, ToolError, ToolSpec, TurnContext};

pub struct ScheduleTool;
#[must_use] pub fn tool() -> ScheduleTool { ScheduleTool }

fn path(context: &TurnContext) -> Result<std::path::PathBuf, ToolError> { zk_kit_tools::safe_path(context, ".zeokeel/schedules.json") }
fn load(context: &TurnContext) -> Result<serde_json::Value, ToolError> {
    let path = path(context)?;
    if !path.is_file() { return Ok(serde_json::json!([])); }
    serde_json::from_str(&zk_kit_tools::read_text(&path)?).map_err(|error| ToolError::InvalidArgs(error.to_string()))
}
fn save(context: &TurnContext, value: &serde_json::Value) -> Result<(), ToolError> { zk_kit_tools::write_text(&path(context)?, &value.to_string()) }

impl Tool for ScheduleTool {
    fn spec(&self) -> ToolSpec { ToolSpec { name: "schedule".to_owned(), description: "Create/list/update/delete workspace schedule definitions.".to_owned(), parameters: serde_json::json!({"type":"object","properties":{"action":{"type":"string","enum":["create","list","update","delete"]},"id":{"type":"string"},"cron":{"type":"string"},"prompt":{"type":"string"},"enabled":{"type":"boolean"},"interval_seconds":{"type":"integer"}},"required":["action"],"additionalProperties":false}), mutating: true, requires_approval: false } }
    fn call(&self, _args: &str) -> Result<String, ToolError> { Err(ToolError::InvalidArgs("schedule requires JSON".to_owned())) }
    fn call_json(&self, context: &TurnContext, args: serde_json::Value) -> Result<String, ToolError> {
        let action = args.get("action").and_then(serde_json::Value::as_str).ok_or_else(|| ToolError::InvalidArgs("action".to_owned()))?;
        let items = load(context)?;
        if action == "list" { return Ok(items.to_string()); }
        let mut array = items.as_array().cloned().unwrap_or_default();
        let output = match action {
            "create" => {
                let cron = args.get("cron").and_then(serde_json::Value::as_str).ok_or_else(|| ToolError::InvalidArgs("cron".to_owned()))?;
                let prompt = args.get("prompt").and_then(serde_json::Value::as_str).ok_or_else(|| ToolError::InvalidArgs("prompt".to_owned()))?;
                let id = format!("schedule-{}", array.len().saturating_add(1));
                let interval = args.get("interval_seconds").and_then(serde_json::Value::as_u64).unwrap_or(3600).max(60);
                array.push(serde_json::json!({"id":id,"cron":cron,"prompt":prompt,"enabled":true,"interval_seconds":interval,"next_run_ms":now_ms().saturating_add(interval.saturating_mul(1000))}));
                format!("created {id}")
            }
            "update" => {
                let id = args.get("id").and_then(serde_json::Value::as_str).ok_or_else(|| ToolError::InvalidArgs("id".to_owned()))?;
                let item = array.iter_mut().find(|item| item.get("id").and_then(serde_json::Value::as_str) == Some(id)).ok_or_else(|| ToolError::InvalidArgs("schedule not found".to_owned()))?;
                if let Some(enabled) = args.get("enabled").and_then(serde_json::Value::as_bool) { if let Some(object) = item.as_object_mut() { object.insert("enabled".to_owned(), serde_json::json!(enabled)); } }
                if let Some(interval) = args.get("interval_seconds").and_then(serde_json::Value::as_u64) { if let Some(object) = item.as_object_mut() { object.insert("interval_seconds".to_owned(), serde_json::json!(interval.max(60))); } }
                if let Some(prompt) = args.get("prompt").and_then(serde_json::Value::as_str) { if let Some(object) = item.as_object_mut() { object.insert("prompt".to_owned(), serde_json::json!(prompt)); } }
                format!("updated {id}")
            }
            "delete" => {
                let id = args.get("id").and_then(serde_json::Value::as_str).ok_or_else(|| ToolError::InvalidArgs("id".to_owned()))?;
                let before = array.len(); array.retain(|item| item.get("id").and_then(serde_json::Value::as_str) != Some(id));
                format!("deleted {}", before.saturating_sub(array.len()))
            }
            other => return Err(ToolError::InvalidArgs(format!("unknown action {other}"))),
        };
        save(context, &serde_json::Value::Array(array))?;
        Ok(output)
    }

}

fn now_ms() -> u64 { std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|value| u64::try_from(value.as_millis()).unwrap_or(u64::MAX)).unwrap_or(0) }
