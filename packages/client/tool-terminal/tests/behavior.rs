#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only
use zk_contract_tool::{Tool, TurnContext};
#[test]
fn open_send_close() {
    let dir = std::env::temp_dir();
    let ctx = TurnContext {
        conversation_id: "c".to_owned(),
        workspace_id: "w".to_owned(),
        workspace_root: dir.display().to_string(),
    };
    let tool = zk_tool_terminal::tool();
    assert!(tool
        .call_json(&ctx, serde_json::json!({"action":"open"}))
        .unwrap()
        .contains("open"));
    assert!(tool
        .call_json(
            &ctx,
            serde_json::json!({"action":"send","command":"printf hi"})
        )
        .unwrap()
        .contains("hi"));
    assert!(tool
        .call_json(&ctx, serde_json::json!({"action":"close"}))
        .unwrap()
        .contains("closed"));
}
