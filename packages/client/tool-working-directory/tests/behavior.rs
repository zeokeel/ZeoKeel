#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only
use zk_contract_tool::{Tool, TurnContext};
#[test]
fn returns_workspace_root() {
    let dir = std::env::temp_dir();
    let ctx = TurnContext { conversation_id: "c".to_owned(), workspace_id: "w".to_owned(), workspace_root: dir.display().to_string() };
    let out = zk_tool_working_directory::tool().call_json(&ctx, serde_json::json!({})).unwrap();
    assert!(out.contains("tmp") || out.contains("Temp"));
}
