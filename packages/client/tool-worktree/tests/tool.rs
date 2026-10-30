#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only
use zk_contract_tool::{Tool, TurnContext};
#[test]
fn rejects_injection_and_escape() {
    let dir = std::env::temp_dir().join(format!("zk-wt-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    std::fs::create_dir_all(&dir).unwrap();
    let ctx = TurnContext { conversation_id: "c".to_owned(), workspace_id: "w".to_owned(), workspace_root: dir.display().to_string() };
    let tool = zk_tool_worktree::tool();
    assert!(tool.call_json(&ctx, serde_json::json!({"action":"create","path":"../x","branch":"good"})).is_err());
    assert!(tool.call_json(&ctx, serde_json::json!({"action":"create","path":"x","branch":"a; rm -rf /"})).is_err());
}
