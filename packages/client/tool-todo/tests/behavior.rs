#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only
use zk_contract_tool::{Tool, TurnContext};
#[test]
fn todo_crud() {
    let ctx = TurnContext { conversation_id: "c".to_owned(), workspace_id: "w".to_owned(), workspace_root: ".".to_owned() };
    let tool = zk_tool_todo::tool();
    assert!(tool.call_json(&ctx, serde_json::json!({"action":"add","text":"ship"})).unwrap().contains("todo-1"));
    assert!(tool.call_json(&ctx, serde_json::json!({"action":"update","id":"todo-1","status":"completed"})).unwrap().contains("updated"));
    assert!(tool.call_json(&ctx, serde_json::json!({"action":"list"})).unwrap().contains("completed"));
}
