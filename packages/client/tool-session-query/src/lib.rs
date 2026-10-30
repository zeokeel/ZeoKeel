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
                let conversations = store
                    .list_conversations_in(&context.workspace_id)
                    .map_err(|error| ToolError::Io(error.to_string()))?;
                let mut out = Vec::new();
                for conversation in conversations {
                    if out.len() >= 50 {
                        break;
                    }
                    let messages = store
                        .messages_in(&context.workspace_id, &conversation.id)
                        .map_err(|error| ToolError::Io(error.to_string()))?;
                    for message in messages {
                        if message.content.contains(query) {
                            out.push(serde_json::json!({"conversation_id":conversation.id,"title":conversation.title,"message_id":message.id,"role":message.role,"content":message.content}));
                            if out.len() >= 50 {
                                break;
                            }
                        }
                    }
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
