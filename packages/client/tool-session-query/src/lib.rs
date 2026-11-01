//! Session history search/read.
#![forbid(unsafe_code)]
use zk_contract_tool::{Tool, ToolError, ToolSpec, TurnContext};
pub struct SessionQueryTool;
#[must_use]
pub fn tool() -> SessionQueryTool {
    SessionQueryTool
}
impl Tool for SessionQueryTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "session_search".to_owned(),
            description: "Search or read conversation history in the current workspace.".to_owned(),
            parameters: serde_json::json!({"type":"object","properties":{"action":{"type":"string","enum":["search","read"]},"query":{"type":"string"},"conversation_id":{"type":"string"}},"required":["action"],"additionalProperties":false}),
            mutating: false,
            requires_approval: false,
        }
    }
    fn call(&self, _args: &str) -> Result<String, ToolError> {
        Err(ToolError::InvalidArgs(
            "session_search requires JSON".to_owned(),
        ))
    }
    fn call_json(
        &self,
        context: &TurnContext,
        args: serde_json::Value,
    ) -> Result<String, ToolError> {
        let store = zk_kit_session::global()
            .ok_or_else(|| ToolError::Denied("session store unavailable".to_owned()))?;
        let action = args
            .get("action")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| ToolError::InvalidArgs("action".to_owned()))?;
        match action {
            "search" => {
                let query = args
                    .get("query")
                    .and_then(serde_json::Value::as_str)
                    .ok_or_else(|| ToolError::InvalidArgs("query".to_owned()))?;
                // Single search implementation (kit); the tool keeps full content
                // for agents, the IPC surface only exposes snippets (P2-6).
                let result = zk_kit_session::search(&*store, &context.workspace_id, query, 50)
                    .map_err(|error| ToolError::Io(error.to_string()))?;
                let mut out = Vec::new();
                for hit in result.items {
                    let messages = store
                        .messages_in(&context.workspace_id, &hit.conversation_id)
                        .map_err(|error| ToolError::Io(error.to_string()))?;
                    let Some(message) =
                        messages.iter().find(|message| message.id == hit.message_id)
                    else {
                        continue;
                    };
                    out.push(serde_json::json!({"conversation_id":hit.conversation_id,"title":hit.title,"message_id":hit.message_id,"role":hit.role,"content":message.content}));
                }
                Ok(serde_json::json!({"matches":out}).to_string())
            }
            "read" => {
                let conversation_id = args
                    .get("conversation_id")
                    .and_then(serde_json::Value::as_str)
                    .ok_or_else(|| ToolError::InvalidArgs("conversation_id".to_owned()))?;
                let messages = store
                    .messages_in(&context.workspace_id, conversation_id)
                    .map_err(|error| ToolError::Io(error.to_string()))?;
                Ok(serde_json::json!({"messages":messages}).to_string())
            }
            other => Err(ToolError::InvalidArgs(format!("unknown action {other}"))),
        }
    }
}
