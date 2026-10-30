#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only
use zk_contract_tool::{Tool, TurnContext};
#[test]
fn rejects_private_and_non_http() {
    let tool = zk_tool_web_fetch::tool();
    let ctx = TurnContext { conversation_id: "c".to_owned(), workspace_id: "w".to_owned(), workspace_root: ".".to_owned() };
    assert!(tool.call_json(&ctx, serde_json::json!({"url":"http://127.0.0.1/"})).is_err());
    assert!(tool.call_json(&ctx, serde_json::json!({"url":"file:///etc/passwd"})).is_err());
}
