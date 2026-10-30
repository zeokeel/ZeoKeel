//! Session-scoped todo tool.
#![forbid(unsafe_code)]
use std::sync::Mutex;
use zk_contract_tool::{Tool, ToolError, ToolSpec, TurnContext};

pub struct TodoTool {
    items: Mutex<Vec<serde_json::Value>>,
}
impl Default for TodoTool {
    fn default() -> Self {
        Self {
            items: Mutex::new(Vec::new()),
        }
    }
}
#[must_use]
pub fn tool() -> TodoTool {
    TodoTool::default()
}
impl Tool for TodoTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "todo".to_owned(),
            description: "Read or update the session todo list.".to_owned(),
            parameters: serde_json::json!({"type":"object","properties":{"action":{"type":"string","enum":["list","add","update","remove"]},"id":{"type":"string"},"text":{"type":"string"},"status":{"type":"string","enum":["pending","in_progress","completed"]}},"required":["action"],"additionalProperties":false}),
            mutating: false,
            requires_approval: false,
        }
    }
    fn call(&self, _args: &str) -> Result<String, ToolError> {
        Err(ToolError::InvalidArgs(
            "todo requires JSON arguments".to_owned(),
        ))
    }
    fn call_json(
        &self,
        _context: &TurnContext,
        args: serde_json::Value,
    ) -> Result<String, ToolError> {
        let action = args
            .get("action")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| ToolError::InvalidArgs("action".to_owned()))?;
        let mut items = self
            .items
            .lock()
            .map_err(|_| ToolError::Io("todo lock poisoned".to_owned()))?;
        match action {
            "list" => Ok(serde_json::to_string(&*items).unwrap_or_default()),
            "add" => {
                let text = args
                    .get("text")
                    .and_then(serde_json::Value::as_str)
                    .ok_or_else(|| ToolError::InvalidArgs("text".to_owned()))?;
                let id = format!("todo-{}", items.len().saturating_add(1));
                items.push(serde_json::json!({"id":id,"text":text,"status":"pending"}));
                Ok(format!("added {id}"))
            }
            "update" => {
                let id = args
                    .get("id")
                    .and_then(serde_json::Value::as_str)
                    .ok_or_else(|| ToolError::InvalidArgs("id".to_owned()))?;
                let status = args
                    .get("status")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("pending");
                let item = items
                    .iter_mut()
                    .find(|item| item.get("id").and_then(serde_json::Value::as_str) == Some(id))
                    .ok_or_else(|| ToolError::InvalidArgs("todo not found".to_owned()))?;
                *item = serde_json::json!({"id":id,"text":item.get("text").cloned().unwrap_or_default(),"status":status});
                Ok(format!("updated {id}"))
            }
            "remove" => {
                let id = args
                    .get("id")
                    .and_then(serde_json::Value::as_str)
                    .ok_or_else(|| ToolError::InvalidArgs("id".to_owned()))?;
                let before = items.len();
                items.retain(|item| item.get("id").and_then(serde_json::Value::as_str) != Some(id));
                Ok(format!("removed {}", before.saturating_sub(items.len())))
            }
            other => Err(ToolError::InvalidArgs(format!("unknown action {other}"))),
        }
    }
}
